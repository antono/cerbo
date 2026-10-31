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
        println!("{}: {}", p.uuid, p.title);
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
#[command(about = "A local-first markdown wiki CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new vault (creates .cerbo/ directory)
    Init {
        #[arg(long)]
        json: bool,
    },
    /// Vault management
    Vault {
        #[command(subcommand)]
        action: VaultCommands,
    },
    /// Page management
    Page {
        #[command(subcommand)]
        action: PageCommands,
    },
    /// Resolve object UUID to local filesystem path
    Resolve {
        uuid: String,
        #[arg(long)]
        json: bool,
    },
    /// Show configuration and vault info
    Info {
        #[arg(long)]
        json: bool,
    },
    /// Import URL as Source object (read-only)
    Import {
        url: String,
        #[arg(long)]
        json: bool,
    },
    /// Import ontology (creates an Ontology object, rdf:type schema:DefinedTermSet)
    ImportOntology {
        url: String,
        /// Annotation prefix to register for this ontology (needs --namespace)
        #[arg(long, requires = "namespace")]
        prefix: Option<String>,
        /// Namespace IRI the prefix expands to (needs --prefix)
        #[arg(long, requires = "prefix")]
        namespace: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Rebuild page metadata (annotations.ttl) and the semantic index of every vault, and report broken links
    Index {
        /// Specific page UUID to index (incremental)
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
        /// Output as JSON
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
    /// Rewrite stored metadata into the current format
    Migrate {
        #[command(subcommand)]
        action: MigrateCommands,
    },
    /// Build the human-readable symlink tree at <repo-root>/cerbo/
    Symlink {
        /// Explicit vault path (default: discover from CWD like Git)
        #[arg(long)]
        vault: Option<String>,
        /// Output as JSON
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
    /// List all vaults
    List {
        #[arg(long)]
        json: bool,
    },
    /// Add a new vault
    Add {
        name: String,
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Remove a vault
    Remove {
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Set the active vault
    Active {
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Promote an auto-registered vault to the manual registry
    Approve {
        id: String,
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
        #[arg(long)]
        dry_run: bool,
        /// Print only {"scanned", "rewritten", "errors"} as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum PageCommands {
    /// List all pages
    List {
        #[arg(long)]
        json: bool,
        /// Optional vault ID to list pages from. If omitted, uses the current active vault.
        #[arg(long)]
        vault: Option<String>,
    },
    /// Create a new page
    Create {
        title: String,
        #[arg(long)]
        json: bool,
    },
    /// Read page content
    Read {
        uuid: String,
        #[arg(long)]
        json: bool,
    },
    /// Write page content
    Write {
        uuid: String,
        content: String,
        #[arg(long)]
        json: bool,
    },
    /// Delete a page
    Delete {
        uuid: String,
        #[arg(long)]
        json: bool,
    },
    /// List the pages that link to an object
    Backlinks {
        uuid: String,
        /// Print a JSON array of {"uuid", "title"}
        #[arg(long)]
        json: bool,
        /// Vault ID to read from. If omitted, the vault is resolved like other page commands.
        #[arg(long)]
        vault: Option<String>,
    },
}

/// Ensure the vault's ignore rules cover the symlink tree and the trash.
///
/// Missing entries are appended; existing content keeps its order, and an entry
/// already covered by any equivalent spelling is left alone.
fn ensure_gitignore(vault_root: &std::path::Path) -> Result<(), String> {
    let gitignore_path = vault_root.join(".gitignore");
    let existing = std::fs::read_to_string(&gitignore_path).unwrap_or_default();

    let wanted: [(&str, &[&str]); 2] = [
        ("/cerbo/", &["/cerbo/", "cerbo/"]),
        (
            "/.cerbo/trash/",
            &["/.cerbo/trash/", ".cerbo/trash/", "/.cerbo/", ".cerbo/"],
        ),
    ];

    let mut content = existing.clone();
    for (entry, aliases) in wanted {
        if content.lines().any(|l| aliases.contains(&l.trim())) {
            continue;
        }
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(entry);
        content.push('\n');
    }

    if content == existing {
        return Ok(());
    }

    cerbo_core::fsio::write_atomic_str(&gitignore_path, &content)
        .map_err(|e| format!("Failed to write .gitignore: {}", e))
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
    if done == total || done % step == 0 {
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
    if let (Some(root), None) = (cwd_vault_root.as_ref(), cwd_vault_id.as_ref()) {
        let _ = cerbo_core::vault::auto_vault_register(&ctx, root);
        cwd_vault_id = cerbo_core::vault::vault_id_from_path(&ctx, root);
    }

    match cli.command {
        Commands::Init { json } => {
            let current_dir = std::env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?;
            let cerbo_dir = current_dir.join(".cerbo");

            if cerbo_dir.exists() {
                if json {
                    print_json_success("Vault already initialized");
                } else {
                    println!("Vault already initialized in {}", cerbo_dir.display());
                }
            } else {
                // Create .cerbo/objects/ directory
                let objects_dir = cerbo_dir.join("objects");
                std::fs::create_dir_all(&objects_dir)
                    .map_err(|e| format!("Failed to create objects dir: {}", e))?;

                // Create an empty v2 ontology-map.json
                cerbo_core::prefix_map::PrefixMap::default().save(&cerbo_dir)?;

                // Use a context rooted at the new local vault, not the global XDG ctx.
                // get_context() falls back to XDG when .cerbo/ didn't exist at startup.
                // The cache stays global: the semantic index never lives in a vault.
                let local_ctx = CerboContext {
                    config_dir: cerbo_dir.clone(),
                    cache_dir: ctx.cache_dir.clone(),
                };

                // Bundle Schema.org and FOAF. Both prefixes are built in, so the
                // map entry only records which object documents the namespace.
                let bundled = [
                    ("Schema.org", "schema", "https://schema.org/version/latest/schema.ttl"),
                    ("FOAF", "foaf", "http://xmlns.com/foaf/spec/index.rdf"),
                ];
                for (name, prefix, url) in bundled {
                    let namespace = cerbo_core::prefix_map::builtin_namespace(prefix)
                        .expect("bundled prefixes are built in");
                    match cerbo_core::object::object_import_ontology(&local_ctx, url, Some((prefix, namespace))) {
                        Ok(uuid) => if !json {
                            println!("Bundled {} ontology with UUID: {}", name, uuid);
                        },
                        Err(e) => if !json { println!("Warning: Failed to bundle {}: {}", name, e); },
                    }
                }

                if json {
                    print_json_success("Vault initialized with bundled ontologies");
                } else {
                    println!("Vault initialized in {}", cerbo_dir.display());
                }
            }

            // Idempotent, and runs on an already-initialised vault too so an
            // existing one picks up any newly required ignore rule.
            let _ = ensure_gitignore(&current_dir);
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
                let pages = cerbo_core::page::page_list(&eff_ctx)?;

                if json {
                    let pages_json: Vec<PageJson> = pages.into_iter().map(|p| PageJson {
                        uuid: p.uuid,
                        title: p.title,
                    }).collect();
                    print_json(&pages_json);
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
                cerbo_core::page::page_delete(&eff_ctx, uuid)?;
                if json {
                    print_json_success("Page deleted");
                } else {
                    println!("Deleted page");
                }
            }
            PageCommands::Backlinks { uuid, json, vault } => {
                let eff_ctx = resolve_vault_ctx(
                    &ctx,
                    vault.as_deref(),
                    cwd_vault_id.as_deref(),
                    cwd_vault_root.as_deref(),
                )?;
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
        Commands::Migrate { action } => match action {
            MigrateCommands::Vocabulary { vault, dry_run, json } => {
                use cerbo_core::VaultContext;
                use cerbo_core::vocabulary_migration::migrate_vocabulary;

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
