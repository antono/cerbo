//! Integration tests: a real daemon on a real socket in a temporary runtime
//! directory, driven through the blocking client, over temporary vaults.

use cerbo_core::app_lock::AppLock;
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::client::{Client, ConnectError};
use cerbo_core::rpc::{
    Call, ChangeKind, ClientKind, Event, Origin, StatusInfo, CODE_PROTOCOL_MISMATCH, CODE_UNKNOWN_VAULT, PROTOCOL_VERSION,
};
use cerbo_core::semantic::{LazyEmbedder, MockEmbedder};
use cerbo_core::CerboContext;
use cerboid::{Config, Running};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
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
    fn config(&self) -> Config {
        let mut config = Config::new(self.ctx.clone(), self.paths.clone());
        config.registry_poll = Duration::from_millis(50);
        config.event_quiet = Duration::from_millis(30);
        config.embedder = Arc::new(|_| {
            let dim = cerbo_core::semantic::Embedder::dim(&MockEmbedder::default());
            LazyEmbedder::new(dim, || Ok(Box::new(MockEmbedder::default())))
        });
        config
    }

    fn start(&self) -> Running {
        cerboid::spawn(self.config()).unwrap().expect("no other daemon runs")
    }

    fn connect(&self, kind: ClientKind) -> Client {
        Client::connect(&self.paths.socket, kind, Some("test")).unwrap()
    }

    /// A registered, initialised vault. Returns its id and root.
    fn add_vault(&self, name: &str) -> (String, PathBuf) {
        let path = self._tmp.path().join("vaults").join(name);
        let (vault, _) = cerbo_core::vault::vault_create(&self.ctx, &path, name.to_string(), false).unwrap();
        (vault.id, vault.path)
    }

    /// The context page functions use for the vault at `root`.
    fn page_ctx(&self, root: &std::path::Path) -> CerboContext {
        CerboContext { config_dir: root.join(".cerbo"), cache_dir: self.ctx.cache_dir.clone() }
    }
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_else(|| panic!("no string {key} in {value}"))
}

fn create(client: &mut Client, vault: &str, title: &str) -> String {
    let made = client
        .call(Call::PageCreate { vault: vault.into(), title: title.into(), content: None, path: None })
        .unwrap();
    text(&made, "uuid").to_string()
}

fn read(client: &mut Client, vault: &str, page: &str) -> Value {
    client.call(Call::PageRead { vault: vault.into(), page: page.into() }).unwrap()
}

fn edit(client: &mut Client, vault: &str, page: &str, content: &str) {
    client
        .call(Call::PageEdit { vault: vault.into(), page: page.into(), content: content.into() })
        .unwrap();
}

/// The first event matching `want`, within a few seconds.
fn wait_event(client: &mut Client, want: impl Fn(&Event) -> bool) -> Event {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Some(event) = client.next_event(Duration::from_millis(200)).unwrap()
            && want(&event)
        {
            return event;
        }
    }
    panic!("no matching event arrived");
}

fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if cond() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for {what}");
}

// ── Handshake and version ────────────────────────────────────────────────────

#[test]
fn connect_handshake_and_status() {
    let env = env();
    let (id, _) = env.add_vault("main");
    let _daemon = env.start();

    let mut client = env.connect(ClientKind::Cli);
    assert_eq!(client.welcome().protocol, PROTOCOL_VERSION);
    assert_eq!(client.welcome().pid, std::process::id());
    assert_eq!(client.call(Call::Ping).unwrap(), json!("pong"));

    let status: StatusInfo = serde_json::from_value(client.call(Call::Status).unwrap()).unwrap();
    assert_eq!(status.protocol, PROTOCOL_VERSION);
    assert_eq!(status.vaults.len(), 1);
    assert_eq!(status.vaults[0].id, id);
    assert_eq!(status.vaults[0].name, "main");
}

#[test]
fn a_protocol_mismatch_is_refused_and_nothing_else_is_served() {
    let env = env();
    let _daemon = env.start();

    let err = Client::connect_with_protocol(&env.paths.socket, ClientKind::App, None, PROTOCOL_VERSION + 1)
        .err()
        .expect("a newer client must be refused");
    match err {
        ConnectError::VersionMismatch(e) => {
            assert_eq!(e.code, CODE_PROTOCOL_MISMATCH);
            assert!(e.message.contains("restart the daemon"), "{}", e.message);
        }
        other => panic!("expected a version mismatch, got {other:?}"),
    }

    // Even a client that ignores the refusal gets no answers: the daemon
    // closed the connection after the error line.
    let mut raw = UnixStream::connect(&env.paths.socket).unwrap();
    raw.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    writeln!(raw, r#"{{"hello":{{"protocol":{},"client":"app"}}}}"#, PROTOCOL_VERSION + 1).unwrap();
    let mut lines = BufReader::new(raw.try_clone().unwrap());
    let mut first = String::new();
    lines.read_line(&mut first).unwrap();
    assert!(first.contains(CODE_PROTOCOL_MISMATCH), "{first}");
    let _ = writeln!(raw, r#"{{"id":1,"method":"ping"}}"#);
    let mut second = String::new();
    assert_eq!(lines.read_line(&mut second).unwrap_or(0), 0, "no answer after a refused hello: {second}");

    // The daemon is fine afterwards.
    assert_eq!(env.connect(ClientKind::Cli).call(Call::Ping).unwrap(), json!("pong"));
}

#[test]
fn a_connection_without_a_hello_is_closed() {
    let env = env();
    let _daemon = env.start();

    let mut raw = UnixStream::connect(&env.paths.socket).unwrap();
    raw.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    writeln!(raw, r#"{{"id":1,"method":"ping"}}"#).unwrap();
    let mut lines = BufReader::new(raw);
    let mut first = String::new();
    lines.read_line(&mut first).unwrap();
    assert!(first.contains("bad_request"), "{first}");
    let mut second = String::new();
    assert_eq!(lines.read_line(&mut second).unwrap_or(0), 0);
}

#[test]
fn a_bad_request_is_answered_and_the_connection_stays_usable() {
    let env = env();
    let _daemon = env.start();

    let mut raw = UnixStream::connect(&env.paths.socket).unwrap();
    raw.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut lines = BufReader::new(raw.try_clone().unwrap());
    let mut line = String::new();
    writeln!(raw, r#"{{"hello":{{"protocol":{PROTOCOL_VERSION},"client":"cli"}}}}"#).unwrap();
    lines.read_line(&mut line).unwrap();
    assert!(line.contains("welcome"), "{line}");

    writeln!(raw, r#"{{"id":7,"method":"no_such_method"}}"#).unwrap();
    line.clear();
    lines.read_line(&mut line).unwrap();
    let answer: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(answer["id"], 7);
    assert_eq!(answer["error"]["code"], "bad_request");

    writeln!(raw, "this is not json").unwrap();
    line.clear();
    lines.read_line(&mut line).unwrap();
    assert!(line.contains("bad_request"), "{line}");

    writeln!(raw, r#"{{"id":8,"method":"ping"}}"#).unwrap();
    line.clear();
    lines.read_line(&mut line).unwrap();
    let answer: Value = serde_json::from_str(&line).unwrap();
    assert_eq!((answer["id"].clone(), answer["result"].clone()), (json!(8), json!("pong")));
}

// ── Single instance and socket ───────────────────────────────────────────────

#[test]
fn a_second_daemon_exits_and_leaves_the_first_untouched() {
    let env = env();
    let first = env.start();

    let second = cerboid::spawn(env.config()).unwrap();
    assert!(second.is_none(), "a second daemon must not start");

    let mut client = env.connect(ClientKind::Cli);
    assert_eq!(client.call(Call::Ping).unwrap(), json!("pong"));
    first.stop().unwrap();
}

#[test]
fn a_stale_socket_after_a_crash_is_replaced() {
    let env = env();
    env.paths.prepare_dir().unwrap();
    // What a killed daemon leaves behind: a socket file nobody listens on.
    drop(std::os::unix::net::UnixListener::bind(&env.paths.socket).unwrap());
    assert!(env.paths.socket.exists());
    assert!(UnixStream::connect(&env.paths.socket).is_err());

    let _daemon = env.start();
    assert_eq!(env.connect(ClientKind::Cli).call(Call::Ping).unwrap(), json!("pong"));
}

#[test]
fn the_socket_is_reachable_by_its_owner_only() {
    let env = env();
    let _daemon = env.start();

    let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&env.paths.dir), 0o700);
    assert_eq!(mode(&env.paths.socket), 0o600);
}

#[test]
fn the_socket_goes_away_when_the_daemon_stops() {
    let env = env();
    let daemon = env.start();
    assert!(env.paths.socket.exists());
    daemon.stop().unwrap();
    assert!(!env.paths.socket.exists());
    assert!(matches!(
        Client::connect(&env.paths.socket, ClientKind::Cli, None),
        Err(ConnectError::Unavailable(_))
    ));
}

/// The `cerboid` binary end to end: it starts, a second copy exits with
/// status 0 saying so, and SIGTERM stops the first cleanly.
#[test]
fn the_binary_runs_once_and_stops_on_sigterm() {
    let env = env();
    let run_base = env._tmp.path().join("run");
    let command = || {
        let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_cerboid"));
        c.env("XDG_RUNTIME_DIR", &run_base)
            .env("XDG_CONFIG_HOME", env._tmp.path().join("xdg-config"))
            .env("XDG_CACHE_HOME", env._tmp.path().join("xdg-cache"));
        c
    };

    let mut first = command().stderr(std::process::Stdio::piped()).spawn().unwrap();
    wait_until("the daemon's socket", || env.paths.socket.exists());
    assert_eq!(env.connect(ClientKind::Cli).call(Call::Ping).unwrap(), json!("pong"));

    let second = command().output().unwrap();
    assert!(second.status.success(), "a second instance must exit with status 0");
    assert!(
        String::from_utf8_lossy(&second.stderr).contains("already running"),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(env.connect(ClientKind::Cli).call(Call::Ping).is_ok(), "the first daemon is untouched");

    let killed = std::process::Command::new("kill").arg("-TERM").arg(first.id().to_string()).status().unwrap();
    assert!(killed.success());
    let status = first.wait().unwrap();
    assert!(status.success(), "SIGTERM is a clean stop: {status:?}");
    assert!(!env.paths.socket.exists());
}

// ── Vault registry ───────────────────────────────────────────────────────────

#[test]
fn a_vault_added_or_removed_while_running_is_picked_up() {
    let env = env();
    let daemon = env.start();
    let mut client = env.connect(ClientKind::Cli);
    client.call(Call::Subscribe { vault: None }).unwrap();

    let none = client.call(Call::Vaults).unwrap();
    assert_eq!(none, json!([]));
    let err = client.call(Call::PageList { vault: "late".into() }).unwrap_err();
    assert_eq!(err.code, CODE_UNKNOWN_VAULT);

    let (id, _) = env.add_vault("late");
    // No restart: the new vault is served, by name and by id.
    assert_eq!(client.call(Call::PageList { vault: "late".into() }).unwrap(), json!([]));
    assert_eq!(client.call(Call::PageList { vault: id.clone() }).unwrap(), json!([]));
    wait_event(&mut client, |e| *e == Event::VaultsChanged);

    cerbo_core::vault::vault_remove(&env.ctx, id).unwrap();
    wait_until("the removed vault to go", || {
        client.call(Call::Vaults).unwrap() == json!([])
    });
    let err = client.call(Call::PageList { vault: "late".into() }).unwrap_err();
    assert_eq!(err.code, CODE_UNKNOWN_VAULT);
    daemon.stop().unwrap();
}

#[test]
fn every_vault_in_the_list_is_served() {
    let env = env();
    let (a, _) = env.add_vault("a");
    let (b, _) = env.add_vault("b");
    let _daemon = env.start();
    let mut client = env.connect(ClientKind::Cli);

    let page_a = create(&mut client, &a, "In A");
    let page_b = create(&mut client, &b, "In B");
    assert_eq!(text(&read(&mut client, &a, &page_a), "title"), "In A");
    assert_eq!(text(&read(&mut client, &b, &page_b), "title"), "In B");
    // A page lives in its own vault only.
    assert!(client.call(Call::PageRead { vault: a.clone(), page: page_b }).is_err());
}

// ── Read endpoints ───────────────────────────────────────────────────────────

#[test]
fn read_endpoints_answer_from_the_vault() {
    let env = env();
    let (id, root) = env.add_vault("reads");
    let ctx = env.page_ctx(&root);
    let berlin = cerbo_core::page::page_create(&ctx, "Berlin".into()).unwrap();
    let germany = cerbo_core::page::page_create(&ctx, "Germany".into()).unwrap();
    cerbo_core::page::page_write(
        &ctx,
        berlin.clone(),
        format!("# Berlin\n\nThe capital of [Germany](cerbo://{germany}), famous for bears.\n"),
    )
    .unwrap();
    let attachment_src = env._tmp.path().join("note.txt");
    std::fs::write(&attachment_src, "attached").unwrap();
    let attachment = cerbo_core::object::attachment_add(&ctx, &berlin, &attachment_src).unwrap();
    cerbo_core::page::page_write(
        &ctx,
        berlin.clone(),
        format!("# Berlin\n\n[Germany](cerbo://{germany}) [file](cerbo://{attachment})\n bears\n"),
    )
    .unwrap();

    let _daemon = env.start();
    let mut client = env.connect(ClientKind::Cli);

    let pages = client.call(Call::PageList { vault: id.clone() }).unwrap();
    let titles: Vec<&str> = pages.as_array().unwrap().iter().map(|p| text(p, "title")).collect();
    assert!(titles.contains(&"Berlin") && titles.contains(&"Germany"), "{titles:?}");

    let page = read(&mut client, &id, &berlin);
    assert_eq!(text(&page, "title"), "Berlin");
    assert!(text(&page, "content").contains(&format!("cerbo://{germany}")));

    let err = client.call(Call::PageRead { vault: id.clone(), page: "../../etc/passwd".into() }).unwrap_err();
    assert!(err.message.contains("not a page id"), "{}", err.message);
    let err = client.call(Call::PageRead { vault: id.clone(), page: uuid_not_in_vault() }).unwrap_err();
    assert!(err.message.contains("not found"), "{}", err.message);

    // Attachments: of the page and of the vault.
    let of_page = client.call(Call::AttachmentList { vault: id.clone(), page: Some(berlin.clone()) }).unwrap();
    assert_eq!(of_page[0]["uuid"], json!(attachment));
    let of_vault = client.call(Call::AttachmentList { vault: id.clone(), page: None }).unwrap();
    assert_eq!(of_vault.as_array().unwrap().len(), 1);
    let info = client.call(Call::AttachmentInfo { vault: id.clone(), attachment: attachment.clone() }).unwrap();
    assert_eq!(std::fs::read_to_string(text(&info, "path")).unwrap(), "attached");
    assert!(client.call(Call::AttachmentInfo { vault: id.clone(), attachment: berlin.clone() }).is_err());

    // Links, outgoing links and backlinks.
    let links = client.call(Call::Links { vault: id.clone() }).unwrap();
    assert!(links.as_array().unwrap().contains(&json!({"from": berlin, "to": germany})));
    let outgoing = client.call(Call::OutgoingLinks { vault: id.clone(), page: berlin.clone() }).unwrap();
    assert_eq!(outgoing.as_array().unwrap().len(), 2);
    assert!(outgoing.as_array().unwrap().iter().any(|l| l["uuid"] == json!(germany) && l["title"] == "Germany"));
    let backlinks = client.call(Call::Backlinks { vault: id.clone(), page: germany.clone() }).unwrap();
    assert_eq!(backlinks, json!([{"uuid": berlin, "title": "Berlin"}]));

    // Text search: title matches first, with a snippet from the body.
    let hits = client.call(Call::SearchText { vault: id.clone(), query: "BEARS".into(), limit: 10 }).unwrap();
    assert_eq!(hits.as_array().unwrap().len(), 1);
    assert_eq!(hits[0]["uuid"], json!(berlin));
    assert!(text(&hits[0], "snippet").contains("bears"));
    assert!(client.call(Call::SearchText { vault: id.clone(), query: "  ".into(), limit: 10 }).is_err());

    // SPARQL over the index the daemon builds on demand.
    let result = client
        .call(Call::Sparql { vault: id.clone(), query: "SELECT ?s WHERE { ?s ?p ?o } LIMIT 3".into() })
        .unwrap();
    assert!(result["results"]["bindings"].is_array(), "{result}");
    let ask = client.call(Call::Sparql { vault: id.clone(), query: "ASK { ?s ?p ?o }".into() }).unwrap();
    assert_eq!(ask["boolean"], json!(true));
    let err = client.call(Call::Sparql { vault: id.clone(), query: "DELETE WHERE { ?s ?p ?o }".into() }).unwrap_err();
    assert!(err.message.to_lowercase().contains("sparql"), "{}", err.message);
}

fn uuid_not_in_vault() -> String {
    "1e5d7dc7-a34c-4a2a-aa49-8bfe2b9805b6".to_string()
}

#[test]
fn semantic_search_and_similar_use_the_embeddings_of_the_vault() {
    let env = env();
    let (id, root) = env.add_vault("semantic");
    let ctx = env.page_ctx(&root);
    let a = cerbo_core::page::page_create(&ctx, "Cats".into()).unwrap();
    let b = cerbo_core::page::page_create(&ctx, "Dogs".into()).unwrap();
    cerbo_core::page::page_write(&ctx, a.clone(), "# Cats\n\nCats purr.\n".into()).unwrap();
    cerbo_core::page::page_write(&ctx, b.clone(), "# Dogs\n\nDogs bark.\n".into()).unwrap();
    let vault_ctx = cerbo_core::VaultContext { vault_path: root.clone(), global: env.ctx.clone() };
    let mut mock = MockEmbedder::default();
    cerbo_core::semantic::refresh_embeddings(&vault_ctx, &mut mock, Default::default()).unwrap();

    let _daemon = env.start();
    let mut client = env.connect(ClientKind::Cli);

    let hits = client
        .call(Call::SearchSemantic {
            vault: id.clone(),
            query: "Cats purr".into(),
            limit: 5,
            types: vec![],
            annotated: vec![],
        })
        .unwrap();
    assert_eq!(hits[0]["uuid"], json!(a), "{hits}");
    let similar = client
        .call(Call::Similar { vault: id.clone(), page: a.clone(), limit: 5, types: vec![], annotated: vec![] })
        .unwrap();
    assert!(similar.as_array().unwrap().iter().all(|h| h["uuid"] != json!(a)), "the page itself is left out");
}

#[test]
fn semantic_search_without_embeddings_says_so() {
    let env = env();
    let (id, _) = env.add_vault("plain");
    let _daemon = env.start();
    let mut client = env.connect(ClientKind::Cli);
    let err = client
        .call(Call::SearchSemantic { vault: id, query: "x".into(), limit: 5, types: vec![], annotated: vec![] })
        .unwrap_err();
    assert!(err.message.contains("embeddings"), "{}", err.message);
}

// ── Write endpoints ──────────────────────────────────────────────────────────

#[test]
fn write_endpoints_change_the_vault() {
    let env = env();
    let (id, root) = env.add_vault("writes");
    let _daemon = env.start();
    let mut client = env.connect(ClientKind::Cli);

    // Create, with content and a folder.
    let made = client
        .call(Call::PageCreate {
            vault: id.clone(),
            title: "Plan".into(),
            content: Some("# Plan\n\nstep one\n".into()),
            path: Some("work/plans".into()),
        })
        .unwrap();
    let plan = text(&made, "uuid").to_string();
    let page = read(&mut client, &id, &plan);
    assert_eq!(text(&page, "content"), "# Plan\n\nstep one\n");
    assert_eq!(page["virtualPath"], json!("work/plans"));
    assert!(root.join(".cerbo/objects").join(&plan).join("page.md").is_file());

    // Edit.
    edit(&mut client, &id, &plan, "# Plan\n\nstep two\n");
    assert_eq!(text(&read(&mut client, &id, &plan), "content"), "# Plan\n\nstep two\n");

    // Rename: title in the metadata and in the first heading.
    client.call(Call::PageRename { vault: id.clone(), page: plan.clone(), title: "Roadmap".into() }).unwrap();
    let page = read(&mut client, &id, &plan);
    assert_eq!(text(&page, "title"), "Roadmap");
    assert!(text(&page, "content").starts_with("# Roadmap"), "{}", text(&page, "content"));

    // Move, and back to the root.
    client
        .call(Call::PageMove { vault: id.clone(), page: plan.clone(), path: Some("archive".into()) })
        .unwrap();
    assert_eq!(read(&mut client, &id, &plan)["virtualPath"], json!("archive"));
    client.call(Call::PageMove { vault: id.clone(), page: plan.clone(), path: None }).unwrap();
    assert_eq!(read(&mut client, &id, &plan)["virtualPath"], Value::Null);
    let err = client
        .call(Call::PageMove { vault: id.clone(), page: plan.clone(), path: Some("../escape".into()) })
        .unwrap_err();
    assert!(err.message.contains("invalid"), "{}", err.message);

    // Link: appended once, and not again.
    let other = create(&mut client, &id, "Other");
    let added = client.call(Call::LinkAdd { vault: id.clone(), from: plan.clone(), to: other.clone() }).unwrap();
    assert_eq!(added["added"], json!(true));
    assert!(text(&read(&mut client, &id, &plan), "content").contains(&format!("(cerbo://{other})")));
    let again = client.call(Call::LinkAdd { vault: id.clone(), from: plan.clone(), to: other.clone() }).unwrap();
    assert_eq!(again["added"], json!(false));
    let backlinks = client.call(Call::Backlinks { vault: id.clone(), page: other.clone() }).unwrap();
    assert_eq!(backlinks[0]["uuid"], json!(plan));

    // Trash: the page leaves the list but stays in the vault trash.
    client.call(Call::PageTrash { vault: id.clone(), page: other.clone() }).unwrap();
    assert!(client.call(Call::PageRead { vault: id.clone(), page: other.clone() }).is_err());
    assert!(!root.join(".cerbo/objects").join(&other).exists());
    let trashed: Vec<_> = std::fs::read_dir(root.join(".cerbo/trash")).unwrap().collect();
    assert_eq!(trashed.len(), 1);

    // Refusals.
    assert!(client
        .call(Call::PageCreate { vault: id.clone(), title: "  ".into(), content: None, path: None })
        .is_err());
    assert!(client
        .call(Call::PageEdit { vault: id.clone(), page: other, content: "x".into() })
        .is_err());
}

#[test]
fn two_writers_at_once_lose_nothing() {
    let env = env();
    let (id, _) = env.add_vault("race");
    let daemon = env.start();

    const EACH: usize = 20;
    let writers: Vec<_> = ["app", "agent"]
        .into_iter()
        .map(|who| {
            let paths = env.paths.clone();
            let id = id.clone();
            let kind = if who == "app" { ClientKind::App } else { ClientKind::Agent };
            std::thread::spawn(move || {
                let mut client = Client::connect(&paths.socket, kind, Some(who)).unwrap();
                let mut made = Vec::new();
                for n in 0..EACH {
                    let page = create(&mut client, &id, &format!("{who} {n}"));
                    edit(&mut client, &id, &page, &format!("# {who} {n}\n\nbody of {who} {n}\n"));
                    made.push((page, format!("# {who} {n}\n\nbody of {who} {n}\n")));
                }
                made
            })
        })
        .collect();
    let made: Vec<(String, String)> = writers.into_iter().flat_map(|w| w.join().unwrap()).collect();

    let mut client = env.connect(ClientKind::Cli);
    let pages = client.call(Call::PageList { vault: id.clone() }).unwrap();
    assert_eq!(pages.as_array().unwrap().len(), 2 * EACH);
    for (page, content) in made {
        assert_eq!(text(&read(&mut client, &id, &page), "content"), content);
    }
    daemon.stop().unwrap();
}

#[test]
fn pipelined_writes_on_one_page_apply_in_the_order_sent() {
    let env = env();
    let (id, _) = env.add_vault("order");
    let _daemon = env.start();
    let mut client = env.connect(ClientKind::Cli);
    let page = create(&mut client, &id, "Counter");

    let ids: Vec<u64> = (0..40)
        .map(|n| {
            client
                .send(Call::PageEdit { vault: id.clone(), page: page.clone(), content: format!("# Counter\n\n{n}\n") })
                .unwrap()
        })
        .collect();
    for request in ids {
        client.wait(request).unwrap();
    }
    assert_eq!(text(&read(&mut client, &id, &page), "content"), "# Counter\n\n39\n");
}

// ── App lock ─────────────────────────────────────────────────────────────────

#[test]
fn a_running_daemon_keeps_automatic_rewrites_out() {
    let env = env();
    let daemon = env.start();

    // The lock an automatic vocabulary migration needs is taken: it skips.
    assert!(AppLock::try_exclusive(&env.ctx.cache_dir).unwrap().is_none());

    daemon.stop().unwrap();
    assert!(AppLock::try_exclusive(&env.ctx.cache_dir).unwrap().is_some(), "released when the daemon exits");
}

#[test]
fn an_automatic_vocabulary_migration_is_skipped_while_the_daemon_runs() {
    use cerbo_core::vocabulary_check::{ensure_current_vocabulary, EnsureOutcome};

    let env = env();
    let (_, root) = env.add_vault("old");
    // An object in the old metadata format, as an older Cerbo wrote it.
    let uuid = "ba079793-b2ff-4ca6-ba88-8764ce6668c3";
    let dir = root.join(".cerbo/objects").join(uuid);
    std::fs::create_dir_all(&dir).unwrap();
    let legacy = format!(
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
         @prefix schema: <cerbo://ontology/schema/> .\n\
         @prefix cerbo: <cerbo://ontology/> .\n\
         <cerbo://objects/{uuid}> cerbo:type cerbo:Product ;\n\
         \tcerbo:title \"Index\" ;\n\
         \tschema:dateCreated \"2026-10-25T13:17:54+00:00\"^^xsd:dateTime ;\n\
         \tschema:dateModified \"2026-10-25T13:18:16+00:00\"^^xsd:dateTime ;\n\
         \tcerbo:slug \"index\" .\n"
    );
    std::fs::write(dir.join("meta.ttl"), &legacy).unwrap();
    std::fs::write(dir.join("page.md"), "# Index\n").unwrap();
    let vault_ctx = cerbo_core::VaultContext { vault_path: root.clone(), global: env.ctx.clone() };

    let daemon = env.start();
    let outcome = ensure_current_vocabulary(&vault_ctx, &env.ctx.cache_dir, |_| {}).unwrap();
    assert!(matches!(outcome, EnsureOutcome::Deferred { .. }), "migrated under a running daemon: {outcome:?}");
    assert_eq!(std::fs::read_to_string(dir.join("meta.ttl")).unwrap(), legacy, "nothing was written");

    // Once the daemon is gone the next run migrates.
    daemon.stop().unwrap();
    let outcome = ensure_current_vocabulary(&vault_ctx, &env.ctx.cache_dir, |_| {}).unwrap();
    assert!(matches!(outcome, EnsureOutcome::Migrated { .. }), "{outcome:?}");
}

// ── Events ───────────────────────────────────────────────────────────────────

#[test]
fn a_client_write_reaches_subscribers_with_its_origin() {
    let env = env();
    let (id, _) = env.add_vault("events");
    let _daemon = env.start();
    let mut app = env.connect(ClientKind::App);
    let mut agent = env.connect(ClientKind::Agent);
    let page = create(&mut agent, &id, "Shared");
    app.call(Call::Subscribe { vault: Some(id.clone()) }).unwrap();

    edit(&mut agent, &id, &page, "# Shared\n\nedited by an agent\n");
    let event = wait_event(&mut app, |e| matches!(e, Event::PageChanged { page: p, .. } if *p == page));
    assert_eq!(
        event,
        Event::PageChanged { vault: id.clone(), page: page.clone(), change: ChangeKind::Changed, origin: Origin::Agent }
    );

    // Trashing is a removal.
    agent.call(Call::PageTrash { vault: id.clone(), page: page.clone() }).unwrap();
    let event = wait_event(&mut app, |e| matches!(e, Event::PageChanged { change: ChangeKind::Removed, .. }));
    assert!(matches!(event, Event::PageChanged { origin: Origin::Agent, .. }), "{event:?}");
}

#[test]
fn an_edit_outside_the_daemon_is_announced_as_external() {
    let env = env();
    let (id, root) = env.add_vault("outside");
    let ctx = env.page_ctx(&root);
    let page = cerbo_core::page::page_create(&ctx, "Hand edited".into()).unwrap();
    let _daemon = env.start();
    let mut client = env.connect(ClientKind::App);
    client.call(Call::Subscribe { vault: None }).unwrap();

    // Another editor saves the page file directly.
    let file = root.join(".cerbo/objects").join(&page).join("page.md");
    cerbo_core::fsio::write_atomic_str(&file, "# Hand edited\n\nchanged in an editor\n").unwrap();

    let event = wait_event(&mut client, |e| matches!(e, Event::PageChanged { page: p, .. } if *p == page));
    assert_eq!(
        event,
        Event::PageChanged { vault: id, page, change: ChangeKind::Changed, origin: Origin::External }
    );
}

#[test]
fn subscribers_only_hear_about_their_vault() {
    let env = env();
    let (a, _) = env.add_vault("a");
    let (b, _) = env.add_vault("b");
    let _daemon = env.start();
    let mut writer = env.connect(ClientKind::Cli);
    let mut listener = env.connect(ClientKind::App);
    listener.call(Call::Subscribe { vault: Some(a.clone()) }).unwrap();

    let in_b = create(&mut writer, &b, "Elsewhere");
    let in_a = create(&mut writer, &a, "Here");
    let event = wait_event(&mut listener, |e| matches!(e, Event::PageChanged { .. }));
    assert_eq!(event.vault(), Some(a.as_str()), "{event:?}");
    assert!(matches!(&event, Event::PageChanged { page, .. } if *page == in_a));
    assert_ne!(in_a, in_b);
}

// ── Graceful stop ────────────────────────────────────────────────────────────

#[test]
fn stop_finishes_queued_writes_before_the_daemon_exits() {
    let env = env();
    let (id, root) = env.add_vault("stopping");
    let daemon = env.start();
    let mut client = env.connect(ClientKind::Cli);
    let page = create(&mut client, &id, "Log");

    // Queue many writes without waiting, then ask the daemon to stop.
    let writes: Vec<u64> = (0..60)
        .map(|n| {
            client
                .send(Call::PageEdit { vault: id.clone(), page: page.clone(), content: format!("# Log\n\nentry {n}\n") })
                .unwrap()
        })
        .collect();
    let stop = client.send(Call::Stop).unwrap();

    for request in writes {
        client.wait(request).expect("a write accepted before the stop must finish");
    }
    client.wait(stop).unwrap();
    daemon.join().unwrap();

    let content = std::fs::read_to_string(root.join(".cerbo/objects").join(&page).join("page.md")).unwrap();
    assert_eq!(content, "# Log\n\nentry 59\n");
    assert!(!env.paths.socket.exists());
}

#[test]
fn a_stopping_daemon_takes_no_new_writes() {
    let env = env();
    let (id, _) = env.add_vault("closed");
    let daemon = env.start();
    let mut first = env.connect(ClientKind::Cli);
    let mut second = env.connect(ClientKind::Cli);
    assert_eq!(second.call(Call::Ping).unwrap(), json!("pong"));

    first.call(Call::Stop).unwrap();
    // The stop is under way: a write on another connection is refused or
    // the connection is already gone, but never half applied.
    let result = second.call(Call::PageCreate { vault: id, title: "Late".into(), content: None, path: None });
    if let Err(e) = result {
        assert!(e.code == "shutting_down" || e.message.contains("closed"), "{e:?}");
    }
    daemon.join().unwrap();
}
