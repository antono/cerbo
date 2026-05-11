use std::process::Command;
use std::path::Path;
use tempfile::TempDir;

struct TestContext {
    #[allow(dead_code)]
    tmp_dir: TempDir,
    config_dir: std::path::PathBuf,
    bin_path: std::path::PathBuf,
}

fn setup() -> TestContext {
    let tmp_dir = TempDir::new().unwrap();
    let config_dir = tmp_dir.path().join(".cerbo");

    // The binary is in the workspace target/debug directory
    let bin_path = std::path::PathBuf::from("/home/antono/Code/cerbo/target/debug/cerbo");

    if !bin_path.exists() {
        panic!("cerbo binary not found at {:?}. Run 'cargo build --package cerbo' first.", bin_path);
    }

    TestContext {
        tmp_dir,
        config_dir,
        bin_path,
    }
}

#[test]
fn test_init_creates_cerbo_directory() {
    let ctx = setup();

    let output = Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(ctx.config_dir.exists());
    assert!(ctx.config_dir.join("objects").exists());
    assert!(ctx.config_dir.join("index.json").exists());
    assert!(ctx.config_dir.join("ontology-map.json").exists());
}

#[test]
fn test_init_idempotent() {
    let ctx = setup();

    // First init
    let output1 = Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output1.status.success());

    // Second init should succeed (idempotent)
    let output2 = Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output2.status.success());
}

#[test]
fn test_page_create_and_read() {
    let ctx = setup();

    // Init vault
    let _ = Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    // Create page
    let output = Command::new(&ctx.bin_path)
        .args(&["page", "create", "Test Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let output_str = String::from_utf8(output.stdout).unwrap();
    let uuid = output_str
        .trim()
        .split(": ")
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();
    assert!(!uuid.is_empty());
    assert!(ctx.config_dir.join("objects").join(&uuid).exists());

    // Read page
    let output = Command::new(&ctx.bin_path)
        .args(&["page", "read", &uuid])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let content = String::from_utf8(output.stdout).unwrap();
    assert!(content.contains("# Test Page"));
}

#[test]
fn test_page_write_and_read() {
    let ctx = setup();

    // Init and create
    let _ = Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    let output = Command::new(&ctx.bin_path)
        .args(&["page", "create", "Editable Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let output_str = String::from_utf8(output.stdout).unwrap();
    let uuid = output_str
        .trim()
        .split(": ")
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();

    // Write
    let new_content = "# Editable Page\n\nUpdated content here.";
    let output = Command::new(&ctx.bin_path)
        .args(&["page", "write", &uuid, new_content])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    // Read back
    let output = Command::new(&ctx.bin_path)
        .args(&["page", "read", &uuid])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    let content = String::from_utf8(output.stdout).unwrap();
    assert!(content.contains("Updated content here."));
}

#[test]
fn test_page_delete() {
    let ctx = setup();

    // Init and create
    let _ = Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    let output = Command::new(&ctx.bin_path)
        .args(&["page", "create", "Page to Delete"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let output_str = String::from_utf8(output.stdout).unwrap();
    let uuid = output_str
        .trim()
        .split(": ")
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();

    // Delete
    let output = Command::new(&ctx.bin_path)
        .args(&["page", "delete", &uuid])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!ctx.config_dir.join("objects").join(&uuid).exists());
}

#[test]
fn test_page_list() {
    let ctx = setup();

    // Init and create several pages
    let _ = Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    for i in 1..=3 {
        let _ = Command::new(&ctx.bin_path)
            .args(&["page", "create", &format!("Page {}", i)])
            .current_dir(ctx.tmp_dir.path())
            .output();
    }

    // List pages
    let output = Command::new(&ctx.bin_path)
        .args(&["page", "list"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("Page 1"));
    assert!(output_str.contains("Page 2"));
    assert!(output_str.contains("Page 3"));
}

#[test]
fn test_resolve_command() {
    let ctx = setup();

    // Init and create
    let _ = Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    let output = Command::new(&ctx.bin_path)
        .args(&["page", "create", "Resolvable Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let output_str = String::from_utf8(output.stdout).unwrap();
    let uuid = output_str
        .trim()
        .split(": ")
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();

    // Resolve
    let output = Command::new(&ctx.bin_path)
        .args(&["resolve", &uuid])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let path = String::from_utf8(output.stdout).unwrap();
    assert!(path.contains(&uuid));
    assert!(Path::new(path.trim()).exists());
}

#[test]
fn test_import_url_creates_source() {
    let ctx = setup();

    // Init vault
    let _ = Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    // Import URL as Source
    let output = Command::new(&ctx.bin_path)
        .args(&["import", "https://example.com"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("UUID:"));

    // Extract UUID
    let uuid = output_str
        .trim()
        .split("UUID: ")
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();

    // Verify it's type: Source (read-only)
    let obj_dir = ctx.config_dir.join("objects").join(&uuid);
    assert!(obj_dir.exists());
    let meta_path = obj_dir.join("meta.ttl");
    let meta_content = std::fs::read_to_string(&meta_path).unwrap();
    assert!(meta_content.contains(":type :Source"));

    // Try to write - should fail
    let write_output = Command::new(&ctx.bin_path)
        .args(&["page", "write", &uuid, "Should fail"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(!write_output.status.success());
}

#[test]
fn test_import_ontology() {
    let ctx = setup();

    // Init vault (this bundles Schema.org and FOAF)
    let _ = Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    // Verify ontologies were bundled
    let map_path = ctx.config_dir.join("ontology-map.json");
    assert!(map_path.exists());
    let map_content = std::fs::read_to_string(&map_path).unwrap();
    assert!(map_content.contains("schema"));
    assert!(map_content.contains("foaf"));

    // Import a new ontology
    let output = Command::new(&ctx.bin_path)
        .args(&["import-ontology", "https://schema.org/version/latest/schema.ttl"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("UUID:"));
}

#[test]
fn test_old_vault_not_compatible() {
    let ctx = setup();

    // Create a fake old-style vault (with slug-based storage)
    let old_slug_dir = ctx.tmp_dir.path().join("old-style-page");
    std::fs::create_dir_all(&old_slug_dir).unwrap();
    std::fs::write(old_slug_dir.join("page.md"), "# Old Style Page\n\nThis is old.\n").unwrap();

    // Try to init in the same directory - should create .cerbo/ but NOT recognize old pages
    let _ = Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    // List pages - should NOT find "old-style-page"
    let output = Command::new(&ctx.bin_path)
        .args(&["page", "list"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(!output_str.contains("old-style-page"));

    // The old page.md is NOT in .cerbo/objects/
    // This demonstrates the breaking change
}

// ── Integration Tests for `cerbo index` Command ────────────────────────

#[test]
fn test_index_full_vault_rebuild() {
    let ctx = setup();

    // Init vault
    Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Create two pages
    let output1 = Command::new(&ctx.bin_path)
        .args(&["page", "create", "Page 1"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let uuid1 = extract_uuid(&output1.stdout);

    let output2 = Command::new(&ctx.bin_path)
        .args(&["page", "create", "Page 2"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let uuid2 = extract_uuid(&output2.stdout);

    // Write content with link from page1 to page2
    let content = format!("Link to [page2](cerbo://{})", uuid2);
    let page1_path = ctx.config_dir.join("objects").join(&uuid1).join("page.md");
    std::fs::write(&page1_path, content).unwrap();

    // Run index
    let output = Command::new(&ctx.bin_path)
        .arg("index")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("Indexed 2 pages"));
    assert!(output_str.contains("Found 1 links"));

    // Verify backrefs.ttl was created for page2
    let backrefs_path = ctx.config_dir.join("objects").join(&uuid2).join("backrefs.ttl");
    assert!(backrefs_path.exists());
    let backrefs = std::fs::read_to_string(&backrefs_path).unwrap();
    assert!(backrefs.contains(&uuid1));
}

#[test]
fn test_index_single_page_incremental() {
    let ctx = setup();

    // Init vault
    Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Create page with annotation
    let output = Command::new(&ctx.bin_path)
        .args(&["page", "create", "Test Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let uuid = extract_uuid(&output.stdout);

    // Write content with annotation
    let page_path = ctx.config_dir.join("objects").join(&uuid).join("page.md");
    std::fs::write(&page_path, "Some [knowledge]{schema:Thing} here").unwrap();

    // Run incremental index on single page
    let output = Command::new(&ctx.bin_path)
        .args(&["index", "--page", &uuid])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("Indexed 1 pages"));
    assert!(output_str.contains("1 annotations"));

    // Verify annotations.ttl was created
    let annotations_path = ctx.config_dir.join("objects").join(&uuid).join("annotations.ttl");
    assert!(annotations_path.exists());
    let annotations = std::fs::read_to_string(&annotations_path).unwrap();
    assert!(annotations.contains("knowledge"));
}

#[test]
fn test_index_with_explicit_vault_path() {
    let ctx = setup();

    // Init vault
    Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Create page
    let output = Command::new(&ctx.bin_path)
        .args(&["page", "create", "Test Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Run index from different directory with explicit --vault path
    let output = Command::new(&ctx.bin_path)
        .args(&["index", "--vault", ctx.tmp_dir.path().to_str().unwrap()])
        .current_dir("/tmp")
        .output()
        .unwrap();

    assert!(output.status.success());
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("Indexed 1 pages"));
}

#[test]
fn test_index_git_style_discovery_from_subdirectory() {
    let ctx = setup();

    // Init vault
    Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Create page
    let output = Command::new(&ctx.bin_path)
        .args(&["page", "create", "Test Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Create subdirectory
    let subdir = ctx.tmp_dir.path().join("subdir");
    std::fs::create_dir_all(&subdir).unwrap();

    // Run index from subdirectory (should discover vault like Git)
    let output = Command::new(&ctx.bin_path)
        .arg("index")
        .current_dir(&subdir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("Indexed 1 pages"));
}

#[test]
fn test_index_json_output() {
    let ctx = setup();

    // Init vault
    Command::new(&ctx.bin_path)
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Create page
    Command::new(&ctx.bin_path)
        .args(&["page", "create", "Test Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Run index with --json
    let output = Command::new(&ctx.bin_path)
        .args(&["index", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let output_str = String::from_utf8(output.stdout).unwrap();
    
    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert_eq!(json["pages_processed"], 1);
    assert!(json.get("links_found").is_some());
    assert!(json.get("annotations_found").is_some());
    assert_eq!(json["errors"], 0);
}

// Helper to extract UUID from command output
fn extract_uuid(stdout: &[u8]) -> String {
    String::from_utf8(stdout.to_vec())
        .unwrap()
        .trim()
        .split(": ")
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string()
}
