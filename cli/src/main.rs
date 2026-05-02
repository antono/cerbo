use cerbo_core::CerboContext;
use cerbo_core::context::CoreContext;
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json;
use std::path::PathBuf;

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
struct VaultJson {
    id: String,
    name: String,
    path: String,
}

#[derive(Serialize)]
struct PageJson {
    slug: String,
    title: String,
    path: String,
}

#[derive(Serialize)]
struct AttachmentJson {
    filename: String,
    path: String,
}

#[derive(Serialize)]
struct BacklinkJson {
    slug: String,
    title: String,
}

#[derive(Serialize)]
struct InfoJson {
    config_dir: String,
    cache_dir: String,
    vaults: Vec<VaultInfoJson>,
}

#[derive(Serialize)]
struct VaultInfoJson {
    id: String,
    name: String,
    path: String,
    page_count: usize,
}

#[derive(Serialize)]
struct WatchEventJson {
    event: String,
    vault_id: String,
    message: String,
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
    /// Index management
    Index {
        #[command(subcommand)]
        action: IndexCommands,
    },
    /// Watch vaults for changes
    Watch {
        #[arg(long)]
        json: bool,
    },
    /// Show configuration and vault info
    Info {
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
    /// List all pages in a vault
    List {
        vault_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Create a new page
    Create {
        vault_id: Option<String>,
        title: String,
        #[arg(long)]
        json: bool,
    },
    /// Read page content
    Read {
        vault_id: Option<String>,
        slug: String,
        #[arg(long)]
        json: bool,
    },
    /// Write page content
    Write {
        vault_id: Option<String>,
        slug: String,
        content: String,
        #[arg(long)]
        json: bool,
    },
    /// Delete a page
    Delete {
        vault_id: Option<String>,
        slug: String,
        #[arg(long)]
        json: bool,
    },
    /// Rename a page (triggers cascade)
    Rename {
        vault_id: Option<String>,
        slug: String,
        title: String,
        #[arg(long)]
        json: bool,
    },
    /// Attachment management
    Attachment {
        #[command(subcommand)]
        action: AttachmentCommands,
    },
}

#[derive(Subcommand)]
enum AttachmentCommands {
    /// List attachments for a page
    List {
        vault_id: String,
        slug: String,
        #[arg(long)]
        json: bool,
    },
    /// Add an attachment to a page
    Add {
        vault_id: String,
        slug: String,
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Delete an attachment from a page
    Delete {
        vault_id: String,
        slug: String,
        filename: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum IndexCommands {
    /// Rebuild the index for a vault
    Build {
        vault_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Get backlinks for a page
    Backlinks {
        vault_id: String,
        slug: String,
        #[arg(long)]
        json: bool,
    },
}

fn get_context() -> Result<CerboContext, String> {
    let core = CoreContext::new()?;
    let ctx = CerboContext {
        config_dir: core.config_dir,
        cache_dir: core.cache_dir,
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
            PageCommands::List { vault_id, json } => {
                let vid = match vault_id {
                    Some(id) => id,
                    None => cerbo_core::state::load_state(&ctx)
                        .ok()
                        .and_then(|s| s.active_vault_id)
                        .ok_or("No vault_id provided and no active vault set")?,
                };
                let pages = cerbo_core::page::page_list(&ctx, vid.clone())?;
                if json {
                    let pages_json: Vec<PageJson> = pages.into_iter().map(|p| {
                        let slug = p.slug.clone();
                        PageJson {
                            slug: p.slug,
                            title: p.title,
                            path: format!("{}/{}", vid, slug),
                        }
                    }).collect();
                    print_json(&pages_json);
                } else {
                    println!("{:#?}", pages);
                }
            }
            PageCommands::Create { vault_id, title, json } => {
                let vid = match vault_id {
                    Some(id) => id,
                    None => cerbo_core::state::load_state(&ctx)
                        .ok()
                        .and_then(|s| s.active_vault_id)
                        .ok_or("No vault_id provided and no active vault set")?,
                };
                let slug = cerbo_core::page::page_create(&ctx, vid, title)?;
                if json {
                    print_json(&serde_json::json!({"slug": slug}));
                } else {
                    println!("Created page with slug: {}", slug);
                }
            }
            PageCommands::Read { vault_id, slug, json } => {
                let vid = match vault_id {
                    Some(id) => id,
                    None => cerbo_core::state::load_state(&ctx)
                        .ok()
                        .and_then(|s| s.active_vault_id)
                        .ok_or("No vault_id provided and no active vault set")?,
                };
                let content = cerbo_core::page::page_read(&ctx, vid, slug)?;
                if json {
                    print_json(&serde_json::json!({"content": content}));
                } else {
                    println!("{}", content);
                }
            }
            PageCommands::Write { vault_id, slug, content, json } => {
                let vid = match vault_id {
                    Some(id) => id,
                    None => cerbo_core::state::load_state(&ctx)
                        .ok()
                        .and_then(|s| s.active_vault_id)
                        .ok_or("No vault_id provided and no active vault set")?,
                };
                let _ = cerbo_core::page::page_write(&ctx, vid, slug, content)?;
                if json {
                    print_json_success("Page updated");
                } else {
                    println!("Updated page");
                }
            }
            PageCommands::Delete { vault_id, slug, json } => {
                let vid = match vault_id {
                    Some(id) => id,
                    None => cerbo_core::state::load_state(&ctx)
                        .ok()
                        .and_then(|s| s.active_vault_id)
                        .ok_or("No vault_id provided and no active vault set")?,
                };
                cerbo_core::page::page_delete(&ctx, vid, slug)?;
                if json {
                    print_json_success("Page deleted");
                } else {
                    println!("Deleted page");
                }
            }
            PageCommands::Rename { vault_id, slug, title, json } => {
                let vid = match vault_id {
                    Some(id) => id,
                    None => cerbo_core::state::load_state(&ctx)
                        .ok()
                        .and_then(|s| s.active_vault_id)
                        .ok_or("No vault_id provided and no active vault set")?,
                };
                let new_slug = cerbo_core::rename::page_rename(&ctx, vid, slug, title, None)?;
                if json {
                    print_json(&serde_json::json!({"new_slug": new_slug}));
                } else {
                    println!("Renamed page to slug: {}", new_slug);
                }
            }
            PageCommands::Attachment { action } => match action {
                AttachmentCommands::List { vault_id, slug, json } => {
                    let files = cerbo_core::page::attachment_list(&ctx, vault_id, slug)?;
                    if json {
                        let files_json: Vec<AttachmentJson> = files.into_iter().map(|f| AttachmentJson {
                            filename: f.clone(),
                            path: f,
                        }).collect();
                        print_json(&files_json);
                    } else {
                        println!("{:#?}", files);
                    }
                }
                AttachmentCommands::Add { vault_id, slug, path, json } => {
                    let filename = cerbo_core::page::attachment_add(&ctx, vault_id, slug, path)?;
                    if json {
                        print_json(&serde_json::json!({"filename": filename}));
                    } else {
                        println!("Added attachment: {}", filename);
                    }
                }
                AttachmentCommands::Delete { vault_id, slug, filename, json } => {
                    cerbo_core::page::attachment_delete(&ctx, vault_id, slug, filename)?;
                    if json {
                        print_json_success("Attachment deleted");
                    } else {
                        println!("Deleted attachment");
                    }
                }
            },
        },
        Commands::Index { action } => match action {
            IndexCommands::Build { vault_id, json } => {
                let vault_path = cerbo_core::vault::get_vault_path(&ctx, &vault_id)
                    .ok_or_else(|| format!("Vault not found: {}", vault_id))?;
                let idx = cerbo_core::index::build_index(&vault_path)?;
                cerbo_core::index::save_index(&ctx, &vault_id, &idx)?;
                if json {
                    print_json_success("Index rebuilt and saved");
                } else {
                    println!("Index rebuilt and saved");
                }
            }
            IndexCommands::Backlinks { vault_id, slug, json } => {
                let index = cerbo_core::index::load_index(&ctx, &vault_id)
                    .ok_or_else(|| format!("No index for vault {}", vault_id))?;
                let backlinks = cerbo_core::index::compute_backlinks(&index, &slug);
                if json {
                    let backlinks_json: Vec<BacklinkJson> = backlinks.into_iter().map(|b| BacklinkJson {
                        slug: b.slug,
                        title: b.title,
                    }).collect();
                    print_json(&backlinks_json);
                } else {
                    println!("{:#?}", backlinks);
                }
            }
        },
        Commands::Watch { json } => {
            if json {
                println!("{}", serde_json::json!({"event": "start", "vault_id": "", "message": "Watching vaults for changes..."}));
            } else {
                println!("Watching vaults for changes...");
            }
            let reg = cerbo_core::vault::load_vaults(&ctx)?;
            let mut watchers = Vec::new();
            
            for vault in reg.vaults {
                let ctx_clone = ctx.clone();
                let vid = vault.id.clone();
                let handler = move |result: notify::Result<notify::Event>| {
                    if let Ok(event) = result {
                        let affects_page = event
                            .paths
                            .iter()
                            .any(|p| p.file_name().map(|n| n == "page.md").unwrap_or(false));
                        if affects_page {
                            if let Some(vpath) = cerbo_core::vault::get_vault_path(&ctx_clone, &vid) {
                                // ── Ensure all pages have H1 ──
                                let _ = cerbo_core::page::sync_markdown_titles(&vpath);
                                
                                if let Ok(idx) = cerbo_core::index::build_index(&vpath) {
                                    let _ = cerbo_core::index::save_index(&ctx_clone, &vid, &idx);
                                    if json {
                                        let event_json = WatchEventJson {
                                            event: "index_updated".to_string(),
                                            vault_id: vid.clone(),
                                            message: "Index updated".to_string(),
                                        };
                                        println!("{}", serde_json::to_string(&event_json).unwrap());
                                    } else {
                                        println!("Index updated for vault: {}", vid);
                                    }
                                }
                            }
                        }
                    }
                };
                
                use notify::Watcher;
                let mut watcher = cerbo_core::index::create_watcher(handler)?;
                watcher.watch(&vault.path, notify::RecursiveMode::Recursive).map_err(|e| e.to_string())?;
                watchers.push(watcher);
            }
            
            // Keep the process alive
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use cerbo_core::fixtures::create_fixture_vault;
    use cerbo_core::index;
    use std::fs;

    #[tokio::test]
    async fn test_cli_rename_cascade() {
        let fixture = create_fixture_vault().unwrap();
        
        // Simulating the action of PageCommands::Rename
        let new_slug = cerbo_core::rename::page_rename(
            &fixture.ctx,
            fixture.vault_id.clone(),
            "page-b".into(),
            "New Page B".into(),
            None
        ).unwrap();

        assert_eq!(new_slug, "new-page-b");

        // Verify Page A was updated
        let a_content = fs::read_to_string(fixture.vault_path.join("page-a").join("page.md")).unwrap();
        assert!(a_content.contains("[[New Page B]]"));

        // Verify index was updated and saved
        let idx = index::load_index(&fixture.ctx, &fixture.vault_id).unwrap();
        assert!(idx.pages.contains_key("new-page-b"));
    }
}
