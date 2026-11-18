//! The ACP messages the client sends and reads, checked against the official
//! `schema/v1/schema.json` of github.com/agentclientprotocol/agent-client-protocol
//! (field names are camelCase on the wire).
//!
//! What the agent sends is parsed leniently: an unknown enum value or an unknown
//! block type is kept (as `Other` / `Unknown`) instead of failing the whole
//! message, because agents run newer protocol minors than this client.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Map, Value};

/// The one protocol major version this client speaks.
pub const PROTOCOL_VERSION: u64 = 1;

/// A string-valued enum with a catch-all, so new values from newer agents are
/// kept and never fail a parse.
macro_rules! str_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $s:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $name {
            $($variant,)+
            /// A value this client does not know.
            Other(String),
        }

        impl $name {
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $s,)+
                    Self::Other(s) => s,
                }
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                match s {
                    $($s => Self::$variant,)+
                    other => Self::Other(other.to_string()),
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                Ok(Self::from(s.as_str()))
            }
        }
    };
}

str_enum! {
    /// Why a prompt turn ended.
    StopReason {
        EndTurn = "end_turn",
        MaxTokens = "max_tokens",
        MaxTurnRequests = "max_turn_requests",
        Refusal = "refusal",
        Cancelled = "cancelled",
    }
}

str_enum! {
    ToolKind {
        Read = "read",
        Edit = "edit",
        Delete = "delete",
        Move = "move",
        Search = "search",
        Execute = "execute",
        Think = "think",
        Fetch = "fetch",
        SwitchMode = "switch_mode",
        OtherKind = "other",
    }
}

str_enum! {
    ToolStatus {
        Pending = "pending",
        InProgress = "in_progress",
        Completed = "completed",
        Failed = "failed",
    }
}

str_enum! {
    PermissionKind {
        AllowOnce = "allow_once",
        AllowAlways = "allow_always",
        RejectOnce = "reject_once",
        RejectAlways = "reject_always",
    }
}

str_enum! {
    PlanPriority { High = "high", Medium = "medium", Low = "low" }
}

str_enum! {
    PlanStatus { Pending = "pending", InProgress = "in_progress", Completed = "completed" }
}

fn str_of(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

// ---------------------------------------------------------------------------
// Content blocks
// ---------------------------------------------------------------------------

/// The contents of an embedded resource.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceContents {
    Text { uri: String, text: String, mime_type: Option<String> },
    /// `blob` is base64.
    Blob { uri: String, blob: String, mime_type: Option<String> },
}

/// A content block, in prompts (client to agent) and in updates (agent to
/// client).
#[derive(Debug, Clone, PartialEq)]
pub enum ContentBlock {
    Text { text: String },
    Image { data: String, mime_type: String, uri: Option<String> },
    Audio { data: String, mime_type: String },
    ResourceLink { name: String, uri: String, mime_type: Option<String>, size: Option<u64>, title: Option<String>, description: Option<String> },
    Resource { resource: ResourceContents },
    /// A block type this client does not know, kept as it came.
    Unknown(Value),
}

impl ContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    pub fn resource_link(name: impl Into<String>, uri: impl Into<String>) -> Self {
        Self::ResourceLink { name: name.into(), uri: uri.into(), mime_type: None, size: None, title: None, description: None }
    }

    /// The text of a text block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Self::Text { text } => json!({"type": "text", "text": text}),
            Self::Image { data, mime_type, uri } => {
                let mut m = json!({"type": "image", "data": data, "mimeType": mime_type});
                if let Some(uri) = uri {
                    m["uri"] = json!(uri);
                }
                m
            }
            Self::Audio { data, mime_type } => json!({"type": "audio", "data": data, "mimeType": mime_type}),
            Self::ResourceLink { name, uri, mime_type, size, title, description } => {
                let mut m = json!({"type": "resource_link", "name": name, "uri": uri});
                if let Some(v) = mime_type {
                    m["mimeType"] = json!(v);
                }
                if let Some(v) = size {
                    m["size"] = json!(v);
                }
                if let Some(v) = title {
                    m["title"] = json!(v);
                }
                if let Some(v) = description {
                    m["description"] = json!(v);
                }
                m
            }
            Self::Resource { resource } => {
                let inner = match resource {
                    ResourceContents::Text { uri, text, mime_type } => {
                        let mut m = json!({"uri": uri, "text": text});
                        if let Some(v) = mime_type {
                            m["mimeType"] = json!(v);
                        }
                        m
                    }
                    ResourceContents::Blob { uri, blob, mime_type } => {
                        let mut m = json!({"uri": uri, "blob": blob});
                        if let Some(v) = mime_type {
                            m["mimeType"] = json!(v);
                        }
                        m
                    }
                };
                json!({"type": "resource", "resource": inner})
            }
            Self::Unknown(v) => v.clone(),
        }
    }

    pub fn from_value(v: &Value) -> Self {
        let parsed = (|| -> Option<Self> {
            match v.get("type")?.as_str()? {
                "text" => Some(Self::Text { text: str_of(v, "text")? }),
                "image" => Some(Self::Image { data: str_of(v, "data")?, mime_type: str_of(v, "mimeType")?, uri: str_of(v, "uri") }),
                "audio" => Some(Self::Audio { data: str_of(v, "data")?, mime_type: str_of(v, "mimeType")? }),
                "resource_link" => Some(Self::ResourceLink {
                    name: str_of(v, "name")?,
                    uri: str_of(v, "uri")?,
                    mime_type: str_of(v, "mimeType"),
                    size: v.get("size").and_then(Value::as_u64),
                    title: str_of(v, "title"),
                    description: str_of(v, "description"),
                }),
                "resource" => {
                    let r = v.get("resource")?;
                    let uri = str_of(r, "uri")?;
                    let mime_type = str_of(r, "mimeType");
                    if let Some(text) = str_of(r, "text") {
                        Some(Self::Resource { resource: ResourceContents::Text { uri, text, mime_type } })
                    } else {
                        Some(Self::Resource { resource: ResourceContents::Blob { uri, blob: str_of(r, "blob")?, mime_type } })
                    }
                }
                _ => None,
            }
        })();
        parsed.unwrap_or_else(|| Self::Unknown(v.clone()))
    }
}

impl Serialize for ContentBlock {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.to_value().serialize(s)
    }
}

impl<'de> Deserialize<'de> for ContentBlock {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Self::from_value(&Value::deserialize(d)?))
    }
}

// ---------------------------------------------------------------------------
// Tool calls
// ---------------------------------------------------------------------------

/// What a tool call shows.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolCallContent {
    Content { content: ContentBlock },
    Diff { path: String, old_text: Option<String>, new_text: String },
    Terminal { terminal_id: String },
    Unknown(Value),
}

impl ToolCallContent {
    pub fn from_value(v: &Value) -> Self {
        let parsed = (|| -> Option<Self> {
            match v.get("type")?.as_str()? {
                "content" => Some(Self::Content { content: ContentBlock::from_value(v.get("content")?) }),
                "diff" => Some(Self::Diff { path: str_of(v, "path")?, old_text: str_of(v, "oldText"), new_text: str_of(v, "newText")? }),
                "terminal" => Some(Self::Terminal { terminal_id: str_of(v, "terminalId")? }),
                _ => None,
            }
        })();
        parsed.unwrap_or_else(|| Self::Unknown(v.clone()))
    }

    pub fn to_value(&self) -> Value {
        match self {
            Self::Content { content } => json!({"type": "content", "content": content.to_value()}),
            Self::Diff { path, old_text, new_text } => {
                let mut m = json!({"type": "diff", "path": path, "newText": new_text});
                if let Some(o) = old_text {
                    m["oldText"] = json!(o);
                }
                m
            }
            Self::Terminal { terminal_id } => json!({"type": "terminal", "terminalId": terminal_id}),
            Self::Unknown(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolLocation {
    pub path: String,
    pub line: Option<u64>,
}

/// A new tool call (`session/update` `tool_call`).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub tool_call_id: String,
    pub title: String,
    pub name: Option<String>,
    pub kind: Option<ToolKind>,
    pub status: Option<ToolStatus>,
    pub content: Vec<ToolCallContent>,
    pub locations: Vec<ToolLocation>,
    pub raw_input: Option<Value>,
    pub raw_output: Option<Value>,
}

/// A change to a tool call (`tool_call_update`, and the `toolCall` of a
/// permission request). Only the fields that are present changed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolCallUpdate {
    pub tool_call_id: String,
    pub title: Option<String>,
    pub name: Option<String>,
    pub kind: Option<ToolKind>,
    pub status: Option<ToolStatus>,
    pub content: Option<Vec<ToolCallContent>>,
    pub locations: Option<Vec<ToolLocation>>,
    pub raw_input: Option<Value>,
    pub raw_output: Option<Value>,
}

fn parse_content_list(v: &Value) -> Option<Vec<ToolCallContent>> {
    v.get("content")?.as_array().map(|a| a.iter().map(ToolCallContent::from_value).collect())
}

fn parse_locations(v: &Value) -> Option<Vec<ToolLocation>> {
    v.get("locations")?.as_array().map(|a| {
        a.iter()
            .filter_map(|l| Some(ToolLocation { path: str_of(l, "path")?, line: l.get("line").and_then(Value::as_u64) }))
            .collect()
    })
}

fn non_null(v: &Value, key: &str) -> Option<Value> {
    v.get(key).filter(|x| !x.is_null()).cloned()
}

impl ToolCall {
    pub fn from_value(v: &Value) -> Option<Self> {
        Some(Self {
            tool_call_id: str_of(v, "toolCallId")?,
            title: str_of(v, "title").unwrap_or_default(),
            name: str_of(v, "name"),
            kind: str_of(v, "kind").map(|s| ToolKind::from(s.as_str())),
            status: str_of(v, "status").map(|s| ToolStatus::from(s.as_str())),
            content: parse_content_list(v).unwrap_or_default(),
            locations: parse_locations(v).unwrap_or_default(),
            raw_input: non_null(v, "rawInput"),
            raw_output: non_null(v, "rawOutput"),
        })
    }
}

impl ToolCallUpdate {
    pub fn from_value(v: &Value) -> Option<Self> {
        Some(Self {
            tool_call_id: str_of(v, "toolCallId")?,
            title: str_of(v, "title"),
            name: str_of(v, "name"),
            kind: str_of(v, "kind").map(|s| ToolKind::from(s.as_str())),
            status: str_of(v, "status").map(|s| ToolStatus::from(s.as_str())),
            content: parse_content_list(v),
            locations: parse_locations(v),
            raw_input: non_null(v, "rawInput"),
            raw_output: non_null(v, "rawOutput"),
        })
    }
}

// ---------------------------------------------------------------------------
// Plans, updates
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct PlanEntry {
    pub content: String,
    pub priority: PlanPriority,
    pub status: PlanStatus,
}

/// What an agent reports in `session/update`.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionUpdate {
    /// Replayed history during `session/load`.
    UserMessageChunk { content: ContentBlock, message_id: Option<String> },
    AgentMessageChunk { content: ContentBlock, message_id: Option<String> },
    AgentThoughtChunk { content: ContentBlock, message_id: Option<String> },
    ToolCall(ToolCall),
    ToolCallUpdate(ToolCallUpdate),
    Plan { entries: Vec<PlanEntry> },
    /// Anything else (`available_commands_update`, `current_mode_update`,
    /// `config_option_update`, `session_info_update`, `usage_update`,
    /// `notice`, ...) and any update that did not parse: the discriminator and
    /// the update object as received.
    Other { kind: String, raw: Value },
}

impl SessionUpdate {
    pub fn from_value(v: &Value) -> Self {
        let kind = str_of(v, "sessionUpdate").unwrap_or_default();
        let chunk = |v: &Value| -> Option<(ContentBlock, Option<String>)> {
            Some((ContentBlock::from_value(v.get("content")?), str_of(v, "messageId")))
        };
        let parsed = match kind.as_str() {
            "user_message_chunk" => chunk(v).map(|(content, message_id)| Self::UserMessageChunk { content, message_id }),
            "agent_message_chunk" => chunk(v).map(|(content, message_id)| Self::AgentMessageChunk { content, message_id }),
            "agent_thought_chunk" => chunk(v).map(|(content, message_id)| Self::AgentThoughtChunk { content, message_id }),
            "tool_call" => ToolCall::from_value(v).map(Self::ToolCall),
            "tool_call_update" => ToolCallUpdate::from_value(v).map(Self::ToolCallUpdate),
            "plan" => v.get("entries").and_then(Value::as_array).map(|a| Self::Plan {
                entries: a
                    .iter()
                    .filter_map(|e| {
                        Some(PlanEntry {
                            content: str_of(e, "content")?,
                            priority: PlanPriority::from(str_of(e, "priority")?.as_str()),
                            status: PlanStatus::from(str_of(e, "status")?.as_str()),
                        })
                    })
                    .collect(),
            }),
            _ => None,
        };
        parsed.unwrap_or(Self::Other { kind, raw: v.clone() })
    }
}

/// A `session/update` notification.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionNotification {
    pub session_id: String,
    pub update: SessionUpdate,
}

impl SessionNotification {
    pub fn from_params(params: &Value) -> Option<Self> {
        Some(Self { session_id: str_of(params, "sessionId")?, update: SessionUpdate::from_value(params.get("update")?) })
    }
}

// ---------------------------------------------------------------------------
// Permission
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionOption {
    pub option_id: String,
    pub name: String,
    pub kind: PermissionKind,
}

/// The decision on a permission request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionOutcome {
    Selected(String),
    Cancelled,
}

impl PermissionOutcome {
    /// The `result` of the `session/request_permission` response: the outcome is
    /// nested, `{"outcome": {"outcome": "selected", "optionId": ...}}`.
    pub fn to_result(&self) -> Value {
        match self {
            Self::Selected(id) => json!({"outcome": {"outcome": "selected", "optionId": id}}),
            Self::Cancelled => json!({"outcome": {"outcome": "cancelled"}}),
        }
    }
}

// ---------------------------------------------------------------------------
// Session setup
// ---------------------------------------------------------------------------

/// A stdio MCP server handed to the agent in `session/new` / `session/load`.
/// Every agent must support this transport; the entry has no `type` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServer {
    pub name: String,
    /// Absolute path of the executable.
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl McpServer {
    pub fn stdio(name: impl Into<String>, command: impl Into<String>, args: Vec<String>, env: Vec<(String, String)>) -> Self {
        Self { name: name.into(), command: command.into(), args, env }
    }

    pub fn to_value(&self) -> Value {
        let env: Vec<Value> = self.env.iter().map(|(k, v)| json!({"name": k, "value": v})).collect();
        json!({"name": self.name, "command": self.command, "args": self.args, "env": env})
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PromptCapabilities {
    pub image: bool,
    pub audio: bool,
    pub embedded_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpCapabilities {
    pub http: bool,
    pub sse: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentCapabilities {
    pub load_session: bool,
    pub prompt: PromptCapabilities,
    pub mcp: McpCapabilities,
    /// `agentCapabilities.sessionCapabilities` as received (resume, close, ...).
    pub session_capabilities: Value,
    /// The whole `agentCapabilities` object as received.
    pub raw: Value,
}

impl AgentCapabilities {
    pub fn from_value(v: &Value) -> Self {
        let flag = |o: &Value, k: &str| o.get(k).and_then(Value::as_bool).unwrap_or(false);
        let empty = Value::Object(Map::new());
        let prompt = v.get("promptCapabilities").unwrap_or(&empty);
        let mcp = v.get("mcpCapabilities").unwrap_or(&empty);
        Self {
            load_session: flag(v, "loadSession"),
            prompt: PromptCapabilities { image: flag(prompt, "image"), audio: flag(prompt, "audio"), embedded_context: flag(prompt, "embeddedContext") },
            mcp: McpCapabilities { http: flag(mcp, "http"), sse: flag(mcp, "sse") },
            session_capabilities: v.get("sessionCapabilities").cloned().unwrap_or(Value::Null),
            raw: v.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentInfo {
    pub name: String,
    pub title: Option<String>,
    pub version: Option<String>,
}

/// The result of `initialize`.
#[derive(Debug, Clone, PartialEq)]
pub struct InitializeResult {
    pub protocol_version: u64,
    pub capabilities: AgentCapabilities,
    pub agent_info: Option<AgentInfo>,
    /// `authMethods` as received.
    pub auth_methods: Vec<Value>,
}

/// The result of `session/new`.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSession {
    pub session_id: String,
    /// `modes` as received, when the agent sends them.
    pub modes: Option<Value>,
    /// `configOptions` as received (model, effort, mode, ...).
    pub config_options: Option<Value>,
}

/// The result of `session/load`; the history came as updates before it.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedSession {
    pub modes: Option<Value>,
    pub config_options: Option<Value>,
}
