use cerbo_core::CerboContext;
use cerbo_core::context::CoreContext;
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json;

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
// vault name when it is the active vault.
fn print_vaults_template(ctx: &CerboContext, vaults_file: &cerbo_core::vault::VaultsFile) -> Result<(), String> {
    use cerbo_core::state;

    let st = state::load_state(ctx)?;
    let active = st.active_vault_id;

    for v in &vaults_file.vaults {
        let current_marker = if Some(v.id.clone()) == active { " (current)" } else { "" };
        println!("{}: {}{}", v.id, v.name, current_marker);
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
    /// Import ontology (creates type: Ontology object)
    ImportOntology {
        url: String,
        #[arg(long)]
        json: bool,
    },
    /// Rebuild page metadata (backrefs.ttl and annotations.ttl)
    Index {
        /// Specific page UUID to index (incremental)
        #[arg(long)]
        page: Option<String>,
        /// Explicit vault path (default: discover from CWD like Git)
        #[arg(long)]
        vault: Option<String>,
        /// Skip backfilling missing cerbo:slug values
        #[arg(long)]
        no_backfill_slug: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
}

/// Ensure `/cerbo/` is present in `.gitignore` at the given repo root.
fn ensure_gitignore(vault_root: &std::path::Path) -> Result<(), String> {
    let gitignore_path = vault_root.join(".gitignore");
    let entry = "/cerbo/\n";

    let existing = std::fs::read_to_string(&gitignore_path).unwrap_or_default();
    if existing.lines().any(|l| l == "/cerbo/" || l == "cerbo/") {
        return Ok(());
    }

    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(entry);

    std::fs::write(&gitignore_path, content)
        .map_err(|e| format!("Failed to write .gitignore: {}", e))
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
    let cwd_vault_id = cwd_vault_root
        .as_deref()
        .and_then(|root| cerbo_core::vault::vault_id_from_path(&ctx, root));
    if let (Some(ref root), None) = (cwd_vault_root.as_ref(), cwd_vault_id.as_ref()) {
        if root.join(".cerbo").is_dir() {
            eprintln!(
                "warning: vault at {} is not registered; run 'cerbo vault add'",
                root.display()
            );
        }
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

                // Create empty index.json
                let index = cerbo_core::index::IndexJson::default();
                let index_path = cerbo_dir.join("index.json");
                let index_content = serde_json::to_string_pretty(&index)
                    .map_err(|e| format!("Failed to serialize index: {}", e))?;
                std::fs::write(&index_path, index_content)
                    .map_err(|e| format!("Failed to write index.json: {}", e))?;

                // Create ontology-map.json with empty prefixes
                let ontology_map = serde_json::json!({"prefixes": {}});
                let map_path = cerbo_dir.join("ontology-map.json");
                std::fs::write(&map_path, serde_json::to_string_pretty(&ontology_map).unwrap())
                    .map_err(|e| format!("Failed to write ontology-map.json: {}", e))?;

                // Use a context rooted at the new local vault, not the global XDG ctx.
                // get_context() falls back to XDG when .cerbo/ didn't exist at startup.
                let local_ctx = CerboContext {
                    config_dir: cerbo_dir.clone(),
                    cache_dir: current_dir.join(".cache"),
                };

                // Bundle Schema.org ontology
                let schema_url = "https://schema.org/version/latest/schema.ttl";
                match cerbo_core::object::object_import_ontology(&local_ctx, schema_url) {
                    Ok(uuid) => {
                        // Update ontology-map.json with "schema" prefix
                        let mut map: serde_json::Value = serde_json::from_str(
                            &std::fs::read_to_string(&map_path).unwrap_or_default()
                        ).unwrap_or(serde_json::json!({"prefixes": {}}));
                        if let Some(prefixes) = map.get_mut("prefixes") {
                            prefixes["schema"] = serde_json::json!(uuid);
                            std::fs::write(&map_path, serde_json::to_string_pretty(&map).unwrap()).ok();
                        }
                        if !json {
                            println!("Bundled Schema.org ontology with UUID: {}", uuid);
                        }
                    },
                    Err(e) => if !json { println!("Warning: Failed to bundle Schema.org: {}", e); },
                }

                // Bundle FOAF ontology
                let foaf_url = "http://xmlns.com/foaf/spec/index.rdf";
                match cerbo_core::object::object_import_ontology(&local_ctx, foaf_url) {
                    Ok(uuid) => {
                        // Update ontology-map.json with "foaf" prefix
                        let mut map: serde_json::Value = serde_json::from_str(
                            &std::fs::read_to_string(&map_path).unwrap_or_default()
                        ).unwrap_or(serde_json::json!({"prefixes": {}}));
                        if let Some(prefixes) = map.get_mut("prefixes") {
                            prefixes["foaf"] = serde_json::json!(uuid);
                            std::fs::write(&map_path, serde_json::to_string_pretty(&map).unwrap()).ok();
                        }
                        if !json {
                            println!("Bundled FOAF ontology with UUID: {}", uuid);
                        }
                    },
                    Err(e) => if !json { println!("Warning: Failed to bundle FOAF: {}", e); },
                }

                let _ = ensure_gitignore(&current_dir);

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
        Commands::ImportOntology { url, json } => {
            let eff_ctx = resolve_vault_ctx(
                &ctx,
                None,
                cwd_vault_id.as_deref(),
                cwd_vault_root.as_deref(),
            )?;
            let uuid = cerbo_core::object::object_import_ontology(&eff_ctx, &url)?;
            if json {
                print_json(&serde_json::json!({"uuid": uuid, "url": url}));
            } else {
                println!("Imported ontology from {} with UUID: {}", url, uuid);
            }
        },
        Commands::Index { page, vault, no_backfill_slug, json } => {
            use cerbo_core::VaultContext;
            use cerbo_core::metadata_index;

            // Explicit --vault path takes priority; otherwise use CWD-discovered root.
            let vault_ctx = if let Some(vault_path) = vault {
                VaultContext::from_path(std::path::PathBuf::from(&vault_path))?
            } else if let Some(ref root) = cwd_vault_root {
                VaultContext::from_path(root.clone())?
            } else {
                return Err("not inside a cerbo vault (or any parent up to mount point); use --vault <path>".to_string());
            };

            let stats = if let Some(page_uuid) = page {
                metadata_index::index_page(&vault_ctx, &page_uuid)?
            } else {
                metadata_index::index_vault(&vault_ctx)?
            };

            // Backfill missing slugs (full index only, unless --no-backfill-slug)
            let slugs_backfilled = if !no_backfill_slug {
                metadata_index::backfill_slugs(&vault_ctx).unwrap_or(0)
            } else {
                0
            };

            // Validate virtual paths and report issues
            let path_errors = metadata_index::validate_virtual_paths(&vault_ctx);
            for (uuid, msg) in &path_errors {
                eprintln!("Warning: page {}: {}", uuid, msg);
            }

            // Detect and report collisions
            let collisions = metadata_index::detect_path_collisions(&vault_ctx);
            for (path, uuids) in &collisions {
                eprintln!("Warning: symlink collision at {:?}: {:?}", path, uuids);
            }

            if json {
                print_json(&serde_json::json!({
                    "pages_processed": stats.pages_processed,
                    "links_found": stats.links_found,
                    "annotations_found": stats.annotations_found,
                    "slugs_backfilled": slugs_backfilled,
                    "path_errors": path_errors.len(),
                    "collisions": collisions.len(),
                    "errors": stats.errors.len()
                }));
            } else {
                println!("Indexed {} pages", stats.pages_processed);
                println!("Found {} links, {} annotations", stats.links_found, stats.annotations_found);
                if slugs_backfilled > 0 {
                    println!("Backfilled {} missing slugs", slugs_backfilled);
                }
                if !path_errors.is_empty() {
                    eprintln!("Virtual path errors: {}", path_errors.len());
                }
                if !collisions.is_empty() {
                    eprintln!("Symlink collisions detected: {}", collisions.len());
                }
                if !stats.errors.is_empty() {
                    eprintln!("Errors: {}", stats.errors.len());
                    for err in &stats.errors {
                        eprintln!("  {}: {}", err.page_uuid, err.error_message);
                    }
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
                            "dirs_created": report.dirs_created
                        }));
                    } else {
                        println!(
                            "Symlink tree built: {} objects, {} symlinks, {} dirs",
                            report.objects_scanned, report.leaves_created, report.dirs_created
                        );
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
            use std::path::PathBuf;

            fn display_path(p: &PathBuf) -> String {
                if let Ok(home) = std::env::var("HOME") {
                    if let Some(rest) = p.to_str().and_then(|s| s.strip_prefix(&home)) {
                        return format!("~{}", rest);
                    }
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
}

#[derive(Serialize)]
struct VaultInfoJson {
    id: String,
    name: String,
    path: String,
    page_count: usize,
}

#[derive(Serialize)]
struct InfoJson {
    config_dir: String,
    cache_dir: String,
    vaults: Vec<VaultInfoJson>,
}
