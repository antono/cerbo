//! The ACP client against the scripted fake agent (`fake-acp-agent`), and
//! against recorded logs replayed by it.

use cerboid::acp::replay::{self, Dir};
use cerboid::acp::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::process::Command;

const FAKE: &str = env!("CARGO_BIN_EXE_fake-acp-agent");

struct Env {
    dir: TempDir,
}

impl Env {
    fn new() -> Self {
        Self { dir: tempfile::tempdir().unwrap() }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// The vault stand-in: the session directory.
    fn cwd(&self) -> PathBuf {
        let p = self.path("cwd");
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn log(&self) -> PathBuf {
        self.path("received.log")
    }

    /// The lines the fake agent received from the client, as JSON.
    fn received(&self) -> Vec<Value> {
        std::fs::read_to_string(self.log())
            .unwrap_or_default()
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }

    fn command(&self, script: Option<Value>) -> Command {
        let mut cmd = Command::new(FAKE);
        if let Some(script) = script {
            let path = self.path("script.json");
            std::fs::write(&path, script.to_string()).unwrap();
            cmd.env("FAKE_ACP_SCRIPT", path);
        }
        cmd.env("FAKE_ACP_LOG", self.log());
        cmd
    }

    fn start(&self, script: Value) -> (Arc<AcpClient>, AcpEvents) {
        self.start_with(script, ClientOptions::default())
    }

    fn start_with(&self, script: Value, opts: ClientOptions) -> (Arc<AcpClient>, AcpEvents) {
        let (client, events) = AcpClient::spawn(self.command(Some(script)), opts).unwrap();
        (Arc::new(client), events)
    }

    /// A client already initialized with a session.
    async fn session(&self, script: Value) -> (Arc<AcpClient>, AcpEvents, String) {
        let (client, events) = self.start(script);
        client.initialize().await.unwrap();
        let s = client.new_session(&self.cwd(), &[]).await.unwrap();
        (client, events, s.session_id)
    }
}

fn drain(events: &mut AcpEvents) -> Vec<AcpEvent> {
    let mut out = Vec::new();
    while let Ok(e) = events.try_recv() {
        out.push(e);
    }
    out
}

/// The text of the agent message chunks, joined.
fn message_text(events: &[AcpEvent]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            AcpEvent::Update(SessionNotification { update: SessionUpdate::AgentMessageChunk { content, .. }, .. }) => content.as_text(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

async fn next_permission(events: &mut AcpEvents) -> PermissionRequest {
    loop {
        match tokio::time::timeout(Duration::from_secs(10), events.recv()).await.expect("a permission request in time") {
            Some(AcpEvent::Permission(p)) => return p,
            Some(_) => continue,
            None => panic!("the stream ended before a permission request"),
        }
    }
}

fn prompt(client: &Arc<AcpClient>, sid: &str, text: &str) -> tokio::task::JoinHandle<Result<StopReason, AcpError>> {
    let client = Arc::clone(client);
    let sid = sid.to_string();
    let text = text.to_string();
    tokio::spawn(async move { client.prompt(&sid, &[ContentBlock::text(text)]).await })
}

fn script(turns: Value) -> Value {
    json!({ "turns": turns })
}

// ---------------------------------------------------------------------------
// Handshake and sessions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn initialize_sends_version_one_and_no_fs_or_terminal() {
    let env = Env::new();
    let (client, _events) = env.start(json!({"agent_info": {"name": "scripted", "version": "9"}, "load_session": true}));
    let init = client.initialize().await.unwrap();
    assert_eq!(init.protocol_version, 1);
    assert!(init.capabilities.load_session);
    assert_eq!(init.agent_info.unwrap().name, "scripted");

    let sent = env.received();
    let req = &sent[0];
    assert_eq!(req["method"], "initialize");
    assert_eq!(req["params"]["protocolVersion"], 1);
    assert_eq!(req["params"]["clientInfo"]["name"], "cerbo");
    let caps = &req["params"]["clientCapabilities"];
    assert!(caps.get("fs").is_none(), "no fs capability: {caps}");
    assert!(caps.get("terminal").is_none(), "no terminal capability: {caps}");
    client.shutdown().await;
}

#[tokio::test]
async fn a_different_protocol_version_ends_the_agent() {
    let env = Env::new();
    let (client, _events) = env.start(json!({"protocol_version": 2}));
    match client.initialize().await {
        Err(AcpError::UnsupportedVersion(2)) => {}
        other => panic!("{other:?}"),
    }
    let exit = tokio::time::timeout(Duration::from_secs(5), client.wait_exit()).await.expect("the agent is killed");
    assert!(!exit.success);
}

#[tokio::test]
async fn new_session_sends_cwd_and_a_stdio_mcp_entry_without_type() {
    let env = Env::new();
    let (client, _events) = env.start(json!({"config_options": [{"id": "mode"}]}));
    client.initialize().await.unwrap();
    let server = McpServer::stdio("cerbo", "/bin/cerbo", vec!["mcp".into(), "--vault".into(), "/v".into()], vec![("K".into(), "V".into())]);
    let s = client.new_session(&env.cwd(), &[server]).await.unwrap();
    assert!(s.session_id.starts_with("fake-session-"));
    assert_eq!(s.config_options.unwrap(), json!([{"id": "mode"}]));

    let sent = env.received();
    let params = &sent[1]["params"];
    assert_eq!(params["cwd"], env.cwd().to_str().unwrap());
    assert_eq!(params["mcpServers"], json!([{"name": "cerbo", "command": "/bin/cerbo", "args": ["mcp", "--vault", "/v"], "env": [{"name": "K", "value": "V"}]}]));

    // The spec wants an absolute path.
    assert!(matches!(client.new_session(Path::new("relative"), &[]).await, Err(AcpError::Protocol(_))));
    client.shutdown().await;
}

#[tokio::test]
async fn load_session_needs_the_capability() {
    let env = Env::new();
    let (client, _events) = env.start(json!({}));
    client.initialize().await.unwrap();
    match client.load_session("x", &env.cwd(), &[]).await {
        Err(AcpError::Unsupported(_)) => {}
        other => panic!("{other:?}"),
    }
    assert!(env.received().iter().all(|m| m["method"] != "session/load"), "nothing was sent");
    client.shutdown().await;
}

#[tokio::test]
async fn load_session_replays_the_history_as_updates() {
    let env = Env::new();
    let (client, mut events) = env.start(json!({
        "load_session": true,
        "known_sessions": ["old"],
        "history": [{"role": "user", "text": "hi"}, {"role": "agent", "text": "hello"}],
    }));
    client.initialize().await.unwrap();
    client.load_session("old", &env.cwd(), &[]).await.unwrap();
    let updates: Vec<_> = drain(&mut events)
        .into_iter()
        .filter_map(|e| match e {
            AcpEvent::Update(n) => Some(n.update),
            _ => None,
        })
        .collect();
    assert_eq!(
        updates,
        vec![
            SessionUpdate::UserMessageChunk { content: ContentBlock::text("hi"), message_id: None },
            SessionUpdate::AgentMessageChunk { content: ContentBlock::text("hello"), message_id: None },
        ]
    );
    match client.load_session("nope", &env.cwd(), &[]).await {
        Err(AcpError::Rpc { code: -32602, .. }) => {}
        other => panic!("{other:?}"),
    }
    client.shutdown().await;
}

#[tokio::test]
async fn blocks_the_agent_did_not_advertise_are_not_sent() {
    let env = Env::new();
    let (client, _events, sid) = env.session(json!({})).await;
    let image = ContentBlock::Image { data: "AAAA".into(), mime_type: "image/png".into(), uri: None };
    assert!(matches!(client.prompt(&sid, &[image]).await, Err(AcpError::Unsupported(_))));
    let embedded = ContentBlock::Resource { resource: ResourceContents::Text { uri: "file:///a".into(), text: "a".into(), mime_type: None } };
    assert!(matches!(client.prompt(&sid, &[embedded]).await, Err(AcpError::Unsupported(_))));
    // Text and resource links always go.
    let link = ContentBlock::resource_link("a", "cerbo://page/a");
    client.prompt(&sid, &[ContentBlock::text("see"), link]).await.unwrap();
    let sent = env.received();
    let prompt = sent.iter().rev().find(|m| m["method"] == "session/prompt").unwrap();
    assert_eq!(prompt["params"]["prompt"][1], json!({"type": "resource_link", "name": "a", "uri": "cerbo://page/a"}));
    client.shutdown().await;
}

// ---------------------------------------------------------------------------
// Turns and updates
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_turn_streams_typed_updates_in_order_before_prompt_returns() {
    let env = Env::new();
    let (client, mut events, sid) = env
        .session(script(json!([[
            {"do": "thought", "text": "hmm"},
            {"do": "plan", "entries": [{"content": "step", "priority": "high", "status": "pending"}]},
            {"do": "tool_call", "id": "t1", "title": "Read a", "kind": "read", "status": "pending", "raw_input": {"p": 1}},
            {"do": "tool_update", "id": "t1", "status": "completed", "text": "body", "raw_output": {"n": 2}},
            {"do": "raw", "line": "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionId\":\"S\",\"update\":{\"sessionUpdate\":\"usage_update\",\"used\":1,\"size\":2}}}"},
            {"do": "text", "text": "Hello, world", "chunk_size": 5},
        ]])))
        .await;
    let stop = client.prompt(&sid, &[ContentBlock::text("go")]).await.unwrap();
    assert_eq!(stop, StopReason::EndTurn);

    // Everything is in the stream already.
    let events = drain(&mut events);
    let updates: Vec<&SessionUpdate> = events
        .iter()
        .filter_map(|e| match e {
            AcpEvent::Update(n) => {
                assert!(n.session_id == sid || n.session_id == "S");
                Some(&n.update)
            }
            _ => None,
        })
        .collect();
    assert!(matches!(updates[0], SessionUpdate::AgentThoughtChunk { content, .. } if content.as_text() == Some("hmm")));
    match updates[1] {
        SessionUpdate::Plan { entries } => {
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].priority, PlanPriority::High);
            assert_eq!(entries[0].status, PlanStatus::Pending);
        }
        other => panic!("{other:?}"),
    }
    match updates[2] {
        SessionUpdate::ToolCall(c) => {
            assert_eq!(c.tool_call_id, "t1");
            assert_eq!(c.title, "Read a");
            assert_eq!(c.kind, Some(ToolKind::Read));
            assert_eq!(c.status, Some(ToolStatus::Pending));
            assert_eq!(c.raw_input, Some(json!({"p": 1})));
        }
        other => panic!("{other:?}"),
    }
    match updates[3] {
        SessionUpdate::ToolCallUpdate(u) => {
            assert_eq!(u.status, Some(ToolStatus::Completed));
            assert_eq!(u.title, None);
            assert_eq!(u.content, Some(vec![ToolCallContent::Content { content: ContentBlock::text("body") }]));
            assert_eq!(u.raw_output, Some(json!({"n": 2})));
        }
        other => panic!("{other:?}"),
    }
    match updates[4] {
        SessionUpdate::Other { kind, raw } => {
            assert_eq!(kind, "usage_update");
            assert_eq!(raw["used"], 1);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(message_text(&events), "Hello, world");
    client.shutdown().await;
}

#[tokio::test]
async fn the_prompt_text_reaches_the_agent() {
    let env = Env::new();
    let (client, mut events, sid) = env.session(json!({})).await;
    client.prompt(&sid, &[ContentBlock::text("one"), ContentBlock::text("two")]).await.unwrap();
    assert_eq!(message_text(&drain(&mut events)), "echo: one\ntwo");
    client.shutdown().await;
}

#[tokio::test]
async fn stop_reasons_map() {
    let env = Env::new();
    let (client, _events, sid) = env.session(script(json!([[{"do": "stop", "reason": "refusal"}], [{"do": "stop", "reason": "something_new"}]]))).await;
    assert_eq!(client.prompt(&sid, &[ContentBlock::text("a")]).await.unwrap(), StopReason::Refusal);
    assert_eq!(client.prompt(&sid, &[ContentBlock::text("b")]).await.unwrap(), StopReason::Other("something_new".into()));
    client.shutdown().await;
}

#[tokio::test]
async fn garbage_lines_are_warnings_not_the_end() {
    let env = Env::new();
    let (client, mut events, sid) = env
        .session(script(json!([[
            {"do": "raw", "line": "this is not json"},
            {"do": "raw", "line": "{\"jsonrpc\":\"2.0\",\"id\":999,\"result\":{}}"},
            {"do": "text", "text": "still here"},
        ]])))
        .await;
    client.prompt(&sid, &[ContentBlock::text("go")]).await.unwrap();
    let events = drain(&mut events);
    assert_eq!(events.iter().filter(|e| matches!(e, AcpEvent::Warning(_))).count(), 2);
    assert_eq!(message_text(&events), "still here");
    client.shutdown().await;
}

// ---------------------------------------------------------------------------
// Permissions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_permission_request_is_answered_with_the_nested_outcome() {
    let env = Env::new();
    let (client, mut events, sid) = env
        .session(script(json!([[{"do": "permission", "title": "Write a", "allow": [{"do": "text", "text": "wrote it"}], "reject": [{"do": "text", "text": "not written"}]}]])))
        .await;
    let turn = prompt(&client, &sid, "go");
    let req = next_permission(&mut events).await;
    assert_eq!(req.session_id, sid);
    assert_eq!(req.tool_call.title.as_deref(), Some("Write a"));
    assert_eq!(req.tool_call.kind, Some(ToolKind::Edit));
    assert_eq!(req.options.len(), 2);
    assert_eq!(req.options[0].kind, PermissionKind::AllowOnce);
    req.select("allow").unwrap();
    assert_eq!(turn.await.unwrap().unwrap(), StopReason::EndTurn);

    let text = message_text(&drain(&mut events));
    assert!(text.contains("permission: selected allow"), "{text}");
    assert!(text.contains("wrote it"));

    // The agent got {"outcome": {"outcome": "selected", "optionId": ...}}: the
    // fake agent reports `malformed` for any other nesting.
    let response = env.received().into_iter().find(|m| m.get("method").is_none() && m.get("result").is_some()).unwrap();
    assert_eq!(response["result"], json!({"outcome": {"outcome": "selected", "optionId": "allow"}}));
    client.shutdown().await;
}

#[tokio::test]
async fn rejecting_runs_the_reject_branch() {
    let env = Env::new();
    let (client, mut events, sid) = env
        .session(script(json!([[{"do": "permission", "allow": [{"do": "text", "text": "ALLOWED"}], "reject": [{"do": "text", "text": "REJECTED"}]}]])))
        .await;
    let turn = prompt(&client, &sid, "go");
    let req = next_permission(&mut events).await;
    let id = req.option_of_kind(PermissionKind::RejectOnce).unwrap();
    req.select(&id).unwrap();
    turn.await.unwrap().unwrap();
    let text = message_text(&drain(&mut events));
    assert!(text.contains("REJECTED") && !text.contains("ALLOWED"), "{text}");
    client.shutdown().await;
}

#[tokio::test]
async fn an_option_that_was_not_offered_is_never_an_approval() {
    let env = Env::new();
    let (client, mut events, sid) = env.session(script(json!([[{"do": "permission", "allow": [{"do": "text", "text": "ALLOWED"}]}]]))).await;
    let turn = prompt(&client, &sid, "go");
    let req = next_permission(&mut events).await;
    assert!(matches!(req.select("hack"), Err(AcpError::Protocol(_))));
    turn.await.unwrap().unwrap();
    let text = message_text(&drain(&mut events));
    assert!(text.contains("permission: cancelled") && !text.contains("ALLOWED"), "{text}");
    client.shutdown().await;
}

#[tokio::test]
async fn a_dropped_permission_request_is_cancelled() {
    let env = Env::new();
    let (client, mut events, sid) = env.session(script(json!([[{"do": "permission", "allow": [{"do": "text", "text": "ALLOWED"}]}]]))).await;
    let turn = prompt(&client, &sid, "go");
    drop(next_permission(&mut events).await);
    turn.await.unwrap().unwrap();
    assert!(message_text(&drain(&mut events)).contains("permission: cancelled"));
    client.shutdown().await;
}

#[tokio::test]
async fn cancel_answers_the_pending_permission_and_ends_the_turn_cancelled() {
    let env = Env::new();
    let (client, mut events, sid) = env
        .session(script(json!([[
            {"do": "permission", "allow": [{"do": "text", "text": "ALLOWED"}], "cancelled": [{"do": "text", "text": "turn cancelled"}]},
            {"do": "text", "text": "NOT REACHED"},
        ]])))
        .await;
    let turn = prompt(&client, &sid, "go");
    let req = next_permission(&mut events).await;
    // The person never answers; the turn is cancelled instead.
    client.cancel(&sid);
    assert_eq!(turn.await.unwrap().unwrap(), StopReason::Cancelled);
    let text = message_text(&drain(&mut events));
    assert!(text.contains("permission: cancelled"), "{text}");
    assert!(!text.contains("NOT REACHED") && !text.contains("ALLOWED"));
    // Answering afterwards is too late, and says so.
    assert!(req.select("allow").is_err());

    let sent = env.received();
    assert!(sent.iter().any(|m| m["method"] == "session/cancel" && m["params"]["sessionId"] == sid.as_str() && m.get("id").is_none()), "cancel is a notification");
    client.shutdown().await;
}

#[tokio::test]
async fn a_permission_request_made_while_the_turn_is_cancelled_is_answered_cancelled_and_never_shown() {
    let env = Env::new();
    // The agent asks only after it has been told to cancel.
    let (client, mut events, sid) = env.session(script(json!([[{"do": "permission", "after_cancel": true, "allow": [{"do": "text", "text": "ALLOWED"}]}], [{"do": "permission"}]]))).await;
    let turn = prompt(&client, &sid, "go");
    tokio::time::sleep(Duration::from_millis(100)).await;
    client.cancel(&sid);
    let stop = tokio::time::timeout(Duration::from_secs(10), turn).await.expect("the agent was answered, so the turn ends").unwrap().unwrap();
    assert_eq!(stop, StopReason::Cancelled);
    let seen = drain(&mut events);
    assert!(!seen.iter().any(|e| matches!(e, AcpEvent::Permission(_))), "no card for a request of a cancelled turn: {seen:?}");
    let text = message_text(&seen);
    assert!(text.contains("permission: cancelled") && !text.contains("ALLOWED"), "{text}");

    // The next turn is not affected by the old cancel.
    let again = prompt(&client, &sid, "again");
    let req = next_permission(&mut events).await;
    assert!(!req.is_cancelled());
    req.cancel();
    again.await.unwrap().unwrap();
    client.shutdown().await;
}

#[tokio::test]
async fn a_request_waiting_in_the_event_stream_when_cancel_runs_is_marked_cancelled() {
    let env = Env::new();
    let (client, mut events, sid) = env
        .session(script(json!([[{"do": "permission", "allow": [{"do": "text", "text": "ALLOWED"}]}, {"do": "text", "text": "x"}]])))
        .await;
    let turn = prompt(&client, &sid, "go");
    let req = next_permission(&mut events).await;
    assert!(!req.is_cancelled());
    client.cancel(&sid);
    // A reader that only now looks at the request (the driver behind a busy
    // queue) must see that it is stale, with no await between cancel and look.
    assert!(req.is_cancelled());
    assert_eq!(turn.await.unwrap().unwrap(), StopReason::Cancelled);
    client.shutdown().await;
}

#[tokio::test]
async fn cancel_ends_a_running_turn() {
    let env = Env::new();
    let (client, _events, sid) = env.session(script(json!([[{"do": "wait_cancel"}, {"do": "text", "text": "x"}]]))).await;
    let turn = prompt(&client, &sid, "go");
    tokio::time::sleep(Duration::from_millis(100)).await;
    client.cancel(&sid);
    assert_eq!(turn.await.unwrap().unwrap(), StopReason::Cancelled);
    // The session is usable afterwards.
    client.shutdown().await;
}

// ---------------------------------------------------------------------------
// Requests the client does not serve
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fs_terminal_and_elicitation_requests_get_method_not_found() {
    let env = Env::new();
    let (client, mut events, sid) = env
        .session(script(json!([[
            {"do": "call_client", "method": "fs/read_text_file", "params": {"sessionId": "S", "path": "/etc/passwd"}},
            {"do": "call_client", "method": "fs/write_text_file", "params": {"sessionId": "S", "path": "/x", "content": "y"}},
            {"do": "call_client", "method": "terminal/create", "params": {"sessionId": "S", "command": "sh"}},
            {"do": "call_client", "method": "elicitation/create", "params": {}},
        ]])))
        .await;
    client.prompt(&sid, &[ContentBlock::text("go")]).await.unwrap();
    let events = drain(&mut events);
    let text = message_text(&events);
    for m in ["fs/read_text_file", "fs/write_text_file", "terminal/create", "elicitation/create"] {
        assert!(text.contains(&format!("client_call {m}: error -32601")), "{m}: {text}");
    }
    let refused: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            AcpEvent::Refused { method } => Some(method.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(refused, ["fs/read_text_file", "fs/write_text_file", "terminal/create", "elicitation/create"]);
    client.shutdown().await;
}

// ---------------------------------------------------------------------------
// MCP and direct writes (the sandbox tests use these steps later)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_agent_calls_an_mcp_tool_through_the_stdio_entry_it_was_given() {
    let env = Env::new();
    let (client, mut events) = env.start(script(json!([[
        {"do": "mcp_call", "tool": "echo", "arguments": {"page": "a"}},
        {"do": "mcp_call", "tool": "fail"},
    ]])));
    client.initialize().await.unwrap();
    let server = McpServer::stdio("cerbo", FAKE, vec!["mcp-echo".into()], vec![("FAKE_MCP_TAG".into(), "t137".into())]);
    let sid = client.new_session(&env.cwd(), &[server]).await.unwrap().session_id;
    client.prompt(&sid, &[ContentBlock::text("go")]).await.unwrap();

    let events = drain(&mut events);
    let text = message_text(&events);
    assert!(text.contains(r#"mcp_result: echo[t137]:{"page":"a"}"#), "{text}");
    assert!(text.contains("mcp_result: failed on purpose"), "{text}");
    let statuses: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            AcpEvent::Update(SessionNotification { update: SessionUpdate::ToolCallUpdate(u), .. }) => u.status.clone(),
            _ => None,
        })
        .collect();
    assert_eq!(statuses, [ToolStatus::Completed, ToolStatus::Failed]);
    client.shutdown().await;
}

#[tokio::test]
async fn a_missing_mcp_server_is_reported_by_the_agent() {
    let env = Env::new();
    let (client, mut events, sid) = env.session(script(json!([[{"do": "mcp_call", "tool": "echo", "server": "nope"}]]))).await;
    client.prompt(&sid, &[ContentBlock::text("go")]).await.unwrap();
    assert!(message_text(&drain(&mut events)).contains("no mcpServers entry named `nope`"));
    client.shutdown().await;
}

#[tokio::test]
async fn a_direct_write_into_the_cwd_is_reported() {
    let env = Env::new();
    let (client, mut events, sid) = env
        .session(script(json!([[
            {"do": "write_file", "path": "note.md", "contents": "hi"},
            {"do": "write_file", "path": "no-such-dir/x.md"},
        ]])))
        .await;
    client.prompt(&sid, &[ContentBlock::text("go")]).await.unwrap();
    let text = message_text(&drain(&mut events));
    assert!(text.contains("write_file note.md: ok"), "{text}");
    assert!(text.contains("write_file no-such-dir/x.md: error:"), "{text}");
    assert_eq!(std::fs::read_to_string(env.cwd().join("note.md")).unwrap(), "hi");
    client.shutdown().await;
}

// ---------------------------------------------------------------------------
// Process supervision
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_crash_reports_the_exit_and_the_stderr_tail_and_fails_the_prompt() {
    let env = Env::new();
    let (client, mut events, sid) = env
        .session(script(json!([[
            {"do": "text", "text": "before"},
            {"do": "crash", "stderr": "thread panicked: boom", "code": 3},
        ]])))
        .await;
    let err = client.prompt(&sid, &[ContentBlock::text("go")]).await.unwrap_err();
    let AcpError::Exited(info) = err else { panic!("{err:?}") };
    assert_eq!(info.code, Some(3));
    assert!(!info.success);
    assert!(info.stderr_tail.contains("thread panicked: boom"), "{:?}", info.stderr_tail);
    assert!(info.to_string().contains("exited with code 3"));
    assert!(client.stderr_tail().contains("boom"));

    // The stream: the update first, the exit last, then it ends.
    let mut seen = Vec::new();
    while let Some(e) = events.recv().await {
        seen.push(e);
    }
    assert!(matches!(seen.last(), Some(AcpEvent::Exited(i)) if i.code == Some(3)));
    assert_eq!(message_text(&seen), "before");
    // Later calls fail the same way.
    assert!(matches!(client.new_session(&env.cwd(), &[]).await, Err(AcpError::Exited(_))));
}

#[tokio::test]
async fn a_program_that_does_not_exist_does_not_spawn() {
    match AcpClient::spawn_argv(&["/nonexistent/agent".to_string()], &[], None, ClientOptions::default()) {
        Err(AcpError::Spawn(_)) => {}
        Ok(_) => panic!("spawned"),
        Err(e) => panic!("{e:?}"),
    }
    assert!(matches!(AcpClient::spawn_argv(&[], &[], None, ClientOptions::default()), Err(AcpError::Spawn(_))));
}

#[tokio::test]
async fn spawn_argv_passes_env_and_cwd() {
    let env = Env::new();
    // `sh -c` is the agent: it prints what it got to stderr and exits.
    let argv = vec!["/bin/sh".to_string(), "-c".to_string(), "echo \"v=$FAKE_VAR d=$(pwd)\" >&2".to_string()];
    let cwd = env.cwd();
    let (client, _events) = AcpClient::spawn_argv(&argv, &[("FAKE_VAR".into(), "137".into())], Some(&cwd), ClientOptions::default()).unwrap();
    let exit = client.wait_exit().await;
    assert!(exit.success);
    assert!(exit.stderr_tail.contains("v=137"), "{}", exit.stderr_tail);
    assert!(exit.stderr_tail.contains(cwd.file_name().unwrap().to_str().unwrap()));
}

#[tokio::test]
async fn stderr_is_kept_in_a_bounded_tail() {
    let env = Env::new();
    let opts = ClientOptions { stderr_tail_bytes: 100, ..ClientOptions::default() };
    let big = "e".repeat(5000);
    let (client, _events) = env.start_with(script(json!([[{"do": "stderr", "text": big}, {"do": "stderr", "text": "THE-END"}, {"do": "crash"}]])), opts);
    client.initialize().await.unwrap();
    let sid = client.new_session(&env.cwd(), &[]).await.unwrap().session_id;
    let _ = client.prompt(&sid, &[ContentBlock::text("go")]).await;
    let exit = client.wait_exit().await;
    assert!(exit.stderr_tail.len() <= 100, "{}", exit.stderr_tail.len());
    assert!(exit.stderr_tail.trim_end().ends_with("THE-END"));
}

#[tokio::test]
async fn a_request_that_is_not_answered_times_out() {
    let env = Env::new();
    let opts = ClientOptions { request_timeout: Duration::from_millis(200), ..ClientOptions::default() };
    let (client, _events) = env.start_with(json!({"init_delay_ms": 3000}), opts);
    match client.initialize().await {
        Err(AcpError::Timeout { method, .. }) => assert_eq!(method, "initialize"),
        other => panic!("{other:?}"),
    }
    client.kill().await;
}

#[tokio::test]
async fn a_prompt_can_have_a_time_limit() {
    let env = Env::new();
    let opts = ClientOptions { prompt_timeout: Some(Duration::from_millis(200)), ..ClientOptions::default() };
    let (client, _events) = env.start_with(script(json!([[{"do": "hang"}]])), opts);
    client.initialize().await.unwrap();
    let sid = client.new_session(&env.cwd(), &[]).await.unwrap().session_id;
    assert!(matches!(client.prompt(&sid, &[ContentBlock::text("x")]).await, Err(AcpError::Timeout { .. })));
    client.kill().await;
}

#[tokio::test]
async fn a_line_over_the_limit_ends_the_agent() {
    let env = Env::new();
    let opts = ClientOptions { max_line_bytes: 1000, ..ClientOptions::default() };
    let (client, _events) = env.start_with(script(json!([[{"do": "big_line", "bytes": 5000}, {"do": "text", "text": "never read"}]])), opts);
    client.initialize().await.unwrap();
    let sid = client.new_session(&env.cwd(), &[]).await.unwrap().session_id;
    let err = client.prompt(&sid, &[ContentBlock::text("go")]).await.unwrap_err();
    let AcpError::Exited(info) = err else { panic!("{err:?}") };
    assert!(info.reason.as_deref().unwrap_or("").contains("over 1000 bytes"), "{info:?}");
}

#[tokio::test]
async fn shutdown_closes_stdin_and_the_agent_leaves_by_itself() {
    let env = Env::new();
    let (client, _events, _sid) = env.session(json!({})).await;
    let exit = client.shutdown().await;
    assert!(exit.success, "{exit}");
    assert_eq!(exit.code, Some(0));
    assert!(exit.reason.is_none());
}

#[tokio::test]
async fn shutdown_kills_an_agent_that_ignores_the_closed_stdin() {
    let env = Env::new();
    // `sleep` never reads stdin.
    let mut cmd = Command::new("/bin/sh");
    cmd.args(["-c", "exec sleep 600"]);
    let opts = ClientOptions { shutdown_grace: Duration::from_millis(200), ..ClientOptions::default() };
    let (client, _events) = AcpClient::spawn(cmd, opts).unwrap();
    let started = std::time::Instant::now();
    let exit = client.shutdown().await;
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(!exit.success);
    assert_eq!(exit.signal, Some(9));
    drop(env);
}

#[tokio::test]
async fn dropping_the_client_stops_the_agent() {
    let (client, events) = AcpClient::spawn(
        {
            let mut cmd = Command::new("/bin/sh");
            cmd.args(["-c", "echo $$ > /dev/null; exec sleep 600"]);
            cmd
        },
        ClientOptions::default(),
    )
    .unwrap();
    let pid = client.pid().unwrap();
    drop(client);
    let mut events = events;
    let last = tokio::time::timeout(Duration::from_secs(5), async {
        let mut last = None;
        while let Some(e) = events.recv().await {
            last = Some(e);
        }
        last
    })
    .await
    .expect("the agent ended");
    assert!(matches!(last, Some(AcpEvent::Exited(_))));
    assert!(!Path::new(&format!("/proc/{pid}")).exists() || std::fs::read_to_string(format!("/proc/{pid}/stat")).map(|s| s.contains(") Z")).unwrap_or(true));
}

// ---------------------------------------------------------------------------
// Record and replay (tasks 2.4)
// ---------------------------------------------------------------------------

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/acp").join(name)
}

fn replay_command(log: &Path) -> Command {
    let mut cmd = Command::new(FAKE);
    cmd.env("FAKE_ACP_REPLAY", log);
    cmd
}

fn replay_opts(record: &Path) -> ClientOptions {
    ClientOptions { client_version: "test".into(), record: Some(record.to_path_buf()), ..ClientOptions::default() }
}

#[test]
fn the_log_maps_to_typed_updates_without_any_process() {
    let text = std::fs::read_to_string(fixture("turn-with-tools-and-permission.jsonl")).unwrap();
    let entries = replay::parse_log(&text).unwrap();
    let updates = replay::updates(&entries);
    let kinds: Vec<&str> = updates
        .iter()
        .map(|n| match &n.update {
            SessionUpdate::Other { kind, .. } => kind.as_str(),
            SessionUpdate::AgentThoughtChunk { .. } => "thought",
            SessionUpdate::Plan { .. } => "plan",
            SessionUpdate::ToolCall(_) => "tool_call",
            SessionUpdate::ToolCallUpdate(_) => "tool_call_update",
            SessionUpdate::AgentMessageChunk { .. } => "message",
            SessionUpdate::UserMessageChunk { .. } => "user",
        })
        .collect();
    assert_eq!(kinds, ["available_commands_update", "thought", "plan", "tool_call", "tool_call_update", "tool_call_update", "message", "message", "usage_update"]);
    // A diff in a tool call update.
    assert!(matches!(
        &updates[5].update,
        SessionUpdate::ToolCallUpdate(u) if matches!(&u.content.as_ref().unwrap()[0], ToolCallContent::Diff { new_text, .. } if new_text == "# Alpha")
    ));
    // The location of the first tool call.
    assert!(matches!(&updates[3].update, SessionUpdate::ToolCall(c) if c.locations[0].path == "/vault/a.md" && c.locations[0].line == Some(1)));
}

#[tokio::test]
async fn a_recorded_turn_replays_through_the_client() {
    let env = Env::new();
    let record = env.path("out.jsonl");
    let (client, mut events) = AcpClient::spawn(replay_command(&fixture("turn-with-tools-and-permission.jsonl")), replay_opts(&record)).unwrap();
    let client = Arc::new(client);
    let init = client.initialize().await.unwrap();
    assert!(init.capabilities.load_session && init.capabilities.prompt.image && init.capabilities.mcp.http);
    let server = McpServer::stdio("cerbo", "/bin/cerbo", vec!["mcp".into(), "--vault".into(), "/vault".into()], vec![]);
    let session = client.new_session(Path::new("/vault"), &[server]).await.unwrap();
    assert_eq!(session.session_id, "ses_1");
    let turn = {
        let (client, sid) = (Arc::clone(&client), session.session_id.clone());
        tokio::spawn(async move { client.prompt(&sid, &[ContentBlock::text("tidy page A")]).await })
    };
    let req = next_permission(&mut events).await;
    assert_eq!(req.tool_call.tool_call_id, "call_2");
    assert!(matches!(&req.tool_call.content.as_ref().unwrap()[0], ToolCallContent::Diff { old_text: Some(o), .. } if o == "# A"));
    let id = req.option_of_kind(PermissionKind::AllowOnce).unwrap();
    req.select(&id).unwrap();
    assert_eq!(turn.await.unwrap().unwrap(), StopReason::EndTurn);

    let events = drain(&mut events);
    assert_eq!(message_text(&events), "Done: heading fixed.");
    let first = events.iter().find_map(|e| match e {
        AcpEvent::Update(SessionNotification { update: SessionUpdate::AgentMessageChunk { message_id, .. }, .. }) => message_id.clone(),
        _ => None,
    });
    assert_eq!(first.as_deref(), Some("m1"));

    // What the client sent is exactly what the log says it should send.
    let exit = client.shutdown().await;
    assert!(exit.success, "the replay agent saw a mismatch: {exit}");
    let expected = replay::parse_log(&std::fs::read_to_string(fixture("turn-with-tools-and-permission.jsonl")).unwrap()).unwrap();
    let got = replay::parse_log(&std::fs::read_to_string(&record).unwrap()).unwrap();
    let out = |v: &[replay::LogEntry]| v.iter().filter(|e| e.dir == Dir::Out).map(|e| e.msg.clone()).collect::<Vec<_>>();
    assert_eq!(out(&got), out(&expected));
}

#[tokio::test]
async fn the_opencode_handshake_replays() {
    let env = Env::new();
    let record = env.path("out.jsonl");
    let (client, mut events) = AcpClient::spawn(replay_command(&fixture("opencode-2.2.6-handshake.jsonl")), replay_opts(&record)).unwrap();
    let init = client.initialize().await.unwrap();
    assert_eq!(init.agent_info.as_ref().unwrap().name, "OpenCode");
    assert!(init.capabilities.load_session && init.capabilities.prompt.embedded_context);
    assert_eq!(init.capabilities.session_capabilities["resume"], json!({}));
    assert_eq!(init.auth_methods[0]["id"], "opencode-login");
    let s = client.new_session(Path::new("/vault"), &[]).await.unwrap();
    assert!(s.session_id.starts_with("ses_"));
    assert_eq!(s.config_options.unwrap().as_array().unwrap().len(), 3);
    client.load_session(&s.session_id, Path::new("/vault"), &[]).await.unwrap();
    let kinds: Vec<_> = drain(&mut events)
        .into_iter()
        .filter_map(|e| match e {
            AcpEvent::Update(SessionNotification { update: SessionUpdate::Other { kind, .. }, .. }) => Some(kind),
            _ => None,
        })
        .collect();
    assert_eq!(kinds, ["available_commands_update"]);
    assert!(client.shutdown().await.success);
}

/// The cerbo MCP entry as the daemon builds it, with the paths the fixtures use.
fn fixture_mcp() -> McpServer {
    McpServer::stdio(
        "cerbo",
        "/nix/store/cerbo/bin/cerbo",
        vec!["mcp".into(), "--vault".into(), "/vault".into()],
        vec![("CERBO_SOCKET".into(), "/run/cerbo/session/agent.sock".into())],
    )
}

/// The recorded session of the real Claude adapter (0.106.0): what the daemon
/// sends on `session/new` is exactly what that adapter was started with, and the
/// start mode the vault's settings picked is put back.
#[tokio::test]
async fn the_real_claude_adapter_session_replays_with_the_locked_down_meta_and_the_mode_pin() {
    let env = Env::new();
    let record = env.path("out.jsonl");
    let log = fixture("claude-agent-acp-0.106.0-session.jsonl");
    let (client, _events) = AcpClient::spawn(replay_command(&log), replay_opts(&record)).unwrap();
    let init = client.initialize().await.unwrap();
    assert_eq!(init.agent_info.as_ref().unwrap().name, "@agentclientprotocol/claude-agent-acp");
    assert!(init.capabilities.load_session && init.capabilities.prompt.embedded_context && init.capabilities.mcp.http);
    let meta = cerboid::sandbox::claude_session_meta();
    let session = client.new_session_with_meta(Path::new("/vault"), &[fixture_mcp()], Some(&meta)).await.unwrap();
    // The adapter chose `auto` itself, because the vault's settings asked for bypassPermissions.
    assert_eq!(session.modes.as_ref().unwrap()["currentModeId"], "auto");
    let changed = client.pin_mode(&session.session_id, session.modes.as_ref(), cerboid::sandbox::START_MODE).await.unwrap();
    assert!(changed, "the start mode was put back");
    let exit = client.shutdown().await;
    assert!(exit.success, "what the client sent differs from what the real adapter was sent: {exit}");
    // The meta in the recording is the one the daemon sends today.
    let expected = replay::parse_log(&std::fs::read_to_string(&log).unwrap()).unwrap();
    let sent_meta = expected.iter().find(|e| e.msg["method"] == "session/new").unwrap().msg["params"]["_meta"].clone();
    assert_eq!(sent_meta, meta);
    assert_eq!(sent_meta["claudeCode"]["options"]["strictMcpConfig"], true);
}

#[tokio::test]
async fn the_real_opencode_session_with_the_cerbo_server_replays() {
    let env = Env::new();
    let record = env.path("out.jsonl");
    let (client, mut events) = AcpClient::spawn(replay_command(&fixture("opencode-2.2.6-session.jsonl")), replay_opts(&record)).unwrap();
    client.initialize().await.unwrap();
    let session = client.new_session(Path::new("/vault"), &[fixture_mcp()]).await.unwrap();
    assert!(session.session_id.starts_with("ses_"));
    assert!(session.modes.is_none(), "OpenCode has config options, not modes");
    // No modes, so nothing to pin.
    assert!(!client.pin_mode(&session.session_id, session.modes.as_ref(), cerboid::sandbox::START_MODE).await.unwrap());
    // Give the command list that follows session/new a moment to arrive.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!drain(&mut events).is_empty(), "the command list arrives after session/new");
    assert!(client.shutdown().await.success);
}

#[tokio::test]
async fn the_start_mode_is_pinned_only_when_the_agent_is_somewhere_else() {
    let modes = |current: &str| json!({"currentModeId": current, "availableModes": [{"id": "default"}, {"id": "auto"}]});
    let scripted = |current: &str| json!({"modes": modes(current), "turns": [[{"do": "text", "text": "x"}]]});

    // Started in `auto`: put into `default`.
    let env = Env::new();
    let (client, _events) = env.start(scripted("auto"));
    client.initialize().await.unwrap();
    let started = client.new_session(&env.cwd(), &[]).await.unwrap();
    assert_eq!(started.modes.as_ref().unwrap()["currentModeId"], "auto");
    assert!(client.pin_mode(&started.session_id, started.modes.as_ref(), "default").await.unwrap());
    assert!(env.received().iter().any(|m| m["method"] == "session/set_mode" && m["params"]["modeId"] == "default" && m["params"]["sessionId"] == started.session_id.as_str()));

    // Already there, or no modes at all: nothing is sent.
    let env2 = Env::new();
    let (client2, _events2, sid2) = env2.session(scripted("default")).await;
    assert!(!client2.pin_mode(&sid2, Some(&modes("default")), "default").await.unwrap());
    assert!(!client2.pin_mode(&sid2, None, "default").await.unwrap());
    assert!(!env2.received().iter().any(|m| m["method"] == "session/set_mode"));

    // A mode the agent does not offer is an error, not a silent skip.
    let err = client2.pin_mode(&sid2, Some(&json!({"currentModeId": "auto", "availableModes": [{"id": "auto"}]})), "default").await.unwrap_err();
    assert!(matches!(err, AcpError::Unsupported(_)), "{err}");
    client.shutdown().await;
    client2.shutdown().await;
}

#[tokio::test]
async fn a_live_session_records_a_log_that_replays() {
    // Record a live turn against the scripted agent, then replay the recording.
    let env = Env::new();
    let record = env.path("rec.jsonl");
    let opts = ClientOptions { client_version: "test".into(), record: Some(record.clone()), ..ClientOptions::default() };
    let (client, _events) = env.start_with(
        script(json!([[{"do": "tool_call", "id": "t", "kind": "read"}, {"do": "text", "text": "recorded"}]])),
        opts,
    );
    client.initialize().await.unwrap();
    let sid = client.new_session(&env.cwd(), &[]).await.unwrap().session_id;
    client.prompt(&sid, &[ContentBlock::text("rec")]).await.unwrap();
    assert!(client.shutdown().await.success);

    let log = replay::parse_log(&std::fs::read_to_string(&record).unwrap()).unwrap();
    assert_eq!(log[0].dir, Dir::Out);
    assert_eq!(log[0].msg["method"], "initialize");
    assert_eq!(replay::updates(&log).len(), 2);

    let record2 = env.path("rec2.jsonl");
    let (client, mut events) = AcpClient::spawn(replay_command(&record), replay_opts(&record2)).unwrap();
    client.initialize().await.unwrap();
    let sid2 = client.new_session(&env.cwd(), &[]).await.unwrap().session_id;
    assert_eq!(sid2, sid);
    assert_eq!(client.prompt(&sid2, &[ContentBlock::text("rec")]).await.unwrap(), StopReason::EndTurn);
    assert_eq!(message_text(&drain(&mut events)), "recorded");
    assert!(client.shutdown().await.success);
}
