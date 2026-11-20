//! Host access end to end: a real daemon, an agent connection and a person's
//! connection over temporary vaults and temporary folders outside them.

use cerbo_core::agent::{self, AgentPermission};
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::client::Client;
use cerbo_core::rpc::{ApprovalInfo, Call, ClientKind, CODE_FORBIDDEN, CODE_REJECTED, CODE_TIMED_OUT};
use cerbo_core::semantic::{LazyEmbedder, MockEmbedder};
use cerbo_core::CerboContext;
use cerboid::{Config, Running};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Env {
    tmp: tempfile::TempDir,
    ctx: CerboContext,
    paths: DaemonPaths,
    vault: String,
    /// Real, canonical folders outside the vault.
    inbox: PathBuf,
    out: PathBuf,
    elsewhere: PathBuf,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(tmp.path()).unwrap();
    let ctx = CerboContext { config_dir: root.join("config"), cache_dir: root.join("cache") };
    let paths = DaemonPaths::in_dir(root.join("run").join("cerbo"));
    let files = root.join("files");
    let (inbox, out, elsewhere) = (files.join("inbox"), files.join("out"), files.join("elsewhere"));
    for dir in [&inbox, &out, &elsewhere] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let vault_path = root.join("vaults/main");
    let (vault, _) = cerbo_core::vault::vault_create(&ctx, &vault_path, "main".to_string(), false).unwrap();
    Env { tmp, ctx, paths, vault: vault.id, inbox, out, elsewhere }
}

impl Env {
    fn start(&self, approval_timeout: Option<Duration>) -> Running {
        let mut config = Config::new(self.ctx.clone(), self.paths.clone());
        config.registry_poll = Duration::from_millis(50);
        config.event_quiet = Duration::from_millis(30);
        config.approval_timeout = approval_timeout;
        config.embedder = Arc::new(|_| {
            let dim = cerbo_core::semantic::Embedder::dim(&MockEmbedder::default());
            LazyEmbedder::new(dim, || Ok(Box::new(MockEmbedder::default())))
        });
        cerboid::spawn(config).unwrap().expect("no other daemon runs")
    }

    fn person(&self) -> Client {
        Client::connect(&self.paths.socket, ClientKind::Cli, Some("person")).unwrap()
    }

    fn agent(&self) -> Client {
        Client::connect_bound(&self.paths.socket, ClientKind::Agent, Some("opencode"), &self.vault).unwrap()
    }

    /// Host access on, with the standard list: `inbox` read, `out` write.
    fn host_on(&self) {
        agent::set_host_enabled(&self.ctx, &self.vault, true).unwrap();
        agent::add_host_path(&self.ctx, &self.vault, self.inbox.clone(), false).unwrap();
        agent::add_host_path(&self.ctx, &self.vault, self.out.clone(), true).unwrap();
    }

    fn log(&self) -> Vec<agent::ChangeEntry> {
        agent::read_changes(&self.ctx, &self.vault, 50).unwrap()
    }
}

fn read(env: &Env, path: &std::path::Path) -> Call {
    Call::HostRead { vault: env.vault.clone(), path: path.to_string_lossy().into(), offset: 0, limit: None }
}

fn write(env: &Env, path: &std::path::Path, content: &str) -> Call {
    Call::HostWrite { vault: env.vault.clone(), path: path.to_string_lossy().into(), content: content.into() }
}

fn wait_queue(person: &mut Client, count: usize) -> Vec<ApprovalInfo> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let list: Vec<ApprovalInfo> =
            serde_json::from_value(person.call(Call::ApprovalList { vault: None }).unwrap()).unwrap();
        if list.len() == count {
            return list;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("the approval queue never held {count} requests");
}

#[test]
fn host_access_is_off_by_default_and_the_agent_is_told_so() {
    let env = env();
    std::fs::write(env.inbox.join("a.txt"), "hello").unwrap();
    let _daemon = env.start(None);
    let mut agent = env.agent();
    let err = agent.call(read(&env, &env.inbox.join("a.txt"))).unwrap_err();
    assert_eq!(err.code, CODE_FORBIDDEN);
    assert!(err.message.contains("off"), "{}", err.message);
    let state = agent.call(Call::Permission { vault: env.vault.clone() }).unwrap();
    assert_eq!(state["host"], false);
    assert_eq!(env.log()[0].outcome.as_deref(), Some("refused"));
}

#[test]
fn a_listed_path_is_open_within_its_mode_and_every_action_is_logged() {
    let env = env();
    env.host_on();
    std::fs::write(env.inbox.join("a.txt"), "hello").unwrap();
    let _daemon = env.start(None);
    let mut agent = env.agent();
    assert_eq!(agent.call(Call::Permission { vault: env.vault.clone() }).unwrap()["host"], true);

    let got = agent.call(read(&env, &env.inbox.join("a.txt"))).unwrap();
    assert_eq!(got["content"], "hello");

    // Write under a read path fails and does not ask anybody.
    let err = agent.call(write(&env, &env.inbox.join("b.txt"), "x")).unwrap_err();
    assert_eq!(err.code, CODE_FORBIDDEN);
    assert!(!env.inbox.join("b.txt").exists());

    let done = agent.call(write(&env, &env.out.join("c.txt"), "result")).unwrap();
    assert_eq!(done["created"], true);
    assert_eq!(std::fs::read_to_string(env.out.join("c.txt")).unwrap(), "result");

    let list = agent
        .call(Call::HostList { vault: env.vault.clone(), path: env.out.to_string_lossy().into() })
        .unwrap();
    assert_eq!(list["entries"][0]["name"], "c.txt");

    let log = env.log();
    let outcomes: Vec<(&str, &str)> =
        log.iter().map(|e| (e.action.as_str(), e.outcome.as_deref().unwrap_or(""))).collect();
    assert_eq!(outcomes, [("host_read", "done"), ("host_write", "refused"), ("host_write", "done"), ("host_list", "done")]);
    assert!(log.iter().all(|e| !e.approved));
    assert_eq!(log[2].page, env.out.join("c.txt").to_string_lossy());
}

#[test]
fn a_path_outside_the_list_waits_for_a_person_even_in_an_auto_vault() {
    let env = env();
    env.host_on();
    agent::set_permission(&env.ctx, &env.vault, AgentPermission::Auto).unwrap();
    std::fs::write(env.elsewhere.join("r.txt"), "secret-ish").unwrap();
    let _daemon = env.start(None);

    let mut agent = env.agent();
    let path = env.elsewhere.join("r.txt");
    let call = read(&env, &path);
    let thread = std::thread::spawn(move || agent.call(call));

    let mut person = env.person();
    let queue = wait_queue(&mut person, 1);
    assert_eq!(queue[0].action, "host read");
    assert!(queue[0].summary.contains(&path.to_string_lossy().to_string()), "{}", queue[0].summary);
    assert!(!thread.is_finished());

    person.call(Call::ApprovalAnswer { approval: queue[0].id, approve: true }).unwrap();
    let got = thread.join().unwrap().expect("approved");
    assert_eq!(got["content"], "secret-ish");
    let log = env.log();
    assert_eq!((log[0].outcome.as_deref(), log[0].approved), (Some("done"), true));
}

#[test]
fn a_rejected_write_touches_nothing_and_the_agent_is_told() {
    let env = env();
    env.host_on();
    let _daemon = env.start(None);
    let mut agent = env.agent();
    let target = env.elsewhere.join("n.txt");
    let call = write(&env, &target, "no");
    let thread = std::thread::spawn(move || agent.call(call));

    let mut person = env.person();
    let queue = wait_queue(&mut person, 1);
    assert!(queue[0].summary.contains("Create the file"), "{}", queue[0].summary);
    person.call(Call::ApprovalAnswer { approval: queue[0].id, approve: false }).unwrap();
    assert_eq!(thread.join().unwrap().unwrap_err().code, CODE_REJECTED);
    assert!(!target.exists());
    assert_eq!(env.log()[0].outcome.as_deref(), Some("rejected"));
}

#[test]
fn a_request_nobody_answers_times_out_as_rejected() {
    let env = env();
    env.host_on();
    std::fs::write(env.elsewhere.join("t.txt"), "x").unwrap();
    let _daemon = env.start(Some(Duration::from_millis(300)));
    let mut agent = env.agent();
    let err = agent.call(read(&env, &env.elsewhere.join("t.txt"))).unwrap_err();
    assert_eq!(err.code, CODE_TIMED_OUT);
    assert_eq!(env.log()[0].outcome.as_deref(), Some("timed_out"));
}

#[test]
fn the_vault_and_other_vaults_are_refused_before_any_card_is_shown() {
    let env = env();
    env.host_on();
    // A write entry that covers everything, vaults included.
    agent::add_host_path(&env.ctx, &env.vault, env.tmp.path().canonicalize().unwrap(), true).unwrap();
    let other_path = env.tmp.path().canonicalize().unwrap().join("vaults/other");
    cerbo_core::vault::vault_create(&env.ctx, &other_path, "other".to_string(), false).unwrap();
    let own = env.tmp.path().canonicalize().unwrap().join("vaults/main/note.md");
    std::fs::write(&own, "x").unwrap();
    let _daemon = env.start(None);
    let mut agent = env.agent();

    let err = agent.call(write(&env, &own, "changed")).unwrap_err();
    assert_eq!(err.code, CODE_FORBIDDEN);
    assert!(err.message.contains("vault tools"), "{}", err.message);
    assert_eq!(std::fs::read_to_string(&own).unwrap(), "x");

    let err = agent.call(read(&env, &other_path.join(".cerbo"))).unwrap_err();
    assert_eq!(err.code, CODE_FORBIDDEN);
    assert!(err.message.contains("another vault"), "{}", err.message);

    let mut person = env.person();
    wait_queue(&mut person, 0);
}

#[test]
fn a_link_out_of_an_allowed_path_asks_instead_of_opening() {
    let env = env();
    env.host_on();
    std::fs::write(env.elsewhere.join("l.txt"), "behind the link").unwrap();
    std::os::unix::fs::symlink(&env.elsewhere, env.inbox.join("link")).unwrap();
    let _daemon = env.start(Some(Duration::from_millis(300)));
    let mut agent = env.agent();
    let err = agent.call(read(&env, &env.inbox.join("link/l.txt"))).unwrap_err();
    assert_eq!(err.code, CODE_TIMED_OUT, "the link target is outside the list, so it asked: {err:?}");
}

#[test]
fn tightening_while_a_request_waits_refuses_it_after_approval() {
    let env = env();
    env.host_on();
    let _daemon = env.start(None);
    let mut agent = env.agent();
    let target = env.elsewhere.join("w.txt");
    let call = write(&env, &target, "late");
    let thread = std::thread::spawn(move || agent.call(call));

    let mut person = env.person();
    let queue = wait_queue(&mut person, 1);
    agent::set_host_enabled(&env.ctx, &env.vault, false).unwrap();
    person.call(Call::ApprovalAnswer { approval: queue[0].id, approve: true }).unwrap();
    assert_eq!(thread.join().unwrap().unwrap_err().code, CODE_FORBIDDEN);
    assert!(!target.exists());
}

#[test]
fn removing_a_path_makes_the_next_call_ask() {
    let env = env();
    env.host_on();
    std::fs::write(env.inbox.join("a.txt"), "hello").unwrap();
    let _daemon = env.start(Some(Duration::from_millis(300)));
    let mut agent = env.agent();
    agent.call(read(&env, &env.inbox.join("a.txt"))).unwrap();
    assert!(agent::remove_host_path(&env.ctx, &env.vault, &env.inbox).unwrap());
    let err = agent.call(read(&env, &env.inbox.join("a.txt"))).unwrap_err();
    assert_eq!(err.code, CODE_TIMED_OUT);
}

#[test]
fn only_agent_connections_may_use_host_calls() {
    let env = env();
    env.host_on();
    std::fs::write(env.inbox.join("a.txt"), "hello").unwrap();
    let _daemon = env.start(None);
    let mut person = env.person();
    let err = person.call(read(&env, &env.inbox.join("a.txt"))).unwrap_err();
    assert_eq!(err.code, CODE_FORBIDDEN);
}

#[test]
fn vault_permission_and_host_access_are_independent() {
    let env = env();
    env.host_on();
    // The vault stays read-only; host access still works as set.
    let _daemon = env.start(None);
    let mut agent = env.agent();
    let state = agent.call(Call::Permission { vault: env.vault.clone() }).unwrap();
    assert_eq!(state["permission"], "read");
    assert_eq!(state["host"], true);
    agent.call(write(&env, &env.out.join("ok.txt"), "fine")).unwrap();
    let err = agent
        .call(Call::PageCreate { vault: env.vault.clone(), title: "t".into(), content: None, path: None })
        .unwrap_err();
    assert_eq!(err.code, cerbo_core::rpc::CODE_READ_ONLY);
}
