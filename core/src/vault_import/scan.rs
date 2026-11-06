//! Source detection and the walk over the source folder. Both only read.

use super::dialect::Dialect;
use super::{Format, frontmatter};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// The message for a folder that is neither format, as the spec words it.
pub const UNDETECTABLE: &str = "cannot detect vault format, pass --from obsidian|tolaria";

/// Which format a source is read as, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub format: Format,
    pub reason: String,
}

/// Decide the source format. `from` (the `--from` flag) always wins, except
/// that a Cerbo vault is never a source.
pub fn detect(source: &Path, from: Option<Format>) -> Result<Detection, String> {
    if !source.is_dir() {
        return Err(format!("Source is not a folder: {}", source.display()));
    }
    if source.join(".cerbo").exists() {
        return Err(format!(
            "{} is a Cerbo vault; import-vault reads Obsidian or Tolaria vaults",
            source.display()
        ));
    }
    if let Some(format) = from {
        return Ok(Detection {
            format,
            reason: "--from".to_string(),
        });
    }
    if source.join(".obsidian").is_dir() {
        return Ok(Detection {
            format: Format::Obsidian,
            reason: ".obsidian/ folder".to_string(),
        });
    }
    if has_yml_views(&source.join("views")) {
        return Ok(Detection {
            format: Format::Tolaria,
            reason: "views/*.yml".to_string(),
        });
    }
    if let Some(note) = note_with_type_field(source) {
        return Ok(Detection {
            format: Format::Tolaria,
            reason: format!("type: frontmatter in {note}"),
        });
    }
    Err(UNDETECTABLE.to_string())
}

fn has_yml_views(views: &Path) -> bool {
    std::fs::read_dir(views)
        .map(|entries| {
            entries.flatten().any(|e| {
                let name = e.file_name().to_string_lossy().to_lowercase();
                e.path().is_file() && (name.ends_with(".yml") || name.ends_with(".yaml"))
            })
        })
        .unwrap_or(false)
}

/// The first note (relative path) whose frontmatter has a `type:` field.
fn note_with_type_field(source: &Path) -> Option<String> {
    visible_entries(source)
        .filter(|e| e.file_type().is_file() && is_markdown(e.path()))
        .find_map(|e| {
            let text = std::fs::read_to_string(e.path()).ok()?;
            let (yaml, _) = frontmatter::split(&text);
            let has_type = yaml?.lines().any(|l| l.starts_with("type:"));
            has_type.then(|| rel_string(source, e.path()))
        })
}

/// One file of the source folder that is imported or may be.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Path relative to the source root, `/`-separated.
    pub rel: String,
    pub abs: PathBuf,
    pub is_note: bool,
    pub size: u64,
    /// RFC 3339: birth time where the file system has one, else `modified`.
    pub created: String,
    pub modified: String,
}

#[derive(Debug, Default)]
pub struct Scan {
    /// Notes and other files, sorted by relative path.
    pub files: Vec<SourceFile>,
    /// Non-hidden files left out by the format's rules, sorted.
    pub skipped: Vec<String>,
}

/// Walk `source`. Hidden files and folders (`.obsidian/`, `.git/`, `.trash/`,
/// dotfiles) are left out silently; files the dialect skips, symbolic links
/// and names that are not UTF-8 are listed as skipped.
pub fn scan(source: &Path, dialect: &dyn Dialect) -> Result<Scan, String> {
    let mut out = Scan::default();
    for entry in visible_entries(source) {
        let path = entry.path();
        if entry.file_type().is_dir() {
            continue;
        }
        let rel = rel_string(source, path);
        let utf8 = path
            .strip_prefix(source)
            .ok()
            .and_then(Path::to_str)
            .is_some();
        if entry.file_type().is_symlink() || !utf8 || dialect.skip_file(&rel) {
            out.skipped.push(rel);
            continue;
        }
        let meta = entry
            .metadata()
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        let modified = meta
            .modified()
            .map(rfc3339)
            .unwrap_or_else(|_| rfc3339(SystemTime::now()));
        let created = meta
            .created()
            .map(rfc3339)
            .unwrap_or_else(|_| modified.clone());
        out.files.push(SourceFile {
            rel,
            abs: path.to_path_buf(),
            is_note: is_markdown(path),
            size: meta.len(),
            created,
            modified,
        });
    }
    out.files.sort_by(|a, b| a.rel.cmp(&b.rel));
    out.skipped.sort();
    Ok(out)
}

/// Every entry under `source` except hidden ones and what lies inside them.
fn visible_entries(source: &Path) -> impl Iterator<Item = walkdir::DirEntry> {
    WalkDir::new(source)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(Result::ok)
        .filter(|e| e.depth() > 0)
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
}

fn rel_string(source: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(source).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn rfc3339(time: SystemTime) -> String {
    chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339()
}

#[cfg(test)]
// Fixtures write files directly; the atomic-write rule guards vault code, not setup.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::vault_import::dialect::{Obsidian, Tolaria};
    use crate::vault_import::tests::{fixture, obsidian_fixture, tolaria_fixture};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn detects_obsidian_by_folder() {
        let d = detect(&obsidian_fixture(), None).unwrap();
        assert_eq!(d.format, Format::Obsidian);
    }

    #[test]
    fn detects_tolaria_by_views() {
        let d = detect(&tolaria_fixture(), None).unwrap();
        assert_eq!(d.format, Format::Tolaria);
        assert_eq!(d.reason, "views/*.yml");
    }

    #[test]
    fn detects_tolaria_by_type_field() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.md"), "no frontmatter\n").unwrap();
        fs::write(tmp.path().join("b.md"), "---\ntype: Note\n---\n# B\n").unwrap();
        let d = detect(tmp.path(), None).unwrap();
        assert_eq!(d.format, Format::Tolaria);
        assert!(d.reason.contains("b.md"), "{}", d.reason);
    }

    #[test]
    fn empty_folder_is_undetectable() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(detect(tmp.path(), None).unwrap_err(), UNDETECTABLE);
        // --from decides instead.
        assert_eq!(
            detect(tmp.path(), Some(Format::Obsidian)).unwrap().format,
            Format::Obsidian
        );
    }

    #[test]
    fn plain_markdown_folder_is_undetectable() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.md"), "---\ntitle: A\n---\n").unwrap();
        assert_eq!(detect(tmp.path(), None).unwrap_err(), UNDETECTABLE);
    }

    #[test]
    fn from_overrides_detection() {
        let d = detect(&obsidian_fixture(), Some(Format::Tolaria)).unwrap();
        assert_eq!(d.format, Format::Tolaria);
    }

    #[test]
    fn cerbo_vault_is_refused_even_with_from() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".cerbo/objects")).unwrap();
        fs::create_dir_all(tmp.path().join(".obsidian")).unwrap();
        assert!(
            detect(tmp.path(), None)
                .unwrap_err()
                .contains("is a Cerbo vault")
        );
        assert!(detect(tmp.path(), Some(Format::Obsidian)).is_err());
    }

    #[test]
    fn obsidian_scan_skips_hidden_and_canvas() {
        let scan = scan(&obsidian_fixture(), &Obsidian).unwrap();
        let rels: Vec<&str> = scan.files.iter().map(|f| f.rel.as_str()).collect();
        assert!(rels.contains(&"Welcome.md"));
        assert!(rels.contains(&"attachments/photo.png"));
        assert!(!rels.iter().any(|r| r.starts_with(".obsidian")), "{rels:?}");
        assert_eq!(scan.skipped, ["plan.canvas"]);
        let photo = scan
            .files
            .iter()
            .find(|f| f.rel == "attachments/photo.png")
            .unwrap();
        assert!(!photo.is_note);
        assert!(
            scan.files
                .iter()
                .find(|f| f.rel == "Welcome.md")
                .unwrap()
                .is_note
        );
    }

    #[test]
    fn tolaria_scan_skips_views() {
        let scan = scan(&tolaria_fixture(), &Tolaria).unwrap();
        assert_eq!(scan.skipped, ["views/devices.yml"]);
        assert!(scan.files.iter().any(|f| f.rel == "journals/2025-04-27.md"));
        assert!(scan.files.iter().any(|f| f.rel == "филаменты.md"));
    }

    #[test]
    fn times_come_from_the_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("old.md");
        fs::write(&path, "x").unwrap();
        let when = chrono::DateTime::parse_from_rfc3339("2025-12-07T10:00:00Z").unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(SystemTime::from(when))
            .unwrap();
        let scan = scan(tmp.path(), &Obsidian).unwrap();
        assert!(
            scan.files[0].modified.starts_with("2025-12-07"),
            "{}",
            scan.files[0].modified
        );
        assert!(!scan.files[0].created.is_empty());
    }

    #[test]
    fn fixture_dir_exists() {
        assert!(fixture("").is_dir());
    }
}
