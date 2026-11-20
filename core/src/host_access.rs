//! What an agent may do with files outside its vault (host access).
//!
//! Host access is a per-vault setting kept in `agents.toml` (see
//! [`crate::agent`]): `off` (the default) or `on`, and a list of allowed paths,
//! each `read` or `write`. With it `on`, a path covered by the list is open
//! within its mode; any other path needs a person's approval for each action.
//! A few places are never reachable, whatever the list says or a person
//! approves.
//!
//! Everything here is a pure decision plus the file operations themselves; the
//! daemon owns approvals and logging. A path is judged after `..` and symbolic
//! links are resolved, so a link inside an allowed folder cannot lead out of it.

use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

/// Bytes one read returns at most when the caller does not ask for less.
pub const DEFAULT_READ_LIMIT: usize = 137 * 1024;
/// The most entries one directory listing returns.
pub const MAX_LIST_ENTRIES: usize = 1000;

/// One allowed path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostPath {
    /// Absolute, resolved at the time it was added.
    pub path: PathBuf,
    /// `write` includes `read`.
    #[serde(default)]
    pub write: bool,
}

/// The host access setting of one vault.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub paths: Vec<HostPath>,
}

/// What the agent wants to do with a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    List,
    Read,
    Write,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Action::List => "list",
            Action::Read => "read",
            Action::Write => "write",
        }
    }
}

/// The answer for one call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Covered by the list: do it.
    Allow,
    /// Not covered: a person must approve this one action.
    Ask,
    /// Never: the text says why.
    Refuse(String),
}

/// What the decision needs to know about the machine.
#[derive(Debug, Default, Clone)]
pub struct Guard {
    /// The session's own vault. Its files are changed through the vault
    /// tools, never through host tools.
    pub vault_root: PathBuf,
    /// The `.cerbo` directory of the other vaults is found from these roots.
    pub other_vaults: Vec<PathBuf>,
    /// Private places of the daemon and the agents.
    pub protected: Vec<PathBuf>,
    /// The user's home, for the credential stores below it.
    pub home: Option<PathBuf>,
}

/// Credential stores below the home folder.
const CREDENTIAL_DIRS: [&str; 3] = [".ssh", ".gnupg", ".aws"];

/// Best effort: the real location of `path`, or `path` itself when it does not
/// exist (a protected place that is not there yet still protects).
fn real_or_self(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Resolve `path` to the real location the action would touch.
///
/// The path must be absolute. An existing path is canonicalized. For a write
/// to a file that does not exist yet, the parent is canonicalized and the file
/// name appended; the parent must exist.
pub fn resolve(path: &str, action: Action) -> Result<PathBuf, String> {
    let given = Path::new(path);
    if !given.is_absolute() {
        return Err(format!("{path}: the path must be absolute"));
    }
    match std::fs::canonicalize(given) {
        Ok(real) => Ok(real),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && action == Action::Write => {
            let name = match given.components().next_back() {
                Some(Component::Normal(name)) => name.to_owned(),
                _ => return Err(format!("{path}: not a file path")),
            };
            let parent = given.parent().ok_or_else(|| format!("{path}: not a file path"))?;
            let parent = std::fs::canonicalize(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            Ok(parent.join(name))
        }
        Err(e) => Err(format!("{path}: {e}")),
    }
}

/// Why `real` can never be reached, if it can not.
pub fn never_reachable(real: &Path, guard: &Guard) -> Option<String> {
    for protected in &guard.protected {
        let protected = real_or_self(protected);
        if real.starts_with(&protected) || protected.starts_with(real) {
            return Some(format!("{} is a private place of Cerbo and its agents", protected.display()));
        }
    }
    for root in &guard.other_vaults {
        let cerbo = real_or_self(&root.join(".cerbo"));
        if real.starts_with(&cerbo) {
            return Some(format!("{} belongs to another vault", cerbo.display()));
        }
    }
    if let Some(home) = &guard.home {
        for dir in CREDENTIAL_DIRS {
            let store = real_or_self(&home.join(dir));
            if real.starts_with(&store) {
                return Some(format!("{} holds credentials", store.display()));
            }
        }
    }
    None
}

/// Judge one action on an already resolved path.
pub fn decide(settings: &HostSettings, guard: &Guard, real: &Path, action: Action) -> Decision {
    if let Some(reason) = never_reachable(real, guard) {
        return Decision::Refuse(reason);
    }
    if !guard.vault_root.as_os_str().is_empty() && real.starts_with(real_or_self(&guard.vault_root)) {
        return Decision::Refuse("this path is inside the vault: use the vault tools for it".into());
    }
    if !settings.enabled {
        return Decision::Refuse("host access is off for this vault".into());
    }
    let covering: Vec<&HostPath> = settings.paths.iter().filter(|p| real.starts_with(&p.path)).collect();
    match action {
        Action::List | Action::Read if !covering.is_empty() => Decision::Allow,
        Action::Write if covering.iter().any(|p| p.write) => Decision::Allow,
        Action::Write if !covering.is_empty() => {
            Decision::Refuse("this path is allowed for reading only; the user can allow writing".into())
        }
        _ => Decision::Ask,
    }
}

/// An entry accepted for the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    pub path: PathBuf,
    /// Set when the entry is very wide.
    pub warning: Option<String>,
}

/// Check a path the user wants to allow: absolute, existing, resolved, and not
/// one of the never-reachable places.
pub fn accept_entry(path: &str, guard: &Guard) -> Result<Accepted, String> {
    let real = resolve(path, Action::Read)?;
    if let Some(reason) = never_reachable(&real, guard) {
        return Err(format!("cannot allow {}: {reason}", real.display()));
    }
    let wide = real == Path::new("/") || guard.home.as_ref().is_some_and(|h| h.starts_with(&real));
    let warning = wide.then(|| {
        format!("{} covers your whole home folder or more; the agent can reach everything below it", real.display())
    });
    Ok(Accepted { path: real, warning })
}

// ── The file operations ──────────────────────────────────────────────────────

/// Open `real` for reading and check that it is still the file that was judged:
/// a link swapped in after the check would show up as a different location.
fn open_checked(real: &Path) -> Result<std::fs::File, String> {
    let file = std::fs::File::open(real).map_err(|e| format!("{}: {e}", real.display()))?;
    let now = std::fs::read_link(format!("/proc/self/fd/{}", std::os::fd::AsRawFd::as_raw_fd(&file)))
        .map_err(|e| format!("{}: {e}", real.display()))?;
    if now != real {
        return Err(format!("{}: the file changed while it was opened; try again", real.display()));
    }
    Ok(file)
}

/// The entries of a directory: `{name, kind, size}`, sorted by name.
pub fn list_dir(real: &Path) -> Result<serde_json::Value, String> {
    let read = std::fs::read_dir(real).map_err(|e| format!("{}: {e}", real.display()))?;
    let mut entries: Vec<(String, &'static str, u64)> = Vec::new();
    for entry in read.flatten() {
        let meta = entry.metadata().ok();
        let link = entry.file_type().map(|t| t.is_symlink()).unwrap_or(false);
        let kind = if link {
            "link"
        } else if meta.as_ref().is_some_and(|m| m.is_dir()) {
            "dir"
        } else {
            "file"
        };
        entries.push((entry.file_name().to_string_lossy().into_owned(), kind, meta.map_or(0, |m| m.size())));
    }
    entries.sort();
    let truncated = entries.len() > MAX_LIST_ENTRIES;
    entries.truncate(MAX_LIST_ENTRIES);
    Ok(serde_json::json!({
        "path": real,
        "entries": entries.iter().map(|(name, kind, size)| serde_json::json!({"name": name, "kind": kind, "size": size})).collect::<Vec<_>>(),
        "truncated": truncated,
    }))
}

/// A chunk of a UTF-8 text file from `offset`: `{path, content, size, offset,
/// next_offset}`. `next_offset` is null at the end of the file.
pub fn read_file(real: &Path, offset: u64, limit: Option<usize>) -> Result<serde_json::Value, String> {
    let limit = limit.filter(|l| *l > 0).map_or(DEFAULT_READ_LIMIT, |l| l.min(DEFAULT_READ_LIMIT));
    let mut file = open_checked(real)?;
    let meta = file.metadata().map_err(|e| format!("{}: {e}", real.display()))?;
    if !meta.is_file() {
        return Err(format!("{}: not a regular file", real.display()));
    }
    let size = meta.len();
    file.seek(SeekFrom::Start(offset)).map_err(|e| format!("{}: {e}", real.display()))?;
    let mut buf = Vec::new();
    file.take(limit as u64).read_to_end(&mut buf).map_err(|e| format!("{}: {e}", real.display()))?;
    let end = offset + buf.len() as u64;
    // A chunk may stop inside a character: give back the incomplete tail.
    let text = match std::str::from_utf8(&buf) {
        Ok(text) => text.to_string(),
        Err(e) if e.error_len().is_none() && end < size => String::from_utf8_lossy(&buf[..e.valid_up_to()]).into_owned(),
        Err(_) => return Err(format!("{}: not valid UTF-8 text; only text files can be read", real.display())),
    };
    let next = offset + text.len() as u64;
    Ok(serde_json::json!({
        "path": real,
        "content": text,
        "size": size,
        "offset": offset,
        "next_offset": if next < size { serde_json::Value::from(next) } else { serde_json::Value::Null },
    }))
}

/// The size of the file at `real`, if it exists (for the approval card).
pub fn existing_size(real: &Path) -> Option<u64> {
    std::fs::metadata(real).ok().filter(|m| m.is_file()).map(|m| m.len())
}

/// Write `content` to `real` (create or replace, through a temporary file).
pub fn write_file(real: &Path, content: &str) -> Result<serde_json::Value, String> {
    if real.is_dir() {
        return Err(format!("{}: is a directory", real.display()));
    }
    let created = !real.exists();
    crate::fsio::write_atomic_str(real, content).map_err(|e| format!("{}: {e}", real.display()))?;
    Ok(serde_json::json!({"path": real, "bytes": content.len(), "created": created}))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        _tmp: tempfile::TempDir,
        root: PathBuf,
        guard: Guard,
    }

    fn fixture() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        for dir in ["vault", "inbox", "out", "other/.cerbo", "home/.ssh", "private", "elsewhere"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        let guard = Guard {
            vault_root: root.join("vault"),
            other_vaults: vec![root.join("other")],
            protected: vec![root.join("private")],
            home: Some(root.join("home")),
        };
        Fixture { _tmp: tmp, root, guard }
    }

    fn settings(f: &Fixture) -> HostSettings {
        HostSettings {
            enabled: true,
            paths: vec![
                HostPath { path: f.root.join("inbox"), write: false },
                HostPath { path: f.root.join("out"), write: true },
            ],
        }
    }

    fn decide_on(f: &Fixture, s: &HostSettings, rel: &str, action: Action) -> Decision {
        decide(s, &f.guard, &f.root.join(rel), action)
    }

    #[test]
    fn host_access_is_off_by_default() {
        let f = fixture();
        let d = decide(&HostSettings::default(), &f.guard, &f.root.join("inbox"), Action::Read);
        assert!(matches!(d, Decision::Refuse(_)));
    }

    #[test]
    fn a_listed_path_is_open_within_its_mode() {
        let f = fixture();
        let s = settings(&f);
        assert_eq!(decide_on(&f, &s, "inbox/a.txt", Action::Read), Decision::Allow);
        assert_eq!(decide_on(&f, &s, "inbox", Action::List), Decision::Allow);
        assert_eq!(decide_on(&f, &s, "out/a.txt", Action::Write), Decision::Allow);
        assert_eq!(decide_on(&f, &s, "out/a.txt", Action::Read), Decision::Allow);
    }

    #[test]
    fn writing_under_a_read_path_fails_and_does_not_ask() {
        let f = fixture();
        let d = decide_on(&f, &settings(&f), "inbox/a.txt", Action::Write);
        assert!(matches!(d, Decision::Refuse(_)), "{d:?}");
    }

    #[test]
    fn anything_not_listed_asks() {
        let f = fixture();
        let s = settings(&f);
        for action in [Action::List, Action::Read, Action::Write] {
            assert_eq!(decide_on(&f, &s, "elsewhere/x", action), Decision::Ask);
        }
    }

    #[test]
    fn credentials_private_places_and_other_vaults_are_never_reachable() {
        let f = fixture();
        let mut s = settings(&f);
        // Even a list entry that covers them does not open them.
        s.paths.push(HostPath { path: f.root.clone(), write: true });
        for rel in ["home/.ssh/id_ed25519", "private/agents/x", "other/.cerbo/objects"] {
            let d = decide_on(&f, &s, rel, Action::Read);
            assert!(matches!(d, Decision::Refuse(_)), "{rel}: {d:?}");
        }
    }

    #[test]
    fn a_parent_of_a_private_place_is_refused_too() {
        let f = fixture();
        let d = decide_on(&f, &settings(&f), "", Action::List);
        assert!(matches!(d, Decision::Refuse(_)));
    }

    #[test]
    fn the_vault_is_reached_through_vault_tools_only() {
        let f = fixture();
        let mut s = settings(&f);
        s.paths.push(HostPath { path: f.root.clone(), write: true });
        let d = decide_on(&f, &s, "vault/page.md", Action::Write);
        assert!(matches!(&d, Decision::Refuse(m) if m.contains("vault tools")), "{d:?}");
    }

    #[test]
    fn a_link_out_of_an_allowed_path_is_judged_by_where_it_leads() {
        let f = fixture();
        std::os::unix::fs::symlink(f.root.join("home/.ssh"), f.root.join("inbox/ssh")).unwrap();
        std::fs::write(f.root.join("home/.ssh/id"), "key").unwrap();
        let real = resolve(&f.root.join("inbox/ssh/id").to_string_lossy(), Action::Read).unwrap();
        assert!(matches!(decide(&settings(&f), &f.guard, &real, Action::Read), Decision::Refuse(_)));

        std::os::unix::fs::symlink(f.root.join("elsewhere"), f.root.join("inbox/out-link")).unwrap();
        let real = resolve(&f.root.join("inbox/out-link").to_string_lossy(), Action::Read).unwrap();
        assert_eq!(decide(&settings(&f), &f.guard, &real, Action::Read), Decision::Ask);
    }

    #[test]
    fn a_relative_path_is_refused() {
        assert!(resolve("notes.txt", Action::Read).is_err());
        assert!(resolve("../x", Action::Write).is_err());
    }

    #[test]
    fn dotdot_cannot_leave_an_allowed_path() {
        let f = fixture();
        let sneaky = format!("{}/inbox/../elsewhere", f.root.display());
        let real = resolve(&sneaky, Action::Read).unwrap();
        assert_eq!(decide(&settings(&f), &f.guard, &real, Action::Read), Decision::Ask);
    }

    #[test]
    fn a_new_file_resolves_through_its_parent() {
        let f = fixture();
        let real = resolve(&f.root.join("out/new.txt").to_string_lossy(), Action::Write).unwrap();
        assert_eq!(real, f.root.join("out/new.txt"));
        assert!(resolve(&f.root.join("missing/new.txt").to_string_lossy(), Action::Write).is_err());
    }

    #[test]
    fn the_list_refuses_credentials_and_warns_about_a_wide_entry() {
        let f = fixture();
        assert!(accept_entry(&f.root.join("home/.ssh").to_string_lossy(), &f.guard).is_err());
        let ok = accept_entry(&f.root.join("inbox").to_string_lossy(), &f.guard).unwrap();
        assert!(ok.warning.is_none());
        // The home folder itself is allowed, with a warning; `.ssh` below it stays closed.
        let wide = accept_entry(&f.root.join("home").to_string_lossy(), &f.guard).unwrap();
        assert!(wide.warning.is_some());
    }

    #[test]
    fn reading_gives_chunks_that_end_on_a_character() {
        let f = fixture();
        let file = f.root.join("inbox/t.txt");
        std::fs::write(&file, "aé€z").unwrap(); // 1 + 2 + 3 + 1 bytes
        let first = read_file(&file, 0, Some(2)).unwrap();
        assert_eq!(first["content"], "a");
        assert_eq!(first["next_offset"], 1);
        let second = read_file(&file, 1, Some(5)).unwrap();
        assert_eq!(second["content"], "é€");
        let last = read_file(&file, 6, None).unwrap();
        assert_eq!(last["content"], "z");
        assert!(last["next_offset"].is_null());
    }

    #[test]
    fn binary_files_are_not_text() {
        let f = fixture();
        let file = f.root.join("inbox/b.bin");
        std::fs::write(&file, [0xff, 0xfe, 0x00]).unwrap();
        assert!(read_file(&file, 0, None).is_err());
    }

    #[test]
    fn writing_creates_and_replaces() {
        let f = fixture();
        let file = f.root.join("out/w.txt");
        assert_eq!(write_file(&file, "one").unwrap()["created"], true);
        assert_eq!(write_file(&file, "two").unwrap()["created"], false);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "two");
    }

    #[test]
    fn listing_is_sorted_and_names_kinds() {
        let f = fixture();
        std::fs::write(f.root.join("inbox/b"), "x").unwrap();
        std::fs::create_dir(f.root.join("inbox/a")).unwrap();
        let list = list_dir(&f.root.join("inbox")).unwrap();
        assert_eq!(list["entries"][0]["name"], "a");
        assert_eq!(list["entries"][0]["kind"], "dir");
        assert_eq!(list["entries"][1]["kind"], "file");
    }
}
