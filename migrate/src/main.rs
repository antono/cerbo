use cerbo_core::{CerboContext, object::{object_create, object_write, ObjectType}, index::index_load};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Migration tool for Cerbo vaults (slug-based → UUID-based)
#[derive(Parser)]
#[command(name = "cerbo-migrate")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Migrate Cerbo vault from slug-based to UUID-based storage", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Migrate a vault to UUID-based storage
    Migrate {
        /// Path to the vault (default: current directory)
        path: Option<PathBuf>,
        /// Dry run - show what would be migrated without making changes
        #[arg(long)]
        dry_run: bool,
        /// Force migration even if .cerbo/ already exists
        #[arg(long)]
        force: bool,
    },
    /// Verify a migrated vault
    Verify {
        /// Path to the vault (default: current directory)
        path: Option<PathBuf>,
    },
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Migrate { path, dry_run, force } => {
            let vault_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
            migrate_vault(&vault_path, dry_run, force)?;
        }
        Commands::Verify { path } => {
            let vault_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
            verify_migration(&vault_path)?;
        }
    }

    Ok(())
}

/// Migrate a vault from slug-based to UUID-based storage
fn migrate_vault(vault_path: &Path, dry_run: bool, force: bool) -> Result<(), String> {
    println!("Migrating vault: {}", vault_path.display());

    // Check if .cerbo/ already exists
    let cerbo_dir = vault_path.join(".cerbo");
    if cerbo_dir.exists() && !force {
        return Err("Vault already has .cerbo/ directory. Use --force to migrate anyway.".to_string());
    }

    if dry_run {
        println!("[DRY RUN] No changes will be made.");
    }

    // Create CerboContext
    let ctx = CerboContext {
        config_dir: cerbo_dir.clone(),
        cache_dir: vault_path.join(".cerbo").join("cache"),
    };

    if !dry_run {
        // Create .cerbo/objects/ directory
        let objects_dir = cerbo_dir.join("objects");
        fs::create_dir_all(&objects_dir)
            .map_err(|e| format!("Failed to create objects dir: {}", e))?;

        // Create index.json and ontology-map.json
        let index = cerbo_core::index::IndexJson::default();
        let index_path = cerbo_dir.join("index.json");
        let index_content = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("Failed to serialize index: {}", e))?;
        fs::write(&index_path, index_content)
            .map_err(|e| format!("Failed to write index.json: {}", e))?;

        let ontology_map = serde_json::json!({"prefixes": {}});
        let map_path = cerbo_dir.join("ontology-map.json");
        fs::write(&map_path, serde_json::to_string_pretty(&ontology_map).unwrap())
            .map_err(|e| format!("Failed to write ontology-map.json: {}", e))?;
    }

    // Find all slug-based pages (<slug>/page.md)
    let slug_dirs = find_slug_directories(vault_path)?;

    println!("Found {} slug-based pages to migrate.", slug_dirs.len());

    let mut migrated = 0;
    let mut failed = 0;

    for slug_dir in slug_dirs {
        match migrate_page(&ctx, vault_path, &slug_dir, dry_run) {
            Ok(uuid) => {
                println!("✓ Migrated: {} → {}", slug_dir.display(), uuid);
                migrated += 1;
            }
            Err(e) => {
                println!("✗ Failed to migrate {}: {}", slug_dir.display(), e);
                failed += 1;
            }
        }
    }

    println!("\nMigration complete: {} migrated, {} failed.", migrated, failed);

    if !dry_run && migrated >0 {
        println!("\nNext steps:");
        println!("1. Run 'cerbo-migrate verify' to check the migration");
        println!("2. Run 'cerbo backlinks <uuid>' to regenerate backlinks");
        println!("3. Review the migrated content in .cerbo/objects/");
        if failed == 0 {
            println!("\n✓ All pages migrated and old directories removed.");
        }
    }

    Ok(())
}

/// Find all slug-based directories with page.md
fn find_slug_directories(vault_path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut slug_dirs = Vec::new();

    for entry in WalkDir::new(vault_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_dir() && path.file_name().is_some() {
            let page_md = path.join("page.md");
            if page_md.exists() {
                // Check it's not .cerbo/ or other hidden directory
                if let Some(name) = path.file_name() {
                    if !name.to_string_lossy().starts_with('.') {
                        slug_dirs.push(path.to_path_buf());
                    }
                }
            }
        }
    }

    Ok(slug_dirs)
}

/// Migrate a single page from slug-based to UUID-based storage
fn migrate_page(ctx: &CerboContext, _vault_path: &Path, slug_dir: &Path, dry_run: bool) -> Result<String, String> {
    // Read page.md content
    let page_md_path = slug_dir.join("page.md");
    let content = fs::read_to_string(&page_md_path)
        .map_err(|e| format!("Failed to read page.md: {}", e))?;

    // Extract title from first heading or directory name
    let title = extract_title(&content)
        .unwrap_or_else(|| slug_dir.file_name().unwrap().to_string_lossy().to_string());

    if dry_run {
        println!("[DRY RUN] Would migrate: {} (title: {})", slug_dir.display(), title);
        return Ok("dry-run-uuid".to_string());
    }

    // Create UUID object
    let uuid = object_create(ctx, None, ObjectType::Product, title.clone())?;

    // Write content to new location
    object_write(ctx, &uuid, &content)?;

    // Copy assets/ directory if it exists
    let assets_dir = slug_dir.join("assets");
    if assets_dir.exists() {
        let obj_dir = cerbo_core::object::object_path(ctx, &uuid);
        let new_assets = obj_dir.join("assets");
        copy_dir_recursive(&assets_dir, &new_assets)?;
        println!("  → Copied assets/ directory");
    }

    // Copy any other files (except page.md and assets/)
    for entry in fs::read_dir(slug_dir)
        .map_err(|e| format!("Failed to read directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        if file_name != "page.md" && file_name != "assets" && !file_name.starts_with('.') {
            let obj_dir = cerbo_core::object::object_path(ctx, &uuid);
            let dest = obj_dir.join(&file_name);
            if path.is_file() {
                fs::copy(&path, &dest)
                    .map_err(|e| format!("Failed to copy {}: {}", file_name, e))?;
            } else if path.is_dir() {
                copy_dir_recursive(&path, &dest)?;
            }
        }
    }

    // Remove old slug directory after successful migration
    if !dry_run {
        fs::remove_dir_all(slug_dir)
            .map_err(|e| format!("Failed to remove old directory: {}", e))?;
        println!("  → Removed old directory");
    }

    Ok(uuid)
}

/// Extract title from markdown content (first # heading)
fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return Some(trimmed.trim_start_matches("# ").to_string());
        }
    }
    None
}

/// Copy directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    for entry in fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        let dest_path = dst.join(path.file_name().unwrap());

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)
                .map_err(|e| format!("Failed to copy file: {}", e))?;
        }
    }

    Ok(())
}

/// Verify a migrated vault
fn verify_migration(vault_path: &Path) -> Result<(), String> {
    println!("Verifying migration: {}", vault_path.display());

    let cerbo_dir = vault_path.join(".cerbo");
    if !cerbo_dir.exists() {
        return Err("No .cerbo/ directory found. Is this a migrated vault?".to_string());
    }

    let objects_dir = cerbo_dir.join("objects");
    if !objects_dir.exists() {
        return Err("No .cerbo/objects/ directory found.".to_string());
    }

    // Load index
    let index = index_load(&CerboContext {
        config_dir: cerbo_dir.clone(),
        cache_dir: cerbo_dir.join("cache"),
    })?;

    println!("Found {} objects in index.", index.title_to_uuid.len());

    // Check each object
    let mut valid = 0;
    let mut invalid = 0;

    for (title, uuid) in &index.title_to_uuid {
        let obj_dir = objects_dir.join(uuid);
        if !obj_dir.exists() {
            println!("✗ Object directory missing: {}", uuid);
            invalid += 1;
            continue;
        }

        let page_md = obj_dir.join("page.md");
        if !page_md.exists() {
            println!("✗ page.md missing for: {} ({})", title, uuid);
            invalid += 1;
            continue;
        }

        valid += 1;
    }

    println!("\nVerification complete: {} valid, {} invalid.", valid, invalid);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_find_slug_directories() {
        let dir = tempdir().unwrap();
        let vault = dir.path();

        // Create slug-based pages
        let page1 = vault.join("first-page");
        fs::create_dir_all(&page1).unwrap();
        fs::write(page1.join("page.md"), "# First Page\n\nContent.").unwrap();

        let page2 = vault.join("second-page");
        fs::create_dir_all(&page2).unwrap();
        fs::write(page2.join("page.md"), "# Second Page\n\nMore content.").unwrap();

        let slug_dirs = find_slug_directories(vault).unwrap();
        assert_eq!(slug_dirs.len(), 2);
    }

    #[test]
    fn test_extract_title() {
        let content = "# My Title\n\nSome content.";
        let title = extract_title(content);
        assert_eq!(title, Some("My Title".to_string()));

        let content_no_heading = "Just some text.";
        let title = extract_title(content_no_heading);
        assert_eq!(title, None);
    }
}
