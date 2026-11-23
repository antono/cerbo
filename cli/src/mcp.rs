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
    CallToolResult, ContentBlock, InitializeRequestParams, InitializeResult, ListResourcesResult, ListToolsResult,
    PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource,
    ResourceContents, ServerCapabilities, ServerConfig,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::transport::stdio;
use rmcp::{tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The tools that change the vault. Not listed at `read`.
pub(crate) const WRITE_TOOLS: [&str; 14] = [
    "create_page",
    "edit_page",
    "rename_page",
    "move_page",
    "delete_page",
    "link_pages",
    "ontology_create",
    "ontology_add_type",
    "ontology_import",
    "index_rebuild",
    "restore_page",
    "view_create",
    "view_update",
    "view_delete",
];

/// The tools that reach files outside the vault. Listed only while the vault's
/// host access is on.
pub(crate) const HOST_TOOLS: [&str; 3] = ["host_list", "host_read", "host_write"];

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
user allowed it; a change may wait for the user's approval, and deleting a page moves it to the vault trash. A view is \
an alternative sidebar tree over the same pages (folders, page references, SPARQL queries): make one to group pages \
for a task instead of moving them.";

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
        // A host write may have applied too, so it is not repeated either.
        let retry = !call.is_write() && !matches!(call, Call::HostWrite { .. });
        let mut client = self.take()?;
        let result = if call.is_write() || call.is_host() {
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

    /// The vault's permission and whether its host access is on. When the
    /// daemon cannot say: `read` and off, the safe answer.
    async fn state(self: &Arc<Self>) -> (AgentPermission, bool) {
        let answer = self.call(Call::Permission { vault: self.vault.clone() }).await.ok();
        let permission = answer
            .as_ref()
            .and_then(|v| v["permission"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or_default();
        let host = answer.as_ref().and_then(|v| v["host"].as_bool()).unwrap_or(false);
        (permission, host)
    }
}

// ── Tool parameters ──────────────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema)]
struct GuideParams {
    /// `usage` (default), `management` or `ontology`.
    topic: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct PageParams {
    /// UUID of the page.
    page: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct HostListParams {
    /// Absolute path of a directory outside the vault.
    path: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct HostReadParams {
    /// Absolute path of a text file outside the vault.
    path: String,
    /// Byte offset to start at; use the `next_offset` of the previous answer to continue. Default 0.
    offset: Option<u64>,
    /// Return at most this many bytes (default and maximum 140288).
    limit: Option<usize>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct HostWriteParams {
    /// Absolute path of the file to create or replace, outside the vault. Its folder must exist.
    path: String,
    /// The whole text of the file.
    content: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct OntologyTypesParams {
    /// The prefix, as in `[Text]{prefix:Type}`, for example `schema` or `ex`.
    prefix: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct OntologyCreateParams {
    /// A new prefix: letters, digits, `-`, `_` or `.`, starting with a letter. It must not be in use.
    prefix: String,
    /// A human title for the vocabulary. Default: `Vocabulary: <prefix>`.
    title: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct OntologyImportParams {
    /// An http or https address of an ontology file (Turtle, RDF/XML...).
    url: String,
    /// A new prefix for it, to use in annotations. Give it together with `namespace`, or neither.
    prefix: Option<String>,
    /// The namespace IRI the ontology's names live in, such as `https://example.org/vocab#`.
    namespace: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct OntologyAddTypeParams {
    /// The prefix of a vocabulary owned by this vault (see ontology_list, `vaultOwned`).
    prefix: String,
    /// The type's name: letters, digits, `_` and `-`, starting with a letter, such as `Recipe`.
    name: String,
    /// A human label. Default: the name.
    label: Option<String>,
    /// A sentence saying what the type means.
    comment: Option<String>,
    /// A broader type: a name in the same vocabulary, or `prefix:Name` such as `schema:CreativeWork`.
    parent: Option<String>,
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

#[derive(Deserialize, schemars::JsonSchema)]
struct ViewParams {
    /// UUID of the view.
    view: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct ViewCreateParams {
    /// The view's title.
    title: String,
    /// The tree, an array of nodes: `{"kind": "folder", "name": "...", "children": [...]}`,
    /// `{"kind": "page", "uuid": "..."}` (an existing page; the same page may appear many
    /// times) and `{"kind": "query", "name": "...", "sparql": "SELECT ?page WHERE {...}"}`.
    nodes: Vec<Value>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct ViewUpdateParams {
    /// UUID of the view.
    view: String,
    /// The new title; omit to keep it.
    title: Option<String>,
    /// The whole new tree, replacing the old one; same shape as in `view_create`.
    nodes: Vec<Value>,
}

/// The tree an agent sent, or the error to hand back to it.
fn view_nodes(nodes: Vec<Value>) -> Result<Vec<cerbo_core::view::ViewNode>, RpcError> {
    serde_json::from_value(Value::Array(nodes))
        .map_err(|e| RpcError::failed(format!("the nodes are not a valid view tree: {e}")))
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

    #[tool(
        description = "Read a guide for working with this vault. `topic` is `usage` (finding and reading: which \
                       search to use, how to read scores), `management` (changing pages safely, folders, links, \
                       deleting, approvals) or `ontology` (types and prefixes, known ontologies, the RDF graph, \
                       SPARQL recipes, creating types). Without a topic you get `usage`. Each guide describes only \
                       the tools you were given. Read the one you need before you start."
    )]
    async fn guide(&self, Parameters(p): Parameters<GuideParams>) -> Result<CallToolResult, ErrorData> {
        let id = p.topic.as_deref().map(str::trim).filter(|t| !t.is_empty()).unwrap_or("usage");
        let Some(topic) = crate::mcp_guide::topic(id) else {
            return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "no guide named {id:?}: the topics are {}",
                crate::mcp_guide::ids()
            ))]));
        };
        let (permission, host) = self.link.state().await;
        Ok(CallToolResult::success(vec![ContentBlock::text(topic.render(permission.can_write(), host))]))
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

    #[tool(description = "List the vault's views as {uuid, title}. A view is an alternative sidebar tree over the same pages.")]
    async fn view_list(&self) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::ViewList { vault: self.vault() }).await)
    }

    #[tool(description = "Read a view's tree: folders, pages (a deleted page is marked missing) and the pages each query node finds.")]
    async fn view_get(&self, Parameters(p): Parameters<ViewParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::ViewGet { vault: self.vault(), view: p.view }).await)
    }

    #[tool(
        description = "Create a view: an alternative sidebar tree over existing pages, for one task. It does not move or \
                       copy pages. Every page must exist and every query must run, or nothing is created. May wait for the \
                       user's approval."
    )]
    async fn view_create(&self, Parameters(p): Parameters<ViewCreateParams>) -> Result<CallToolResult, ErrorData> {
        let call = view_nodes(p.nodes).map(|nodes| Call::ViewCreate { vault: self.vault(), title: p.title, nodes });
        match call {
            Ok(call) => answer(self.link.call(call).await),
            Err(e) => answer(Err(e)),
        }
    }

    #[tool(
        description = "Replace a view's whole tree, and its title when given. Read the view first and send the full tree. \
                       May wait for the user's approval."
    )]
    async fn view_update(&self, Parameters(p): Parameters<ViewUpdateParams>) -> Result<CallToolResult, ErrorData> {
        let call = view_nodes(p.nodes)
            .map(|nodes| Call::ViewUpdate { vault: self.vault(), view: p.view, title: p.title, nodes });
        match call {
            Ok(call) => answer(self.link.call(call).await),
            Err(e) => answer(Err(e)),
        }
    }

    #[tool(description = "Delete a view by moving it to the vault trash; its pages are not touched. May wait for the user's approval.")]
    async fn view_delete(&self, Parameters(p): Parameters<ViewParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::ViewDelete { vault: self.vault(), view: p.view }).await)
    }

    #[tool(
        description = "List a directory outside the vault. A path the user did not allow waits for the user's \
                       approval; some places are never available."
    )]
    async fn host_list(&self, Parameters(p): Parameters<HostListParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::HostList { vault: self.vault(), path: p.path }).await)
    }

    #[tool(
        description = "Read a text file outside the vault, in chunks: pass `next_offset` as `offset` to continue. \
                       A path the user did not allow waits for the user's approval; some places are never available."
    )]
    async fn host_read(&self, Parameters(p): Parameters<HostReadParams>) -> Result<CallToolResult, ErrorData> {
        answer(
            self.link
                .call(Call::HostRead { vault: self.vault(), path: p.path, offset: p.offset.unwrap_or(0), limit: p.limit })
                .await,
        )
    }

    #[tool(
        description = "Create or replace a text file outside the vault. Allowed only under a path the user opened for \
                       writing; any other path waits for the user's approval. The call may wait. Files inside the \
                       vault are changed with the page tools, not with this."
    )]
    async fn host_write(&self, Parameters(p): Parameters<HostWriteParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::HostWrite { vault: self.vault(), path: p.path, content: p.content }).await)
    }

    #[tool(
        description = "List the prefixes you can use in annotations `[Text]{prefix:Type}`: built-in ones such as \
                       `schema` and `foaf`, and the vault's own, with `vaultOwned` marking the vocabularies that \
                       accept new types."
    )]
    async fn ontology_list(&self) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::OntologyList { vault: self.vault() }).await)
    }

    #[tool(
        description = "List the types and properties a vocabulary declares, with labels, descriptions and parent \
                       types. Check this before creating a type. Built-in vocabularies such as `schema` declare \
                       nothing here, but any of their names still works in an annotation."
    )]
    async fn ontology_types(&self, Parameters(p): Parameters<OntologyTypesParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::OntologyTypes { vault: self.vault(), prefix: p.prefix }).await)
    }

    #[tool(
        description = "Say whether the vault's semantic index (the RDF graph SPARQL and search use) is built, usable \
                       and current, how many pages there are and how many have embeddings for search by meaning."
    )]
    async fn index_status(&self) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::IndexStatus { vault: self.vault() }).await)
    }

    #[tool(
        description = "Rebuild the semantic index from the vault's files (without embeddings). Use it when \
                       index_status says stale or unusable, or when SPARQL results miss pages you know exist. May \
                       take a while."
    )]
    async fn index_rebuild(&self) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::IndexRebuild { vault: self.vault() }).await)
    }

    #[tool(
        description = "Fetch an ontology file from an http(s) address and store it as an Ontology object, optionally \
                       registering a prefix for it. It always waits for the user's approval, whatever the vault \
                       permission, because it reaches the network. Imported ontologies cannot be extended with \
                       ontology_add_type."
    )]
    async fn ontology_import(&self, Parameters(p): Parameters<OntologyImportParams>) -> Result<CallToolResult, ErrorData> {
        answer(
            self.link
                .call(Call::OntologyImport { vault: self.vault(), url: p.url, prefix: p.prefix, namespace: p.namespace })
                .await,
        )
    }

    #[tool(description = "List the pages in the vault trash, newest first, as {uuid, title, deleted}.")]
    async fn list_trash(&self) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::TrashList { vault: self.vault() }).await)
    }

    #[tool(
        description = "Put a page back from the trash (the newest entry of that UUID). Refused when a page with the \
                       UUID exists. May wait for the user's approval."
    )]
    async fn restore_page(&self, Parameters(p): Parameters<PageParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::PageRestore { vault: self.vault(), page: p.page }).await)
    }

    #[tool(
        description = "Create a vocabulary owned by this vault, with a new prefix, to hold types of your own. \
                       Returns its prefix and namespace. May wait for the user's approval."
    )]
    async fn ontology_create(&self, Parameters(p): Parameters<OntologyCreateParams>) -> Result<CallToolResult, ErrorData> {
        answer(self.link.call(Call::OntologyCreate { vault: self.vault(), prefix: p.prefix, title: p.title }).await)
    }

    #[tool(
        description = "Add a type to a vocabulary owned by this vault (not to `schema`, `foaf` or imported ones). \
                       Adding a type that exists changes nothing and returns it. Afterwards `[Text]{prefix:Name}` \
                       works. May wait for the user's approval."
    )]
    async fn ontology_add_type(&self, Parameters(p): Parameters<OntologyAddTypeParams>) -> Result<CallToolResult, ErrorData> {
        answer(
            self.link
                .call(Call::OntologyAddType {
                    vault: self.vault(),
                    prefix: p.prefix,
                    name: p.name,
                    label: p.label,
                    comment: p.comment,
                    parent: p.parent,
                })
                .await,
        )
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
        ServerConfig::new(ServerCapabilities::builder().enable_tools().enable_tool_list_changed().enable_resources().build())
            .with_server_info(rmcp::model::Implementation::new("cerbo", env!("CARGO_PKG_VERSION")))
            .with_instructions(format!("{INSTRUCTIONS} {}", crate::mcp_guide::SUMMARY))
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
        let (permission, host) = self.link.state().await;
        if !permission.can_write() {
            for name in WRITE_TOOLS {
                router.disable_route(name);
            }
        }
        if !host {
            for name in HOST_TOOLS {
                router.disable_route(name);
            }
        }
        Ok(ListToolsResult {
            tools: router.list_all(),
            ..Default::default()
        })
    }

    /// The guides as resources, for clients that show or attach resources.
    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let resources = crate::mcp_guide::TOPICS
            .iter()
            .map(|t| {
                Resource::new(crate::mcp_guide::uri(t), format!("guide-{}", t.id))
                    .with_title(t.title)
                    .with_description(t.about)
                    .with_mime_type("text/markdown")
            })
            .collect();
        Ok(ListResourcesResult::with_all_items(resources))
    }

    /// A guide, rendered for what this session may do, like the guide tool.
    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let Some(topic) = crate::mcp_guide::topic_of_uri(&request.uri) else {
            return Err(ErrorData::resource_not_found(format!("no such resource: {}", request.uri), None));
        };
        let (permission, host) = self.link.state().await;
        let text = topic.render(permission.can_write(), host);
        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(text, request.uri.clone()).with_mime_type("text/markdown"),
        ])
        .into())
    }

    /// Tell the client when the user changes the vault's permission, so the
    /// tool list is read again.
    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        let link = Arc::clone(&self.link);
        let peer = context.peer.clone();
        tokio::spawn(async move {
            let mut last = link.state().await;
            loop {
                tokio::time::sleep(PERMISSION_POLL).await;
                let now = link.state().await;
                let changed = now.0.can_write() != last.0.can_write() || now.1 != last.1;
                if changed && peer.notify_tool_list_changed().await.is_err() {
                    break;
                }
                last = now;
            }
        });
    }
}

/// The names of every tool the server can offer, whatever the permissions.
#[cfg(test)]
pub(crate) fn tool_names() -> Vec<String> {
    VaultServer::tool_router().list_all().into_iter().map(|t| t.name.to_string()).collect()
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
