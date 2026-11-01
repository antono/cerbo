//! Finding objects still in the old vocabulary, cheaply enough to do before
//! every command, and migrating just those.
//!
//! Before it works on a vault the CLI runs [`check_vocabulary`] (commands that
//! only read the vault) or [`ensure_current_vocabulary`] (commands that write
//! to it), and the desktop launcher runs the latter (through `cerbo migrate
//! vocabulary --if-needed`) before the app starts. A vault synced from a device
//! that still runs an old Cerbo can receive legacy files at any time, so this
//! is not a one-off upgrade step: every call looks again, and only what changed
//! costs a read. The migration itself runs only under the exclusive
//! [`AppLock`], so it never rewrites files under a running desktop app.
//!
//! "Legacy" means exactly what [`crate::vocabulary_migration`] would rewrite:
//! both use [`plan_object`], so the check and the migration never disagree.
//!
//! The cost on a large vault is one `stat` per file. A per-vault table in the
//! user cache, `$XDG_CACHE_HOME/cerbo/vocabulary/<vault key>`, records the
//! `(mtime_ns, size)` of the files of every object already found current; an
//! object is read again only when one of its stamps changed. The vault key is
//! the semantic index's ([`crate::semantic::vault_key`]); the table's name also
//! carries [`rules_stamp`], so a build whose legacy rules differ starts from an
//! empty table instead of trusting verdicts it did not make. The check writes
//! nothing into the vault, and failing to write the table only costs a re-read
//! next time.

use crate::VaultContext;
use crate::app_lock::AppLock;
use crate::prefix_map::PrefixMap;
use crate::semantic::fingerprints::{self, Fingerprints};
use crate::vocabulary_migration::{MigrationReport, live_objects, migrate_objects, plan_object};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Bump when what counts as legacy changes in a way [`rules_stamp`] cannot see:
/// the logic of [`plan_object`] or of the meta and annotation writers. Changes
/// to the alias tables in [`crate::rdf`] and every release already change the
/// stamp on their own.
const RULES_REVISION: u32 = 1;

/// Threads that `stat` and read a large vault in parallel.
const CHECK_THREADS: usize = 8;
/// Below this many objects one thread is faster than starting the pool.
const PARALLEL_THRESHOLD: usize = 1024;

/// What [`check_vocabulary`] found in one vault.
#[derive(Debug, Default, Clone)]
pub struct VocabularyCheck {
    /// Live objects whose `meta.ttl` or `annotations.ttl` the migration would
    /// rewrite, sorted by UUID.
    pub legacy: Vec<String>,
    /// Objects whose files had to be read because the cache could not vouch
    /// for them: new, changed, legacy or unreadable ones.
    pub read: usize,
    /// Problems with the cache table. Never fatal: the next check re-reads.
    pub warnings: Vec<String>,
}

/// What a cached "current" verdict depends on: the Cerbo version, the legacy
/// alias tables, and [`RULES_REVISION`]. A table written under another stamp
/// is never read, so a release that learns a new legacy spelling re-reads
/// every object once instead of trusting objects an older build cached.
pub fn rules_stamp() -> String {
    let mut hasher = Sha256::new();
    let mut feed = |part: &str| {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    };
    feed(env!("CARGO_PKG_VERSION"));
    feed(&RULES_REVISION.to_string());
    feed(crate::rdf::LEGACY_NS_SCHEMA);
    for (legacy, standard) in crate::rdf::LEGACY_PREDICATES.iter().chain(crate::rdf::LEGACY_CLASSES) {
        feed(legacy);
        feed(standard);
    }
    let mut stamp: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    stamp.truncate(12);
    stamp
}

/// The cache table of the vault at `vault_path`, under `cache_dir`
/// (`$XDG_CACHE_HOME/cerbo`): `vocabulary/<vault key>-<rules stamp>`.
pub fn cache_path(cache_dir: &Path, vault_path: &Path) -> Result<PathBuf, String> {
    table_path(cache_dir, vault_path, &rules_stamp())
}

fn table_path(cache_dir: &Path, vault_path: &Path, rules: &str) -> Result<PathBuf, String> {
    let canonical = vault_path
        .canonicalize()
        .map_err(|e| format!("cannot resolve vault path {}: {e}", vault_path.display()))?;
    let key = crate::semantic::vault_key(&canonical);
    Ok(cache_dir.join("vocabulary").join(format!("{key}-{rules}")))
}

/// UUIDs of the live objects that `cerbo migrate vocabulary` would rewrite.
pub fn legacy_objects(vault_ctx: &VaultContext, cache_dir: &Path) -> Result<Vec<String>, String> {
    Ok(check_vocabulary(vault_ctx, cache_dir)?.legacy)
}

/// Find the live objects still in the old vocabulary, reading only the
/// objects whose files changed since the last check.
///
/// An object that cannot be read is neither legacy nor cached: the migration
/// could not rewrite it either, and it is looked at again next time. Only a
/// vault-wide failure (an unreadable `objects/` or prefix map) is an `Err`.
pub fn check_vocabulary(vault_ctx: &VaultContext, cache_dir: &Path) -> Result<VocabularyCheck, String> {
    let mut check = VocabularyCheck::default();
    let objects = live_objects(&vault_ctx.objects_dir())?;

    let table_path = match cache_path(cache_dir, &vault_ctx.vault_path) {
        Ok(path) => Some(path),
        Err(e) => {
            check.warnings.push(format!("vocabulary cache disabled: {e}"));
            None
        }
    };
    let cached = match table_path.as_deref().map(fingerprints::load) {
        Some(Ok(Some(table))) => table,
        Some(Ok(None)) | None => Fingerprints::new(),
        Some(Err(e)) => {
            check.warnings.push(format!("ignoring the vocabulary cache: {e}"));
            Fingerprints::new()
        }
    };

    // Stamp before reading: a file that changes in between then carries a
    // stamp that no longer matches, and is read again next time.
    let stamps = par_map(&objects, |(_, dir)| fingerprints::stamp_object(dir).ok());

    let mut current = Fingerprints::with_capacity(objects.len());
    let mut to_read = Vec::new();
    for ((uuid, dir), stamp) in objects.iter().zip(stamps) {
        match stamp {
            Some(stamp) if cached.get(uuid) == Some(&stamp) => {
                current.insert(uuid.clone(), stamp);
            }
            stamp => to_read.push((uuid, dir, stamp)),
        }
    }
    check.read = to_read.len();

    if !to_read.is_empty() {
        let map = PrefixMap::load_vault(vault_ctx)?;
        let verdicts = par_map(&to_read, |(uuid, dir, _)| {
            // The notes are the migration's to report, when it runs.
            plan_object(&map, dir, uuid, &mut Vec::new()).map(|plan| plan.is_empty())
        });
        for ((uuid, _, stamp), verdict) in to_read.into_iter().zip(verdicts) {
            match (verdict, stamp) {
                (Ok(true), Some(stamp)) => {
                    current.insert(uuid.clone(), stamp);
                }
                (Ok(true), None) | (Err(_), _) => {}
                (Ok(false), _) => check.legacy.push(uuid.clone()),
            }
        }
    }

    if let Some(path) = table_path
        && current != cached
        && let Err(e) = save_table(&path, &current)
    {
        check.warnings.push(format!("could not write the vocabulary cache: {e}"));
    }

    Ok(check)
}

/// What [`ensure_current_vocabulary`] did.
#[derive(Debug, Clone)]
pub enum EnsureOutcome {
    /// Nothing in the vault uses the old vocabulary.
    Current,
    /// These objects use the old vocabulary, but a desktop app (or another
    /// migration) holds the [`AppLock`], so nothing was written.
    Deferred { legacy: Vec<String> },
    /// These objects used the old vocabulary and the migration ran on them.
    Migrated { legacy: Vec<String>, report: MigrationReport },
}

/// Migrate whatever [`check_vocabulary`] finds legacy, then refresh the cache
/// so the rewritten files are not read again next time.
///
/// The migration runs only while this process holds the [`AppLock`]
/// exclusively; when a desktop app is running it is deferred and nothing is
/// written. `before_migrating` is called with the legacy UUIDs before anything
/// is written, so a caller can say what is about to happen. Cache warnings go
/// to stderr: they never stop the caller.
pub fn ensure_current_vocabulary(
    vault_ctx: &VaultContext,
    cache_dir: &Path,
    before_migrating: impl FnOnce(&[String]),
) -> Result<EnsureOutcome, String> {
    let check = check_vocabulary(vault_ctx, cache_dir)?;
    print_warnings(&check.warnings);
    if check.legacy.is_empty() {
        return Ok(EnsureOutcome::Current);
    }

    let lock = AppLock::try_exclusive(cache_dir).map_err(|e| format!("not migrating, the app lock is unusable: {e}"))?;
    let Some(_lock) = lock else {
        return Ok(EnsureOutcome::Deferred { legacy: check.legacy });
    };
    before_migrating(&check.legacy);
    let report = migrate_objects(vault_ctx, &check.legacy, false)?;

    // Only the rewritten objects are read again; they are then cached.
    match check_vocabulary(vault_ctx, cache_dir) {
        Ok(refresh) => print_warnings(&refresh.warnings),
        Err(e) => eprintln!("Warning: could not refresh the vocabulary cache: {e}"),
    }
    Ok(EnsureOutcome::Migrated { legacy: check.legacy, report })
}

fn print_warnings(warnings: &[String]) {
    for warning in warnings {
        eprintln!("Warning: {warning}");
    }
}

fn save_table(path: &Path, table: &Fingerprints) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fingerprints::save(path, table).map_err(|e| e.to_string())?;
    remove_stale_tables(path);
    Ok(())
}

/// Delete this vault's tables written under other rules stamps. Best effort:
/// a leftover table is never read, it only takes space.
fn remove_stale_tables(current: &Path) {
    let (Some(dir), Some(name)) = (current.parent(), current.file_name().and_then(|n| n.to_str())) else {
        return;
    };
    let Some((key, _)) = name.rsplit_once('-') else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let other = entry.file_name();
        let Some(other) = other.to_str() else { continue };
        if other != name && other.rsplit_once('-').is_some_and(|(k, _)| k == key) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// `f` over `items`, in order, on [`CHECK_THREADS`] threads for a large vault.
fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    if items.len() < PARALLEL_THRESHOLD {
        return items.iter().map(f).collect();
    }
    let f = &f;
    let chunk_size = items.len().div_ceil(CHECK_THREADS);
    std::thread::scope(|scope| {
        let workers: Vec<_> = items
            .chunks(chunk_size)
            .map(|chunk| scope.spawn(move || chunk.iter().map(f).collect::<Vec<R>>()))
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("vocabulary check worker panicked"))
            .collect()
    })
}

#[cfg(test)]
// Fixtures write files directly; the atomic-write rule guards vault code, not setup.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::CerboContext;
    use crate::object::{ObjectMeta, ObjectType};
    use crate::vocabulary_migration::migrate_vocabulary;
    use std::collections::BTreeMap;
    use std::fs;
    use std::time::SystemTime;
    use tempfile::TempDir;

    const PAGE: &str = "ba079793-b2ff-4ca6-ba88-8764ce6668c3";

    const LEGACY_META: &str = concat!(
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
        "@prefix schema: <cerbo://ontology/schema/> .\n",
        "@prefix cerbo: <cerbo://ontology/> .\n",
        "<cerbo://objects/ba079793-b2ff-4ca6-ba88-8764ce6668c3> cerbo:type cerbo:Product ;\n",
        "\tcerbo:title \"Index\" ;\n",
        "\tschema:dateCreated \"2026-10-25T13:17:54.772697296+00:00\"^^xsd:dateTime ;\n",
        "\tschema:dateModified \"2026-10-25T13:18:16.608529895+00:00\"^^xsd:dateTime ;\n",
        "\tcerbo:slug \"index\" .\n",
    );

    fn vault() -> (TempDir, VaultContext) {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("vault/.cerbo/objects")).unwrap();
        let ctx = VaultContext {
            vault_path: tmp.path().join("vault"),
            global: CerboContext {
                config_dir: tmp.path().join("config"),
                cache_dir: tmp.path().join("cache"),
            },
        };
        (tmp, ctx)
    }

    fn cache(ctx: &VaultContext) -> &Path {
        &ctx.global.cache_dir
    }

    /// Stamps of an object as the cache stores them.
    fn stamp(dir: &Path) -> fingerprints::ObjectStamp {
        fingerprints::stamp_object(dir).unwrap()
    }

    fn uuid_n(n: usize) -> String {
        format!("00000000-0000-4000-8000-{n:012}")
    }

    fn put(ctx: &VaultContext, uuid: &str, file: &str, content: &str) -> PathBuf {
        let dir = ctx.object_path(uuid);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(file);
        fs::write(&path, content).unwrap();
        path
    }

    fn legacy_meta(uuid: &str) -> String {
        LEGACY_META.replace(PAGE, uuid)
    }

    fn current_meta(uuid: &str, title: &str) -> String {
        ObjectMeta {
            object_type: ObjectType::Product,
            title: title.into(),
            created: "2024-01-01T00:00:00Z".into(),
            modified: "2024-01-01T00:00:00Z".into(),
            original_url: None,
            mime_type: None,
            slug: None,
            virtual_path: None,
        }
        .to_turtle(uuid)
    }

    fn legacy_annotations() -> String {
        format!(
            "<cerbo://objects/{PAGE}> <cerbo://ontology/annotation> [ \
             <cerbo://ontology/concept> \"Berlin\" ; <cerbo://ontology/position> \"1,10\" ] .\n"
        )
    }

    /// Every file under `.cerbo/` with its bytes and modification time.
    fn snapshot(ctx: &VaultContext) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
        walkdir::WalkDir::new(ctx.cerbo_dir())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let path = e.path().to_path_buf();
                let mtime = e.metadata().unwrap().modified().unwrap();
                (path.clone(), (fs::read(&path).unwrap(), mtime))
            })
            .collect()
    }

    /// The objects a dry run would touch, from the files it lists.
    fn dry_run_objects(ctx: &VaultContext) -> Vec<String> {
        let report = migrate_vocabulary(ctx, true).unwrap();
        let mut uuids: Vec<String> = report
            .files
            .iter()
            .map(|f| f.parent().unwrap().file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        uuids.dedup();
        uuids
    }

    /// A vault mixing every case: legacy meta, legacy annotations only,
    /// current, unreadable, and an ontology with a raw annotations file.
    fn mixed_vault() -> (TempDir, VaultContext) {
        let (tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", LEGACY_META);
        put(&ctx, PAGE, "page.md", "I visited [Berlin]{schema:Place}.\n");
        put(&ctx, PAGE, "annotations.ttl", &legacy_annotations());

        let annotated = uuid_n(1);
        put(&ctx, &annotated, "meta.ttl", &current_meta(&annotated, "Annotated"));
        put(&ctx, &annotated, "page.md", "[Berlin]{schema:Place}\n");
        put(&ctx, &annotated, "annotations.ttl", &legacy_annotations().replace(PAGE, &annotated));

        let current = uuid_n(137);
        put(&ctx, &current, "meta.ttl", &current_meta(&current, "Current"));
        put(&ctx, &current, "page.md", "# Current\n");

        put(&ctx, &uuid_n(2), "meta.ttl", "this is {{ not turtle at all");

        let ontology = uuid_n(3);
        put(&ctx, &ontology, "meta.ttl", &legacy_meta(&ontology).replace("cerbo:Product", "cerbo:Ontology"));
        put(&ctx, &ontology, "annotations.ttl", "# raw\n");
        (tmp, ctx)
    }

    #[test]
    fn legacy_objects_are_exactly_what_a_dry_run_would_rewrite() {
        let (_tmp, ctx) = mixed_vault();

        let legacy = legacy_objects(&ctx, cache(&ctx)).unwrap();

        assert_eq!(legacy, dry_run_objects(&ctx));
        assert_eq!(legacy, [uuid_n(1), uuid_n(3), PAGE.to_string()]);
    }

    #[test]
    fn the_check_writes_nothing_into_the_vault() {
        let (_tmp, ctx) = mixed_vault();
        let before = snapshot(&ctx);

        let (check, writes) = crate::fsio::audit::recording(|| check_vocabulary(&ctx, cache(&ctx)).unwrap());

        assert_eq!(check.legacy.len(), 3, "{check:?}");
        assert_eq!(snapshot(&ctx), before, "the check changed .cerbo/");
        assert!(
            writes.iter().all(|w| !w.starts_with(&ctx.vault_path)),
            "the check wrote into the vault: {writes:?}"
        );
        assert!(cache_path(cache(&ctx), &ctx.vault_path).unwrap().is_file(), "no cache table written");
    }

    #[test]
    fn unchanged_objects_are_not_read_again() {
        let (_tmp, ctx) = vault();
        for n in 0..137 {
            put(&ctx, &uuid_n(n), "meta.ttl", &current_meta(&uuid_n(n), &format!("Page {n}")));
        }

        let first = check_vocabulary(&ctx, cache(&ctx)).unwrap();
        assert_eq!((first.read, first.legacy.len()), (137, 0), "{first:?}");
        assert!(first.warnings.is_empty(), "{first:?}");

        let second = check_vocabulary(&ctx, cache(&ctx)).unwrap();
        assert_eq!(second.read, 0, "an unchanged vault was read again: {second:?}");

        // A page edit changes one stamp: only that object is read.
        put(&ctx, &uuid_n(42), "page.md", "edited\n");
        let third = check_vocabulary(&ctx, cache(&ctx)).unwrap();
        assert_eq!((third.read, third.legacy.len()), (1, 0), "{third:?}");
    }

    #[test]
    fn a_cached_object_is_not_even_opened() {
        let (_tmp, ctx) = vault();
        let path = put(&ctx, PAGE, "meta.ttl", &current_meta(PAGE, "Index"));
        check_vocabulary(&ctx, cache(&ctx)).unwrap();

        // Swap the file for one that cannot be parsed, keeping its stamp: a
        // check that read it would now report an unreadable object and read 1.
        let stamp_before = stamp(&ctx.object_path(PAGE));
        let original = fs::read_to_string(&path).unwrap();
        let mtime = fs::metadata(&path).unwrap().modified().unwrap();
        fs::write(&path, "x".repeat(original.len())).unwrap();
        fs::File::options().write(true).open(&path).unwrap().set_modified(mtime).unwrap();
        assert_eq!(stamp(&ctx.object_path(PAGE)), stamp_before);

        let check = check_vocabulary(&ctx, cache(&ctx)).unwrap();
        assert_eq!((check.read, check.legacy.len()), (0, 0), "{check:?}");
    }

    #[test]
    fn a_file_synced_in_later_with_the_old_format_is_found() {
        let (_tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", &current_meta(PAGE, "Index"));
        put(&ctx, &uuid_n(137), "meta.ttl", &current_meta(&uuid_n(137), "Other"));
        assert!(legacy_objects(&ctx, cache(&ctx)).unwrap().is_empty());

        // An old Cerbo on another device saves the page; sync delivers it.
        put(&ctx, PAGE, "meta.ttl", LEGACY_META);
        // And creates a page that did not exist here before.
        put(&ctx, &uuid_n(7), "meta.ttl", &legacy_meta(&uuid_n(7)));

        let check = check_vocabulary(&ctx, cache(&ctx)).unwrap();
        assert_eq!(check.legacy, [uuid_n(7), PAGE.to_string()]);
        assert_eq!(check.read, 2, "{check:?}");
    }

    #[test]
    fn unreadable_objects_are_never_cached() {
        let (_tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", "this is {{ not turtle at all");

        for _ in 0..2 {
            let check = check_vocabulary(&ctx, cache(&ctx)).unwrap();
            assert_eq!((check.read, check.legacy.len()), (1, 0), "{check:?}");
        }
    }

    #[test]
    fn an_unwritable_cache_is_a_warning_not_an_error() {
        let (tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", LEGACY_META);
        // A file where the cache directory should be.
        let blocked = tmp.path().join("blocked");
        fs::write(&blocked, "").unwrap();

        let check = check_vocabulary(&ctx, &blocked).unwrap();

        assert_eq!(check.legacy, [PAGE]);
        assert!(!check.warnings.is_empty(), "{check:?}");
        assert!(check.warnings.iter().all(|w| w.contains("vocabulary cache")), "{check:?}");

        // The lock lives in the cache directory too, so nothing can be
        // migrated safely: the check's verdict stands, nothing is written.
        assert!(ensure_current_vocabulary(&ctx, &blocked, |_| {}).is_err());
        assert!(fs::read_to_string(ctx.object_path(PAGE).join("meta.ttl")).unwrap().contains("cerbo:title"));
    }

    #[test]
    fn a_corrupt_cache_table_only_costs_a_re_read() {
        let (_tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", &current_meta(PAGE, "Index"));
        let table = cache_path(cache(&ctx), &ctx.vault_path).unwrap();
        fs::create_dir_all(table.parent().unwrap()).unwrap();
        fs::write(&table, "garbage").unwrap();

        let check = check_vocabulary(&ctx, cache(&ctx)).unwrap();
        assert_eq!((check.read, check.legacy.len()), (1, 0), "{check:?}");
        assert_eq!(check_vocabulary(&ctx, cache(&ctx)).unwrap().read, 0);
    }

    #[test]
    fn ensure_migrates_only_the_legacy_objects_and_refreshes_the_cache() {
        let (_tmp, ctx) = mixed_vault();
        let current = ctx.object_path(&uuid_n(137)).join("meta.ttl");
        let current_mtime = fs::metadata(&current).unwrap().modified().unwrap();

        let mut announced = Vec::new();
        let outcome = ensure_current_vocabulary(&ctx, cache(&ctx), |legacy| announced = legacy.to_vec()).unwrap();
        let EnsureOutcome::Migrated { legacy, report } = outcome else {
            panic!("the vault had legacy objects: {outcome:?}");
        };
        assert_eq!(legacy, announced);

        assert_eq!(announced, [uuid_n(1), uuid_n(3), PAGE.to_string()]);
        assert_eq!(report.scanned, 3, "only the legacy objects: {report:?}");
        assert_eq!(report.rewritten, 4, "two files of PAGE, one of each other: {report:?}");
        assert!(report.errors.is_empty(), "{report:?}");
        assert_eq!(fs::metadata(&current).unwrap().modified().unwrap(), current_mtime);
        assert!(migrate_vocabulary(&ctx, true).unwrap().files.is_empty(), "something is still legacy");

        // The rewritten objects are cached; only the unreadable one is read.
        let check = check_vocabulary(&ctx, cache(&ctx)).unwrap();
        assert_eq!((check.read, check.legacy.len()), (1, 0), "{check:?}");
    }

    #[test]
    fn ensure_on_a_current_vault_does_nothing() {
        let (_tmp, ctx) = vault();
        put(&ctx, PAGE, "meta.ttl", &current_meta(PAGE, "Index"));
        let before = snapshot(&ctx);

        let mut called = false;
        let (result, writes) = crate::fsio::audit::recording(|| {
            ensure_current_vocabulary(&ctx, cache(&ctx), |_| called = true).unwrap()
        });

        assert!(matches!(result, EnsureOutcome::Current), "{result:?}");
        assert!(!called);
        assert!(writes.iter().all(|w| !w.starts_with(&ctx.vault_path)), "{writes:?}");
        assert_eq!(snapshot(&ctx), before);
    }

    #[test]
    fn a_running_desktop_app_defers_the_migration_and_nothing_is_written() {
        let (_tmp, ctx) = mixed_vault();
        let before = snapshot(&ctx);
        let app = AppLock::shared(cache(&ctx)).unwrap();

        let mut called = false;
        let (outcome, writes) = crate::fsio::audit::recording(|| {
            ensure_current_vocabulary(&ctx, cache(&ctx), |_| called = true).unwrap()
        });

        let EnsureOutcome::Deferred { legacy } = outcome else {
            panic!("migrated under a running app: {outcome:?}");
        };
        assert_eq!(legacy, [uuid_n(1), uuid_n(3), PAGE.to_string()]);
        assert!(!called);
        assert!(writes.iter().all(|w| !w.starts_with(&ctx.vault_path)), "{writes:?}");
        assert_eq!(snapshot(&ctx), before);

        // Once the app has exited, the next run migrates.
        drop(app);
        let outcome = ensure_current_vocabulary(&ctx, cache(&ctx), |_| {}).unwrap();
        assert!(matches!(outcome, EnsureOutcome::Migrated { .. }), "{outcome:?}");
    }

    #[test]
    fn a_table_cached_under_other_legacy_rules_is_not_trusted() {
        let (_tmp, ctx) = vault();
        // A long title, so the legacy file below can be padded to its size.
        let path = put(&ctx, PAGE, "meta.ttl", &current_meta(PAGE, &"x".repeat(137)));
        check_vocabulary(&ctx, cache(&ctx)).unwrap();
        let table = cache_path(cache(&ctx), &ctx.vault_path).unwrap();

        // An older build, whose rules did not know this legacy spelling, cached
        // the object as current; its file has that spelling and the same stamp.
        let older = table_path(cache(&ctx), &ctx.vault_path, "0ld0ld0ld0ld").unwrap();
        fs::rename(&table, &older).unwrap();
        let size = fs::metadata(&path).unwrap().len() as usize;
        assert!(size > LEGACY_META.len() + 2);
        let legacy = format!("{LEGACY_META}#{}\n", "#".repeat(size - LEGACY_META.len() - 2));
        let mtime = fs::metadata(&path).unwrap().modified().unwrap();
        let stamp_before = stamp(&ctx.object_path(PAGE));
        fs::write(&path, &legacy).unwrap();
        fs::File::options().write(true).open(&path).unwrap().set_modified(mtime).unwrap();
        assert_eq!(stamp(&ctx.object_path(PAGE)), stamp_before, "the fixture must keep the stamp");

        let check = check_vocabulary(&ctx, cache(&ctx)).unwrap();

        assert_eq!((check.read, check.legacy.as_slice()), (1, [PAGE.to_string()].as_slice()), "{check:?}");
        // The table written now replaces the one from the older rules.
        put(&ctx, PAGE, "meta.ttl", &current_meta(PAGE, "Index"));
        check_vocabulary(&ctx, cache(&ctx)).unwrap();
        assert!(table.is_file());
        assert!(!older.exists(), "the stale table was kept");
    }

    #[test]
    fn the_rules_stamp_follows_the_legacy_tables() {
        let stamp = rules_stamp();
        assert_eq!(stamp, rules_stamp(), "the stamp must be stable");
        assert_eq!(stamp.len(), 12);
        assert!(cache_path(Path::new("/c"), Path::new("/")).unwrap().to_string_lossy().ends_with(&stamp));
    }

    #[test]
    fn a_large_vault_is_checked_in_parallel_with_the_same_result() {
        let (_tmp, ctx) = vault();
        let n = PARALLEL_THRESHOLD + 137;
        for i in 0..n {
            let uuid = uuid_n(i);
            let meta = if i % 137 == 0 { legacy_meta(&uuid) } else { current_meta(&uuid, "Page") };
            put(&ctx, &uuid, "meta.ttl", &meta);
        }

        let check = check_vocabulary(&ctx, cache(&ctx)).unwrap();

        assert_eq!(check.read, n);
        assert_eq!(check.legacy, dry_run_objects(&ctx));
        assert_eq!(check.legacy.len(), n.div_ceil(137));
        assert_eq!(check_vocabulary(&ctx, cache(&ctx)).unwrap().read, check.legacy.len());
    }
}
