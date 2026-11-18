//! `cerbo agent run | list | stop` against a real daemon (in this process), the
//! real `cerbo mcp` shim and the scripted fake ACP agent of the `cerboid`
//! crate. Agents run without the sandbox here (`sandbox = "none"`): the
//! sandbox has its own tests in `cerboid`.
//!
//! The fake agent is the source of `cerboid`'s `fake-acp-agent`, built as
//! `cerbo-fake-agent` by this package (see `cli/Cargo.toml`).

#![cfg(unix)]

use cerbo_core::agent::{self, AgentPermission};
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::client::Client;
use cerbo_core::rpc::{AgentKind, Call, ClientKind, SessionInfo, SessionState};
use cerbo_core::semantic::{LazyEmbedder, MockEmbedder};
use cerbo_core::CerboContext;
use cerboid::agents::HostEnv;
use cerboid::sessions::Settings;
use cerboid::{Config, Running};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const CERBO: &str = env!("CARGO_BIN_EXE_cerbo");

fn fake_agent() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cerbo-fake-agent"))
}

struct Env {
    xdg: TempDir,
    ctx: CerboContext,
    paths: DaemonPaths,
}

impl Env {
    fn new() -> Self {
        let xdg = TempDir::new().unwrap();
        let ctx = CerboContext { config_dir: xdg.path().join("config").join("cerbo"), cache_dir: xdg.path().join("cache").join("cerbo") };
        let paths = DaemonPaths::in_dir(xdg.path().join("run").join("cerbo"));
        let env = Env { xdg, ctx, paths };
        assert!(env.cerbo(&["vault", "list"]).status.success());
        // The agent: the fake one as "OpenCode", signed in, no sandbox.
        std::fs::write(env.ctx.config_dir.join("agents.toml"), format!("[agent]\nsandbox = \"none\"\nopencode = \"{}\"\n", fake_agent().display())).unwrap();
        let login = env.xdg.path().join("data/opencode");
        std::fs::create_dir_all(&login).unwrap();
        std::fs::write(login.join("auth.json"), "{}").unwrap();
        env
    }

    fn cmd(&self) -> Command {
        let mut cmd = Command::new(CERBO);
        cmd.env("XDG_CONFIG_HOME", self.xdg.path().join("config"))
            .env("XDG_CACHE_HOME", self.xdg.path().join("cache"))
            .env("XDG_RUNTIME_DIR", self.xdg.path().join("run"))
            .env(cerbo_core::semantic::MOCK_EMBEDDER_ENV, "1")
            .stdin(Stdio::null());
        cmd
    }

    fn cerbo(&self, args: &[&str]) -> Output {
        self.cmd().args(args).output().unwrap()
    }

    fn start_daemon(&self, approval_timeout: Duration) -> Running {
        self.start_daemon_with(approval_timeout, None)
    }

    fn start_daemon_with(&self, approval_timeout: Duration, bwrap: Option<std::ffi::OsString>) -> Running {
        let mut config = Config::new(self.ctx.clone(), self.paths.clone());
        config.registry_poll = Duration::from_millis(50);
        config.event_quiet = Duration::from_millis(30);
        config.approval_timeout = Some(approval_timeout);
        config.embedder = Arc::new(|_| {
            let dim = cerbo_core::semantic::Embedder::dim(&MockEmbedder::default());
            LazyEmbedder::new(dim, || Ok(Box::new(MockEmbedder::default())))
        });
        config.agents = Settings {
            host: HostEnv {
                path: Some(self.xdg.path().join("bin").into_os_string()),
                home: Some(self.xdg.path().join("home")),
                xdg_data_home: Some(self.xdg.path().join("data")),
                claude_config_dir: None,
            },
            bwrap_env: bwrap,
            cerbo_bin_env: Some(CERBO.into()),
            current_exe: None,
            sockets_dir: self.paths.dir.join("sessions"),
            feed_cap: cerboid::sessions::store::FEED_CAP_BYTES,
        };
        cerboid::spawn(config).unwrap().expect("no other daemon runs")
    }

    fn add_vault(&self, name: &str) -> (String, PathBuf) {
        let path = self.xdg.path().join("vaults").join(name);
        let (vault, _) = cerbo_core::vault::vault_create(&self.ctx, &path, name.to_string(), false).unwrap();
        (vault.id, vault.path)
    }

    fn page(&self, root: &Path, title: &str, content: &str) -> String {
        let ctx = CerboContext { config_dir: root.join(".cerbo"), cache_dir: self.ctx.cache_dir.clone() };
        let id = cerbo_core::page::page_create(&ctx, title.to_string()).unwrap();
        cerbo_core::page::page_write(&ctx, id.clone(), content.to_string()).unwrap();
        id
    }

    fn content(&self, root: &Path, page: &str) -> String {
        let ctx = CerboContext { config_dir: root.join(".cerbo"), cache_dir: self.ctx.cache_dir.clone() };
        cerbo_core::page::page_read(&ctx, page.to_string()).unwrap()
    }

    fn script(&self, script: Value) {
        let home = self.xdg.path().join("data/cerbo/agents/opencode/home");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("fake-acp-script.json"), script.to_string()).unwrap();
    }

    fn set(&self, vault: &str, level: AgentPermission) {
        agent::set_permission(&self.ctx, vault, level).unwrap();
    }

    fn client(&self) -> Client {
        Client::connect(&self.paths.socket, ClientKind::Cli, Some("test")).unwrap()
    }

    fn run(&self, vault: &str, extra: &[&str], prompt: &str) -> Output {
        let mut cmd = self.cmd();
        cmd.args(["agent", "run", "--vault", vault]).args(extra).arg(prompt);
        cmd.output().unwrap()
    }
}

/// The state of session `id` once `wanted` holds for it.
fn wait_for(client: &mut Client, id: &str, wanted: impl Fn(SessionState) -> bool) -> SessionState {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let list: Vec<SessionInfo> = serde_json::from_value(client.call(Call::SessionList { vault: None }).unwrap()).unwrap();
        if let Some(info) = list.iter().find(|s| s.id == id)
            && wanted(info.state)
        {
            return info.state;
        }
        assert!(Instant::now() < deadline, "the session never reached the wanted state: {list:?}");
        std::thread::sleep(Duration::from_millis(30));
    }
}

/// The id of the one session that exists, once it is `working`.
fn the_working_session(client: &mut Client) -> String {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let list: Vec<SessionInfo> = serde_json::from_value(client.call(Call::SessionList { vault: None }).unwrap()).unwrap();
        if let Some(info) = list.iter().find(|s| s.state == SessionState::Working) {
            return info.id.clone();
        }
        assert!(Instant::now() < deadline, "no session ever worked: {list:?}");
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn code(o: &Output) -> i32 {
    o.status.code().unwrap_or(-1)
}

#[test]
fn run_prints_the_answer_and_exits_zero_on_a_read_only_vault() {
    let env = Env::new();
    let (id, _) = env.add_vault("notes");
    let _daemon = env.start_daemon(Duration::from_secs(30));
    let o = env.run("notes", &[], "Summarize today's inbox page");
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(out(&o).contains("echo: Summarize today's inbox page"), "{}", out(&o));
    // The session of the run ended, and its transcript stays.
    let mut c = env.client();
    let list: Vec<SessionInfo> = serde_json::from_value(c.call(Call::SessionList { vault: Some(id) }).unwrap()).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].state, SessionState::Ended);
    assert_eq!(list[0].title, "Summarize today's inbox page");
}

#[test]
fn run_json_prints_the_transcript() {
    let env = Env::new();
    env.add_vault("notes");
    let _daemon = env.start_daemon(Duration::from_secs(30));
    let o = env.run("notes", &["--json"], "hello");
    assert_eq!(code(&o), 0, "{}", err(&o));
    let v: Value = serde_json::from_str(out(&o).trim()).unwrap();
    assert_eq!(v["exit_code"], 0);
    assert_eq!(v["stop_reason"], "end_turn");
    assert_eq!(v["agent"], "opencode");
    assert_eq!(v["rejected"], 0);
    assert!(v["text"].as_str().unwrap().contains("echo: hello"));
    let events = v["events"].as_array().unwrap();
    assert!(events.iter().any(|e| e["body"]["type"] == "user_message" && e["body"]["text"] == "hello"));
    assert!(events.iter().any(|e| e["body"]["type"] == "turn_ended"));
}

#[test]
fn run_on_an_auto_vault_changes_the_page_through_the_mcp_shim() {
    let env = Env::new();
    let (id, root) = env.add_vault("notes");
    let page = env.page(&root, "Inbox", "old\n");
    env.set(&id, AgentPermission::Auto);
    env.script(json!({"turns": [[
        {"do": "mcp_call", "tool": "edit_page", "arguments": {"page": page, "content": "summary\n"}},
        {"do": "text", "text": "all done"}
    ]]}));
    let _daemon = env.start_daemon(Duration::from_secs(30));
    let o = env.run("notes", &[], "tidy the inbox");
    assert_eq!(code(&o), 0, "{} {}", out(&o), err(&o));
    assert!(out(&o).contains("all done"), "{}", out(&o));
    assert_eq!(env.content(&root, &page), "summary\n");
    // The change is in the change log.
    let log: Value = serde_json::from_str(&String::from_utf8(env.cerbo(&["approvals", "log", "notes", "--json"]).stdout).unwrap()).unwrap();
    assert_eq!(log.as_array().unwrap().len(), 1, "{log}");
    assert_eq!(log[0]["page"], page.as_str());
}

#[test]
fn run_on_a_read_vault_cannot_change_a_page() {
    let env = Env::new();
    let (_id, root) = env.add_vault("notes");
    let page = env.page(&root, "Inbox", "old\n");
    env.script(json!({"turns": [[{"do": "mcp_call", "tool": "edit_page", "arguments": {"page": page, "content": "hacked\n"}}]]}));
    let _daemon = env.start_daemon(Duration::from_secs(30));
    let o = env.run("notes", &[], "try to edit");
    // The agent finished its turn; the write was refused, not rejected by a person.
    assert_eq!(code(&o), 0, "{} {}", out(&o), err(&o));
    assert!(out(&o).contains("mcp_result:"), "{}", out(&o));
    assert_eq!(env.content(&root, &page), "old\n");
}

#[test]
fn run_on_an_approve_vault_waits_for_a_person_and_then_applies_the_change() {
    let env = Env::new();
    let (id, root) = env.add_vault("notes");
    let page = env.page(&root, "Inbox", "old\n");
    env.set(&id, AgentPermission::Approve);
    env.script(json!({"turns": [[
        {"do": "mcp_call", "tool": "edit_page", "arguments": {"page": page, "content": "approved\n"}},
        {"do": "text", "text": "finished"}
    ]]}));
    let _daemon = env.start_daemon(Duration::from_secs(60));
    let cmd = env.cmd();
    let runner = std::thread::spawn(move || {
        let mut cmd = cmd;
        cmd.args(["agent", "run", "--vault", "notes", "edit please"]).output().unwrap()
    });
    let mut c = env.client();
    let deadline = Instant::now() + Duration::from_secs(20);
    let approval = loop {
        let list = c.call(Call::ApprovalList { vault: None }).unwrap();
        if let Some(first) = list.as_array().and_then(|l| l.first()) {
            break first.clone();
        }
        assert!(Instant::now() < deadline, "no approval card");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(approval["session"].is_string(), "the card names its session: {approval}");
    assert_eq!(env.content(&root, &page), "old\n");
    assert!(env.cerbo(&["approvals", "approve", &approval["id"].to_string()]).status.success());
    let o = runner.join().unwrap();
    assert_eq!(code(&o), 0, "{} {}", out(&o), err(&o));
    assert!(out(&o).contains("finished"));
    assert_eq!(env.content(&root, &page), "approved\n");
}

#[test]
fn run_on_an_approve_vault_with_nobody_there_is_rejected_at_the_deadline_and_exits_three() {
    let env = Env::new();
    let (id, root) = env.add_vault("notes");
    let page = env.page(&root, "Inbox", "old\n");
    env.set(&id, AgentPermission::Approve);
    env.script(json!({"turns": [[
        {"do": "mcp_call", "tool": "edit_page", "arguments": {"page": page, "content": "never\n"}},
        {"do": "text", "text": "gave up"}
    ]]}));
    let _daemon = env.start_daemon(Duration::from_secs(1));
    let started = Instant::now();
    let o = env.run("notes", &[], "edit please");
    assert_eq!(code(&o), 3, "{} {}", out(&o), err(&o));
    assert!(started.elapsed() < Duration::from_secs(30));
    assert!(err(&o).contains("rejected"), "{}", err(&o));
    assert!(out(&o).contains("gave up"), "the agent's answer is still printed: {}", out(&o));
    assert_eq!(env.content(&root, &page), "old\n");
    let json = env.run("notes", &["--json"], "again");
    assert_eq!(code(&json), 3);
    let v: Value = serde_json::from_str(out(&json).trim()).unwrap();
    assert_eq!(v["exit_code"], 3);
    assert_eq!(v["rejected"], 1);
}

#[test]
fn run_times_out_with_four_and_cancels_the_turn() {
    let env = Env::new();
    env.add_vault("notes");
    env.script(json!({"turns": [[{"do": "text", "text": "slow start"}, {"do": "wait_cancel"}]]}));
    let _daemon = env.start_daemon(Duration::from_secs(30));
    let started = Instant::now();
    let o = env.run("notes", &["--timeout", "1"], "take forever");
    assert_eq!(code(&o), 4, "{} {}", out(&o), err(&o));
    assert!(started.elapsed() < Duration::from_secs(30));
    assert!(err(&o).contains("time limit"), "{}", err(&o));
}

#[test]
fn run_ends_its_session_when_it_is_told_to_stop() {
    let env = Env::new();
    env.add_vault("notes");
    env.script(json!({"turns": [[{"do": "text", "text": "working"}, {"do": "wait_cancel"}]]}));
    let _daemon = env.start_daemon(Duration::from_secs(30));
    let mut c = env.client();
    for (signal, expected) in [(libc::SIGTERM, 143), (libc::SIGINT, 130)] {
        let child = env.cmd().args(["agent", "run", "--vault", "notes", "take forever"]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let id = the_working_session(&mut c);
        // SAFETY: a signal to the child process this test started.
        unsafe { libc::kill(child.id() as i32, signal) };
        let o = child.wait_with_output().unwrap();
        assert_eq!(code(&o), expected, "{} {}", out(&o), err(&o));
        assert!(err(&o).contains("interrupted"), "{}", err(&o));
        assert_eq!(wait_for(&mut c, &id, |s| s == SessionState::Ended), SessionState::Ended, "the session ended with the command");
    }
}

#[test]
fn a_killed_run_leaves_no_session_running() {
    let env = Env::new();
    env.add_vault("notes");
    env.script(json!({"turns": [[{"do": "text", "text": "working"}, {"do": "wait_cancel"}]]}));
    let _daemon = env.start_daemon(Duration::from_secs(30));
    let mut c = env.client();
    let mut child = env.cmd().args(["agent", "run", "--vault", "notes", "take forever"]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let id = the_working_session(&mut c);
    // No chance to clean up: the daemon notices the connection is gone.
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(wait_for(&mut c, &id, |s| s == SessionState::Ended), SessionState::Ended);
}

#[test]
fn run_fails_with_one_when_the_agent_crashes_or_is_missing() {
    let env = Env::new();
    env.add_vault("notes");
    env.script(json!({"turns": [[{"do": "crash", "stderr": "kaboom 137", "code": 3}]]}));
    let _daemon = env.start_daemon(Duration::from_secs(30));
    let o = env.run("notes", &[], "go");
    assert_eq!(code(&o), 1, "{} {}", out(&o), err(&o));
    assert!(err(&o).contains("exited with code 3") && err(&o).contains("kaboom 137"), "{}", err(&o));

    // Claude Code is not installed in this environment.
    let o = env.run("notes", &["--agent", "claude"], "go");
    assert_eq!(code(&o), 1);
    assert!(err(&o).contains("not available"), "{}", err(&o));

    let o = env.run("no-such-vault", &[], "go");
    assert_eq!(code(&o), 1);
    assert!(err(&o).contains("vault not found"), "{}", err(&o));
}

#[test]
fn list_and_stop_show_and_end_a_running_session() {
    let env = Env::new();
    let (id, _) = env.add_vault("notes");
    env.script(json!({"turns": [[{"do": "wait_cancel"}]]}));
    let _daemon = env.start_daemon(Duration::from_secs(30));
    let mut c = env.client();
    let none = env.cerbo(&["agent", "list"]);
    assert!(out(&none).contains("No running agent sessions"), "{}", out(&none));

    let info: SessionInfo =
        serde_json::from_value(c.call(Call::SessionCreate { vault: id.clone(), agent: AgentKind::Opencode, resume_from: None, ephemeral: false }).unwrap()).unwrap();
    wait_for(&mut c, &info.id, |s| s != SessionState::Starting);
    c.call(Call::SessionSend { session: info.id.clone(), text: "work".into(), context: vec![] }).unwrap();

    let list = env.cerbo(&["agent", "list"]);
    let text = out(&list);
    assert!(text.contains(&info.id) && text.contains("notes") && text.contains("opencode") && text.contains("working"), "{text}");
    let json: Value = serde_json::from_str(&out(&env.cerbo(&["agent", "list", "--json"]))).unwrap();
    assert_eq!(json[0]["id"], info.id.as_str());
    assert_eq!(json[0]["state"], "working");

    let stop = env.cerbo(&["agent", "stop", &info.id[..8]]);
    assert!(stop.status.success(), "{}", err(&stop));
    assert!(out(&stop).contains(&info.id));
    let after = env.cerbo(&["agent", "list"]);
    assert!(out(&after).contains("No running agent sessions"), "{}", out(&after));
    let all = env.cerbo(&["agent", "list", "--all"]);
    assert!(out(&all).contains("ended"), "{}", out(&all));
    let missing = env.cerbo(&["agent", "stop", "zzzz"]);
    assert!(!missing.status.success());
    assert!(err(&missing).contains("no agent session"), "{}", err(&missing));
}

/// The shim runs inside the sandbox and reaches the daemon only through the
/// session socket, which is bound into it; the vault itself is read-only there.
#[test]
fn under_bubblewrap_the_mcp_shim_changes_a_page_through_the_session_socket() {
    let Some(bwrap) = std::env::var_os("CERBO_BWRAP").filter(|v| !v.is_empty()) else {
        eprintln!("SKIPPED: CERBO_BWRAP is not set (the devshell sets it)");
        return;
    };
    if let Err(e) = cerboid::sandbox::confinement(cerbo_core::agent::SandboxMode::Bwrap, Some(bwrap.clone())) {
        eprintln!("SKIPPED: no usable sandbox here: {e}");
        return;
    }
    let env = Env::new();
    // The sandbox is on again (the default).
    std::fs::write(env.ctx.config_dir.join("agents.toml"), format!("[agent]\nopencode = \"{}\"\n", fake_agent().display())).unwrap();
    let (id, root) = env.add_vault("notes");
    let page = env.page(&root, "Inbox", "old\n");
    env.set(&id, AgentPermission::Auto);
    env.script(json!({"turns": [[
        {"do": "write_file", "path": "direct.md", "contents": "x"},
        {"do": "mcp_call", "tool": "edit_page", "arguments": {"page": page, "content": "through the socket\n"}},
        {"do": "text", "text": "done"}
    ]]}));
    let _daemon = env.start_daemon_with(Duration::from_secs(30), Some(bwrap));
    let o = env.run("notes", &["--json"], "edit");
    assert_eq!(code(&o), 0, "{} {}", out(&o), err(&o));
    let v: Value = serde_json::from_str(out(&o).trim()).unwrap();
    let text = v["text"].as_str().unwrap();
    assert!(text.contains("write_file direct.md: error"), "{text}");
    assert!(!root.join("direct.md").exists());
    assert_eq!(env.content(&root, &page), "through the socket\n", "{text}");
}

/// Smoke test with the real OpenCode, and no prompt: the daemon starts it in
/// the bubblewrap sandbox with an isolated home (never the real one, no login
/// file), and the session has to come up (`initialize` and `session/new`
/// succeed). OpenCode runs through a wrapper that adds `--print-logs` and keeps
/// its stderr in the agent home, where the test looks for the `cerbo` MCP
/// server.
///
/// Ignored by default: it needs the real `opencode` and a usable `CERBO_BWRAP`,
/// and it prints what happened. Run it with
/// `CERBO_REAL_OPENCODE=$(command -v opencode) cargo test -p cerbo --test agent_cli -- --ignored --nocapture real_opencode`.
#[test]
#[ignore = "needs the real opencode (CERBO_REAL_OPENCODE) and bubblewrap (CERBO_BWRAP)"]
fn real_opencode_smoke_starts_a_sandboxed_session_without_a_prompt() {
    use std::os::unix::fs::PermissionsExt;
    let Some(real) = std::env::var_os("CERBO_REAL_OPENCODE").map(PathBuf::from) else {
        eprintln!("SKIPPED: CERBO_REAL_OPENCODE is not set");
        return;
    };
    let Some(bwrap) = std::env::var_os("CERBO_BWRAP").filter(|v| !v.is_empty()) else {
        eprintln!("SKIPPED: CERBO_BWRAP is not set");
        return;
    };
    cerboid::sandbox::confinement(cerbo_core::agent::SandboxMode::Bwrap, Some(bwrap.clone())).expect("a usable sandbox");

    let env = Env::new();
    let real = std::fs::canonicalize(real).unwrap();
    // In a directory of its own: the sandbox binds the install root of the agent.
    let wrapper = env.xdg.path().join("opencode-install").join("bin").join("opencode-logged");
    std::fs::create_dir_all(wrapper.parent().unwrap()).unwrap();
    std::fs::write(&wrapper, format!("#!/bin/sh\nexec '{}' --print-logs \"$@\" 2>>\"$HOME/opencode-stderr.log\"\n", real.display())).unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
    // The sandbox is on (the default); no login file exists anywhere.
    std::fs::write(env.ctx.config_dir.join("agents.toml"), format!("[agent]\nopencode = \"{}\"\n", wrapper.display())).unwrap();
    std::fs::remove_file(env.xdg.path().join("data/opencode/auth.json")).unwrap();
    let (id, _) = env.add_vault("notes");
    let _daemon = env.start_daemon_with(Duration::from_secs(30), Some(bwrap));

    let mut c = env.client();
    let agents = c.call(Call::AgentList {}).unwrap();
    eprintln!("agent_list: {agents}");
    // Without a login the daemon refuses before it starts anything.
    let refused = c.call(Call::SessionCreate { vault: id.clone(), agent: AgentKind::Opencode, resume_from: None, ephemeral: false }).unwrap_err();
    eprintln!("without a login: {refused:?}");
    assert!(refused.message.contains("not signed in"), "{refused:?}");
    // A placeholder login file in the isolated data directory (never the real
    // one): the daemon binds it into the sandbox and starts the agent.
    std::fs::write(env.xdg.path().join("data/opencode/auth.json"), "{}").unwrap();
    let created = c.call(Call::SessionCreate { vault: id, agent: AgentKind::Opencode, resume_from: None, ephemeral: false });
    // Creating answers at once; the start finishes in the background.
    if let Ok(started) = &created {
        let started: SessionInfo = serde_json::from_value(started.clone()).unwrap();
        let state = wait_for(&mut c, &started.id, |s| s != SessionState::Starting);
        eprintln!("state after the start: {state:?}");
        assert_eq!(state, SessionState::Idle, "the agent did not come up");
    }
    let log = std::fs::read_to_string(env.xdg.path().join("data/cerbo/agents/opencode/home/opencode-stderr.log")).unwrap_or_default();
    // Only the lines about MCP, whatever else the agent logs.
    let mcp: Vec<&str> = log.lines().filter(|l| l.to_lowercase().contains("mcp") || l.contains("cerbo")).collect();
    eprintln!("stderr: {} lines, {} about mcp/cerbo:\n{}", log.lines().count(), mcp.len(), mcp.join("\n"));
    let info: SessionInfo = serde_json::from_value(created.unwrap_or_else(|e| panic!("session create failed: {e:?}\nstderr:\n{log}"))).unwrap();
    eprintln!("session: state {:?}, sandboxed {}, warning {:?}", info.state, info.sandboxed, info.warning);
    assert!(info.sandboxed);
    assert!(!mcp.is_empty(), "nothing about the cerbo MCP server in the agent's log:\n{log}");
    let _ = c.call(Call::SessionEnd { session: info.id });
}
