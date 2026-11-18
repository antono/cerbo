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

/// Bumped whenever a message changes in a way an older peer cannot read.
pub const PROTOCOL_VERSION: u32 = 1;

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

/// Something the daemon tells subscribed clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// An object of a vault changed: written, created, trashed, or edited
    /// outside Cerbo.
    PageChanged { vault: String, page: String, change: ChangeKind, origin: Origin },
    /// The list of served vaults changed.
    VaultsChanged,
}

impl Event {
    /// The id of the vault this event is about, if any.
    pub fn vault(&self) -> Option<&str> {
        match self {
            Event::PageChanged { vault, .. } => Some(vault),
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
    /// The vaults served: a list of [`VaultInfo`].
    Vaults,
    /// Stop after queued writes finish.
    Stop,
    /// Receive [`Event`]s on this connection, for one vault or all.
    Subscribe {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vault: Option<String>,
    },

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
    PageEdit { vault: String, page: String, content: String },
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

    /// The vault the call is about, as the client named it.
    pub fn vault(&self) -> Option<&str> {
        match self {
            Call::Ping | Call::Status | Call::Vaults | Call::Stop | Call::Subscribe { .. } => None,
            Call::PageList { vault }
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
    pub vaults: Vec<VaultInfo>,
}

/// One served vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultInfo {
    pub id: String,
    pub name: String,
    pub path: String,
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
            hello: Hello { protocol: PROTOCOL_VERSION, client: ClientKind::Agent, name: Some("opencode".into()) },
        });
        assert_eq!(line, r#"{"hello":{"protocol":1,"client":"agent","name":"opencode"}}"#);
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
        round_trip(ServerMessage::Welcome { protocol: 1, daemon_version: "0.1.0".into(), pid: 137 });
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
        assert!(Call::PageEdit { vault: "v".into(), page: "p".into(), content: "c".into() }.is_write());
        assert!(Call::PageTrash { vault: "v".into(), page: "p".into() }.is_write());
        assert!(!Call::PageRead { vault: "v".into(), page: "p".into() }.is_write());
        assert!(!Call::Sparql { vault: "v".into(), query: "ASK {}".into() }.is_write());
        assert!(!Call::Stop.is_write());
    }

    #[test]
    fn a_protocol_mismatch_tells_the_user_to_restart_the_daemon() {
        assert!(check_protocol(PROTOCOL_VERSION).is_ok());
        let err = check_protocol(PROTOCOL_VERSION + 1).unwrap_err();
        assert_eq!(err.code, CODE_PROTOCOL_MISMATCH);
        assert!(err.message.contains("restart the daemon"), "{}", err.message);
    }
}
