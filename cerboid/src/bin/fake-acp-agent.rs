//! A scripted fake ACP agent for tests of the client, the daemon, the CLI and
//! the desktop app. It speaks ACP v1 over stdio like a real agent but does what
//! a JSON script says, so no model and no network are involved.
//!
//! ```text
//! FAKE_ACP_SCRIPT=script.json fake-acp-agent      # the agent
//! FAKE_ACP_REPLAY=log.jsonl   fake-acp-agent      # replay a recorded log
//! fake-acp-agent mcp-echo                          # a tiny MCP stdio server
//! ```
//!
//! Environment:
//! - `FAKE_ACP_SCRIPT`: path of the script (default: initialize works, no
//!   loadSession, every prompt is answered by echoing its text).
//!   Without it, `$HOME/fake-acp-script.json` when that file exists.
//! - `FAKE_ACP_LOG`: every line received from the client is appended to this file.
//! - `FAKE_ACP_REPLAY`: replay mode, see [`replay`].
//!
//! The script is one JSON object, all keys optional:
//! ```json
//! {
//!   "agent_info": {"name": "fake", "version": "0.1.0"},
//!   "load_session": false,
//!   "protocol_version": 1,
//!   "prompt_capabilities": {"image": false, "audio": false, "embeddedContext": false},
//!   "config_options": [],
//!   "modes": {"currentModeId": "auto", "availableModes": [{"id": "default"}, {"id": "auto"}]},
//!   "init_delay_ms": 0, "init_error": "text", "new_session_error": "text",
//!   "known_sessions": ["ses-old"],
//!   "history": [{"role": "user", "text": "hi"}, {"role": "agent", "text": "hello"}],
//!   "turns": [ [step, step], [step] ]
//! }
//! ```
//! `turns[i]` is what the i-th prompt does (the last one repeats). A step is
//! an object with a `do` key:
//! - `text` {text, chunk_size?}, `thought` {text}: message / thought chunks
//! - `echo`: the text blocks of the prompt as an agent message, `echo: ...`
//! - `plan` {entries}
//! - `tool_call` {id, title?, kind?, status?, raw_input?}
//! - `tool_update` {id, status?, text?, raw_output?}
//! - `mcp_call` {server?, tool, arguments?, id?}: really starts the stdio entry
//!   of `mcpServers` it was given (`server` by name, default the first), does
//!   MCP initialize, notifications/initialized and tools/call, and reports
//!   the call as tool_call / tool_call_update plus `mcp_result: <text>`
//! - `permission` {id?, title?, options?, allow?, reject?, cancelled?, after_cancel?}: sends
//!   session/request_permission, reports `permission: selected <optionId>` or
//!   `permission: cancelled`, then runs the steps of the matching branch
//! - `write_file` {path, contents}: a direct write (relative paths are
//!   relative to the session cwd), reported as `write_file <path>: ok` or
//!   `write_file <path>: error: <os error>`
//! - `call_client` {method, params?}: an agent-to-client request (`fs/read_text_file`,
//!   `terminal/create`, ...), reported as `client_call <method>: error <code>` or `: ok`
//! - `sleep` {ms}: interrupted by session/cancel
//! - `wait_cancel`: wait for session/cancel
//! - `hang`: never answer (ends when stdin closes)
//! - `crash` {stderr?, code?}: write to stderr and exit
//! - `stderr` {text}: write to stderr
//! - `stop` {reason}: end the turn with this stop reason
//! - `raw` {line}: write this line as it is; `big_line` {bytes}: one line of that size
//!
//! After `session/cancel` the turn ends at the next step with stop reason
//! `cancelled`; a pending permission request is still waited for.

use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
enum Step {
    Text { text: String, chunk_size: Option<usize> },
    Thought { text: String },
    Echo,
    Plan { entries: Value },
    ToolCall { id: String, title: Option<String>, kind: Option<String>, status: Option<String>, raw_input: Option<Value> },
    ToolUpdate { id: String, status: Option<String>, text: Option<String>, raw_output: Option<Value> },
    McpCall { server: Option<String>, tool: String, arguments: Option<Value>, id: Option<String> },
    Permission {
        id: Option<String>,
        title: Option<String>,
        options: Option<Vec<Value>>,
        #[serde(default)]
        allow: Vec<Step>,
        #[serde(default)]
        reject: Vec<Step>,
        #[serde(default)]
        cancelled: Vec<Step>,
        /// Wait for `session/cancel` first and ask anyway (an agent that
        /// asks while its turn is being cancelled).
        #[serde(default)]
        after_cancel: bool,
    },
    WriteFile { path: String, contents: Option<String> },
    CallClient { method: String, params: Option<Value> },
    Sleep { ms: u64 },
    WaitCancel,
    Hang,
    Crash { stderr: Option<String>, code: Option<i32> },
    Stderr { text: String },
    Stop { reason: String },
    Raw { line: String },
    BigLine { bytes: usize },
}

#[derive(Debug, Clone, Deserialize)]
struct History {
    role: String,
    text: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Script {
    agent_info: Option<Value>,
    load_session: bool,
    protocol_version: Option<u64>,
    prompt_capabilities: Option<Value>,
    config_options: Option<Value>,
    /// `modes` of session/new and session/load: {"currentModeId", "availableModes": [{"id"}]}.
    modes: Option<Value>,
    init_delay_ms: u64,
    init_error: Option<String>,
    new_session_error: Option<String>,
    known_sessions: Vec<String>,
    history: Vec<History>,
    turns: Vec<Vec<Step>>,
}

enum Incoming {
    Msg(Value),
    Cancel,
    Timeout,
}

enum Flow {
    Continue,
    Stop(String),
}

struct McpConn {
    child: Child,
    lines: Receiver<String>,
    stdin: std::process::ChildStdin,
    next_id: u64,
}

struct Agent {
    script: Script,
    rx: Receiver<String>,
    log: Option<std::fs::File>,
    sessions: HashMap<String, Value>,
    session_cwd: String,
    mcp_servers: Vec<Value>,
    mcp: HashMap<String, McpConn>,
    turn: usize,
    cancelled: bool,
    next_id: u64,
    created: u64,
    prompt_text: String,
}

fn stdin_lines() -> Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    rx
}

fn write_out(v: &Value) {
    write_raw(&v.to_string());
}

fn write_raw(line: &str) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(line.as_bytes());
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}

fn eprint_line(text: &str) {
    let mut err = std::io::stderr().lock();
    let _ = writeln!(err, "{text}");
    let _ = err.flush();
}

fn main() {
    let first = std::env::args().nth(1);
    if first.as_deref() == Some("--version") {
        println!("fake-acp-agent {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if first.as_deref() == Some("mcp-echo") {
        mcp_echo();
        return;
    }
    if let Ok(path) = std::env::var("FAKE_ACP_REPLAY") {
        replay(&path);
        return;
    }
    // A sandboxed or daemon-started agent has a cleared environment: its script
    // may also lie in its home, as `fake-acp-script.json`.
    let script_path = std::env::var("FAKE_ACP_SCRIPT").ok().or_else(|| {
        let file = std::path::Path::new(&std::env::var_os("HOME")?).join("fake-acp-script.json");
        file.is_file().then(|| file.to_string_lossy().into_owned())
    });
    let script = match script_path {
        Some(path) => {
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| die(&format!("cannot read the script {path}: {e}")));
            serde_json::from_str(&text).unwrap_or_else(|e| die(&format!("bad script {path}: {e}")))
        }
        None => Script::default(),
    };
    let log = std::env::var("FAKE_ACP_LOG").ok().and_then(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).ok());
    let mut agent = Agent {
        script,
        rx: stdin_lines(),
        log,
        sessions: HashMap::new(),
        session_cwd: ".".into(),
        mcp_servers: Vec::new(),
        mcp: HashMap::new(),
        turn: 0,
        cancelled: false,
        next_id: 1000,
        created: 0,
        prompt_text: String::new(),
    };
    agent.run();
}

fn die(msg: &str) -> ! {
    eprint_line(&format!("fake-acp-agent: {msg}"));
    std::process::exit(2);
}

fn ok(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn err(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

impl Agent {
    fn exit_clean(&mut self) -> ! {
        for (_, mut conn) in self.mcp.drain() {
            drop(conn.stdin);
            let _ = conn.child.kill();
            let _ = conn.child.wait();
        }
        std::process::exit(0);
    }

    fn next_message(&mut self, timeout: Option<Duration>) -> Incoming {
        let line = match timeout {
            Some(t) => match self.rx.recv_timeout(t) {
                Ok(l) => l,
                Err(RecvTimeoutError::Timeout) => return Incoming::Timeout,
                Err(RecvTimeoutError::Disconnected) => self.exit_clean(),
            },
            None => match self.rx.recv() {
                Ok(l) => l,
                Err(_) => self.exit_clean(),
            },
        };
        if let Some(log) = &mut self.log {
            let _ = writeln!(log, "{line}");
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                eprint_line(&format!("fake-acp-agent: not JSON from the client: {e}"));
                return Incoming::Timeout;
            }
        };
        if msg.get("method").and_then(Value::as_str) == Some("session/cancel") && msg.get("id").is_none() {
            self.cancelled = true;
            return Incoming::Cancel;
        }
        Incoming::Msg(msg)
    }

    fn run(&mut self) {
        loop {
            match self.next_message(None) {
                Incoming::Msg(msg) => self.handle(msg),
                Incoming::Cancel | Incoming::Timeout => {
                    // A cancel while idle is nothing to cancel.
                    self.cancelled = false;
                }
            }
        }
    }

    fn handle(&mut self, msg: Value) {
        let Some(method) = msg.get("method").and_then(Value::as_str).map(str::to_string) else { return };
        let Some(id) = msg.get("id").cloned() else { return };
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        match method.as_str() {
            "initialize" => {
                if self.script.init_delay_ms > 0 {
                    std::thread::sleep(Duration::from_millis(self.script.init_delay_ms));
                }
                if let Some(e) = &self.script.init_error {
                    write_out(&err(&id, -32000, e));
                    return;
                }
                let caps = json!({
                    "loadSession": self.script.load_session,
                    "promptCapabilities": self.script.prompt_capabilities.clone()
                        .unwrap_or(json!({"image": false, "audio": false, "embeddedContext": false})),
                    "mcpCapabilities": {"http": false, "sse": false},
                });
                let info = self.script.agent_info.clone().unwrap_or(json!({"name": "fake-acp-agent", "version": "0.1.0"}));
                write_out(&ok(
                    &id,
                    json!({
                        "protocolVersion": self.script.protocol_version.unwrap_or(1),
                        "agentCapabilities": caps,
                        "agentInfo": info,
                        "authMethods": [],
                    }),
                ));
            }
            "session/new" => {
                if let Some(e) = &self.script.new_session_error {
                    write_out(&err(&id, -32603, e));
                    return;
                }
                self.created += 1;
                let sid = format!("fake-session-{}", self.created);
                self.remember_session(&sid, &params);
                let mut result = json!({"sessionId": sid});
                if let Some(c) = &self.script.config_options {
                    result["configOptions"] = c.clone();
                }
                if let Some(m) = &self.script.modes {
                    result["modes"] = m.clone();
                }
                write_out(&ok(&id, result));
            }
            "session/set_mode" => {
                let wanted = params.get("modeId").and_then(Value::as_str).unwrap_or_default().to_string();
                let offered = self
                    .script
                    .modes
                    .as_ref()
                    .and_then(|m| m.get("availableModes")?.as_array().cloned())
                    .is_some_and(|all| all.iter().any(|m| m.get("id").and_then(Value::as_str) == Some(wanted.as_str())));
                if !offered {
                    write_out(&err(&id, -32602, &format!("no such mode {wanted}")));
                    return;
                }
                // Written where the daemon tests (which cannot set FAKE_ACP_LOG) can read it.
                if let Ok(home) = std::env::var("HOME") {
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(std::path::Path::new(&home).join("fake-acp-modes.log"))
                        .and_then(|mut f| writeln!(f, "{wanted}"));
                }
                write_out(&ok(&id, json!({})));
            }
            "session/load" if self.script.load_session => {
                let sid = params.get("sessionId").and_then(Value::as_str).unwrap_or_default().to_string();
                let known = self.sessions.contains_key(&sid) || self.script.known_sessions.contains(&sid);
                if !known {
                    write_out(&err(&id, -32602, &format!("unknown session {sid}")));
                    return;
                }
                self.remember_session(&sid, &params);
                let history = self.script.history.clone();
                for h in history {
                    let kind = if h.role == "user" { "user_message_chunk" } else { "agent_message_chunk" };
                    self.update(&sid, json!({"sessionUpdate": kind, "content": {"type": "text", "text": h.text}}));
                }
                let mut result = json!({});
                if let Some(m) = &self.script.modes {
                    result["modes"] = m.clone();
                }
                write_out(&ok(&id, result));
            }
            "session/prompt" => self.prompt(id, params),
            _ => write_out(&err(&id, -32601, "Method not found")),
        }
    }

    fn remember_session(&mut self, sid: &str, params: &Value) {
        self.session_cwd = params.get("cwd").and_then(Value::as_str).unwrap_or(".").to_string();
        self.mcp_servers = params.get("mcpServers").and_then(Value::as_array).cloned().unwrap_or_default();
        self.sessions.insert(sid.to_string(), params.clone());
    }

    fn update(&self, sid: &str, update: Value) {
        write_out(&json!({"jsonrpc": "2.0", "method": "session/update", "params": {"sessionId": sid, "update": update}}));
    }

    fn text(&self, sid: &str, text: &str) {
        self.update(sid, json!({"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": text}}));
    }

    fn prompt(&mut self, id: Value, params: Value) {
        let sid = params.get("sessionId").and_then(Value::as_str).unwrap_or_default().to_string();
        if !self.sessions.contains_key(&sid) {
            write_out(&err(&id, -32602, &format!("unknown session {sid}")));
            return;
        }
        self.prompt_text = params
            .get("prompt")
            .and_then(Value::as_array)
            .map(|blocks| blocks.iter().filter_map(|b| b.get("text").and_then(Value::as_str)).collect::<Vec<_>>().join("\n"))
            .unwrap_or_default();
        let steps = if self.script.turns.is_empty() {
            vec![Step::Echo]
        } else {
            self.script.turns[self.turn.min(self.script.turns.len() - 1)].clone()
        };
        self.turn += 1;
        self.cancelled = false;
        let reason = match self.run_steps(&sid, &steps) {
            Flow::Stop(r) => r,
            Flow::Continue => "end_turn".to_string(),
        };
        let reason = if self.cancelled { "cancelled".to_string() } else { reason };
        self.cancelled = false;
        write_out(&ok(&id, json!({"stopReason": reason})));
    }

    fn run_steps(&mut self, sid: &str, steps: &[Step]) -> Flow {
        for step in steps {
            if self.cancelled {
                return Flow::Stop("cancelled".into());
            }
            if let Flow::Stop(r) = self.run_step(sid, step) {
                return Flow::Stop(r);
            }
        }
        Flow::Continue
    }

    /// Wait for the response to the agent request `id`; a cancel in between is
    /// only noted.
    fn await_response(&mut self, id: u64) -> Value {
        loop {
            match self.next_message(None) {
                Incoming::Msg(m) => {
                    if m.get("method").is_none() && m.get("id").and_then(Value::as_u64) == Some(id) {
                        return m;
                    }
                    if let (Some(_), Some(other)) = (m.get("method"), m.get("id")) {
                        write_out(&err(other, -32603, "busy: a turn is running"));
                    }
                }
                Incoming::Cancel | Incoming::Timeout => {}
            }
        }
    }

    fn run_step(&mut self, sid: &str, step: &Step) -> Flow {
        match step {
            Step::Text { text, chunk_size } => match chunk_size {
                Some(n) if *n > 0 => {
                    let chars: Vec<char> = text.chars().collect();
                    for piece in chars.chunks(*n) {
                        self.text(sid, &piece.iter().collect::<String>());
                    }
                }
                _ => self.text(sid, text),
            },
            Step::Thought { text } => {
                self.update(sid, json!({"sessionUpdate": "agent_thought_chunk", "content": {"type": "text", "text": text}}));
            }
            Step::Echo => {
                let text = format!("echo: {}", self.prompt_text);
                self.text(sid, &text);
            }
            Step::Plan { entries } => self.update(sid, json!({"sessionUpdate": "plan", "entries": entries})),
            Step::ToolCall { id, title, kind, status, raw_input } => {
                let mut u = json!({
                    "sessionUpdate": "tool_call",
                    "toolCallId": id,
                    "title": title.clone().unwrap_or_else(|| id.clone()),
                    "kind": kind.clone().unwrap_or_else(|| "other".into()),
                    "status": status.clone().unwrap_or_else(|| "pending".into()),
                });
                if let Some(r) = raw_input {
                    u["rawInput"] = r.clone();
                }
                self.update(sid, u);
            }
            Step::ToolUpdate { id, status, text, raw_output } => {
                let mut u = json!({"sessionUpdate": "tool_call_update", "toolCallId": id});
                if let Some(s) = status {
                    u["status"] = json!(s);
                }
                if let Some(t) = text {
                    u["content"] = json!([{"type": "content", "content": {"type": "text", "text": t}}]);
                }
                if let Some(r) = raw_output {
                    u["rawOutput"] = r.clone();
                }
                self.update(sid, u);
            }
            Step::McpCall { server, tool, arguments, id } => self.mcp_call(sid, server.as_deref(), tool, arguments.clone().unwrap_or(json!({})), id.clone()),
            Step::Permission { id, title, options, allow, reject, cancelled, after_cancel } => {
                if *after_cancel {
                    while !self.cancelled {
                        self.next_message(None);
                    }
                }
                let call_id = id.clone().unwrap_or_else(|| "perm-1".into());
                let options = options.clone().unwrap_or_else(|| {
                    vec![
                        json!({"optionId": "allow", "name": "Allow once", "kind": "allow_once"}),
                        json!({"optionId": "reject", "name": "Reject", "kind": "reject_once"}),
                    ]
                });
                self.next_id += 1;
                let rid = self.next_id;
                write_out(&json!({
                    "jsonrpc": "2.0", "id": rid, "method": "session/request_permission",
                    "params": {
                        "sessionId": sid,
                        "toolCall": {
                            "toolCallId": call_id,
                            "title": title.clone().unwrap_or_else(|| "Do a thing".into()),
                            "kind": "edit",
                            "status": "pending",
                        },
                        "options": options,
                    }
                }));
                let resp = self.await_response(rid);
                let outcome = resp.get("result").and_then(|r| r.get("outcome"));
                let (branch, report) = match outcome.and_then(|o| o.get("outcome")).and_then(Value::as_str) {
                    Some("cancelled") => (cancelled, "permission: cancelled".to_string()),
                    Some("selected") => {
                        let option_id = outcome.and_then(|o| o.get("optionId")).and_then(Value::as_str).unwrap_or_default();
                        let kind = options
                            .iter()
                            .find(|o| o.get("optionId").and_then(Value::as_str) == Some(option_id))
                            .and_then(|o| o.get("kind").and_then(Value::as_str))
                            .unwrap_or("");
                        let branch = if kind.starts_with("allow") { allow } else { reject };
                        (branch, format!("permission: selected {option_id}"))
                    }
                    _ => (reject, format!("permission: malformed {resp}")),
                };
                self.text(sid, &report);
                return self.run_steps(sid, branch);
            }
            Step::WriteFile { path, contents } => {
                let target = std::path::Path::new(&self.session_cwd).join(path);
                let report = match std::fs::write(&target, contents.clone().unwrap_or_else(|| "written by fake-acp-agent\n".into())) {
                    Ok(()) => format!("write_file {path}: ok"),
                    Err(e) => format!("write_file {path}: error: {e}"),
                };
                self.text(sid, &report);
            }
            Step::CallClient { method, params } => {
                self.next_id += 1;
                let rid = self.next_id;
                write_out(&json!({"jsonrpc": "2.0", "id": rid, "method": method, "params": params.clone().unwrap_or(json!({}))}));
                let resp = self.await_response(rid);
                let report = match resp.get("error") {
                    Some(e) => format!("client_call {method}: error {}", e.get("code").and_then(Value::as_i64).unwrap_or(0)),
                    None => format!("client_call {method}: ok"),
                };
                self.text(sid, &report);
            }
            Step::Sleep { ms } => {
                let end = std::time::Instant::now() + Duration::from_millis(*ms);
                loop {
                    let left = end.saturating_duration_since(std::time::Instant::now());
                    if left.is_zero() {
                        break;
                    }
                    if let Incoming::Cancel = self.next_message(Some(left)) {
                        break;
                    }
                }
            }
            Step::WaitCancel => {
                while !self.cancelled {
                    self.next_message(None);
                }
            }
            Step::Hang => loop {
                self.next_message(None);
            },
            Step::Crash { stderr, code } => {
                if let Some(s) = stderr {
                    eprint_line(s);
                }
                std::process::exit(code.unwrap_or(1));
            }
            Step::Stderr { text } => eprint_line(text),
            Step::Stop { reason } => return Flow::Stop(reason.clone()),
            Step::Raw { line } => write_raw(line),
            Step::BigLine { bytes } => {
                let pad = "x".repeat(*bytes);
                write_raw(&format!("{{\"jsonrpc\":\"2.0\",\"method\":\"x/pad\",\"params\":{{\"pad\":\"{pad}\"}}}}"));
            }
        }
        Flow::Continue
    }

    fn mcp_call(&mut self, sid: &str, server: Option<&str>, tool: &str, arguments: Value, id: Option<String>) {
        let name = match server {
            Some(s) => s.to_string(),
            None => self.mcp_servers.first().and_then(|e| e.get("name")).and_then(Value::as_str).unwrap_or("").to_string(),
        };
        let call_id = id.unwrap_or_else(|| format!("mcp-{name}-{tool}"));
        self.update(
            sid,
            json!({"sessionUpdate": "tool_call", "toolCallId": call_id, "title": format!("{name}/{tool}"), "kind": "other", "status": "in_progress", "rawInput": arguments}),
        );
        let result = self.mcp_request(&name, "tools/call", json!({"name": tool, "arguments": arguments}));
        let (status, text, raw) = match result {
            Ok(r) => {
                let text: String = r
                    .get("content")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(|c| c.get("text").and_then(Value::as_str)).collect::<Vec<_>>().join("\n"))
                    .unwrap_or_default();
                let failed = r.get("isError").and_then(Value::as_bool).unwrap_or(false);
                (if failed { "failed" } else { "completed" }, text, r)
            }
            Err(e) => ("failed", format!("mcp_error: {e}"), json!({"error": e})),
        };
        self.update(
            sid,
            json!({"sessionUpdate": "tool_call_update", "toolCallId": call_id, "status": status,
                   "content": [{"type": "content", "content": {"type": "text", "text": text}}], "rawOutput": raw}),
        );
        self.text(sid, &format!("mcp_result: {text}"));
    }

    /// One MCP request to the stdio server entry `name`, started on first use.
    fn mcp_request(&mut self, name: &str, method: &str, params: Value) -> Result<Value, String> {
        if !self.mcp.contains_key(name) {
            let entry = self
                .mcp_servers
                .iter()
                .find(|e| e.get("name").and_then(Value::as_str) == Some(name))
                .cloned()
                .ok_or_else(|| format!("no mcpServers entry named `{name}`"))?;
            let command = entry.get("command").and_then(Value::as_str).ok_or("entry has no command")?;
            let args: Vec<String> = entry.get("args").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
            let mut cmd = Command::new(command);
            cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
            for e in entry.get("env").and_then(Value::as_array).cloned().unwrap_or_default() {
                if let (Some(k), Some(v)) = (e.get("name").and_then(Value::as_str), e.get("value").and_then(Value::as_str)) {
                    cmd.env(k, v);
                }
            }
            let mut child = cmd.spawn().map_err(|e| format!("cannot start {command}: {e}"))?;
            let stdin = child.stdin.take().unwrap();
            let stdout = child.stdout.take().unwrap();
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            });
            let mut conn = McpConn { child, lines: rx, stdin, next_id: 0 };
            mcp_roundtrip(
                &mut conn,
                "initialize",
                json!({"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "fake-acp-agent", "version": "0.1.0"}}),
            )?;
            let note = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
            writeln!(conn.stdin, "{note}").map_err(|e| e.to_string())?;
            conn.stdin.flush().map_err(|e| e.to_string())?;
            self.mcp.insert(name.to_string(), conn);
        }
        let conn = self.mcp.get_mut(name).unwrap();
        mcp_roundtrip(conn, method, params)
    }
}

fn mcp_roundtrip(conn: &mut McpConn, method: &str, params: Value) -> Result<Value, String> {
    conn.next_id += 1;
    let id = conn.next_id;
    let req = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
    writeln!(conn.stdin, "{req}").map_err(|e| format!("write to the MCP server: {e}"))?;
    conn.stdin.flush().map_err(|e| e.to_string())?;
    loop {
        let line = conn.lines.recv_timeout(Duration::from_secs(20)).map_err(|e| format!("no answer from the MCP server: {e}"))?;
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        if v.get("id").and_then(Value::as_u64) != Some(id) || v.get("method").is_some() {
            continue;
        }
        if let Some(e) = v.get("error") {
            return Err(format!("MCP error: {e}"));
        }
        return Ok(v.get("result").cloned().unwrap_or(Value::Null));
    }
}

/// Replay mode: walk a recorded log (see `acp::replay`). `in` entries are
/// written to stdout, `out` entries are read from stdin and must match (same
/// method, or same id for a response). A mismatch ends with exit code 3.
fn replay(path: &str) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("cannot read the log {path}: {e}")));
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    for (n, raw) in text.lines().enumerate() {
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        let entry: Value = serde_json::from_str(raw).unwrap_or_else(|e| die(&format!("log line {}: {e}", n + 1)));
        let msg = entry.get("msg").cloned().unwrap_or(Value::Null);
        match entry.get("dir").and_then(Value::as_str) {
            Some("in") => write_out(&msg),
            Some("out") => {
                let Some(Ok(line)) = lines.next() else {
                    eprint_line(&format!("replay: the client closed before log line {}", n + 1));
                    std::process::exit(4);
                };
                let got: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
                let same = match msg.get("method") {
                    Some(m) => got.get("method") == Some(m) && got.get("id") == msg.get("id"),
                    None => got.get("method").is_none() && got.get("id") == msg.get("id"),
                };
                if !same {
                    eprint_line(&format!("replay mismatch at log line {}: expected {msg}, got {got}", n + 1));
                    std::process::exit(3);
                }
            }
            _ => die(&format!("log line {}: bad dir", n + 1)),
        }
    }
    // The log is over: wait for the client to close its side.
    for _ in lines {}
}

/// `fake-acp-agent mcp-echo`: a minimal MCP stdio server with one tool, `echo`,
/// and one that always fails, `fail`. The result of `echo` is
/// `echo[<FAKE_MCP_TAG>]:<arguments as JSON>`.
fn mcp_echo() {
    let tag = std::env::var("FAKE_MCP_TAG").unwrap_or_default();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines().map_while(Result::ok) {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        let Some(id) = msg.get("id").cloned() else { continue };
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let reply = match msg.get("method").and_then(Value::as_str) {
            Some("initialize") => ok(
                &id,
                json!({
                    "protocolVersion": params.get("protocolVersion").cloned().unwrap_or(json!("2025-03-26")),
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "mcp-echo", "version": "0.1.0"},
                }),
            ),
            Some("tools/list") => ok(
                &id,
                json!({"tools": [
                    {"name": "echo", "description": "Echo the arguments", "inputSchema": {"type": "object"}},
                    {"name": "fail", "description": "Always fails", "inputSchema": {"type": "object"}},
                ]}),
            ),
            Some("tools/call") => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                match name {
                    "echo" => ok(&id, json!({"content": [{"type": "text", "text": format!("echo[{tag}]:{args}")}], "isError": false})),
                    "fail" => ok(&id, json!({"content": [{"type": "text", "text": "failed on purpose"}], "isError": true})),
                    other => err(&id, -32602, &format!("unknown tool {other}")),
                }
            }
            _ => err(&id, -32601, "Method not found"),
        };
        write_out(&reply);
    }
}
