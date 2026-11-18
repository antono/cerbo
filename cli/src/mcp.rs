//! `cerbo mcp --vault <name-or-path>`: a stdio MCP server for agent CLIs.
//!
//! It is a shim (design D5): every tool call is forwarded to the daemon over a
//! connection that announces itself as an agent, and the shim opens no vault.
//! Permission is decided in the daemon alone; the shim only asks for the level
//! so it can leave the write tools out of the list at `read`.

use cerbo_core::agent::AgentPermission;
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::client::Client;
use cerbo_core::rpc::{launch, Call, ClientKind, RpcError, CODE_CLIENT_TIMEOUT, CODE_UNKNOWN_VAULT};
use cerbo_core::CerboContext;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, InitializeRequestParams, InitializeResult, ListToolsResult, PaginatedRequestParams,
    ServerCapabilities, ServerConfig,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::transport::stdio;
use rmcp::{tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The tools that change the vault. Not listed at `read`.
const WRITE_TOOLS: [&str; 6] = ["create_page", "edit_page", "rename_page", "move_page", "delete_page", "link_pages"];

/// What a write that waits for approval gets on top of the approval time
/// limit before the shim gives up on the daemon's answer: the write queue may
/// hold it up a little after the approval.
const APPROVAL_MARGIN: Duration = Duration::from_secs(60);

/// The approval time limit assumed when the daemon does not say (its default).
const DEFAULT_APPROVAL_SECS: u64 = 600;

/// How long a write waits for the daemon's answer, given the approval time
/// limit the daemon reported: the limit and a margin, so an approval answered
/// late is still seen by the agent, never reported as a failure while it
/// applies afterwards.
fn write_wait(approval_secs: Option<u64>) -> Duration {
    Duration::from_secs(approval_secs.unwrap_or(DEFAULT_APPROVAL_SECS)) + APPROVAL_MARGIN
}

/// What the agent is told when the daemon did not answer in time.
const TIMEOUT_MESSAGE: &str = "the daemon did not answer in time. The connection was closed, so a change that was \
still waiting for approval or in the write queue is dropped and not applied, but one that was being applied at that \
moment may still have gone through. Read the page again before trying again.";

/// How often the shim checks whether the vault's permission changed.
const PERMISSION_POLL: Duration = Duration::from_secs(1);

const INSTRUCTIONS: &str = "Tools for one Cerbo vault: a local-first markdown wiki. Pages are identified by UUID and \
link to each other as cerbo://<uuid>. Read tools always work. Tools that change the vault are only offered when the \
user allowed it; a change may wait for the user's approval, and deleting a page moves it to the vault trash.";

/// One vault's link to the daemon: idle connections, reused and replaced as
/// needed. A call takes a connection for its whole duration, so a change that
/// waits for approval does not block reads.
struct Link {
    paths: DaemonPaths,
    /// A socket given with `--socket` or `CERBO_SOCKET` (an agent's session
    /// socket, design D9): connect there and never start a daemon.
    socket: Option<std::path::PathBuf>,
    vault: String,
    name: Mutex<Option<String>>,
    idle: Mutex<Vec<Client>>,
    /// How long a plain call waits, when not the client's own 60 s. Tests
    /// make it short to prove that writes do not use it.
    default_timeout: Option<Duration>,
}

impl Link {
    fn new(paths: DaemonPaths, vault: String, name: Option<String>) -> Self {
        Self { paths, socket: None, vault, name: Mutex::new(name), idle: Mutex::new(Vec::new()), default_timeout: None }
    }

    fn take(&self) -> Result<Client, RpcError> {
        if let Some(client) = self.idle.lock().unwrap_or_else(|e| e.into_inner()).pop() {
            return Ok(client);
        }
        let name = self.name.lock().unwrap_or_else(|e| e.into_inner()).clone();
        // The connection is bound to this shim's one vault.
        let name = name.as_deref().or(Some("cerbo mcp"));
        let mut client = match &self.socket {
            Some(socket) => Client::connect_bound(socket, ClientKind::Agent, name, &self.vault)
                .map_err(|e| RpcError::failed(format!("cannot reach the session socket {}: {e}", socket.display())))?,
            None => launch::connect_or_start(&self.paths, ClientKind::Agent, name, Some(&self.vault))?,
        };
        if let Some(timeout) = self.default_timeout {
            client.set_default_timeout(timeout);
        }
        Ok(client)
    }

    fn give_back(&self, client: Client) {
        self.idle.lock().unwrap_or_else(|e| e.into_inner()).push(client);
    }

    /// Run `call`. A read is repeated once on a new connection when the old
    /// one had died (the daemon restarted); a write is never repeated, since
    /// it may have applied.
    fn run(&self, call: Call) -> Result<Value, RpcError> {
        let retry = !call.is_write();
        let mut client = self.take()?;
        let result = if call.is_write() {
            // A write may wait for a person: wait for the answer at least
            // until the approval deadline, not for the short read limit.
            let secs = client
                .call(Call::Permission { vault: self.vault.clone() })
                .ok()
                .and_then(|v| v["approval_timeout_secs"].as_u64());
            client.call_with_timeout(call.clone(), write_wait(secs))
        } else {
            client.call(call.clone())
        };
        match result {
            Ok(value) => {
                self.give_back(client);
                Ok(value)
            }
            // No answer in time. The connection is dropped, not reused:
            // closing it makes the daemon cancel a change that still waits
            // for approval and drop one that is queued but not yet applied.
            // One already being applied may still go through, so the agent
            // is told the outcome is unknown.
            Err(e) if e.code == CODE_CLIENT_TIMEOUT => Err(RpcError::new(CODE_CLIENT_TIMEOUT, TIMEOUT_MESSAGE)),
            Err(e) if client.call(Call::Ping).is_ok() => {
                self.give_back(client);
                Err(e)
            }
            Err(e) if !retry => Err(e),
            Err(_) => {
                // The connection is dead; drop it, and the idle ones with it.
                self.idle.lock().unwrap_or_else(|e| e.into_inner()).clear();
                let mut fresh = self.take()?;
                let result = fresh.call(call);
                self.give_back(fresh);
                result
            }
        }
    }

    async fn call(self: &Arc<Self>, call: Call) -> Result<Value, RpcError> {
        let link = Arc::clone(self);
        tokio::task::spawn_blocking(move || link.run(call))
            .await
            .unwrap_or_else(|_| Err(RpcError::failed("the call crashed")))
    }

    /// The vault's permission. When the daemon cannot say, `read`: the safe
    /// answer is to offer fewer tools.
    async fn permission(self: &Arc<Self>) -> AgentPermission {
        let answer = self.call(Call::Permission { vault: self.vault.clone() }).await;
        answer.ok().and_then(|v| v["permission"].as_str().and_then(|s| s.parse().ok())).unwrap_or_default()
    }
}

// ── Tool parameters ──────────────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema)]
struct PageParams {
    /// UUID of the page.
    page: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct AttachmentParams {
    /// UUID of a page: list only its attachments. Omit for the whole vault.
    page: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// What to look for.
    query: String,
    /// Return at most this many pages (default 10).
    limit: Option<usize>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SparqlParams {
    /// A read-only SPARQL query over the vault's semantic index.
    query: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct CreateParams {
    /// Title of the new page.
    title: String,
    /// The page's markdown. Default: a heading with the title.
    content: Option<String>,
    /// Folder of the page, such as `projects/2026`. Default: the root.
    path: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct EditParams {
    /// UUID of the page.
    page: String,
    /// The new markdown of the whole page. It replaces the old content.
    content: String,
    /// The `version` that read_page returned for the text you based this edit
    /// on. Always pass it: the edit is then refused with a conflict when the
    /// page changed since you read it, instead of overwriting the change.
    base_version: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct RenameParams {
    /// UUID of the page.
    page: String,
    /// The new title.
    title: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct MoveParams {
    /// UUID of the page.
    page: String,
    /// The new folder, such as `notes/rust`. Omit to move the page to the root.
    path: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct LinkParams {
    /// UUID of the page that gets the link, appended at its end.
    from: String,
    /// UUID of the page to link to.
    to: String,
}

// ── The server ───────────────────────────────────────────────────────────────

#[derive(Clone)]
struct VaultServer {
    link: Arc<Link>,
}

/// A tool answer: the daemon's JSON as text, or an error result carrying the
/// daemon's message (an unknown page is an error naming the UUID, not a crash).
fn answer(result: Result<Value, RpcError>) -> Result<CallToolResult, ErrorData> {
    Ok(match result {
        Ok(value) => {
            let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
            CallToolResult::success(vec![ContentBlock::text(text)])
        }
        Err(e) => CallToolResult::error(vec![ContentBlock::text(e.message)]),
    })
}

#[tool_router]
impl VaultServer {
    fn new(link: Arc<Link>) -> Self {
        Self { link }
    }

    fn vault(&self) -> String {
        self.link.vault.clone()
    }

    #[tool(description = "List every page of the vault as {uuid, title, virtualPath}.")]
    async fn list_pages(&self) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::PageList { vault: self.vault() }).await)
    }

    #[tool(
        description = "Read one page: its title, markdown content, metadata and `version`. Pass that version as \
                       `base_version` to edit_page, so an edit built on this read cannot overwrite a change made \
                       after it."
    )]
    async fn read_page(&self, Parameters(p): Parameters<PageParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::PageRead { vault: self.vault(), page: p.page }).await)
    }

    #[tool(description = "Find pages whose title or text contains the query (case-insensitive).")]
    async fn search_text(&self, Parameters(p): Parameters<SearchParams>) -> Result<CallToolResult, ErrorData> {
        let limit = p.limit.unwrap_or(10);
        answer(self.link.call(Call::SearchText { vault: self.vault(), query: p.query, limit }).await)
    }

    #[tool(description = "Find pages by meaning, in any language. Returns {uuid, title, score}.")]
    async fn search_semantic(&self, Parameters(p): Parameters<SearchParams>) -> Result<CallToolResult, ErrorData> {
        let limit = p.limit.unwrap_or(10);
        answer(
            self.link
                .call(Call::SearchSemantic {
                    vault: self.vault(),
                    query: p.query,
                    limit,
                    types: Vec::new(),
                    annotated: Vec::new(),
                })
                .await,
        )
    }

    #[tool(description = "List the pages that link to a page, as {uuid, title}.")]
    async fn backlinks(&self, Parameters(p): Parameters<PageParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::Backlinks { vault: self.vault(), page: p.page }).await)
    }

    #[tool(description = "List the pages a page links to, as {uuid, title}.")]
    async fn outgoing_links(&self, Parameters(p): Parameters<PageParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::OutgoingLinks { vault: self.vault(), page: p.page }).await)
    }

    #[tool(description = "List attachments of one page, or of the whole vault.")]
    async fn list_attachments(&self, Parameters(p): Parameters<AttachmentParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::AttachmentList { vault: self.vault(), page: p.page }).await)
    }

    #[tool(description = "Run a read-only SPARQL query over the vault's semantic index; returns SPARQL JSON results.")]
    async fn sparql_query(&self, Parameters(p): Parameters<SparqlParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::Sparql { vault: self.vault(), query: p.query }).await)
    }

    #[tool(description = "Create a page. Returns its UUID. May wait for the user's approval.")]
    async fn create_page(&self, Parameters(p): Parameters<CreateParams>) -> Result<CallToolResult, ErrorData> {
        answer(
            self.link
                .call(Call::PageCreate { vault: self.vault(), title: p.title, content: p.content, path: p.path })
                .await,
        )
    }

    #[tool(
        description = "Replace the whole markdown of a page. Always pass `base_version`, the `version` from the \
                       read_page call your new text is based on: if the page changed since, the edit is refused with \
                       a conflict and the page is left as it is; read it again and redo the edit. Without it the \
                       edit overwrites whatever the page holds. May wait for the user's approval."
    )]
    async fn edit_page(&self, Parameters(p): Parameters<EditParams>) -> Result<CallToolResult, ErrorData> {
        answer(
            self.link
                .call(Call::PageEdit {
                    vault: self.vault(),
                    page: p.page,
                    content: p.content,
                    base_version: p.base_version,
                })
                .await,
        )
    }

    #[tool(description = "Change the title of a page. May wait for the user's approval.")]
    async fn rename_page(&self, Parameters(p): Parameters<RenameParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::PageRename { vault: self.vault(), page: p.page, title: p.title }).await)
    }

    #[tool(description = "Move a page to a folder, or to the root without a path. May wait for the user's approval.")]
    async fn move_page(&self, Parameters(p): Parameters<MoveParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::PageMove { vault: self.vault(), page: p.page, path: p.path }).await)
    }

    #[tool(description = "Delete a page by moving it to the vault trash, where the user can recover it. May wait for the user's approval.")]
    async fn delete_page(&self, Parameters(p): Parameters<PageParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::PageTrash { vault: self.vault(), page: p.page }).await)
    }

    #[tool(description = "Append a link to page `to` at the end of page `from`. May wait for the user's approval.")]
    async fn link_pages(&self, Parameters(p): Parameters<LinkParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::LinkAdd { vault: self.vault(), from: p.from, to: p.to }).await)
    }
}

#[tool_handler]
impl ServerHandler for VaultServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().enable_tool_list_changed().build())
            .with_server_info(rmcp::model::Implementation::new("cerbo", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS.to_string())
    }

    /// The agent's name for the change log and approval prompts is the MCP
    /// client's own name, unless `--name` was given.
    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        {
            let mut name = self.link.name.lock().unwrap_or_else(|e| e.into_inner());
            if name.is_none() {
                *name = Some(request.client_info.name.clone());
            }
        }
        context.peer.set_peer_info(request.clone());
        self.negotiate_initialize(&request)
    }

    /// Write tools are listed only while the vault allows the agent to change
    /// it. The daemon enforces this on every write regardless.
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut router = Self::tool_router();
        if !self.link.permission().await.can_write() {
            for name in WRITE_TOOLS {
                router.disable_route(name);
            }
        }
        Ok(ListToolsResult {
            tools: router.list_all(),
            ..Default::default()
        })
    }

    /// Tell the client when the user changes the vault's permission, so the
    /// tool list is read again.
    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        let link = Arc::clone(&self.link);
        let peer = context.peer.clone();
        tokio::spawn(async move {
            let mut last = link.permission().await;
            loop {
                tokio::time::sleep(PERMISSION_POLL).await;
                let now = link.permission().await;
                if now.can_write() != last.can_write() && peer.notify_tool_list_changed().await.is_err() {
                    break;
                }
                last = now;
            }
        });
    }
}

/// Run the server until the client closes stdin.
/// The socket to use instead of the user's daemon: `--socket`, else a
/// non-empty `CERBO_SOCKET`.
pub fn socket_override(flag: Option<std::path::PathBuf>, env: Option<std::ffi::OsString>) -> Option<std::path::PathBuf> {
    flag.or_else(|| env.filter(|v| !v.is_empty()).map(std::path::PathBuf::from))
}

pub async fn run(
    ctx: &CerboContext,
    vault: String,
    name: Option<String>,
    socket: Option<std::path::PathBuf>,
) -> Result<(), String> {
    let paths = DaemonPaths::for_user(&ctx.cache_dir);
    let mut link = Link::new(paths, vault.clone(), name);
    link.socket = socket_override(socket, std::env::var_os("CERBO_SOCKET"));
    let link = Arc::new(link);

    // Start the daemon now, and refuse a vault it does not serve, so a wrong
    // `--vault` fails at once with a message instead of at the first tool call.
    let probe = Arc::clone(&link);
    let outcome = tokio::task::spawn_blocking(move || {
        let mut client = probe.take()?;
        let answer = client.call(Call::Permission { vault: probe.vault.clone() });
        probe.give_back(client);
        answer
    })
    .await
    .map_err(|e| e.to_string())?;
    if let Err(e) = outcome {
        return Err(if e.code == CODE_UNKNOWN_VAULT {
            format!("the vault {vault:?} is not registered: add it with `cerbo vault add <name> <path>` (see `cerbo vault list`)")
        } else {
            e.message
        });
    }

    let service = VaultServer::new(link).serve(stdio()).await.map_err(|e| format!("cannot start the MCP server: {e}"))?;
    service.waiting().await.map_err(|e| format!("the MCP server stopped: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_beats_the_environment_and_empty_means_unset() {
        use std::ffi::OsString;
        use std::path::PathBuf;
        let env = Some(OsString::from("/env.sock"));
        assert_eq!(socket_override(Some("/flag.sock".into()), env.clone()), Some(PathBuf::from("/flag.sock")));
        assert_eq!(socket_override(None, env), Some(PathBuf::from("/env.sock")));
        assert_eq!(socket_override(None, Some(OsString::new())), None);
        assert_eq!(socket_override(None, None), None);
    }

    #[test]
    fn a_socket_override_never_starts_a_daemon() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = DaemonPaths::in_dir(tmp.path().join("run"));
        let mut link = Link::new(paths.clone(), "notes".into(), None);
        link.socket = Some(tmp.path().join("missing.sock"));
        let err = link.take().err().expect("no socket, no connection");
        assert!(err.message.contains("session socket"), "{}", err.message);
        assert!(!paths.dir.exists(), "no daemon directory was created");
    }

    #[test]
    fn a_write_waits_past_the_approval_deadline() {
        // The plain call limit is 60 s; an approval may take 10 minutes.
        assert!(write_wait(None) > Duration::from_secs(DEFAULT_APPROVAL_SECS));
        assert_eq!(write_wait(Some(137)), Duration::from_secs(137) + APPROVAL_MARGIN);
        assert!(write_wait(Some(1)) > Duration::from_secs(1));
        assert!(write_wait(Some(3600)) >= Duration::from_secs(3600 + 1));
    }

    use cerbo_core::rpc::{Request, ServerMessage, CODE_CLIENT_TIMEOUT as TIMEOUT, PROTOCOL_VERSION};
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;

    /// A daemon that answers the permission query at once (approval limit 1 s)
    /// but takes `slow` to answer anything else.
    fn slow_daemon(paths: &DaemonPaths, slow: Duration) {
        paths.prepare_dir().unwrap();
        let listener = UnixListener::bind(&paths.socket).unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut out = stream;
                    let mut line = String::new();
                    let _ = reader.read_line(&mut line); // the hello
                    let welcome =
                        ServerMessage::Welcome { protocol: PROTOCOL_VERSION, daemon_version: "test".into(), pid: 137 };
                    let _ = writeln!(out, "{}", serde_json::to_string(&welcome).unwrap());
                    loop {
                        line.clear();
                        if reader.read_line(&mut line).unwrap_or(0) == 0 {
                            return;
                        }
                        let Ok(request) = serde_json::from_str::<Request>(line.trim_end()) else { continue };
                        let result = match request.call {
                            Call::Permission { .. } => serde_json::json!({ "permission": "approve", "approval_timeout_secs": 1 }),
                            _ => {
                                std::thread::sleep(slow);
                                serde_json::json!({ "uuid": "answered late" })
                            }
                        };
                        let _ = writeln!(out, "{}", serde_json::to_string(&ServerMessage::ok(request.id, result)).unwrap());
                    }
                });
            }
        });
    }

    fn link_with_short_default(paths: DaemonPaths) -> Link {
        let mut link = Link::new(paths, "notes".into(), Some("test".into()));
        link.default_timeout = Some(Duration::from_millis(100));
        link
    }

    #[test]
    fn link_run_waits_for_a_write_past_the_plain_call_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = DaemonPaths::in_dir(tmp.path().join("run"));
        slow_daemon(&paths, Duration::from_millis(500));
        let link = link_with_short_default(paths);
        let edit = Call::PageEdit {
            vault: "notes".into(),
            page: "p".into(),
            content: "c".into(),
            base_version: None,
        };

        // The answer comes after the plain call limit (100 ms here) and long
        // before the approval deadline (1 s + margin): a write must wait for it.
        // With a plain `client.call` this would be a client timeout.
        let answer = link.run(edit).expect("a write waits for the approval deadline, not the plain call limit");
        assert_eq!(answer["uuid"], "answered late");

        // Control: the injected limit is really in force for everything else,
        // so the test above can tell the two apart.
        let err = link.run(Call::PageRead { vault: "notes".into(), page: "p".into() }).unwrap_err();
        assert_eq!(err.code, TIMEOUT);
    }

    #[test]
    fn the_timeout_message_does_not_promise_the_change_was_not_applied() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = DaemonPaths::in_dir(tmp.path().join("run"));
        slow_daemon(&paths, Duration::from_millis(500));
        let link = link_with_short_default(paths);

        let err: RpcError = link.run(Call::PageRead { vault: "notes".into(), page: "p".into() }).unwrap_err();
        assert_eq!(err.code, TIMEOUT);
        assert!(!err.message.contains("cancelled and not applied"), "{}", err.message);
        assert!(err.message.contains("may still have gone through"), "{}", err.message);
        assert!(err.message.contains("Read the page again"), "{}", err.message);
    }
}
