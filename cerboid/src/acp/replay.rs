//! Recorded ACP logs. With [`super::ClientOptions::record`] the client appends
//! one JSON object per message to a file: `{"dir": "in" | "out", "msg": {...}}`
//! (`in` is agent to client). A log can be read back here, and the
//! `fake-acp-agent` binary replays one in the agent's place (env
//! `FAKE_ACP_REPLAY`), so the whole client is exercised from a recording
//! without a real agent.

use super::types::SessionNotification;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    /// Agent to client.
    In,
    /// Client to agent.
    Out,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub dir: Dir,
    pub msg: Value,
}

/// Parse a JSONL log. Blank lines and lines starting with `#` are skipped.
pub fn parse_log(text: &str) -> Result<Vec<LogEntry>, String> {
    let mut entries = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let v: Value = serde_json::from_str(line).map_err(|e| format!("log line {}: {e}", i + 1))?;
        let dir = match v.get("dir").and_then(Value::as_str) {
            Some("in") => Dir::In,
            Some("out") => Dir::Out,
            _ => return Err(format!("log line {}: `dir` must be \"in\" or \"out\"", i + 1)),
        };
        let msg = v.get("msg").cloned().ok_or_else(|| format!("log line {}: no `msg`", i + 1))?;
        entries.push(LogEntry { dir, msg });
    }
    Ok(entries)
}

/// The `session/update`s in a log, mapped the way the client maps them.
pub fn updates(entries: &[LogEntry]) -> Vec<SessionNotification> {
    entries
        .iter()
        .filter(|e| e.dir == Dir::In && e.msg.get("method").and_then(Value::as_str) == Some("session/update") && e.msg.get("id").is_none())
        .filter_map(|e| SessionNotification::from_params(e.msg.get("params")?))
        .collect()
}
