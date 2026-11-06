//! The vault's sidebar layout (spec `vault-tree-layout`, design D1-D4): the
//! order of each folder's children and the folders that exist without pages,
//! stored in `.cerbo/tree.toml`.
//!
//! Folders are still derived from page paths; the layout only adds pinned
//! folders and an order on top. A vault without the file looks as before.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::CerboContext;
use crate::page::{page_list, PageMeta};

/// File name of the layout inside `.cerbo/`.
pub const LAYOUT_FILE: &str = "tree.toml";

const HEADER: &str = "# Sidebar layout, written by Cerbo. Safe to edit by hand.\n\
# pinned: folders that exist even with no page in them.\n\
# order: per folder (\"\" is the root), its children in display order;\n\
# a page is its UUID, a folder is its name followed by \"/\".\n\n";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TreeLayout {
    pub pinned: Vec<String>,
    pub order: BTreeMap<String, Vec<String>>,
}

/// One node of the ordered tree (D2), as `cerbo folder list --json` prints it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TreeNode {
    Folder { name: String, path: String, pinned: bool, children: Vec<TreeNode> },
    Page { uuid: String, title: String },
}

pub fn layout_path(ctx: &CerboContext) -> PathBuf {
    ctx.config_dir.join(LAYOUT_FILE)
}

/// Read the layout. A missing file is an empty layout; a file that does not
/// parse is an error naming it, and callers then write nothing.
pub fn tree_layout(ctx: &CerboContext) -> Result<TreeLayout, String> {
    let path = layout_path(ctx);
    match std::fs::read_to_string(&path) {
        Ok(text) => toml::from_str(&text).map_err(|e| format!("cannot parse {}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(TreeLayout::default()),
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
    }
}

/// Write the layout atomically with pinned paths sorted and deduplicated,
/// duplicate entries dropped (first one wins) and empty lists removed.
fn write_layout(ctx: &CerboContext, layout: &TreeLayout) -> Result<(), String> {
    let mut clean = layout.clone();
    clean.pinned.sort();
    clean.pinned.dedup();
    for list in clean.order.values_mut() {
        let mut seen = BTreeSet::new();
        list.retain(|e| seen.insert(e.clone()));
    }
    clean.order.retain(|_, list| !list.is_empty());
    let body = toml::to_string(&clean).map_err(|e| format!("cannot encode tree layout: {e}"))?;
    let path = layout_path(ctx);
    crate::fsio::write_atomic_str(&path, &format!("{HEADER}{body}"))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

// ── Paths and entries ───────────────────────────────────────────────────────

fn validate_folder(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("folder path must not be empty".into());
    }
    crate::vault::validate_virtual_path(path).map_err(|e| format!("invalid folder path {path:?}: {e}"))
}

/// The parent folder of `path` (`""` for a top-level folder) and its last segment.
fn split_parent(path: &str) -> (&str, &str) {
    match path.rsplit_once('/') {
        Some((parent, name)) => (parent, name),
        None => ("", path),
    }
}

fn is_at_or_under(path: &str, prefix: &str) -> bool {
    path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
}

fn folder_entry(name: &str) -> String {
    format!("{name}/")
}

/// Whether `entry` is a page UUID or a valid folder segment followed by `/`.
fn valid_entry(entry: &str) -> bool {
    match entry.strip_suffix('/') {
        Some(name) => !name.is_empty() && !name.contains('/') && crate::vault::validate_virtual_path(name).is_ok(),
        None => uuid::Uuid::parse_str(entry).is_ok(),
    }
}

/// Turn a user-typed entry into its stored form: a page UUID stays as is, a
/// folder name gets its trailing `/` (`notes` and `notes/` both name a child
/// folder). Used by `cerbo folder order`.
pub fn normalize_entry(entry: &str) -> String {
    if uuid::Uuid::parse_str(entry).is_ok() || entry.ends_with('/') {
        entry.to_string()
    } else {
        format!("{entry}/")
    }
}

/// Every folder in the tree: page paths, pinned paths, and their ancestors.
fn all_folders(pages: &[PageMeta], layout: &TreeLayout) -> BTreeSet<String> {
    let mut folders = BTreeSet::new();
    let paths = pages.iter().filter_map(|p| p.virtual_path.as_deref()).chain(layout.pinned.iter().map(String::as_str));
    for path in paths {
        let mut end = 0;
        for segment in path.split('/') {
            end += segment.len();
            folders.insert(path[..end].to_string());
            end += 1;
        }
    }
    folders
}

fn page_folder(page: &PageMeta) -> &str {
    page.virtual_path.as_deref().unwrap_or("")
}

/// The entry keys of `parent`'s existing children.
fn child_keys(parent: &str, pages: &[PageMeta], folders: &BTreeSet<String>) -> BTreeSet<String> {
    let mut keys: BTreeSet<String> = pages.iter().filter(|p| page_folder(p) == parent).map(|p| p.uuid.clone()).collect();
    for folder in folders {
        let (folder_parent, name) = split_parent(folder);
        if folder_parent == parent {
            keys.insert(folder_entry(name));
        }
    }
    keys
}

// ── Display order (D2) ──────────────────────────────────────────────────────

/// The ordered tree: listed children first in list order, then the rest with
/// folders before pages, each by name or title, case-insensitively.
pub fn folder_tree(ctx: &CerboContext) -> Result<Vec<TreeNode>, String> {
    let layout = tree_layout(ctx)?;
    let pages = page_list(ctx)?;
    Ok(build_tree(&pages, &layout))
}

pub fn build_tree(pages: &[PageMeta], layout: &TreeLayout) -> Vec<TreeNode> {
    let folders = all_folders(pages, layout);
    let pinned: BTreeSet<&str> = layout.pinned.iter().map(String::as_str).collect();
    build_level("", pages, layout, &folders, &pinned)
}

fn build_level(
    parent: &str,
    pages: &[PageMeta],
    layout: &TreeLayout,
    folders: &BTreeSet<String>,
    pinned: &BTreeSet<&str>,
) -> Vec<TreeNode> {
    let mut children: Vec<(String, TreeNode)> = Vec::new();
    for folder in folders {
        let (folder_parent, name) = split_parent(folder);
        if folder_parent != parent {
            continue;
        }
        let node = TreeNode::Folder {
            name: name.to_string(),
            path: folder.clone(),
            pinned: pinned.contains(folder.as_str()),
            children: build_level(folder, pages, layout, folders, pinned),
        };
        children.push((folder_entry(name), node));
    }
    for page in pages.iter().filter(|p| page_folder(p) == parent) {
        children.push((page.uuid.clone(), TreeNode::Page { uuid: page.uuid.clone(), title: page.title.clone() }));
    }
    children.sort_by(|(_, a), (_, b)| default_order(a, b));
    apply_order(children, layout.order.get(parent))
}

fn default_order(a: &TreeNode, b: &TreeNode) -> std::cmp::Ordering {
    let rank = |n: &TreeNode| match n {
        TreeNode::Folder { name, .. } => (0, name.to_lowercase(), name.clone()),
        TreeNode::Page { uuid, title } => (1, title.to_lowercase(), format!("{title}\u{0}{uuid}")),
    };
    rank(a).cmp(&rank(b))
}

fn apply_order(mut children: Vec<(String, TreeNode)>, list: Option<&Vec<String>>) -> Vec<TreeNode> {
    let mut ordered = Vec::with_capacity(children.len());
    for entry in list.into_iter().flatten() {
        if let Some(i) = children.iter().position(|(key, _)| key == entry) {
            ordered.push(children.remove(i).1);
        }
    }
    ordered.extend(children.into_iter().map(|(_, node)| node));
    ordered
}

// ── Mutations ───────────────────────────────────────────────────────────────

/// Pin a new folder (spec "Pinned folders"). Fails when it already exists.
pub fn folder_create(ctx: &CerboContext, path: &str) -> Result<(), String> {
    validate_folder(path)?;
    let mut layout = tree_layout(ctx)?;
    let pages = page_list(ctx)?;
    if all_folders(&pages, &layout).contains(path) {
        return Err(format!("folder already exists: {path}"));
    }
    layout.pinned.push(path.to_string());
    write_layout(ctx, &layout)
}

/// Delete an empty folder (spec "Folder delete").
pub fn folder_delete(ctx: &CerboContext, path: &str) -> Result<(), String> {
    validate_folder(path)?;
    let mut layout = tree_layout(ctx)?;
    let pages = page_list(ctx)?;
    let inside = pages.iter().filter(|p| is_at_or_under(page_folder(p), path)).count();
    if inside > 0 {
        return Err(format!("folder is not empty: {path} ({inside} pages)"));
    }
    if !all_folders(&pages, &layout).contains(path) {
        return Err(format!("folder not found: {path}"));
    }
    layout.pinned.retain(|p| !is_at_or_under(p, path));
    layout.order.retain(|key, _| !is_at_or_under(key, path));
    let (parent, name) = split_parent(path);
    if let Some(list) = layout.order.get_mut(parent) {
        list.retain(|e| *e != folder_entry(name));
    }
    write_layout(ctx, &layout)
}

/// Replace the order of `parent`'s children (`""` is the root). Entries that
/// name no existing child, and duplicates, are dropped; a malformed entry
/// fails before anything is written. Returns the stored list.
pub fn folder_set_order(ctx: &CerboContext, parent: &str, entries: Vec<String>) -> Result<Vec<String>, String> {
    if !parent.is_empty() {
        validate_folder(parent)?;
    }
    if let Some(bad) = entries.iter().find(|e| !valid_entry(e)) {
        return Err(format!("invalid order entry {bad:?}: expected a page UUID or a folder name followed by \"/\""));
    }
    let mut layout = tree_layout(ctx)?;
    let pages = page_list(ctx)?;
    let folders = all_folders(&pages, &layout);
    if !parent.is_empty() && !folders.contains(parent) {
        return Err(format!("folder not found: {parent}"));
    }
    let existing = child_keys(parent, &pages, &folders);
    let mut seen = BTreeSet::new();
    let list: Vec<String> = entries.into_iter().filter(|e| existing.contains(e) && seen.insert(e.clone())).collect();
    layout.order.insert(parent.to_string(), list.clone());
    write_layout(ctx, &layout)?;
    Ok(list)
}

/// Check that the layout can be read before a folder rename writes pages, so
/// a broken file fails the rename up front instead of halfway.
pub(crate) fn check_readable(ctx: &CerboContext) -> Result<(), String> {
    tree_layout(ctx).map(|_| ())
}

/// After a folder rename: carry pins and order lists to the new prefix and
/// fix the folder's entry in its parent's list (spec "Folder rename keeps the
/// layout"). Writes only when something changed.
pub(crate) fn rename_layout_prefix(ctx: &CerboContext, old: &str, new: &str) -> Result<(), String> {
    let mut layout = tree_layout(ctx)?;
    if rename_prefix(&mut layout, old, new) {
        write_layout(ctx, &layout)?;
    }
    Ok(())
}

fn rename_prefix(layout: &mut TreeLayout, old: &str, new: &str) -> bool {
    let before = layout.clone();
    let map = |p: &str| -> Option<String> {
        if p == old {
            Some(new.to_string())
        } else {
            p.strip_prefix(old).and_then(|rest| rest.strip_prefix('/')).map(|rest| format!("{new}/{rest}"))
        }
    };

    for p in layout.pinned.iter_mut() {
        if let Some(moved) = map(p) {
            *p = moved;
        }
    }

    // Renaming onto an existing folder merges: its own list keeps its place,
    // so unmoved keys go in first and the moved lists extend them.
    let order = std::mem::take(&mut layout.order);
    let (moved, kept): (Vec<_>, Vec<_>) = order.into_iter().partition(|(key, _)| map(key).is_some());
    layout.order.extend(kept);
    for (key, list) in moved {
        let key = map(&key).unwrap_or(key);
        layout.order.entry(key).or_default().extend(list);
    }

    let (old_parent, old_name) = split_parent(old);
    let (new_parent, new_name) = split_parent(new);
    if let Some(list) = layout.order.get_mut(old_parent) {
        let old_entry = folder_entry(old_name);
        if old_parent == new_parent {
            let new_entry = folder_entry(new_name);
            let already = list.contains(&new_entry);
            if let Some(i) = list.iter().position(|e| *e == old_entry) {
                if already {
                    list.remove(i);
                } else {
                    list[i] = new_entry;
                }
            }
        } else {
            list.retain(|e| *e != old_entry);
        }
    }
    *layout != before
}

#[cfg(test)]
// Fixtures write files directly; the atomic-write rule guards vault code, not setup.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::page::{page_create, page_update_virtual_path};

    fn ctx() -> (tempfile::TempDir, CerboContext) {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();
        std::fs::create_dir_all(dir.join("objects")).unwrap();
        (tmp, CerboContext { config_dir: dir.clone(), cache_dir: dir.join("cache") })
    }

    fn page(ctx: &CerboContext, title: &str, path: Option<&str>) -> String {
        let uuid = page_create(ctx, title.to_string()).unwrap();
        if let Some(p) = path {
            page_update_virtual_path(ctx, uuid.clone(), Some(p.to_string())).unwrap();
        }
        uuid
    }

    fn meta(uuid: &str, title: &str, path: Option<&str>) -> PageMeta {
        PageMeta { uuid: uuid.into(), title: title.into(), virtual_path: path.map(str::to_string) }
    }

    /// Row labels of the tree, depth-first: folders as `name/`, pages by title.
    fn labels(nodes: &[TreeNode]) -> Vec<String> {
        let mut out = Vec::new();
        for n in nodes {
            match n {
                TreeNode::Folder { name, children, .. } => {
                    out.push(format!("{name}/"));
                    out.extend(labels(children).into_iter().map(|l| format!("  {l}")));
                }
                TreeNode::Page { title, .. } => out.push(title.clone()),
            }
        }
        out
    }

    const A: &str = "00000000-0000-4000-8000-00000000000a";
    const B: &str = "00000000-0000-4000-8000-00000000000b";
    const Z: &str = "00000000-0000-4000-8000-00000000000f";
    const GONE: &str = "00000000-0000-4000-8000-000000000137";

    // ── File ──

    #[test]
    fn missing_file_is_an_empty_layout() {
        let (_tmp, ctx) = ctx();
        assert_eq!(tree_layout(&ctx).unwrap(), TreeLayout::default());
    }

    #[test]
    fn layout_round_trips_sorted_and_deduplicated() {
        let (_tmp, ctx) = ctx();
        let layout = TreeLayout {
            pinned: vec!["zeta".into(), "alpha".into(), "zeta".into()],
            order: BTreeMap::from([
                ("notes".into(), vec![A.into(), B.into(), A.into()]),
                ("empty".into(), vec![]),
                (String::new(), vec!["notes/".into()]),
            ]),
        };
        write_layout(&ctx, &layout).unwrap();
        let read = tree_layout(&ctx).unwrap();
        assert_eq!(read.pinned, vec!["alpha", "zeta"]);
        assert_eq!(read.order.get("notes").unwrap(), &vec![A.to_string(), B.to_string()]);
        assert!(!read.order.contains_key("empty"));
        assert_eq!(read.order.get("").unwrap(), &vec!["notes/".to_string()]);
        let text = std::fs::read_to_string(layout_path(&ctx)).unwrap();
        assert!(text.starts_with("# Sidebar layout"), "{text}");
    }

    #[test]
    fn broken_file_fails_and_is_left_unchanged() {
        let (_tmp, ctx) = ctx();
        let path = layout_path(&ctx);
        std::fs::write(&path, "order = [").unwrap();
        let err = folder_create(&ctx, "inbox").unwrap_err();
        assert!(err.contains("tree.toml"), "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "order = [");
    }

    // ── Display order ──

    #[test]
    fn without_a_layout_folders_come_first_then_titles_case_insensitively() {
        let pages = [meta(B, "beta", None), meta(A, "Alpha", None), meta(Z, "Own", Some("notes/rust"))];
        let tree = build_tree(&pages, &TreeLayout::default());
        assert_eq!(labels(&tree), vec!["notes/", "  rust/", "    Own", "Alpha", "beta"]);
    }

    #[test]
    fn listed_children_come_first_then_the_rest_in_default_order() {
        // Spec "Listed children first": notes has rust/, Alpha, Beta; list = [Beta].
        let pages = [meta(A, "Alpha", Some("notes")), meta(B, "Beta", Some("notes")), meta(Z, "Own", Some("notes/rust"))];
        let layout = TreeLayout { order: BTreeMap::from([("notes".into(), vec![B.into()])]), ..Default::default() };
        let tree = build_tree(&pages, &layout);
        assert_eq!(labels(&tree), vec!["notes/", "  Beta", "  rust/", "    Own", "  Alpha"]);
    }

    #[test]
    fn stale_and_duplicate_entries_are_skipped() {
        let pages = [meta(A, "Home", None), meta(B, "Other", None)];
        let layout = TreeLayout {
            order: BTreeMap::from([(String::new(), vec![GONE.into(), B.into(), "gone/".into(), B.into(), A.into()])]),
            ..Default::default()
        };
        assert_eq!(labels(&build_tree(&pages, &layout)), vec!["Other", "Home"]);
    }

    #[test]
    fn pinned_folders_render_with_their_ancestors_even_when_empty() {
        let pages = [meta(A, "Home", None)];
        let layout = TreeLayout { pinned: vec!["projects/2026".into()], ..Default::default() };
        let tree = build_tree(&pages, &layout);
        assert_eq!(labels(&tree), vec!["projects/", "  2026/", "Home"]);
        let TreeNode::Folder { pinned, children, .. } = &tree[0] else { panic!() };
        assert!(!pinned, "the ancestor is derived, not pinned");
        assert!(matches!(&children[0], TreeNode::Folder { pinned: true, .. }));
    }

    #[test]
    fn a_folder_can_be_listed_before_a_page() {
        let pages = [meta(A, "Home", None), meta(B, "Own", Some("notes"))];
        let layout = TreeLayout { order: BTreeMap::from([(String::new(), vec![A.into(), "notes/".into()])]), ..Default::default() };
        assert_eq!(labels(&build_tree(&pages, &layout)), vec!["Home", "notes/", "  Own"]);
    }

    // ── Create / delete ──

    #[test]
    fn create_pins_an_empty_folder() {
        let (_tmp, ctx) = ctx();
        folder_create(&ctx, "projects/2026").unwrap();
        assert_eq!(tree_layout(&ctx).unwrap().pinned, vec!["projects/2026"]);
        assert_eq!(labels(&folder_tree(&ctx).unwrap()), vec!["projects/", "  2026/"]);
    }

    #[test]
    fn create_refuses_an_existing_folder() {
        let (_tmp, ctx) = ctx();
        page(&ctx, "Own", Some("notes/rust"));
        assert_eq!(folder_create(&ctx, "notes").unwrap_err(), "folder already exists: notes");
        folder_create(&ctx, "inbox").unwrap();
        assert_eq!(folder_create(&ctx, "inbox").unwrap_err(), "folder already exists: inbox");
    }

    #[test]
    fn create_refuses_an_invalid_path_without_writing() {
        let (_tmp, ctx) = ctx();
        for bad in ["a//b", "/a", "a/", "", "a/./b"] {
            assert!(folder_create(&ctx, bad).is_err(), "{bad:?}");
        }
        assert!(!layout_path(&ctx).exists());
    }

    #[test]
    fn delete_removes_pins_order_and_the_parent_entry() {
        let (_tmp, ctx) = ctx();
        let home = page(&ctx, "Home", None);
        folder_create(&ctx, "inbox").unwrap();
        folder_create(&ctx, "inbox/later").unwrap();
        folder_set_order(&ctx, "", vec!["inbox/".into(), home.clone()]).unwrap();
        folder_set_order(&ctx, "inbox", vec!["later/".into()]).unwrap();

        folder_delete(&ctx, "inbox").unwrap();
        let layout = tree_layout(&ctx).unwrap();
        assert!(layout.pinned.is_empty());
        assert_eq!(layout.order.get("").unwrap(), &vec![home]);
        assert!(!layout.order.contains_key("inbox"));
        assert_eq!(labels(&folder_tree(&ctx).unwrap()), vec!["Home"]);
    }

    #[test]
    fn delete_refuses_a_folder_with_pages() {
        let (_tmp, ctx) = ctx();
        for t in ["One", "Two", "Three"] {
            page(&ctx, t, Some("notes/deep"));
        }
        folder_create(&ctx, "inbox").unwrap();
        let before = std::fs::read_to_string(layout_path(&ctx)).unwrap();
        assert_eq!(folder_delete(&ctx, "notes").unwrap_err(), "folder is not empty: notes (3 pages)");
        assert_eq!(std::fs::read_to_string(layout_path(&ctx)).unwrap(), before);
    }

    #[test]
    fn delete_of_an_unknown_folder_fails() {
        let (_tmp, ctx) = ctx();
        assert_eq!(folder_delete(&ctx, "nothing").unwrap_err(), "folder not found: nothing");
    }

    // ── Order ──

    #[test]
    fn set_order_reorders_and_prunes() {
        let (_tmp, ctx) = ctx();
        let home = page(&ctx, "Home", None);
        page(&ctx, "Own", Some("notes"));
        let stored = folder_set_order(&ctx, "", vec!["gone/".into(), home.clone(), GONE.into(), home.clone(), "notes/".into()]).unwrap();
        assert_eq!(stored, vec![home.clone(), "notes/".to_string()]);
        assert_eq!(labels(&folder_tree(&ctx).unwrap()), vec!["Home", "notes/", "  Own"]);
    }

    #[test]
    fn set_order_rejects_a_malformed_entry_without_writing() {
        let (_tmp, ctx) = ctx();
        page(&ctx, "Own", Some("notes"));
        for bad in ["notes", "a/b/", "/", "not-a-uuid"] {
            let err = folder_set_order(&ctx, "", vec![bad.into()]).unwrap_err();
            assert!(err.starts_with("invalid order entry"), "{bad}: {err}");
        }
        assert!(!layout_path(&ctx).exists());
    }

    #[test]
    fn set_order_of_an_unknown_folder_fails() {
        let (_tmp, ctx) = ctx();
        assert_eq!(folder_set_order(&ctx, "nowhere", vec![]).unwrap_err(), "folder not found: nowhere");
    }

    #[test]
    fn normalize_entry_adds_the_folder_slash() {
        assert_eq!(normalize_entry("notes"), "notes/");
        assert_eq!(normalize_entry("notes/"), "notes/");
        assert_eq!(normalize_entry(A), A);
    }

    // ── Rename ──

    fn layout_with(order: &[(&str, &[&str])], pinned: &[&str]) -> TreeLayout {
        TreeLayout {
            pinned: pinned.iter().map(|s| s.to_string()).collect(),
            order: order.iter().map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect())).collect(),
        }
    }

    #[test]
    fn rename_keeps_the_position_in_the_parent() {
        let mut l = layout_with(&[("", &["inbox/", "notes/"])], &[]);
        assert!(rename_prefix(&mut l, "notes", "journal"));
        assert_eq!(l.order[""], vec!["inbox/", "journal/"]);
    }

    #[test]
    fn rename_carries_nested_order_and_pins() {
        let mut l = layout_with(&[("notes/rust", &[A]), ("notes", &["rust/"]), ("", &["notes/"])], &["notes/empty", "notebook"]);
        rename_prefix(&mut l, "notes", "archive/notes");
        assert_eq!(l.order["archive/notes/rust"], vec![A]);
        assert_eq!(l.order["archive/notes"], vec!["rust/"]);
        assert!(!l.order.contains_key("notes/rust"));
        assert_eq!(l.pinned, vec!["archive/notes/empty", "notebook"], "notebook is not under notes");
        // Moved to another parent: the old entry leaves the root list.
        assert!(l.order[""].is_empty());
    }

    #[test]
    fn rename_onto_an_existing_folder_merges_lists() {
        let mut l = layout_with(&[("a", &[A]), ("b", &[B, A]), ("", &["a/", "b/"])], &[]);
        rename_prefix(&mut l, "a", "b");
        assert_eq!(l.order["b"], vec![B, A, A], "duplicates are dropped on write");
        assert_eq!(l.order[""], vec!["b/"]);
    }

    #[test]
    fn folder_rename_updates_the_layout_and_renames_a_pinned_empty_folder() {
        let (_tmp, ctx) = ctx();
        let own = page(&ctx, "Own", Some("notes"));
        folder_create(&ctx, "inbox").unwrap();
        folder_set_order(&ctx, "", vec!["notes/".into(), "inbox/".into()]).unwrap();
        folder_set_order(&ctx, "notes", vec![own.clone()]).unwrap();

        let report = crate::page::folder_rename(&ctx, "notes", "journal").unwrap();
        assert_eq!(report.changed, vec![own.clone()]);
        let l = tree_layout(&ctx).unwrap();
        assert_eq!(l.order[""], vec!["journal/", "inbox/"]);
        assert_eq!(l.order["journal"], vec![own]);

        let report = crate::page::folder_rename(&ctx, "inbox", "todo").unwrap();
        assert!(report.changed.is_empty());
        let l = tree_layout(&ctx).unwrap();
        assert_eq!(l.pinned, vec!["todo"]);
        assert_eq!(labels(&folder_tree(&ctx).unwrap()), vec!["journal/", "  Own", "todo/"]);
    }

    #[test]
    fn folder_rename_with_a_broken_layout_writes_no_page() {
        let (_tmp, ctx) = ctx();
        let own = page(&ctx, "Own", Some("notes"));
        std::fs::write(layout_path(&ctx), "pinned = 7").unwrap();
        assert!(crate::page::folder_rename(&ctx, "notes", "journal").unwrap_err().contains("tree.toml"));
        let pages = page_list(&ctx).unwrap();
        assert_eq!(pages.iter().find(|p| p.uuid == own).unwrap().virtual_path.as_deref(), Some("notes"));
    }
}
