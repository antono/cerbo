//! Vault symlink tree materialization (cerbo symlink).
//!
//! Builds a human-navigable directory tree at `<repo-root>/cerbo/`
//! from the UUID-based object store at `<repo-root>/.cerbo/objects/`.
//! Every leaf symlink is relative so the repository stays portable.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::object::{ObjectMeta, ObjectType};

// ── Public types ──────────────────────────────────────────────────────────────

/// Summary of a successful materialize run.
#[derive(Debug)]
pub struct MaterializeReport {
    pub objects_scanned: usize,
    pub leaves_created: usize,
    pub dirs_created: usize,
}

/// Errors that can occur during materialization.
#[derive(Debug)]
pub enum SymlinkError {
    /// Two or more pages map to the same rendered path.
    Conflict { collisions: Vec<Collision> },
    /// The existing cerbo/ directory contains non-cerbo entries.
    UnsafeWipe { offenders: Vec<PathBuf> },
    /// Underlying I/O error.
    Io(std::io::Error),
    /// Other error with message.
    Other(String),
}

impl std::fmt::Display for SymlinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymlinkError::Conflict { collisions } => {
                write!(f, "symlink tree conflicts:")?;
                for c in collisions {
                    write!(f, "\n  {}: {}", c.path.display(), c.uuids.join(", "))?;
                }
                Ok(())
            }
            SymlinkError::UnsafeWipe { offenders } => {
                write!(f, "unsafe wipe — foreign entries in cerbo/:")?;
                for p in offenders {
                    write!(f, "\n  {}", p.display())?;
                }
                Ok(())
            }
            SymlinkError::Io(e) => write!(f, "I/O error: {}", e),
            SymlinkError::Other(s) => write!(f, "{}", s),
        }
    }
}

/// A path collision between two or more pages.
#[derive(Debug)]
pub struct Collision {
    pub path: PathBuf,
    pub uuids: Vec<String>,
}

// ── Internal types ────────────────────────────────────────────────────────────

struct PlanEntry {
    uuid: String,
    virtual_path: String,
    slug: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Build (or rebuild) the symlink tree at `<vault_root>/cerbo/`.
///
/// Algorithm:
/// 1. Clean stale `cerbo.tmp-*` / `cerbo.gc-*` siblings from prior crashes
/// 2. Scan `.cerbo/objects/*/meta.ttl`, skip `:Ontology`
/// 3. Validate plan — abort on any collision, filesystem untouched
/// 4. Safe-wipe check on existing `cerbo/`
/// 5. Materialize into `cerbo.tmp-<pid>/` with relative symlinks
/// 6. Atomic two-rename swap into place
pub fn materialize(vault_root: &Path) -> Result<MaterializeReport, SymlinkError> {
    cleanup_stale_siblings(vault_root)?;

    let plan = build_plan(vault_root)?;
    let objects_scanned = plan.len();

    validate_plan(&plan)?;

    let cerbo_out = vault_root.join("cerbo");
    if cerbo_out.exists() {
        safe_wipe_check(&cerbo_out, vault_root)?;
    }

    // Count unique directories that will be created
    let mut unique_dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for entry in &plan {
        if !entry.virtual_path.is_empty() {
            let mut p = PathBuf::from(&entry.virtual_path);
            loop {
                unique_dirs.insert(p.clone());
                match p.parent().map(|x| x.to_path_buf()) {
                    Some(parent) if !parent.as_os_str().is_empty() => p = parent,
                    _ => break,
                }
            }
        }
    }
    let dirs_created = unique_dirs.len();

    let pid = std::process::id();
    let tmp_dir = vault_root.join(format!("cerbo.tmp-{}", pid));
    std::fs::create_dir(&tmp_dir).map_err(SymlinkError::Io)?;

    let mut leaves_created = 0usize;
    for entry in &plan {
        let leaf_parent = if entry.virtual_path.is_empty() {
            tmp_dir.clone()
        } else {
            let vp_dir = tmp_dir.join(&entry.virtual_path);
            std::fs::create_dir_all(&vp_dir).map_err(SymlinkError::Io)?;
            vp_dir
        };

        let leaf_path = leaf_parent.join(&entry.slug);
        let object_dir = vault_root.join(".cerbo").join("objects").join(&entry.uuid);

        let rel_target = pathdiff::diff_paths(&object_dir, &leaf_parent)
            .ok_or_else(|| SymlinkError::Other(format!(
                "cannot compute relative path: {} → {}",
                leaf_parent.display(),
                object_dir.display()
            )))?;

        create_symlink(&leaf_path, &rel_target)?;
        leaves_created += 1;
    }

    // Atomic swap
    let gc_dir = vault_root.join(format!("cerbo.gc-{}", pid));
    if cerbo_out.exists() {
        std::fs::rename(&cerbo_out, &gc_dir).map_err(SymlinkError::Io)?;
    }
    std::fs::rename(&tmp_dir, &cerbo_out).map_err(SymlinkError::Io)?;
    if gc_dir.exists() {
        std::fs::remove_dir_all(&gc_dir).map_err(SymlinkError::Io)?;
    }

    Ok(MaterializeReport { objects_scanned, leaves_created, dirs_created })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn cleanup_stale_siblings(vault_root: &Path) -> Result<(), SymlinkError> {
    let entries = std::fs::read_dir(vault_root).map_err(SymlinkError::Io)?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let n = name.to_string_lossy();
        if n.starts_with("cerbo.tmp-") || n.starts_with("cerbo.gc-") {
            let p = entry.path();
            if p.is_dir() {
                std::fs::remove_dir_all(&p).map_err(SymlinkError::Io)?;
            }
        }
    }
    Ok(())
}

fn build_plan(vault_root: &Path) -> Result<Vec<PlanEntry>, SymlinkError> {
    let objects_dir = vault_root.join(".cerbo").join("objects");
    if !objects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut plan = Vec::new();

    let entries = std::fs::read_dir(&objects_dir).map_err(SymlinkError::Io)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let uuid = entry.file_name().to_string_lossy().to_string();
        let meta_path = path.join("meta.ttl");
        if !meta_path.exists() {
            continue;
        }

        let meta = ObjectMeta::read_from_file(&meta_path)
            .map_err(|e| SymlinkError::Other(format!(
                "failed to read {}: {}", meta_path.display(), e
            )))?;

        if matches!(meta.object_type, ObjectType::Ontology) {
            continue;
        }

        let slug = meta.slug.unwrap_or_else(|| {
            let id = uuid::Uuid::parse_str(&uuid).unwrap_or_else(|_| uuid::Uuid::nil());
            crate::slug::slugify(&meta.title, id)
        });

        let virtual_path = meta.virtual_path.unwrap_or_default();

        plan.push(PlanEntry { uuid, virtual_path, slug });
    }

    Ok(plan)
}

/// Returns the rendered path for a plan entry: `<virtual_path>/<slug>` or just `<slug>`.
fn rendered_path(entry: &PlanEntry) -> PathBuf {
    if entry.virtual_path.is_empty() {
        PathBuf::from(&entry.slug)
    } else {
        PathBuf::from(&entry.virtual_path).join(&entry.slug)
    }
}

fn validate_plan(plan: &[PlanEntry]) -> Result<(), SymlinkError> {
    // Map rendered_path → vec of UUIDs
    let mut leaf_map: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for entry in plan {
        leaf_map
            .entry(rendered_path(entry))
            .or_default()
            .push(entry.uuid.clone());
    }

    let mut collisions: Vec<Collision> = Vec::new();

    // Leaf-vs-leaf: two entries share the same path
    for (path, uuids) in &leaf_map {
        if uuids.len() > 1 {
            collisions.push(Collision { path: path.clone(), uuids: uuids.clone() });
        }
    }

    // Dir-vs-leaf: one entry's path is a strict prefix of another's
    let leaf_paths: Vec<&PathBuf> = leaf_map.keys().collect();
    for (i, p1) in leaf_paths.iter().enumerate() {
        for p2 in &leaf_paths[i + 1..] {
            if p2.starts_with(p1) && p2.as_path() != p1.as_path() {
                // p1 must be both a symlink and a directory — conflict
                let mut uuids = leaf_map[*p1].clone();
                uuids.extend(leaf_map[*p2].iter().cloned());
                collisions.push(Collision { path: (*p1).clone(), uuids });
            } else if p1.starts_with(p2) && p1.as_path() != p2.as_path() {
                let mut uuids = leaf_map[*p2].clone();
                uuids.extend(leaf_map[*p1].iter().cloned());
                collisions.push(Collision { path: (*p2).clone(), uuids });
            }
        }
    }

    if !collisions.is_empty() {
        collisions.sort_by(|a, b| a.path.cmp(&b.path));
        collisions.dedup_by(|a, b| {
            if a.path == b.path {
                b.uuids.extend(a.uuids.drain(..));
                b.uuids.sort();
                b.uuids.dedup();
                true
            } else {
                false
            }
        });
        return Err(SymlinkError::Conflict { collisions });
    }

    Ok(())
}

fn safe_wipe_check(cerbo_dir: &Path, vault_root: &Path) -> Result<(), SymlinkError> {
    let objects_root = vault_root.join(".cerbo").join("objects");
    let mut offenders = Vec::new();
    collect_unsafe_entries(cerbo_dir, &objects_root, &mut offenders);
    if !offenders.is_empty() {
        return Err(SymlinkError::UnsafeWipe { offenders });
    }
    Ok(())
}

fn collect_unsafe_entries(dir: &Path, objects_root: &Path, offenders: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else {
            offenders.push(path);
            continue;
        };

        if ft.is_dir() {
            collect_unsafe_entries(&path, objects_root, offenders);
        } else if ft.is_symlink() {
            if !symlink_points_into(path.as_path(), objects_root) {
                offenders.push(path);
            }
        } else {
            // Regular file or other — foreign
            offenders.push(path);
        }
    }
}

/// Return true if `link` resolves (or would resolve) into `objects_root`.
fn symlink_points_into(link: &Path, objects_root: &Path) -> bool {
    // Try canonical resolution first (works for valid symlinks)
    if let Ok(canonical) = std::fs::canonicalize(link) {
        return canonical.starts_with(objects_root);
    }
    // For broken symlinks, resolve the raw target manually
    if let Ok(raw) = std::fs::read_link(link) {
        let parent = link.parent().unwrap_or(link);
        let joined = parent.join(&raw);
        let normalized = normalize_path(&joined);
        return normalized.starts_with(objects_root);
    }
    false
}

/// Lexically normalize a path (resolve `.` and `..` without syscalls).
fn normalize_path(path: &Path) -> PathBuf {
    let mut out: Vec<std::path::Component<'_>> = Vec::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => { out.pop(); }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out.iter().collect()
}

#[cfg(unix)]
fn create_symlink(link_path: &Path, target: &Path) -> Result<(), SymlinkError> {
    std::os::unix::fs::symlink(target, link_path).map_err(SymlinkError::Io)
}

#[cfg(windows)]
fn create_symlink(link_path: &Path, target: &Path) -> Result<(), SymlinkError> {
    std::os::windows::fs::symlink_dir(target, link_path).map_err(|e| {
        // ERROR_PRIVILEGE_NOT_HELD = 1314
        if e.raw_os_error() == Some(1314) {
            SymlinkError::Other(
                "Creating directory symlinks requires Developer Mode or administrator privileges.\n\
                Enable Developer Mode: https://learn.microsoft.com/en-us/windows/apps/get-started/enable-your-device-for-development"
                    .to_string()
            )
        } else {
            SymlinkError::Io(e)
        }
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn make_repo(temp: &TempDir) -> PathBuf {
        let root = temp.path().to_path_buf();
        fs::create_dir_all(root.join(".cerbo").join("objects")).unwrap();
        root
    }

    fn make_object(repo: &Path, uuid: &str, title: &str, slug: Option<&str>, vpath: Option<&str>) {
        let obj_dir = repo.join(".cerbo").join("objects").join(uuid);
        fs::create_dir_all(&obj_dir).unwrap();
        fs::write(obj_dir.join("page.md"), format!("# {}", title)).unwrap();

        let mut extra = String::new();
        if let Some(s) = slug {
            extra.push_str(&format!("\n    cerbo:slug \"{}\" .", s));
        }
        if let Some(vp) = vpath {
            extra.push_str(&format!("\n    cerbo:virtualPath \"{}\" .", vp));
        }

        let ttl = format!(
            "@prefix : <cerbo://ontology/> .\n@prefix schema: <cerbo://ontology/schema/> .\n@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\n<cerbo://objects/<uuid>>\n    :type :Product ;\n    :title \"{title}\" ;\n    schema:dateCreated \"2026-01-01T00:00:00Z\"^^xsd:dateTime ;\n    schema:dateModified \"2026-01-01T00:00:00Z\"^^xsd:dateTime .{extra}\n"
        );
        fs::write(obj_dir.join("meta.ttl"), ttl).unwrap();
    }

    #[test]
    fn test_materialize_empty_repo() {
        let temp = TempDir::new().unwrap();
        let repo = make_repo(&temp);
        let report = materialize(&repo).unwrap();
        assert_eq!(report.objects_scanned, 0);
        assert_eq!(report.leaves_created, 0);
        assert!(repo.join("cerbo").exists());
    }

    #[test]
    fn test_materialize_single_page() {
        let temp = TempDir::new().unwrap();
        let repo = make_repo(&temp);
        make_object(&repo, "aaaa-1111", "Rust Ownership", Some("rust-ownership"), None);

        let report = materialize(&repo).unwrap();
        assert_eq!(report.leaves_created, 1);

        let leaf = repo.join("cerbo").join("rust-ownership");
        assert!(leaf.exists(), "leaf symlink should exist");
        // Verify it's a symlink
        assert!(fs::symlink_metadata(&leaf).unwrap().file_type().is_symlink());
        // Verify it resolves to the object dir
        let target = fs::canonicalize(&leaf).unwrap();
        assert!(target.ends_with("aaaa-1111"));
    }

    #[test]
    fn test_materialize_with_virtual_path() {
        let temp = TempDir::new().unwrap();
        let repo = make_repo(&temp);
        make_object(&repo, "bbbb-2222", "Lifetimes", Some("lifetimes"), Some("notes/rust"));

        materialize(&repo).unwrap();

        let leaf = repo.join("cerbo").join("notes").join("rust").join("lifetimes");
        assert!(leaf.exists(), "nested leaf should exist at {}", leaf.display());
    }

    #[test]
    fn test_materialize_is_idempotent() {
        let temp = TempDir::new().unwrap();
        let repo = make_repo(&temp);
        make_object(&repo, "cccc-3333", "Hello", Some("hello"), None);

        materialize(&repo).unwrap();
        let r2 = materialize(&repo).unwrap();
        assert_eq!(r2.leaves_created, 1);
        // No tmp/gc siblings remain
        let siblings: Vec<_> = fs::read_dir(&repo)
            .unwrap()
            .flatten()
            .filter(|e| {
                let n = e.file_name();
                let n = n.to_string_lossy();
                n.starts_with("cerbo.tmp-") || n.starts_with("cerbo.gc-")
            })
            .collect();
        assert!(siblings.is_empty(), "stale siblings left: {:?}", siblings);
    }

    #[test]
    fn test_materialize_portability() {
        let temp = TempDir::new().unwrap();
        let repo = make_repo(&temp);
        make_object(&repo, "dddd-4444", "Page", Some("page"), Some("a/b"));

        materialize(&repo).unwrap();

        // Move the repo to a new path
        let new_temp = TempDir::new().unwrap();
        let new_repo = new_temp.path().join("moved-repo");
        fs::rename(&repo, &new_repo).unwrap();

        // Symlink should still resolve
        let leaf = new_repo.join("cerbo").join("a").join("b").join("page");
        assert!(fs::canonicalize(&leaf).is_ok(), "symlink must resolve after move");
    }

    #[test]
    fn test_validate_plan_leaf_vs_leaf_conflict() {
        let plan = vec![
            PlanEntry { uuid: "u1".into(), virtual_path: "".into(), slug: "foo".into() },
            PlanEntry { uuid: "u2".into(), virtual_path: "".into(), slug: "foo".into() },
        ];
        let err = validate_plan(&plan).unwrap_err();
        assert!(matches!(err, SymlinkError::Conflict { .. }));
    }

    #[test]
    fn test_validate_plan_dir_vs_leaf_conflict() {
        let plan = vec![
            PlanEntry { uuid: "u1".into(), virtual_path: "".into(), slug: "notes".into() },
            PlanEntry { uuid: "u2".into(), virtual_path: "notes".into(), slug: "rust".into() },
        ];
        let err = validate_plan(&plan).unwrap_err();
        assert!(matches!(err, SymlinkError::Conflict { .. }));
    }

    #[test]
    fn test_normalize_path() {
        let p = PathBuf::from("/a/b/../c/./d");
        assert_eq!(normalize_path(&p), PathBuf::from("/a/c/d"));
    }

    #[test]
    fn test_safe_wipe_rejects_regular_file() {
        let temp = TempDir::new().unwrap();
        let repo = make_repo(&temp);
        let cerbo_dir = repo.join("cerbo");
        fs::create_dir_all(&cerbo_dir).unwrap();
        fs::write(cerbo_dir.join("foreign.txt"), "oops").unwrap();

        let err = safe_wipe_check(&cerbo_dir, &repo).unwrap_err();
        assert!(matches!(err, SymlinkError::UnsafeWipe { .. }));
    }
}
