//! Agent sessions end to end: a real daemon, the scripted fake ACP agent
//! (`fake-acp-agent`) and the daemon's own socket. The agent runs without the
//! sandbox (`sandbox = "none"`) except in the one test that needs a real
//! bubblewrap, which is skipped, with a message, where there is none.
//!
//! The fake agent finds its script in its home, `fake-acp-script.json`, because
//! the daemon starts agents with a cleared environment.

use cerbo_core::agent::{self, AgentPermission};
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::client::Client;
use cerbo_core::rpc::{
    AgentInfo, AgentKind, ApprovalInfo, ApprovalOutcome, Call, ClientKind, Event, ResumeHow, SessionEvent, SessionEventBody, SessionInfo,
    SessionState, CODE_FAILED, CODE_FORBIDDEN, CODE_READ_ONLY, CODE_REJECTED,
};
use cerbo_core::semantic::{LazyEmbedder, MockEmbedder};
use cerbo_core::CerboContext;
use cerboid::agents::HostEnv;
use cerboid::sessions::Settings;
use cerboid::{Config, Running};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const FAKE: &str = env!("CARGO_BIN_EXE_fake-acp-agent");

struct Env {
    tmp: tempfile::TempDir,
    ctx: CerboContext,
    paths: DaemonPaths,
    data: PathBuf,
    bin_dir: PathBuf,
    sandbox_none: bool,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = CerboContext { config_dir: tmp.path().join("config"), cache_dir: tmp.path().join("cache") };
    let paths = DaemonPaths::in_dir(tmp.path().join("run").join("cerbo"));
    let data = tmp.path().join("data");
    let bin_dir = tmp.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let env = Env { tmp, ctx, paths, data, bin_dir, sandbox_none: true };
    env.write_agents_toml(Path::new(FAKE));
    env.login();
    env
}

impl Env {
    /// `agents.toml` with OpenCode pointing at `agent` and the sandbox off.
    fn write_agents_toml(&self, agent: &Path) {
        std::fs::create_dir_all(&self.ctx.config_dir).unwrap();
        let sandbox = if self.sandbox_none { "sandbox = \"none\"\n" } else { "" };
        std::fs::write(self.ctx.config_dir.join("agents.toml"), format!("[agent]\n{sandbox}opencode = \"{}\"\n", agent.display())).unwrap();
    }

    /// The login file the registry looks for.
    fn login(&self) {
        let dir = self.data.join("opencode");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("auth.json"), "{}").unwrap();
    }

    fn settings(&self, bwrap: Option<std::ffi::OsString>) -> Settings {
        Settings {
            host: HostEnv {
                path: Some(self.bin_dir.as_os_str().to_owned()),
                home: Some(self.tmp.path().join("home")),
                xdg_data_home: Some(self.data.clone()),
                claude_config_dir: None,
            },
            bwrap_env: bwrap,
            // Any executable file will do: the fake agent never starts it here.
            cerbo_bin_env: Some(FAKE.into()),
            current_exe: None,
            sockets_dir: self.paths.dir.join("sessions"),
        }
    }

    fn config(&self, approval_timeout: Option<Duration>, bwrap: Option<std::ffi::OsString>) -> Config {
        let mut config = Config::new(self.ctx.clone(), self.paths.clone());
        config.registry_poll = Duration::from_millis(50);
        config.event_quiet = Duration::from_millis(30);
        config.approval_timeout = approval_timeout;
        config.embedder = Arc::new(|_| {
            let dim = cerbo_core::semantic::Embedder::dim(&MockEmbedder::default());
            LazyEmbedder::new(dim, || Ok(Box::new(MockEmbedder::default())))
        });
        config.agents = self.settings(bwrap);
        config
    }

    fn start(&self) -> Running {
        cerboid::spawn(self.config(Some(Duration::from_secs(30)), None)).unwrap().expect("no other daemon runs")
    }

    fn start_with_timeout(&self, timeout: Duration) -> Running {
        cerboid::spawn(self.config(Some(timeout), None)).unwrap().expect("no other daemon runs")
    }

    fn person(&self) -> Client {
        Client::connect(&self.paths.socket, ClientKind::Cli, Some("test")).unwrap()
    }

    /// A person's connection that hears every event.
    fn viewer(&self) -> Client {
        let mut client = self.person();
        client.call(Call::Subscribe { vault: None }).unwrap();
        client
    }

    fn add_vault(&self, name: &str) -> (String, PathBuf) {
        let path = self.tmp.path().join("vaults").join(name);
        let (vault, _) = cerbo_core::vault::vault_create(&self.ctx, &path, name.to_string(), false).unwrap();
        (vault.id, vault.path)
    }

    fn set(&self, vault: &str, level: AgentPermission) {
        agent::set_permission(&self.ctx, vault, level).unwrap();
    }

    /// Put the fake agent's script where the agent finds it.
    fn script(&self, script: Value) {
        let home = self.data.join("cerbo/agents/opencode/home");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("fake-acp-script.json"), script.to_string()).unwrap();
    }

    fn session_dir(&self, session: &str) -> PathBuf {
        self.paths.dir.join("sessions").join(session)
    }

    fn agent_socket(&self, session: &str) -> PathBuf {
        self.session_dir(session).join("agent.sock")
    }
}

fn create(client: &mut Client, vault: &str) -> SessionInfo {
    let value = client.call(Call::SessionCreate { vault: vault.into(), agent: AgentKind::Opencode, resume_from: None }).unwrap();
    serde_json::from_value(value).unwrap()
}

fn send(client: &mut Client, session: &str, text: &str) {
    client.call(Call::SessionSend { session: session.into(), text: text.into(), context: vec![] }).unwrap();
}

/// Read the events of `session` from a subscribed connection until `stop`
/// holds for one of them (that one included).
fn collect(client: &mut Client, session: &str, stop: impl Fn(&SessionEventBody) -> bool) -> Vec<SessionEventBody> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        match client.next_event(Duration::from_millis(200)).unwrap() {
            Some(Event::Session { session: s, event, .. }) if s == session => {
                let done = stop(&event.body);
                seen.push(event.body);
                if done {
                    return seen;
                }
            }
            _ => {}
        }
    }
    panic!("timed out waiting; saw {seen:#?}");
}

fn turn_ended(b: &SessionEventBody) -> bool {
    matches!(b, SessionEventBody::TurnEnded { .. })
}

fn texts(events: &[SessionEventBody]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEventBody::AgentText { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn replay(client: &mut Client, session: &str) -> Vec<SessionEvent> {
    let value = client.call(Call::SessionEvents { session: session.into(), after_seq: None, limit: 10_000 }).unwrap();
    serde_json::from_value(value).unwrap()
}

fn state_of(client: &mut Client, session: &str) -> SessionState {
    let list: Vec<SessionInfo> = serde_json::from_value(client.call(Call::SessionList { vault: None }).unwrap()).unwrap();
    list.into_iter().find(|s| s.id == session).expect("the session is listed").state
}

fn seed_page(env: &Env, vault: &str, title: &str, content: &str) -> String {
    let root = cerbo_core::vault::get_vault_path(&env.ctx, vault).unwrap();
    let ctx = CerboContext { config_dir: root.join(".cerbo"), cache_dir: env.ctx.cache_dir.clone() };
    let id = cerbo_core::page::page_create(&ctx, title.to_string()).unwrap();
    cerbo_core::page::page_write(&ctx, id.clone(), content.to_string()).unwrap();
    id
}

fn content_of(client: &mut Client, vault: &str, page: &str) -> String {
    client.call(Call::PageRead { vault: vault.into(), page: page.into() }).unwrap()["content"].as_str().unwrap().to_string()
}

fn edit(vault: &str, page: &str, content: &str) -> Call {
    Call::PageEdit { vault: vault.into(), page: page.into(), content: content.into(), base_version: None }
}

// ── The agents ───────────────────────────────────────────────────────────────

#[test]
fn agent_list_shows_what_is_installed_and_why_not() {
    let env = env();
    let _daemon = env.start();
    let mut person = env.person();
    let list: Vec<AgentInfo> = serde_json::from_value(person.call(Call::AgentList {}).unwrap()).unwrap();
    let opencode = list.iter().find(|a| a.agent == AgentKind::Opencode).unwrap();
    assert!(opencode.available, "{:?}", opencode.reason);
    assert_eq!(opencode.logged_in, Some(true));
    let claude = list.iter().find(|a| a.agent == AgentKind::Claude).unwrap();
    assert!(!claude.available);
    assert!(claude.reason.as_deref().unwrap().contains("not found on PATH"));
}

#[test]
fn an_unavailable_or_signed_out_agent_makes_no_session() {
    let env = env();
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut person = env.person();
    let err = person.call(Call::SessionCreate { vault: vault.clone(), agent: AgentKind::Claude, resume_from: None }).unwrap_err();
    assert_eq!(err.code, CODE_FAILED);
    assert!(err.message.contains("not available") && err.message.contains("claude-agent-acp"), "{}", err.message);

    std::fs::remove_file(env.data.join("opencode/auth.json")).unwrap();
    // The registry looks again after its time to live; ask for the list to refresh it.
    person.call(Call::AgentList {}).unwrap();
    let err = person.call(Call::SessionCreate { vault, agent: AgentKind::Opencode, resume_from: None }).unwrap_err();
    assert!(err.message.contains("not signed in") && err.message.contains("opencode auth login"), "{}", err.message);
    let list: Vec<SessionInfo> = serde_json::from_value(person.call(Call::SessionList { vault: None }).unwrap()).unwrap();
    assert!(list.is_empty(), "nothing is left behind");
}

// ── Chat ─────────────────────────────────────────────────────────────────────

#[test]
fn a_message_makes_a_turn_and_the_feed_is_stored_and_listed() {
    let env = env();
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();

    let info = create(&mut person, &vault);
    assert_eq!(info.state, SessionState::Idle);
    assert_eq!(info.agent, AgentKind::Opencode);
    assert!(!info.sandboxed);
    assert!(info.warning.as_deref().unwrap().contains("without a sandbox"));
    assert_eq!(info.title, "New session");

    send(&mut person, &info.id, "Plan the trip\nsecond line");
    let live = collect(&mut viewer, &info.id, turn_ended);
    assert!(live.iter().any(|e| matches!(e, SessionEventBody::UserMessage { text, .. } if text.starts_with("Plan the trip"))));
    assert!(texts(&live).contains("echo: Plan the trip"), "{live:?}");
    assert!(matches!(live.last(), Some(SessionEventBody::TurnEnded { stop_reason }) if stop_reason == "end_turn"));

    // The turn is over and the session is ready again.
    let deadline = Instant::now() + Duration::from_secs(5);
    while state_of(&mut person, &info.id) != SessionState::Idle && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(state_of(&mut person, &info.id), SessionState::Idle);

    // The stored feed is what was streamed, numbered from 1 without gaps.
    let stored = replay(&mut person, &info.id);
    let seqs: Vec<u64> = stored.iter().map(|e| e.seq).collect();
    assert_eq!(seqs, (1..=stored.len() as u64).collect::<Vec<_>>());
    assert!(stored.iter().any(|e| matches!(&e.body, SessionEventBody::UserMessage { text, .. } if text.starts_with("Plan the trip"))));
    let tail = replay_after(&mut person, &info.id, 2);
    assert_eq!(tail.first().map(|e| e.seq), Some(3));

    // Listed with the title from the first message, newest first.
    let second = create(&mut person, &vault);
    send(&mut person, &second.id, "later");
    collect(&mut viewer, &second.id, turn_ended);
    let list: Vec<SessionInfo> = serde_json::from_value(person.call(Call::SessionList { vault: Some(vault.clone()) }).unwrap()).unwrap();
    assert_eq!(list.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), ["later", "Plan the trip"]);
    let other = env.tmp.path().join("vaults").join("Other");
    let (other_vault, _) = cerbo_core::vault::vault_create(&env.ctx, &other, "Other".to_string(), false).unwrap();
    let none: Vec<SessionInfo> = serde_json::from_value(person.call(Call::SessionList { vault: Some(other_vault.id) }).unwrap()).unwrap();
    assert!(none.is_empty());

    // Only a person's connection may use sessions.
    let mut agent_conn = Client::connect_bound(&env.agent_socket(&info.id), ClientKind::Agent, Some("x"), &vault).unwrap();
    let err = agent_conn.call(Call::SessionList { vault: None }).unwrap_err();
    assert_eq!(err.code, CODE_FORBIDDEN);
}

fn replay_after(client: &mut Client, session: &str, after: u64) -> Vec<SessionEvent> {
    let value = client.call(Call::SessionEvents { session: session.into(), after_seq: Some(after), limit: 10_000 }).unwrap();
    serde_json::from_value(value).unwrap()
}

#[test]
fn the_store_is_private_and_a_session_can_be_deleted_only_when_not_running() {
    use std::os::unix::fs::PermissionsExt;
    let env = env();
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    send(&mut person, &info.id, "hello");
    collect(&mut viewer, &info.id, turn_ended);

    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    let dir = env.data.join("cerbo/sessions").join(&vault);
    assert_eq!(mode(&dir), 0o700);
    assert_eq!(mode(&dir.join(format!("{}.jsonl", info.id))), 0o600);
    // The socket directory and the socket are the owner's.
    assert_eq!(mode(&env.session_dir(&info.id)), 0o700);
    assert_eq!(mode(&env.agent_socket(&info.id)), 0o600);

    let err = person.call(Call::SessionDelete { session: info.id.clone() }).unwrap_err();
    assert!(err.message.contains("end it first"), "{}", err.message);
    person.call(Call::SessionEnd { session: info.id.clone() }).unwrap();
    person.call(Call::SessionDelete { session: info.id.clone() }).unwrap();
    assert!(!dir.join(format!("{}.jsonl", info.id)).exists());
    let list: Vec<SessionInfo> = serde_json::from_value(person.call(Call::SessionList { vault: None }).unwrap()).unwrap();
    assert!(list.is_empty());
}

#[test]
fn tool_calls_become_rows() {
    let env = env();
    env.script(json!({"turns": [[
        {"do": "tool_call", "id": "t1", "title": "Read page", "kind": "read", "status": "in_progress", "raw_input": {"page": "p137"}},
        {"do": "tool_update", "id": "t1", "status": "completed", "text": "page text"},
        {"do": "text", "text": "done"}
    ]]}));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    send(&mut person, &info.id, "go");
    let events = collect(&mut viewer, &info.id, turn_ended);
    let call = events.iter().find_map(|e| match e {
        SessionEventBody::ToolCall { id, title, kind, status, input_summary } => Some((id, title, kind, status, input_summary)),
        _ => None,
    });
    let (id, title, kind, status, input) = call.expect("a tool call row");
    assert_eq!((id.as_str(), title.as_str(), kind.as_str(), status.as_str()), ("t1", "Read page", "read", "in_progress"));
    assert!(input.as_deref().unwrap().contains("p137"));
    assert!(events.iter().any(|e| matches!(e, SessionEventBody::ToolUpdate { id, status, output_summary }
        if id == "t1" && status == "completed" && output_summary.as_deref() == Some("page text"))));
    assert_eq!(texts(&events), "done");
}

// ── The session socket (D9) ──────────────────────────────────────────────────

#[test]
fn the_session_socket_counts_every_connection_as_the_sessions_agent() {
    let env = env();
    let (vault, _) = env.add_vault("Notes");
    let (other, _) = env.add_vault("Other");
    let page = seed_page(&env, &vault, "Page", "old");
    let _daemon = env.start();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    let socket = env.agent_socket(&info.id);

    // Says it is the app, and bound to nothing: still an agent of the vault.
    let mut posing = Client::connect(&socket, ClientKind::App, Some("pretender")).unwrap();
    let err = posing.call(Call::ApprovalAnswer { approval: 1, approve: true }).unwrap_err();
    assert_eq!(err.code, CODE_FORBIDDEN);
    assert_eq!(posing.call(Call::ApprovalList { vault: None }).unwrap_err().code, CODE_FORBIDDEN);
    assert_eq!(posing.call(Call::Stop).unwrap_err().code, CODE_FORBIDDEN);
    assert_eq!(posing.call(Call::SessionEnd { session: info.id.clone() }).unwrap_err().code, CODE_FORBIDDEN);
    // It sees only its own vault, and cannot reach another.
    let vaults = posing.call(Call::Vaults).unwrap();
    assert_eq!(vaults.as_array().unwrap().len(), 1);
    let err = posing.call(Call::PageList { vault: other.clone() }).unwrap_err();
    assert_eq!(err.code, CODE_FORBIDDEN);
    // The vault is read-only for agents: a write is refused, as for any agent.
    let err = posing.call(edit(&vault, &page, "new")).unwrap_err();
    assert_eq!(err.code, CODE_READ_ONLY);
    assert_eq!(content_of(&mut person, &vault, &page), "old");

    // A hello naming another vault does not change the binding either.
    let mut other_hello = Client::connect_bound(&socket, ClientKind::Agent, Some("x"), &other).unwrap();
    assert!(other_hello.call(Call::PageList { vault: vault.clone() }).is_ok());
    assert_eq!(other_hello.call(Call::PageList { vault: other }).unwrap_err().code, CODE_FORBIDDEN);

    // Administration lines are refused on it too.
    use std::io::{BufRead, BufReader, Write};
    let mut raw = std::os::unix::net::UnixStream::connect(&socket).unwrap();
    raw.write_all(b"{\"admin\":\"stop\"}\n").unwrap();
    let mut line = String::new();
    BufReader::new(raw).read_line(&mut line).unwrap();
    assert!(line.contains("forbidden"), "{line}");
    // The daemon is still there.
    assert_eq!(person.call(Call::Ping).unwrap(), json!("pong"));
}

#[test]
fn writes_on_the_session_socket_follow_the_vaults_permission() {
    let env = env();
    let (vault, _) = env.add_vault("Notes");
    let page = seed_page(&env, &vault, "Page", "old");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    let mut agent_conn = Client::connect_bound(&env.agent_socket(&info.id), ClientKind::Agent, Some("fake"), &vault).unwrap();

    // read: refused.
    assert_eq!(agent_conn.call(edit(&vault, &page, "one")).unwrap_err().code, CODE_READ_ONLY);

    // auto: applied, and the page change is in the session's feed.
    env.set(&vault, AgentPermission::Auto);
    agent_conn.call(edit(&vault, &page, "two")).unwrap();
    assert_eq!(content_of(&mut person, &vault, &page), "two");
    let events = collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::PageChanged { .. }));
    assert!(matches!(events.last(), Some(SessionEventBody::PageChanged { path }) if *path == page));
    assert!(replay(&mut person, &info.id).iter().any(|e| matches!(&e.body, SessionEventBody::PageChanged { path } if *path == page)));

    // approve: a card in the feed, then applied after the person approves.
    env.set(&vault, AgentPermission::Approve);
    let socket = env.agent_socket(&info.id);
    let (vault2, page2) = (vault.clone(), page.clone());
    let write = std::thread::spawn(move || {
        let mut c = Client::connect_bound(&socket, ClientKind::Agent, Some("fake"), &vault2).unwrap();
        c.call(edit(&vault2, &page2, "three"))
    });
    let events = collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::ApprovalRequested { .. }));
    let Some(SessionEventBody::ApprovalRequested { approval }) = events.last() else { unreachable!() };
    assert_eq!(approval.action, "edit");
    assert_eq!(approval.session.as_deref(), Some(info.id.as_str()));
    assert!(approval.diff.as_deref().unwrap().contains("+three"));
    person.call(Call::ApprovalAnswer { approval: approval.id, approve: true }).unwrap();
    write.join().unwrap().unwrap();
    assert_eq!(content_of(&mut person, &vault, &page), "three");
    let events = collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::PageChanged { .. }));
    assert!(events.iter().any(|e| matches!(e, SessionEventBody::ApprovalResolved { outcome: ApprovalOutcome::Approved, .. })));

    // approve, rejected: the agent sees the rejection and the page stays.
    let socket = env.agent_socket(&info.id);
    let (vault2, page2) = (vault.clone(), page.clone());
    let write = std::thread::spawn(move || {
        let mut c = Client::connect_bound(&socket, ClientKind::Agent, Some("fake"), &vault2).unwrap();
        c.call(edit(&vault2, &page2, "four"))
    });
    let events = collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::ApprovalRequested { .. }));
    let Some(SessionEventBody::ApprovalRequested { approval }) = events.last() else { unreachable!() };
    person.call(Call::ApprovalAnswer { approval: approval.id, approve: false }).unwrap();
    assert_eq!(write.join().unwrap().unwrap_err().code, CODE_REJECTED);
    assert_eq!(content_of(&mut person, &vault, &page), "three");
    collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::ApprovalResolved { outcome: ApprovalOutcome::Rejected, .. }));
}

// ── Tool permission (D3) ─────────────────────────────────────────────────────

fn approval_of(events: &[SessionEventBody]) -> ApprovalInfo {
    events
        .iter()
        .find_map(|e| match e {
            SessionEventBody::ApprovalRequested { approval } => Some((**approval).clone()),
            _ => None,
        })
        .expect("an approval card")
}

#[test]
fn a_tool_permission_request_is_a_card_and_the_answer_reaches_the_agent() {
    let env = env();
    env.script(json!({"turns": [
        [{"do": "permission", "title": "rm -rf notes", "allow": [{"do": "text", "text": "ran it"}], "reject": [{"do": "text", "text": "not allowed"}]}],
        [{"do": "permission", "title": "rm -rf notes", "allow": [{"do": "text", "text": "ran it"}], "reject": [{"do": "text", "text": "not allowed"}]}]
    ]}));
    let (vault, _) = env.add_vault("Notes");
    env.set(&vault, AgentPermission::Auto);
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);

    // Under auto, a tool that is not a cerbo tool still asks.
    send(&mut person, &info.id, "first");
    let events = collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::ApprovalRequested { .. }));
    let card = approval_of(&events);
    assert_eq!(card.action, "tool");
    assert_eq!(card.tool_title.as_deref(), Some("rm -rf notes"));
    assert_eq!(card.tool_kind.as_deref(), Some("edit"));
    assert_eq!(card.session.as_deref(), Some(info.id.as_str()));
    person.call(Call::ApprovalAnswer { approval: card.id, approve: true }).unwrap();
    let events = collect(&mut viewer, &info.id, turn_ended);
    assert!(events.iter().any(|e| matches!(e, SessionEventBody::ApprovalResolved { outcome: ApprovalOutcome::Approved, .. })));
    let text = texts(&events);
    // `allow` is the allow_once option; an always option is never chosen.
    assert!(text.contains("permission: selected allow") && text.contains("ran it"), "{text}");

    // A person's rejection becomes the agent's reject option.
    send(&mut person, &info.id, "second");
    let events = collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::ApprovalRequested { .. }));
    person.call(Call::ApprovalAnswer { approval: approval_of(&events).id, approve: false }).unwrap();
    let events = collect(&mut viewer, &info.id, turn_ended);
    assert!(texts(&events).contains("permission: selected reject"), "{events:?}");
}

#[test]
fn auto_allows_the_cerbo_tools_without_a_card_but_approve_asks() {
    let env = env();
    env.script(json!({"turns": [[
        {"do": "permission", "title": "cerbo_edit_page", "allow": [{"do": "text", "text": "edited"}], "reject": [{"do": "text", "text": "refused"}]}
    ]]}));
    let (vault, _) = env.add_vault("Notes");
    env.set(&vault, AgentPermission::Auto);
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);

    send(&mut person, &info.id, "one");
    let events = collect(&mut viewer, &info.id, turn_ended);
    assert!(texts(&events).contains("edited"), "{events:?}");
    assert!(!events.iter().any(|e| matches!(e, SessionEventBody::ApprovalRequested { .. })), "no card under auto");

    env.set(&vault, AgentPermission::Approve);
    send(&mut person, &info.id, "two");
    let events = collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::ApprovalRequested { .. }));
    assert_eq!(approval_of(&events).tool_title.as_deref(), Some("cerbo_edit_page"));
    person.call(Call::ApprovalAnswer { approval: approval_of(&events).id, approve: true }).unwrap();
    collect(&mut viewer, &info.id, turn_ended);
}

#[test]
fn an_unanswered_tool_card_is_rejected_at_the_deadline() {
    let env = env();
    env.script(json!({"turns": [[
        {"do": "permission", "title": "curl example.com", "allow": [{"do": "text", "text": "ran"}], "reject": [{"do": "text", "text": "rejected"}]}
    ]]}));
    let (vault, _) = env.add_vault("Notes");
    env.set(&vault, AgentPermission::Approve);
    let _daemon = env.start_with_timeout(Duration::from_secs(1));
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    send(&mut person, &info.id, "go");
    let events = collect(&mut viewer, &info.id, turn_ended);
    assert!(events.iter().any(|e| matches!(e, SessionEventBody::ApprovalResolved { outcome: ApprovalOutcome::TimedOut, .. })), "{events:?}");
    assert!(texts(&events).contains("permission: selected reject"), "{events:?}");
}

// ── Cancel, end, failures ────────────────────────────────────────────────────

#[test]
fn cancel_stops_the_turn_and_keeps_the_session() {
    let env = env();
    env.script(json!({"turns": [
        [{"do": "text", "text": "working"}, {"do": "wait_cancel"}],
        [{"do": "echo"}]
    ]}));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    send(&mut person, &info.id, "long task");
    collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::AgentText { text } if text == "working"));
    assert_eq!(state_of(&mut person, &info.id), SessionState::Working);
    // A second message while working is refused.
    let err = person.call(Call::SessionSend { session: info.id.clone(), text: "again".into(), context: vec![] }).unwrap_err();
    assert!(err.message.contains("still working"), "{}", err.message);

    person.call(Call::SessionCancel { session: info.id.clone() }).unwrap();
    let events = collect(&mut viewer, &info.id, turn_ended);
    assert!(matches!(events.last(), Some(SessionEventBody::TurnEnded { stop_reason }) if stop_reason == "cancelled"), "{events:?}");

    // Still open: the next message works.
    send(&mut person, &info.id, "next");
    let events = collect(&mut viewer, &info.id, turn_ended);
    assert!(texts(&events).contains("echo: next"), "{events:?}");
}

#[test]
fn ending_a_session_rejects_its_approvals_stops_the_agent_and_removes_the_socket() {
    let env = env();
    env.script(json!({"turns": [[
        {"do": "permission", "title": "make coffee", "allow": [{"do": "text", "text": "yes"}], "reject": [{"do": "text", "text": "no"}]}
    ]]}));
    let (vault, _) = env.add_vault("Notes");
    env.set(&vault, AgentPermission::Approve);
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    send(&mut person, &info.id, "go");
    collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::ApprovalRequested { .. }));
    assert!(env.agent_socket(&info.id).exists());
    let pending: Vec<ApprovalInfo> = serde_json::from_value(person.call(Call::ApprovalList { vault: None }).unwrap()).unwrap();
    assert_eq!(pending.len(), 1);

    person.call(Call::SessionEnd { session: info.id.clone() }).unwrap();
    let events = collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::State { state: SessionState::Ended, .. }));
    assert!(events.iter().any(|e| matches!(e, SessionEventBody::ApprovalResolved { outcome: ApprovalOutcome::Rejected, .. })), "{events:?}");
    let pending: Vec<ApprovalInfo> = serde_json::from_value(person.call(Call::ApprovalList { vault: None }).unwrap()).unwrap();
    assert!(pending.is_empty());
    assert!(!env.session_dir(&info.id).exists(), "the socket directory is removed");
    assert_eq!(state_of(&mut person, &info.id), SessionState::Ended);
    // Ending again does nothing; a message to an ended session is an error.
    person.call(Call::SessionEnd { session: info.id.clone() }).unwrap();
    let err = person.call(Call::SessionSend { session: info.id.clone(), text: "x".into(), context: vec![] }).unwrap_err();
    assert!(err.message.contains("ended"), "{}", err.message);
}

#[test]
fn a_crash_fails_the_session_with_the_end_of_stderr_and_does_not_retry() {
    let env = env();
    env.script(json!({"turns": [[{"do": "text", "text": "about to crash"}, {"do": "crash", "stderr": "kaboom 137", "code": 3}]]}));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    send(&mut person, &info.id, "go");
    let events = collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::State { state: SessionState::Failed, .. }));
    let Some(SessionEventBody::State { reason, stderr_tail, .. }) = events.last() else { unreachable!() };
    assert!(reason.as_deref().unwrap().contains("exited with code 3"), "{reason:?}");
    assert!(stderr_tail.as_deref().unwrap().contains("kaboom 137"), "{stderr_tail:?}");
    assert_eq!(state_of(&mut person, &info.id), SessionState::Failed);
    assert!(!env.session_dir(&info.id).exists());
    // No retry: the session stays failed and takes no message.
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(state_of(&mut person, &info.id), SessionState::Failed);
    let err = person.call(Call::SessionSend { session: info.id.clone(), text: "again".into(), context: vec![] }).unwrap_err();
    assert!(err.message.contains("ended"), "{}", err.message);
}

#[test]
fn an_agent_that_does_not_start_fails_the_session_with_its_stderr() {
    let env = env();
    let broken = env.bin_dir.join("opencode-broken");
    std::fs::write(&broken, "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo v1; exit 0; fi\necho \"no display 137\" >&2\nexit 7\n").unwrap();
    std::fs::set_permissions(&broken, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    env.write_agents_toml(&broken);
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    assert_eq!(info.state, SessionState::Failed);
    let feed = replay(&mut person, &info.id);
    let failed = feed.iter().find_map(|e| match &e.body {
        SessionEventBody::State { state: SessionState::Failed, reason, stderr_tail } => Some((reason.clone(), stderr_tail.clone())),
        _ => None,
    });
    let (reason, tail) = failed.expect("a failed state in the feed");
    assert!(reason.unwrap().contains("exited with code 7"));
    assert!(tail.unwrap().contains("no display 137"));
}

// ── Resume (D6) ──────────────────────────────────────────────────────────────

#[test]
fn resuming_without_load_session_sends_the_transcript_as_context() {
    let env = env();
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    assert!(!info.resumable);
    send(&mut person, &info.id, "remember seven");
    collect(&mut viewer, &info.id, turn_ended);
    person.call(Call::SessionEnd { session: info.id.clone() }).unwrap();

    let value = person.call(Call::SessionCreate { vault: vault.clone(), agent: AgentKind::Opencode, resume_from: Some(info.id.clone()) }).unwrap();
    let resumed: SessionInfo = serde_json::from_value(value).unwrap();
    assert_eq!(resumed.id, info.id, "the same session continues");
    assert_eq!(resumed.state, SessionState::Idle);
    let feed = replay(&mut person, &info.id);
    assert!(feed.iter().any(|e| matches!(&e.body, SessionEventBody::Resumed { how: ResumeHow::Transcript })), "{feed:?}");

    send(&mut person, &info.id, "what did I say");
    let events = collect(&mut viewer, &info.id, turn_ended);
    let text = texts(&events);
    assert!(text.contains("User: remember seven") && text.contains("what did I say"), "{text}");

    // Still running sessions cannot be resumed; another vault's cannot either.
    let err = person.call(Call::SessionCreate { vault, agent: AgentKind::Opencode, resume_from: Some(info.id.clone()) }).unwrap_err();
    assert!(err.message.contains("still running"), "{}", err.message);
}

#[test]
fn resuming_with_load_session_loads_the_agents_own_session_and_keeps_the_feed() {
    let env = env();
    env.script(json!({
        "load_session": true,
        "known_sessions": ["fake-session-1"],
        "history": [{"role": "user", "text": "old question"}, {"role": "agent", "text": "old answer"}]
    }));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    assert!(info.resumable);
    send(&mut person, &info.id, "first");
    collect(&mut viewer, &info.id, turn_ended);
    person.call(Call::SessionEnd { session: info.id.clone() }).unwrap();
    let before = replay(&mut person, &info.id).len();

    person.call(Call::SessionCreate { vault, agent: AgentKind::Opencode, resume_from: Some(info.id.clone()) }).unwrap();
    let feed = replay(&mut person, &info.id);
    assert!(feed.iter().any(|e| matches!(&e.body, SessionEventBody::Resumed { how: ResumeHow::Load })), "{feed:?}");
    // The history the agent replays while loading is not added a second time.
    assert!(!feed.iter().any(|e| matches!(&e.body, SessionEventBody::AgentText { text } if text == "old answer")));
    assert!(feed.len() >= before);
    send(&mut person, &info.id, "second");
    let events = collect(&mut viewer, &info.id, turn_ended);
    // No transcript was sent: the agent has its own history.
    assert!(!texts(&events).contains("earlier conversation"), "{events:?}");
}

#[test]
fn stored_sessions_survive_a_restart_and_show_as_ended() {
    let env = env();
    let (vault, _) = env.add_vault("Notes");
    let id = {
        let _daemon = env.start();
        let mut viewer = env.viewer();
        let mut person = env.person();
        let info = create(&mut person, &vault);
        send(&mut person, &info.id, "persist me");
        collect(&mut viewer, &info.id, turn_ended);
        info.id
    };
    // The daemon stopped with the first block; its agent went with it.
    let _daemon = env.start();
    let mut person = env.person();
    let list: Vec<SessionInfo> = serde_json::from_value(person.call(Call::SessionList { vault: None }).unwrap()).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "persist me");
    assert_eq!(list[0].state, SessionState::Ended);
    let feed = replay(&mut person, &id);
    assert!(feed.iter().any(|e| matches!(&e.body, SessionEventBody::UserMessage { text, .. } if text == "persist me")));
    // It can be deleted without ending: nothing runs.
    person.call(Call::SessionDelete { session: id }).unwrap();
}

// ── The real sandbox ─────────────────────────────────────────────────────────

fn bwrap_or_skip() -> Option<std::ffi::OsString> {
    let Some(path) = std::env::var_os("CERBO_BWRAP").filter(|v| !v.is_empty()) else {
        eprintln!("SKIPPED: CERBO_BWRAP is not set (the devshell sets it)");
        return None;
    };
    match cerboid::sandbox::confinement(cerbo_core::agent::SandboxMode::Bwrap, Some(path.clone())) {
        Ok(_) => Some(path),
        Err(e) => {
            eprintln!("SKIPPED: no usable sandbox here: {e}");
            None
        }
    }
}

#[test]
fn under_bubblewrap_the_agent_cannot_write_the_vault_directly() {
    let Some(bwrap) = bwrap_or_skip() else { return };
    let mut env = env();
    env.sandbox_none = false;
    env.write_agents_toml(Path::new(FAKE));
    env.script(json!({"turns": [[
        {"do": "write_file", "path": "intruder.md", "contents": "x"},
        {"do": "write_file", "path": "/tmp/outside-marker-137", "contents": "x"}
    ]]}));
    let (vault, vault_path) = env.add_vault("Notes");
    let _daemon = cerboid::spawn(env.config(Some(Duration::from_secs(30)), Some(bwrap))).unwrap().expect("no other daemon runs");
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    assert_eq!(info.state, SessionState::Idle, "{:?}", replay(&mut person, &info.id));
    assert!(info.sandboxed);
    assert!(info.warning.is_none());

    send(&mut person, &info.id, "write");
    let events = collect(&mut viewer, &info.id, turn_ended);
    let text = texts(&events);
    assert!(text.contains("write_file intruder.md: error"), "{text}");
    assert!(!vault_path.join("intruder.md").exists(), "the vault is unchanged");
    // /tmp inside the sandbox is private.
    assert!(!Path::new("/tmp/outside-marker-137").exists());
}
