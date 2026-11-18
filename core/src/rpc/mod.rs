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
pub const PROTOCOL_VERSION: u32 = 3;

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
    /// `create`, `edit`, `rename`, `move`, `trash` or `link`.
    pub action: String,
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
}

impl Event {
    /// The id of the vault this event is about, if any.
    pub fn vault(&self) -> Option<&str> {
        match self {
            Event::PageChanged { vault, .. } | Event::ApprovalResolved { vault, .. } => Some(vault),
            Event::ApprovalRequested { approval } => Some(&approval.vault),
            Event::VaultsChanged => None,
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
    /// "approve", "approval_timeout_secs": n}`; the second is how long a write
    /// may wait for an approval, so a client can wait that long for its
    /// answer. Read-only knowledge, so agent connections may ask; it lets
    /// the MCP shim list only the tools that can work.
    Permission { vault: String },

    // Reads
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
}

fn default_limit() -> usize {
    10
}

fn default_log_limit() -> usize {
    50
}

impl Call {
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
        )
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
            | Call::ApprovalAnswer { .. } => None,
            Call::AgentChanges { vault, .. }
            | Call::Permission { vault }
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
            | Call::LinkAdd { vault, .. } => Some(vault),
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
