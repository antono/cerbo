//! The wire protocol between `cerboid` and its clients (the desktop app, the
//! `cerbo` CLI, the MCP shim).
//!
//! One JSON object per line over a Unix socket (design D3). A connection
//! starts with a handshake: the client sends [`ClientHello`], the daemon
//! answers [`ServerMessage::Welcome`] or, when the protocol versions differ,
//! [`ServerMessage::Error`] with code [`CODE_PROTOCOL_MISMATCH`] and closes.
//! After that the client sends [`Request`]s, each with an id, and the daemon
//! answers each with a [`ServerMessage::Response`] carrying the same id.
//! Responses may arrive in any order. A client that sent [`Call::Subscribe`]
//! also receives [`ServerMessage::Event`] lines at any time.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(unix)]
pub mod client;
#[cfg(unix)]
pub mod launch;

/// Bumped whenever a message changes in a way an older peer cannot read.
///
/// 2: an agent's hello names the one vault it is bound to.
/// 3: [`Call::PageEdit`] may carry `base_version`; an older daemon would
///    silently ignore it and overwrite a newer page.
/// 4: agent sessions (`agent_list`, `session_*`, [`Event::Session`]) and tool
///    approvals ([`ApprovalInfo`] gained `session`, `tool_title`, `tool_kind`);
///    an older daemon would answer the new calls with a parse error.
/// 5: [`Event::Resync`] (a client that fell behind is told to read again; an
///    older client cannot parse an event kind it does not know) and the
///    `ephemeral` flag of [`Call::SessionCreate`].
pub const PROTOCOL_VERSION: u32 = 5;

/// Error code of a refused handshake.
pub const CODE_PROTOCOL_MISMATCH: &str = "protocol_mismatch";
/// Error code for a malformed line.
pub const CODE_BAD_REQUEST: &str = "bad_request";
/// Error code for a vault the daemon does not serve.
pub const CODE_UNKNOWN_VAULT: &str = "unknown_vault";
/// Error code when the daemon is stopping and takes no more writes.
pub const CODE_SHUTTING_DOWN: &str = "shutting_down";
/// Error code for a failed vault operation; the message says why.
pub const CODE_FAILED: &str = "failed";
/// Error code for an agent write to a vault whose agent permission is `read`.
pub const CODE_READ_ONLY: &str = "read_only";
/// Error code for an agent write that a person rejected.
pub const CODE_REJECTED: &str = "rejected";
/// Error code for an agent write nobody answered in time.
pub const CODE_TIMED_OUT: &str = "timed_out";
/// Error code for a call an agent connection may not make.
pub const CODE_FORBIDDEN: &str = "forbidden";
/// Error code for an approved change that was not applied because the page
/// changed after the agent asked.
pub const CODE_CONFLICT: &str = "conflict";
/// Error code for an agent connection that already has the most approval
/// requests waiting that it may have ([`MAX_PENDING_PER_AGENT`]).
pub const CODE_TOO_MANY_PENDING: &str = "too_many_pending";
/// Error code the blocking client gives when no answer came within its
/// time limit. The request may still be pending in the daemon.
pub const CODE_CLIENT_TIMEOUT: &str = "client_timeout";

/// Error code for a session call the daemon cannot serve (yet, or on this
/// build).
pub const CODE_UNAVAILABLE: &str = "unavailable";

/// How many approval requests one agent connection may have waiting at once.
pub const MAX_PENDING_PER_AGENT: usize = 137;

/// What kind of program is on the other end. Agent connections are subject to
/// the vault's agent permission; app and CLI connections are not (design D5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientKind {
    App,
    Cli,
    Agent,
}

/// Who caused a change, as told in a [`Event::PageChanged`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    App,
    Cli,
    Agent,
    /// Someone edited the files directly, outside every client.
    External,
}

impl From<ClientKind> for Origin {
    fn from(kind: ClientKind) -> Self {
        match kind {
            ClientKind::App => Origin::App,
            ClientKind::Cli => Origin::Cli,
            ClientKind::Agent => Origin::Agent,
        }
    }
}

/// The client's first line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientHello {
    pub hello: Hello,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: u32,
    pub client: ClientKind,
    /// A name for logs, the change log and approval prompts, such as the agent
    /// CLI's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The vault (id, name or path) an agent connection is bound to. Required
    /// for [`ClientKind::Agent`]: the daemon refuses an agent without one, and
    /// then refuses any call that names another vault. Ignored for the app
    /// and the CLI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault: Option<String>,
}

/// A request line: an id chosen by the client plus the call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    #[serde(flatten)]
    pub call: Call,
}

/// An error answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RpcError {
    pub code: String,
    pub message: String,
}

impl RpcError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self { code: code.to_string(), message: message.into() }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self::new(CODE_FAILED, message)
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RpcError {}

/// Whether a page was changed or is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Changed,
    Removed,
}

/// An agent change waiting for a person (`approve` mode).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalInfo {
    pub id: u64,
    /// Vault id.
    pub vault: String,
    pub vault_name: String,
    /// The agent's name.
    pub agent: String,
    /// `create`, `edit`, `rename`, `move`, `trash` or `link` for a vault
    /// change; `tool` for an agent session asking to run one of its own tools
    /// (then `tool_title` and `tool_kind` are set).
    pub action: String,
    /// For `tool`: the id of the agent session that asks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// For `tool`: the agent's title for the tool call, such as a command line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_title: Option<String>,
    /// For `tool`: the agent's kind of tool, as in [`SessionEventBody::ToolCall`]
    /// (`read`, `edit`, `execute`, `fetch`, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_kind: Option<String>,
    /// The page it is about; none for a page that does not exist yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// One line saying what will change.
    pub summary: String,
    /// For an edit or a new page, the difference as a unified diff.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
    /// Unix seconds when it was asked.
    pub requested: u64,
    /// Unix seconds after which it counts as rejected.
    pub deadline: u64,
}

/// How an approval request ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalOutcome {
    Approved,
    Rejected,
    TimedOut,
    /// The daemon stopped, or the vault went away, before anyone answered.
    Cancelled,
}

// ── Agent sessions ───────────────────────────────────────────────────────────

/// Which agent CLI a session runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Opencode,
    Claude,
}

/// What [`Call::AgentList`] says about one agent CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent: AgentKind,
    /// Whether a session with it can be started.
    pub available: bool,
    /// Why not, when it is not available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Where its binary was found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Whether it is signed in, where that can be told.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logged_in: Option<bool>,
}

/// The state of an agent session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Starting,
    Idle,
    Working,
    Ended,
    Failed,
}

/// A page the person attached to a message as context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRef {
    pub path: String,
    pub title: String,
}

/// One agent session, as listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    /// Vault id.
    pub vault: String,
    pub agent: AgentKind,
    pub title: String,
    pub state: SessionState,
    /// RFC 3339.
    pub created_at: String,
    /// RFC 3339.
    pub updated_at: String,
    /// Whether [`Call::SessionCreate`] with `resume_from` can continue it.
    pub resumable: bool,
    /// Whether the agent runs in the sandbox.
    pub sandboxed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// One entry of a session's feed. `seq` grows by one per event within a
/// session, from 1, and is what [`Call::SessionEvents`] pages by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEvent {
    pub seq: u64,
    /// RFC 3339.
    pub at: String,
    pub body: SessionEventBody,
}

/// How a resumed session got its history back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeHow {
    /// The agent loaded its own session.
    Load,
    /// The stored transcript was sent to a new agent session.
    Transcript,
}

/// What happened, tagged by `type`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEventBody {
    UserMessage {
        text: String,
        #[serde(default)]
        context: Vec<PageRef>,
    },
    AgentText { text: String },
    AgentThought { text: String },
    ToolCall {
        id: String,
        title: String,
        kind: String,
        status: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input_summary: Option<String>,
    },
    ToolUpdate {
        id: String,
        status: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_summary: Option<String>,
    },
    ApprovalRequested { approval: Box<ApprovalInfo> },
    ApprovalResolved { approval_id: u64, outcome: ApprovalOutcome },
    PageChanged { path: String },
    TurnEnded { stop_reason: String },
    State {
        state: SessionState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stderr_tail: Option<String>,
    },
    Resumed { how: ResumeHow },
    Error { message: String },
}

/// Something the daemon tells subscribed clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// An object of a vault changed: written, created, trashed, or edited
    /// outside Cerbo.
    PageChanged { vault: String, page: String, change: ChangeKind, origin: Origin },
    /// The list of served vaults changed.
    VaultsChanged,
    /// An agent asked for a change that waits for a person.
    ApprovalRequested { approval: ApprovalInfo },
    /// A pending request ended; clients drop it from their queue.
    ApprovalResolved { vault: String, approval: u64, outcome: ApprovalOutcome },
    /// Something happened in an agent session. Sent to app and CLI
    /// connections only, never to agent connections.
    Session { vault: String, session: String, event: SessionEvent },
    /// This connection fell behind and the daemon dropped events for it: read
    /// the state again (`session_list`, `session_events` after the last `seq`
    /// held, `approval_list`). Sent to app and CLI connections only.
    Resync,
}

impl Event {
    /// The id of the vault this event is about, if any.
    pub fn vault(&self) -> Option<&str> {
        match self {
            Event::PageChanged { vault, .. }
            | Event::ApprovalResolved { vault, .. }
            | Event::Session { vault, .. } => Some(vault),
            Event::ApprovalRequested { approval } => Some(&approval.vault),
            Event::VaultsChanged | Event::Resync => None,
        }
    }
}

/// A line the daemon sends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome { protocol: u32, daemon_version: String, pid: u32 },
    Response {
        id: u64,
        #[serde(default)]
        result: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<RpcError>,
    },
    Event { event: Event },
    /// A connection-level failure (refused handshake, unreadable first line).
    Error { error: RpcError },
}

impl ServerMessage {
    pub fn ok(id: u64, result: Value) -> Self {
        Self::Response { id, result, error: None }
    }

    pub fn err(id: u64, error: RpcError) -> Self {
        Self::Response { id, result: Value::Null, error: Some(error) }
    }
}

/// What a client asks. Calls that change a vault report [`Call::is_write`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Call {
    /// Liveness check; answers `"pong"`.
    Ping,
    /// Version, process id, uptime and the vaults served: a [`StatusInfo`].
    Status,
    /// The vaults served: a list of [`VaultInfo`]. An agent connection sees
    /// only its own vault.
    Vaults,
    /// Stop after queued writes finish.
    Stop,
    /// Receive [`Event`]s on this connection, for one vault or all. An agent
    /// connection only gets the page events of its own vault.
    Subscribe {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vault: Option<String>,
    },

    /// The pending approval requests, of one vault or all: a list of
    /// [`ApprovalInfo`]. Not for agent connections.
    ApprovalList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vault: Option<String>,
    },
    /// Approve or reject a pending request. Not for agent connections.
    ApprovalAnswer { approval: u64, approve: bool },
    /// The last applied agent changes of a vault, oldest first: a list of
    /// [`crate::agent::ChangeEntry`]. Not for agent connections.
    AgentChanges {
        vault: String,
        #[serde(default = "default_log_limit")]
        limit: usize,
    },
    /// The agent permission of a vault: `{"permission": "read" | "auto" |
    /// "approve", "approval_timeout_secs": n, "host": bool}`; the second is how
    /// long a write may wait for an approval, so a client can wait that long
    /// for its answer; the third says whether host access is on. Read-only knowledge, so agent connections may ask; it lets
    /// the MCP shim list only the tools that can work.
    Permission { vault: String },

    // Host access: files outside the vault. They never go through the vault's
    // write queue; the daemon judges each one by the vault's host access
    // setting and may ask a person first. Agent connections only.
    /// List a directory outside the vault.
    HostList { vault: String, path: String },
    /// Read a chunk of a text file outside the vault.
    HostRead {
        vault: String,
        path: String,
        #[serde(default)]
        offset: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<usize>,
    },
    /// Create or replace a text file outside the vault.
    HostWrite { vault: String, path: String, content: String },

    // Reads
    /// The prefixes the vault can annotate with.
    OntologyList { vault: String },
    /// The declared types of one prefix's ontology.
    OntologyTypes { vault: String, prefix: String },
    PageList { vault: String },
    PageRead { vault: String, page: String },
    /// The attachments of one page, or of the whole vault without `page`.
    AttachmentList {
        vault: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page: Option<String>,
    },
    /// Metadata and the stored file's absolute path of one attachment.
    AttachmentInfo { vault: String, attachment: String },
    /// Every link in the vault as `{from, to}` pairs, live targets only.
    Links { vault: String },
    Backlinks { vault: String, page: String },
    OutgoingLinks { vault: String, page: String },
    /// Case-insensitive substring search over titles and page text.
    SearchText {
        vault: String,
        query: String,
        #[serde(default = "default_limit")]
        limit: usize,
    },
    SearchSemantic {
        vault: String,
        query: String,
        #[serde(default = "default_limit")]
        limit: usize,
        #[serde(default)]
        types: Vec<String>,
        #[serde(default)]
        annotated: Vec<String>,
    },
    Similar {
        vault: String,
        page: String,
        #[serde(default = "default_limit")]
        limit: usize,
        #[serde(default)]
        types: Vec<String>,
        #[serde(default)]
        annotated: Vec<String>,
    },
    /// Run a SPARQL query; the result is the SPARQL JSON results document
    /// (JSON-LD for CONSTRUCT and DESCRIBE).
    Sparql { vault: String, query: String },

    // Writes
    PageCreate {
        vault: String,
        title: String,
        /// The page text; the default is a heading with the title.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        /// The page's virtual path (its folder).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
    /// Replace the page's text. With `base_version`, the `version` that
    /// [`Call::PageRead`] returned for the text the new content was written
    /// against, the daemon refuses with [`CODE_CONFLICT`] when the page is
    /// not that text any more. The check runs inside the write queue.
    PageEdit {
        vault: String,
        page: String,
        content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_version: Option<String>,
    },
    PageRename { vault: String, page: String, title: String },
    /// Set or clear (`None`) the page's virtual path.
    PageMove {
        vault: String,
        page: String,
        #[serde(default)]
        path: Option<String>,
    },
    /// Move the page to the vault trash.
    PageTrash { vault: String, page: String },
    /// Append a link to `to` at the end of page `from`.
    LinkAdd { vault: String, from: String, to: String },
    /// Create a vault-owned ontology with its own prefix.
    OntologyCreate {
        vault: String,
        prefix: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// Add a type to a vault-owned ontology.
    OntologyAddType {
        vault: String,
        prefix: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        comment: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
    },

    // Agent sessions. None of these are for agent connections.
    /// The agent CLIs and whether each can run: a list of [`AgentInfo`].
    AgentList {},
    /// Start a session in a vault: answers a [`SessionInfo`] at once, in the
    /// `starting` state; the agent's start finishes in the background and is
    /// told by `state` events (`idle`, or `failed` with the reason). With
    /// `resume_from`, continue that earlier session of the same vault. With
    /// `ephemeral`, the daemon ends the session when this connection closes.
    SessionCreate {
        vault: String,
        agent: AgentKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resume_from: Option<String>,
        #[serde(default, skip_serializing_if = "is_false")]
        ephemeral: bool,
    },
    /// Send a message to the agent; answers null once it is accepted. The
    /// turn's progress arrives as [`Event::Session`].
    SessionSend {
        session: String,
        text: String,
        #[serde(default)]
        context: Vec<PageRef>,
    },
    /// Cancel the agent's current turn.
    SessionCancel { session: String },
    /// End the session and stop the agent process.
    SessionEnd { session: String },
    /// The sessions of a vault or all, newest first: a list of [`SessionInfo`].
    SessionList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vault: Option<String>,
    },
    /// Delete an ended session and its stored feed.
    SessionDelete { session: String },
    /// Replay a session's stored feed: a list of [`SessionEvent`] with `seq`
    /// greater than `after_seq` (all when absent), at most `limit`.
    SessionEvents {
        session: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after_seq: Option<u64>,
        #[serde(default = "default_events_limit")]
        limit: usize,
    },
}

fn default_limit() -> usize {
    10
}

fn default_events_limit() -> usize {
    500
}

fn default_log_limit() -> usize {
    50
}

impl Call {
    /// True for `agent_list` and the `session_*` calls; agent connections
    /// may not make them.
    pub fn is_session_call(&self) -> bool {
        matches!(
            self,
            Call::AgentList {}
                | Call::SessionCreate { .. }
                | Call::SessionSend { .. }
                | Call::SessionCancel { .. }
                | Call::SessionEnd { .. }
                | Call::SessionList { .. }
                | Call::SessionDelete { .. }
                | Call::SessionEvents { .. }
        )
    }

    /// True for calls that change a vault; they go through its write queue.
    pub fn is_write(&self) -> bool {
        matches!(
            self,
            Call::PageCreate { .. }
                | Call::PageEdit { .. }
                | Call::PageRename { .. }
                | Call::PageMove { .. }
                | Call::PageTrash { .. }
                | Call::LinkAdd { .. }
                | Call::OntologyCreate { .. }
                | Call::OntologyAddType { .. }
        )
    }

    /// True for the calls that reach files outside the vault.
    pub fn is_host(&self) -> bool {
        matches!(self, Call::HostList { .. } | Call::HostRead { .. } | Call::HostWrite { .. })
    }

    /// What a write does, in the word the change log and approval prompts use.
    pub fn action(&self) -> Option<&'static str> {
        Some(match self {
            Call::PageCreate { .. } => "create",
            Call::PageEdit { .. } => "edit",
            Call::PageRename { .. } => "rename",
            Call::PageMove { .. } => "move",
            Call::PageTrash { .. } => "trash",
            Call::LinkAdd { .. } => "link",
            Call::OntologyCreate { .. } => "ontology",
            Call::OntologyAddType { .. } => "add_type",
            _ => return None,
        })
    }

    /// The vault the call is about, as the client named it.
    pub fn vault(&self) -> Option<&str> {
        match self {
            Call::Ping
            | Call::Status
            | Call::Vaults
            | Call::Stop
            | Call::Subscribe { .. }
            | Call::ApprovalList { .. }
            | Call::ApprovalAnswer { .. }
            | Call::AgentList {}
            | Call::SessionSend { .. }
            | Call::SessionCancel { .. }
            | Call::SessionEnd { .. }
            | Call::SessionDelete { .. }
            | Call::SessionEvents { .. } => None,
            Call::SessionList { vault } => vault.as_deref(),
            Call::AgentChanges { vault, .. }
            | Call::Permission { vault }
            | Call::HostList { vault, .. }
            | Call::HostRead { vault, .. }
            | Call::HostWrite { vault, .. }
            | Call::PageList { vault }
            | Call::PageRead { vault, .. }
            | Call::AttachmentList { vault, .. }
            | Call::AttachmentInfo { vault, .. }
            | Call::Links { vault }
            | Call::Backlinks { vault, .. }
            | Call::OutgoingLinks { vault, .. }
            | Call::SearchText { vault, .. }
            | Call::SearchSemantic { vault, .. }
            | Call::Similar { vault, .. }
            | Call::Sparql { vault, .. }
            | Call::PageCreate { vault, .. }
            | Call::PageEdit { vault, .. }
            | Call::PageRename { vault, .. }
            | Call::PageMove { vault, .. }
            | Call::PageTrash { vault, .. }
            | Call::SessionCreate { vault, .. }
            | Call::LinkAdd { vault, .. }
            | Call::OntologyList { vault }
            | Call::OntologyTypes { vault, .. }
            | Call::OntologyCreate { vault, .. }
            | Call::OntologyAddType { vault, .. } => Some(vault),
        }
    }
}

/// The answer to [`Call::Status`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusInfo {
    pub version: String,
    pub protocol: u32,
    pub pid: u32,
    pub uptime_secs: u64,
    /// Resident memory of the daemon in KiB, where the system tells.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_kb: Option<u64>,
    pub vaults: Vec<VaultInfo>,
}

/// One served vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

// ── Version-independent administration ───────────────────────────────────────
//
// The rest of the protocol changes between versions, so a client of one
// version cannot talk to a daemon of another, not even to stop it. These two
// lines are the exception and never change shape: instead of a hello, the
// first line of a connection may be an [`AdminRequest`]; the daemon answers
// one [`AdminReply`] line, whatever its protocol version, and closes. A daemon
// that predates them answers with an error line or closes, which the client
// reads as "not supported".

/// What an admin line asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminAction {
    /// Say who you are.
    Status,
    /// Say who you are, then stop after queued writes finish.
    Stop,
}

/// The first line of an administration connection: `{"admin":"status"}` or
/// `{"admin":"stop"}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminRequest {
    pub admin: AdminAction,
}

/// What the daemon says about itself to an admin line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminInfo {
    pub protocol: u32,
    pub version: String,
    pub pid: u32,
    pub uptime_secs: u64,
}

/// The daemon's answer to an [`AdminRequest`]: `{"admin":{...}}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminReply {
    pub admin: AdminInfo,
}

/// Judge a peer's protocol version. Shared by the daemon and by the clients,
/// so both sides decide a mismatch the same way.
pub fn check_protocol(peer_protocol: u32) -> Result<(), RpcError> {
    if peer_protocol == PROTOCOL_VERSION {
        return Ok(());
    }
    Err(RpcError::new(
        CODE_PROTOCOL_MISMATCH,
        format!(
            "this program speaks protocol {PROTOCOL_VERSION} but the other side speaks {peer_protocol}: \
             restart the daemon (`cerbo daemon stop`, then start it again)"
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug>(value: T) -> String {
        let line = serde_json::to_string(&value).unwrap();
        assert!(!line.contains('\n'), "a message is one line");
        let back: T = serde_json::from_str(&line).unwrap();
        assert_eq!(back, value);
        line
    }

    #[test]
    fn hello_round_trips() {
        let line = round_trip(ClientHello {
            hello: Hello {
                protocol: PROTOCOL_VERSION,
                client: ClientKind::Agent,
                name: Some("opencode".into()),
                vault: Some("Notes".into()),
            },
        });
        assert_eq!(
            line,
            format!(r#"{{"hello":{{"protocol":{PROTOCOL_VERSION},"client":"agent","name":"opencode","vault":"Notes"}}}}"#)
        );
    }

    #[test]
    fn requests_round_trip_with_their_wire_form() {
        let line = round_trip(Request { id: 137, call: Call::Ping });
        assert_eq!(line, r#"{"id":137,"method":"ping"}"#);

        let line = round_trip(Request {
            id: 1,
            call: Call::PageRead { vault: "v".into(), page: "p".into() },
        });
        assert_eq!(line, r#"{"id":1,"method":"page_read","params":{"vault":"v","page":"p"}}"#);

        round_trip(Request { id: 2, call: Call::Subscribe { vault: None } });
        round_trip(Request {
            id: 3,
            call: Call::SearchSemantic {
                vault: "v".into(),
                query: "berlin".into(),
                limit: 5,
                types: vec!["schema:Person".into()],
                annotated: vec![],
            },
        });
        round_trip(Request {
            id: 4,
            call: Call::PageCreate { vault: "v".into(), title: "T".into(), content: None, path: Some("a/b".into()) },
        });
        round_trip(Request { id: 5, call: Call::PageMove { vault: "v".into(), page: "p".into(), path: None } });
    }

    #[test]
    fn approval_calls_and_events_round_trip() {
        round_trip(Request { id: 1, call: Call::ApprovalList { vault: None } });
        round_trip(Request { id: 2, call: Call::ApprovalAnswer { approval: 137, approve: true } });
        round_trip(Request { id: 3, call: Call::AgentChanges { vault: "v".into(), limit: 5 } });
        round_trip(Request { id: 4, call: Call::Permission { vault: "v".into() } });
        let info = ApprovalInfo {
            id: 137,
            vault: "v".into(),
            vault_name: "Notes".into(),
            agent: "opencode".into(),
            action: "edit".into(),
            session: None,
            tool_title: None,
            tool_kind: None,
            page: Some("p".into()),
            summary: "Edit \"T\"".into(),
            diff: Some("-a\n+b\n".into()),
            requested: 1,
            deadline: 601,
        };
        round_trip(ServerMessage::Event { event: Event::ApprovalRequested { approval: info.clone() } });
        round_trip(ServerMessage::Event {
            event: Event::ApprovalResolved { vault: "v".into(), approval: 137, outcome: ApprovalOutcome::TimedOut },
        });
        assert_eq!(Event::ApprovalRequested { approval: info }.vault(), Some("v"));
    }


    fn sample_session() -> SessionInfo {
        SessionInfo {
            id: "s137".into(),
            vault: "v".into(),
            agent: AgentKind::Opencode,
            title: "Plan".into(),
            state: SessionState::Working,
            created_at: "2026-11-18T10:00:00+00:00".into(),
            updated_at: "2026-11-18T10:01:00+00:00".into(),
            resumable: true,
            sandboxed: false,
            warning: None,
        }
    }

    #[test]
    fn session_calls_round_trip_with_their_wire_form() {
        let line = round_trip(Request { id: 1, call: Call::AgentList {} });
        assert_eq!(line, r#"{"id":1,"method":"agent_list","params":{}}"#);
        let line = round_trip(Request {
            id: 2,
            call: Call::SessionCreate { vault: "v".into(), agent: AgentKind::Claude, resume_from: None, ephemeral: false },
        });
        assert_eq!(line, r#"{"id":2,"method":"session_create","params":{"vault":"v","agent":"claude"}}"#);
        round_trip(Request {
            id: 3,
            call: Call::SessionCreate { vault: "v".into(), agent: AgentKind::Opencode, resume_from: Some("s1".into()), ephemeral: false },
        });
        let line = round_trip(Request {
            id: 4,
            call: Call::SessionSend {
                session: "s".into(),
                text: "hi".into(),
                context: vec![PageRef { path: "a/b".into(), title: "B".into() }],
            },
        });
        assert_eq!(
            line,
            r#"{"id":4,"method":"session_send","params":{"session":"s","text":"hi","context":[{"path":"a/b","title":"B"}]}}"#
        );
        round_trip(Request { id: 5, call: Call::SessionCancel { session: "s".into() } });
        round_trip(Request { id: 6, call: Call::SessionEnd { session: "s".into() } });
        round_trip(Request { id: 7, call: Call::SessionList { vault: None } });
        round_trip(Request { id: 8, call: Call::SessionList { vault: Some("v".into()) } });
        round_trip(Request { id: 9, call: Call::SessionDelete { session: "s".into() } });
        let line = round_trip(Request {
            id: 10,
            call: Call::SessionEvents { session: "s".into(), after_seq: Some(137), limit: 50 },
        });
        assert_eq!(line, r#"{"id":10,"method":"session_events","params":{"session":"s","after_seq":137,"limit":50}}"#);
    }

    #[test]
    fn session_call_params_take_their_defaults() {
        let req: Request =
            serde_json::from_str(r#"{"id":1,"method":"session_send","params":{"session":"s","text":"x"}}"#).unwrap();
        assert_eq!(req.call, Call::SessionSend { session: "s".into(), text: "x".into(), context: vec![] });
        let req: Request = serde_json::from_str(r#"{"id":1,"method":"session_events","params":{"session":"s"}}"#).unwrap();
        assert_eq!(req.call, Call::SessionEvents { session: "s".into(), after_seq: None, limit: 500 });
    }

    #[test]
    fn session_calls_know_their_vault_and_kind() {
        assert!(Call::AgentList {}.is_session_call());
        assert!(Call::SessionEnd { session: "s".into() }.is_session_call());
        assert!(!Call::Ping.is_session_call());
        assert_eq!(Call::SessionList { vault: Some("v".into()) }.vault(), Some("v"));
        assert_eq!(Call::SessionList { vault: None }.vault(), None);
        assert_eq!(
            Call::SessionCreate { vault: "v".into(), agent: AgentKind::Claude, resume_from: None, ephemeral: false }.vault(),
            Some("v")
        );
        assert!(!Call::SessionSend { session: "s".into(), text: "x".into(), context: vec![] }.is_write());
    }

    #[test]
    fn agent_and_session_info_round_trip() {
        let line = round_trip(AgentInfo {
            agent: AgentKind::Opencode,
            available: false,
            reason: Some("not installed".into()),
            path: None,
            version: None,
            logged_in: None,
        });
        assert_eq!(line, r#"{"agent":"opencode","available":false,"reason":"not installed"}"#);
        round_trip(AgentInfo {
            agent: AgentKind::Claude,
            available: true,
            reason: None,
            path: Some("/bin/claude".into()),
            version: Some("2.0".into()),
            logged_in: Some(true),
        });
        let line = round_trip(sample_session());
        assert!(!line.contains("warning"), "{line}");
        round_trip(SessionInfo { warning: Some("no sandbox".into()), sandboxed: true, ..sample_session() });
        for (state, word) in [
            (SessionState::Starting, "starting"),
            (SessionState::Idle, "idle"),
            (SessionState::Working, "working"),
            (SessionState::Ended, "ended"),
            (SessionState::Failed, "failed"),
        ] {
            assert_eq!(serde_json::to_string(&state).unwrap(), format!("\"{word}\""));
        }
    }

    fn tool_approval() -> ApprovalInfo {
        ApprovalInfo {
            id: 137,
            vault: "v".into(),
            vault_name: "Notes".into(),
            agent: "opencode".into(),
            action: "tool".into(),
            session: Some("s137".into()),
            tool_title: Some("ls -la".into()),
            tool_kind: Some("execute".into()),
            page: None,
            summary: "Run: ls -la".into(),
            diff: None,
            requested: 1,
            deadline: 601,
        }
    }

    #[test]
    fn every_session_event_body_round_trips_with_its_tag() {
        let bodies = [
            (SessionEventBody::UserMessage { text: "hi".into(), context: vec![] }, "user_message"),
            (SessionEventBody::AgentText { text: "a".into() }, "agent_text"),
            (SessionEventBody::AgentThought { text: "t".into() }, "agent_thought"),
            (
                SessionEventBody::ToolCall {
                    id: "t1".into(),
                    title: "Read".into(),
                    kind: "read".into(),
                    status: "pending".into(),
                    input_summary: Some("a.md".into()),
                },
                "tool_call",
            ),
            (
                SessionEventBody::ToolUpdate { id: "t1".into(), status: "completed".into(), output_summary: None },
                "tool_update",
            ),
            (SessionEventBody::ApprovalRequested { approval: Box::new(tool_approval()) }, "approval_requested"),
            (
                SessionEventBody::ApprovalResolved { approval_id: 137, outcome: ApprovalOutcome::Approved },
                "approval_resolved",
            ),
            (SessionEventBody::PageChanged { path: "a/b".into() }, "page_changed"),
            (SessionEventBody::TurnEnded { stop_reason: "end_turn".into() }, "turn_ended"),
            (
                SessionEventBody::State {
                    state: SessionState::Failed,
                    reason: Some("exited".into()),
                    stderr_tail: Some("boom".into()),
                },
                "state",
            ),
            (SessionEventBody::Resumed { how: ResumeHow::Load }, "resumed"),
            (SessionEventBody::Error { message: "x".into() }, "error"),
        ];
        for (body, tag) in bodies {
            let line = round_trip(SessionEvent { seq: 1, at: "2026-11-18T10:00:00+00:00".into(), body });
            assert!(line.contains(&format!(r#""body":{{"type":"{tag}""#)), "{line}");
        }
        let line = round_trip(SessionEvent {
            seq: 2,
            at: "t".into(),
            body: SessionEventBody::Resumed { how: ResumeHow::Transcript },
        });
        assert!(line.contains(r#""how":"transcript""#), "{line}");
        // The context may be omitted.
        let body: SessionEventBody = serde_json::from_str(r#"{"type":"user_message","text":"x"}"#).unwrap();
        assert_eq!(body, SessionEventBody::UserMessage { text: "x".into(), context: vec![] });
    }

    #[test]
    fn the_session_event_is_delivered_with_its_vault() {
        let event = Event::Session {
            vault: "v".into(),
            session: "s137".into(),
            event: SessionEvent { seq: 3, at: "t".into(), body: SessionEventBody::AgentText { text: "hello".into() } },
        };
        let line = round_trip(ServerMessage::Event { event: event.clone() });
        assert_eq!(
            line,
            r#"{"type":"event","event":{"kind":"session","vault":"v","session":"s137","event":{"seq":3,"at":"t","body":{"type":"agent_text","text":"hello"}}}}"#
        );
        assert_eq!(event.vault(), Some("v"));
    }

    #[test]
    fn a_vault_approval_keeps_its_old_wire_form_and_a_tool_approval_adds_fields() {
        let old = ApprovalInfo {
            id: 1,
            vault: "v".into(),
            vault_name: "N".into(),
            agent: "a".into(),
            action: "edit".into(),
            session: None,
            tool_title: None,
            tool_kind: None,
            page: Some("p".into()),
            summary: "s".into(),
            diff: None,
            requested: 1,
            deadline: 2,
        };
        let line = round_trip(old);
        assert!(!line.contains("session") && !line.contains("tool_"), "{line}");
        // A version-3 line without the new fields still reads.
        let parsed: ApprovalInfo = serde_json::from_str(
            r#"{"id":1,"vault":"v","vault_name":"N","agent":"a","action":"edit","summary":"s","requested":1,"deadline":2}"#,
        )
        .unwrap();
        assert_eq!(parsed.session, None);
        let line = round_trip(tool_approval());
        assert!(line.contains(r#""session":"s137""#) && line.contains(r#""tool_kind":"execute""#), "{line}");
    }

    #[test]
    fn only_writes_have_an_action_word() {
        assert_eq!(Call::PageTrash { vault: "v".into(), page: "p".into() }.action(), Some("trash"));
        assert_eq!(Call::Ping.action(), None);
        assert_eq!(Call::PageList { vault: "v".into() }.action(), None);
    }

    #[test]
    fn omitted_params_take_their_defaults() {
        let req: Request =
            serde_json::from_str(r#"{"id":9,"method":"search_text","params":{"vault":"v","query":"x"}}"#).unwrap();
        assert_eq!(req.call, Call::SearchText { vault: "v".into(), query: "x".into(), limit: 10 });
    }

    #[test]
    fn an_unknown_method_is_an_error_not_a_panic() {
        assert!(serde_json::from_str::<Request>(r#"{"id":1,"method":"format_disk"}"#).is_err());
    }

    #[test]
    fn server_messages_round_trip() {
        round_trip(ServerMessage::Welcome { protocol: PROTOCOL_VERSION, daemon_version: "0.1.0".into(), pid: 137 });
        round_trip(ServerMessage::ok(1, serde_json::json!({"a": [1, 2]})));
        round_trip(ServerMessage::err(2, RpcError::failed("no such page")));
        round_trip(ServerMessage::Event {
            event: Event::PageChanged {
                vault: "v".into(),
                page: "p".into(),
                change: ChangeKind::Changed,
                origin: Origin::External,
            },
        });
        round_trip(ServerMessage::Event { event: Event::VaultsChanged });
        let line = round_trip(ServerMessage::Event { event: Event::Resync });
        assert_eq!(line, r#"{"type":"event","event":{"kind":"resync"}}"#);
        round_trip(ServerMessage::Error { error: RpcError::new(CODE_BAD_REQUEST, "x") });
    }

    #[test]
    fn a_success_response_has_no_error_field() {
        let line = serde_json::to_string(&ServerMessage::ok(1, Value::Null)).unwrap();
        assert!(!line.contains("error"), "{line}");
    }

    #[test]
    fn writes_are_the_calls_that_change_a_vault() {
        assert!(
            Call::PageEdit { vault: "v".into(), page: "p".into(), content: "c".into(), base_version: None }.is_write()
        );
        assert!(Call::PageTrash { vault: "v".into(), page: "p".into() }.is_write());
        assert!(!Call::PageRead { vault: "v".into(), page: "p".into() }.is_write());
        assert!(!Call::Sparql { vault: "v".into(), query: "ASK {}".into() }.is_write());
        assert!(!Call::Stop.is_write());
    }

    #[test]
    fn admin_lines_have_a_frozen_wire_form() {
        // These strings are what every daemon version must keep
        // understanding: changing them breaks `cerbo daemon stop` across
        // upgrades.
        assert_eq!(serde_json::to_string(&AdminRequest { admin: AdminAction::Stop }).unwrap(), r#"{"admin":"stop"}"#);
        assert_eq!(
            serde_json::to_string(&AdminRequest { admin: AdminAction::Status }).unwrap(),
            r#"{"admin":"status"}"#
        );
        let reply: AdminReply =
            serde_json::from_str(r#"{"admin":{"protocol":137,"version":"9","pid":1,"uptime_secs":2}}"#).unwrap();
        assert_eq!(reply.admin.protocol, 137);
        // A hello is not an admin line, so the two never get confused.
        assert!(serde_json::from_str::<AdminRequest>(r#"{"hello":{"protocol":3,"client":"cli"}}"#).is_err());
    }

    #[test]
    fn an_edit_without_a_base_version_has_the_old_wire_form() {
        let line = round_trip(Request {
            id: 1,
            call: Call::PageEdit { vault: "v".into(), page: "p".into(), content: "c".into(), base_version: None },
        });
        assert!(!line.contains("base_version"), "{line}");
        round_trip(Request {
            id: 2,
            call: Call::PageEdit {
                vault: "v".into(),
                page: "p".into(),
                content: "c".into(),
                base_version: Some("00ff".into()),
            },
        });
    }

    #[test]
    fn a_protocol_mismatch_tells_the_user_to_restart_the_daemon() {
        assert!(check_protocol(PROTOCOL_VERSION).is_ok());
        let err = check_protocol(PROTOCOL_VERSION + 1).unwrap_err();
        assert_eq!(err.code, CODE_PROTOCOL_MISMATCH);
        assert!(err.message.contains("restart the daemon"), "{}", err.message);
    }
}
