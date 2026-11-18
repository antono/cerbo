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
    feed_cap: u64,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = CerboContext { config_dir: tmp.path().join("config"), cache_dir: tmp.path().join("cache") };
    let paths = DaemonPaths::in_dir(tmp.path().join("run").join("cerbo"));
    let data = tmp.path().join("data");
    let bin_dir = tmp.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let env = Env { tmp, ctx, paths, data, bin_dir, sandbox_none: true, feed_cap: cerboid::sessions::store::FEED_CAP_BYTES };
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
            feed_cap: self.feed_cap,
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

/// A session once its agent has started (or failed to): creating answers at
/// once, in the `starting` state.
fn settle(client: &mut Client, id: &str) -> SessionInfo {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let list: Vec<SessionInfo> = serde_json::from_value(client.call(Call::SessionList { vault: None }).unwrap()).unwrap();
        let info = list.into_iter().find(|s| s.id == id).expect("the session is listed");
        if info.state != SessionState::Starting {
            return info;
        }
        assert!(Instant::now() < deadline, "the session never left starting");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn create(client: &mut Client, vault: &str) -> SessionInfo {
    let value = client.call(Call::SessionCreate { vault: vault.into(), agent: AgentKind::Opencode, resume_from: None, ephemeral: false }).unwrap();
    let created: SessionInfo = serde_json::from_value(value).unwrap();
    assert_eq!(created.state, SessionState::Starting, "creating answers before the agent is ready");
    settle(client, &created.id)
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
    let err = person.call(Call::SessionCreate { vault: vault.clone(), agent: AgentKind::Claude, resume_from: None, ephemeral: false }).unwrap_err();
    assert_eq!(err.code, CODE_FAILED);
    assert!(err.message.contains("not available") && err.message.contains("claude-agent-acp"), "{}", err.message);

    std::fs::remove_file(env.data.join("opencode/auth.json")).unwrap();
    // The registry looks again after its time to live; ask for the list to refresh it.
    person.call(Call::AgentList {}).unwrap();
    let err = person.call(Call::SessionCreate { vault, agent: AgentKind::Opencode, resume_from: None, ephemeral: false }).unwrap_err();
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

    let value = person.call(Call::SessionCreate { vault: vault.clone(), agent: AgentKind::Opencode, resume_from: Some(info.id.clone()), ephemeral: false }).unwrap();
    let resumed: SessionInfo = serde_json::from_value(value).unwrap();
    assert_eq!(resumed.id, info.id, "the same session continues");
    assert_eq!(settle(&mut person, &info.id).state, SessionState::Idle);
    let feed = replay(&mut person, &info.id);
    assert!(feed.iter().any(|e| matches!(&e.body, SessionEventBody::Resumed { how: ResumeHow::Transcript })), "{feed:?}");

    send(&mut person, &info.id, "what did I say");
    let events = collect(&mut viewer, &info.id, turn_ended);
    let text = texts(&events);
    assert!(text.contains("User: remember seven") && text.contains("what did I say"), "{text}");

    // Still running sessions cannot be resumed; another vault's cannot either.
    let err = person.call(Call::SessionCreate { vault, agent: AgentKind::Opencode, resume_from: Some(info.id.clone()), ephemeral: false }).unwrap_err();
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

    person.call(Call::SessionCreate { vault, agent: AgentKind::Opencode, resume_from: Some(info.id.clone()), ephemeral: false }).unwrap();
    settle(&mut person, &info.id);
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

// ── Review fixes: starting, resuming, cancelling, limits ─────────────────────

/// `start` of the daemon with `tweak` applied to its config.
fn start_tweaked(env: &Env, tweak: impl FnOnce(&mut Config)) -> Running {
    let mut config = env.config(Some(Duration::from_secs(30)), None);
    tweak(&mut config);
    cerboid::spawn(config).unwrap().expect("no other daemon runs")
}

fn raw_create(client: &mut Client, vault: &str, resume: Option<&str>, ephemeral: bool) -> Result<SessionInfo, cerbo_core::rpc::RpcError> {
    client
        .call(Call::SessionCreate { vault: vault.into(), agent: AgentKind::Opencode, resume_from: resume.map(str::to_string), ephemeral })
        .map(|v| serde_json::from_value(v).unwrap())
}

fn feed_states(feed: &[SessionEvent]) -> Vec<SessionState> {
    feed.iter()
        .filter_map(|e| match &e.body {
            SessionEventBody::State { state, .. } => Some(*state),
            _ => None,
        })
        .collect()
}

#[test]
fn two_creates_that_resume_the_same_session_make_one() {
    let env = env();
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    person.call(Call::SessionEnd { session: info.id.clone() }).unwrap();

    // From now on the agent takes its time to start, so both requests meet
    // while the first one is still starting.
    env.script(json!({"init_delay_ms": 600}));
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let attempts: Vec<_> = (0..2)
        .map(|_| {
            let (socket, vault, id, barrier) = (env.paths.socket.clone(), vault.clone(), info.id.clone(), Arc::clone(&barrier));
            std::thread::spawn(move || {
                let mut client = Client::connect(&socket, ClientKind::Cli, Some("racer")).unwrap();
                barrier.wait();
                raw_create(&mut client, &vault, Some(&id), false)
            })
        })
        .collect();
    let results: Vec<_> = attempts.into_iter().map(|h| h.join().unwrap()).collect();
    let ok = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(ok, 1, "exactly one resume wins: {results:?}");
    let refused = results.iter().find_map(|r| r.as_ref().err()).unwrap();
    assert!(refused.message.contains("still running or starting"), "{}", refused.message);

    assert_eq!(settle(&mut person, &info.id).state, SessionState::Idle);
    let feed = replay(&mut person, &info.id);
    assert_eq!(feed.iter().filter(|e| matches!(e.body, SessionEventBody::Resumed { .. })).count(), 1, "one agent was started: {feed:?}");
    // And a third try, with the session live, is refused as well.
    let err = raw_create(&mut person, &vault, Some(&info.id), false).unwrap_err();
    assert!(err.message.contains("still running"), "{}", err.message);
}

#[test]
fn creating_answers_at_once_and_the_start_finishes_through_state_events() {
    let env = env();
    env.script(json!({"init_delay_ms": 1500}));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let began = Instant::now();
    let created = raw_create(&mut person, &vault, None, false).unwrap();
    assert!(began.elapsed() < Duration::from_millis(1200), "create waited for the agent: {:?}", began.elapsed());
    assert_eq!(created.state, SessionState::Starting);
    // A message now is refused, politely.
    let err = person.call(Call::SessionSend { session: created.id.clone(), text: "too early".into(), context: vec![] }).unwrap_err();
    assert!(err.message.contains("still starting"), "{}", err.message);
    // The session cannot be deleted while it starts.
    assert!(person.call(Call::SessionDelete { session: created.id.clone() }).is_err());
    // The listing shows it starting, not ended.
    assert_eq!(state_of(&mut person, &created.id), SessionState::Starting);

    let events = collect(&mut viewer, &created.id, |b| matches!(b, SessionEventBody::State { state: SessionState::Idle, .. }));
    assert!(matches!(events.last(), Some(SessionEventBody::State { state: SessionState::Idle, .. })));
    send(&mut person, &created.id, "now");
    collect(&mut viewer, &created.id, turn_ended);
}

#[test]
fn a_failed_start_after_an_instant_answer_is_a_failed_state_event() {
    let env = env();
    env.script(json!({"init_error": "no way 137"}));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let created = raw_create(&mut person, &vault, None, false).unwrap();
    assert_eq!(created.state, SessionState::Starting);
    let events = collect(&mut viewer, &created.id, |b| matches!(b, SessionEventBody::State { state: SessionState::Failed, .. }));
    let Some(SessionEventBody::State { reason, .. }) = events.last() else { unreachable!() };
    assert!(reason.as_deref().unwrap().contains("no way 137"), "{reason:?}");
}

#[test]
fn ending_a_session_while_it_starts_wins() {
    let env = env();
    env.script(json!({"init_delay_ms": 800}));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut person = env.person();
    let created = raw_create(&mut person, &vault, None, false).unwrap();
    person.call(Call::SessionEnd { session: created.id.clone() }).unwrap();
    assert_eq!(state_of(&mut person, &created.id), SessionState::Ended);

    // The agent finishes starting meanwhile; the session stays ended.
    let deadline = Instant::now() + Duration::from_secs(10);
    while env.session_dir(&created.id).exists() {
        assert!(Instant::now() < deadline, "the socket of the ended session was never removed");
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(state_of(&mut person, &created.id), SessionState::Ended);
    let feed = replay(&mut person, &created.id);
    let states = feed_states(&feed);
    assert_eq!(states.last(), Some(&SessionState::Ended), "{states:?}");
    assert!(!states.contains(&SessionState::Idle) && !states.contains(&SessionState::Failed), "{states:?}");
    let err = person.call(Call::SessionSend { session: created.id.clone(), text: "x".into(), context: vec![] }).unwrap_err();
    assert!(err.message.contains("ended"), "{}", err.message);
}

#[test]
fn a_permission_request_made_while_the_turn_is_cancelled_makes_no_card() {
    let env = env();
    // The agent asks only after it was told to cancel.
    env.script(json!({"turns": [[{"do": "permission", "after_cancel": true, "title": "Run a shell"}]]}));
    let (vault, _) = env.add_vault("Notes");
    env.set(&vault, AgentPermission::Approve);
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    send(&mut person, &info.id, "go");
    collect(&mut viewer, &info.id, |b| matches!(b, SessionEventBody::State { state: SessionState::Working, .. }));
    std::thread::sleep(Duration::from_millis(200));
    person.call(Call::SessionCancel { session: info.id.clone() }).unwrap();
    let events = collect(&mut viewer, &info.id, turn_ended);
    assert!(!events.iter().any(|e| matches!(e, SessionEventBody::ApprovalRequested { .. })), "no card for the cancelled turn: {events:?}");
    assert!(matches!(events.last(), Some(SessionEventBody::TurnEnded { stop_reason }) if stop_reason == "cancelled"));
    let queue: Vec<ApprovalInfo> = serde_json::from_value(person.call(Call::ApprovalList { vault: None }).unwrap()).unwrap();
    assert!(queue.is_empty(), "{queue:?}");
}

#[test]
fn a_subscriber_that_fell_behind_is_told_to_read_again() {
    let env = env();
    // Tens of thousands of one-character chunks: far more than the daemon's
    // event buffer holds while a slow reader's socket is full.
    env.script(json!({"turns": [[{"do": "text", "text": "x".repeat(40_000), "chunk_size": 1}]]}));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut slow = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    send(&mut person, &info.id, "flood");
    // Do not read until the turn is over.
    let deadline = Instant::now() + Duration::from_secs(60);
    while state_of(&mut person, &info.id) != SessionState::Idle {
        assert!(Instant::now() < deadline, "the turn never ended");
        std::thread::sleep(Duration::from_millis(100));
    }
    let mut resync = false;
    let mut seen = 0usize;
    while let Some(event) = slow.next_event(Duration::from_millis(500)).unwrap() {
        seen += 1;
        resync |= event == Event::Resync;
    }
    assert!(resync, "the slow reader lost events and was not told ({seen} events read)");
    // After a resync the stored feed has everything, to the end of the turn.
    let tail = replay_after(&mut person, &info.id, 40_000);
    assert!(tail.iter().any(|e| matches!(e.body, SessionEventBody::TurnEnded { .. })), "{} events after 40000", tail.len());
}

#[test]
fn an_ephemeral_session_ends_with_the_connection_that_made_it() {
    let env = env();
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut person = env.person();
    let mut owner = env.person();
    let ephemeral = raw_create(&mut owner, &vault, None, true).unwrap();
    let lasting = raw_create(&mut owner, &vault, None, false).unwrap();
    assert_eq!(settle(&mut person, &ephemeral.id).state, SessionState::Idle);
    assert_eq!(settle(&mut person, &lasting.id).state, SessionState::Idle);

    drop(owner);
    let deadline = Instant::now() + Duration::from_secs(10);
    while state_of(&mut person, &ephemeral.id) != SessionState::Ended {
        assert!(Instant::now() < deadline, "the session outlived its connection");
        std::thread::sleep(Duration::from_millis(50));
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while env.session_dir(&ephemeral.id).exists() {
        assert!(Instant::now() < deadline, "its socket stayed");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(state_of(&mut person, &lasting.id), SessionState::Idle, "a session made without the flag stays");
}

#[test]
fn a_session_a_dead_daemon_left_running_is_ended_with_the_reason() {
    use cerboid::sessions::store::{Meta, Store};
    let env = env();
    let (vault, _) = env.add_vault("Notes");
    let id = "00000000-0000-4000-8000-000000000137".to_string();
    {
        // What a daemon that crashed in the middle of a turn leaves.
        let store = Store::new(env.data.join("cerbo/sessions"));
        let mut log = store.open_log(&vault, &id).unwrap();
        log.append(SessionEventBody::State { state: SessionState::Idle, reason: None, stderr_tail: None }).unwrap();
        log.append(SessionEventBody::UserMessage { text: "mid-turn".into(), context: vec![] }).unwrap();
        log.append(SessionEventBody::State { state: SessionState::Working, reason: None, stderr_tail: None }).unwrap();
        store
            .write_meta(&Meta {
                id: id.clone(),
                vault: vault.clone(),
                agent: AgentKind::Opencode,
                title: "mid-turn".into(),
                state: SessionState::Working,
                created_at: "2026-01-01T00:00:00.000Z".into(),
                updated_at: "2026-01-01T00:00:01.000Z".into(),
                resumable: false,
                sandboxed: false,
                warning: None,
                acp_session_id: None,
            })
            .unwrap();
    }
    let daemon = env.start();
    let mut person = env.person();
    let feed = replay(&mut person, &id);
    let last = feed.last().unwrap();
    assert!(
        matches!(&last.body, SessionEventBody::State { state: SessionState::Ended, reason, .. } if reason.as_deref() == Some("daemon restarted")),
        "{feed:?}"
    );
    assert_eq!(state_of(&mut person, &id), SessionState::Ended);
    // Looking again changes nothing, and neither does another daemon.
    assert_eq!(replay(&mut person, &id).len(), feed.len());
    drop(daemon);
    let _again = env.start();
    let mut person = env.person();
    assert_eq!(replay(&mut person, &id).len(), feed.len(), "settled once");
}

#[test]
fn what_an_agent_sends_is_cut_to_a_size_and_the_feed_has_a_cap() {
    let env = env();
    env.script(json!({"turns": [[
        {"do": "tool_call", "id": "i".repeat(500), "title": "t".repeat(2000), "kind": "other"},
        {"do": "text", "text": "y".repeat(70_000)}
    ]]}));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    send(&mut person, &info.id, "go");
    let events = collect(&mut viewer, &info.id, turn_ended);
    let text = texts(&events);
    assert_eq!(text.chars().count(), 64 * 1024 + 1, "one chunk is cut to the limit, with a mark");
    let call = events.iter().find_map(|e| match e {
        SessionEventBody::ToolCall { id, title, .. } => Some((id.clone(), title.clone())),
        _ => None,
    });
    let (id, title) = call.unwrap();
    assert_eq!(id.chars().count(), 200);
    assert_eq!(title.chars().count(), 401);
}

#[test]
fn a_feed_that_reaches_its_cap_ends_the_session_with_an_error() {
    let mut env = env();
    env.feed_cap = 20_000;
    env.script(json!({"turns": [[{"do": "text", "text": "z".repeat(100_000), "chunk_size": 100}]]}));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    send(&mut person, &info.id, "flood");
    let deadline = Instant::now() + Duration::from_secs(30);
    while state_of(&mut person, &info.id) != SessionState::Failed {
        assert!(Instant::now() < deadline, "the session never failed");
        std::thread::sleep(Duration::from_millis(50));
    }
    let feed = replay(&mut person, &info.id);
    let reason = feed.iter().rev().find_map(|e| match &e.body {
        SessionEventBody::State { state: SessionState::Failed, reason, .. } => reason.clone(),
        _ => None,
    });
    assert!(reason.unwrap().contains("size limit"));
    assert!(feed.iter().any(|e| matches!(&e.body, SessionEventBody::Error { message } if message.contains("size limit"))));
    let file = env.data.join("cerbo/sessions").join(&vault).join(format!("{}.jsonl", info.id));
    let size = std::fs::metadata(&file).unwrap().len();
    assert!(size < 20_000 + 1024 * 1024, "{size}");
    assert!(size > 19_000 - 2_000, "the feed grew to its cap: {size}");
    // The agent is stopped: nothing more arrives.
    let before = replay(&mut person, &info.id).len();
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(replay(&mut person, &info.id).len(), before);
    assert!(person.call(Call::SessionSend { session: info.id.clone(), text: "x".into(), context: vec![] }).is_err());
}

#[test]
fn the_pending_cap_counts_a_session_not_a_connection() {
    let env = env();
    let (vault, _) = env.add_vault("Notes");
    let page = seed_page(&env, &vault, "Page", "old");
    env.set(&vault, AgentPermission::Approve);
    let _daemon = start_tweaked(&env, |config| config.max_pending_per_agent = 2);
    let mut person = env.person();
    let info = create(&mut person, &vault);
    let socket = env.agent_socket(&info.id);
    let connect = || Client::connect_bound(&socket, ClientKind::Agent, Some("fake"), &vault).unwrap();

    // Two connections of the agent use the two places.
    let mut a = connect();
    let mut b = connect();
    a.send(edit(&vault, &page, "one")).unwrap();
    b.send(edit(&vault, &page, "two")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let queue: Vec<ApprovalInfo> = serde_json::from_value(person.call(Call::ApprovalList { vault: None }).unwrap()).unwrap();
        if queue.len() == 2 {
            break;
        }
        assert!(Instant::now() < deadline, "{queue:?}");
        std::thread::sleep(Duration::from_millis(25));
    }
    // A third connection, and a fourth, get none: opening connections does not help.
    for _ in 0..2 {
        let mut more = connect();
        let err = more.call(edit(&vault, &page, "three")).unwrap_err();
        assert_eq!(err.code, cerbo_core::rpc::CODE_TOO_MANY_PENDING, "{}", err.message);
        assert!(err.message.contains("this session"), "{}", err.message);
    }
}

#[test]
fn an_agent_that_starts_in_another_mode_is_put_into_the_default_mode() {
    let env = env();
    // A real adapter can pick `auto` from a vault's settings; the fake does.
    env.script(json!({"modes": {"currentModeId": "auto", "availableModes": [{"id": "default"}, {"id": "auto"}]}}));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    assert_eq!(info.state, SessionState::Idle);
    let log = std::fs::read_to_string(env.data.join("cerbo/agents/opencode/home/fake-acp-modes.log")).unwrap_or_default();
    assert_eq!(log.trim(), "default", "session/set_mode was sent once, to `default`");

    // An agent that offers no `default` mode cannot be put there: the session does not start.
    env.script(json!({"modes": {"currentModeId": "auto", "availableModes": [{"id": "auto"}]}}));
    let second = create(&mut person, &vault);
    assert_eq!(second.state, SessionState::Failed);
}

#[test]
fn a_finished_search_of_cerbo_shows_its_result_count() {
    let env = env();
    env.script(json!({"turns": [[
        {"do": "tool_call", "id": "s1", "title": "cerbo_search_text", "kind": "search", "status": "in_progress", "raw_input": {"query": "vendor"}},
        {"do": "tool_update", "id": "s1", "status": "completed", "text": "[\n  {\"uuid\": \"a\", \"title\": \"A\"},\n  {\"uuid\": \"b\", \"title\": \"B\"}\n]"},
        {"do": "tool_call", "id": "s2", "title": "mcp__cerbo__search_semantic", "kind": "other", "status": "in_progress"},
        {"do": "tool_update", "id": "s2", "status": "completed", "text": "[{\"uuid\": \"a\"}]"},
        {"do": "tool_call", "id": "s3", "title": "cerbo_search_text", "kind": "search", "status": "in_progress"},
        {"do": "tool_update", "id": "s3", "status": "completed", "text": "[]"},
        {"do": "tool_call", "id": "r1", "title": "cerbo_read_page", "kind": "read", "status": "in_progress"},
        {"do": "tool_update", "id": "r1", "status": "completed", "text": "[1, 2, 3]"},
    ]]}));
    let (vault, _) = env.add_vault("Notes");
    let _daemon = env.start();
    let mut viewer = env.viewer();
    let mut person = env.person();
    let info = create(&mut person, &vault);
    send(&mut person, &info.id, "go");
    let events = collect(&mut viewer, &info.id, turn_ended);
    let summary = |wanted: &str| {
        events.iter().find_map(|e| match e {
            SessionEventBody::ToolUpdate { id, output_summary, .. } if id == wanted => output_summary.clone(),
            _ => None,
        })
    };
    assert_eq!(summary("s1").as_deref(), Some("2 results"));
    assert_eq!(summary("s2").as_deref(), Some("1 result"));
    assert_eq!(summary("s3").as_deref(), Some("0 results"));
    // Other tools keep their own output.
    assert_eq!(summary("r1").as_deref(), Some("[1, 2, 3]"));
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
