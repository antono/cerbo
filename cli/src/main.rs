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

fn get_context() -> Result<CerboContext, String> {
    // Check for local .cerbo/ directory first (for init'd vaults)
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    let local_cerbo = current_dir.join(".cerbo");

    let (config_dir, cache_dir) = if local_cerbo.exists() {
        (local_cerbo.clone(), current_dir.join(".cache"))
    } else {
        // Fall back to XDG directories
        let core = CoreContext::new()?;
        (core.config_dir.clone(), core.cache_dir.clone())
    };

    let ctx = CerboContext {
        config_dir: config_dir.clone(),
        cache_dir: cache_dir.clone(),
    };

    if local_cerbo.exists() {
        // For local vaults, skip migration and config creation
        return Ok(ctx);
    }

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
    Ok(CerboContext {
        config_dir: ctx.config_dir,
        cache_dir: ctx.cache_dir,
    })
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let ctx = get_context()?;

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

                // Bundle Schema.org ontology
                let schema_url = "https://schema.org/version/latest/schema.ttl";
                match cerbo_core::object::object_import_ontology(&ctx, schema_url) {
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
                match cerbo_core::object::object_import_ontology(&ctx, foaf_url) {
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
                    println!("{:#?}", vaults_file);
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
                    println!("Added vault: {:#?}", vault);
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
            PageCommands::List { json } => {
                let pages = cerbo_core::page::page_list(&ctx)?;
                if json {
                    let pages_json: Vec<PageJson> = pages.into_iter().map(|p| PageJson {
                        uuid: p.uuid,
                        title: p.title,
                    }).collect();
                    print_json(&pages_json);
                } else {
                    println!("{:#?}", pages);
                }
            }
            PageCommands::Create { title, json } => {
                let uuid = cerbo_core::object::object_create(&ctx, None, cerbo_core::object::ObjectType::Product, title)?;
                if json {
                    print_json(&serde_json::json!({"uuid": uuid}));
                } else {
                    println!("Created page with UUID: {}", uuid);
                }
            }
            PageCommands::Read { uuid, json } => {
                let content = cerbo_core::page::page_read(&ctx, uuid)?;
                if json {
                    print_json(&serde_json::json!({"content": content}));
                } else {
                    println!("{}", content);
                }
            }
            PageCommands::Write { uuid, content, json } => {
                let _ = cerbo_core::page::page_write(&ctx, uuid, content)?;
                if json {
                    print_json_success("Page updated");
                } else {
                    println!("Updated page");
                }
            }
            PageCommands::Delete { uuid, json } => {
                cerbo_core::page::page_delete(&ctx, uuid)?;
                if json {
                    print_json_success("Page deleted");
                } else {
                    println!("Deleted page");
                }
            }
        },
        Commands::Resolve { uuid, json } => {
            let obj_path = cerbo_core::object::object_path(&ctx, &uuid);
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
            let uuid = cerbo_core::object::object_import(&ctx, &url)?;
            if json {
                print_json(&serde_json::json!({"uuid": uuid, "url": url}));
            } else {
                println!("Imported URL as Source object with UUID: {}", uuid);
            }
        },
        Commands::ImportOntology { url, json } => {
            let uuid = cerbo_core::object::object_import_ontology(&ctx, &url)?;
            if json {
                print_json(&serde_json::json!({"uuid": uuid, "url": url}));
            } else {
                println!("Imported ontology from {} with UUID: {}", url, uuid);
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
                        println!("├── {} ({}) - {} pages", v.name, display_path(&v.path), count);
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
