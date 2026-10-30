use std::process::Command;
use std::path::Path;
use tempfile::TempDir;

struct TestContext {
    #[allow(dead_code)]
    tmp_dir: TempDir,
    #[allow(dead_code)]
    xdg_dir: TempDir,
    config_dir: std::path::PathBuf,
    bin_path: std::path::PathBuf,
}

impl TestContext {
    fn cmd(&self) -> Command {
        cerbo_cmd(&self.bin_path, self.xdg_dir.path())
    }
}

fn setup() -> TestContext {
    let tmp_dir = TempDir::new().unwrap();
    let xdg_dir = TempDir::new().unwrap();
    let config_dir = tmp_dir.path().join(".cerbo");

    // Cargo builds the binary for us and hands us its path, so this works in any
    // checkout and inside the Nix sandbox.
    let bin_path = std::path::PathBuf::from(env!("CARGO_BIN_EXE_cerbo"));

    TestContext {
        tmp_dir,
        xdg_dir,
        config_dir,
        bin_path,
    }
}

#[test]
fn test_init_creates_cerbo_directory() {
    let ctx = setup();

    let output = ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(ctx.config_dir.exists());
    assert!(ctx.config_dir.join("objects").exists());
    assert!(!ctx.config_dir.join("index.json").exists(), "index.json is no longer part of a vault");
    let map: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(ctx.config_dir.join("ontology-map.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(map["version"], 2, "{map}");
    assert!(map["prefixes"].is_object(), "{map}");
}

#[test]
fn test_init_idempotent() {
    let ctx = setup();

    // First init
    let output1 = ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output1.status.success());

    // Second init should succeed (idempotent)
    let output2 = ctx.cmd()
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
    let _ = ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    // Create page
    let output = ctx.cmd()
        .args(["page", "create", "Test Page"])
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
    let output = ctx.cmd()
        .args(["page", "read", &uuid])
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
    let _ = ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    let output = ctx.cmd()
        .args(["page", "create", "Editable Page"])
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
    let output = ctx.cmd()
        .args(["page", "write", &uuid, new_content])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    // Read back
    let output = ctx.cmd()
        .args(["page", "read", &uuid])
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
    let _ = ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    let output = ctx.cmd()
        .args(["page", "create", "Page to Delete"])
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
    let output = ctx.cmd()
        .args(["page", "delete", &uuid])
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
    let _ = ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    for i in 1..=3 {
        let _ = ctx.cmd()
            .args(["page", "create", &format!("Page {}", i)])
            .current_dir(ctx.tmp_dir.path())
            .output();
    }

    // List pages
    let output = ctx.cmd()
        .args(["page", "list"])
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
    let _ = ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    let output = ctx.cmd()
        .args(["page", "create", "Resolvable Page"])
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
    let output = ctx.cmd()
        .args(["resolve", &uuid])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let path = String::from_utf8(output.stdout).unwrap();
    assert!(path.contains(&uuid));
    assert!(Path::new(path.trim()).exists());
}

#[test]
#[ignore = "requires network access; run with `cargo test -- --ignored`"]
fn test_import_url_creates_source() {
    let ctx = setup();

    // Init vault
    let _ = ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    // Import URL as Source
    let output = ctx.cmd()
        .args(["import", "https://example.com"])
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
    assert!(meta_content.contains("schema:WebPage"), "{meta_content}");

    // Try to write - should fail
    let write_output = ctx.cmd()
        .args(["page", "write", &uuid, "Should fail"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(!write_output.status.success());
}

#[test]
#[ignore = "requires network access; run with `cargo test -- --ignored`"]
fn test_import_ontology() {
    let ctx = setup();

    // Init vault (this bundles Schema.org and FOAF)
    let _ = ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    // Verify ontologies were bundled and registered with their namespaces
    let map_path = ctx.config_dir.join("ontology-map.json");
    let map: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&map_path).unwrap()).unwrap();
    assert_eq!(map["prefixes"]["schema"]["namespace"], "https://schema.org/");
    assert_eq!(map["prefixes"]["foaf"]["namespace"], "http://xmlns.com/foaf/0.1/");

    // Import a new ontology
    let output = ctx.cmd()
        .args(["import-ontology", "https://schema.org/version/latest/schema.ttl"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("UUID:"));
}

/// A vault made by hand (no network-bound `cerbo init`) with an empty v2 map,
/// plus a local ontology document reachable through a `file://` URL.
fn setup_offline_vault() -> (TestContext, String) {
    let ctx = setup();
    std::fs::create_dir_all(ctx.config_dir.join("objects")).unwrap();
    std::fs::write(
        ctx.config_dir.join("ontology-map.json"),
        r#"{"version":2,"prefixes":{}}"#,
    )
    .unwrap();
    let doc = ctx.tmp_dir.path().join("vocab.ttl");
    std::fs::write(&doc, "@prefix ex: <https://example.org/vocab#> .\nex:Person a <http://www.w3.org/2000/01/rdf-schema#Class> .\n").unwrap();
    let url = format!("file://{}", doc.display());
    (ctx, url)
}

fn object_count(ctx: &TestContext) -> usize {
    std::fs::read_dir(ctx.config_dir.join("objects")).unwrap().count()
}

#[test]
fn test_import_ontology_registers_prefix_and_namespace() {
    let (ctx, url) = setup_offline_vault();

    let output = ctx.cmd()
        .args(["import-ontology", &url, "--prefix", "ex", "--namespace", "https://example.org/vocab#", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let out: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let uuid = out["uuid"].as_str().unwrap().to_string();

    let map: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(ctx.config_dir.join("ontology-map.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(map["version"], 2);
    assert_eq!(map["prefixes"]["ex"]["namespace"], "https://example.org/vocab#");
    assert_eq!(map["prefixes"]["ex"]["ontology"], uuid.as_str());

    // A page that uses the prefix now resolves to the full IRI.
    let page_uuid = "aaaaaaaa-0000-0000-0000-000000000137";
    let page_dir = ctx.config_dir.join("objects").join(page_uuid);
    std::fs::create_dir_all(&page_dir).unwrap();
    std::fs::write(page_dir.join("page.md"), "[Alice]{ex:Person}").unwrap();
    std::fs::write(
        page_dir.join("meta.ttl"),
        format!(
            "<cerbo://objects/{page_uuid}> a <https://schema.org/NoteDigitalDocument> ; \
             <https://schema.org/name> \"Alice\" ."
        ),
    )
    .unwrap();
    let output = ctx.cmd()
        .args(["index", "--page", page_uuid])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let ttl = std::fs::read_to_string(page_dir.join("annotations.ttl")).unwrap();
    assert!(ttl.contains("<https://example.org/vocab#Person>"), "{ttl}");
}

#[test]
fn test_import_ontology_without_namespace_leaves_map_alone() {
    let (ctx, url) = setup_offline_vault();
    let map_path = ctx.config_dir.join("ontology-map.json");
    let map_before = std::fs::read_to_string(&map_path).unwrap();

    let output = ctx.cmd()
        .args(["import-ontology", &url])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("UUID:"));
    assert_eq!(object_count(&ctx), 1, "the ontology object is created");

    assert_eq!(std::fs::read_to_string(&map_path).unwrap(), map_before);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--prefix") && stderr.contains("--namespace"), "{stderr}");
}

#[test]
fn test_import_ontology_with_only_one_flag_fails() {
    let (ctx, url) = setup_offline_vault();
    let map_path = ctx.config_dir.join("ontology-map.json");
    let map_before = std::fs::read_to_string(&map_path).unwrap();

    for args in [
        vec!["import-ontology", url.as_str(), "--prefix", "ex"],
        vec!["import-ontology", url.as_str(), "--namespace", "https://example.org/vocab#"],
    ] {
        let output = ctx.cmd()
            .args(&args)
            .current_dir(ctx.tmp_dir.path())
            .output()
            .unwrap();
        assert!(!output.status.success(), "{args:?} must fail");
        assert_eq!(object_count(&ctx), 0, "{args:?} must create no object");
        assert_eq!(std::fs::read_to_string(&map_path).unwrap(), map_before);
    }
}

#[test]
fn test_import_ontology_rejects_an_invalid_namespace_before_creating_anything() {
    let (ctx, url) = setup_offline_vault();

    let output = ctx.cmd()
        .args(["import-ontology", &url, "--prefix", "ex", "--namespace", "not an iri"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(object_count(&ctx), 0);
}

#[test]
fn test_old_vault_not_compatible() {
    let ctx = setup();

    // Create a fake old-style vault (with slug-based storage)
    let old_slug_dir = ctx.tmp_dir.path().join("old-style-page");
    std::fs::create_dir_all(&old_slug_dir).unwrap();
    std::fs::write(old_slug_dir.join("page.md"), "# Old Style Page\n\nThis is old.\n").unwrap();

    // Try to init in the same directory - should create .cerbo/ but NOT recognize old pages
    let _ = ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output();

    // List pages - should NOT find "old-style-page"
    let output = ctx.cmd()
        .args(["page", "list"])
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
    ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Create two pages
    let output1 = ctx.cmd()
        .args(["page", "create", "Page 1"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let uuid1 = extract_uuid(&output1.stdout);

    let output2 = ctx.cmd()
        .args(["page", "create", "Page 2"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let uuid2 = extract_uuid(&output2.stdout);

    // Write content with link from page1 to page2
    let content = format!("Link to [page2](cerbo://{})", uuid2);
    let page1_path = ctx.config_dir.join("objects").join(&uuid1).join("page.md");
    std::fs::write(&page1_path, content).unwrap();

    // Run index
    let output = ctx.cmd()
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
    ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Create page with annotation
    let output = ctx.cmd()
        .args(["page", "create", "Test Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let uuid = extract_uuid(&output.stdout);

    // Write content with annotation
    let page_path = ctx.config_dir.join("objects").join(&uuid).join("page.md");
    std::fs::write(&page_path, "Some [knowledge]{schema:Thing} here").unwrap();

    // Run incremental index on single page
    let output = ctx.cmd()
        .args(["index", "--page", &uuid])
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
    ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Create page
    let output = ctx.cmd()
        .args(["page", "create", "Test Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Run index from different directory with explicit --vault path
    let output = ctx.cmd()
        .args(["index", "--vault", ctx.tmp_dir.path().to_str().unwrap()])
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
    ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Create page
    let output = ctx.cmd()
        .args(["page", "create", "Test Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Create subdirectory
    let subdir = ctx.tmp_dir.path().join("subdir");
    std::fs::create_dir_all(&subdir).unwrap();

    // Run index from subdirectory (should discover vault like Git)
    let output = ctx.cmd()
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
    ctx.cmd()
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Create page
    ctx.cmd()
        .args(["page", "create", "Test Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Run index with --json
    let output = ctx.cmd()
        .args(["index", "--json"])
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

/// Run the cerbo binary with isolated XDG dirs so tests don't touch the real user config.
fn cerbo_cmd(bin: &std::path::Path, xdg_home: &std::path::Path) -> Command {
    let mut cmd = Command::new(bin);
    cmd.env("XDG_CONFIG_HOME", xdg_home.join("config"))
        .env("XDG_CACHE_HOME", xdg_home.join("cache"));
    cmd
}

// ── CWD vault auto-discovery tests ────────────────────────────────────────────

#[test]
fn test_cwd_vault_registered_auto_discovery() {
    let ctx = setup();
    let xdg = TempDir::new().unwrap();

    // Init vault
    cerbo_cmd(&ctx.bin_path, xdg.path())
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Register the vault
    let add_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["vault", "add", "TestVault", ctx.tmp_dir.path().to_str().unwrap()])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(add_out.status.success(), "vault add failed: {}", String::from_utf8_lossy(&add_out.stderr));

    // Create a page
    let create_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["page", "create", "Discovery Page"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(create_out.status.success());

    // List pages WITHOUT --vault — should auto-discover from CWD
    let list_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["page", "list", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(list_out.status.success(), "page list failed: {}", String::from_utf8_lossy(&list_out.stderr));
    let pages: serde_json::Value = serde_json::from_slice(&list_out.stdout).unwrap();
    let titles: Vec<&str> = pages.as_array().unwrap()
        .iter()
        .filter_map(|p| p["title"].as_str())
        .collect();
    assert!(titles.contains(&"Discovery Page"), "expected page not found in: {:?}", titles);
}

#[test]
fn test_explicit_vault_flag_overrides_cwd() {
    let ctx = setup();
    let xdg = TempDir::new().unwrap();

    // Create two vaults
    let vault_a = TempDir::new().unwrap();
    let vault_b = TempDir::new().unwrap();

    for v in &[vault_a.path(), vault_b.path()] {
        cerbo_cmd(&ctx.bin_path, xdg.path())
            .arg("init")
            .current_dir(v)
            .output()
            .unwrap();
    }

    // Register vault-A
    let add_a = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["vault", "add", "VaultA", vault_a.path().to_str().unwrap()])
        .current_dir(vault_a.path())
        .output()
        .unwrap();
    assert!(add_a.status.success());
    let vault_a_id = String::from_utf8(add_a.stdout).unwrap()
        .split(':').next().unwrap_or("").trim().to_string();
    assert!(!vault_a_id.is_empty(), "could not extract vault-A id");

    // Register vault-B
    cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["vault", "add", "VaultB", vault_b.path().to_str().unwrap()])
        .current_dir(vault_b.path())
        .output()
        .unwrap();

    // Create a page in vault-A (from vault-A dir)
    let create_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["page", "create", "VaultA Page", "--json"])
        .current_dir(vault_a.path())
        .output()
        .unwrap();
    assert!(create_out.status.success());

    // List pages with --vault vault-A-id from vault-B dir → should show vault-A's page
    let list_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["page", "list", "--vault", &vault_a_id, "--json"])
        .current_dir(vault_b.path())
        .output()
        .unwrap();
    assert!(list_out.status.success(), "page list --vault failed: {}", String::from_utf8_lossy(&list_out.stderr));
    let pages: serde_json::Value = serde_json::from_slice(&list_out.stdout).unwrap();
    let titles: Vec<&str> = pages.as_array().unwrap()
        .iter()
        .filter_map(|p| p["title"].as_str())
        .collect();
    assert!(titles.contains(&"VaultA Page"), "expected VaultA page in: {:?}", titles);
}

#[test]
fn test_unregistered_cwd_vault_auto_registers_silently() {
    let ctx = setup();
    let xdg = TempDir::new().unwrap();

    // Init vault but do NOT register it manually
    cerbo_cmd(&ctx.bin_path, xdg.path())
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // page list should succeed and produce NO warning on stderr (auto-registered silently)
    let list_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["page", "list", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(list_out.status.success(), "page list failed: {}", String::from_utf8_lossy(&list_out.stderr));
    let stderr = String::from_utf8_lossy(&list_out.stderr);
    assert!(
        !stderr.contains("not registered"),
        "unexpected 'not registered' warning; auto-registration should be silent: {}", stderr
    );
}

#[test]
fn test_auto_registration_appears_in_vault_list() {
    let ctx = setup();
    let xdg = TempDir::new().unwrap();

    // Init vault without manually registering it
    cerbo_cmd(&ctx.bin_path, xdg.path())
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Run any command from inside the vault to trigger auto-registration
    cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["page", "list", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // vault list --json should now show the vault with is_auto: true
    let list_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["vault", "list", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(list_out.status.success(), "vault list failed: {}", String::from_utf8_lossy(&list_out.stderr));
    let vaults: serde_json::Value = serde_json::from_slice(&list_out.stdout).unwrap();
    let vaults_arr = vaults.as_array().unwrap();
    assert_eq!(vaults_arr.len(), 1, "expected 1 vault in list");
    assert_eq!(vaults_arr[0]["is_auto"], serde_json::Value::Bool(true));
}

#[test]
fn test_auto_registration_is_idempotent() {
    let ctx = setup();
    let xdg = TempDir::new().unwrap();

    cerbo_cmd(&ctx.bin_path, xdg.path())
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Run twice from the same vault
    for _ in 0..2 {
        cerbo_cmd(&ctx.bin_path, xdg.path())
            .args(["page", "list", "--json"])
            .current_dir(ctx.tmp_dir.path())
            .output()
            .unwrap();
    }

    let list_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["vault", "list", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let vaults: serde_json::Value = serde_json::from_slice(&list_out.stdout).unwrap();
    assert_eq!(vaults.as_array().unwrap().len(), 1, "duplicate entry on second invocation");
}

#[test]
fn test_vault_approve_promotes_auto_to_manual() {
    let ctx = setup();
    let xdg = TempDir::new().unwrap();

    cerbo_cmd(&ctx.bin_path, xdg.path())
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Trigger auto-registration
    cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["page", "list", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Get the auto-registered vault ID
    let list_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["vault", "list", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let vaults: serde_json::Value = serde_json::from_slice(&list_out.stdout).unwrap();
    let id = vaults[0]["id"].as_str().unwrap().to_string();

    // Approve it
    let approve_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["vault", "approve", &id, "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(approve_out.status.success(), "vault approve failed: {}", String::from_utf8_lossy(&approve_out.stderr));

    // Subsequent vault list should show is_auto: false
    let list_out2 = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["vault", "list", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let vaults2: serde_json::Value = serde_json::from_slice(&list_out2.stdout).unwrap();
    assert_eq!(vaults2.as_array().unwrap().len(), 1);
    assert_eq!(vaults2[0]["is_auto"], serde_json::Value::Bool(false));
}

#[test]
fn test_vault_remove_works_on_auto_vault() {
    let ctx = setup();
    let xdg = TempDir::new().unwrap();

    cerbo_cmd(&ctx.bin_path, xdg.path())
        .arg("init")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Trigger auto-registration
    cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["page", "list", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    // Get ID
    let list_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["vault", "list", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let vaults: serde_json::Value = serde_json::from_slice(&list_out.stdout).unwrap();
    let id = vaults[0]["id"].as_str().unwrap().to_string();

    // Remove it — run from a plain dir so it doesn't re-register during the command
    let plain = TempDir::new().unwrap();
    let remove_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["vault", "remove", &id, "--json"])
        .current_dir(plain.path())
        .output()
        .unwrap();
    assert!(remove_out.status.success(), "vault remove failed: {}", String::from_utf8_lossy(&remove_out.stderr));

    // vault list should now be empty (running from plain dir)
    let list_out2 = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["vault", "list", "--json"])
        .current_dir(plain.path())
        .output()
        .unwrap();
    let vaults2: serde_json::Value = serde_json::from_slice(&list_out2.stdout).unwrap();
    assert_eq!(vaults2.as_array().unwrap().len(), 0, "vault should be gone after remove");
}

#[test]
fn test_no_vault_no_active_returns_error() {
    let ctx = setup();
    let xdg = TempDir::new().unwrap();

    // Use a plain dir with NO .cerbo/ and an empty global config
    let plain_dir = TempDir::new().unwrap();

    let list_out = cerbo_cmd(&ctx.bin_path, xdg.path())
        .args(["page", "list"])
        .current_dir(plain_dir.path())
        .output()
        .unwrap();
    assert!(!list_out.status.success(), "expected non-zero exit when no vault available");
    let stderr = String::from_utf8_lossy(&list_out.stderr);
    let stdout = String::from_utf8_lossy(&list_out.stdout);
    assert!(
        stderr.contains("vault") || stdout.contains("vault"),
        "expected vault-related error, got stderr={} stdout={}", stderr, stdout
    );
}

// ── cerbo migrate vocabulary ──────────────────────────────────────────────────

const MIGRATE_PAGE: &str = "ba079793-b2ff-4ca6-ba88-8764ce6668c3";
const MIGRATE_SCHEMA_ONTOLOGY: &str = "5c1e0137-0000-4000-8000-000000000137";

/// A `meta.ttl` as Cerbo wrote it before the schema.org switch.
fn legacy_meta(uuid: &str, title: &str) -> String {
    format!(
        concat!(
            "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
            "@prefix schema: <cerbo://ontology/schema/> .\n",
            "@prefix cerbo: <cerbo://ontology/> .\n",
            "<cerbo://objects/{uuid}> cerbo:type cerbo:Product ;\n",
            "\tcerbo:title \"{title}\" ;\n",
            "\tschema:dateCreated \"2026-10-25T13:17:54.772697296+00:00\"^^xsd:dateTime ;\n",
            "\tschema:dateModified \"2026-10-29T11:44:53.954037862+00:00\"^^xsd:dateTime ;\n",
            "\tcerbo:slug \"index\" ;\n",
            "\tcerbo:virtualPath \"notes/index\" .\n",
        ),
        uuid = uuid,
        title = title
    )
}

fn put_object_file(vault: &Path, uuid: &str, file: &str, content: &str) -> std::path::PathBuf {
    let dir = vault.join(".cerbo").join("objects").join(uuid);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(file);
    std::fs::write(&path, content).unwrap();
    path
}

/// A vault holding one legacy page with a legacy annotation, registered under
/// `name`. Returns the vault ID.
fn legacy_vault(ctx: &TestContext, root: &Path, name: &str) -> String {
    std::fs::create_dir_all(root.join(".cerbo").join("objects")).unwrap();
    std::fs::write(
        root.join(".cerbo").join("ontology-map.json"),
        format!(
            r#"{{"version":2,"prefixes":{{"schema":{{"namespace":"https://schema.org/","ontology":"{MIGRATE_SCHEMA_ONTOLOGY}"}}}}}}"#
        ),
    )
    .unwrap();
    put_object_file(root, MIGRATE_PAGE, "meta.ttl", &legacy_meta(MIGRATE_PAGE, "Index"));
    put_object_file(root, MIGRATE_PAGE, "page.md", "# Index\nI visited [Berlin]{schema:Place}.\n");
    put_object_file(
        root,
        MIGRATE_PAGE,
        "annotations.ttl",
        &format!(
            concat!(
                "@prefix cerbo: <cerbo://ontology/> .\n",
                "<cerbo://objects/{page}> cerbo:annotation [\n",
                "    cerbo:concept \"Berlin\" ;\n",
                "    cerbo:type <cerbo://objects/{ontology}/Place> ;\n",
                "    cerbo:position \"2,11\" ] .\n",
            ),
            page = MIGRATE_PAGE,
            ontology = MIGRATE_SCHEMA_ONTOLOGY
        ),
    );

    let output = ctx.cmd()
        .args(["vault", "add", name, root.to_str().unwrap(), "--json"])
        .current_dir(ctx.xdg_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let vault: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    vault["id"].as_str().unwrap().to_string()
}

/// Run `cerbo migrate vocabulary <args>` from outside any vault.
fn migrate(ctx: &TestContext, args: &[&str]) -> std::process::Output {
    ctx.cmd()
        .args(["migrate", "vocabulary"])
        .args(args)
        .current_dir(ctx.xdg_dir.path())
        .output()
        .unwrap()
}

fn json_summary(output: &std::process::Output) -> serde_json::Value {
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {}", String::from_utf8_lossy(&output.stdout)));
    for field in ["scanned", "rewritten", "errors"] {
        assert!(summary[field].is_u64(), "{field} is not a number: {summary}");
    }
    summary
}

/// Every file under `dir` with its bytes and modification time.
fn snapshot(dir: &Path) -> std::collections::BTreeMap<std::path::PathBuf, (Vec<u8>, std::time::SystemTime)> {
    let mut files = std::collections::BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
                files.insert(path.clone(), (std::fs::read(&path).unwrap(), mtime));
            }
        }
    }
    files
}

#[test]
fn test_migrate_vocabulary_rewrites_legacy_meta() {
    let ctx = setup();
    let root = ctx.tmp_dir.path();
    legacy_vault(&ctx, root, "Legacy");

    let output = migrate(&ctx, &[]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let meta = std::fs::read_to_string(
        root.join(".cerbo/objects").join(MIGRATE_PAGE).join("meta.ttl"),
    )
    .unwrap();
    assert!(meta.contains("a schema:NoteDigitalDocument"), "{meta}");
    assert!(meta.contains("@prefix schema: <https://schema.org/>"), "{meta}");
    assert!(meta.contains("schema:name \"Index\""), "{meta}");
    assert!(meta.contains("schema:dateCreated"), "{meta}");
    for legacy in ["cerbo:Product", "cerbo:type", "cerbo:title", "cerbo://ontology/schema/"] {
        assert!(!meta.contains(legacy), "{legacy} survived:\n{meta}");
    }
    // Values are carried over unchanged.
    for value in [
        "\"2026-10-25T13:17:54.772697296+00:00\"",
        "\"2026-10-29T11:44:53.954037862+00:00\"",
        "cerbo:slug \"index\"",
        "cerbo:virtualPath \"notes/index\"",
    ] {
        assert!(meta.contains(value), "{value} lost:\n{meta}");
    }
}

#[test]
fn test_migrate_vocabulary_rewrites_legacy_annotation_type() {
    let ctx = setup();
    let root = ctx.tmp_dir.path();
    legacy_vault(&ctx, root, "Legacy");

    let output = migrate(&ctx, &[]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let ttl = std::fs::read_to_string(
        root.join(".cerbo/objects").join(MIGRATE_PAGE).join("annotations.ttl"),
    )
    .unwrap();
    assert!(ttl.contains("schema:mentions"), "{ttl}");
    assert!(ttl.contains("a schema:Place"), "{ttl}");
    assert!(ttl.contains("schema:name \"Berlin\""), "{ttl}");
    assert!(!ttl.contains("cerbo:annotation") && !ttl.contains("cerbo:concept"), "{ttl}");
    assert!(!ttl.contains(MIGRATE_SCHEMA_ONTOLOGY), "{ttl}");
}

#[test]
fn test_migrate_vocabulary_re_derives_a_lost_prefix() {
    let ctx = setup();
    let root = ctx.tmp_dir.path();
    legacy_vault(&ctx, root, "Legacy");
    let annotations = put_object_file(
        root,
        MIGRATE_PAGE,
        "annotations.ttl",
        &format!(
            "<cerbo://objects/{MIGRATE_PAGE}> <cerbo://ontology/annotation> [ \
             <cerbo://ontology/concept> \"Berlin\" ; \
             <cerbo://ontology/type> <cerbo://ontology/Place> ] .\n"
        ),
    );

    let output = migrate(&ctx, &[]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let ttl = std::fs::read_to_string(&annotations).unwrap();
    assert!(ttl.contains("a schema:Place"), "{ttl}");
    assert!(!ttl.contains("cerbo://ontology/Place") && !ttl.contains("cerbo:Place"), "{ttl}");
}

#[test]
fn test_migrate_vocabulary_one_vault_only() {
    let ctx = setup();
    let first = ctx.tmp_dir.path().join("first");
    let second = ctx.tmp_dir.path().join("second");
    let first_id = legacy_vault(&ctx, &first, "First");
    legacy_vault(&ctx, &second, "Second");
    let untouched = snapshot(&second.join(".cerbo"));

    let output = migrate(&ctx, &["--vault", &first_id, "--json"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(json_summary(&output)["rewritten"], 2);

    let meta = std::fs::read_to_string(
        first.join(".cerbo/objects").join(MIGRATE_PAGE).join("meta.ttl"),
    )
    .unwrap();
    assert!(meta.contains("schema:NoteDigitalDocument"), "{meta}");
    assert_eq!(snapshot(&second.join(".cerbo")), untouched, "the other vault changed");

    // A vault path works too, the same as for `cerbo index --vault`.
    let output = migrate(&ctx, &["--vault", second.to_str().unwrap(), "--json"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(json_summary(&output)["rewritten"], 2);
}

#[test]
fn test_migrate_vocabulary_unknown_vault_fails() {
    let ctx = setup();

    let output = migrate(&ctx, &["--vault", "no-such-vault-137"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no-such-vault-137"));
}

#[test]
fn test_migrate_vocabulary_unreadable_object() {
    let ctx = setup();
    let root = ctx.tmp_dir.path();
    std::fs::create_dir_all(root.join(".cerbo/objects")).unwrap();
    let uuid = |n: usize| format!("00000000-0000-4000-8000-{n:012}");
    for n in 0..137 {
        put_object_file(root, &uuid(n), "meta.ttl", &legacy_meta(&uuid(n), &format!("Page {n}")));
    }
    let broken = uuid(42);
    let broken_path = put_object_file(root, &broken, "meta.ttl", "this is {{ not turtle at all");

    let output = migrate(&ctx, &["--vault", root.to_str().unwrap(), "--json"]);

    assert!(!output.status.success(), "an unreadable object must fail the run");
    assert!(String::from_utf8_lossy(&output.stderr).contains(&broken), "stderr must name {broken}");
    let summary = json_summary(&output);
    assert_eq!(summary["scanned"], 137);
    assert_eq!(summary["rewritten"], 136);
    assert_eq!(summary["errors"], 1);
    assert_eq!(std::fs::read_to_string(&broken_path).unwrap(), "this is {{ not turtle at all");
}

#[test]
fn test_migrate_vocabulary_dry_run_changes_nothing() {
    let ctx = setup();
    let root = ctx.tmp_dir.path();
    legacy_vault(&ctx, root, "Legacy");
    let before = snapshot(&root.join(".cerbo"));

    let output = migrate(&ctx, &["--dry-run"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    assert_eq!(snapshot(&root.join(".cerbo")), before, "a dry run modified .cerbo/");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Would rewrite 2 files"), "{stdout}");
    assert!(stdout.contains("meta.ttl") && stdout.contains("annotations.ttl"), "{stdout}");

    let output = migrate(&ctx, &["--dry-run", "--json"]);
    assert_eq!(json_summary(&output)["rewritten"], 2);
    assert_eq!(snapshot(&root.join(".cerbo")), before);
}

#[test]
fn test_migrate_vocabulary_second_run_touches_nothing() {
    let ctx = setup();
    let root = ctx.tmp_dir.path();
    legacy_vault(&ctx, root, "Legacy");

    let first = migrate(&ctx, &["--json"]);
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    assert_eq!(json_summary(&first)["rewritten"], 2);

    let before = snapshot(&root.join(".cerbo"));
    let second = migrate(&ctx, &["--json"]);
    assert!(second.status.success(), "{}", String::from_utf8_lossy(&second.stderr));
    assert_eq!(json_summary(&second)["rewritten"], 0);
    assert_eq!(snapshot(&root.join(".cerbo")), before, "a file or its mtime changed");
}

#[test]
fn test_migrate_vocabulary_json_summary() {
    let ctx = setup();
    let root = ctx.tmp_dir.path();
    legacy_vault(&ctx, root, "Legacy");

    let output = migrate(&ctx, &["--json"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let summary = json_summary(&output);
    let mut fields: Vec<&str> = summary
        .as_object()
        .expect("a JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    fields.sort();
    assert_eq!(fields, ["errors", "rewritten", "scanned"], "{summary}");
    assert_eq!(summary["scanned"], 1);
    assert_eq!(summary["rewritten"], 2);
    assert_eq!(summary["errors"], 0);
}

#[test]
fn page_write_refreshes_annotations_without_an_index_run() {
    let ctx = setup();
    ctx.cmd().arg("init").current_dir(ctx.tmp_dir.path()).output().unwrap();
    let output = ctx.cmd()
        .args(["page", "create", "Trip"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    let uuid = extract_uuid(&output.stdout);
    let annotations = ctx.config_dir.join("objects").join(&uuid).join("annotations.ttl");

    let write = |content: &str| {
        let output = ctx.cmd()
            .args(["page", "write", &uuid, content])
            .current_dir(ctx.tmp_dir.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        std::fs::read_to_string(&annotations).unwrap()
    };

    let doc = write("I visited [Berlin]{schema:Place} with [Alice]{foaf:Person}.");
    assert!(doc.contains("\"Berlin\"") && doc.contains("<http://xmlns.com/foaf/0.1/Person>"), "{doc}");

    let doc = write("I visited Berlin.");
    assert!(!doc.contains("schema:mentions") && !doc.contains("Berlin"), "{doc}");
}

#[test]
fn index_names_an_object_with_two_classes_and_fails() {
    let ctx = setup();
    ctx.cmd().arg("init").current_dir(ctx.tmp_dir.path()).output().unwrap();
    let mut uuids = Vec::new();
    for title in ["Fine", "Both"] {
        let output = ctx.cmd()
            .args(["page", "create", title])
            .current_dir(ctx.tmp_dir.path())
            .output()
            .unwrap();
        uuids.push(extract_uuid(&output.stdout));
    }
    let both_dir = ctx.config_dir.join("objects").join(&uuids[1]);
    std::fs::write(
        both_dir.join("meta.ttl"),
        format!(
            "<cerbo://objects/{}> a <https://schema.org/NoteDigitalDocument> , <https://schema.org/WebPage> .\n",
            uuids[1]
        ),
    )
    .unwrap();
    std::fs::write(both_dir.join("page.md"), "[Berlin]{schema:Place}\n").unwrap();

    let output = ctx.cmd()
        .args(["index", "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&uuids[1]), "{stderr}");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["pages_processed"], 1, "{json}");
    assert_eq!(json["errors"], 1, "reported once: {json}");
    assert!(!both_dir.join("annotations.ttl").exists());
}
