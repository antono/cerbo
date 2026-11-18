//! Agent permissions end to end: a real daemon, an agent connection and a
//! person's (CLI) connection over temporary vaults.

use cerbo_core::agent::{self, AgentPermission};
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::client::Client;
use cerbo_core::rpc::{
    ApprovalInfo, ApprovalOutcome, Call, ClientKind, Event, CODE_FORBIDDEN, CODE_READ_ONLY, CODE_REJECTED,
    CODE_BAD_REQUEST, CODE_CLIENT_TIMEOUT, CODE_CONFLICT, CODE_TIMED_OUT, CODE_TOO_MANY_PENDING, CODE_UNKNOWN_VAULT,
    MAX_PENDING_PER_AGENT,
};
use cerbo_core::semantic::{LazyEmbedder, MockEmbedder};
use cerbo_core::CerboContext;
use cerboid::{Config, Running};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Env {
    _tmp: tempfile::TempDir,
    ctx: CerboContext,
    paths: DaemonPaths,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = CerboContext { config_dir: tmp.path().join("config"), cache_dir: tmp.path().join("cache") };
    let paths = DaemonPaths::in_dir(tmp.path().join("run").join("cerbo"));
    Env { _tmp: tmp, ctx, paths }
}

impl Env {
    fn config(&self, approval_timeout: Option<Duration>) -> Config {
        let mut config = Config::new(self.ctx.clone(), self.paths.clone());
        config.registry_poll = Duration::from_millis(50);
        config.event_quiet = Duration::from_millis(30);
        config.approval_timeout = approval_timeout;
        config.embedder = Arc::new(|_| {
            let dim = cerbo_core::semantic::Embedder::dim(&MockEmbedder::default());
            LazyEmbedder::new(dim, || Ok(Box::new(MockEmbedder::default())))
        });
        config
    }

    fn start(&self) -> Running {
        cerboid::spawn(self.config(None)).unwrap().expect("no other daemon runs")
    }

    fn start_with_timeout(&self, timeout: Duration) -> Running {
        cerboid::spawn(self.config(Some(timeout))).unwrap().expect("no other daemon runs")
    }

    fn start_capped(&self, cap: usize) -> Running {
        let mut config = self.config(None);
        config.max_pending_per_agent = cap;
        cerboid::spawn(config).unwrap().expect("no other daemon runs")
    }

    fn connect(&self, kind: ClientKind, name: &str) -> Client {
        Client::connect(&self.paths.socket, kind, Some(name)).unwrap()
    }

    /// An agent connection bound to `vault`.
    fn agent(&self, name: &str, vault: &str) -> Client {
        Client::connect_bound(&self.paths.socket, ClientKind::Agent, Some(name), vault).unwrap()
    }

    fn add_vault(&self, name: &str) -> (String, PathBuf) {
        let path = self._tmp.path().join("vaults").join(name);
        let (vault, _) = cerbo_core::vault::vault_create(&self.ctx, &path, name.to_string(), false).unwrap();
        (vault.id, vault.path)
    }

    fn set(&self, vault: &str, level: AgentPermission) {
        agent::set_permission(&self.ctx, vault, level).unwrap();
    }
}

fn content_of(client: &mut Client, vault: &str, page: &str) -> String {
    let page = client.call(Call::PageRead { vault: vault.into(), page: page.into() }).unwrap();
    page["content"].as_str().unwrap().to_string()
}

fn edit_call(vault: &str, page: &str, content: &str) -> Call {
    Call::PageEdit { vault: vault.into(), page: page.into(), content: content.into() }
}

/// A page made by the person, for the agent to work on.
fn seed_page(env: &Env, vault: &str, title: &str, content: &str) -> String {
    let root = cerbo_core::vault::get_vault_path(&env.ctx, vault).unwrap();
    let ctx = CerboContext { config_dir: root.join(".cerbo"), cache_dir: env.ctx.cache_dir.clone() };
    let id = cerbo_core::page::page_create(&ctx, title.to_string()).unwrap();
    cerbo_core::page::page_write(&ctx, id.clone(), content.to_string()).unwrap();
    id
}

/// The queue as the person sees it, once it holds `count` requests.
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
fn a_vault_nobody_configured_is_read_only_for_an_agent() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "original\n");
    let _daemon = env.start();

    let mut agent = env.agent("opencode", &id);
    let permission = agent.call(Call::Permission { vault: id.clone() }).unwrap();
    assert_eq!(permission["permission"], "read");

    // Reads work.
    assert_eq!(content_of(&mut agent, &id, &page), "original\n");
    assert!(agent.call(Call::SearchText { vault: id.clone(), query: "orig".into(), limit: 5 }).is_ok());

    // Every kind of write is refused with a message that says why.
    let writes = [
        edit_call(&id, &page, "changed\n"),
        Call::PageCreate { vault: id.clone(), title: "New".into(), content: None, path: None },
        Call::PageRename { vault: id.clone(), page: page.clone(), title: "Other".into() },
        Call::PageMove { vault: id.clone(), page: page.clone(), path: Some("a".into()) },
        Call::PageTrash { vault: id.clone(), page: page.clone() },
        Call::LinkAdd { vault: id.clone(), from: page.clone(), to: page.clone() },
    ];
    for call in writes {
        let err = agent.call(call.clone()).unwrap_err();
        assert_eq!(err.code, CODE_READ_ONLY, "{call:?}");
        assert!(err.message.contains("read-only"), "{}", err.message);
    }
    assert_eq!(content_of(&mut agent, &id, &page), "original\n", "the page must stay unchanged");
    let pages = agent.call(Call::PageList { vault: id.clone() }).unwrap();
    assert_eq!(pages.as_array().unwrap().len(), 1, "no page was created");
}

#[test]
fn auto_applies_at_once_and_logs_the_change_and_read_takes_it_back_at_once() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "original\n");
    env.set(&id, AgentPermission::Auto);
    let _daemon = env.start();

    let mut agent = env.agent("opencode", &id);
    agent.call(edit_call(&id, &page, "by the agent\n")).unwrap();
    assert_eq!(content_of(&mut agent, &id, &page), "by the agent\n");

    let mut person = env.connect(ClientKind::Cli, "person");
    let log: Vec<agent::ChangeEntry> =
        serde_json::from_value(person.call(Call::AgentChanges { vault: id.clone(), limit: 10 }).unwrap()).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].agent, "opencode");
    assert_eq!(log[0].page, page);
    assert_eq!(log[0].action, "edit");
    assert!(!log[0].approved);
    assert!(log[0].time > 0);

    // The person's own edits are not agent changes and are not logged.
    person.call(edit_call(&id, &page, "by the person\n")).unwrap();
    let log: Vec<agent::ChangeEntry> =
        serde_json::from_value(person.call(Call::AgentChanges { vault: id.clone(), limit: 10 }).unwrap()).unwrap();
    assert_eq!(log.len(), 1);

    // Tighten while the agent is connected: its next write is refused.
    env.set(&id, AgentPermission::Read);
    let err = agent.call(edit_call(&id, &page, "again\n")).unwrap_err();
    assert_eq!(err.code, CODE_READ_ONLY);
    assert_eq!(content_of(&mut agent, &id, &page), "by the person\n");
}

#[test]
fn an_agent_delete_goes_to_the_trash() {
    let env = env();
    let (id, root) = env.add_vault("main");
    let page = seed_page(&env, &id, "Doomed", "x\n");
    env.set(&id, AgentPermission::Auto);
    let _daemon = env.start();

    let mut agent = env.agent("opencode", &id);
    agent.call(Call::PageTrash { vault: id.clone(), page: page.clone() }).unwrap();

    let trash = std::fs::read_dir(root.join(".cerbo").join("trash")).unwrap().flatten().collect::<Vec<_>>();
    assert!(
        trash.iter().any(|e| e.file_name().to_string_lossy().contains(&page)),
        "the page must be recoverable from the trash"
    );
    assert!(!root.join(".cerbo").join("objects").join(&page).exists());
}

#[test]
fn approve_waits_for_a_person_and_applies_only_after_approval() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "line one\nline two\n");
    env.set(&id, AgentPermission::Approve);
    let _daemon = env.start();

    let mut person = env.connect(ClientKind::App, "app");
    person.call(Call::Subscribe { vault: None }).unwrap();

    let mut agent = env.agent("opencode", &id);
    let (vault, page_id) = (id.clone(), page.clone());
    let agent_thread =
        std::thread::spawn(move || agent.call(edit_call(&vault, &page_id, "line one\nline 2\n")).map(|_| agent));

    // The subscriber is told, and the queue shows what will change.
    let event = loop {
        let event = person.next_event(Duration::from_secs(10)).unwrap().expect("no approval event");
        if matches!(event, Event::ApprovalRequested { .. }) {
            break event;
        }
    };
    let Event::ApprovalRequested { approval } = event else { unreachable!() };
    let queue = wait_queue(&mut person, 1);
    assert_eq!(queue[0].id, approval.id);
    assert_eq!(queue[0].agent, "opencode");
    assert_eq!(queue[0].action, "edit");
    assert_eq!(queue[0].page.as_deref(), Some(page.as_str()));
    let diff = queue[0].diff.as_deref().expect("an edit shows its difference");
    assert!(diff.contains("-line two") && diff.contains("+line 2"), "{diff}");
    assert!(queue[0].deadline > queue[0].requested);

    // Nothing applied yet, and the agent is still waiting.
    assert_eq!(content_of(&mut person, &id, &page), "line one\nline two\n");
    assert!(!agent_thread.is_finished());

    person.call(Call::ApprovalAnswer { approval: approval.id, approve: true }).unwrap();
    let mut agent = agent_thread.join().unwrap().expect("the agent is told the edit succeeded");
    assert_eq!(content_of(&mut agent, &id, &page), "line one\nline 2\n");
    wait_queue(&mut person, 0);

    let log: Vec<agent::ChangeEntry> =
        serde_json::from_value(person.call(Call::AgentChanges { vault: id.clone(), limit: 10 }).unwrap()).unwrap();
    assert_eq!(log.len(), 1);
    assert!(log[0].approved);
}

#[test]
fn a_rejected_change_leaves_the_vault_unchanged_and_tells_the_agent() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "keep me\n");
    env.set(&id, AgentPermission::Approve);
    let _daemon = env.start();

    let mut agent = env.agent("opencode", &id);
    let (vault, page_id) = (id.clone(), page.clone());
    let agent_thread = std::thread::spawn(move || agent.call(edit_call(&vault, &page_id, "gone\n")));

    let mut person = env.connect(ClientKind::Cli, "person");
    let queue = wait_queue(&mut person, 1);
    person.call(Call::ApprovalAnswer { approval: queue[0].id, approve: false }).unwrap();

    let err = agent_thread.join().unwrap().unwrap_err();
    assert_eq!(err.code, CODE_REJECTED);
    assert!(err.message.contains("rejected"), "{}", err.message);
    assert_eq!(content_of(&mut person, &id, &page), "keep me\n");
    assert!(
        serde_json::from_value::<Vec<agent::ChangeEntry>>(
            person.call(Call::AgentChanges { vault: id.clone(), limit: 10 }).unwrap()
        )
        .unwrap()
        .is_empty()
    );
}

#[test]
fn a_request_nobody_answers_times_out_as_rejected() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "keep me\n");
    env.set(&id, AgentPermission::Approve);
    let _daemon = env.start_with_timeout(Duration::from_millis(300));

    let mut agent = env.agent("cron-job", &id);
    let started = Instant::now();
    let err = agent.call(edit_call(&id, &page, "late\n")).unwrap_err();
    assert_eq!(err.code, CODE_TIMED_OUT);
    assert!(err.message.contains("in time"), "{}", err.message);
    assert!(started.elapsed() < Duration::from_secs(10));

    let mut person = env.connect(ClientKind::Cli, "person");
    wait_queue(&mut person, 0);
    assert_eq!(content_of(&mut person, &id, &page), "keep me\n");
}

#[test]
fn a_client_that_connects_later_sees_and_answers_the_queue() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "before\n");
    env.set(&id, AgentPermission::Approve);
    let _daemon = env.start();

    let mut agent = env.agent("opencode", &id);
    let (vault, page_id) = (id.clone(), page.clone());
    let agent_thread = std::thread::spawn(move || agent.call(edit_call(&vault, &page_id, "after\n")));

    // Nobody listened when it was asked; the app opens later.
    std::thread::sleep(Duration::from_millis(300));
    let mut app = env.connect(ClientKind::App, "app");
    let queue = wait_queue(&mut app, 1);
    app.call(Call::ApprovalAnswer { approval: queue[0].id, approve: true }).unwrap();

    agent_thread.join().unwrap().unwrap();
    assert_eq!(content_of(&mut app, &id, &page), "after\n");

    // An answered request cannot be answered again.
    assert!(app.call(Call::ApprovalAnswer { approval: queue[0].id, approve: true }).is_err());
}

#[test]
fn tightening_while_a_request_waits_refuses_it_after_approval() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "before\n");
    env.set(&id, AgentPermission::Approve);
    let _daemon = env.start();

    let mut agent = env.agent("opencode", &id);
    let (vault, page_id) = (id.clone(), page.clone());
    let agent_thread = std::thread::spawn(move || agent.call(edit_call(&vault, &page_id, "after\n")));

    let mut person = env.connect(ClientKind::Cli, "person");
    let queue = wait_queue(&mut person, 1);
    env.set(&id, AgentPermission::Read);
    person.call(Call::ApprovalAnswer { approval: queue[0].id, approve: true }).unwrap();

    let err = agent_thread.join().unwrap().unwrap_err();
    assert_eq!(err.code, CODE_READ_ONLY);
    assert_eq!(content_of(&mut person, &id, &page), "before\n");
}

#[test]
fn an_agent_connection_cannot_answer_approvals_stop_the_daemon_or_read_the_log() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let _daemon = env.start();

    let mut agent = env.agent("opencode", &id);
    for call in [
        Call::ApprovalAnswer { approval: 1, approve: true },
        Call::ApprovalList { vault: None },
        Call::AgentChanges { vault: id.clone(), limit: 5 },
        Call::Stop,
    ] {
        let err = agent.call(call.clone()).unwrap_err();
        assert_eq!(err.code, CODE_FORBIDDEN, "{call:?}");
    }
    // Still running.
    assert!(agent.call(Call::Ping).is_ok());
}

#[test]
fn stopping_ends_waiting_requests_instead_of_hanging() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "before\n");
    env.set(&id, AgentPermission::Approve);
    let daemon = env.start();

    let mut agent = env.agent("opencode", &id);
    let (vault, page_id) = (id.clone(), page.clone());
    let agent_thread = std::thread::spawn(move || agent.call(edit_call(&vault, &page_id, "after\n")));
    let mut person = env.connect(ClientKind::Cli, "person");
    wait_queue(&mut person, 1);

    let started = Instant::now();
    daemon.stop().unwrap();
    assert!(agent_thread.join().unwrap().is_err());
    assert!(started.elapsed() < Duration::from_secs(10), "a stop must not wait for the approval deadline");
    let _ = ApprovalOutcome::Cancelled;
}

// ── Waiting for approval ─────────────────────────────────────────────────────

#[test]
fn an_agent_can_learn_how_long_a_write_may_wait() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let _daemon = env.start_with_timeout(Duration::from_secs(137));
    let mut agent = env.agent("opencode", &id);
    let permission = agent.call(Call::Permission { vault: id.clone() }).unwrap();
    assert_eq!(permission["approval_timeout_secs"], 137);
}

#[test]
fn a_late_approval_still_reaches_an_agent_that_waits_for_the_deadline() {
    // The client waits as long as it is told to (the approval deadline plus a
    // margin), not for a fixed short limit: an answer that comes late is
    // still the answer the agent sees.
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "before\n");
    env.set(&id, AgentPermission::Approve);
    let _daemon = env.start_with_timeout(Duration::from_secs(30));

    let mut agent = env.agent("opencode", &id);
    let (vault, page_id) = (id.clone(), page.clone());
    let agent_thread = std::thread::spawn(move || {
        agent.call_with_timeout(edit_call(&vault, &page_id, "after\n"), Duration::from_secs(30 + 5))
    });
    let mut person = env.connect(ClientKind::Cli, "person");
    let queue = wait_queue(&mut person, 1);
    std::thread::sleep(Duration::from_millis(1500));
    person.call(Call::ApprovalAnswer { approval: queue[0].id, approve: true }).unwrap();
    agent_thread.join().unwrap().expect("the late approval is reported as a success");
    assert_eq!(content_of(&mut person, &id, &page), "after\n");
}

#[test]
fn a_client_that_gives_up_waiting_is_told_so_and_its_request_never_applies() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "before\n");
    env.set(&id, AgentPermission::Approve);
    let _daemon = env.start();

    let mut agent = env.agent("opencode", &id);
    let err = agent.call_with_timeout(edit_call(&id, &page, "after\n"), Duration::from_millis(300)).unwrap_err();
    assert_eq!(err.code, CODE_CLIENT_TIMEOUT);

    // The agent closes the connection (the shim does this on a timeout): the
    // daemon rejects the request that was still pending.
    let mut person = env.connect(ClientKind::Cli, "person");
    let queue = wait_queue(&mut person, 1);
    drop(agent);
    wait_queue(&mut person, 0);
    assert!(person.call(Call::ApprovalAnswer { approval: queue[0].id, approve: true }).is_err());
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(content_of(&mut person, &id, &page), "before\n", "a cancelled request is never applied");
}

#[test]
fn an_agent_that_hangs_up_cancels_what_it_left_pending() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "before\n");
    env.set(&id, AgentPermission::Approve);
    let _daemon = env.start();

    let mut person = env.connect(ClientKind::App, "app");
    person.call(Call::Subscribe { vault: None }).unwrap();
    let mut agent = env.agent("opencode", &id);
    agent.send(edit_call(&id, &page, "after\n")).unwrap();
    agent.send(Call::PageCreate { vault: id.clone(), title: "Extra".into(), content: None, path: None }).unwrap();
    wait_queue(&mut person, 2);

    drop(agent);
    wait_queue(&mut person, 0);
    // The people watching are told each request ended, as cancelled.
    let mut cancelled = 0;
    while cancelled < 2 {
        if let Event::ApprovalResolved { outcome: ApprovalOutcome::Cancelled, .. } =
            person.next_event(Duration::from_secs(10)).unwrap().expect("no resolve event")
        {
            cancelled += 1;
        }
    }
    assert_eq!(content_of(&mut person, &id, &page), "before\n");
    let pages = person.call(Call::PageList { vault: id.clone() }).unwrap();
    assert_eq!(pages.as_array().unwrap().len(), 1, "the page the agent asked for was not created");
}

#[test]
fn an_edit_is_refused_when_the_page_changed_after_the_agent_asked() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "line one\n");
    env.set(&id, AgentPermission::Approve);
    let _daemon = env.start();

    let mut agent = env.agent("opencode", &id);
    let (vault, page_id) = (id.clone(), page.clone());
    let agent_thread = std::thread::spawn(move || agent.call(edit_call(&vault, &page_id, "the agent's whole page\n")));
    let mut person = env.connect(ClientKind::Cli, "person");
    let queue = wait_queue(&mut person, 1);

    // The person edits the page while the request waits, then approves.
    person.call(edit_call(&id, &page, "line one\nthe person's own line\n")).unwrap();
    person.call(Call::ApprovalAnswer { approval: queue[0].id, approve: true }).unwrap();

    let err = agent_thread.join().unwrap().unwrap_err();
    assert_eq!(err.code, CODE_CONFLICT);
    assert!(err.message.contains("changed"), "{}", err.message);
    assert_eq!(content_of(&mut person, &id, &page), "line one\nthe person's own line\n", "the person's edit stays");
    let log: Vec<agent::ChangeEntry> =
        serde_json::from_value(person.call(Call::AgentChanges { vault: id.clone(), limit: 10 }).unwrap()).unwrap();
    assert!(log.is_empty(), "a refused change is not recorded as applied");
}

#[test]
fn an_agent_connection_may_have_only_so_many_requests_waiting() {
    assert_eq!(MAX_PENDING_PER_AGENT, 137, "the documented number");
    let default_env = env();
    assert_eq!(Config::new(default_env.ctx.clone(), default_env.paths.clone()).max_pending_per_agent, 137);

    let env = env();
    let (id, _) = env.add_vault("main");
    let page = seed_page(&env, &id, "Hello", "before\n");
    env.set(&id, AgentPermission::Approve);
    let _daemon = env.start_capped(2);
    let rename = |title: &str| Call::PageRename { vault: id.clone(), page: page.clone(), title: title.into() };

    let mut person = env.connect(ClientKind::Cli, "person");
    let mut agent = env.agent("opencode", &id);
    agent.send(rename("One")).unwrap();
    agent.send(rename("Two")).unwrap();
    let queue = wait_queue(&mut person, 2);

    let err = agent.call(rename("Three")).unwrap_err();
    assert_eq!(err.code, CODE_TOO_MANY_PENDING);
    assert!(err.message.contains('2'), "{}", err.message);
    assert_eq!(person.call(Call::ApprovalList { vault: None }).unwrap().as_array().unwrap().len(), 2);

    // The limit is per connection: another agent connection still queues.
    let mut other = env.agent("other", &id);
    other.send(rename("Four")).unwrap();
    wait_queue(&mut person, 3);

    // An answered request frees a place.
    person.call(Call::ApprovalAnswer { approval: queue[0].id, approve: false }).unwrap();
    wait_queue(&mut person, 2);
    agent.send(rename("Five")).unwrap();
    wait_queue(&mut person, 3);
}

// ── Bound to one vault ───────────────────────────────────────────────────────

#[test]
fn an_agent_must_name_its_vault_and_it_must_exist() {
    use cerbo_core::rpc::client::ConnectError;
    let env = env();
    let (id, _) = env.add_vault("main");
    let _daemon = env.start();

    match Client::connect(&env.paths.socket, ClientKind::Agent, Some("opencode")).err().expect("refused") {
        ConnectError::Rejected(e) => assert_eq!(e.code, CODE_BAD_REQUEST),
        other => panic!("{other:?}"),
    }
    match Client::connect_bound(&env.paths.socket, ClientKind::Agent, Some("opencode"), "nowhere").err().unwrap() {
        ConnectError::Rejected(e) => assert_eq!(e.code, CODE_UNKNOWN_VAULT),
        other => panic!("{other:?}"),
    }
    // By name is as good as by id.
    let mut agent = Client::connect_bound(&env.paths.socket, ClientKind::Agent, Some("opencode"), "main").unwrap();
    assert!(agent.call(Call::PageList { vault: id }).is_ok());
}

#[test]
fn an_agent_cannot_reach_a_vault_it_is_not_bound_to() {
    let env = env();
    let (a, a_root) = env.add_vault("a");
    let (b, b_root) = env.add_vault("b");
    let page_b = seed_page(&env, &b, "Secret", "in b\n");
    env.set(&a, AgentPermission::Auto);
    env.set(&b, AgentPermission::Auto);
    let _daemon = env.start();

    let mut agent = env.agent("opencode", &a);
    // Its own vault, however it names it.
    for selector in [a.clone(), "a".to_string(), a_root.to_string_lossy().into_owned()] {
        assert!(agent.call(Call::PageList { vault: selector.clone() }).is_ok(), "{selector}");
    }
    // Any other vault, by id, name or path, for reads and writes.
    for selector in [b.clone(), "b".to_string(), b_root.to_string_lossy().into_owned(), "nowhere".to_string()] {
        for call in [
            Call::PageList { vault: selector.clone() },
            Call::PageRead { vault: selector.clone(), page: page_b.clone() },
            Call::SearchText { vault: selector.clone(), query: "b".into(), limit: 5 },
            Call::Permission { vault: selector.clone() },
            edit_call(&selector, &page_b, "overwritten\n"),
            Call::PageCreate { vault: selector.clone(), title: "Planted".into(), content: None, path: None },
            Call::Subscribe { vault: Some(selector.clone()) },
        ] {
            let err = agent.call(call.clone()).unwrap_err();
            assert_eq!(err.code, CODE_FORBIDDEN, "{call:?}");
        }
    }
    let mut person = env.connect(ClientKind::Cli, "person");
    assert_eq!(content_of(&mut person, &b, &page_b), "in b\n");
    assert_eq!(person.call(Call::PageList { vault: b.clone() }).unwrap().as_array().unwrap().len(), 1);

    // It sees only its own vault in the list, and no daemon status.
    let vaults = agent.call(Call::Vaults).unwrap();
    let vaults = vaults.as_array().unwrap();
    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0]["id"], a.as_str());
    assert_eq!(agent.call(Call::Status).unwrap_err().code, CODE_FORBIDDEN);
    // The person, unchanged, sees both.
    assert_eq!(person.call(Call::Vaults).unwrap().as_array().unwrap().len(), 2);
}

#[test]
fn an_agent_subscription_carries_only_page_events_of_its_own_vault() {
    let env = env();
    let (a, _) = env.add_vault("a");
    let (b, _) = env.add_vault("b");
    let page_a = seed_page(&env, &a, "Mine", "a\n");
    let page_b = seed_page(&env, &b, "Other", "b\n");
    env.set(&a, AgentPermission::Approve);
    let _daemon = env.start();

    let mut person = env.connect(ClientKind::App, "app");
    person.call(Call::Subscribe { vault: None }).unwrap();
    let mut watcher = env.agent("watcher", &a);
    watcher.call(Call::Subscribe { vault: None }).unwrap();

    // Events the agent must not hear: a change in the other vault, a vault
    // added (VaultsChanged) and an approval request in its own vault.
    person.call(edit_call(&b, &page_b, "b changed\n")).unwrap();
    env.add_vault("c");
    let mut asker = env.agent("asker", &a);
    let (vault, page) = (a.clone(), page_a.clone());
    let asker_thread = std::thread::spawn(move || asker.call(edit_call(&vault, &page, "a changed\n")));
    let queue = wait_queue(&mut person, 1);
    // Everything above reached the person.
    let mut saw_vaults_changed = false;
    let mut saw_b = false;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !(saw_vaults_changed && saw_b) {
        match person.next_event(Duration::from_millis(200)).unwrap() {
            Some(Event::VaultsChanged) => saw_vaults_changed = true,
            Some(Event::PageChanged { vault, .. }) if vault == b => saw_b = true,
            _ => {}
        }
    }
    assert!(saw_vaults_changed && saw_b, "the person's subscription works");

    // A change in its own vault does reach the agent.
    person.call(Call::ApprovalAnswer { approval: queue[0].id, approve: true }).unwrap();
    asker_thread.join().unwrap().unwrap();
    let mut heard = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        match watcher.next_event(Duration::from_millis(500)).unwrap() {
            Some(event) => heard.push(event),
            None if !heard.is_empty() => break,
            None => {}
        }
    }
    assert!(!heard.is_empty(), "the agent hears its own vault");
    for event in &heard {
        assert!(matches!(event, Event::PageChanged { vault, .. } if *vault == a), "{event:?}");
    }
}
