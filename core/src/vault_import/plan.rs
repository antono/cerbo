//! One pass over the source that decides everything before anything is
//! written (design D1): UUIDs, titles, slugs, rewritten bodies, triples. A dry
//! run reports the plan; a real run writes exactly this plan.

use super::dialect::{Dialect, SpecialKey, dialect_for};
use super::report::{FrontmatterError, ImportReport, RenamedSlug, ReportOntology};
use super::resolve::{FileEntry, Findings, LinkIndex, NoteEntry};
use super::scan::{SourceFile, detect, scan};
use super::triples::{self, Vocab};
use super::{Format, frontmatter, markdown, rewrite};
use oxrdf::Triple;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Everything an import will create, decided up front.
#[derive(Debug)]
pub struct ImportPlan {
    /// Canonical path of the source folder.
    pub source: PathBuf,
    pub format: Format,
    pub ontology_uuid: String,
    pub ontology_title: String,
    /// `cerbo://vocab/<ontology_uuid>/`.
    pub namespace: String,
    /// The prefix to register before uniquifying: the source folder name,
    /// slugified.
    pub prefix_base: String,
    pub(crate) pages: Vec<PlannedPage>,
    pub(crate) attachments: Vec<PlannedAttachment>,
    pub(crate) vocab: Vocab,
    report: ImportReport,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedPage {
    pub uuid: String,
    pub rel: String,
    pub title: String,
    pub slug: String,
    pub virtual_path: Option<String>,
    pub created: String,
    pub modified: String,
    /// The final `page.md`.
    pub content: String,
    /// Frontmatter triples for `meta.ttl`.
    pub triples: Vec<Triple>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedAttachment {
    pub uuid: String,
    pub rel: String,
    pub abs: PathBuf,
    pub name: String,
    pub size: u64,
    pub created: String,
    pub modified: String,
}

/// A note read and split, before links are resolved.
struct ReadNote {
    file: SourceFile,
    entry: NoteEntry,
    slug: String,
    body: String,
    front: frontmatter::Frontmatter,
    /// The raw block, kept only when it failed to parse.
    broken_front: Option<String>,
}

impl ImportPlan {
    /// The report of this plan as a dry run: nothing written, no vault.
    pub fn report(&self) -> ImportReport {
        self.report.clone()
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn attachment_count(&self) -> usize {
        self.attachments.len()
    }

    /// Every object UUID the plan creates, in commit order: attachments, pages,
    /// the ontology.
    pub fn object_uuids(&self) -> Vec<String> {
        self.attachments
            .iter()
            .map(|a| a.uuid.clone())
            .chain(self.pages.iter().map(|p| p.uuid.clone()))
            .chain(std::iter::once(self.ontology_uuid.clone()))
            .collect()
    }

    /// The UUID of the page planned for the source note at `rel`.
    pub fn page_uuid(&self, rel: &str) -> Option<&str> {
        self.pages
            .iter()
            .find(|p| p.rel == rel)
            .map(|p| p.uuid.as_str())
    }
}

/// Detect, scan and plan an import of `source`. Reads the source only.
pub fn plan_import(source: &Path, from: Option<Format>) -> Result<ImportPlan, String> {
    let detection = detect(source, from)?;
    let source = source
        .canonicalize()
        .map_err(|e| format!("Cannot resolve {}: {e}", source.display()))?;
    let dialect = dialect_for(detection.format);
    let scanned = scan(&source, dialect)?;

    let ontology_uuid = Uuid::new_v4();
    let namespace = triples::namespace_for(&ontology_uuid.to_string());
    let folder_name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "vault".to_string());

    let mut renamed = Vec::new();
    let mut frontmatter_errors = Vec::new();
    let notes = read_notes(
        &scanned.files,
        dialect,
        &mut renamed,
        &mut frontmatter_errors,
    )?;
    let files: Vec<FileEntry> = scanned
        .files
        .iter()
        .filter(|f| !f.is_note)
        .map(|f| {
            let (folder, name) = f.rel.rsplit_once('/').unwrap_or(("", &f.rel));
            FileEntry {
                rel: f.rel.clone(),
                folder: folder.to_string(),
                name: name.to_string(),
                uuid: Uuid::new_v4().to_string(),
            }
        })
        .collect();
    let others: Vec<&SourceFile> = scanned.files.iter().filter(|f| !f.is_note).collect();

    let aliases: Vec<Vec<String>> = notes
        .iter()
        .map(|n| triples::aliases(&n.front, dialect))
        .collect();
    let index = LinkIndex::new(
        notes.iter().map(|n| n.entry.clone()).collect(),
        &aliases,
        files,
    );

    let mut findings = Findings::default();
    let mut vocab = Vocab::new(namespace.clone());
    let mut pages = Vec::with_capacity(notes.len());
    for note in &notes {
        let rewritten = rewrite::rewrite_body(&note.body, &note.entry, &index, &mut findings);
        let content = compose(
            &note.entry.title,
            &note.body,
            &rewritten,
            note.broken_front.as_deref(),
        );
        let triples = triples::frontmatter_triples(
            &note.front,
            dialect,
            &note.entry,
            &index,
            &mut findings,
            &mut vocab,
        );
        if declares_type(&note.front, dialect) {
            vocab.class(&note.entry.title, false);
        }
        pages.push(PlannedPage {
            uuid: note.entry.uuid.clone(),
            rel: note.entry.rel.clone(),
            title: note.entry.title.clone(),
            slug: note.slug.clone(),
            virtual_path: Some(note.entry.folder.clone())
                .filter(|f| !f.is_empty() && crate::vault::validate_virtual_path(f).is_ok()),
            created: note.file.created.clone(),
            modified: note.file.modified.clone(),
            content,
            triples,
        });
    }

    let attachments: Vec<PlannedAttachment> = findings
        .used_files
        .iter()
        .map(|&i| {
            let entry = &index.files[i];
            let file = others[i];
            PlannedAttachment {
                uuid: entry.uuid.clone(),
                rel: entry.rel.clone(),
                abs: file.abs.clone(),
                name: entry.name.clone(),
                size: file.size,
                created: file.created.clone(),
                modified: file.modified.clone(),
            }
        })
        .collect();
    let unreferenced: Vec<String> = (0..index.files.len())
        .filter(|&i| !findings.is_used(i))
        .map(|i| index.files[i].rel.clone())
        .collect();

    let ontology_title = format!("{}: {folder_name}", detection.format.display_name());
    let report = ImportReport {
        vault: None,
        source: source.to_string_lossy().into_owned(),
        format: detection.format,
        format_reason: detection.reason,
        dry_run: true,
        pages: pages.len(),
        attachments: attachments.len(),
        attachment_bytes: attachments.iter().map(|a| a.size).sum(),
        relations: findings.relations,
        links: findings.links,
        dropped_fragments: findings.dropped_fragments,
        ontology: ReportOntology {
            uuid: ontology_uuid.to_string(),
            title: ontology_title.clone(),
            namespace: namespace.clone(),
            prefix: None,
        },
        skipped: scanned.skipped,
        unreferenced,
        unresolved: findings.unresolved,
        ambiguous: findings.ambiguous,
        renamed_slugs: renamed,
        frontmatter_errors,
        recovered: 0,
        warnings: Vec::new(),
    };

    Ok(ImportPlan {
        prefix_base: prefix_base(&folder_name, ontology_uuid),
        source,
        format: detection.format,
        ontology_uuid: ontology_uuid.to_string(),
        ontology_title,
        namespace,
        pages,
        attachments,
        vocab,
        report,
    })
}

fn read_notes(
    files: &[SourceFile],
    dialect: &dyn Dialect,
    renamed: &mut Vec<RenamedSlug>,
    errors: &mut Vec<FrontmatterError>,
) -> Result<Vec<ReadNote>, String> {
    let mut used_slugs: HashMap<String, HashSet<String>> = HashMap::new();
    let mut notes = Vec::new();
    for file in files.iter().filter(|f| f.is_note) {
        let bytes = std::fs::read(&file.abs)
            .map_err(|e| format!("Failed to read {}: {e}", file.abs.display()))?;
        let text = String::from_utf8_lossy(&bytes);
        let (raw, body) = frontmatter::split(&text);
        let (front, broken_front) = match raw.map(frontmatter::parse) {
            None => (Vec::new(), None),
            Some(Ok(front)) => (front, None),
            Some(Err(error)) => {
                errors.push(FrontmatterError {
                    note: file.rel.clone(),
                    error,
                });
                (Vec::new(), raw.map(str::to_string))
            }
        };

        let (folder, name) = file.rel.rsplit_once('/').unwrap_or(("", &file.rel));
        let stem = Path::new(name)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| name.to_string());
        let uuid = Uuid::new_v4();
        let title = dialect.title(&stem, body);

        let base = crate::slug::slugify(&stem, uuid);
        let slug = unique_slug(&base, used_slugs.entry(folder.to_string()).or_default());
        if slug != base {
            renamed.push(RenamedSlug {
                note: file.rel.clone(),
                from: base,
                to: slug.clone(),
            });
        }

        notes.push(ReadNote {
            file: file.clone(),
            entry: NoteEntry {
                rel: file.rel.clone(),
                folder: folder.to_string(),
                title,
                uuid: uuid.to_string(),
            },
            slug,
            body: body.to_string(),
            front,
            broken_front,
        });
    }
    Ok(notes)
}

/// `base`, or `base-2`, `base-3`, … — the first not yet used in the folder,
/// kept within the 80-character slug limit.
fn unique_slug(base: &str, used: &mut HashSet<String>) -> String {
    if used.insert(base.to_string()) {
        return base.to_string();
    }
    for n in 2.. {
        let suffix = format!("-{n}");
        let keep = base.len().min(80 - suffix.len());
        let candidate = format!("{}{suffix}", base[..keep].trim_end_matches('-'));
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("some suffix is always free")
}

/// Whether a Tolaria note is a type definition (`type: Type`).
fn declares_type(front: &frontmatter::Frontmatter, dialect: &dyn Dialect) -> bool {
    front.iter().any(|(k, v)| {
        dialect.special_key(k) == Some(SpecialKey::Type)
            && triples::type_names(v).iter().any(|t| t == "Type")
    })
}

/// The final `page.md`: the rewritten body, with `# <title>` in front unless
/// the body already starts with that heading, and a frontmatter block that did
/// not parse kept as a YAML code block right after the title.
fn compose(title: &str, original: &str, rewritten: &str, broken_front: Option<&str>) -> String {
    let starts_with_title = markdown::leading_h1(original).is_some_and(|(text, _)| text == title);
    let heading_end = if starts_with_title {
        markdown::leading_h1(rewritten).map(|(_, end)| end)
    } else {
        None
    };
    let (head, rest) = match heading_end {
        Some(end) => {
            let mut head = rewritten[..end].to_string();
            if !head.ends_with('\n') {
                head.push('\n');
            }
            (head, &rewritten[end..])
        }
        None => (
            format!("# {title}\n"),
            rewritten.trim_start_matches(['\n', '\r']),
        ),
    };
    match broken_front {
        None if heading_end.is_some() => rewritten.to_string(),
        None => format!("{head}\n{rest}"),
        Some(raw) => {
            let longest = raw.split(|c| c != '`').map(str::len).max().unwrap_or(0);
            let fence = "`".repeat(longest.max(2) + 1);
            let raw = raw.trim_end_matches(['\n', '\r']);
            let rest = rest.trim_start_matches(['\n', '\r']);
            format!("{head}\n{fence}yaml\n{raw}\n{fence}\n\n{rest}")
        }
    }
}

/// The source folder name as a prefix: slugified, starting with a letter as
/// `prefix_map::validate_registration` requires.
fn prefix_base(folder_name: &str, uuid: Uuid) -> String {
    let slug = crate::slug::slugify(folder_name, uuid);
    if slug.starts_with(|c: char| c.is_ascii_alphabetic()) {
        slug
    } else {
        format!("import-{slug}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_collisions_get_suffixes() {
        let mut used = HashSet::new();
        assert_eq!(unique_slug("note", &mut used), "note");
        assert_eq!(unique_slug("note", &mut used), "note-2");
        assert_eq!(unique_slug("note", &mut used), "note-3");
        let long = "a".repeat(80);
        assert_eq!(unique_slug(&long, &mut used), long);
        let second = unique_slug(&long, &mut used);
        assert_eq!(second.len(), 80);
        assert!(second.ends_with("-2"));
    }

    #[test]
    fn compose_keeps_existing_h1() {
        assert_eq!(
            compose(
                "Devices",
                "# Devices\n\nbody\n",
                "# Devices\n\nbody\n",
                None
            ),
            "# Devices\n\nbody\n"
        );
    }

    #[test]
    fn compose_prepends_title() {
        assert_eq!(
            compose("Rust Ownership", "\nText\n", "\nText\n", None),
            "# Rust Ownership\n\nText\n"
        );
        // A different H1 is not the title.
        assert_eq!(
            compose("Note", "# Other\n", "# Other\n", None),
            "# Note\n\n# Other\n"
        );
    }

    #[test]
    fn compose_keeps_broken_frontmatter_as_yaml_block() {
        let out = compose(
            "Broken",
            "# Broken\n\nbody\n",
            "# Broken\n\nbody\n",
            Some("status: [unclosed\n"),
        );
        assert_eq!(out, "# Broken\n\n```yaml\nstatus: [unclosed\n```\n\nbody\n");
        let out = compose("b", "body\n", "body\n", Some("a: ```\n"));
        assert_eq!(out, "# b\n\n````yaml\na: ```\n````\n\nbody\n");
    }

    #[test]
    fn prefix_starts_with_a_letter() {
        let uuid = Uuid::new_v4();
        assert_eq!(prefix_base("Braindump", uuid), "braindump");
        assert_eq!(prefix_base("2025 Notes", uuid), "import-2025-notes");
    }
}
