//! `cerbo mcp`, `cerbo daemon`, `cerbo vault agent`, `cerbo approvals` and the
//! daemon-first path of the page commands, against a daemon running inside
//! the test process and temporary vaults.
//!
//! The MCP tests drive the server with a scripted client: JSON-RPC lines on
//! the child's stdin and stdout, the way an agent CLI does.

use cerbo_core::agent::{self, AgentPermission};
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::client::Client;
use cerbo_core::rpc::{Call, ChangeKind, ClientKind, Event, Origin};
use cerbo_core::semantic::{LazyEmbedder, MockEmbedder};
use cerbo_core::CerboContext;
use cerboid::{Config, Running};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct Env {
    xdg: TempDir,
    ctx: CerboContext,
    paths: DaemonPaths,
}

impl Env {
    fn new() -> Self {
        let xdg = TempDir::new().unwrap();
        let ctx = CerboContext {
            config_dir: xdg.path().join("config").join("cerbo"),
            cache_dir: xdg.path().join("cache").join("cerbo"),
        };
        let paths = DaemonPaths::in_dir(xdg.path().join("run").join("cerbo"));
        // The first run writes the default config files.
        let env = Env { xdg, ctx, paths };
        assert!(env.cerbo(&["vault", "list"]).status.success());
        env
    }

    fn cmd(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerbo"));
        cmd.env("XDG_CONFIG_HOME", self.xdg.path().join("config"))
            .env("XDG_CACHE_HOME", self.xdg.path().join("cache"))
            .env("XDG_RUNTIME_DIR", self.xdg.path().join("run"))
            .env(cerbo_core::semantic::MOCK_EMBEDDER_ENV, "1")
            .stdin(Stdio::null());
        cmd
    }

    fn cerbo(&self, args: &[&str]) -> std::process::Output {
        self.cmd().args(args).output().unwrap()
    }

    fn cerbo_ok(&self, args: &[&str]) -> String {
        let out = self.cerbo(args);
        assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8(out.stdout).unwrap()
    }

    fn start_daemon(&self) -> Running {
        let mut config = Config::new(self.ctx.clone(), self.paths.clone());
        config.registry_poll = Duration::from_millis(50);
        config.event_quiet = Duration::from_millis(30);
        config.embedder = Arc::new(|_| {
            let dim = cerbo_core::semantic::Embedder::dim(&MockEmbedder::default());
            LazyEmbedder::new(dim, || Ok(Box::new(MockEmbedder::default())))
        });
        cerboid::spawn(config).unwrap().expect("no other daemon runs")
    }

    fn add_vault(&self, name: &str) -> (String, PathBuf) {
        let path = self.xdg.path().join("vaults").join(name);
        let (vault, _) = cerbo_core::vault::vault_create(&self.ctx, &path, name.to_string(), false).unwrap();
        (vault.id, vault.path)
    }

    fn connect(&self, kind: ClientKind) -> Client {
        Client::connect(&self.paths.socket, kind, Some("test")).unwrap()
    }

    fn mcp(&self, vault: &str, name: Option<&str>) -> Mcp {
        let mut cmd = self.cmd();
        cmd.args(["mcp", "--vault", vault]);
        if let Some(name) = name {
            cmd.args(["--name", name]);
        }
        Mcp::start(cmd)
    }
}

// ── A scripted MCP client ────────────────────────────────────────────────────

struct Mcp {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<Value>,
    next_id: u64,
}

impl Mcp {
    fn start(mut cmd: Command) -> Self {
        let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Ok(value) = serde_json::from_str::<Value>(&line) {
                    let _ = tx.send(value);
                }
            }
        });
        let mut mcp = Mcp { child, stdin, lines, next_id: 0 };
        let init = mcp.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "testagent", "version": "1.0" }
            }),
        );
        let answer = mcp.wait(init);
        assert!(answer["result"]["serverInfo"].is_object(), "initialize failed: {answer}");
        mcp.notify("notifications/initialized");
        mcp
    }

    fn send(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn notify(&mut self, method: &str) {
        self.send(&json!({ "jsonrpc": "2.0", "method": method }));
    }

    fn request(&mut self, method: &str, params: Value) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        id
    }

    /// The answer to request `id`; server notifications on the way are skipped.
    fn wait_for(&mut self, id: u64, within: Duration) -> Option<Value> {
        let deadline = Instant::now() + within;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let message = self.lines.recv_timeout(left).ok()?;
            if message["id"] == json!(id) {
                return Some(message);
            }
        }
    }

    fn wait(&mut self, id: u64) -> Value {
        self.wait_for(id, Duration::from_secs(30)).expect("no answer from the MCP server")
    }

    fn tools(&mut self) -> Vec<String> {
        let id = self.request("tools/list", json!({}));
        let answer = self.wait(id);
        let mut names: Vec<String> = answer["result"]["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("no tool list in {answer}"))
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        names.sort();
        names
    }

    fn start_call(&mut self, tool: &str, arguments: Value) -> u64 {
        self.request("tools/call", json!({ "name": tool, "arguments": arguments }))
    }

    /// A finished tool call: whether it is an error result, and its text.
    fn finish(&mut self, id: u64) -> (bool, String) {
        let answer = self.wait(id);
        let result = &answer["result"];
        assert!(result.is_object(), "protocol error instead of a tool result: {answer}");
        let text = result["content"][0]["text"].as_str().unwrap_or_default().to_string();
        (result["isError"].as_bool().unwrap_or(false), text)
    }

    fn call(&mut self, tool: &str, arguments: Value) -> (bool, String) {
        let id = self.start_call(tool, arguments);
        self.finish(id)
    }

    fn call_ok(&mut self, tool: &str, arguments: Value) -> Value {
        let (is_error, text) = self.call(tool, arguments);
        assert!(!is_error, "{tool} failed: {text}");
        serde_json::from_str(&text).unwrap_or(Value::String(text))
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

const READ_TOOLS: [&str; 13] = [
    "backlinks",
    "guide",
    "index_status",
    "list_attachments",
    "list_pages",
    "list_trash",
    "ontology_list",
    "ontology_types",
    "outgoing_links",
    "read_page",
    "search_semantic",
    "search_text",
    "sparql_query",
];
const WRITE_TOOLS: [&str; 11] = [
    "create_page",
    "delete_page",
    "edit_page",
    "index_rebuild",
    "link_pages",
    "move_page",
    "ontology_add_type",
    "ontology_create",
    "ontology_import",
    "rename_page",
    "restore_page",
];
const HOST_TOOLS: [&str; 3] = ["host_list", "host_read", "host_write"];

fn sorted(names: impl IntoIterator<Item = &'static str>) -> Vec<String> {
    let mut v: Vec<String> = names.into_iter().map(str::to_string).collect();
    v.sort();
    v
}

// ── MCP ──────────────────────────────────────────────────────────────────────

#[test]
fn a_read_only_vault_lists_no_write_tool_and_refuses_a_write_call() {
    let env = Env::new();
    let (id, _) = env.add_vault("notes");
    let _daemon = env.start_daemon();
    let mut mcp = env.mcp("notes", None);

    assert_eq!(mcp.tools(), sorted(READ_TOOLS), "only read tools at the default level");

    // Calling a write tool anyway is refused by the daemon with a clear message.
    let (is_error, text) = mcp.call("create_page", json!({ "title": "Sneaky" }));
    assert!(is_error);
    assert!(text.contains("read-only"), "{text}");
    let pages = mcp.call_ok("list_pages", json!({}));
    assert_eq!(pages.as_array().unwrap().len(), 0, "no page was created");
    drop(id);
}

#[test]
fn host_tools_are_listed_only_while_host_access_is_on() {
    let env = Env::new();
    let (_id, _) = env.add_vault("notes");
    let _daemon = env.start_daemon();
    let mut mcp = env.mcp("notes", None);
    assert_eq!(mcp.tools(), sorted(READ_TOOLS));

    env.cerbo_ok(&["vault", "agent", "notes", "host", "on"]);
    let mut with_host: Vec<&'static str> = READ_TOOLS.to_vec();
    with_host.extend(HOST_TOOLS);
    assert_eq!(mcp.tools(), sorted(with_host), "host on adds the host tools and no write tool");

    env.cerbo_ok(&["vault", "agent", "notes", "host", "off"]);
    assert_eq!(mcp.tools(), sorted(READ_TOOLS));
}

#[test]
fn the_guide_describes_only_what_the_session_may_do() {
    let env = Env::new();
    let (_id, _) = env.add_vault("notes");
    let _daemon = env.start_daemon();
    let mut mcp = env.mcp("notes", None);

    // No topic: the usage guide.
    let (is_error, usage) = mcp.call("guide", json!({}));
    assert!(!is_error);
    assert!(usage.contains("search_semantic") && usage.contains("backlinks"), "{usage}");

    let (_, read_only) = mcp.call("guide", json!({ "topic": "ontology" }));
    assert!(read_only.contains("ontology_types") && read_only.contains("you cannot add types or vocabularies"), "{read_only}");
    assert!(!read_only.contains("ontology_add_type"), "{read_only}");
    let (_, management) = mcp.call("guide", json!({ "topic": "management" }));
    assert!(management.contains("read-only for you") && !management.contains("edit_page"), "{management}");

    env.cerbo_ok(&["vault", "agent", "notes", "auto"]);
    let (_, writable) = mcp.call("guide", json!({ "topic": "ontology" }));
    assert!(writable.contains("ontology_add_type") && !writable.contains("you cannot add types or vocabularies"), "{writable}");
    let (_, management) = mcp.call("guide", json!({ "topic": "management" }));
    assert!(management.contains("edit_page") && management.contains("base_version"), "{management}");
    assert!(!management.contains("host_read"), "{management}");

    // An unknown topic names the real ones.
    let (is_error, text) = mcp.call("guide", json!({ "topic": "cooking" }));
    assert!(is_error);
    assert!(text.contains("usage, management, ontology"), "{text}");
}

#[test]
fn the_guides_are_also_resources_rendered_for_the_session() {
    let env = Env::new();
    let (_id, _) = env.add_vault("notes");
    let _daemon = env.start_daemon();
    let mut mcp = env.mcp("notes", None);

    let id = mcp.request("resources/list", json!({}));
    let listed = mcp.wait(id);
    let mut uris: Vec<String> = listed["result"]["resources"]
        .as_array()
        .unwrap_or_else(|| panic!("no resource list in {listed}"))
        .iter()
        .map(|r| r["uri"].as_str().unwrap().to_string())
        .collect();
    uris.sort();
    assert_eq!(uris, ["cerbo://guide/management", "cerbo://guide/ontology", "cerbo://guide/usage"]);
    assert_eq!(listed["result"]["resources"][0]["mimeType"], "text/markdown");

    let id = mcp.request("resources/read", json!({ "uri": "cerbo://guide/management" }));
    let read = mcp.wait(id);
    let text = read["result"]["contents"][0]["text"].as_str().unwrap_or_else(|| panic!("no text in {read}"));
    assert!(text.contains("read-only for you"), "{text}");

    env.cerbo_ok(&["vault", "agent", "notes", "auto"]);
    let id = mcp.request("resources/read", json!({ "uri": "cerbo://guide/management" }));
    let read = mcp.wait(id);
    assert!(read["result"]["contents"][0]["text"].as_str().unwrap().contains("edit_page"), "{read}");

    let id = mcp.request("resources/read", json!({ "uri": "cerbo://guide/nope" }));
    let missing = mcp.wait(id);
    assert!(missing["error"].is_object(), "an unknown guide is a protocol error: {missing}");
}

#[test]
fn an_agent_can_create_a_vocabulary_add_a_type_and_see_it() {
    let env = Env::new();
    let (_id, _) = env.add_vault("notes");
    let _daemon = env.start_daemon();
    env.cerbo_ok(&["vault", "agent", "notes", "auto"]);
    let mut mcp = env.mcp("notes", None);

    let made = mcp.call_ok("ontology_create", json!({ "prefix": "ex", "title": "Kitchen" }));
    assert_eq!(made["prefix"], "ex");
    let added = mcp.call_ok(
        "ontology_add_type",
        json!({ "prefix": "ex", "name": "Recipe", "comment": "A way to cook something" }),
    );
    assert_eq!(added["existing"], false);
    let again = mcp.call_ok("ontology_add_type", json!({ "prefix": "ex", "name": "Recipe" }));
    assert_eq!(again["existing"], true);

    let types = mcp.call_ok("ontology_types", json!({ "prefix": "ex" }));
    assert_eq!(types["vaultOwned"], true);
    assert_eq!(types["types"][0]["name"], "Recipe");

    // A bundled vocabulary is refused, with the way out.
    let (is_error, text) = mcp.call("ontology_add_type", json!({ "prefix": "schema", "name": "Mine" }));
    assert!(is_error);
    assert!(text.contains("ontology_create"), "{text}");

    // The change is recorded in the agent change log.
    let log = env.cerbo_ok(&["approvals", "log", "notes"]);
    assert!(log.contains("add_type") && log.contains("ontology"), "{log}");
}

#[test]
fn raising_the_permission_adds_the_write_tools_and_lowering_it_takes_them_away() {
    let env = Env::new();
    let (id, _) = env.add_vault("notes");
    let _daemon = env.start_daemon();
    let mut mcp = env.mcp("notes", None);
    assert_eq!(mcp.tools(), sorted(READ_TOOLS));

    env.cerbo_ok(&["vault", "agent", "notes", "auto"]);
    let mut all: Vec<&'static str> = READ_TOOLS.to_vec();
    all.extend(WRITE_TOOLS);
    assert_eq!(mcp.tools(), sorted(all));

    env.cerbo_ok(&["vault", "agent", &id, "read"]);
    assert_eq!(mcp.tools(), sorted(READ_TOOLS));
}

#[test]
fn the_tools_read_and_change_a_temporary_vault() {
    let env = Env::new();
    let (id, root) = env.add_vault("notes");
    let (_other_id, other_root) = env.add_vault("elsewhere");
    let _daemon = env.start_daemon();
    env.cerbo_ok(&["vault", "agent", "notes", "auto"]);
    let mut mcp = env.mcp(root.to_str().unwrap(), Some("opencode"));

    // Create, read, edit.
    let created = mcp.call_ok("create_page", json!({ "title": "Berlin", "content": "# Berlin\n\nA city.\n" }));
    let berlin = created["uuid"].as_str().unwrap().to_string();
    let other = mcp.call_ok("create_page", json!({ "title": "Germany" }));
    let germany = other["uuid"].as_str().unwrap().to_string();

    let page = mcp.call_ok("read_page", json!({ "page": berlin }));
    assert_eq!(page["title"], "Berlin");
    assert!(page["content"].as_str().unwrap().contains("A city."), "{page}");

    mcp.call_ok("edit_page", json!({ "page": berlin, "content": "# Berlin\n\nThe capital.\n" }));
    let page = mcp.call_ok("read_page", json!({ "page": berlin }));
    assert!(page["content"].as_str().unwrap().contains("The capital."), "{page}");

    // Rename, move, link and the links around it.
    mcp.call_ok("rename_page", json!({ "page": berlin, "title": "Berlin, Germany" }));
    mcp.call_ok("move_page", json!({ "page": berlin, "path": "cities" }));
    mcp.call_ok("link_pages", json!({ "from": berlin, "to": germany }));
    let outgoing = mcp.call_ok("outgoing_links", json!({ "page": berlin }));
    assert_eq!(outgoing[0]["uuid"], germany.as_str());
    let back = mcp.call_ok("backlinks", json!({ "page": germany }));
    assert_eq!(back[0]["uuid"], berlin.as_str(), "{back}");

    let pages = mcp.call_ok("list_pages", json!({}));
    let listed = pages.as_array().unwrap();
    assert_eq!(listed.len(), 2);
    let berlin_entry = listed.iter().find(|p| p["uuid"] == berlin.as_str()).unwrap();
    assert_eq!(berlin_entry["title"], "Berlin, Germany");
    assert_eq!(berlin_entry["virtualPath"], "cities");

    // Search.
    let hits = mcp.call_ok("search_text", json!({ "query": "capital" }));
    assert_eq!(hits[0]["uuid"], berlin.as_str());
    // Without embeddings the tool says what to do, as a result and not a crash.
    let (is_error, text) = mcp.call("search_semantic", json!({ "query": "capital", "limit": 3 }));
    assert!(is_error && text.contains("index --embeddings"), "{text}");
    env.cerbo_ok(&["index", "--embeddings", "--vault", root.to_str().unwrap()]);
    let hits = mcp.call_ok("search_semantic", json!({ "query": "capital", "limit": 3 }));
    assert!(hits.is_array() && !hits.as_array().unwrap().is_empty(), "{hits}");
    let sparql = mcp.call_ok("sparql_query", json!({ "query": "ASK { ?s ?p ?o }" }));
    assert_eq!(sparql["boolean"], true, "{sparql}");
    assert!(mcp.call_ok("list_attachments", json!({})).as_array().unwrap().is_empty());

    // Delete goes to the trash and can be recovered.
    mcp.call_ok("delete_page", json!({ "page": germany }));
    let trash = std::fs::read_dir(root.join(".cerbo").join("trash")).unwrap().flatten().collect::<Vec<_>>();
    assert!(trash.iter().any(|e| e.file_name().to_string_lossy().contains(&germany)));
    assert_eq!(mcp.call_ok("list_pages", json!({})).as_array().unwrap().len(), 1);

    // The change log names the agent.
    let log = agent::read_changes(&env.ctx, &id, 50).unwrap();
    assert!(log.len() >= 7, "{log:?}");
    assert!(log.iter().all(|e| e.agent == "opencode"), "{log:?}");
    assert!(log.iter().any(|e| e.action == "trash" && e.page == germany));
    let shown = env.cerbo_ok(&["approvals", "log", "notes"]);
    assert!(shown.contains("opencode") && shown.contains("trash"), "{shown}");

    // Errors are results, not crashes: an unknown page names its UUID, and a
    // page of another vault does not resolve.
    let missing = "0b0e2f3a-1d2c-4b5a-8c9d-0e1f2a3b4c5d";
    let (is_error, text) = mcp.call("read_page", json!({ "page": missing }));
    assert!(is_error && text.contains(missing), "{text}");
    let foreign = cerbo_core::page::page_create(
        &CerboContext { config_dir: other_root.join(".cerbo"), cache_dir: env.ctx.cache_dir.clone() },
        "Foreign".to_string(),
    )
    .unwrap();
    for tool in ["read_page", "backlinks", "outgoing_links"] {
        let (is_error, text) = mcp.call(tool, json!({ "page": foreign }));
        assert!(is_error, "{tool} resolved a page of another vault: {text}");
    }
    let (is_error, _) = mcp.call("edit_page", json!({ "page": foreign, "content": "x" }));
    assert!(is_error, "an edit must not reach another vault");
    let (is_error, _) = mcp.call("link_pages", json!({ "from": berlin, "to": foreign }));
    assert!(is_error, "a link must not point into another vault");

    // The server still answers after the errors.
    assert_eq!(mcp.call_ok("list_pages", json!({})).as_array().unwrap().len(), 1);
}

#[test]
fn an_unregistered_vault_fails_at_once_with_a_message() {
    let env = Env::new();
    let _daemon = env.start_daemon();
    let out = env.cerbo(&["mcp", "--vault", "no-such-vault"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no-such-vault") && stderr.contains("not registered"), "{stderr}");
}

#[test]
fn approve_mode_holds_the_agents_change_until_the_cli_approves_it() {
    let env = Env::new();
    let (_id, root) = env.add_vault("notes");
    let _daemon = env.start_daemon();
    env.cerbo_ok(&["vault", "agent", "notes", "approve"]);
    let page = cerbo_core::page::page_create(
        &CerboContext { config_dir: root.join(".cerbo"), cache_dir: env.ctx.cache_dir.clone() },
        "Draft".to_string(),
    )
    .unwrap();
    let mut mcp = env.mcp("notes", Some("opencode"));

    let call = mcp.start_call("edit_page", json!({ "page": page, "content": "approved text\n" }));

    // The request shows up for the person, with the difference.
    let deadline = Instant::now() + Duration::from_secs(10);
    let pending = loop {
        let list: Value = serde_json::from_str(&env.cerbo_ok(&["approvals", "list", "--json"])).unwrap();
        if let Some(first) = list.as_array().and_then(|l| l.first()) {
            break first.clone();
        }
        assert!(Instant::now() < deadline, "the request never showed up");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(pending["agent"], "opencode");
    assert!(pending["diff"].as_str().unwrap().contains("+approved text"), "{pending}");
    let text = env.cerbo_ok(&["approvals", "list"]);
    assert!(text.contains("opencode") && text.contains("+approved text"), "{text}");

    let unchanged = cerbo_core::page::page_read(
        &CerboContext { config_dir: root.join(".cerbo"), cache_dir: env.ctx.cache_dir.clone() },
        page.clone(),
    )
    .unwrap();
    assert!(!unchanged.contains("approved text"));

    env.cerbo_ok(&["approvals", "approve", &pending["id"].to_string()]);
    let (is_error, text) = mcp.finish(call);
    assert!(!is_error, "{text}");
    let content = cerbo_core::page::page_read(
        &CerboContext { config_dir: root.join(".cerbo"), cache_dir: env.ctx.cache_dir.clone() },
        page.clone(),
    )
    .unwrap();
    assert!(content.contains("approved text"), "{content}");

    // A second change, rejected: the agent is told and nothing changes.
    let call = mcp.start_call("rename_page", json!({ "page": page, "title": "Renamed" }));
    let deadline = Instant::now() + Duration::from_secs(10);
    let id = loop {
        let list: Value = serde_json::from_str(&env.cerbo_ok(&["approvals", "list", "--json"])).unwrap();
        if let Some(first) = list.as_array().and_then(|l| l.first()) {
            break first["id"].to_string();
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(50));
    };
    env.cerbo_ok(&["approvals", "reject", &id]);
    let (is_error, text) = mcp.finish(call);
    assert!(is_error && text.contains("rejected"), "{text}");
}

#[test]
fn an_agent_can_restore_a_deleted_page_and_see_the_index() {
    let env = Env::new();
    let (_id, _) = env.add_vault("notes");
    let _daemon = env.start_daemon();
    env.cerbo_ok(&["vault", "agent", "notes", "auto"]);
    let mut mcp = env.mcp("notes", None);

    let created = mcp.call_ok("create_page", json!({ "title": "Fleeting", "content": "# Fleeting\n\nkeep me\n" }));
    let page = created["uuid"].as_str().unwrap().to_string();
    mcp.call_ok("delete_page", json!({ "page": page }));
    let trash = mcp.call_ok("list_trash", json!({}));
    assert_eq!(trash[0]["uuid"], page.as_str());
    assert_eq!(trash[0]["title"], "Fleeting");

    mcp.call_ok("restore_page", json!({ "page": page }));
    let back = mcp.call_ok("read_page", json!({ "page": page }));
    assert!(back["content"].as_str().unwrap().contains("keep me"), "{back}");
    assert!(mcp.call_ok("list_trash", json!({})).as_array().unwrap().is_empty());
    let (is_error, text) = mcp.call("restore_page", json!({ "page": page }));
    assert!(is_error && text.contains("already exists"), "{text}");

    let rebuilt = mcp.call_ok("index_rebuild", json!({}));
    assert!(rebuilt["pagesProcessed"].as_u64().unwrap() >= 1, "{rebuilt}");
    let status = mcp.call_ok("index_status", json!({}));
    assert_eq!(status["built"], true);
    assert_eq!(status["usable"], true);
    assert_eq!(status["stale"], false);
    assert_eq!(status["pages"], 1);
}

#[test]
fn importing_an_ontology_asks_a_person_even_in_an_auto_vault() {
    let env = Env::new();
    let (_id, _) = env.add_vault("notes");
    let _daemon = env.start_daemon();
    env.cerbo_ok(&["vault", "agent", "notes", "auto"]);
    let mut mcp = env.mcp("notes", Some("opencode"));

    let call = mcp.start_call(
        "ontology_import",
        json!({ "url": "https://example.org/vocab.ttl", "prefix": "ex", "namespace": "https://example.org/vocab#" }),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let pending = loop {
        let list: Value = serde_json::from_str(&env.cerbo_ok(&["approvals", "list", "--json"])).unwrap();
        if let Some(first) = list.as_array().and_then(|l| l.first()) {
            break first.clone();
        }
        assert!(Instant::now() < deadline, "no approval was asked for, in an auto vault");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(pending["summary"].as_str().unwrap().contains("https://example.org/vocab.ttl"), "{pending}");
    env.cerbo_ok(&["approvals", "reject", &pending["id"].to_string()]);
    let (is_error, text) = mcp.finish(call);
    assert!(is_error && text.contains("rejected"), "{text}");

    // Nothing was fetched or registered.
    let list = mcp.call_ok("ontology_list", json!({}));
    assert!(list.as_array().unwrap().iter().all(|r| r["prefix"] != "ex"), "{list}");

    // A local file is not an ontology address, so it is refused before anyone is asked.
    let (is_error, text) = mcp.call("ontology_import", json!({ "url": "file:///etc/passwd" }));
    assert!(is_error, "{text}");
}

// ── CLI ──────────────────────────────────────────────────────────────────────

#[test]
fn vault_agent_prints_and_sets_the_level() {
    let env = Env::new();
    let (id, _) = env.add_vault("notes");

    assert_eq!(env.cerbo_ok(&["vault", "agent", "notes"]).trim(), "read", "the default");
    env.cerbo_ok(&["vault", "agent", "notes", "approve"]);
    assert_eq!(env.cerbo_ok(&["vault", "agent", "notes"]).trim(), "approve");
    assert_eq!(agent::get_permission(&env.ctx, &id), AgentPermission::Approve);
    let json: Value = serde_json::from_str(&env.cerbo_ok(&["vault", "agent", &id, "auto", "--json"])).unwrap();
    assert_eq!(json["agent"], "auto");
    assert_eq!(json["name"], "notes");

    let out = env.cerbo(&["vault", "agent", "notes", "write"]);
    assert!(!out.status.success(), "an unknown level is refused");
    let out = env.cerbo(&["vault", "agent", "nope", "read"]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("vault not found"));
    assert_eq!(out.status.code(), Some(2), "an unregistered vault has its own exit status");
}

#[test]
fn vault_agent_works_while_the_daemon_socket_does_not_answer() {
    // The service's ExecStartPre runs it while the socket exists but nobody
    // serves it yet: it must not wait for the daemon.
    let env = Env::new();
    env.add_vault("notes");
    std::fs::create_dir_all(&env.paths.dir).unwrap();
    let _silent = std::os::unix::net::UnixListener::bind(&env.paths.socket).unwrap();

    let out = env.cerbo(&["vault", "agent", "notes", "auto"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(env.cerbo_ok(&["vault", "agent", "notes"]).trim(), "auto");
}

#[test]
fn page_commands_go_through_the_daemon_when_it_runs() {
    let env = Env::new();
    let (id, root) = env.add_vault("notes");
    let _daemon = env.start_daemon();
    let mut watcher = env.connect(ClientKind::App);
    watcher.call(Call::Subscribe { vault: Some(id.clone()) }).unwrap();

    let run = |args: &[&str]| {
        let out = env.cmd().args(args).current_dir(&root).output().unwrap();
        assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8(out.stdout).unwrap()
    };
    let created = run(&["page", "create", "Via daemon"]);
    let uuid = created.trim().rsplit(": ").next().unwrap().to_string();
    run(&["page", "write", &uuid, "written through the daemon"]);
    assert_eq!(run(&["page", "read", &uuid]).trim(), "written through the daemon");
    assert!(run(&["page", "list"]).contains("Via daemon"));
    let listed: Value = serde_json::from_str(&run(&["page", "list", "--json"])).unwrap();
    assert_eq!(listed[0]["uuid"], uuid.as_str());

    // The daemon did the write: its events carry the CLI as the origin, where
    // a direct write would be seen as an outside edit.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut seen = false;
    while Instant::now() < deadline && !seen {
        if let Some(Event::PageChanged { page, change: ChangeKind::Changed, origin, .. }) =
            watcher.next_event(Duration::from_millis(200)).unwrap()
        {
            seen = page == uuid && origin == Origin::Cli;
        }
    }
    assert!(seen, "no event with the CLI as origin: the write did not go through the daemon");

    run(&["page", "move", &uuid, "inbox"]);
    run(&["page", "delete", &uuid]);
    assert!(!run(&["page", "list"]).contains("Via daemon"));
}

#[test]
fn page_commands_fall_back_to_the_files_without_a_daemon() {
    let env = Env::new();
    let (_id, root) = env.add_vault("notes");
    let out = env.cmd().args(["page", "create", "Direct"]).current_dir(&root).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let out = env.cmd().args(["page", "list"]).current_dir(&root).output().unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("Direct"));
}

#[test]
fn daemon_status_when_nothing_runs_exits_with_one() {
    let env = Env::new();
    let out = env.cerbo(&["daemon", "status"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("not running"));
    let json: Value = serde_json::from_slice(&env.cerbo(&["daemon", "status", "--json"]).stdout).unwrap();
    assert_eq!(json["running"], false);
    let out = env.cerbo(&["daemon", "stop"]);
    assert!(out.status.success(), "stopping a daemon that does not run is not an error");
}

#[test]
fn daemon_status_describes_a_running_daemon() {
    let env = Env::new();
    let (id, _) = env.add_vault("notes");
    env.cerbo_ok(&["vault", "agent", "notes", "approve"]);
    let _daemon = env.start_daemon();

    let text = env.cerbo_ok(&["daemon", "status"]);
    assert!(text.contains("running") && text.contains("notes") && text.contains("[agent: approve]"), "{text}");
    let json: Value = serde_json::from_str(&env.cerbo_ok(&["daemon", "status", "--json"])).unwrap();
    assert_eq!(json["running"], true);
    assert_eq!(json["vaults"][0]["id"], id.as_str());
    assert_eq!(json["vaults"][0]["agent"], "approve");
    assert!(json["pid"].as_u64().unwrap() > 0);
    assert!(json["uptime_secs"].is_u64());

    let started = env.cerbo_ok(&["daemon", "start", "--json"]);
    let started: Value = serde_json::from_str(&started).unwrap();
    assert_eq!(started["started"], false, "a running daemon is not started twice");
}

/// The `cerboid` binary beside the test build, when it has been built.
fn cerboid_binary() -> Option<PathBuf> {
    let cerbo = PathBuf::from(env!("CARGO_BIN_EXE_cerbo"));
    let path = cerbo.parent()?.join("cerboid");
    path.is_file().then_some(path)
}

#[test]
fn daemon_start_and_stop_and_mcp_starting_the_daemon() {
    let Some(bin) = cerboid_binary() else {
        eprintln!("skipped: no cerboid binary next to {} (build it with `cargo build -p cerboid`)", env!("CARGO_BIN_EXE_cerbo"));
        return;
    };
    let env = Env::new();
    env.add_vault("notes");
    let with_bin = |args: &[&str]| {
        let out = env.cmd().env(cerbo_core::rpc::launch::DAEMON_BIN_ENV, &bin).args(args).output().unwrap();
        assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8(out.stdout).unwrap()
    };

    assert!(with_bin(&["daemon", "start"]).contains("Started"));
    assert!(with_bin(&["daemon", "start"]).contains("already running"));
    assert!(with_bin(&["daemon", "status"]).contains("running"));
    assert!(with_bin(&["daemon", "stop"]).contains("Stopped"));
    assert!(!env.paths.socket.exists() || Client::connect(&env.paths.socket, ClientKind::Cli, None).is_err());

    // `cerbo mcp` starts the daemon when it is down.
    let mut cmd = env.cmd();
    cmd.env(cerbo_core::rpc::launch::DAEMON_BIN_ENV, &bin).args(["mcp", "--vault", "notes"]);
    let mut mcp = Mcp::start(cmd);
    assert_eq!(mcp.tools(), sorted(READ_TOOLS));
    drop(mcp);
    assert!(with_bin(&["daemon", "status"]).contains("running"), "the shim started the daemon");
    with_bin(&["daemon", "stop"]);
}

#[test]
fn the_cli_paths_use_a_runtime_dir_of_their_own() {
    // Guards the isolation the other tests rely on.
    let env = Env::new();
    let socket: &Path = &env.paths.socket;
    assert!(socket.starts_with(env.xdg.path()));
}

// ── The daemon-first path never goes around a daemon that runs ───────────────

/// A stand-in for a daemon on the socket that answers every handshake with
/// `reply`, such as one of another protocol version.
fn fake_daemon(env: &Env, reply: cerbo_core::rpc::ServerMessage) {
    env.paths.prepare_dir().unwrap();
    let listener = std::os::unix::net::UnixListener::bind(&env.paths.socket).unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            let _ = BufReader::new(&stream).read_line(&mut line);
            let mut out = stream;
            let _ = writeln!(out, "{}", serde_json::to_string(&reply).unwrap());
        }
    });
}

fn pages_on_disk(env: &Env, root: &Path) -> String {
    let out = env.cmd().env("CERBO_NO_DAEMON", "1").args(["page", "list"]).current_dir(root).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn a_daemon_of_another_protocol_version_is_refused_not_bypassed() {
    let env = Env::new();
    let (_id, root) = env.add_vault("notes");
    fake_daemon(
        &env,
        cerbo_core::rpc::ServerMessage::Error {
            error: cerbo_core::rpc::check_protocol(cerbo_core::rpc::PROTOCOL_VERSION + 1).unwrap_err(),
        },
    );

    for args in [["page", "create", "Sneaky"], ["page", "list", ""]] {
        let args: Vec<&str> = args.into_iter().filter(|a| !a.is_empty()).collect();
        let out = env.cmd().args(&args).current_dir(&root).output().unwrap();
        assert!(!out.status.success(), "{args:?} must not fall back to the files");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("restart the daemon"), "{args:?}: {stderr}");
    }
    assert!(!pages_on_disk(&env, &root).contains("Sneaky"), "nothing was written behind the daemon");
}

#[test]
fn a_daemon_that_runs_but_does_not_answer_is_an_error_not_a_fallback() {
    let env = Env::new();
    let (_id, root) = env.add_vault("notes");
    // Alive, as far as the lock says, but nothing listens on the socket.
    env.paths.prepare_dir().unwrap();
    let lock = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&env.paths.lock).unwrap();
    lock.try_lock().unwrap();

    let out = env.cmd().args(["page", "create", "Sneaky"]).current_dir(&root).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("running but does not answer"), "{stderr}");
    assert!(!pages_on_disk(&env, &root).contains("Sneaky"));

    // The daemon goes away: now the command works on the files.
    drop(lock);
    let out = env.cmd().args(["page", "create", "Direct"]).current_dir(&root).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(pages_on_disk(&env, &root).contains("Direct"));
}

#[test]
fn a_stale_socket_without_a_daemon_still_falls_back_to_the_files() {
    let env = Env::new();
    let (_id, root) = env.add_vault("notes");
    env.paths.prepare_dir().unwrap();
    // A crashed daemon leaves its socket file behind, and no lock is held.
    drop(std::os::unix::net::UnixListener::bind(&env.paths.socket).unwrap());
    assert!(env.paths.socket.exists());
    let out = env.cmd().args(["page", "create", "Direct"]).current_dir(&root).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(pages_on_disk(&env, &root).contains("Direct"));
}

// ── `daemon stop` and `daemon status` work across protocol versions ──────────

/// A daemon of protocol 1: it holds the instance lock, writes the pid of a
/// stand-in process into it, and refuses every handshake as a mismatch. With
/// `knows_admin` it also answers the version-independent administration line,
/// as a newer daemon of another protocol would. The stand-in process is the
/// "daemon" the CLI must end: when it dies, the lock and the socket go away.
struct OldDaemon {
    child_pid: u32,
    asked_to_stop: Arc<std::sync::atomic::AtomicBool>,
    gone: Arc<std::sync::atomic::AtomicBool>,
}

impl OldDaemon {
    fn start(env: &Env, knows_admin: bool) -> Self {
        use std::sync::atomic::{AtomicBool, Ordering};
        env.paths.prepare_dir().unwrap();
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&env.paths.lock)
            .unwrap();
        lock.try_lock().unwrap();
        let mut child = Command::new("sleep").arg("300").spawn().unwrap();
        let child_pid = child.id();
        (&lock).write_all(child_pid.to_string().as_bytes()).unwrap();

        let listener = std::os::unix::net::UnixListener::bind(&env.paths.socket).unwrap();
        listener.set_nonblocking(true).unwrap();
        let socket = env.paths.socket.clone();
        let gone = Arc::new(AtomicBool::new(false));
        let asked_to_stop = Arc::new(AtomicBool::new(false));
        let (gone_flag, asked_flag) = (Arc::clone(&gone), Arc::clone(&asked_to_stop));
        std::thread::spawn(move || {
            while !gone_flag.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        let mut line = String::new();
                        let _ = BufReader::new(&stream).read_line(&mut line);
                        let mut out = stream;
                        if knows_admin && line.trim() == r#"{"admin":"status"}"# {
                            let _ = writeln!(
                                out,
                                r#"{{"admin":{{"protocol":1,"version":"0.0.1","pid":{child_pid},"uptime_secs":5}}}}"#
                            );
                        } else if knows_admin && line.trim() == r#"{"admin":"stop"}"# {
                            let _ = writeln!(
                                out,
                                r#"{{"admin":{{"protocol":1,"version":"0.0.1","pid":{child_pid},"uptime_secs":5}}}}"#
                            );
                            asked_flag.store(true, Ordering::SeqCst);
                            let _ = child.kill();
                        } else {
                            let mismatch = cerbo_core::rpc::ServerMessage::Error {
                                error: cerbo_core::rpc::RpcError::new(
                                    cerbo_core::rpc::CODE_PROTOCOL_MISMATCH,
                                    "this program speaks protocol 1 but the other side speaks 3",
                                ),
                            };
                            let _ = writeln!(out, "{}", serde_json::to_string(&mismatch).unwrap());
                        }
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(10)),
                }
                // The process ended (SIGTERM from the CLI, or the kill above):
                // the daemon is gone, its lock and socket with it.
                if child.try_wait().ok().flatten().is_some() {
                    gone_flag.store(true, Ordering::SeqCst);
                }
            }
            let _ = std::fs::remove_file(&socket);
            // Reap the stand-in process (it is gone, or about to be).
            let _ = child.kill();
            let _ = child.wait();
            drop(lock);
        });
        OldDaemon { child_pid, asked_to_stop, gone }
    }

    fn is_gone(&self) -> bool {
        self.gone.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Drop for OldDaemon {
    fn drop(&mut self) {
        if !self.is_gone() {
            // A failed test must not leave the stand-in behind.
            // SAFETY: plain kill(2) of the process this test started.
            unsafe { libc::kill(self.child_pid as libc::pid_t, libc::SIGKILL) };
        }
    }
}

#[test]
fn status_and_stop_end_a_daemon_that_speaks_protocol_1_by_its_pid() {
    let env = Env::new();
    let old = OldDaemon::start(&env, false);
    assert!(env.paths.daemon_alive());

    let status: Value = serde_json::from_str(&env.cerbo_ok(&["daemon", "status", "--json"])).unwrap();
    assert_eq!(status["running"], true);
    assert_eq!(status["protocol_mismatch"], true);
    assert_eq!(status["pid"], old.child_pid);
    let text = env.cerbo_ok(&["daemon", "status"]);
    assert!(text.contains("cerbo daemon stop"), "{text}");

    // The daemon cannot be spoken to, yet `stop` ends it: by the pid in its lock file.
    let stopped = env.cerbo_ok(&["daemon", "stop", "--json"]);
    let stopped: Value = serde_json::from_str(&stopped).unwrap();
    assert_eq!(stopped["stopped"], true);
    assert_eq!(stopped["signalled"], true);
    assert_eq!(stopped["pid"], old.child_pid);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !old.is_gone() {
        assert!(Instant::now() < deadline, "the old daemon is still there");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(env.cerbo_ok(&["daemon", "stop"]).contains("not running"), "after it, there is nothing to stop");
}

#[test]
fn a_daemon_of_another_protocol_that_knows_admin_lines_is_asked_not_signalled() {
    let env = Env::new();
    let old = OldDaemon::start(&env, true);

    let status: Value = serde_json::from_str(&env.cerbo_ok(&["daemon", "status", "--json"])).unwrap();
    assert_eq!(status["protocol"], 1);
    assert_eq!(status["version"], "0.0.1");
    assert!(env.cerbo_ok(&["daemon", "status"]).contains("protocol 1"));

    let stopped: Value = serde_json::from_str(&env.cerbo_ok(&["daemon", "stop", "--json"])).unwrap();
    assert_eq!(stopped["signalled"], false, "the daemon was asked to stop, not killed");
    assert!(old.asked_to_stop.load(std::sync::atomic::Ordering::SeqCst));
}

// ── An agent that goes away takes its pending change with it ─────────────────

#[test]
fn a_killed_mcp_shim_cancels_the_change_it_left_waiting() {
    let env = Env::new();
    let (_id, root) = env.add_vault("notes");
    env.cerbo_ok(&["vault", "agent", "notes", "approve"]);
    let _daemon = env.start_daemon();
    let mut mcp = env.mcp("notes", Some("opencode"));
    let _pending = mcp.start_call("create_page", json!({ "title": "Never" }));

    let queue_len = || {
        let list: Value = serde_json::from_str(&env.cerbo_ok(&["approvals", "list", "--json"])).unwrap();
        list.as_array().unwrap().len()
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    while queue_len() == 0 {
        assert!(Instant::now() < deadline, "the request never queued");
        std::thread::sleep(Duration::from_millis(50));
    }

    drop(mcp); // kills the shim, its connection closes
    let deadline = Instant::now() + Duration::from_secs(10);
    while queue_len() != 0 {
        assert!(Instant::now() < deadline, "the abandoned request is still waiting");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!pages_on_disk(&env, &root).contains("Never"), "an abandoned request is never applied");
}
