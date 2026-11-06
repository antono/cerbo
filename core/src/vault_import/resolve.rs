//! The link resolution index (design D3), built once from the planned notes
//! and the source's other files, plus what resolving links found.

use super::report::{AmbiguousLink, UnresolvedLink};
use std::collections::{HashMap, HashSet};
use unicode_normalization::UnicodeNormalization;

/// A planned note as links see it.
#[derive(Debug, Clone)]
pub struct NoteEntry {
    pub rel: String,
    pub folder: String,
    pub title: String,
    pub uuid: String,
}

/// A non-note file of the source, a possible attachment. Its UUID is
/// assigned up front and used only if a note references the file.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub rel: String,
    pub folder: String,
    pub name: String,
    pub uuid: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Note(usize),
    File(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub target: Target,
    /// Relative paths of every candidate when more than one matched.
    pub candidates: Option<Vec<String>>,
}

#[derive(Debug, Default)]
pub struct LinkIndex {
    pub notes: Vec<NoteEntry>,
    pub files: Vec<FileEntry>,
    /// Lists, because `A.md` and `a.md` (or `x.md` and `x.markdown`) share a key.
    note_by_path: HashMap<String, Vec<usize>>,
    note_by_stem: HashMap<String, Vec<usize>>,
    note_by_title: HashMap<String, Vec<usize>>,
    note_by_alias: HashMap<String, Vec<usize>>,
    file_by_path: HashMap<String, Vec<usize>>,
    file_by_name: HashMap<String, Vec<usize>>,
}

/// Case- and normalisation-insensitive lookup key.
fn key(s: &str) -> String {
    s.nfc().collect::<String>().to_lowercase()
}

fn strip_md(path: &str) -> &str {
    let lower = path.to_lowercase();
    for ext in [".md", ".markdown"] {
        if lower.ends_with(ext) && lower.len() > ext.len() {
            return &path[..path.len() - ext.len()];
        }
    }
    path
}

/// Whether the last segment of `target` has an extension other than Markdown.
fn has_file_extension(target: &str) -> bool {
    let last = target.rsplit('/').next().unwrap_or(target);
    match last.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() && !ext.contains(' ') => {
            !ext.eq_ignore_ascii_case("md") && !ext.eq_ignore_ascii_case("markdown")
        }
        _ => false,
    }
}

/// `target` resolved against `folder` (`/`-separated, `""` for the root): `.`
/// and `..` segments are applied, a leading `/` starts at the root. `None` when
/// `..` climbs out of the source.
pub fn join(folder: &str, target: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    let (base, target) = match target.strip_prefix('/') {
        Some(rest) => ("", rest),
        None => (folder, target),
    };
    for segment in base.split('/').chain(target.split('/')) {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }
    Some(parts.join("/"))
}

fn push(map: &mut HashMap<String, Vec<usize>>, k: String, i: usize) {
    let list = map.entry(k).or_default();
    if !list.contains(&i) {
        list.push(i);
    }
}

impl LinkIndex {
    /// `aliases[i]` are the Obsidian aliases of `notes[i]`.
    pub fn new(notes: Vec<NoteEntry>, aliases: &[Vec<String>], files: Vec<FileEntry>) -> Self {
        let mut index = LinkIndex::default();
        for (i, note) in notes.iter().enumerate() {
            let without_ext = strip_md(&note.rel);
            push(&mut index.note_by_path, key(without_ext), i);
            let stem = without_ext.rsplit('/').next().unwrap_or(without_ext);
            push(&mut index.note_by_stem, key(stem), i);
            push(&mut index.note_by_title, key(&note.title), i);
            for alias in aliases.get(i).into_iter().flatten() {
                push(&mut index.note_by_alias, key(alias), i);
            }
        }
        for (i, file) in files.iter().enumerate() {
            push(&mut index.file_by_path, key(&file.rel), i);
            push(&mut index.file_by_name, key(&file.name), i);
        }
        index.notes = notes;
        index.files = files;
        index
    }

    /// Resolve the target of `[[target]]` (alias and heading already removed)
    /// written in a note in `folder`: a note by relative path, path suffix
    /// (`[[deep/Note]]`), file stem, title, then alias; a target with a file
    /// extension is a file instead.
    pub fn wikilink(&self, target: &str, folder: &str) -> Option<Resolution> {
        let target = target.trim();
        if target.is_empty() {
            return None;
        }
        // `[[Orange Pi 2.0]]` looks like a file too; it falls through to notes.
        if has_file_extension(target)
            && let Some(found) = self.file(target, folder)
        {
            return Some(found);
        }
        let path = strip_md(target);
        if let Some(found) = self.note_by_joined_path(path, &["", folder], folder) {
            return Some(found);
        }
        if path.contains('/')
            && let Some(found) = self.note_by_path_suffix(path, folder)
        {
            return Some(found);
        }
        let steps: [(&HashMap<String, Vec<usize>>, &str); 3] = [
            (&self.note_by_stem, path),
            (&self.note_by_title, target),
            (&self.note_by_alias, target),
        ];
        for (map, k) in steps {
            if let Some(found) = map.get(&key(k)) {
                return Some(self.pick_note(found, folder));
            }
        }
        None
    }

    /// Resolve a relative Markdown link destination (decoded, fragment removed)
    /// written in a note in `folder`: a note by path, then by the shortest path
    /// Obsidian writes (`Note.md`, `deep/Note.md`); else a file.
    pub fn markdown(&self, dest: &str, folder: &str) -> Option<Resolution> {
        let dest = dest.trim();
        if dest.is_empty() {
            return None;
        }
        if !has_file_extension(dest) {
            let path = strip_md(dest);
            return self
                .note_by_joined_path(path, &[folder, ""], folder)
                .or_else(|| self.note_by_path_suffix(path, folder));
        }
        self.file(dest, folder)
    }

    /// A note at `path` joined to the first of `bases` that has one.
    fn note_by_joined_path(&self, path: &str, bases: &[&str], folder: &str) -> Option<Resolution> {
        bases.iter().find_map(|base| {
            let found = join(base, path).and_then(|p| self.note_by_path.get(&key(&p)))?;
            Some(self.pick_note(found, folder))
        })
    }

    /// Notes whose path (without extension) ends in `path`, the way Obsidian
    /// resolves a partial path. A path that starts at the root or climbs with
    /// `..` is not partial.
    fn note_by_path_suffix(&self, path: &str, folder: &str) -> Option<Resolution> {
        if path.starts_with('/') || path.split('/').any(|s| matches!(s, "" | "." | "..")) {
            return None;
        }
        let k = key(path);
        if !k.contains('/') {
            return self
                .note_by_stem
                .get(&k)
                .map(|found| self.pick_note(found, folder));
        }
        let tail = format!("/{k}");
        let found: Vec<usize> = (0..self.notes.len())
            .filter(|&i| {
                let p = key(strip_md(&self.notes[i].rel));
                p == k || p.ends_with(&tail)
            })
            .collect();
        (!found.is_empty()).then(|| self.pick_note(&found, folder))
    }

    /// A file by path relative to the note, then to the root, then by file name
    /// anywhere in the source — how Obsidian finds `![[photo.png]]`.
    fn file(&self, target: &str, folder: &str) -> Option<Resolution> {
        let found = [folder, ""]
            .iter()
            .find_map(|base| join(base, target).and_then(|p| self.file_by_path.get(&key(&p))))
            .or_else(|| {
                let name = target.rsplit('/').next().unwrap_or(target);
                self.file_by_name.get(&key(name))
            })?;
        let rels: Vec<(&str, &str)> = found
            .iter()
            .map(|&i| (self.files[i].rel.as_str(), self.files[i].folder.as_str()))
            .collect();
        let (chosen, candidates) = pick(&rels, folder);
        Some(Resolution {
            target: Target::File(found[chosen]),
            candidates,
        })
    }

    fn pick_note(&self, found: &[usize], folder: &str) -> Resolution {
        let rels: Vec<(&str, &str)> = found
            .iter()
            .map(|&i| (self.notes[i].rel.as_str(), self.notes[i].folder.as_str()))
            .collect();
        let (chosen, candidates) = pick(&rels, folder);
        Resolution {
            target: Target::Note(found[chosen]),
            candidates,
        }
    }

    pub fn rel_of(&self, target: Target) -> &str {
        match target {
            Target::Note(i) => &self.notes[i].rel,
            Target::File(i) => &self.files[i].rel,
        }
    }

    pub fn uuid_of(&self, target: Target) -> &str {
        match target {
            Target::Note(i) => &self.notes[i].uuid,
            Target::File(i) => &self.files[i].uuid,
        }
    }
}

/// Among `(rel, folder)` candidates, the one whose folder shares the longest
/// prefix with `from`, then the shortest path. Several candidates are returned
/// sorted, for the report.
fn pick(candidates: &[(&str, &str)], from: &str) -> (usize, Option<Vec<String>>) {
    if candidates.len() == 1 {
        return (0, None);
    }
    let segments =
        |s: &str| -> Vec<String> { s.split('/').filter(|p| !p.is_empty()).map(key).collect() };
    let from = segments(from);
    let shared = |folder: &str| {
        segments(folder)
            .iter()
            .zip(&from)
            .take_while(|(a, b)| a == b)
            .count()
    };
    let chosen = (0..candidates.len())
        .min_by(|&a, &b| {
            let (ra, fa) = candidates[a];
            let (rb, fb) = candidates[b];
            shared(fb)
                .cmp(&shared(fa))
                .then(ra.matches('/').count().cmp(&rb.matches('/').count()))
                .then(ra.len().cmp(&rb.len()))
                .then(ra.cmp(rb))
        })
        .unwrap_or(0);
    let mut all: Vec<String> = candidates.iter().map(|(r, _)| r.to_string()).collect();
    all.sort();
    (chosen, Some(all))
}

/// What resolving links found, gathered over the whole plan.
#[derive(Debug, Default)]
pub struct Findings {
    /// Indexes into [`LinkIndex::files`] of referenced files, in first-use order.
    pub used_files: Vec<usize>,
    used_set: HashSet<usize>,
    pub unresolved: Vec<UnresolvedLink>,
    pub ambiguous: Vec<AmbiguousLink>,
    /// Links rewritten to `cerbo://` in page bodies.
    pub links: usize,
    /// Heading fragments dropped from rewritten links.
    pub dropped_fragments: usize,
    /// Object-valued frontmatter triples.
    pub relations: usize,
}

impl Findings {
    /// Note a successful resolution of `link` in `note`.
    pub fn resolved(&mut self, index: &LinkIndex, note: &str, link: &str, res: &Resolution) {
        if let Target::File(i) = res.target
            && self.used_set.insert(i)
        {
            self.used_files.push(i);
        }
        if let Some(candidates) = &res.candidates {
            self.ambiguous.push(AmbiguousLink {
                note: note.to_string(),
                link: link.to_string(),
                chosen: index.rel_of(res.target).to_string(),
                candidates: candidates.clone(),
            });
        }
    }

    pub fn unresolved(&mut self, note: &str, link: &str, property: Option<&str>) {
        self.unresolved.push(UnresolvedLink {
            note: note.to_string(),
            link: link.to_string(),
            property: property.map(str::to_string),
        });
    }

    pub fn is_used(&self, file: usize) -> bool {
        self.used_set.contains(&file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(rel: &str, title: &str) -> NoteEntry {
        let folder = rel
            .rsplit_once('/')
            .map(|(f, _)| f)
            .unwrap_or("")
            .to_string();
        NoteEntry {
            rel: rel.into(),
            folder,
            title: title.into(),
            uuid: format!("uuid-{rel}"),
        }
    }

    fn file(rel: &str) -> FileEntry {
        let (folder, name) = rel.rsplit_once('/').unwrap_or(("", rel));
        FileEntry {
            rel: rel.into(),
            folder: folder.into(),
            name: name.into(),
            uuid: format!("uuid-{rel}"),
        }
    }

    fn index() -> LinkIndex {
        LinkIndex::new(
            vec![
                note("devices.md", "Devices"),
                note("desktop-pc.md", "Desktop PC"),
                note("notes/index.md", "index"),
                note("archive/index.md", "index"),
                note("Rust Ownership.md", "Rust Ownership"),
                note("journals/2025-04-27.md", "2025-04-27"),
            ],
            &[
                vec![],
                vec![],
                vec![],
                vec![],
                vec!["Borrow checker".into()],
                vec![],
            ],
            vec![
                file("attachments/photo.png"),
                file("attachments/pdf/manual.pdf"),
            ],
        )
    }

    fn rel(index: &LinkIndex, r: Option<Resolution>) -> String {
        index.rel_of(r.expect("resolves").target).to_string()
    }

    #[test]
    fn by_relative_path() {
        let ix = index();
        assert_eq!(
            rel(&ix, ix.wikilink("journals/2025-04-27", "")),
            "journals/2025-04-27.md"
        );
        assert_eq!(
            rel(&ix, ix.wikilink("journals/2025-04-27.md", "")),
            "journals/2025-04-27.md"
        );
        // Path wins over a stem match elsewhere.
        assert_eq!(
            rel(&ix, ix.wikilink("notes/index", "archive")),
            "notes/index.md"
        );
    }

    #[test]
    fn by_stem_case_insensitive() {
        let ix = index();
        assert_eq!(rel(&ix, ix.wikilink("Desktop-PC", "")), "desktop-pc.md");
    }

    #[test]
    fn by_title() {
        let ix = index();
        assert_eq!(rel(&ix, ix.wikilink("Desktop PC", "")), "desktop-pc.md");
        assert_eq!(rel(&ix, ix.wikilink("desktop pc", "")), "desktop-pc.md");
    }

    #[test]
    fn by_alias() {
        let ix = index();
        assert_eq!(
            rel(&ix, ix.wikilink("borrow checker", "")),
            "Rust Ownership.md"
        );
    }

    #[test]
    fn ambiguous_prefers_nearest_folder() {
        let ix = index();
        let r = ix.wikilink("index", "notes/deep").unwrap();
        assert_eq!(ix.rel_of(r.target), "notes/index.md");
        assert_eq!(
            r.candidates.unwrap(),
            ["archive/index.md", "notes/index.md"]
        );
        let r = ix.wikilink("index", "archive").unwrap();
        assert_eq!(ix.rel_of(r.target), "archive/index.md");
        // From the root neither folder is nearer: the shorter path wins.
        let r = ix.wikilink("index", "").unwrap();
        assert_eq!(ix.rel_of(r.target), "notes/index.md");
    }

    #[test]
    fn unresolved() {
        assert!(index().wikilink("Missing Note", "").is_none());
    }

    #[test]
    fn files_by_path_then_name() {
        let ix = index();
        assert_eq!(
            rel(&ix, ix.wikilink("photo.png", "notes")),
            "attachments/photo.png"
        );
        assert_eq!(
            rel(&ix, ix.markdown("attachments/pdf/manual.pdf", "")),
            "attachments/pdf/manual.pdf"
        );
        assert_eq!(
            rel(
                &ix,
                ix.markdown("../attachments/pdf/manual.pdf", "journals")
            ),
            "attachments/pdf/manual.pdf"
        );
        assert!(ix.markdown("missing.pdf", "").is_none());
    }

    #[test]
    fn markdown_note_links() {
        let ix = index();
        assert_eq!(
            rel(&ix, ix.markdown("../devices.md", "journals")),
            "devices.md"
        );
        assert_eq!(
            rel(&ix, ix.markdown("Rust Ownership.md", "")),
            "Rust Ownership.md"
        );
        assert!(ix.markdown("../../devices.md", "journals").is_none());
    }

    #[test]
    fn markdown_shortest_path_links() {
        let ix = index();
        // Obsidian's default "shortest path when possible" format.
        assert_eq!(
            rel(&ix, ix.markdown("2025-04-27.md", "")),
            "journals/2025-04-27.md"
        );
        assert_eq!(
            rel(&ix, ix.markdown("pdf/manual.pdf", "")),
            "attachments/pdf/manual.pdf"
        );
        let r = ix.markdown("index.md", "").unwrap();
        assert_eq!(
            r.candidates.unwrap(),
            ["archive/index.md", "notes/index.md"]
        );
        assert!(ix.markdown("../index.md", "journals").is_none());
    }

    #[test]
    fn partial_path_wikilink() {
        let ix = LinkIndex::new(
            vec![
                note("notes/deep/Deep.md", "Deep"),
                note("Welcome.md", "Welcome"),
            ],
            &[],
            vec![],
        );
        assert_eq!(rel(&ix, ix.wikilink("deep/Deep", "")), "notes/deep/Deep.md");
        assert_eq!(
            rel(&ix, ix.markdown("deep/Deep.md", "")),
            "notes/deep/Deep.md"
        );
        assert!(ix.wikilink("eep/Deep", "").is_none(), "whole segments only");
    }

    #[test]
    fn colliding_paths_are_ambiguous() {
        let ix = LinkIndex::new(
            vec![
                note("notes/A.md", "A"),
                note("notes/a.md", "a"),
                note("Note.md", "Note"),
                note("Note.markdown", "Note"),
            ],
            &[],
            vec![file("x/P.png"), file("x/p.png")],
        );
        let r = ix.wikilink("notes/a", "").unwrap();
        assert_eq!(r.candidates.unwrap(), ["notes/A.md", "notes/a.md"]);
        let r = ix.markdown("notes/a.md", "").unwrap();
        assert!(r.candidates.is_some());
        let r = ix.wikilink("Note", "").unwrap();
        assert_eq!(r.candidates.unwrap(), ["Note.markdown", "Note.md"]);
        let r = ix.markdown("x/p.png", "").unwrap();
        assert_eq!(r.candidates.unwrap(), ["x/P.png", "x/p.png"]);
    }

    #[test]
    fn join_paths() {
        assert_eq!(join("a/b", "../c").as_deref(), Some("a/c"));
        assert_eq!(join("a", "/c").as_deref(), Some("c"));
        assert_eq!(join("", "./c").as_deref(), Some("c"));
        assert_eq!(join("", "../c"), None);
    }
}
