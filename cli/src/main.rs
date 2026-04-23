use cerbo_core::CerboContext;
use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use std::path::PathBuf;

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
    Watch,
}

#[derive(Subcommand)]
enum VaultCommands {
    /// List all vaults
    List,
    /// Add a new vault
    Add { name: String, path: String },
    /// Remove a vault
    Remove { id: String },
    /// Set the active vault
    Active { id: String },
}

#[derive(Subcommand)]
enum PageCommands {
    /// List all pages in a vault
    List { vault_id: String },
    /// Create a new page
    Create { vault_id: String, title: String },
    /// Read page content
    Read { vault_id: String, slug: String },
    /// Write page content
    Write { vault_id: String, slug: String, content: String },
    /// Delete a page
    Delete { vault_id: String, slug: String },
    /// Rename a page (triggers cascade)
    Rename { vault_id: String, slug: String, title: String },
    /// Attachment management
    Attachment {
        #[command(subcommand)]
        action: AttachmentCommands,
    },
}

#[derive(Subcommand)]
enum AttachmentCommands {
    /// List attachments for a page
    List { vault_id: String, slug: String },
    /// Add an attachment to a page
    Add { vault_id: String, slug: String, path: PathBuf },
    /// Delete an attachment from a page
    Delete { vault_id: String, slug: String, filename: String },
}

#[derive(Subcommand)]
enum IndexCommands {
    /// Rebuild the index for a vault
    Build { vault_id: String },
    /// Get backlinks for a page
    Backlinks { vault_id: String, slug: String },
}

fn get_context() -> Result<CerboContext, String> {
    let proj_dirs = ProjectDirs::from("io", "cerbo", "cerbo")
        .ok_or_else(|| "Could not determine project directories".to_string())?;
    
    Ok(CerboContext {
        config_dir: proj_dirs.config_dir().to_path_buf(),
        cache_dir: proj_dirs.cache_dir().to_path_buf(),
    })
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let ctx = get_context()?;

    match cli.command {
        Commands::Vault { action } => match action {
            VaultCommands::List => {
                let vaults = cerbo_core::vault::vault_list(&ctx)?;
                println!("{:#?}", vaults);
            }
            VaultCommands::Add { name, path } => {
                let vault = cerbo_core::vault::vault_add(&ctx, name, path)?;
                println!("Added vault: {:#?}", vault);
            }
            VaultCommands::Remove { id } => {
                cerbo_core::vault::vault_remove(&ctx, id)?;
                println!("Removed vault");
            }
            VaultCommands::Active { id } => {
                cerbo_core::vault::vault_set_active(&ctx, id)?;
                println!("Set active vault");
            }
        },
        Commands::Page { action } => match action {
            PageCommands::List { vault_id } => {
                let pages = cerbo_core::page::page_list(&ctx, vault_id)?;
                println!("{:#?}", pages);
            }
            PageCommands::Create { vault_id, title } => {
                let slug = cerbo_core::page::page_create(&ctx, vault_id, title)?;
                println!("Created page with slug: {}", slug);
            }
            PageCommands::Read { vault_id, slug } => {
                let content = cerbo_core::page::page_read(&ctx, vault_id, slug)?;
                println!("{}", content);
            }
            PageCommands::Write { vault_id, slug, content } => {
                let _ = cerbo_core::page::page_write(&ctx, vault_id, slug, content)?;
                println!("Updated page");
            }
            PageCommands::Delete { vault_id, slug } => {
                cerbo_core::page::page_delete(&ctx, vault_id, slug)?;
                println!("Deleted page");
            }
            PageCommands::Rename { vault_id, slug, title } => {
                let new_slug = cerbo_core::rename::page_rename(&ctx, vault_id, slug, title, None)?;
                println!("Renamed page to slug: {}", new_slug);
            }
            PageCommands::Attachment { action } => match action {
                AttachmentCommands::List { vault_id, slug } => {
                    let files = cerbo_core::page::attachment_list(&ctx, vault_id, slug)?;
                    println!("{:#?}", files);
                }
                AttachmentCommands::Add { vault_id, slug, path } => {
                    let filename = cerbo_core::page::attachment_add(&ctx, vault_id, slug, path)?;
                    println!("Added attachment: {}", filename);
                }
                AttachmentCommands::Delete { vault_id, slug, filename } => {
                    cerbo_core::page::attachment_delete(&ctx, vault_id, slug, filename)?;
                    println!("Deleted attachment");
                }
            },
        },
        Commands::Index { action } => match action {
            IndexCommands::Build { vault_id } => {
                let vault_path = cerbo_core::vault::get_vault_path(&ctx, &vault_id)
                    .ok_or_else(|| format!("Vault not found: {}", vault_id))?;
                let idx = cerbo_core::index::build_index(&vault_path)?;
                cerbo_core::index::save_index(&ctx, &vault_id, &idx)?;
                println!("Index rebuilt and saved");
            }
            IndexCommands::Backlinks { vault_id, slug } => {
                let index = cerbo_core::index::load_index(&ctx, &vault_id)
                    .ok_or_else(|| format!("No index for vault {}", vault_id))?;
                let backlinks = cerbo_core::index::compute_backlinks(&index, &slug);
                println!("{:#?}", backlinks);
            }
        },
        Commands::Watch => {
            println!("Watching vaults for changes...");
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
                                    println!("Index updated for vault: {}", vid);
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
