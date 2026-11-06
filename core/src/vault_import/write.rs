//! Writing a plan into a vault: stage → commit → prefix map → index (design
//! D1). Objects are written in full under `.cerbo/import-staging/<id>/` and
//! then renamed into `.cerbo/objects/` one by one, so no half-written object
//! is ever visible. A `manifest.json` written before the first rename lets the
//! next run take back what an interrupted commit left live.

use super::plan::ImportPlan;
use super::report::{ImportReport, ReportVault};
use super::triples::NS_VOCAB;
use crate::object::{ObjectMeta, ObjectType};
use crate::prefix_map::PrefixMap;
use crate::{CerboContext, VaultContext, fsio, rdf};
use oxrdf::{Literal, Term, Triple};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::path::{Component, Path, PathBuf};

const STAGING_DIR: &str = "import-staging";
const MANIFEST: &str = "manifest.json";

/// How to run an import.
#[derive(Debug, Clone, Copy, Default)]
pub struct ImportOptions {
    /// Import even when this source was imported into the vault before,
    /// creating a second, independent copy.
    pub force: bool,
}

/// Written before the commit starts; lists what the commit makes live.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    version: u32,
    source: String,
    format: String,
    ontology: String,
    prefix: String,
    namespace: String,
    /// Object UUIDs in commit order.
    objects: Vec<String>,
}

#[cfg(test)]
thread_local! {
    /// Simulated crash: the commit stops dead after this many renames.
    pub(crate) static CRASH_AFTER_RENAMES: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
    /// Simulated failure: registering the prefix fails after every rename.
    pub(crate) static FAIL_REGISTRATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Simulated crash right after the prefix map is saved.
    pub(crate) static CRASH_AFTER_REGISTRATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn staging_root(vault: &VaultContext) -> PathBuf {
    vault.cerbo_dir().join(STAGING_DIR)
}

/// The context `object.rs` writes through for this vault.
fn object_ctx(vault: &VaultContext) -> CerboContext {
    CerboContext {
        config_dir: vault.cerbo_dir(),
        cache_dir: vault.global.cache_dir.clone(),
    }
}

/// `file://` URL of a local path, percent-encoded so it is a valid IRI.
pub fn file_url(path: &Path) -> String {
    let mut out = String::from("file://");
    for b in path.to_string_lossy().bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `path` made absolute with symlinks resolved, also when it does not exist
/// yet: its nearest existing ancestor is canonicalised and the rest joined
/// back on, with `.` and `..` applied.
fn resolve_path(path: &Path) -> Result<PathBuf, String> {
    let absolute =
        std::path::absolute(path).map_err(|e| format!("Cannot resolve {}: {e}", path.display()))?;
    let mut existing = absolute.as_path();
    let mut rest: Vec<Component> = Vec::new();
    loop {
        if let Ok(canonical) = existing.canonicalize() {
            let mut out = canonical;
            for component in rest.into_iter().rev() {
                match component {
                    Component::ParentDir => {
                        out.pop();
                    }
                    Component::CurDir => {}
                    other => out.push(other),
                }
            }
            return Ok(out);
        }
        let Some(parent) = existing.parent() else {
            return Err(format!("Cannot resolve {}", path.display()));
        };
        rest.extend(
            existing
                .file_name()
                .map(Component::Normal)
                .or(Some(Component::ParentDir)),
        );
        existing = parent;
    }
}

/// Refuse a target vault at or inside the source folder: writing it would
/// write into the source, which an import never does. `target` need not exist.
pub fn check_target_outside_source(source: &Path, target: &Path) -> Result<(), String> {
    let source = source
        .canonicalize()
        .map_err(|e| format!("Cannot resolve {}: {e}", source.display()))?;
    let target_resolved = resolve_path(target)?;
    if target_resolved.starts_with(&source) {
        return Err(format!(
            "The target vault {} is inside the source folder {}; the source is never written, so pick a vault outside it",
            target.display(),
            source.display()
        ));
    }
    Ok(())
}

/// Hold this vault's import lock: exclusive, without waiting. One lock file
/// per vault under the global cache, so the vault itself gains no file.
pub(crate) fn import_lock(vault: &VaultContext) -> Result<File, String> {
    let dir = vault.global.cache_dir.join("import-locks");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
    let canonical = vault
        .vault_path
        .canonicalize()
        .unwrap_or_else(|_| vault.vault_path.clone());
    let path = dir.join(format!("{}.lock", crate::semantic::vault_key(&canonical)));
    // Opening never truncates: the file is only a lock handle, never data.
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(
            "Another import is running into this vault; try again when it has finished".to_string(),
        ),
        Err(TryLockError::Error(e)) => Err(format!("cannot lock {}: {e}", path.display())),
    }
}

/// Write `plan` into `vault` and return the report.
///
/// The whole run holds the vault's import lock, so a second import cannot
/// take a live one's staging folder for a leftover. Before anything else a
/// leftover staging folder from an interrupted import is cleaned up. Unless
/// `options.force`, an earlier import of the same source is refused
/// ([`check_repeat`]). On failure the vault's live objects and prefix map are
/// as they were.
pub fn run_import(
    vault: &VaultContext,
    plan: &ImportPlan,
    options: ImportOptions,
) -> Result<ImportReport, String> {
    check_target_outside_source(&plan.source, &vault.vault_path)?;
    if !vault.cerbo_dir().is_dir() {
        return Err(format!(
            "Not a Cerbo vault (no .cerbo/ directory): {}",
            vault.vault_path.display()
        ));
    }
    let _lock = import_lock(vault)?;
    if !options.force {
        check_repeat(vault, plan)?;
    }
    fs::create_dir_all(vault.objects_dir())
        .map_err(|e| format!("Failed to create objects dir: {e}"))?;
    let recovered = recover_staging(vault)?;
    let uuids = plan.object_uuids();
    if let Some(taken) = uuids.iter().find(|u| vault.object_path(u).exists()) {
        return Err(format!(
            "This plan was already written (object {taken} exists); plan the import again"
        ));
    }

    let staging = staging_root(vault).join(&plan.ontology_uuid);
    let prefix = unique_prefix(&PrefixMap::load_vault(vault)?, &plan.prefix_base);
    if let Err(e) = stage(&staging, plan) {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }
    let manifest = Manifest {
        version: 1,
        source: plan.source.to_string_lossy().into_owned(),
        format: plan.format.as_str().to_string(),
        ontology: plan.ontology_uuid.clone(),
        prefix: prefix.clone(),
        namespace: plan.namespace.clone(),
        objects: uuids.clone(),
    };
    let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    if let Err(e) = fsio::write_atomic_str(&staging.join(MANIFEST), &json) {
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("Failed to write import manifest: {e}"));
    }

    let mut warnings = Vec::new();
    let prefix = match commit(vault, &staging, &uuids, &prefix, plan) {
        Ok(prefix) => prefix,
        Err(Commit::Crashed) => return Err("simulated crash during commit".to_string()),
        Err(Commit::Failed { error, renamed }) => {
            if rollback(vault, &renamed) {
                let _ = fs::remove_dir_all(&staging);
                let _ = fs::remove_dir(staging_root(vault));
                return Err(error);
            }
            // The manifest stays, so the next import takes back what is left.
            return Err(format!(
                "{error}; some imported objects could not be taken back yet, the next import into this vault will finish the cleanup"
            ));
        }
    };
    if let Err(e) = fs::remove_dir_all(&staging) {
        warnings.push(format!(
            "could not remove the staging folder {}: {e}",
            staging.display()
        ));
    }
    let _ = fs::remove_dir(staging_root(vault));

    let mut report = plan.report();
    report.dry_run = false;
    report.recovered = recovered;
    report.ontology.prefix = Some(prefix);
    report.vault = Some(ReportVault {
        id: None,
        name: None,
        path: vault.vault_path.to_string_lossy().into_owned(),
    });
    report.warnings.extend(warnings);
    if let Err(e) = refresh_index(vault) {
        report
            .warnings
            .push(format!("index not refreshed: {e}; run `cerbo index`"));
    }
    Ok(report)
}

/// Refuse a second import of the plan's source (same canonical path, in any
/// format) into `vault`, naming the earlier one's date. It only reads, so it
/// can run before anything is written. An ontology a leftover staging folder
/// is about to take back does not count.
pub fn check_repeat(vault: &VaultContext, plan: &ImportPlan) -> Result<(), String> {
    let url = file_url(&plan.source);
    let Ok(entries) = fs::read_dir(vault.objects_dir()) else {
        return Ok(());
    };
    let pending = pending_ontologies(vault);
    for entry in entries.flatten() {
        let uuid = entry.file_name().to_string_lossy().into_owned();
        if pending.contains(&uuid) {
            continue;
        }
        let Ok(content) = fs::read_to_string(entry.path().join("meta.ttl")) else {
            continue;
        };
        // Cheap filter before parsing: only import ontologies say this.
        if !content.contains("importFormat") {
            continue;
        }
        let Ok(triples) = rdf::parse(&content) else {
            continue;
        };
        let subject = oxrdf::NamedOrBlankNode::from(rdf::object_iri(&uuid));
        let value = |predicate: oxrdf::NamedNode| {
            triples
                .iter()
                .find(|t| t.subject == subject && t.predicate == predicate)
                .map(|t| match &t.object {
                    Term::NamedNode(n) => n.as_str().to_string(),
                    Term::Literal(l) => l.value().to_string(),
                    other => other.to_string(),
                })
        };
        if value(rdf::schema("url")).as_deref() == Some(url.as_str()) {
            let date = value(rdf::schema("dateCreated")).unwrap_or_default();
            let day = date.get(..10).unwrap_or(&date);
            let format = value(rdf::cerbo("importFormat")).unwrap_or_default();
            return Err(format!(
                "{} was already imported into this vault on {day} as {format} (ontology {uuid}); pass --force to import a second copy",
                plan.source.display()
            ));
        }
    }
    Ok(())
}

/// The manifests of leftover staging folders.
fn leftover_manifests(vault: &VaultContext) -> Vec<(PathBuf, Option<Manifest>)> {
    let Ok(entries) = fs::read_dir(staging_root(vault)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| {
            let path = entry.path();
            let manifest = fs::read_to_string(path.join(MANIFEST))
                .ok()
                .and_then(|json| serde_json::from_str::<Manifest>(&json).ok());
            (path, manifest)
        })
        .collect()
}

/// Whether the import a manifest describes got as far as registering its
/// prefix. The commit may have picked another prefix than the manifest's, and
/// the user may have renamed it since, so any entry for the ontology counts.
fn completed(map: &PrefixMap, manifest: &Manifest) -> bool {
    map.prefixes
        .values()
        .any(|e| e.ontology.as_deref() == Some(manifest.ontology.as_str()))
}

/// Ontologies of interrupted imports that recovery will take back.
fn pending_ontologies(vault: &VaultContext) -> HashSet<String> {
    let manifests = leftover_manifests(vault);
    if manifests.is_empty() {
        return HashSet::new();
    }
    let Ok(map) = PrefixMap::load_vault(vault) else {
        return HashSet::new();
    };
    manifests
        .into_iter()
        .filter_map(|(_, m)| m)
        .filter(|m| !completed(&map, m))
        .map(|m| m.ontology)
        .collect()
}

/// Clean up after an import that did not finish: for each leftover staging
/// folder, the objects its manifest lists that are already live are moved to
/// the trash, unless the import got as far as registering its prefix (then it
/// is complete). The staging folder is removed; a stray file there is too.
/// Returns how many objects were trashed.
///
/// Callers hold the import lock ([`run_import`] does), so every staging folder
/// found belongs to a run that is over.
pub fn recover_staging(vault: &VaultContext) -> Result<usize, String> {
    let root = staging_root(vault);
    let ctx = object_ctx(vault);
    let mut trashed = 0;
    for (dir, manifest) in leftover_manifests(vault) {
        if !dir.is_dir() {
            fs::remove_file(&dir)
                .map_err(|e| format!("Failed to remove {}: {e}", dir.display()))?;
            continue;
        }
        // Without a manifest the commit never started: nothing is live.
        if let Some(manifest) = manifest
            && !completed(&PrefixMap::load_vault(vault)?, &manifest)
        {
            for uuid in &manifest.objects {
                if vault.object_path(uuid).is_dir() {
                    crate::object::object_delete(&ctx, uuid)?;
                    trashed += 1;
                }
            }
        }
        fs::remove_dir_all(&dir).map_err(|e| format!("Failed to remove {}: {e}", dir.display()))?;
    }
    let _ = fs::remove_dir(&root);
    Ok(trashed)
}

/// `base`, or `base-2`, `base-3`, … — the first prefix the map (with its
/// built-in prefixes) does not define.
fn unique_prefix(map: &PrefixMap, base: &str) -> String {
    if map.namespace(base).is_none() {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|p| map.namespace(p).is_none())
        .expect("some suffix is always free")
}

fn stage(staging: &Path, plan: &ImportPlan) -> Result<(), String> {
    if staging.exists() {
        return Err(format!(
            "Staging folder already exists: {}",
            staging.display()
        ));
    }
    let dir = |uuid: &str| -> Result<PathBuf, String> {
        let d = staging.join(uuid);
        fs::create_dir_all(&d).map_err(|e| format!("Failed to create {}: {e}", d.display()))?;
        Ok(d)
    };
    let write = |path: &Path, content: &str| {
        fsio::write_atomic_str(path, content)
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))
    };

    for a in &plan.attachments {
        let d = dir(&a.uuid)?;
        // The object's own files keep their names.
        let name = if matches!(a.name.as_str(), "meta.ttl" | "page.md") {
            format!("attachment-{}", a.name)
        } else {
            a.name.clone()
        };
        fsio::copy_atomic(&a.abs, &d.join(&name))
            .map_err(|e| format!("Failed to copy {}: {e}", a.rel))?;
        let meta = ObjectMeta {
            object_type: ObjectType::Attachment,
            title: a.name.clone(),
            created: a.created.clone(),
            modified: a.modified.clone(),
            original_url: None,
            mime_type: Some(crate::object::detect_mime_type(&a.abs)),
            slug: None,
            virtual_path: None,
        };
        write(&d.join("meta.ttl"), &meta.to_turtle(&a.uuid))?;
    }

    for p in &plan.pages {
        let d = dir(&p.uuid)?;
        let meta = ObjectMeta {
            object_type: ObjectType::Product,
            title: p.title.clone(),
            created: p.created.clone(),
            modified: p.modified.clone(),
            original_url: None,
            mime_type: None,
            slug: Some(p.slug.clone()),
            virtual_path: p.virtual_path.clone(),
        };
        write(
            &d.join("meta.ttl"),
            &meta.to_turtle_with(&p.uuid, &p.triples),
        )?;
        write(&d.join("page.md"), &p.content)?;
    }

    let d = dir(&plan.ontology_uuid)?;
    let now = chrono::Utc::now().to_rfc3339();
    let meta = ObjectMeta {
        object_type: ObjectType::Ontology,
        title: plan.ontology_title.clone(),
        created: now.clone(),
        modified: now,
        original_url: Some(file_url(&plan.source)),
        mime_type: None,
        slug: None,
        virtual_path: None,
    };
    let mut extra = vec![Triple::new(
        rdf::object_iri(&plan.ontology_uuid),
        rdf::cerbo("importFormat"),
        Term::from(Literal::new_simple_literal(plan.format.as_str())),
    )];
    extra.extend(plan.vocab.declarations());
    write(
        &d.join("meta.ttl"),
        &meta.to_turtle_with(&plan.ontology_uuid, &extra),
    )?;
    write(&d.join("page.md"), &ontology_page(plan))?;
    Ok(())
}

/// The Ontology object's page: where the import came from and the classes
/// and properties it found, with counts.
fn ontology_page(plan: &ImportPlan) -> String {
    let mut out = format!(
        "# {}\n\nImported from `{}` ({}).\n\nNamespace: `{}`\n",
        plan.ontology_title,
        plan.source.display(),
        plan.format.display_name(),
        plan.namespace
    );
    out.push_str("\n## Classes\n\n");
    if plan.vocab.classes.is_empty() {
        out.push_str("None.\n");
    } else {
        out.push_str("| Class | Pages |\n|---|---|\n");
        for (local, (label, count)) in &plan.vocab.classes {
            out.push_str(&format!(
                "| `{local}` ({}) | {count} |\n",
                label.replace('|', "\\|")
            ));
        }
    }
    out.push_str("\n## Properties\n\n");
    if plan.vocab.properties.is_empty() {
        out.push_str("None.\n");
    } else {
        out.push_str("| Property | Values |\n|---|---|\n");
        for (local, (key, count)) in &plan.vocab.properties {
            out.push_str(&format!(
                "| `{local}` ({}) | {count} |\n",
                key.replace('|', "\\|")
            ));
        }
    }
    out
}

enum Commit {
    #[cfg_attr(not(test), allow(dead_code))]
    Crashed,
    Failed {
        error: String,
        renamed: Vec<String>,
    },
}

/// Rename every staged object into `objects/`, in order, then register the
/// prefix. Returns the prefix actually registered.
fn commit(
    vault: &VaultContext,
    staging: &Path,
    uuids: &[String],
    prefix: &str,
    plan: &ImportPlan,
) -> Result<String, Commit> {
    let objects = vault.objects_dir();
    let mut renamed: Vec<String> = Vec::new();
    for uuid in uuids {
        #[cfg(test)]
        if CRASH_AFTER_RENAMES.with(|c| c.get()) == Some(renamed.len()) {
            return Err(Commit::Crashed);
        }
        if let Err(e) = fs::rename(staging.join(uuid), objects.join(uuid)) {
            return Err(Commit::Failed {
                error: format!("Failed to move object {uuid} into the vault: {e}"),
                renamed,
            });
        }
        renamed.push(uuid.clone());
    }
    let _ = fsio::sync_dir(&objects);

    // Re-read: another process may have registered the prefix meanwhile.
    let registered = PrefixMap::load_vault(vault).and_then(|mut map| {
        let prefix = if map.namespace(prefix).is_none() {
            prefix.to_string()
        } else {
            unique_prefix(&map, &plan.prefix_base)
        };
        debug_assert!(plan.namespace.starts_with(NS_VOCAB));
        crate::prefix_map::validate_registration(&prefix, &plan.namespace)?;
        #[cfg(test)]
        if FAIL_REGISTRATION.with(|c| c.get()) {
            return Err("simulated registration failure".to_string());
        }
        map.register(&prefix, &plan.namespace, Some(&plan.ontology_uuid));
        map.save(&vault.cerbo_dir())?;
        Ok(prefix)
    });
    let prefix = registered.map_err(|error| Commit::Failed { error, renamed })?;
    #[cfg(test)]
    if CRASH_AFTER_REGISTRATION.with(|c| c.get()) {
        return Err(Commit::Crashed);
    }
    // The import is complete: without its manifest the staging folder is only
    // leftovers, whatever happens to the prefix map later.
    let _ = fs::remove_file(staging.join(MANIFEST));
    Ok(prefix)
}

/// Take back objects a failed commit made live: they go to the trash.
/// Returns whether every one of them was taken back.
fn rollback(vault: &VaultContext, renamed: &[String]) -> bool {
    let ctx = object_ctx(vault);
    let mut all = true;
    for uuid in renamed {
        if let Err(e) = crate::object::object_delete(&ctx, uuid) {
            eprintln!("Warning: could not take back imported object {uuid}: {e}");
            all = false;
        }
    }
    all
}

/// One incremental catch-up of the semantic index — the one `page backlinks`
/// runs — so the imported pages are in backlinks and search right away.
fn refresh_index(vault: &VaultContext) -> Result<(), String> {
    let mut index =
        crate::semantic::SemanticIndex::open(vault, crate::semantic::IndexOptions::default())?;
    index.catch_up(None)?;
    Ok(())
}
