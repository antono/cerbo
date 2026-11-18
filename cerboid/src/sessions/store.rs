//! The session store (design D4): one append-only JSONL feed per session and a
//! small metadata file beside it, per vault, outside the vault.
//!
//! ```text
//! $XDG_DATA_HOME/cerbo/sessions/<vault-id>/<session>.jsonl       the feed, one SessionEvent per line
//! $XDG_DATA_HOME/cerbo/sessions/<vault-id>/<session>.meta.json   the index entry
//! ```
//!
//! Directories are mode 0700 and files 0600: the feeds hold the text of the
//! user's notes. Listing reads the index entries, newest first. Session ids
//! are UUIDs and are checked before they become part of a path.

use cerbo_core::rpc::{AgentKind, SessionEvent, SessionEventBody, SessionInfo, SessionState};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// The index entry of one session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    pub id: String,
    /// Vault id.
    pub vault: String,
    pub agent: AgentKind,
    pub title: String,
    pub state: SessionState,
    pub created_at: String,
    pub updated_at: String,
    pub resumable: bool,
    pub sandboxed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    /// The agent's own id of the session, for `session/load`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_session_id: Option<String>,
}

impl Meta {
    pub fn info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            vault: self.vault.clone(),
            agent: self.agent,
            title: self.title.clone(),
            state: self.state,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            resumable: self.resumable,
            sandboxed: self.sandboxed,
            warning: self.warning.clone(),
        }
    }
}

/// The current time, RFC 3339.
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Whether `id` is a session id (a UUID), which makes it safe in a path.
pub fn valid_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|u| u.hyphenated().to_string() == id)
}

/// The most a session's feed file may hold (events written by the agent's
/// activity stop at this size; see [`LogWriter::append`]).
pub const FEED_CAP_BYTES: u64 = 64 * 1024 * 1024;
/// Room above the cap kept for the events that end a session (state, error,
/// end of turn), so a full feed can still say why it ended.
pub const FEED_RESERVE_BYTES: u64 = 1024 * 1024;
/// How much of the end of a feed is read to find the last `seq` and to build
/// the transcript of a resume. An event is at most a few hundred KiB (chunks
/// are cut per event), so this holds the last events whole.
pub const TAIL_BYTES: u64 = 2 * 1024 * 1024;

fn err(path: &Path, e: impl std::fmt::Display) -> String {
    format!("{}: {e}", path.display())
}

/// Create `dir` and its missing parents with mode 0700, and tighten `dir`
/// itself to it.
fn private_dir(dir: &Path) -> Result<(), String> {
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir).map_err(|e| err(dir, e))?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(|e| err(dir, e))
}

#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
    feed_cap: u64,
}

impl Store {
    pub fn new(root: PathBuf) -> Self {
        Self { root, feed_cap: FEED_CAP_BYTES }
    }

    /// A store whose feeds stop at `feed_cap` bytes (tests use small ones).
    pub fn with_feed_cap(root: PathBuf, feed_cap: u64) -> Self {
        Self { root, feed_cap }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The size limit of a feed, in bytes.
    pub fn feed_cap(&self) -> u64 {
        self.feed_cap
    }

    fn vault_dir(&self, vault: &str) -> PathBuf {
        self.root.join(vault)
    }

    fn meta_path(&self, vault: &str, id: &str) -> PathBuf {
        self.vault_dir(vault).join(format!("{id}.meta.json"))
    }

    fn log_path(&self, vault: &str, id: &str) -> PathBuf {
        self.vault_dir(vault).join(format!("{id}.jsonl"))
    }

    /// Write (replace) the index entry of a session.
    pub fn write_meta(&self, meta: &Meta) -> Result<(), String> {
        if !valid_id(&meta.id) || meta.vault.is_empty() || meta.vault.contains(['/', '\\']) || meta.vault.starts_with('.') {
            return Err(format!("not a valid session or vault id: {}/{}", meta.vault, meta.id));
        }
        private_dir(&self.root)?;
        private_dir(&self.vault_dir(&meta.vault))?;
        let path = self.meta_path(&meta.vault, &meta.id);
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string(meta).map_err(|e| e.to_string())?;
        let mut file = OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp).map_err(|e| err(&tmp, e))?;
        file.write_all(text.as_bytes()).map_err(|e| err(&tmp, e))?;
        drop(file);
        std::fs::rename(&tmp, &path).map_err(|e| err(&path, e))
    }

    fn read_meta_file(path: &Path) -> Option<Meta> {
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    /// The index entry of session `id`, wherever it is stored.
    pub fn find(&self, id: &str) -> Option<Meta> {
        if !valid_id(id) {
            return None;
        }
        for entry in std::fs::read_dir(&self.root).ok()?.flatten() {
            let path = entry.path().join(format!("{id}.meta.json"));
            if let Some(meta) = Self::read_meta_file(&path) {
                return Some(meta);
            }
        }
        None
    }

    /// The sessions of `vault` (all vaults without), newest first.
    pub fn list(&self, vault: Option<&str>) -> Vec<Meta> {
        let dirs: Vec<PathBuf> = match vault {
            Some(v) if v.contains(['/', '\\']) || v.starts_with('.') => return Vec::new(),
            Some(v) => vec![self.vault_dir(v)],
            None => std::fs::read_dir(&self.root).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default(),
        };
        let mut all: Vec<Meta> = Vec::new();
        for dir in dirs {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for entry in rd.flatten() {
                let path = entry.path();
                if path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(".meta.json"))
                    && let Some(meta) = Self::read_meta_file(&path)
                {
                    all.push(meta);
                }
            }
        }
        all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then_with(|| b.created_at.cmp(&a.created_at)).then_with(|| a.id.cmp(&b.id)));
        all
    }

    /// Remove a session's feed and index entry.
    pub fn delete(&self, vault: &str, id: &str) -> Result<(), String> {
        if !valid_id(id) || vault.contains(['/', '\\']) {
            return Err("not a valid session id".into());
        }
        for path in [self.log_path(vault, id), self.meta_path(vault, id)] {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(err(&path, e)),
            }
        }
        Ok(())
    }

    /// Open (create) the feed of a session for appending.
    pub fn open_log(&self, vault: &str, id: &str) -> Result<LogWriter, String> {
        if !valid_id(id) || vault.contains(['/', '\\']) {
            return Err("not a valid session id".into());
        }
        private_dir(&self.root)?;
        private_dir(&self.vault_dir(vault))?;
        let path = self.log_path(vault, id);
        let next_seq = last_seq(&path) + 1;
        let file = OpenOptions::new().create(true).append(true).mode(0o600).open(&path).map_err(|e| err(&path, e))?;
        let size = file.metadata().map_err(|e| err(&path, e))?.len();
        Ok(LogWriter { file, next_seq, path, size, cap: self.feed_cap })
    }

    /// The last events of a feed: those in its final `max_bytes` bytes (the
    /// first, possibly cut, line is dropped), oldest first. A feed of any size
    /// costs at most `max_bytes` to read.
    pub fn read_tail_events(&self, vault: &str, id: &str, max_bytes: u64) -> Vec<SessionEvent> {
        if !valid_id(id) || vault.contains(['/', '\\']) {
            return Vec::new();
        }
        tail_events(&self.log_path(vault, id), max_bytes)
    }

    /// The events of a session with `seq` above `after`, at most `limit`.
    pub fn read_events(&self, vault: &str, id: &str, after: Option<u64>, limit: usize) -> Vec<SessionEvent> {
        if !valid_id(id) || vault.contains(['/', '\\']) {
            return Vec::new();
        }
        let Ok(file) = File::open(self.log_path(vault, id)) else { return Vec::new() };
        BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str::<SessionEvent>(&line).ok())
            .filter(|e| after.is_none_or(|a| e.seq > a))
            .take(limit)
            .collect()
    }
}

/// The events in the last `max_bytes` bytes of the file at `path`.
fn tail_events(path: &Path, max_bytes: u64) -> Vec<SessionEvent> {
    let Ok(mut file) = File::open(path) else { return Vec::new() };
    let Ok(len) = file.metadata().map(|m| m.len()) else { return Vec::new() };
    let start = len.saturating_sub(max_bytes);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    if file.take(max_bytes).read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    let mut lines = text.lines();
    if start > 0 {
        // Where the read began is likely the middle of a line.
        lines.next();
    }
    lines.filter_map(|line| serde_json::from_str::<SessionEvent>(line).ok()).collect()
}

/// The highest `seq` in a feed; 0 for none. Only the end of the file is read.
fn last_seq(path: &Path) -> u64 {
    tail_events(path, TAIL_BYTES).iter().map(|e| e.seq).max().unwrap_or(0)
}

/// Why an event could not be appended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppendError {
    /// The feed reached its size limit.
    Full,
    Io(String),
}

impl std::fmt::Display for AppendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "the session feed reached its size limit"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

/// Appends events to one feed, numbering them.
pub struct LogWriter {
    file: File,
    next_seq: u64,
    path: PathBuf,
    /// Bytes in the file.
    size: u64,
    cap: u64,
}

/// The events that end things stay writable in the reserve above the cap.
fn is_closing(body: &SessionEventBody) -> bool {
    matches!(body, SessionEventBody::State { .. } | SessionEventBody::Error { .. } | SessionEventBody::TurnEnded { .. })
}

impl LogWriter {
    /// Append `body` as the next event. The line is written whole and at once,
    /// so a reader of the file never sees half an event. An event that would
    /// take the file over its cap is refused with [`AppendError::Full`], except
    /// state, error and end-of-turn events, which may use the reserve above it.
    pub fn append(&mut self, body: SessionEventBody) -> Result<SessionEvent, AppendError> {
        let event = SessionEvent { seq: self.next_seq, at: now(), body };
        let mut line = serde_json::to_vec(&event).map_err(|e| AppendError::Io(e.to_string()))?;
        line.push(b'\n');
        let limit = if is_closing(&event.body) { self.cap.saturating_add(FEED_RESERVE_BYTES) } else { self.cap };
        if self.size.saturating_add(line.len() as u64) > limit {
            return Err(AppendError::Full);
        }
        self.file.write_all(&line).map_err(|e| AppendError::Io(err(&self.path, e)))?;
        self.size += line.len() as u64;
        self.next_seq += 1;
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V: &str = "00000000-0000-4000-8000-000000000137";

    fn meta(id: &str, updated: &str) -> Meta {
        Meta {
            id: id.into(),
            vault: V.into(),
            agent: AgentKind::Opencode,
            title: "t".into(),
            state: SessionState::Idle,
            created_at: "2026-01-01T00:00:00.000Z".into(),
            updated_at: updated.into(),
            resumable: false,
            sandboxed: true,
            warning: None,
            acp_session_id: None,
        }
    }

    fn id(n: u8) -> String {
        format!("00000000-0000-4000-8000-0000000000{n:02}")
    }

    #[test]
    fn the_feed_is_numbered_private_and_replayable() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path().join("sessions"));
        let sid = id(1);
        let mut log = store.open_log(V, &sid).unwrap();
        for n in 0..3 {
            let event = log.append(SessionEventBody::AgentText { text: format!("chunk {n}") }).unwrap();
            assert_eq!(event.seq, n + 1);
        }
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&tmp.path().join("sessions")), 0o700);
        assert_eq!(mode(&tmp.path().join("sessions").join(V)), 0o700);
        assert_eq!(mode(&tmp.path().join("sessions").join(V).join(format!("{sid}.jsonl"))), 0o600);

        assert_eq!(store.read_events(V, &sid, None, 100).len(), 3);
        let tail = store.read_events(V, &sid, Some(1), 100);
        assert_eq!(tail.iter().map(|e| e.seq).collect::<Vec<_>>(), [2, 3]);
        assert_eq!(store.read_events(V, &sid, None, 2).len(), 2);

        // Reopening continues the numbering.
        drop(log);
        let mut log = store.open_log(V, &sid).unwrap();
        assert_eq!(log.append(SessionEventBody::AgentText { text: "more".into() }).unwrap().seq, 4);
    }

    #[test]
    fn sessions_are_listed_newest_first_per_vault_and_can_be_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path().join("sessions"));
        store.write_meta(&meta(&id(1), "2026-01-01T00:00:01.000Z")).unwrap();
        store.write_meta(&meta(&id(2), "2026-01-01T00:00:03.000Z")).unwrap();
        store.write_meta(&meta(&id(3), "2026-01-01T00:00:02.000Z")).unwrap();
        let ids: Vec<String> = store.list(Some(V)).into_iter().map(|m| m.id).collect();
        assert_eq!(ids, [id(2), id(3), id(1)]);
        assert!(store.list(Some("other")).is_empty());
        assert_eq!(store.list(None).len(), 3);
        assert_eq!(store.find(&id(3)).unwrap().id, id(3));
        assert!(store.find("../../etc/passwd").is_none());

        store.open_log(V, &id(2)).unwrap().append(SessionEventBody::AgentText { text: "x".into() }).unwrap();
        store.delete(V, &id(2)).unwrap();
        assert!(store.find(&id(2)).is_none());
        assert!(store.read_events(V, &id(2), None, 10).is_empty());
        assert_eq!(store.list(Some(V)).len(), 2);
    }

    #[test]
    fn a_feed_stops_at_its_cap_but_keeps_room_to_say_why() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::with_feed_cap(tmp.path().join("sessions"), 1000);
        let sid = id(1);
        let mut log = store.open_log(V, &sid).unwrap();
        let mut written = 0;
        let refused = loop {
            match log.append(SessionEventBody::AgentText { text: "x".repeat(100) }) {
                Ok(_) => written += 1,
                Err(e) => break e,
            }
        };
        assert_eq!(refused, AppendError::Full);
        assert!(written > 3 && written < 10, "{written}");
        // The events that end things still fit, in the reserve above the cap.
        log.append(SessionEventBody::Error { message: "the feed is full".into() }).unwrap();
        log.append(SessionEventBody::State { state: SessionState::Failed, reason: Some("full".into()), stderr_tail: None }).unwrap();
        // The size counts what an earlier writer left in the file.
        drop(log);
        let mut log = store.open_log(V, &sid).unwrap();
        assert_eq!(log.append(SessionEventBody::AgentText { text: "more".into() }), Err(AppendError::Full));
    }

    #[test]
    fn the_tail_of_a_big_feed_is_read_without_reading_all_of_it() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path().join("sessions"));
        let sid = id(1);
        let mut log = store.open_log(V, &sid).unwrap();
        for n in 0..200 {
            log.append(SessionEventBody::AgentText { text: format!("chunk {n} {}", "y".repeat(200)) }).unwrap();
        }
        let tail = store.read_tail_events(V, &sid, 3000);
        assert!(!tail.is_empty() && tail.len() < 20, "{}", tail.len());
        assert_eq!(tail.last().unwrap().seq, 200, "the end is there");
        assert!(tail.windows(2).all(|w| w[1].seq == w[0].seq + 1), "whole events, in order");
        // A bound above the file's size gives everything.
        assert_eq!(store.read_tail_events(V, &sid, u64::MAX / 2).len(), 200);
        // Numbering continues from the end even when only the tail is looked at.
        drop(log);
        assert_eq!(store.open_log(V, &sid).unwrap().append(SessionEventBody::AgentText { text: "next".into() }).unwrap().seq, 201);
    }

    #[test]
    fn ids_that_are_not_uuids_never_reach_a_path() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path().join("sessions"));
        assert!(!valid_id("../x"));
        assert!(store.open_log(V, "../x").is_err());
        assert!(store.delete(V, "a/b").is_err());
        assert!(store.read_events("../..", &id(1), None, 1).is_empty());
        assert!(store.list(Some("../..")).is_empty());
    }
}
