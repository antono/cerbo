use cerbo_core::CerboContext;
use cerbo_core::context::CoreContext;
use clap::{Parser, Subcommand};
use serde::Serialize;

fn print_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()));
}

fn print_json_success(message: &str) {
    println!("{}", serde_json::json!({"success": true, "message": message}));
}

fn print_json_error(message: &str) {
    println!("{}", serde_json::json!({"error": true, "message": message}));
}

// Print pages in the template specified by the user:
// Pages in "Vault Name" (<vault_uuid>):
// <uuid>: Page Title
fn print_pages_template(
    global_ctx: &CerboContext,
    eff_ctx: &CerboContext,
    pages: &[cerbo_core::page::PageMeta],
) -> Result<(), String> {
    use cerbo_core::vault;

    // Identify which vault we're displaying by matching eff_ctx.config_dir against registry.
    let reg = vault::vault_list(global_ctx)?;
    let vault_path = eff_ctx.config_dir.parent().unwrap_or(&eff_ctx.config_dir);
    let matched = reg.vaults.iter().find(|v| {
        std::fs::canonicalize(&v.path).ok()
            == std::fs::canonicalize(vault_path).ok()
    });

    let (vault_name, vault_id) = if let Some(v) = matched {
        (v.name.clone(), v.id.clone())
    } else {
        // Unregistered vault: use directory name as display name
        let name = vault_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "(unknown)".to_string());
        (name, "<unregistered>".to_string())
    };

    println!("Pages in \"{}\" ({}):", vault_name, vault_id);
    for p in pages {
        match &p.virtual_path {
            Some(path) => println!("{}: {} [{}]", p.uuid, p.title, path),
            None => println!("{}: {}", p.uuid, p.title),
        }
    }

    Ok(())
}

// Print vaults in a compact template similar to pages. Shows (current) next to the
// vault name when it is the active vault, and (auto) for auto-registered vaults.
fn print_vaults_template(ctx: &CerboContext, vaults_file: &cerbo_core::vault::VaultsFile) -> Result<(), String> {
    use cerbo_core::state;

    let st = state::load_state(ctx)?;
    let active = st.active_vault_id;

    for v in &vaults_file.vaults {
        let auto_marker = if v.is_auto { " (auto)" } else { "" };
        let current_marker = if Some(v.id.clone()) == active { " (current)" } else { "" };
        println!("{}: {}{}{}", v.id, v.name, auto_marker, current_marker);
    }

    Ok(())
}

#[derive(Serialize)]
struct PageJson {
    uuid: String,
    title: String,
}

#[derive(Parser)]
#[command(name = "cerbo")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    about = "Semantic knowledge management tool for agents and humans",
    long_about = "Cerbo is a local-first markdown wiki: every page is an object stored by UUID \
under .cerbo/objects/, linked with cerbo://<uuid> and annotated with [Text]{prefix:Type}.\n\n\
Commands find the vault like Git, by walking up from the current directory to a .cerbo/ \
directory, so --vault is rarely needed. Most commands take --json for scripting.\n\n\
Full reference: man cerbo"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new vault in the current directory (creates .cerbo/, bundles Schema.org and FOAF)
    Init {
        /// Output the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Manage registered vaults (workspaces)
    Vault {
        #[command(subcommand)]
        action: VaultCommands,
    },
    /// Create, read, write, delete pages and list their backlinks
    Page {
        #[command(subcommand)]
        action: PageCommands,
    },
    /// Resolve an object UUID to its local filesystem path
    Resolve {
        /// UUID of the object
        uuid: String,
        /// Output the result as JSON ({"path"})
        #[arg(long)]
        json: bool,
    },
    /// Show the config dir, cache dir and registered vaults
    Info {
        /// Output the result as JSON ({"config_dir", "cache_dir", "vaults"})
        #[arg(long)]
        json: bool,
    },
    /// Import a URL as a read-only Source object (rdf:type schema:WebPage)
    Import {
        /// The URL to import
        url: String,
        /// Output the result as JSON ({"uuid", "url"})
        #[arg(long)]
        json: bool,
    },
    /// Import an ontology file as an Ontology object (rdf:type schema:DefinedTermSet)
    ImportOntology {
        /// URL of the ontology file (e.g. Schema.org or FOAF)
        url: String,
        /// Annotation prefix to register for this ontology (needs --namespace)
        #[arg(long, requires = "namespace")]
        prefix: Option<String>,
        /// Namespace IRI the prefix expands to, e.g. https://example.org/vocab# (needs --prefix)
        #[arg(long, requires = "prefix")]
        namespace: Option<String>,
        /// Output the result as JSON ({"uuid", "url", "prefix", "namespace"})
        #[arg(long)]
        json: bool,
    },
    /// Import an Obsidian or Tolaria vault (a folder of Markdown notes) as pages; the folder is only read
    #[command(group = clap::ArgGroup::new("target").args(["vault", "new"]))]
    ImportVault {
        /// The folder to import (the Obsidian or Tolaria vault)
        dir: std::path::PathBuf,
        /// Source format, when detection fails or guesses wrong: obsidian or tolaria
        #[arg(long, value_name = "FORMAT")]
        from: Option<cerbo_core::vault_import::Format>,
        /// Import into this registered vault (an ID from `cerbo vault list`)
        #[arg(long, value_name = "ID")]
        vault: Option<String>,
        /// Create a new vault in this empty folder (as `cerbo init` does), register it and import into it
        #[arg(long, value_name = "PATH")]
        new: Option<std::path::PathBuf>,
        /// Name of the vault created by --new (default: the source folder name)
        #[arg(long, requires = "new")]
        name: Option<String>,
        /// Import again even though this folder was already imported into the vault (makes a second copy)
        #[arg(long)]
        force: bool,
        /// Print the report without asking for a vault or writing anything
        #[arg(long)]
        dry_run: bool,
        /// Print only the report, as one JSON object
        #[arg(long)]
        json: bool,
    },
    /// Rebuild page metadata (annotations.ttl) and the semantic index of every vault, and report broken links
    Index {
        /// Index only this page (incremental); cannot be combined with --vault
        #[arg(long)]
        page: Option<String>,
        /// Index only this vault: a vault ID, or a vault path (default: every registered vault)
        #[arg(long)]
        vault: Option<String>,
        /// Skip backfilling missing cerbo:slug values
        #[arg(long)]
        no_backfill_slug: bool,
        /// Also embed every page whose content changed and refresh the similarity links
        #[arg(long)]
        embeddings: bool,
        /// Output the statistics as JSON
        #[arg(long)]
        json: bool,
    },
    /// Find pages by meaning (needs `cerbo index --embeddings` once)
    Search {
        /// What to look for, in any language
        text: String,
        #[command(flatten)]
        filters: SearchArgs,
    },
    /// Find the pages closest in meaning to a page (needs `cerbo index --embeddings` once)
    Similar {
        /// UUID of the page
        uuid: String,
        #[command(flatten)]
        filters: SearchArgs,
    },
    /// Run a read-only SPARQL query against the vault's semantic index
    Sparql {
        /// The query, or `-` to read it from stdin
        #[arg(required_unless_present = "file", conflicts_with = "file")]
        query: Option<String>,
        /// Read the query from this file
        #[arg(long)]
        file: Option<std::path::PathBuf>,
        /// Explicit vault path (default: discover from CWD like Git)
        #[arg(long)]
        vault: Option<String>,
        /// Print SPARQL Results JSON (SELECT, ASK) or JSON-LD (CONSTRUCT, DESCRIBE)
        #[arg(long)]
        json: bool,
    },
    /// Rename a folder of the virtual tree (moves every page under it)
    Folder {
        #[command(subcommand)]
        action: FolderCommands,
    },
    /// Rewrite stored metadata into the current format
    Migrate {
        #[command(subcommand)]
        action: MigrateCommands,
    },
    /// Build (or rebuild) the human-readable symlink tree at <vault>/cerbo/
    Symlink {
        /// Explicit vault path (default: discover from CWD like Git)
        #[arg(long)]
        vault: Option<String>,
        /// Output the statistics as JSON ({"objects_scanned", "leaves_created", "dirs_created"})
        #[arg(long)]
        json: bool,
    },
}

/// Options shared by `cerbo search` and `cerbo similar`.
#[derive(clap::Args)]
struct SearchArgs {
    /// Return at most this many pages
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..))]
    limit: u64,
    /// Keep only objects with this rdf:type (prefix:Class or IRI; repeat to require several)
    #[arg(long = "type", value_name = "CLASS")]
    types: Vec<String>,
    /// Keep only pages with an annotation of this type (prefix:Type or IRI; repeat to require several)
    #[arg(long, value_name = "TYPE")]
    annotated: Vec<String>,
    /// Explicit vault path (default: discover from CWD like Git)
    #[arg(long)]
    vault: Option<String>,
    /// Print a JSON array of {uuid, title, score}
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand)]
enum VaultCommands {
    /// List all registered vaults; auto-discovered ones are marked (auto)
    List {
        /// Output a JSON list; each vault has an "is_auto" field
        #[arg(long)]
        json: bool,
    },
    /// Register a vault under a name
    Add {
        /// Display name of the vault
        name: String,
        /// Path to the vault directory (the one containing .cerbo/)
        path: String,
        /// Output the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Remove a vault from the registry (manual or auto); its files are kept
    Remove {
        /// Vault ID, as shown by `cerbo vault list`
        id: String,
        /// Output the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Set the active vault, used when no vault is found around the current directory
    Active {
        /// Vault ID, as shown by `cerbo vault list`
        id: String,
        /// Output the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Promote an auto-registered vault to the manual registry
    Approve {
        /// Vault ID, as shown by `cerbo vault list`
        id: String,
        /// Output the result as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum MigrateCommands {
    /// Rewrite meta.ttl and annotations.ttl into the schema.org vocabulary
    Vocabulary {
        /// Vault ID or path to migrate (default: every registered vault)
        #[arg(long)]
        vault: Option<String>,
        /// Report what would change without writing anything
        #[arg(long, conflicts_with = "if_needed")]
        dry_run: bool,
        /// Check cheaply (cached file stamps) and migrate only the objects still in the old format
        #[arg(long)]
        if_needed: bool,
        /// Print only {"scanned", "rewritten", "errors"} as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum FolderCommands {
    /// Replace the folder prefix OLD with NEW on every page whose virtual path is OLD or starts with OLD/
    Rename {
        /// Current folder path, e.g. notes
        old: String,
        /// New folder path, e.g. archive/notes
        new: String,
        /// Output the result as JSON ({"from", "to", "pages", "failed"})
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum PageCommands {
    /// List all pages of the vault: a header line, then one `<uuid>: <title> [<virtual path>]` per page
    List {
        /// Print a JSON array of {"uuid", "title", "virtualPath"}
        #[arg(long)]
        json: bool,
        /// Vault ID to list pages from. If omitted, the vault is discovered from the current directory
        #[arg(long)]
        vault: Option<String>,
    },
    /// Create a new page and print its UUID
    Create {
        /// Title of the new page
        title: String,
        /// Output the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Print the markdown content of a page
    Read {
        /// UUID of the page
        uuid: String,
        /// Output the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Replace the content of a page; links and annotations are re-extracted
    Write {
        /// UUID of the page
        uuid: String,
        /// The new markdown content (the whole page)
        content: String,
        /// Output the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Move a page to the trash (.cerbo/trash/); imported Source objects cannot be deleted
    Delete {
        /// UUID of the page
        uuid: String,
        /// Output the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Set a page's virtual path (its folder in the symlink tree and the desktop sidebar), or clear it with --root
    #[command(group = clap::ArgGroup::new("target").required(true).args(["path", "root"]))]
    Move {
        /// UUID of the page
        uuid: String,
        /// The new folder path, e.g. notes/rust (no leading or trailing slash, no . or .. segments)
        path: Option<String>,
        /// Move the page to the root: remove its virtual path
        #[arg(long)]
        root: bool,
        /// Output the result as JSON ({"uuid", "virtualPath"}; virtualPath is null at the root)
        #[arg(long)]
        json: bool,
    },
    /// List the pages that link to an object, one per line as `<uuid>: <title>`
    Backlinks {
        /// UUID of the object
        uuid: String,
        /// Print a JSON array of {"uuid", "title"}
        #[arg(long)]
        json: bool,
        /// Vault ID to read from. If omitted, the vault is resolved like other page commands
        #[arg(long)]
        vault: Option<String>,
    },
}

/// The vaults `cerbo migrate vocabulary` works on, as (label, root) pairs.
///
/// `--vault` takes a registered vault ID or a vault path. Without it, every
/// registered vault is migrated; one whose directory is missing is skipped with
/// a warning, since there is nothing there to rewrite.
fn migration_targets(
    ctx: &CerboContext,
    explicit: Option<&str>,
) -> Result<Vec<(String, std::path::PathBuf)>, String> {
    use cerbo_core::vault;

    if let Some(target) = explicit {
        if let Some(path) = vault::get_vault_path(ctx, target) {
            return Ok(vec![(target.to_string(), path)]);
        }
        let path = std::path::PathBuf::from(target);
        if path.join(".cerbo").is_dir() {
            return Ok(vec![(path.display().to_string(), path)]);
        }
        return Err(format!("vault not found: {} (neither a vault ID nor a path containing .cerbo/)", target));
    }

    let mut targets = Vec::new();
    for v in vault::vault_list(ctx)?.vaults {
        if v.path.join(".cerbo").is_dir() {
            targets.push((v.id, v.path));
        } else {
            eprintln!("Warning: skipping vault {} ({}): no .cerbo/ directory", v.id, v.path.display());
        }
    }
    Ok(targets)
}

/// A vault's name in the registry, or its path when it is not registered.
fn vault_label(ctx: &CerboContext, vault_root: &std::path::Path) -> String {
    let canonical = std::fs::canonicalize(vault_root).ok();
    cerbo_core::vault::vault_list(ctx)
        .ok()
        .and_then(|reg| {
            reg.vaults
                .into_iter()
                .find(|v| canonical.is_some() && std::fs::canonicalize(&v.path).ok() == canonical)
        })
        .map(|v| v.name)
        .unwrap_or_else(|| vault_root.display().to_string())
}

/// Look for objects in the pre-schema.org vocabulary before a command works on
/// a vault. Old Cerbo builds may still write that format into a synced vault,
/// so this runs on every command; a vault already checked costs one `stat` per
/// file. Every such command migrates them first, but only while no desktop
/// app holds the app lock, so a running app's saves are never overwritten. Everything goes to stderr, so `--json` output stays
/// clean, and nothing here fails the command: the readers understand the old
/// format anyway.
///
/// Returns `(objects found in the old format and migrated, files rewritten,
/// errors)`.
fn check_old_vocabulary(ctx: &CerboContext, vault_root: &std::path::Path) -> (usize, usize, usize) {
    use cerbo_core::vocabulary_check::{EnsureOutcome, ensure_current_vocabulary};

    if !vault_root.join(".cerbo").is_dir() {
        return (0, 0, 0);
    }
    let vault_ctx = cerbo_core::VaultContext { vault_path: vault_root.to_path_buf(), global: ctx.clone() };
    let label = vault_label(ctx, vault_root);
    let old_format = |n: usize, what: &str| {
        eprintln!("Warning: {} objects in vault \"{}\" use the old metadata format; {}", n, label, what);
    };

    let result = ensure_current_vocabulary(&vault_ctx, &ctx.cache_dir, |legacy| {
        old_format(legacy.len(), "migrating them to schema.org");
    });
    match result {
        Ok(EnsureOutcome::Current) => (0, 0, 0),
        Ok(EnsureOutcome::Deferred { legacy }) => {
            old_format(
                legacy.len(),
                "not migrating them while Cerbo desktop or another migration is running",
            );
            (0, 0, 0)
        }
        Ok(EnsureOutcome::Migrated { report, .. }) => {
            // The same notes `cerbo migrate vocabulary` prints, such as a type
            // kept as cerbo:unresolvedType.
            for warning in &report.warnings {
                eprintln!("Warning: {}", warning);
            }
            for err in &report.errors {
                eprintln!("Warning: could not migrate object {}: {}", err.uuid, err.message);
            }
            eprintln!(
                "Migrated vault \"{}\": rewrote {} files, {} errors",
                label,
                report.rewritten,
                report.errors.len()
            );
            (report.scanned, report.rewritten, report.errors.len())
        }
        Err(e) => {
            eprintln!("Warning: could not migrate vault \"{}\" from the old metadata format: {}", label, e);
            (0, 0, 1)
        }
    }
}

/// The vault root a resolved page context points into.
fn vault_root_of(vault_ctx: &CerboContext) -> &std::path::Path {
    vault_ctx.config_dir.parent().unwrap_or(&vault_ctx.config_dir)
}

/// The vault a page context points into, for the semantic index: its
/// `config_dir` is the vault's `.cerbo/`, and the index lives in the global
/// cache.
fn index_vault_ctx(global_ctx: &CerboContext, vault_ctx: &CerboContext) -> cerbo_core::VaultContext {
    cerbo_core::VaultContext {
        vault_path: vault_ctx
            .config_dir
            .parent()
            .unwrap_or(&vault_ctx.config_dir)
            .to_path_buf(),
        global: global_ctx.clone(),
    }
}

/// Report a semantic index build on stderr, so stdout stays machine-readable.
fn print_build_progress(progress: cerbo_core::semantic::BuildProgress) {
    use cerbo_core::semantic::BuildProgress;
    match progress {
        BuildProgress::Started { objects } => {
            eprintln!("Building the semantic index ({} objects)...", objects)
        }
        BuildProgress::Finished { triples, .. } => {
            eprintln!("Semantic index ready ({} triples)", triples)
        }
        BuildProgress::Read { .. } | BuildProgress::Loaded { .. } => {}
    }
}

/// Report embedding progress on stderr, about every tenth of the pages.
fn print_embed_progress(done: usize, total: usize) {
    let step = (total / 10).max(1);
    if done == total || done.is_multiple_of(step) {
        eprintln!("Embedded {} of {} pages", done, total);
    }
}

/// The vaults a full `cerbo index` works on: every registered vault, plus the
/// vault around the current directory should it not be registered. A
/// registered vault whose directory is missing is skipped with a warning.
fn all_index_targets(
    ctx: &CerboContext,
    cwd_vault_root: Option<&std::path::Path>,
) -> Result<Vec<(String, std::path::PathBuf)>, String> {
    let mut targets = migration_targets(ctx, None)?;
    if let Some(root) = cwd_vault_root {
        let canonical = std::fs::canonicalize(root).ok();
        if !targets.iter().any(|(_, path)| std::fs::canonicalize(path).ok() == canonical) {
            targets.push((root.display().to_string(), root.to_path_buf()));
        }
    }
    if targets.is_empty() {
        return Err("no vaults to index: register one with `cerbo vault add` or run from inside a vault".to_string());
    }
    Ok(targets)
}

/// The vault `cerbo index --page <uuid>` works on: the vault around the current
/// directory if the object is there, otherwise the registered vault that holds
/// it. Without either, the current vault, which then reports the object as not
/// found.
fn page_index_target(
    ctx: &CerboContext,
    page_uuid: &str,
    cwd_vault_root: Option<&std::path::Path>,
) -> Result<(String, std::path::PathBuf), String> {
    let holds = |root: &std::path::Path| root.join(".cerbo").join("objects").join(page_uuid).is_dir();
    if let Some(root) = cwd_vault_root.filter(|root| holds(root)) {
        return Ok((root.display().to_string(), root.to_path_buf()));
    }
    if let Some(v) = cerbo_core::vault::vault_list(ctx)?.vaults.into_iter().find(|v| holds(&v.path)) {
        return Ok((v.id, v.path));
    }
    match cwd_vault_root {
        Some(root) => Ok((root.display().to_string(), root.to_path_buf())),
        None => Err(format!("Object not found in any vault: {}", page_uuid)),
    }
}

/// What `cerbo index` did in one vault.
#[derive(Default)]
struct IndexRun {
    stats: cerbo_core::metadata_index::IndexStats,
    slugs_backfilled: usize,
    embedded: Option<usize>,
    path_errors: usize,
    collisions: usize,
}

impl IndexRun {
    fn merge(&mut self, other: IndexRun) {
        self.stats.merge(other.stats);
        self.slugs_backfilled += other.slugs_backfilled;
        self.embedded = match (self.embedded, other.embedded) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(0) + b.unwrap_or(0)),
        };
        self.path_errors += other.path_errors;
        self.collisions += other.collisions;
    }
}

/// Run `cerbo index` on one vault: the page or the whole vault, slug
/// backfill, embeddings, and the virtual path checks. Warnings go to stderr.
fn index_one_vault(
    vault_ctx: &cerbo_core::VaultContext,
    page: Option<&str>,
    backfill_slugs: bool,
    embedder: Option<&mut dyn cerbo_core::semantic::Embedder>,
    options: cerbo_core::semantic::IndexOptions<'_>,
) -> Result<IndexRun, String> {
    use cerbo_core::metadata_index;

    let mut stats = match page {
        Some(page_uuid) => metadata_index::index_page(vault_ctx, page_uuid, options)?,
        None => metadata_index::index_vault(vault_ctx, options)?,
    };

    // Backfill missing slugs (full index only, unless --no-backfill-slug)
    let slugs_backfilled = if backfill_slugs {
        let backfill = metadata_index::backfill_slugs(vault_ctx)?;
        // A page the index already reported is not reported twice.
        for err in backfill.errors {
            if !stats.errors.iter().any(|e| e.page_uuid == err.page_uuid) {
                stats.errors.push(err);
            }
        }
        backfill.updated
    } else {
        0
    };

    // Embeddings cover the whole vault, also with --page: only pages
    // whose content changed since their last embedding are embedded.
    let embedded = match embedder {
        Some(embedder) => Some(cerbo_core::semantic::refresh_embeddings(vault_ctx, embedder, options)?.embedded),
        None => None,
    };

    // Validate virtual paths and report issues
    let path_errors = metadata_index::validate_virtual_paths(vault_ctx);
    for (uuid, msg) in &path_errors {
        eprintln!("Warning: page {}: {}", uuid, msg);
    }

    // Detect and report collisions
    let collisions = metadata_index::detect_path_collisions(vault_ctx);
    for (path, uuids) in &collisions {
        eprintln!("Warning: symlink collision at {:?}: {:?}", path, uuids);
    }

    Ok(IndexRun { stats, slugs_backfilled, embedded, path_errors: path_errors.len(), collisions: collisions.len() })
}

/// The vault `index`, `sparql`, `search` and `similar` work on: `--vault
/// <path>`, otherwise the vault found from the current directory.
fn path_vault_ctx(
    vault: Option<String>,
    cwd_vault_root: Option<&std::path::Path>,
) -> Result<cerbo_core::VaultContext, String> {
    let root = match (vault, cwd_vault_root) {
        (Some(path), _) => std::path::PathBuf::from(path),
        (None, Some(root)) => root.to_path_buf(),
        (None, None) => {
            return Err("not inside a cerbo vault (or any parent up to mount point); use --vault <path>".to_string())
        }
    };
    cerbo_core::VaultContext::from_path(root)
}

/// Run `cerbo search` or `cerbo similar` and print the pages found.
fn run_search(
    filters: SearchArgs,
    cwd_vault_root: Option<&std::path::Path>,
    find: impl FnOnce(
        &cerbo_core::VaultContext,
        &cerbo_core::semantic::SearchOptions,
        &mut dyn cerbo_core::semantic::Embedder,
        cerbo_core::semantic::IndexOptions<'_>,
    ) -> Result<Vec<cerbo_core::semantic::SearchHit>, cerbo_core::semantic::SemanticError>,
) -> Result<(), String> {
    let vault_ctx = path_vault_ctx(filters.vault, cwd_vault_root)?;
    check_old_vocabulary(&vault_ctx.global, &vault_ctx.vault_path);
    let search = cerbo_core::semantic::SearchOptions {
        limit: usize::try_from(filters.limit).unwrap_or(usize::MAX),
        types: filters.types,
        annotated: filters.annotated,
    };
    let options = cerbo_core::semantic::IndexOptions {
        progress: Some(&print_build_progress),
        embed_progress: Some(&print_embed_progress),
        ..Default::default()
    };
    // The model is loaded only once a text has to be embedded.
    let mut embedder = cerbo_core::semantic::default_embedder(&vault_ctx.global.cache_dir);
    let hits = find(&vault_ctx, &search, &mut embedder, options)?;
    if filters.json {
        print_json(&hits);
    } else {
        for hit in hits {
            println!("{:.3}  {}: {}", hit.score, hit.uuid, hit.title);
        }
    }
    Ok(())
}

// ── import-vault ──────────────────────────────────────────────────────────────

const NO_TARGET: &str = "no target vault: pass --vault <ID> or --new <PATH>";

/// Where `cerbo import-vault` writes.
#[derive(Debug)]
enum ImportTarget {
    /// A registered vault.
    Existing(cerbo_core::vault::Vault),
    /// A vault to create, initialise and register, once the plan succeeded.
    New { path: std::path::PathBuf, name: String },
}

/// `~/x` → `$HOME/x`, so a path typed at the prompt works as on a shell line.
fn expand_tilde(input: &str) -> std::path::PathBuf {
    match (input.strip_prefix("~/").or((input == "~").then_some("")), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => std::path::Path::new(&home).join(rest),
        _ => std::path::PathBuf::from(input),
    }
}

/// Ask on `out` (stderr) which vault to import into, reading answers from
/// `input` (stdin): the registered vaults by number, then "Create a new
/// vault", which asks for a path and a name. A wrong number asks again; end of
/// input gives up.
fn prompt_import_target(
    vaults: &[cerbo_core::vault::Vault],
    default_name: &str,
    input: &mut dyn std::io::BufRead,
    out: &mut dyn std::io::Write,
) -> Result<ImportTarget, String> {
    fn ask(question: &str, input: &mut dyn std::io::BufRead, out: &mut dyn std::io::Write) -> Result<String, String> {
        write!(out, "{}", question).and_then(|_| out.flush()).map_err(|e| e.to_string())?;
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) => Err(NO_TARGET.to_string()),
            Ok(_) => Ok(line.trim().to_string()),
            Err(e) => Err(format!("Failed to read the answer: {}", e)),
        }
    }

    let io = |e: std::io::Error| e.to_string();
    writeln!(out, "Import into which vault?").map_err(io)?;
    for (i, v) in vaults.iter().enumerate() {
        writeln!(out, "  {}. {} ({})", i + 1, v.name, v.path.display()).map_err(io)?;
    }
    let create = vaults.len() + 1;
    writeln!(out, "  {}. Create a new vault", create).map_err(io)?;

    let choice = loop {
        let answer = ask(&format!("Choose 1-{}: ", create), input, out)?;
        match answer.parse::<usize>() {
            Ok(n) if (1..=create).contains(&n) => break n,
            _ => writeln!(out, "Enter a number from 1 to {}.", create).map_err(io)?,
        }
    };
    if choice < create {
        return Ok(ImportTarget::Existing(vaults[choice - 1].clone()));
    }

    let path = loop {
        let answer = ask("Folder for the new vault: ", input, out)?;
        if !answer.is_empty() {
            break expand_tilde(&answer);
        }
    };
    let name = ask(&format!("Vault name [{}]: ", default_name), input, out)?;
    let name = if name.is_empty() { default_name.to_string() } else { name };
    Ok(ImportTarget::New { path, name })
}

/// `1536` → `1.5 KB`.
fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{} B", bytes);
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", value, UNITS[unit])
}

/// Print the human-readable import report on stdout; warnings go to stderr.
fn print_import_report(report: &cerbo_core::vault_import::ImportReport) {
    let count = |n: usize, word: &str| format!("{} {}{}", n, word, if n == 1 { "" } else { "s" });
    let what = format!(
        "{} and {} ({})",
        count(report.pages, "page"),
        count(report.attachments, "attachment"),
        format_bytes(report.attachment_bytes)
    );
    match &report.vault {
        Some(v) if !report.dry_run => {
            let label = match (&v.name, &v.id) {
                (Some(name), Some(id)) => format!("\"{}\" ({})", name, id),
                _ => v.path.clone(),
            };
            println!("Imported {} into {}", what, label);
            println!("Vault: {}", v.path);
        }
        _ => println!("Dry run: would import {}; nothing was written", what),
    }
    println!("Source: {} ({}, {})", report.source, report.format.display_name(), report.format_reason);
    let prefix = report.ontology.prefix.as_deref().map(|p| format!(", prefix {}", p)).unwrap_or_default();
    println!(
        "Ontology: {} ({}{}) <{}>",
        report.ontology.title, report.ontology.uuid, prefix, report.ontology.namespace
    );
    println!(
        "Links rewritten: {}, relations: {}, heading fragments dropped: {}",
        report.links, report.relations, report.dropped_fragments
    );
    if report.recovered > 0 {
        println!("Moved {} objects of an interrupted import to the trash", report.recovered);
    }

    let section = |title: &str, lines: Vec<String>| {
        if !lines.is_empty() {
            println!("{} ({}):", title, lines.len());
            for line in lines {
                // A YAML error spans several lines; keep them under their entry.
                println!("  {}", line.trim_end().replace('\n', "\n    "));
            }
        }
    };
    section("Skipped", report.skipped.clone());
    section("Unreferenced files, not imported", report.unreferenced.clone());
    section(
        "Unresolved links",
        report
            .unresolved
            .iter()
            .map(|u| match &u.property {
                Some(key) => format!("{}: {} (in {})", u.note, u.link, key),
                None => format!("{}: {}", u.note, u.link),
            })
            .collect(),
    );
    section(
        "Ambiguous links",
        report
            .ambiguous
            .iter()
            .map(|a| format!("{}: {} -> {} (candidates: {})", a.note, a.link, a.chosen, a.candidates.join(", ")))
            .collect(),
    );
    section(
        "Renamed slugs",
        report.renamed_slugs.iter().map(|r| format!("{}: {} -> {}", r.note, r.from, r.to)).collect(),
    );
    section(
        "Frontmatter kept as a yaml block",
        report.frontmatter_errors.iter().map(|f| format!("{}: {}", f.note, f.error)).collect(),
    );
    for warning in &report.warnings {
        eprintln!("Warning: {}", warning);
    }
}

/// Fail `cerbo import-vault`: a JSON error object with --json, else stderr.
fn import_vault_fail(json: bool, message: &str) -> ! {
    if json {
        print_json_error(message);
    } else {
        eprintln!("Error: {}", message);
    }
    std::process::exit(1);
}

/// Run `cerbo import-vault`. The order keeps every failure before the first
/// write: target flags are checked, then the plan is built (it only reads the
/// source), then the target is asked for or created, then the import runs.
#[allow(clippy::too_many_arguments)]
fn run_import_vault(
    ctx: &CerboContext,
    dir: &std::path::Path,
    from: Option<cerbo_core::vault_import::Format>,
    vault: Option<String>,
    new: Option<std::path::PathBuf>,
    name: Option<String>,
    force: bool,
    dry_run: bool,
    json: bool,
) -> Result<(), String> {
    use cerbo_core::vault_import::{
        ImportOptions, ReportVault, check_repeat, check_target_outside_source, plan_import, run_import,
    };
    use std::io::IsTerminal;

    // Checked before the (possibly long) scan, so a wrong flag fails at once.
    let mut target = match (vault, new) {
        (Some(id), _) => Some(ImportTarget::Existing(
            cerbo_core::vault::vault_list(ctx)?
                .vaults
                .into_iter()
                .find(|v| v.id == id)
                .ok_or_else(|| format!("Vault not found: {}", id))?,
        )),
        (None, Some(path)) => {
            cerbo_core::vault::check_new_vault_dir(&path)?;
            Some(ImportTarget::New { path, name: String::new() })
        }
        (None, None) if dry_run || std::io::stdin().is_terminal() => None,
        (None, None) => return Err(NO_TARGET.to_string()),
    };

    let plan = plan_import(dir, from)?;
    // A target from the prompt is checked once it is picked, below.
    match &target {
        Some(ImportTarget::Existing(v)) => check_target_outside_source(&plan.source, &v.path)?,
        Some(ImportTarget::New { path, .. }) => check_target_outside_source(&plan.source, path)?,
        None => {}
    }
    if dry_run {
        let report = plan.report();
        if json {
            print_json(&report);
        } else {
            print_import_report(&report);
        }
        return Ok(());
    }

    // The default vault name: the source folder's own name.
    let default_name = plan
        .source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Imported vault".to_string());
    if let Some(ImportTarget::New { name: n, .. }) = &mut target {
        *n = name.unwrap_or_else(|| default_name.clone());
    }
    let target = match target {
        Some(target) => target,
        None => {
            let vaults = cerbo_core::vault::vault_list(ctx)?.vaults;
            let stdin = std::io::stdin();
            prompt_import_target(&vaults, &default_name, &mut stdin.lock(), &mut std::io::stderr())?
        }
    };

    let vault = match target {
        ImportTarget::Existing(v) => {
            check_target_outside_source(&plan.source, &v.path)?;
            if !v.path.join(".cerbo").is_dir() {
                return Err(format!("Not a Cerbo vault (no .cerbo/ directory): {}", v.path.display()));
            }
            // Read-only, so a refused repeat import writes nothing at all,
            // not even the vocabulary migration below.
            if !force {
                let vault_ctx = cerbo_core::VaultContext { vault_path: v.path.clone(), global: ctx.clone() };
                check_repeat(&vault_ctx, &plan)?;
            }
            check_old_vocabulary(ctx, &v.path);
            v
        }
        ImportTarget::New { path, name } => {
            check_target_outside_source(&plan.source, &path)?;
            let (v, init) = cerbo_core::vault::vault_create(ctx, &path, name, true)?;
            for (bundle, result) in &init.bundled {
                if let Err(e) = result {
                    eprintln!("Warning: Failed to bundle {}: {}", bundle, e);
                }
            }
            eprintln!("Created vault \"{}\" ({}) in {}", v.name, v.id, v.path.display());
            v
        }
    };

    let vault_ctx = cerbo_core::VaultContext { vault_path: vault.path.clone(), global: ctx.clone() };
    let mut report = run_import(&vault_ctx, &plan, ImportOptions { force })?;
    let path = report.vault.as_ref().map(|v| v.path.clone()).unwrap_or_else(|| vault.path.display().to_string());
    report.vault = Some(ReportVault { id: Some(vault.id), name: Some(vault.name), path });
    if json {
        print_json(&report);
    } else {
        print_import_report(&report);
    }
    Ok(())
}

/// Always returns the XDG global context used for vault registry operations.
fn get_context() -> Result<CerboContext, String> {
    let core = CoreContext::new()?;
    let ctx = CerboContext {
        config_dir: core.config_dir.clone(),
        cache_dir: core.cache_dir.clone(),
    };
    let _ = cerbo_core::migration::migrate_if_needed(&ctx)?;
    if !ctx.config_dir.join("vaults.toml").exists() {
        cerbo_core::config::save_config(&ctx, &cerbo_core::config::Config::default())?;
    }
    if !ctx.config_dir.join("ui.toml").exists() {
        cerbo_core::ui_settings::save_ui_settings(
            &ctx,
            &cerbo_core::ui_settings::UiSettings::default(),
        )?;
    }
    if !ctx.cache_dir.join("state.toml").exists() {
        cerbo_core::state::save_state(&ctx, &cerbo_core::state::State::default())?;
    }
    let _ = cerbo_core::config::load_config(&ctx)?;
    let _ = cerbo_core::ui_settings::load_ui_settings(&ctx)?;
    let _ = cerbo_core::state::load_state(&ctx)?;
    Ok(ctx)
}

/// Resolve which vault context to use for content operations.
///
/// Priority (highest to lowest):
///   1. Explicit `--vault <id>` flag
///   2. CWD-discovered registered vault
///   3. CWD vault root (unregistered — use directly)
///   4. Active vault from persisted state
///   5. Single registered vault fallback
///   6. Error
fn resolve_vault_ctx(
    global_ctx: &CerboContext,
    explicit_vault_id: Option<&str>,
    cwd_vault_id: Option<&str>,
    cwd_vault_root: Option<&std::path::Path>,
) -> Result<CerboContext, String> {
    use cerbo_core::vault;
    use cerbo_core::state;

    let make_ctx = |path: std::path::PathBuf| CerboContext {
        config_dir: path.join(".cerbo"),
        cache_dir: global_ctx.cache_dir.clone(),
    };

    // 1. Explicit --vault flag
    if let Some(id) = explicit_vault_id {
        let path = vault::get_vault_path(global_ctx, id)
            .ok_or_else(|| format!("vault not found: {}", id))?;
        return Ok(make_ctx(path));
    }
    // 2. CWD-discovered registered vault
    if let Some(id) = cwd_vault_id {
        let path = vault::get_vault_path(global_ctx, id)
            .ok_or_else(|| format!("vault not found: {}", id))?;
        return Ok(make_ctx(path));
    }
    // 3. CWD vault root even if not registered
    if let Some(root) = cwd_vault_root {
        return Ok(make_ctx(root.to_path_buf()));
    }
    // 4. Active vault from state
    let st = state::load_state(global_ctx)?;
    if let Some(id) = st.active_vault_id {
        let path = vault::get_vault_path(global_ctx, &id)
            .ok_or_else(|| format!("vault not found: {}", id))?;
        return Ok(make_ctx(path));
    }
    // 5. Single registered vault
    let reg = vault::vault_list(global_ctx)?;
    if reg.vaults.len() == 1 {
        return Ok(make_ctx(reg.vaults[0].path.clone()));
    }
    Err("not a cerbo vault (or any parent up to mount point); use --vault <id> or run from inside a vault".to_string())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let ctx = get_context()?;

    // CWD vault discovery — runs once, results propagated to all commands.
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    let cwd_vault_root = cerbo_core::vault::find_vault_root(&cwd);
    let mut cwd_vault_id = cwd_vault_root
        .as_deref()
        .and_then(|root| cerbo_core::vault::vault_id_from_path(&ctx, root));
    // `cerbo migrate` works on registered vaults or an explicit --vault, never
    // on the current directory, and the desktop launcher runs it from wherever
    // the app was started: registering that directory's vault would be a side
    // effect nobody asked for, and the migration would then rewrite it.
    // `cerbo import-vault` picks its target from --vault, --new or the prompt,
    // never from the current directory, and a refused import writes nothing.
    let registers_cwd_vault = !matches!(cli.command, Commands::Migrate { .. } | Commands::ImportVault { .. });
    if let (true, Some(root), None) = (registers_cwd_vault, cwd_vault_root.as_ref(), cwd_vault_id.as_ref()) {
        let _ = cerbo_core::vault::auto_vault_register(&ctx, root);
        cwd_vault_id = cerbo_core::vault::vault_id_from_path(&ctx, root);
    }

    match cli.command {
        Commands::Init { json } => {
            let current_dir = std::env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?;
            let cerbo_dir = current_dir.join(".cerbo");

            // Also runs on an already-initialised vault, so an existing one
            // picks up any newly required ignore rule.
            let init = cerbo_core::vault::vault_init(&ctx.cache_dir, &current_dir, true)?;
            if !init.created {
                if json {
                    print_json_success("Vault already initialized");
                } else {
                    println!("Vault already initialized in {}", cerbo_dir.display());
                }
            } else {
                if !json {
                    for (name, result) in &init.bundled {
                        match result {
                            Ok(uuid) => println!("Bundled {} ontology with UUID: {}", name, uuid),
                            Err(e) => println!("Warning: Failed to bundle {}: {}", name, e),
                        }
                    }
                }
                if json {
                    print_json_success("Vault initialized with bundled ontologies");
                } else {
                    println!("Vault initialized in {}", cerbo_dir.display());
                }
            }
        }
        Commands::Vault { action } => match action {
            VaultCommands::List { json } => {
                let vaults_file = cerbo_core::vault::vault_list(&ctx)?;
                if json {
                    let vaults_json: Vec<VaultJson> = vaults_file.vaults.into_iter().map(|v| VaultJson {
                        id: v.id,
                        name: v.name,
                        path: v.path.to_string_lossy().to_string(),
                        is_auto: v.is_auto,
                    }).collect();
                    print_json(&vaults_json);
                } else {
                    // Print compact human-readable vault list
                    print_vaults_template(&ctx, &vaults_file)?;
                }
            }
            VaultCommands::Add { name, path, json } => {
                let vault = cerbo_core::vault::vault_add(&ctx, name, path)?;
                if json {
                    let vault_json = VaultJson {
                        id: vault.id,
                        name: vault.name,
                        path: vault.path.to_string_lossy().to_string(),
                        is_auto: vault.is_auto,
                    };
                    print_json(&vault_json);
                } else {
                    // Indicate added vault in compact format
                    let current_id = cerbo_core::state::load_state(&ctx)?.active_vault_id;
                    let mut current_marker = "";
                    if Some(vault.id.clone()) == current_id {
                        current_marker = " (current)";
                    }
                    println!("{}: {}{}", vault.id, vault.name, current_marker);
                }
            }
            VaultCommands::Remove { id, json } => {
                cerbo_core::vault::vault_remove(&ctx, id)?;
                if json {
                    print_json_success("Vault removed");
                } else {
                    println!("Removed vault");
                }
            }
            VaultCommands::Active { id, json } => {
                cerbo_core::vault::vault_set_active(&ctx, id)?;
                if json {
                    print_json_success("Active vault set");
                } else {
                    println!("Set active vault");
                }
            }
            VaultCommands::Approve { id, json } => {
                let vault = cerbo_core::vault::auto_vault_approve(&ctx, &id)?;
                if json {
                    let vault_json = VaultJson {
                        id: vault.id,
                        name: vault.name,
                        path: vault.path.to_string_lossy().to_string(),
                        is_auto: vault.is_auto,
                    };
                    print_json(&vault_json);
                } else {
                    println!("Approved vault: {} ({})", vault.name, vault.id);
                }
            }
        },
        Commands::Page { action } => match action {
            PageCommands::List { json, vault } => {
                let eff_ctx = resolve_vault_ctx(
                    &ctx,
                    vault.as_deref(),
                    cwd_vault_id.as_deref(),
                    cwd_vault_root.as_deref(),
                )?;
                check_old_vocabulary(&ctx, vault_root_of(&eff_ctx));
                let pages = cerbo_core::page::page_list(&eff_ctx)?;

                if json {
                    print_json(&pages);
                } else {
                    print_pages_template(&ctx, &eff_ctx, &pages)?;
                }
            }
            PageCommands::Create { title, json } => {
                let eff_ctx = resolve_vault_ctx(
                    &ctx,
                    None,
                    cwd_vault_id.as_deref(),
                    cwd_vault_root.as_deref(),
                )?;
                check_old_vocabulary(&ctx, vault_root_of(&eff_ctx));
                let uuid = cerbo_core::object::object_create(&eff_ctx, None, cerbo_core::object::ObjectType::Product, title)?;
                if json {
                    print_json(&serde_json::json!({"uuid": uuid}));
                } else {
                    println!("Created page with UUID: {}", uuid);
                }
            }
            PageCommands::Read { uuid, json } => {
                let eff_ctx = resolve_vault_ctx(
                    &ctx,
                    None,
                    cwd_vault_id.as_deref(),
                    cwd_vault_root.as_deref(),
                )?;
                check_old_vocabulary(&ctx, vault_root_of(&eff_ctx));
                let content = cerbo_core::page::page_read(&eff_ctx, uuid)?;
                if json {
                    print_json(&serde_json::json!({"content": content}));
                } else {
                    println!("{}", content);
                }
            }
            PageCommands::Write { uuid, content, json } => {
                let eff_ctx = resolve_vault_ctx(
                    &ctx,
                    None,
                    cwd_vault_id.as_deref(),
                    cwd_vault_root.as_deref(),
                )?;
                check_old_vocabulary(&ctx, vault_root_of(&eff_ctx));
                let _ = cerbo_core::page::page_write(&eff_ctx, uuid, content)?;
                if json {
                    print_json_success("Page updated");
                } else {
                    println!("Updated page");
                }
            }
            PageCommands::Delete { uuid, json } => {
                let eff_ctx = resolve_vault_ctx(
                    &ctx,
                    None,
                    cwd_vault_id.as_deref(),
                    cwd_vault_root.as_deref(),
                )?;
                check_old_vocabulary(&ctx, vault_root_of(&eff_ctx));
                cerbo_core::page::page_delete(&eff_ctx, uuid)?;
                if json {
                    print_json_success("Page deleted");
                } else {
                    println!("Deleted page");
                }
            }
            PageCommands::Move { uuid, path, root, json } => {
                let eff_ctx = resolve_vault_ctx(
                    &ctx,
                    None,
                    cwd_vault_id.as_deref(),
                    cwd_vault_root.as_deref(),
                )?;
                check_old_vocabulary(&ctx, vault_root_of(&eff_ctx));
                let target = if root { None } else { path }.filter(|p| !p.is_empty());
                cerbo_core::page::page_update_virtual_path(&eff_ctx, uuid.clone(), target.clone())?;
                if json {
                    print_json(&serde_json::json!({"uuid": uuid, "virtualPath": target}));
                } else {
                    println!("Moved {} to {}", uuid, target.as_deref().unwrap_or("root"));
                }
            }
            PageCommands::Backlinks { uuid, json, vault } => {
                let eff_ctx = resolve_vault_ctx(
                    &ctx,
                    vault.as_deref(),
                    cwd_vault_id.as_deref(),
                    cwd_vault_root.as_deref(),
                )?;
                check_old_vocabulary(&ctx, vault_root_of(&eff_ctx));
                let vault_ctx = index_vault_ctx(&ctx, &eff_ctx);
                let options = cerbo_core::semantic::IndexOptions {
                    progress: Some(&print_build_progress),
                    ..Default::default()
                };
                let entries = cerbo_core::semantic::backlinks(&vault_ctx, &uuid, options)?;
                if json {
                    let entries_json: Vec<PageJson> = entries.into_iter().map(|e| PageJson {
                        uuid: e.uuid,
                        title: e.title,
                    }).collect();
                    print_json(&entries_json);
                } else {
                    for e in entries {
                        println!("{}: {}", e.uuid, e.title);
                    }
                }
            }
        },
        Commands::Resolve { uuid, json } => {
            let eff_ctx = resolve_vault_ctx(
                &ctx,
                None,
                cwd_vault_id.as_deref(),
                cwd_vault_root.as_deref(),
            )?;
            check_old_vocabulary(&ctx, vault_root_of(&eff_ctx));
            let obj_path = cerbo_core::object::object_path(&eff_ctx, &uuid);
            if !obj_path.exists() {
                if json {
                    print_json_error(&format!("Object not found: {}", uuid));
                } else {
                    eprintln!("Error: Object not found: {}", uuid);
                }
                std::process::exit(1);
            }
            if json {
                print_json(&serde_json::json!({"path": obj_path.to_string_lossy().to_string()}));
            } else {
                println!("{}", obj_path.to_string_lossy());
            }
        },
        Commands::Import { url, json } => {
            let eff_ctx = resolve_vault_ctx(
                &ctx,
                None,
                cwd_vault_id.as_deref(),
                cwd_vault_root.as_deref(),
            )?;
            check_old_vocabulary(&ctx, vault_root_of(&eff_ctx));
            let uuid = cerbo_core::object::object_import(&eff_ctx, &url)?;
            if json {
                print_json(&serde_json::json!({"uuid": uuid, "url": url}));
            } else {
                println!("Imported URL as Source object with UUID: {}", uuid);
            }
        },
        Commands::ImportOntology { url, prefix, namespace, json } => {
            let eff_ctx = resolve_vault_ctx(
                &ctx,
                None,
                cwd_vault_id.as_deref(),
                cwd_vault_root.as_deref(),
            )?;
            check_old_vocabulary(&ctx, vault_root_of(&eff_ctx));
            // clap guarantees both flags or neither.
            let registration = prefix.as_deref().zip(namespace.as_deref());
            let uuid = cerbo_core::object::object_import_ontology(&eff_ctx, &url, registration)?;
            if registration.is_none() {
                eprintln!(
                    "Warning: ontology-map.json was not changed; annotations cannot use this \
                     ontology until it is registered with --prefix and --namespace"
                );
            }
            if json {
                print_json(&serde_json::json!({
                    "uuid": uuid,
                    "url": url,
                    "prefix": prefix,
                    "namespace": namespace,
                }));
            } else {
                println!("Imported ontology from {} with UUID: {}", url, uuid);
                if let (Some(prefix), Some(namespace)) = (&prefix, &namespace) {
                    println!("Registered prefix {}: <{}>", prefix, namespace);
                }
            }
        },
        Commands::ImportVault { dir, from, vault, new, name, force, dry_run, json } => {
            if let Err(e) = run_import_vault(&ctx, &dir, from, vault, new, name, force, dry_run, json) {
                import_vault_fail(json, &e);
            }
        },
        Commands::Index { page, vault, no_backfill_slug, embeddings, json } => {
            if page.is_some() && vault.is_some() {
                eprintln!("Error: Cannot specify both --vault and --page");
                std::process::exit(2);
            }
            // --page: the vault holding it; --vault: that vault (ID or path);
            // otherwise every vault.
            let targets = match (page.as_deref(), vault.as_deref()) {
                (Some(page_uuid), _) => vec![page_index_target(&ctx, page_uuid, cwd_vault_root.as_deref())?],
                (None, Some(target)) => migration_targets(&ctx, Some(target))?,
                (None, None) => all_index_targets(&ctx, cwd_vault_root.as_deref())?,
            };

            let options = cerbo_core::semantic::IndexOptions {
                progress: Some(&print_build_progress),
                embed_progress: Some(&print_embed_progress),
                ..Default::default()
            };
            // One embedder for every vault; the model loads on first use.
            let mut embedder = embeddings.then(|| cerbo_core::semantic::default_embedder(&ctx.cache_dir));
            let mut run = IndexRun::default();
            // A vault that fails does not stop the others: each is reported
            // after the summary, and the run then fails.
            let mut vault_errors: Vec<String> = Vec::new();
            for (label, root) in &targets {
                if targets.len() > 1 {
                    eprintln!("Vault {} ({}):", label, root.display());
                }
                // Before indexing, so the index reads the migrated files.
                check_old_vocabulary(&ctx, root);
                let embedder = embedder.as_mut().map(|e| e as &mut dyn cerbo_core::semantic::Embedder);
                let result = cerbo_core::VaultContext::from_path(root.clone()).and_then(|vault_ctx| {
                    index_one_vault(&vault_ctx, page.as_deref(), !no_backfill_slug, embedder, options)
                });
                match result {
                    Ok(one) => run.merge(one),
                    Err(e) if targets.len() > 1 => vault_errors.push(format!("vault {} ({}): {}", label, root.display(), e)),
                    Err(e) => vault_errors.push(e),
                }
            }
            let IndexRun { stats, slugs_backfilled, embedded, path_errors, collisions } = run;

            if json {
                let mut summary = serde_json::json!({
                    "pages_processed": stats.pages_processed,
                    "links_found": stats.links_found,
                    "annotations_found": stats.annotations_found,
                    "triples": stats.triples,
                    "legacy_backrefs_removed": stats.legacy_backrefs_removed,
                    "slugs_backfilled": slugs_backfilled,
                    "path_errors": path_errors,
                    "collisions": collisions,
                    "errors": stats.errors.len(),
                    "vault_errors": vault_errors.len()
                });
                if let Some(embedded) = embedded {
                    summary["embedded"] = serde_json::json!(embedded);
                }
                print_json(&summary);
            } else {
                println!("Indexed {} pages", stats.pages_processed);
                println!("Found {} links, {} annotations", stats.links_found, stats.annotations_found);
                println!("Semantic index: {} triples", stats.triples);
                if stats.legacy_backrefs_removed > 0 {
                    println!("Removed {} legacy backrefs.ttl files", stats.legacy_backrefs_removed);
                }
                if let Some(embedded) = embedded {
                    println!("Embedded {} pages", embedded);
                }
                if slugs_backfilled > 0 {
                    println!("Backfilled {} missing slugs", slugs_backfilled);
                }
                if path_errors > 0 {
                    eprintln!("Virtual path errors: {}", path_errors);
                }
                if collisions > 0 {
                    eprintln!("Symlink collisions detected: {}", collisions);
                }
            }

            // An object that could not be read or written leaves the index
            // incomplete, so say which one and fail — in both output modes.
            if !stats.errors.is_empty() {
                eprintln!("Errors: {}", stats.errors.len());
                for err in &stats.errors {
                    eprintln!("  {}: {}", err.page_uuid, err.error_message);
                }
            }
            // The files are written either way; an index that could not be
            // updated is still a failed run.
            for e in stats.index_errors.iter().chain(&vault_errors) {
                eprintln!("Error: {}", e);
            }
            if !stats.errors.is_empty() || !stats.index_errors.is_empty() || !vault_errors.is_empty() {
                std::process::exit(1);
            }
        },
        Commands::Search { text, filters } => {
            run_search(filters, cwd_vault_root.as_deref(), |vault_ctx, search, embedder, options| {
                cerbo_core::semantic::search(vault_ctx, &text, search, embedder, options)
            })?;
        },
        Commands::Similar { uuid, filters } => {
            run_search(filters, cwd_vault_root.as_deref(), |vault_ctx, search, embedder, options| {
                cerbo_core::semantic::similar(vault_ctx, &uuid, search, embedder, options)
            })?;
        },
        Commands::Sparql { query, file, vault, json } => {
            use std::io::Read;

            let vault_ctx = path_vault_ctx(vault, cwd_vault_root.as_deref())?;
            check_old_vocabulary(&ctx, &vault_ctx.vault_path);

            // clap guarantees exactly one of the query argument and --file.
            let sparql = match (query.as_deref(), file) {
                (_, Some(path)) => std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?,
                (Some("-"), None) => {
                    let mut buf = String::new();
                    std::io::stdin()
                        .read_to_string(&mut buf)
                        .map_err(|e| format!("Failed to read the query from stdin: {}", e))?;
                    buf
                }
                (Some(q), None) => q.to_string(),
                (None, None) => unreachable!("clap requires a query or --file"),
            };

            let options = cerbo_core::semantic::IndexOptions {
                progress: Some(&print_build_progress),
                ..Default::default()
            };
            match cerbo_core::semantic::query(&vault_ctx, &sparql, options) {
                Ok(output) if json => print!("{}", output.to_json()),
                Ok(output) => print!("{}", output.to_text()),
                // Printed as is: a syntax error's message spans its position.
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        },
        Commands::Folder { action } => match action {
            FolderCommands::Rename { old, new, json } => {
                let eff_ctx = resolve_vault_ctx(
                    &ctx,
                    None,
                    cwd_vault_id.as_deref(),
                    cwd_vault_root.as_deref(),
                )?;
                check_old_vocabulary(&ctx, vault_root_of(&eff_ctx));
                let report = cerbo_core::page::folder_rename(&eff_ctx, &old, &new)?;
                for (uuid, err) in &report.failed {
                    eprintln!("Error: could not move {uuid}: {err}");
                }
                if json {
                    print_json(&serde_json::json!({
                        "from": old,
                        "to": new,
                        "pages": report.changed.len(),
                        "failed": report.failed.iter().map(|(u, _)| u).collect::<Vec<_>>(),
                    }));
                } else {
                    println!("Renamed {} to {} ({} pages)", old, new, report.changed.len());
                }
                if !report.failed.is_empty() {
                    return Err(format!("{} pages could not be moved", report.failed.len()));
                }
            }
        },
        Commands::Migrate { action } => match action {
            MigrateCommands::Vocabulary { vault, dry_run, if_needed: true, json } => {
                // What the CLI does before every command that writes to a
                // vault, for every target; the desktop launcher runs this
                // before the app starts. Only the objects found in the old
                // format count as scanned. With an app already running nothing
                // is migrated, and that is not an error.
                let (mut scanned, mut rewritten, mut errors) = (0usize, 0usize, 0usize);
                debug_assert!(!dry_run, "clap rejects --dry-run with --if-needed");
                for (_, root) in migration_targets(&ctx, vault.as_deref())? {
                    let (s, r, e) = check_old_vocabulary(&ctx, &root);
                    scanned += s;
                    rewritten += r;
                    errors += e;
                }
                if json {
                    print_json(&serde_json::json!({
                        "scanned": scanned,
                        "rewritten": rewritten,
                        "errors": errors
                    }));
                }
                if errors > 0 {
                    std::process::exit(1);
                }
            }
            MigrateCommands::Vocabulary { vault, dry_run, if_needed: false, json } => {
                use cerbo_core::VaultContext;
                use cerbo_core::vocabulary_migration::migrate_vocabulary;

                // A running desktop app may be saving these files; rewriting
                // them under it could put back what it just replaced.
                let _lock = if dry_run {
                    None
                } else {
                    match cerbo_core::app_lock::AppLock::try_exclusive(&ctx.cache_dir)? {
                        Some(lock) => Some(lock),
                        None => {
                            eprintln!(
                                "Error: Cerbo desktop or another migration is running; close it and run `cerbo migrate vocabulary` again"
                            );
                            std::process::exit(1);
                        }
                    }
                };
                let (mut scanned, mut rewritten, mut errors) = (0usize, 0usize, 0usize);
                for (label, root) in migration_targets(&ctx, vault.as_deref())? {
                    let report = match VaultContext::from_path(root).and_then(|v| migrate_vocabulary(&v, dry_run)) {
                        Ok(report) => report,
                        Err(e) => {
                            // A vault that cannot be migrated at all is one error;
                            // the other vaults still go ahead.
                            eprintln!("Error: vault {}: {}", label, e);
                            errors += 1;
                            continue;
                        }
                    };

                    for warning in &report.warnings {
                        eprintln!("Warning: {}", warning);
                    }
                    for err in &report.errors {
                        eprintln!("Error: object {}: {}", err.uuid, err.message);
                    }
                    if !json {
                        let verb = if dry_run { "would rewrite" } else { "rewrote" };
                        println!(
                            "Vault {}: scanned {} objects, {} {} files, {} errors",
                            label, report.scanned, verb, report.rewritten, report.errors.len()
                        );
                        for file in &report.files {
                            println!("  {} {}", verb, file.display());
                        }
                    }

                    scanned += report.scanned;
                    rewritten += report.rewritten;
                    errors += report.errors.len();
                }

                if json {
                    print_json(&serde_json::json!({
                        "scanned": scanned,
                        "rewritten": rewritten,
                        "errors": errors
                    }));
                } else {
                    let verb = if dry_run { "Would rewrite" } else { "Rewrote" };
                    println!("Scanned {} objects. {} {} files. {} errors.", scanned, verb, rewritten, errors);
                }

                // Unreadable objects were left untouched; say so with the exit code.
                if errors > 0 {
                    std::process::exit(1);
                }
            }
        },
        Commands::Symlink { vault, json } => {
            use cerbo_core::vault_symlink;

            // Explicit --vault path takes priority; otherwise use CWD-discovered root.
            let vault_root = if let Some(vault_path) = vault {
                let p = std::path::PathBuf::from(&vault_path);
                if !p.join(".cerbo").is_dir() {
                    if json {
                        print_json_error("No .cerbo/ directory at the specified path");
                    } else {
                        eprintln!("Error: no .cerbo/ directory at {}", p.display());
                    }
                    std::process::exit(1);
                }
                p
            } else {
                match cwd_vault_root.clone() {
                    Some(root) => root,
                    None => {
                        if json {
                            print_json_error("Not inside a Cerbo vault (no .cerbo/ found)");
                        } else {
                            eprintln!("Error: not inside a Cerbo vault. Run 'cerbo init' or use --vault <path>.");
                        }
                        std::process::exit(1);
                    }
                }
            };

            check_old_vocabulary(&ctx, &vault_root);
            match vault_symlink::materialize(&vault_root) {
                Ok(report) => {
                    if json {
                        print_json(&serde_json::json!({
                            "objects_scanned": report.objects_scanned,
                            "leaves_created": report.leaves_created,
                            "dirs_created": report.dirs_created,
                            "path": vault_root.join("cerbo").to_string_lossy()
                        }));
                    } else {
                        println!(
                            "Symlink tree built: {} objects, {} symlinks, {} dirs",
                            report.objects_scanned, report.leaves_created, report.dirs_created
                        );
                        println!("{}", vault_root.join("cerbo").display());
                    }
                }
                Err(vault_symlink::SymlinkError::Conflict { collisions }) => {
                    if json {
                        let col: Vec<_> = collisions.iter().map(|c| serde_json::json!({
                            "path": c.path.to_string_lossy(),
                            "uuids": c.uuids
                        })).collect();
                        print_json_error(&format!("Symlink conflicts: {}", serde_json::to_string(&col).unwrap_or_default()));
                    } else {
                        eprintln!("Error: symlink conflicts detected:");
                        for c in &collisions {
                            eprintln!("  {:?}: {:?}", c.path, c.uuids);
                        }
                    }
                    std::process::exit(1);
                }
                Err(vault_symlink::SymlinkError::UnsafeWipe { offenders }) => {
                    if json {
                        let off: Vec<_> = offenders.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                        print_json_error(&format!("Unsafe wipe — non-cerbo entries in cerbo/: {:?}", off));
                    } else {
                        eprintln!("Error: cerbo/ contains non-symlink entries not owned by cerbo:");
                        for p in &offenders {
                            eprintln!("  {}", p.display());
                        }
                        eprintln!("Remove them manually and retry.");
                    }
                    std::process::exit(1);
                }
                Err(e) => {
                    let msg = match e {
                        vault_symlink::SymlinkError::Io(io_err) => io_err.to_string(),
                        vault_symlink::SymlinkError::Other(s) => s,
                        _ => unreachable!(),
                    };
                    if json {
                        print_json_error(&msg);
                    } else {
                        eprintln!("Error: {}", msg);
                    }
                    std::process::exit(1);
                }
            }
        },
        Commands::Info { json } => {
            use cerbo_core::vault;

            fn display_path(p: &std::path::Path) -> String {
                if let Ok(home) = std::env::var("HOME")
                    && let Some(rest) = p.to_str().and_then(|s| s.strip_prefix(&home)) {
                        return format!("~{}", rest);
                    }
                p.to_string_lossy().to_string()
            }

            if json {
                let reg = vault::vault_list(&ctx)?;
                let vaults_json: Vec<VaultInfoJson> = reg.vaults.into_iter().map(|v| {
                    let count = vault::vault_page_count(&ctx, &v.id).unwrap_or(0);
                    VaultInfoJson {
                        id: v.id,
                        name: v.name,
                        path: display_path(&v.path),
                        page_count: count,
                        is_auto: v.is_auto,
                    }
                }).collect();
                let info_json = InfoJson {
                    config_dir: display_path(&ctx.config_dir),
                    cache_dir: display_path(&ctx.cache_dir),
                    vaults: vaults_json,
                };
                print_json(&info_json);
            } else {
                println!("Config:  {}", display_path(&ctx.config_dir));
                println!("Cache:   {}", display_path(&ctx.cache_dir));
                println!();

                let reg = vault::vault_list(&ctx)?;
                if reg.vaults.is_empty() {
                    println!("No vaults registered");
                } else {
                    println!("Vaults: {} registered", reg.vaults.len());
                    for v in &reg.vaults {
                        let count = vault::vault_page_count(&ctx, &v.id).unwrap_or(0);
                        let current = cwd_vault_id.as_deref() == Some(v.id.as_str());
                        let marker = if current { " (current)" } else { "" };
                        println!("├── {}{} ({}) - {} pages", v.name, marker, display_path(&v.path), count);
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct VaultJson {
    id: String,
    name: String,
    path: String,
    is_auto: bool,
}

#[derive(Serialize)]
struct VaultInfoJson {
    id: String,
    name: String,
    path: String,
    page_count: usize,
    is_auto: bool,
}

#[derive(Serialize)]
struct InfoJson {
    config_dir: String,
    cache_dir: String,
    vaults: Vec<VaultInfoJson>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cerbo_core::vault::Vault;

    fn vaults() -> Vec<Vault> {
        ["Work", "Home"]
            .iter()
            .enumerate()
            .map(|(i, name)| Vault {
                id: format!("id-{}", i + 1),
                name: name.to_string(),
                path: std::path::PathBuf::from(format!("/vaults/{}", name.to_lowercase())),
                is_auto: false,
            })
            .collect()
    }

    /// Run the prompt on `answers`, returning its result and what it printed.
    fn prompt(answers: &str) -> (Result<ImportTarget, String>, String) {
        let mut out = Vec::new();
        let result = prompt_import_target(&vaults(), "Notes", &mut answers.as_bytes(), &mut out);
        (result, String::from_utf8(out).unwrap())
    }

    #[test]
    fn prompt_lists_vaults_and_picks_one() {
        let (result, out) = prompt("2\n");
        assert!(out.contains("  1. Work (/vaults/work)"), "{out}");
        assert!(out.contains("  2. Home (/vaults/home)"), "{out}");
        assert!(out.contains("  3. Create a new vault"), "{out}");
        match result.unwrap() {
            ImportTarget::Existing(v) => assert_eq!(v.id, "id-2"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn prompt_asks_again_after_a_wrong_number() {
        let (result, out) = prompt("0\nfoo\n1\n");
        assert_eq!(out.matches("Enter a number from 1 to 3.").count(), 2, "{out}");
        assert!(matches!(result.unwrap(), ImportTarget::Existing(v) if v.id == "id-1"));
    }

    #[test]
    fn prompt_creates_a_new_vault_with_path_and_name() {
        let (result, _) = prompt("3\n/tmp/notes\nMy Notes\n");
        match result.unwrap() {
            ImportTarget::New { path, name } => {
                assert_eq!(path, std::path::PathBuf::from("/tmp/notes"));
                assert_eq!(name, "My Notes");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn prompt_new_vault_name_defaults_to_the_source_folder() {
        let (result, out) = prompt("3\n\n/tmp/notes\n\n");
        assert!(out.contains("Vault name [Notes]: "), "{out}");
        assert!(matches!(result.unwrap(), ImportTarget::New { name, .. } if name == "Notes"));
    }

    #[test]
    fn prompt_fails_at_end_of_input() {
        assert_eq!(prompt("").0.unwrap_err(), NO_TARGET);
        assert_eq!(prompt("3\n/tmp/x\n").0.unwrap_err(), NO_TARGET);
    }

    #[test]
    fn prompt_with_no_registered_vault_offers_only_create() {
        let mut out = Vec::new();
        let result = prompt_import_target(&[], "Notes", &mut "1\n~/n\nN\n".as_bytes(), &mut out);
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("  1. Create a new vault"), "{out}");
        assert!(matches!(result.unwrap(), ImportTarget::New { .. }));
    }

    #[test]
    fn tilde_expands_to_home() {
        let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
        if let Some(home) = home {
            assert_eq!(expand_tilde("~/cerbo/notes"), home.join("cerbo/notes"));
            assert_eq!(expand_tilde("~"), home.join(""));
        }
        assert_eq!(expand_tilde("rel/~x"), std::path::PathBuf::from("rel/~x"));
    }

    #[test]
    fn bytes_are_human_readable() {
        assert_eq!(format_bytes(137), "137 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(3 * 1024 * 1024), "3.0 MB");
    }
}
