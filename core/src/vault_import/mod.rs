//! Importing a foreign Markdown vault (Obsidian, Tolaria) into a Cerbo vault.
//!
//! The pipeline is scan → plan → stage → commit → index (design D1):
//!
//! - [`plan_import`] detects the format, walks the source and decides every
//!   UUID, title, slug, rewritten body and triple. It only reads the source.
//!   Its [`ImportPlan::report`] is the dry run.
//! - [`run_import`] writes that plan into a vault: staged under
//!   `.cerbo/import-staging/`, then renamed into `.cerbo/objects/`, then the
//!   prefix map, then one index catch-up.
//!
//! The source folder is never written. Picking or creating the target vault is
//! the caller's job; [`crate::vault::vault_create`] creates and registers one.

mod dialect;
mod frontmatter;
mod markdown;
mod plan;
mod report;
mod resolve;
mod rewrite;
mod scan;
mod triples;
mod write;

pub use plan::{ImportPlan, plan_import};
pub use report::{
    AmbiguousLink, FrontmatterError, ImportReport, RenamedSlug, ReportOntology, ReportVault,
    UnresolvedLink,
};
pub use scan::{Detection, UNDETECTABLE, detect};
pub use triples::NS_VOCAB;
pub use write::{
    ImportOptions, check_repeat, check_target_outside_source, file_url, recover_staging, run_import,
};

use serde::{Deserialize, Serialize};

/// A source vault format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Obsidian,
    Tolaria,
}

impl Format {
    /// As written on the command line and in `cerbo:importFormat`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::Obsidian => "obsidian",
            Format::Tolaria => "tolaria",
        }
    }

    /// As written in titles: `Tolaria: Braindump`.
    pub fn display_name(&self) -> &'static str {
        match self {
            Format::Obsidian => "Obsidian",
            Format::Tolaria => "Tolaria",
        }
    }
}

impl std::str::FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "obsidian" => Ok(Format::Obsidian),
            "tolaria" => Ok(Format::Tolaria),
            other => Err(format!(
                "unknown vault format {other:?}, expected obsidian or tolaria"
            )),
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
// Fixtures write files directly; the atomic-write rule guards vault code, not setup.
#[allow(clippy::disallowed_methods)]
pub(crate) mod tests {
    use super::*;
    use crate::object::ObjectMeta;
    use crate::{CerboContext, VaultContext};
    use sha2::{Digest, Sha256};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    pub fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/import")
            .join(name)
    }

    pub fn obsidian_fixture() -> PathBuf {
        fixture("obsidian")
    }

    pub fn tolaria_fixture() -> PathBuf {
        fixture("tolaria")
    }

    /// An empty vault plus a private cache, so no test touches the real
    /// `$XDG_CACHE_HOME`.
    struct TestVault {
        _tmp: TempDir,
        ctx: VaultContext,
    }

    impl TestVault {
        fn new() -> Self {
            let tmp = TempDir::new().unwrap();
            let vault_path = tmp.path().join("vault");
            crate::vault::vault_init(&tmp.path().join("cache"), &vault_path, false).unwrap();
            let ctx = VaultContext {
                vault_path,
                global: CerboContext {
                    config_dir: tmp.path().join("config"),
                    cache_dir: tmp.path().join("cache"),
                },
            };
            Self { _tmp: tmp, ctx }
        }

        fn cerbo(&self) -> CerboContext {
            CerboContext {
                config_dir: self.ctx.cerbo_dir(),
                cache_dir: self.ctx.global.cache_dir.clone(),
            }
        }

        fn import(&self, source: &Path, force: bool) -> Result<(ImportPlan, ImportReport), String> {
            let plan = plan_import(source, None)?;
            let report = run_import(&self.ctx, &plan, ImportOptions { force })?;
            Ok((plan, report))
        }

        fn page_titles(&self) -> Vec<String> {
            let mut titles: Vec<String> = crate::page::page_list(&self.cerbo())
                .unwrap()
                .into_iter()
                .map(|p| p.title)
                .collect();
            titles.sort();
            titles
        }

        fn object_dirs(&self) -> Vec<String> {
            let mut names: Vec<String> = fs::read_dir(self.ctx.objects_dir())
                .unwrap()
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        }

        fn meta(&self, uuid: &str) -> String {
            fs::read_to_string(self.ctx.object_path(uuid).join("meta.ttl")).unwrap()
        }

        fn page_md(&self, uuid: &str) -> String {
            fs::read_to_string(self.ctx.object_path(uuid).join("page.md")).unwrap()
        }
    }

    fn uuid_of(plan: &ImportPlan, rel: &str) -> String {
        plan.page_uuid(rel)
            .unwrap_or_else(|| panic!("no page for {rel}"))
            .to_string()
    }

    // ── Plan (dry run) ────────────────────────────────────────────────────────

    #[test]
    fn dry_run_writes_nothing_and_reports() {
        let plan = plan_import(&tolaria_fixture(), None).unwrap();
        let report = plan.report();
        assert!(report.dry_run);
        assert_eq!(report.format, Format::Tolaria);
        assert_eq!(report.pages, plan.page_count());
        assert_eq!(report.attachments, plan.attachment_count());
        assert_eq!(report.skipped, ["views/devices.yml"]);
        assert!(
            report
                .unreferenced
                .contains(&"attachments/firmware.zip".to_string()),
            "{:?}",
            report.unreferenced
        );
        assert_eq!(report.ontology.title, "Tolaria: tolaria");
        assert!(report.ontology.namespace.starts_with("cerbo://vocab/"));
        assert!(
            !report
                .ontology
                .namespace
                .starts_with(crate::rdf::NS_OBJECTS)
        );
        assert_eq!(report.frontmatter_errors.len(), 1);
        assert_eq!(report.frontmatter_errors[0].note, "broken.md");
    }

    #[test]
    fn undetectable_source_plans_nothing() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(plan_import(tmp.path(), None).unwrap_err(), UNDETECTABLE);
    }

    // ── Notes become pages ────────────────────────────────────────────────────

    #[test]
    fn tolaria_import_maps_notes_to_pages() {
        let vault = TestVault::new();
        let (plan, report) = vault.import(&tolaria_fixture(), false).unwrap();
        assert!(!report.dry_run);
        assert_eq!(report.ontology.prefix.as_deref(), Some("tolaria"));

        // Nested note: virtual path and slug; type → second rdf:type.
        let journal = uuid_of(&plan, "journals/2025-04-27.md");
        let meta =
            ObjectMeta::read_from_file(&vault.ctx.object_path(&journal).join("meta.ttl")).unwrap();
        assert_eq!(meta.virtual_path.as_deref(), Some("journals"));
        assert_eq!(meta.slug.as_deref(), Some("2025-04-27"));
        assert_eq!(meta.object_type, crate::object::ObjectType::Product);
        let ttl = vault.meta(&journal);
        let ns = &plan.namespace;
        let triples = crate::rdf::parse(&ttl).unwrap();
        let types: Vec<String> = triples
            .iter()
            .filter(|t| t.predicate == crate::rdf::rdf_type())
            .map(|t| t.object.to_string())
            .collect();
        assert!(
            types.contains(&"<https://schema.org/NoteDigitalDocument>".to_string()),
            "{types:?}"
        );
        assert!(types.contains(&format!("<{ns}Journal>")), "{types:?}");
        let date = format!("\"2025-04-27\"^^<{}date>", crate::rdf::NS_XSD);
        assert!(
            triples.iter().any(|t| t.object.to_string() == date),
            "{ttl}"
        );

        // Non-ASCII file name → transliterated slug.
        let fil = uuid_of(&plan, "филаменты.md");
        let meta =
            ObjectMeta::read_from_file(&vault.ctx.object_path(&fil).join("meta.ttl")).unwrap();
        let slug = meta.slug.unwrap();
        assert!(slug.is_ascii() && !slug.is_empty(), "{slug}");
        assert_eq!(slug, crate::slug::slugify("филаменты", uuid::Uuid::nil()));

        // Title from H1, H1 appears once; frontmatter gone from the body.
        let devices = uuid_of(&plan, "devices.md");
        let md = vault.page_md(&devices);
        assert!(md.starts_with("# Devices\n"), "{md}");
        assert_eq!(md.matches("# Devices").count(), 1);
        assert!(!md.contains("_favorite"));
        // Missing H1 → stem as title, prepended.
        assert!(
            vault
                .page_md(&uuid_of(&plan, "no-title.md"))
                .starts_with("# no-title\n")
        );
    }

    #[test]
    fn tolaria_links_relations_and_literals() {
        let vault = TestVault::new();
        let (plan, report) = vault.import(&tolaria_fixture(), false).unwrap();
        let ns = &plan.namespace;
        let desktop = uuid_of(&plan, "desktop-pc.md");
        let devices = uuid_of(&plan, "devices.md");
        let printing = uuid_of(&plan, "3d-printing.md");
        let opi = uuid_of(&plan, "orange-pi-zero-2w-2g-octoprint.md");

        // [[Desktop PC|my PC]] by title with alias; [[orange-pi-…]] by stem.
        let md = vault.page_md(&devices);
        assert!(md.contains(&format!("[my PC](cerbo://{desktop})")), "{md}");
        let md = vault.page_md(&printing);
        assert!(
            md.contains(&format!(
                "[Orange PI Zero 2W 2G - Octoprint](cerbo://{opi})"
            )),
            "{md}"
        );
        // Relative Markdown link to a note.
        let md = vault.page_md(&uuid_of(&plan, "journals/2025-04-27.md"));
        assert!(md.contains(&format!("[see](cerbo://{devices})")), "{md}");

        // Relationships, literals, dropped app state.
        let triples = crate::rdf::parse(&vault.meta(&desktop)).unwrap();
        let has = |p: &str, o: &str| {
            triples
                .iter()
                .any(|t| t.predicate.as_str() == p && t.object.to_string() == o)
        };
        assert!(has(
            &format!("{ns}belongs_to"),
            &format!("<cerbo://objects/{devices}>")
        ));
        assert!(has(
            &format!("{ns}related_to"),
            &format!("<cerbo://objects/{printing}>")
        ));
        assert!(has(
            &format!("{ns}related_to"),
            &format!("<cerbo://objects/{opi}>")
        ));
        assert!(has(&format!("{ns}status"), "\"Active\""));
        assert!(has(&format!("{ns}url"), "\"https://example.com\""));
        assert!(!vault.meta(&devices).contains("_favorite"));
        assert!(report.relations >= 4, "{}", report.relations);

        // Unresolved frontmatter link → literal, reported with its key.
        let opi_ttl = vault.meta(&opi);
        assert!(opi_ttl.contains("[[Missing Thing]]"), "{opi_ttl}");
        assert!(
            report
                .unresolved
                .iter()
                .any(|u| u.link == "[[Missing Thing]]"
                    && u.property.as_deref() == Some("related_to"))
        );
        assert!(
            report
                .unresolved
                .iter()
                .any(|u| u.link == "[[Voron Config]]" && u.note == "3d-printing.md")
        );
    }

    #[test]
    fn tolaria_attachment_and_type_declaration() {
        let vault = TestVault::new();
        let (plan, report) = vault.import(&tolaria_fixture(), false).unwrap();
        assert_eq!(report.attachments, 2, "{report:#?}");
        let opi = uuid_of(&plan, "orange-pi-zero-2w-2g-octoprint.md");
        let md = vault.page_md(&opi);
        let pdf = plan
            .attachments
            .iter()
            .find(|a| a.name == "BIGTREETECH-Octopus-1.1-color-PIN.pdf")
            .unwrap();
        assert!(
            md.contains(&format!("[manual](cerbo://{})", pdf.uuid)),
            "{md}"
        );
        let attachment_dir = vault.ctx.object_path(&pdf.uuid);
        assert_eq!(
            fs::read(attachment_dir.join(&pdf.name)).unwrap(),
            fs::read(tolaria_fixture().join(&pdf.rel)).unwrap()
        );
        let meta = ObjectMeta::read_from_file(&attachment_dir.join("meta.ttl")).unwrap();
        assert_eq!(meta.object_type, crate::object::ObjectType::Attachment);
        assert_eq!(meta.mime_type.as_deref(), Some("application/pdf"));
        assert!(!attachment_dir.join("page.md").exists());
        // Listed as the page's attachment.
        let listed = crate::object::attachment_list(&vault.cerbo(), &opi).unwrap();
        assert!(listed.contains(&pdf.uuid));

        // Ontology: title, source, format, class declared by `type: Type`.
        let onto = ObjectMeta::read_from_file(
            &vault.ctx.object_path(&plan.ontology_uuid).join("meta.ttl"),
        )
        .unwrap();
        assert_eq!(onto.object_type, crate::object::ObjectType::Ontology);
        assert_eq!(onto.title, "Tolaria: tolaria");
        assert_eq!(
            onto.original_url.as_deref(),
            Some(file_url(&plan.source).as_str())
        );
        let ttl = vault.meta(&plan.ontology_uuid);
        assert!(ttl.contains("importFormat"), "{ttl}");
        assert!(
            ttl.contains(&format!("<{}Device>", plan.namespace)),
            "{ttl}"
        );
        assert!(ttl.contains("\"Device\""), "{ttl}");
        let map = crate::prefix_map::PrefixMap::load_vault(&vault.ctx).unwrap();
        assert_eq!(map.namespace("tolaria"), Some(plan.namespace.as_str()));
        assert_eq!(
            map.prefixes["tolaria"].ontology.as_deref(),
            Some(plan.ontology_uuid.as_str())
        );
    }

    #[test]
    fn broken_frontmatter_is_kept_as_yaml_block() {
        let vault = TestVault::new();
        let (plan, _) = vault.import(&tolaria_fixture(), false).unwrap();
        let md = vault.page_md(&uuid_of(&plan, "broken.md"));
        assert!(md.starts_with("# Broken\n\n```yaml\n"), "{md}");
    }

    #[test]
    fn obsidian_import() {
        let vault = TestVault::new();
        let (plan, report) = vault.import(&obsidian_fixture(), false).unwrap();
        assert_eq!(report.format, Format::Obsidian);
        assert_eq!(report.skipped, ["plan.canvas"]);
        assert!(
            report
                .unreferenced
                .contains(&"attachments/unused.pdf".to_string())
        );

        // Title from file name, H1 prepended.
        let rust = uuid_of(&plan, "Rust Ownership.md");
        let md = vault.page_md(&rust);
        assert!(md.starts_with("# Rust Ownership\n"), "{md}");
        let triples = crate::rdf::parse(&vault.meta(&rust)).unwrap();
        let has = |p: &str, o: &str| {
            triples
                .iter()
                .any(|t| t.predicate.as_str() == p && t.object.to_string() == o)
        };
        assert!(has("https://schema.org/keywords", "\"rust\""));
        assert!(has("https://schema.org/keywords", "\"lang\""));
        assert!(has("https://schema.org/alternateName", "\"Ownership\""));
        assert!(has(
            "https://schema.org/alternateName",
            "\"Borrow checker\""
        ));

        let welcome = vault.page_md(&uuid_of(&plan, "Welcome.md"));
        // Embed → one shared attachment.
        let photo = plan
            .attachments
            .iter()
            .find(|a| a.name == "photo.png")
            .unwrap();
        assert!(
            welcome.contains(&format!("![photo.png](cerbo://{})", photo.uuid)),
            "{welcome}"
        );
        let other = vault.page_md(&uuid_of(&plan, "notes/Other Note.md"));
        assert!(
            other.contains(&format!("cerbo://{}", photo.uuid)),
            "{other}"
        );
        assert_eq!(
            plan.attachments
                .iter()
                .filter(|a| a.name == "photo.png")
                .count(),
            1
        );
        // Alias, heading, alias resolution, code untouched, unresolved kept.
        assert!(
            welcome.contains(&format!("[ownership rules](cerbo://{rust})")),
            "{welcome}"
        );
        // Resolved by alias; the text is the page title, as for any link.
        assert!(
            other.contains(&format!("[Rust Ownership](cerbo://{rust})")),
            "{other}"
        );
        assert!(
            welcome.contains("```\n[[Rust Ownership]]\n```"),
            "{welcome}"
        );
        assert!(welcome.contains("[[Missing Note]]"));
        assert!(report.dropped_fragments >= 1);
        // Ambiguous [[index]] from notes/deep resolves to notes/index.md.
        let deep = vault.page_md(&uuid_of(&plan, "notes/deep/Deep.md"));
        assert!(
            deep.contains(&format!("cerbo://{}", uuid_of(&plan, "notes/index.md"))),
            "{deep}"
        );
        assert!(
            report
                .ambiguous
                .iter()
                .any(|a| a.note == "notes/deep/Deep.md" && a.chosen == "notes/index.md")
        );
        // Slug collision in one folder.
        assert_eq!(report.renamed_slugs.len(), 1, "{:?}", report.renamed_slugs);
        assert_eq!(report.renamed_slugs[0].to, "slug-test-2");
    }

    // ── Repeat imports ────────────────────────────────────────────────────────

    #[test]
    fn repeat_import_is_refused_then_forced() {
        let vault = TestVault::new();
        let (first, _) = vault.import(&tolaria_fixture(), false).unwrap();
        let before = vault.object_dirs();

        let err = vault.import(&tolaria_fixture(), false).unwrap_err();
        assert!(err.contains("already imported"), "{err}");
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        assert!(err.contains(&today), "{err}");
        assert_eq!(
            vault.object_dirs(),
            before,
            "a refused import writes nothing"
        );

        let (second, report) = vault.import(&tolaria_fixture(), true).unwrap();
        assert_eq!(report.ontology.prefix.as_deref(), Some("tolaria-2"));
        assert_ne!(first.namespace, second.namespace);
        // The first copy is untouched.
        for uuid in &before {
            assert!(vault.ctx.object_path(uuid).is_dir());
        }
        assert_eq!(vault.object_dirs().len(), before.len() * 2);
    }

    #[test]
    fn repeat_import_in_another_format_is_refused() {
        let vault = TestVault::new();
        vault.import(&obsidian_fixture(), false).unwrap();
        let before = vault.object_dirs();
        let plan = plan_import(&obsidian_fixture(), Some(Format::Tolaria)).unwrap();
        let err = run_import(&vault.ctx, &plan, ImportOptions::default()).unwrap_err();
        assert!(
            err.contains("already imported") && err.contains("as obsidian"),
            "{err}"
        );
        assert_eq!(vault.object_dirs(), before);
    }

    #[test]
    fn prefix_taken_gets_suffix() {
        let vault = TestVault::new();
        crate::prefix_map::register_prefix(
            &vault.ctx.cerbo_dir(),
            "tolaria",
            "https://example.org/x#",
            None,
        )
        .unwrap();
        let (_, report) = vault.import(&tolaria_fixture(), false).unwrap();
        assert_eq!(report.ontology.prefix.as_deref(), Some("tolaria-2"));
    }

    // ── All-or-nothing ────────────────────────────────────────────────────────

    #[test]
    fn staged_turtle_is_valid() {
        let vault = TestVault::new();
        let (plan, _) = vault.import(&tolaria_fixture(), false).unwrap();
        for uuid in plan.object_uuids() {
            let meta = vault.ctx.object_path(&uuid).join("meta.ttl");
            ObjectMeta::read_from_file(&meta).unwrap_or_else(|e| panic!("{uuid}: {e}"));
        }
        assert!(!vault.ctx.cerbo_dir().join("import-staging").exists());
    }

    #[test]
    fn interrupted_commit_is_taken_back_on_next_run() {
        let vault = TestVault::new();
        let existing = crate::page::page_create(&vault.cerbo(), "Existing".into()).unwrap();
        let titles_before = vault.page_titles();
        let map_before =
            fs::read_to_string(vault.ctx.cerbo_dir().join("ontology-map.json")).unwrap();

        let plan = plan_import(&tolaria_fixture(), None).unwrap();
        write::CRASH_AFTER_RENAMES.with(|c| c.set(Some(5)));
        let crashed = run_import(&vault.ctx, &plan, ImportOptions::default());
        write::CRASH_AFTER_RENAMES.with(|c| c.set(None));
        assert!(crashed.is_err());
        // Five objects went live before the crash; the staging folder remains.
        assert_eq!(vault.object_dirs().len(), 1 + 5);
        assert!(vault.ctx.cerbo_dir().join("import-staging").exists());

        // The next run cleans up first; the vault is as it was.
        let recovered = recover_staging(&vault.ctx).unwrap();
        assert_eq!(recovered, 5);
        assert_eq!(vault.page_titles(), titles_before);
        assert_eq!(vault.object_dirs(), [existing]);
        assert!(!vault.ctx.cerbo_dir().join("import-staging").exists());
        assert_eq!(
            fs::read_to_string(vault.ctx.cerbo_dir().join("ontology-map.json")).unwrap(),
            map_before
        );

        // And a fresh import then goes through, reporting nothing recovered.
        let (_, report) = vault.import(&tolaria_fixture(), false).unwrap();
        assert_eq!(report.recovered, 0);
    }

    #[test]
    fn run_import_recovers_before_importing() {
        let vault = TestVault::new();
        let plan = plan_import(&tolaria_fixture(), None).unwrap();
        write::CRASH_AFTER_RENAMES.with(|c| c.set(Some(3)));
        let _ = run_import(&vault.ctx, &plan, ImportOptions::default());
        write::CRASH_AFTER_RENAMES.with(|c| c.set(None));

        let (plan, report) = vault.import(&tolaria_fixture(), false).unwrap();
        assert_eq!(report.recovered, 3);
        assert_eq!(vault.object_dirs().len(), plan.object_uuids().len());
    }

    #[test]
    fn failed_rollback_keeps_the_manifest_for_the_next_run() {
        let vault = TestVault::new();
        let plan = plan_import(&tolaria_fixture(), None).unwrap();
        // The commit fails after every rename, and the trash cannot be made.
        let trash = vault.ctx.cerbo_dir().join("trash");
        fs::write(&trash, "in the way").unwrap();
        write::FAIL_REGISTRATION.with(|c| c.set(true));
        let err = run_import(&vault.ctx, &plan, ImportOptions::default()).unwrap_err();
        write::FAIL_REGISTRATION.with(|c| c.set(false));
        assert!(err.contains("next import"), "{err}");
        let staging = vault
            .ctx
            .cerbo_dir()
            .join("import-staging")
            .join(&plan.ontology_uuid);
        assert!(staging.join("manifest.json").is_file());
        assert_eq!(vault.object_dirs().len(), plan.object_uuids().len());

        // Once the trash can be made, the next run takes them all back.
        fs::remove_file(&trash).unwrap();
        let (plan, report) = vault.import(&tolaria_fixture(), false).unwrap();
        assert_eq!(report.recovered, plan.object_uuids().len());
        assert_eq!(vault.object_dirs().len(), plan.object_uuids().len());
    }

    #[test]
    fn finished_import_is_never_taken_back() {
        let vault = TestVault::new();
        let plan = plan_import(&tolaria_fixture(), None).unwrap();
        // The process dies right after the prefix map is saved…
        write::CRASH_AFTER_REGISTRATION.with(|c| c.set(true));
        assert!(run_import(&vault.ctx, &plan, ImportOptions::default()).is_err());
        write::CRASH_AFTER_REGISTRATION.with(|c| c.set(false));
        let objects = vault.object_dirs();
        assert_eq!(objects.len(), plan.object_uuids().len());
        // …and the prefix is renamed before the next run.
        let mut map = crate::prefix_map::PrefixMap::load_vault(&vault.ctx).unwrap();
        let entry = map.prefixes.remove("tolaria").unwrap();
        map.prefixes.insert("renamed".into(), entry);
        map.save(&vault.ctx.cerbo_dir()).unwrap();

        assert_eq!(recover_staging(&vault.ctx).unwrap(), 0);
        assert_eq!(vault.object_dirs(), objects);
        assert!(!vault.ctx.cerbo_dir().join("import-staging").exists());
    }

    #[test]
    fn stray_file_in_staging_does_not_block_imports() {
        let vault = TestVault::new();
        let root = vault.ctx.cerbo_dir().join("import-staging");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(".DS_Store"), "x").unwrap();
        assert_eq!(recover_staging(&vault.ctx).unwrap(), 0);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(".DS_Store"), "x").unwrap();
        vault.import(&tolaria_fixture(), false).unwrap();
        assert!(!root.exists());
    }

    #[test]
    fn concurrent_import_is_refused() {
        let vault = TestVault::new();
        let plan = plan_import(&tolaria_fixture(), None).unwrap();
        let held = write::import_lock(&vault.ctx).unwrap();
        let err = run_import(&vault.ctx, &plan, ImportOptions::default()).unwrap_err();
        assert!(err.contains("Another import is running"), "{err}");
        assert!(vault.object_dirs().is_empty());
        drop(held);
        run_import(&vault.ctx, &plan, ImportOptions::default()).unwrap();
    }

    #[test]
    fn failed_staging_leaves_vault_unchanged() {
        let vault = TestVault::new();
        let titles_before = vault.page_titles();
        let plan = plan_import(&tolaria_fixture(), None).unwrap();
        // A file where the staging folder belongs: staging fails.
        fs::write(vault.ctx.cerbo_dir().join("import-staging"), "in the way").unwrap();
        assert!(run_import(&vault.ctx, &plan, ImportOptions::default()).is_err());
        assert!(vault.object_dirs().is_empty());
        assert_eq!(vault.page_titles(), titles_before);
        assert!(
            crate::prefix_map::PrefixMap::load_vault(&vault.ctx)
                .unwrap()
                .prefixes
                .is_empty()
        );
    }

    // ── Index ─────────────────────────────────────────────────────────────────

    #[test]
    fn backlinks_work_right_after_import() {
        let vault = TestVault::new();
        // An index that exists before the import must see the new pages.
        let seed = crate::page::page_create(&vault.cerbo(), "Seed".into()).unwrap();
        crate::semantic::backlinks(&vault.ctx, &seed, crate::semantic::IndexOptions::default())
            .unwrap();

        let (plan, report) = vault.import(&tolaria_fixture(), false).unwrap();
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        let desktop = uuid_of(&plan, "desktop-pc.md");
        let index = crate::semantic::SemanticIndex::open(
            &vault.ctx,
            crate::semantic::IndexOptions::default(),
        )
        .unwrap();
        let titles: Vec<String> = index
            .backlinks(&desktop)
            .unwrap()
            .into_iter()
            .map(|e| e.title)
            .collect();
        assert!(titles.contains(&"Devices".to_string()), "{titles:?}");
    }

    // ── Source untouched ──────────────────────────────────────────────────────

    fn snapshot(root: &Path) -> BTreeMap<PathBuf, (String, std::time::SystemTime)> {
        walkdir::WalkDir::new(root)
            .into_iter()
            .flatten()
            .map(|e| {
                let meta = e.metadata().unwrap();
                let hash = if meta.is_file() {
                    format!("{:x}", Sha256::digest(fs::read(e.path()).unwrap()))
                } else {
                    String::new()
                };
                (e.path().to_path_buf(), (hash, meta.modified().unwrap()))
            })
            .collect()
    }

    #[test]
    fn source_is_untouched() {
        for source in [tolaria_fixture(), obsidian_fixture()] {
            let before = snapshot(&source);
            let vault = TestVault::new();
            vault.import(&source, false).unwrap();
            assert_eq!(snapshot(&source), before, "{} changed", source.display());
        }
    }

    fn copy_tree(from: &Path, to: &Path) {
        for entry in walkdir::WalkDir::new(from).into_iter().flatten() {
            let dest = to.join(entry.path().strip_prefix(from).unwrap());
            if entry.file_type().is_dir() {
                fs::create_dir_all(&dest).unwrap();
            } else {
                fs::copy(entry.path(), &dest).unwrap();
            }
        }
    }

    #[test]
    fn dates_are_kept_in_meta() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("obsidian");
        copy_tree(&obsidian_fixture(), &source);
        let when = chrono::DateTime::parse_from_rfc3339("2025-12-07T10:00:00Z").unwrap();
        fs::File::options()
            .write(true)
            .open(source.join("Welcome.md"))
            .unwrap()
            .set_modified(std::time::SystemTime::from(when))
            .unwrap();

        let vault = TestVault::new();
        // After the whole run, index refresh included.
        let (plan, _) = vault.import(&source, false).unwrap();
        let uuid = uuid_of(&plan, "Welcome.md");
        let meta =
            ObjectMeta::read_from_file(&vault.ctx.object_path(&uuid).join("meta.ttl")).unwrap();
        assert!(meta.modified.starts_with("2025-12-07"), "{}", meta.modified);
        let planned = plan.pages.iter().find(|p| p.uuid == uuid).unwrap();
        assert!(!meta.created.is_empty());
        assert_eq!(
            meta.created, planned.created,
            "dateCreated comes from the file"
        );
    }

    #[test]
    fn target_inside_source_is_refused() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("src");
        copy_tree(&obsidian_fixture(), &source);
        let plan = plan_import(&source, None).unwrap();

        // Not created yet, or created by someone else: refused either way.
        let err =
            check_target_outside_source(&plan.source, &source.join("inner/vault")).unwrap_err();
        assert!(err.contains("inside the source"), "{err}");
        assert!(check_target_outside_source(&plan.source, &source).is_err());
        assert!(check_target_outside_source(&plan.source, &source.join("x/../../src/y")).is_err());
        assert!(check_target_outside_source(&plan.source, &tmp.path().join("src-vault")).is_ok());
        assert!(check_target_outside_source(&plan.source, &source.join("../elsewhere")).is_ok());

        #[cfg(unix)]
        {
            // A symlink does not hide the nesting.
            std::os::unix::fs::symlink(&source, tmp.path().join("link")).unwrap();
            let nested = tmp.path().join("link/nested");
            assert!(check_target_outside_source(&plan.source, &nested).is_err());
        }
        // run_import guards every caller.
        let before = snapshot(&source);
        crate::vault::vault_init(&tmp.path().join("cache"), &source.join("vault"), false).unwrap();
        let ctx = VaultContext {
            vault_path: source.join("vault"),
            global: CerboContext {
                config_dir: tmp.path().join("config"),
                cache_dir: tmp.path().join("cache"),
            },
        };
        let err = run_import(&ctx, &plan, ImportOptions::default()).unwrap_err();
        assert!(err.contains("inside the source"), "{err}");
        assert_eq!(
            fs::read_dir(source.join("vault/.cerbo/objects"))
                .unwrap()
                .count(),
            0
        );
        let after: Vec<_> = snapshot(&source)
            .into_keys()
            .filter(|p| !p.starts_with(source.join("vault")))
            .collect();
        assert_eq!(after, before.into_keys().collect::<Vec<_>>());
    }

    // ── Creating a target vault ───────────────────────────────────────────────

    #[test]
    fn vault_create_initialises_and_registers() {
        let tmp = TempDir::new().unwrap();
        let ctx = CerboContext {
            config_dir: tmp.path().join("config"),
            cache_dir: tmp.path().join("cache"),
        };
        let path = tmp.path().join("notes");
        let (vault, init) = crate::vault::vault_create(&ctx, &path, "Notes".into(), false).unwrap();
        assert!(init.created);
        assert_eq!(vault.name, "Notes");
        assert!(path.join(".cerbo/objects").is_dir());
        assert!(path.join(".cerbo/ontology-map.json").is_file());
        let listed = crate::vault::vault_list(&ctx).unwrap();
        assert!(listed.vaults.iter().any(|v| v.id == vault.id));
    }

    #[test]
    fn vault_create_refuses_non_empty_folder() {
        let tmp = TempDir::new().unwrap();
        let ctx = CerboContext {
            config_dir: tmp.path().join("config"),
            cache_dir: tmp.path().join("cache"),
        };
        let path = tmp.path().join("notes");
        fs::create_dir_all(path.join(".git")).unwrap();
        assert!(
            crate::vault::check_new_vault_dir(&path).is_ok(),
            ".git alone is fine"
        );
        fs::write(path.join("a.txt"), "x").unwrap();
        assert!(crate::vault::vault_create(&ctx, &path, "Notes".into(), false).is_err());
        assert!(!path.join(".cerbo").exists());
        assert!(crate::vault::vault_list(&ctx).unwrap().vaults.is_empty());
    }

    #[test]
    fn format_parses() {
        assert_eq!("Obsidian".parse::<Format>().unwrap(), Format::Obsidian);
        assert_eq!("tolaria".parse::<Format>().unwrap(), Format::Tolaria);
        assert!("logseq".parse::<Format>().is_err());
    }
}
