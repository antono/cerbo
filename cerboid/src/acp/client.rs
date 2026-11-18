//! The ACP client: spawns an agent, speaks newline-delimited JSON-RPC 2.0 over
//! its stdio and turns what the agent says into [`AcpEvent`]s.

use super::types::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout, Command};
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tokio::task::JoinHandle;

/// JSON-RPC "method not found", the answer to every request we do not serve.
pub const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

/// Settings of one client.
#[derive(Debug, Clone)]
pub struct ClientOptions {
    /// The longest line read from the agent. A longer one ends the connection:
    /// the agent is killed and the exit reports why.
    pub max_line_bytes: usize,
    /// How long `initialize`, `session/new` and `session/load` may take.
    pub request_timeout: Duration,
    /// How long `session/prompt` may take; `None` is no limit (a turn can be long).
    pub prompt_timeout: Option<Duration>,
    /// How much of the end of stderr is kept for crash reports.
    pub stderr_tail_bytes: usize,
    /// How long [`AcpClient::shutdown`] waits for the agent to leave after its
    /// stdin is closed, before it kills it.
    pub shutdown_grace: Duration,
    /// Name and version sent as `clientInfo`.
    pub client_name: String,
    pub client_version: String,
    /// Append every message, both directions, to this JSONL file (see
    /// [`super::replay`]).
    pub record: Option<PathBuf>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            max_line_bytes: 8 * 1024 * 1024,
            request_timeout: Duration::from_secs(30),
            prompt_timeout: None,
            stderr_tail_bytes: 16 * 1024,
            shutdown_grace: Duration::from_secs(2),
            client_name: "cerbo".to_string(),
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            record: None,
        }
    }
}

/// How the agent process ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitInfo {
    /// The exit code; `None` when a signal ended it.
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub success: bool,
    /// The end of what the agent wrote to stderr.
    pub stderr_tail: String,
    /// Set when the client ended the agent for a protocol reason (a line over
    /// the limit); `None` for an exit of the agent's own.
    pub reason: Option<String>,
}

impl fmt::Display for ExitInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the agent ")?;
        if let Some(reason) = &self.reason {
            write!(f, "was stopped ({reason})")?;
        } else if let Some(sig) = self.signal {
            write!(f, "was killed by signal {sig}")?;
        } else if let Some(code) = self.code {
            write!(f, "exited with code {code}")?;
        } else {
            write!(f, "exited")?;
        }
        let tail = self.stderr_tail.trim();
        if !tail.is_empty() {
            write!(f, "; stderr: {tail}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum AcpError {
    /// The process did not start.
    Spawn(String),
    /// The agent did not answer in time.
    Timeout { method: String, after: Duration },
    /// The agent answered with a JSON-RPC error.
    Rpc { code: i64, message: String, data: Option<Value> },
    /// The agent process is gone.
    Exited(Arc<ExitInfo>),
    /// The connection ended without an exit being known (should not happen).
    Closed,
    /// The agent broke the protocol, or the call is wrong.
    Protocol(String),
    /// The agent speaks a protocol version this client does not.
    UnsupportedVersion(u64),
    /// The agent does not support what was asked (the message says what).
    Unsupported(String),
}

impl fmt::Display for AcpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(e) => write!(f, "cannot start the agent: {e}"),
            Self::Timeout { method, after } => write!(f, "the agent did not answer {method} within {after:?}"),
            Self::Rpc { code, message, .. } => write!(f, "the agent answered with an error ({code}): {message}"),
            Self::Exited(info) => write!(f, "{info}"),
            Self::Closed => write!(f, "the connection to the agent is closed"),
            Self::Protocol(e) => write!(f, "ACP protocol error: {e}"),
            Self::UnsupportedVersion(v) => write!(f, "the agent speaks ACP version {v}; this client speaks {PROTOCOL_VERSION}"),
            Self::Unsupported(e) => write!(f, "not supported by the agent: {e}"),
        }
    }
}

impl std::error::Error for AcpError {}

/// What the client reports besides call results, in order.
#[derive(Debug)]
pub enum AcpEvent {
    /// A `session/update`.
    Update(SessionNotification),
    /// The agent asks for permission. Answer it with [`PermissionRequest::select`]
    /// or [`PermissionRequest::cancel`]; dropping it cancels.
    Permission(PermissionRequest),
    /// The agent sent a request this client does not serve (`fs/*`,
    /// `terminal/*`, `elicitation/*`, ...); it was answered with -32601.
    Refused { method: String },
    /// A notification that is not `session/update`.
    Notification { method: String, params: Value },
    /// Something the client could not use: a line that is not JSON, an
    /// unknown response id, a malformed update. The connection goes on.
    Warning(String),
    /// The agent process ended. The last event.
    Exited(Arc<ExitInfo>),
}

/// The stream of events of one client. Events of one connection arrive in the
/// order the agent sent them; by the time `prompt` returns, every update of
/// the turn is in the stream.
pub type AcpEvents = mpsc::UnboundedReceiver<AcpEvent>;

/// A `session/request_permission` waiting for an answer.
#[derive(Debug)]
pub struct PermissionRequest {
    pub session_id: String,
    pub tool_call: ToolCallUpdate,
    pub options: Vec<PermissionOption>,
    /// The request params as received.
    pub raw: Value,
    responder: oneshot::Sender<PermissionOutcome>,
    /// Set, synchronously, by [`AcpClient::cancel`] for every request of the
    /// session that is open then, so a request that waits in the event stream
    /// while the turn is cancelled can be told from a live one.
    cancelled: Arc<AtomicBool>,
}

impl PermissionRequest {
    /// Whether the turn this request belongs to was cancelled after the agent
    /// asked (or before it was read from the event stream). Such a request was
    /// answered `cancelled` already: show no card for it.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Allow or reject by choosing one of the offered options. An id that was
    /// not offered answers `cancelled` (never an approval) and is an error.
    /// Also an error when the turn was cancelled meanwhile: the agent already
    /// got `cancelled`.
    pub fn select(self, option_id: &str) -> Result<(), AcpError> {
        if !self.options.iter().any(|o| o.option_id == option_id) {
            let _ = self.responder.send(PermissionOutcome::Cancelled);
            return Err(AcpError::Protocol(format!("`{option_id}` is not one of the offered options")));
        }
        self.responder
            .send(PermissionOutcome::Selected(option_id.to_string()))
            .map_err(|_| AcpError::Protocol("the permission request is already answered or cancelled".into()))
    }

    /// The id of the first offered option of this kind, to pass to
    /// [`Self::select`].
    pub fn option_of_kind(&self, kind: PermissionKind) -> Option<String> {
        self.options.iter().find(|o| o.kind == kind).map(|o| o.option_id.clone())
    }

    pub fn cancel(self) {
        let _ = self.responder.send(PermissionOutcome::Cancelled);
    }
}

/// A ring of the last bytes of the agent's stderr.
struct Ring {
    cap: usize,
    buf: Mutex<VecDeque<u8>>,
}

impl Ring {
    fn push(&self, bytes: &[u8]) {
        let mut buf = self.buf.lock().unwrap();
        buf.extend(bytes.iter().copied());
        let over = buf.len().saturating_sub(self.cap);
        if over > 0 {
            buf.drain(..over);
        }
    }

    fn tail(&self) -> String {
        let buf = self.buf.lock().unwrap();
        let bytes: Vec<u8> = buf.iter().copied().collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

type Reply = Result<Value, AcpError>;

#[derive(Default)]
struct Pending {
    map: HashMap<u64, oneshot::Sender<Reply>>,
    /// Set once the agent is gone; no request is accepted after.
    closed: Option<Arc<ExitInfo>>,
}

struct Inner {
    opts: ClientOptions,
    writer: mpsc::UnboundedSender<String>,
    close_stdin: watch::Sender<bool>,
    kill: mpsc::UnboundedSender<()>,
    pending: Mutex<Pending>,
    next_id: AtomicU64,
    stderr: Arc<Ring>,
    exit: watch::Receiver<Option<Arc<ExitInfo>>>,
    cancel: broadcast::Sender<String>,
    /// Sessions whose turn was cancelled and whose `prompt` has not returned
    /// yet: a permission request read for one of them is answered `cancelled`
    /// at once and never shown. Always locked before `open_requests`.
    turn_cancelled: Mutex<HashSet<String>>,
    /// The permission requests that wait for an answer, by session.
    open_requests: Mutex<Vec<(String, Arc<AtomicBool>)>>,
    caps: Mutex<Option<AgentCapabilities>>,
    record: Option<Mutex<std::fs::File>>,
    /// Why the client ended the agent, if it did.
    fatal: Mutex<Option<String>>,
    pid: Option<u32>,
}

impl Inner {
    fn record(&self, dir: &str, msg: &Value) {
        if let Some(file) = &self.record {
            let line = json!({"dir": dir, "msg": msg}).to_string();
            let mut file = file.lock().unwrap();
            let _ = writeln!(file, "{line}");
        }
    }

    fn send(&self, msg: Value) -> bool {
        self.record("out", &msg);
        self.writer.send(msg.to_string()).is_ok()
    }

    fn respond_ok(&self, id: Value, result: Value) {
        self.send(json!({"jsonrpc": "2.0", "id": id, "result": result}));
    }

    fn respond_err(&self, id: Value, code: i64, message: &str, data: Option<Value>) {
        let mut error = json!({"code": code, "message": message});
        if let Some(data) = data {
            error["data"] = data;
        }
        self.send(json!({"jsonrpc": "2.0", "id": id, "error": error}));
    }
}

/// A running agent and the calls to it. Not `Clone`; share it in an `Arc`.
/// Dropping it stops the agent.
pub struct AcpClient {
    inner: Arc<Inner>,
}

impl Drop for AcpClient {
    fn drop(&mut self) {
        let _ = self.inner.close_stdin.send(true);
        let _ = self.inner.kill.send(());
    }
}

impl AcpClient {
    /// Start `argv[0]` with its arguments, the given extra environment and
    /// working directory. The sandbox wrapper is the caller's: pass the
    /// wrapper's argv here, or build a [`Command`] yourself for [`Self::spawn`].
    pub fn spawn_argv(
        argv: &[String],
        env: &[(String, String)],
        cwd: Option<&Path>,
        opts: ClientOptions,
    ) -> Result<(Self, AcpEvents), AcpError> {
        let (program, args) = argv.split_first().ok_or_else(|| AcpError::Spawn("empty command".into()))?;
        let mut cmd = Command::new(program);
        cmd.args(args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }
        Self::spawn(cmd, opts)
    }

    /// Start the agent a prepared command describes. Stdio is set up here
    /// (piped), whatever the command had.
    pub fn spawn(mut cmd: Command, opts: ClientOptions) -> Result<(Self, AcpEvents), AcpError> {
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        let record = match &opts.record {
            Some(path) => Some(Mutex::new(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|e| AcpError::Spawn(format!("cannot open the log {}: {e}", path.display())))?,
            )),
            None => None,
        };
        let mut child = cmd.spawn().map_err(|e| AcpError::Spawn(e.to_string()))?;
        let pid = child.id();
        let stdin = child.stdin.take().expect("stdin is piped");
        let stdout = child.stdout.take().expect("stdout is piped");
        let stderr = child.stderr.take().expect("stderr is piped");

        let (writer_tx, writer_rx) = mpsc::unbounded_channel::<String>();
        let (close_tx, close_rx) = watch::channel(false);
        let (kill_tx, kill_rx) = mpsc::unbounded_channel::<()>();
        let (exit_tx, exit_rx) = watch::channel(None);
        let (events_tx, events_rx) = mpsc::unbounded_channel::<AcpEvent>();
        let (cancel_tx, _) = broadcast::channel(64);
        let ring = Arc::new(Ring { cap: opts.stderr_tail_bytes, buf: Mutex::new(VecDeque::new()) });

        let inner = Arc::new(Inner {
            opts,
            writer: writer_tx,
            close_stdin: close_tx,
            kill: kill_tx,
            pending: Mutex::new(Pending::default()),
            next_id: AtomicU64::new(1),
            stderr: Arc::clone(&ring),
            exit: exit_rx,
            cancel: cancel_tx,
            turn_cancelled: Mutex::new(HashSet::new()),
            open_requests: Mutex::new(Vec::new()),
            caps: Mutex::new(None),
            record,
            fatal: Mutex::new(None),
            pid,
        });

        tokio::spawn(write_loop(stdin, writer_rx, close_rx));
        let stderr_task = tokio::spawn(stderr_loop(stderr, ring));
        let reader_task = tokio::spawn(read_loop(Arc::clone(&inner), stdout, events_tx.clone()));
        tokio::spawn(supervise(child, Arc::clone(&inner), kill_rx, reader_task, stderr_task, exit_tx, events_tx));

        Ok((Self { inner }, events_rx))
    }

    /// The process id of the agent (of the sandbox wrapper, when wrapped).
    pub fn pid(&self) -> Option<u32> {
        self.inner.pid
    }

    /// What the agent advertised in `initialize`; `None` before it.
    pub fn capabilities(&self) -> Option<AgentCapabilities> {
        self.inner.caps.lock().unwrap().clone()
    }

    /// The end of the agent's stderr so far.
    pub fn stderr_tail(&self) -> String {
        self.inner.stderr.tail()
    }

    /// How the agent ended; `None` while it runs.
    pub fn exit_info(&self) -> Option<Arc<ExitInfo>> {
        self.inner.exit.borrow().clone()
    }

    /// Wait until the agent has ended.
    pub async fn wait_exit(&self) -> Arc<ExitInfo> {
        let mut rx = self.inner.exit.clone();
        loop {
            if let Some(info) = rx.borrow_and_update().clone() {
                return info;
            }
            if rx.changed().await.is_err() {
                // The supervisor is gone; it always sets the value first.
                return rx.borrow().clone().expect("the exit is set before the supervisor ends");
            }
        }
    }

    async fn request(&self, method: &str, params: Value, timeout: Option<Duration>) -> Result<Value, AcpError> {
        let inner = &self.inner;
        let id = inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = inner.pending.lock().unwrap();
            if let Some(info) = &pending.closed {
                return Err(AcpError::Exited(Arc::clone(info)));
            }
            pending.map.insert(id, tx);
        }
        let sent = inner.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        if !sent {
            inner.pending.lock().unwrap().map.remove(&id);
            return Err(self.exit_info().map(AcpError::Exited).unwrap_or(AcpError::Closed));
        }
        let reply = match timeout {
            Some(after) => match tokio::time::timeout(after, rx).await {
                Ok(reply) => reply,
                Err(_) => {
                    inner.pending.lock().unwrap().map.remove(&id);
                    return Err(AcpError::Timeout { method: method.to_string(), after });
                }
            },
            None => rx.await,
        };
        match reply {
            Ok(reply) => reply,
            Err(_) => Err(self.exit_info().map(AcpError::Exited).unwrap_or(AcpError::Closed)),
        }
    }

    /// `initialize`: protocol version 1, no `fs` and no `terminal` capability.
    /// Ends the agent and fails when it answers with another version.
    pub async fn initialize(&self) -> Result<InitializeResult, AcpError> {
        let o = &self.inner.opts;
        let params = json!({
            "protocolVersion": PROTOCOL_VERSION,
            "clientCapabilities": {},
            "clientInfo": {"name": o.client_name, "title": "Cerbo", "version": o.client_version},
        });
        let result = self.request("initialize", params, Some(o.request_timeout)).await?;
        let version = result.get("protocolVersion").and_then(Value::as_u64).ok_or_else(|| AcpError::Protocol("initialize: no protocolVersion".into()))?;
        if version != PROTOCOL_VERSION {
            let _ = self.inner.kill.send(());
            return Err(AcpError::UnsupportedVersion(version));
        }
        let caps = AgentCapabilities::from_value(result.get("agentCapabilities").unwrap_or(&Value::Null));
        *self.inner.caps.lock().unwrap() = Some(caps.clone());
        let agent_info = result.get("agentInfo").filter(|v| v.is_object()).map(|v| AgentInfo {
            name: v.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
            title: v.get("title").and_then(Value::as_str).map(str::to_string),
            version: v.get("version").and_then(Value::as_str).map(str::to_string),
        });
        let auth_methods = result.get("authMethods").and_then(Value::as_array).cloned().unwrap_or_default();
        Ok(InitializeResult { protocol_version: version, capabilities: caps, agent_info, auth_methods })
    }

    fn session_params(cwd: &Path, mcp_servers: &[McpServer], meta: Option<&Value>) -> Result<Value, AcpError> {
        if !cwd.is_absolute() {
            return Err(AcpError::Protocol(format!("the session directory must be absolute: {}", cwd.display())));
        }
        let cwd = cwd.to_str().ok_or_else(|| AcpError::Protocol("the session directory is not UTF-8".into()))?;
        let servers: Vec<Value> = mcp_servers.iter().map(McpServer::to_value).collect();
        let mut params = json!({"cwd": cwd, "mcpServers": servers});
        if let Some(meta) = meta {
            params["_meta"] = meta.clone();
        }
        Ok(params)
    }

    /// `session/new`. `cwd` must be absolute.
    pub async fn new_session(&self, cwd: &Path, mcp_servers: &[McpServer]) -> Result<NewSession, AcpError> {
        self.new_session_with_meta(cwd, mcp_servers, None).await
    }

    /// `session/new` with an `_meta` object (agent-specific options, such as
    /// the Claude adapter's `claudeCode.options`).
    pub async fn new_session_with_meta(&self, cwd: &Path, mcp_servers: &[McpServer], meta: Option<&Value>) -> Result<NewSession, AcpError> {
        let params = Self::session_params(cwd, mcp_servers, meta)?;
        let result = self.request("session/new", params, Some(self.inner.opts.request_timeout)).await?;
        let session_id = result
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| AcpError::Protocol("session/new: no sessionId".into()))?
            .to_string();
        Ok(NewSession {
            session_id,
            modes: result.get("modes").filter(|v| !v.is_null()).cloned(),
            config_options: result.get("configOptions").filter(|v| !v.is_null()).cloned(),
        })
    }

    /// `session/load`: only when the agent advertised `loadSession`. The
    /// agent replays the history as `UserMessageChunk` / `AgentMessageChunk`
    /// updates before this returns.
    pub async fn load_session(&self, session_id: &str, cwd: &Path, mcp_servers: &[McpServer]) -> Result<LoadedSession, AcpError> {
        self.load_session_with_meta(session_id, cwd, mcp_servers, None).await
    }

    /// `session/load` with an `_meta` object, as [`Self::new_session_with_meta`].
    pub async fn load_session_with_meta(
        &self,
        session_id: &str,
        cwd: &Path,
        mcp_servers: &[McpServer],
        meta: Option<&Value>,
    ) -> Result<LoadedSession, AcpError> {
        match self.capabilities() {
            Some(caps) if caps.load_session => {}
            Some(_) => return Err(AcpError::Unsupported("session/load (the agent did not advertise loadSession)".into())),
            None => return Err(AcpError::Protocol("initialize first".into())),
        }
        let mut params = Self::session_params(cwd, mcp_servers, meta)?;
        params["sessionId"] = json!(session_id);
        let result = self.request("session/load", params, Some(self.inner.opts.request_timeout)).await?;
        Ok(LoadedSession {
            modes: result.get("modes").filter(|v| !v.is_null()).cloned(),
            config_options: result.get("configOptions").filter(|v| !v.is_null()).cloned(),
        })
    }

    /// `session/set_mode`: switch the session to the agent's mode `mode_id`.
    pub async fn set_mode(&self, session_id: &str, mode_id: &str) -> Result<(), AcpError> {
        self.request("session/set_mode", json!({"sessionId": session_id, "modeId": mode_id}), Some(self.inner.opts.request_timeout)).await?;
        Ok(())
    }

    /// Make `wanted` the session's mode when the agent reports a different one
    /// in `modes` (the `modes` of `session/new` or `session/load`): an agent
    /// can choose its start mode from files of the vault, which the vault's
    /// owner is not the person to decide. Nothing is sent when the agent
    /// reports no modes or is in `wanted` already. An agent that does not offer
    /// `wanted` is an error. Returns whether the mode was changed.
    pub async fn pin_mode(&self, session_id: &str, modes: Option<&Value>, wanted: &str) -> Result<bool, AcpError> {
        let Some(modes) = modes else { return Ok(false) };
        let current = modes.get("currentModeId").and_then(Value::as_str);
        if current.is_none() || current == Some(wanted) {
            return Ok(false);
        }
        let offered = modes
            .get("availableModes")
            .and_then(Value::as_array)
            .is_some_and(|all| all.iter().any(|m| m.get("id").and_then(Value::as_str) == Some(wanted)));
        if !offered {
            return Err(AcpError::Unsupported(format!("the permission mode `{wanted}` (the agent started in `{}`)", current.unwrap_or_default())));
        }
        self.set_mode(session_id, wanted).await?;
        Ok(true)
    }

    /// `session/prompt`: returns the stop reason when the turn ends. Only block
    /// types the agent advertised are sent (text and resource links always).
    pub async fn prompt(&self, session_id: &str, blocks: &[ContentBlock]) -> Result<StopReason, AcpError> {
        let caps = self.capabilities().ok_or_else(|| AcpError::Protocol("initialize first".into()))?;
        for block in blocks {
            let missing = match block {
                ContentBlock::Image { .. } if !caps.prompt.image => Some("image"),
                ContentBlock::Audio { .. } if !caps.prompt.audio => Some("audio"),
                ContentBlock::Resource { .. } if !caps.prompt.embedded_context => Some("embedded resource"),
                ContentBlock::Unknown(_) => Some("unknown"),
                _ => None,
            };
            if let Some(kind) = missing {
                return Err(AcpError::Unsupported(format!("{kind} content in a prompt")));
            }
        }
        let prompt: Vec<Value> = blocks.iter().map(ContentBlock::to_value).collect();
        // A cancel that found no turn to stop (it came after the last one
        // ended) must not cancel this one.
        self.inner.turn_cancelled.lock().unwrap().remove(session_id);
        let result = self
            .request("session/prompt", json!({"sessionId": session_id, "prompt": prompt}), self.inner.opts.prompt_timeout)
            .await;
        // The turn is over, however it ended: later requests are a new turn's.
        self.inner.turn_cancelled.lock().unwrap().remove(session_id);
        let result = result?;
        let reason = result.get("stopReason").and_then(Value::as_str).ok_or_else(|| AcpError::Protocol("session/prompt: no stopReason".into()))?;
        Ok(StopReason::from(reason))
    }

    /// `session/cancel`, a notification. Every permission request of that
    /// session that is still open is answered `cancelled`; the pending
    /// `prompt` then ends with `StopReason::Cancelled`.
    pub fn cancel(&self, session_id: &str) {
        // First the permission answers, so no approval can race the cancel.
        // The flag and the open requests are marked under one lock, so a
        // request is either open (and marked here) or arrives later (and finds
        // the flag): none slips between.
        {
            let mut cancelled = self.inner.turn_cancelled.lock().unwrap();
            cancelled.insert(session_id.to_string());
            for (session, flag) in self.inner.open_requests.lock().unwrap().iter() {
                if session == session_id {
                    flag.store(true, Ordering::SeqCst);
                }
            }
        }
        let _ = self.inner.cancel.send(session_id.to_string());
        self.inner.send(json!({"jsonrpc": "2.0", "method": "session/cancel", "params": {"sessionId": session_id}}));
    }

    /// Stop the agent: close its stdin, wait up to the grace period for it to
    /// leave, then kill it. Returns how it ended.
    pub async fn shutdown(&self) -> Arc<ExitInfo> {
        let _ = self.inner.close_stdin.send(true);
        if tokio::time::timeout(self.inner.opts.shutdown_grace, self.wait_exit()).await.is_err() {
            let _ = self.inner.kill.send(());
        }
        self.wait_exit().await
    }

    /// Kill the agent now and wait for it to be gone.
    pub async fn kill(&self) -> Arc<ExitInfo> {
        let _ = self.inner.kill.send(());
        self.wait_exit().await
    }
}

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

async fn write_loop(mut stdin: tokio::process::ChildStdin, mut rx: mpsc::UnboundedReceiver<String>, mut close: watch::Receiver<bool>) {
    loop {
        tokio::select! {
            line = rx.recv() => {
                let Some(line) = line else { break };
                if write_line(&mut stdin, &line).await.is_err() {
                    break;
                }
            }
            _ = close.changed() => {
                // What was queued before the close still goes out.
                while let Ok(line) = rx.try_recv() {
                    if write_line(&mut stdin, &line).await.is_err() {
                        break;
                    }
                }
                break;
            }
        }
    }
    let _ = stdin.shutdown().await;
    // Dropping `stdin` closes the pipe: the agent sees EOF.
}

async fn write_line(stdin: &mut tokio::process::ChildStdin, line: &str) -> std::io::Result<()> {
    stdin.write_all(line.as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await
}

async fn stderr_loop(mut stderr: ChildStderr, ring: Arc<Ring>) {
    let mut buf = [0u8; 4096];
    loop {
        match stderr.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => ring.push(&buf[..n]),
        }
    }
}

enum LineRead {
    Line,
    Eof,
    TooLong,
}

/// One line, at most `max` bytes, into `buf` (without the newline).
async fn read_line_bounded(r: &mut BufReader<ChildStdout>, buf: &mut Vec<u8>, max: usize) -> std::io::Result<LineRead> {
    buf.clear();
    loop {
        let available = r.fill_buf().await?;
        if available.is_empty() {
            return Ok(if buf.is_empty() { LineRead::Eof } else { LineRead::Line });
        }
        match available.iter().position(|b| *b == b'\n') {
            Some(i) => {
                buf.extend_from_slice(&available[..i]);
                r.consume(i + 1);
                return Ok(if buf.len() > max { LineRead::TooLong } else { LineRead::Line });
            }
            None => {
                let n = available.len();
                buf.extend_from_slice(available);
                r.consume(n);
                if buf.len() > max {
                    return Ok(LineRead::TooLong);
                }
            }
        }
    }
}

async fn read_loop(inner: Arc<Inner>, stdout: ChildStdout, events: mpsc::UnboundedSender<AcpEvent>) {
    let mut reader = BufReader::new(stdout);
    let mut buf = Vec::new();
    loop {
        match read_line_bounded(&mut reader, &mut buf, inner.opts.max_line_bytes).await {
            Ok(LineRead::Line) => handle_line(&inner, &buf, &events),
            Ok(LineRead::Eof) | Err(_) => break,
            Ok(LineRead::TooLong) => {
                *inner.fatal.lock().unwrap() = Some(format!("the agent wrote a line over {} bytes", inner.opts.max_line_bytes));
                let _ = inner.kill.send(());
                break;
            }
        }
    }
}

fn handle_line(inner: &Arc<Inner>, line: &[u8], events: &mpsc::UnboundedSender<AcpEvent>) {
    let text = String::from_utf8_lossy(line);
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let msg: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            let shown: String = text.chars().take(200).collect();
            let _ = events.send(AcpEvent::Warning(format!("a line from the agent is not JSON ({e}): {shown}")));
            return;
        }
    };
    inner.record("in", &msg);

    let method = msg.get("method").and_then(Value::as_str).map(str::to_string);
    let id = msg.get("id").filter(|v| !v.is_null()).cloned();
    match (method, id) {
        (Some(method), Some(id)) => handle_request(inner, events, method, id, msg.get("params").cloned().unwrap_or(Value::Null)),
        (Some(method), None) => handle_notification(events, method, msg.get("params").cloned().unwrap_or(Value::Null)),
        (None, Some(id)) => handle_response(inner, events, &id, &msg),
        (None, None) => {
            let _ = events.send(AcpEvent::Warning("a message from the agent has neither method nor id".into()));
        }
    }
}

fn handle_response(inner: &Arc<Inner>, events: &mpsc::UnboundedSender<AcpEvent>, id: &Value, msg: &Value) {
    let key = id.as_u64().or_else(|| id.as_str().and_then(|s| s.parse().ok()));
    let tx = key.and_then(|k| inner.pending.lock().unwrap().map.remove(&k));
    let Some(tx) = tx else {
        let _ = events.send(AcpEvent::Warning(format!("a response to no pending request: id {id}")));
        return;
    };
    let reply = if let Some(error) = msg.get("error") {
        Err(AcpError::Rpc {
            code: error.get("code").and_then(Value::as_i64).unwrap_or(0),
            message: error.get("message").and_then(Value::as_str).unwrap_or_default().to_string(),
            data: error.get("data").cloned(),
        })
    } else {
        Ok(msg.get("result").cloned().unwrap_or(Value::Null))
    };
    let _ = tx.send(reply);
}

fn handle_notification(events: &mpsc::UnboundedSender<AcpEvent>, method: String, params: Value) {
    if method == "session/update" {
        match SessionNotification::from_params(&params) {
            Some(n) => {
                let _ = events.send(AcpEvent::Update(n));
            }
            None => {
                let _ = events.send(AcpEvent::Warning("a session/update that is not understood".into()));
            }
        }
    } else {
        let _ = events.send(AcpEvent::Notification { method, params });
    }
}

fn handle_request(inner: &Arc<Inner>, events: &mpsc::UnboundedSender<AcpEvent>, method: String, id: Value, params: Value) {
    if method != "session/request_permission" {
        // fs/*, terminal/*, elicitation/*, anything else: we advertised none of it.
        inner.respond_err(id, METHOD_NOT_FOUND, "Method not found", Some(json!({"method": method})));
        let _ = events.send(AcpEvent::Refused { method });
        return;
    }
    let parsed = (|| -> Option<(String, ToolCallUpdate, Vec<PermissionOption>)> {
        let session_id = params.get("sessionId")?.as_str()?.to_string();
        let tool_call = ToolCallUpdate::from_value(params.get("toolCall")?)?;
        let options = params
            .get("options")?
            .as_array()?
            .iter()
            .map(|o| {
                Some(PermissionOption {
                    option_id: o.get("optionId")?.as_str()?.to_string(),
                    name: o.get("name")?.as_str()?.to_string(),
                    kind: PermissionKind::from(o.get("kind")?.as_str()?),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some((session_id, tool_call, options))
    })();
    let Some((session_id, tool_call, options)) = parsed else {
        inner.respond_err(id, INVALID_PARAMS, "Invalid params for session/request_permission", None);
        return;
    };

    let (tx, mut rx) = oneshot::channel::<PermissionOutcome>();
    // Subscribed before the request is shown, so a cancel cannot slip between.
    let mut cancelled = inner.cancel.subscribe();
    let flag = Arc::new(AtomicBool::new(false));
    {
        let turn_cancelled = inner.turn_cancelled.lock().unwrap();
        if turn_cancelled.contains(&session_id) {
            // The turn was cancelled already: answer `cancelled`, show nothing.
            drop(turn_cancelled);
            inner.respond_ok(id, PermissionOutcome::Cancelled.to_result());
            return;
        }
        inner.open_requests.lock().unwrap().push((session_id.clone(), Arc::clone(&flag)));
    }
    let request = PermissionRequest { session_id: session_id.clone(), tool_call, options, raw: params, responder: tx, cancelled: Arc::clone(&flag) };
    if events.send(AcpEvent::Permission(request)).is_err() {
        // Nobody listens: never approve.
        inner.open_requests.lock().unwrap().retain(|(_, f)| !Arc::ptr_eq(f, &flag));
        inner.respond_ok(id, PermissionOutcome::Cancelled.to_result());
        return;
    }
    let inner = Arc::clone(inner);
    tokio::spawn(async move {
        let outcome = loop {
            tokio::select! {
                answer = &mut rx => break answer.unwrap_or(PermissionOutcome::Cancelled),
                note = cancelled.recv() => match note {
                    Ok(s) if s == session_id => break PermissionOutcome::Cancelled,
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break PermissionOutcome::Cancelled,
                },
            }
        };
        // The receiver ends here: a late `select` finds the request closed.
        drop(rx);
        inner.open_requests.lock().unwrap().retain(|(_, f)| !Arc::ptr_eq(f, &flag));
        inner.respond_ok(id, outcome.to_result());
    });
}

fn exit_info(status: &ExitStatus, stderr_tail: String, reason: Option<String>) -> ExitInfo {
    #[cfg(unix)]
    let signal = std::os::unix::process::ExitStatusExt::signal(status);
    #[cfg(not(unix))]
    let signal = None;
    ExitInfo { code: status.code(), signal, success: status.success(), stderr_tail, reason }
}

async fn supervise(
    mut child: tokio::process::Child,
    inner: Arc<Inner>,
    mut kill_rx: mpsc::UnboundedReceiver<()>,
    reader: JoinHandle<()>,
    stderr_task: JoinHandle<()>,
    exit_tx: watch::Sender<Option<Arc<ExitInfo>>>,
    events: mpsc::UnboundedSender<AcpEvent>,
) {
    let status = tokio::select! {
        status = child.wait() => status,
        _ = kill_rx.recv() => {
            let _ = child.kill().await;
            child.wait().await
        }
    };
    // A grandchild can keep the pipes open; do not wait for it for long.
    let _ = tokio::time::timeout(Duration::from_millis(500), reader).await;
    let _ = tokio::time::timeout(Duration::from_millis(500), stderr_task).await;
    let reason = inner.fatal.lock().unwrap().clone();
    let info = Arc::new(match status {
        Ok(status) => exit_info(&status, inner.stderr.tail(), reason),
        Err(e) => ExitInfo { code: None, signal: None, success: false, stderr_tail: inner.stderr.tail(), reason: Some(format!("cannot wait for the agent: {e}")) },
    });
    {
        let mut pending = inner.pending.lock().unwrap();
        pending.closed = Some(Arc::clone(&info));
        for (_, tx) in pending.map.drain() {
            let _ = tx.send(Err(AcpError::Exited(Arc::clone(&info))));
        }
    }
    let _ = exit_tx.send(Some(Arc::clone(&info)));
    let _ = events.send(AcpEvent::Exited(info));
}
