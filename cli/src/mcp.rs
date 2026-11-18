//! `cerbo mcp --vault <name-or-path>`: a stdio MCP server for agent CLIs.
//!
//! It is a shim (design D5): every tool call is forwarded to the daemon over a
//! connection that announces itself as an agent, and the shim opens no vault.
//! Permission is decided in the daemon alone; the shim only asks for the level
//! so it can leave the write tools out of the list at `read`.

use cerbo_core::agent::AgentPermission;
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::client::Client;
use cerbo_core::rpc::{launch, Call, ClientKind, RpcError, CODE_UNKNOWN_VAULT};
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
    vault: String,
    name: Mutex<Option<String>>,
    idle: Mutex<Vec<Client>>,
}

impl Link {
    fn take(&self) -> Result<Client, RpcError> {
        if let Some(client) = self.idle.lock().unwrap_or_else(|e| e.into_inner()).pop() {
            return Ok(client);
        }
        let name = self.name.lock().unwrap_or_else(|e| e.into_inner()).clone();
        launch::connect_or_start(&self.paths, ClientKind::Agent, name.as_deref().or(Some("cerbo mcp")))
            .map_err(RpcError::failed)
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
        match client.call(call.clone()) {
            Ok(value) => {
                self.give_back(client);
                Ok(value)
            }
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

    #[tool(description = "Read one page: its title, markdown content and metadata.")]
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

    #[tool(description = "Replace the whole markdown of a page. May wait for the user's approval.")]
    async fn edit_page(&self, Parameters(p): Parameters<EditParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::PageEdit { vault: self.vault(), page: p.page, content: p.content }).await)
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
pub async fn run(ctx: &CerboContext, vault: String, name: Option<String>) -> Result<(), String> {
    let paths = DaemonPaths::for_user(&ctx.cache_dir);
    let link = Arc::new(Link { paths, vault: vault.clone(), name: Mutex::new(name), idle: Mutex::new(Vec::new()) });

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
