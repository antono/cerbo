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

    // The link target gets no file: backlinks come from the semantic index.
    let target_dir = ctx.config_dir.join("objects").join(&uuid2);
    let mut target_files: Vec<_> = std::fs::read_dir(&target_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    target_files.sort();
    assert_eq!(target_files, ["annotations.ttl", "meta.ttl", "page.md"], "only the page's own files");

    let output = ctx.cmd()
        .args(["page", "backlinks", &uuid2])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), format!("{uuid1}: Page 1\n"));
}

/// A vault with no network-fetched ontologies, so the test runs offline.
fn setup_empty_vault() -> TestContext {
    let ctx = setup();
    std::fs::create_dir_all(ctx.config_dir.join("objects")).unwrap();
    std::fs::write(ctx.config_dir.join("ontology-map.json"), r#"{"version":2,"prefixes":{}}"#).unwrap();
    ctx
}

fn create_page(ctx: &TestContext, title: &str) -> String {
    let output = ctx.cmd()
        .args(["page", "create", title])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    extract_uuid(&output.stdout)
}

fn write_page(ctx: &TestContext, uuid: &str, content: &str) {
    let output = ctx.cmd()
        .args(["page", "write", uuid, content])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_page_backlinks_as_json() {
    let ctx = setup_empty_vault();
    let target = create_page(&ctx, "Target");
    let beta = create_page(&ctx, "Beta");
    let alpha = create_page(&ctx, "Alpha");
    create_page(&ctx, "Unrelated");
    write_page(&ctx, &beta, &format!("# Beta\n\n[T](cerbo://{target}) [again](cerbo://objects/{target})"));
    write_page(&ctx, &alpha, &format!("# Alpha\n\n[T](cerbo://{target})"));

    let output = ctx.cmd()
        .args(["page", "backlinks", &target, "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let entries: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("stdout must be only a JSON array");
    assert_eq!(
        entries,
        serde_json::json!([
            {"uuid": alpha, "title": "Alpha"},
            {"uuid": beta, "title": "Beta"},
        ])
    );

    // The index lives in the user cache, never in the vault.
    assert!(ctx.xdg_dir.path().join("cache").join("cerbo").join("index").is_dir());
    let vault_entries: Vec<_> = std::fs::read_dir(ctx.tmp_dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(vault_entries, [".cerbo"], "something besides .cerbo/ was written into the vault");

    // A page with no backlinks is an empty array.
    let output = ctx.cmd()
        .args(["page", "backlinks", &alpha, "--json"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(), serde_json::json!([]));
}

#[test]
fn test_page_backlinks_unknown_object() {
    let ctx = setup_empty_vault();
    create_page(&ctx, "Some page");

    let output = ctx.cmd()
        .args(["page", "backlinks", "00000000-0000-0000-0000-000000000000"])
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Object not found"), "{stderr}");
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

// ── `cerbo index` and the semantic index (spec page-metadata-index) ──────────

/// Run cerbo inside the test vault.
fn run(ctx: &TestContext, args: &[&str]) -> std::process::Output {
    ctx.cmd().args(args).current_dir(ctx.tmp_dir.path()).output().unwrap()
}

/// Run cerbo inside the test vault and expect success.
fn run_ok(ctx: &TestContext, args: &[&str]) -> String {
    let output = run(ctx, args);
    assert!(output.status.success(), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

/// `cerbo index --json` (plus `extra`), parsed.
fn index_json(ctx: &TestContext, extra: &[&str]) -> serde_json::Value {
    let mut args = vec!["index", "--json"];
    args.extend_from_slice(extra);
    serde_json::from_str(&run_ok(ctx, &args)).expect("stdout must be only the JSON summary")
}

/// Write a page's files directly, as git or a sync tool would.
fn plant_page(ctx: &TestContext, uuid: &str, title: &str, body: &str) {
    let dir = ctx.config_dir.join("objects").join(uuid);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("meta.ttl"),
        format!(
            "<cerbo://objects/{uuid}> a <https://schema.org/NoteDigitalDocument> ;\n  \
             <https://schema.org/name> \"{title}\" .\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("page.md"), body).unwrap();
}

fn page_md(ctx: &TestContext, uuid: &str) -> std::path::PathBuf {
    ctx.config_dir.join("objects").join(uuid).join("page.md")
}

fn backrefs_files(ctx: &TestContext) -> usize {
    std::fs::read_dir(ctx.config_dir.join("objects"))
        .unwrap()
        .filter(|e| e.as_ref().unwrap().path().join("backrefs.ttl").exists())
        .count()
}

#[test]
fn test_index_removes_137_legacy_backrefs_files() {
    let ctx = setup_empty_vault();
    let hub = "13700000-0000-4000-8000-000000000000";
    plant_page(&ctx, hub, "Hub", "# Hub");
    for n in 1..137 {
        let uuid = format!("13700000-0000-4000-8000-{n:012}");
        plant_page(&ctx, &uuid, &format!("Page {n}"), &format!("[Hub](cerbo://{hub})"));
    }
    let objects = ctx.config_dir.join("objects");
    for entry in std::fs::read_dir(&objects).unwrap() {
        std::fs::write(entry.unwrap().path().join("backrefs.ttl"), "# written by an older Cerbo\n").unwrap();
    }
    assert_eq!(backrefs_files(&ctx), 137);

    let summary = index_json(&ctx, &[]);

    assert_eq!(summary["legacy_backrefs_removed"], 137, "{summary}");
    assert!(summary["triples"].as_u64().unwrap() > 0, "{summary}");
    assert_eq!(backrefs_files(&ctx), 0, "no backrefs.ttl may remain under .cerbo/objects/");

    let text = run_ok(&ctx, &["index"]);
    assert!(text.contains("triples"), "{text}");
    assert!(!text.contains("legacy"), "nothing left to remove: {text}");

    // The hub's backlinks are all 136 pages, from the index.
    let backlinks: serde_json::Value = serde_json::from_str(&run_ok(&ctx, &["page", "backlinks", hub, "--json"])).unwrap();
    assert_eq!(backlinks.as_array().unwrap().len(), 136);
}

#[test]
fn test_leftover_backrefs_file_is_ignored() {
    let ctx = setup_empty_vault();
    let target = "13700000-0000-4000-8000-000000000001";
    let former = "13700000-0000-4000-8000-000000000002";
    plant_page(&ctx, target, "Target", "# Target");
    plant_page(&ctx, former, "Former", "# Former, no longer linking");
    std::fs::write(
        ctx.config_dir.join("objects").join(target).join("backrefs.ttl"),
        format!(
            "<cerbo://objects/{target}> <cerbo://ontology/hasBacklink> <cerbo://objects/{former}> .\n"
        ),
    )
    .unwrap();

    assert_eq!(run_ok(&ctx, &["page", "backlinks", target]), "");
}

#[test]
fn test_index_default_run_rebuilds_the_graph() {
    let ctx = setup_empty_vault();
    let target = create_page(&ctx, "Target");
    let source = create_page(&ctx, "Source");
    run_ok(&ctx, &["index"]);

    // Changed behind Cerbo's back, then the index is rebuilt from the files.
    std::fs::write(page_md(&ctx, &source), format!("[T](cerbo://{target}) [Berlin]{{schema:Place}}")).unwrap();
    let summary = index_json(&ctx, &[]);

    let count = run_ok(&ctx, &["sparql", "--json", "SELECT (COUNT(*) AS ?n) WHERE { GRAPH ?g { ?s ?p ?o } }"]);
    let count: serde_json::Value = serde_json::from_str(&count).unwrap();
    assert_eq!(
        count["results"]["bindings"][0]["n"]["value"].as_str().unwrap(),
        summary["triples"].to_string(),
        "the index matches the files without any catch-up"
    );
    assert_eq!(run_ok(&ctx, &["page", "backlinks", &target]), format!("{source}: Source\n"));
}

#[test]
fn test_index_single_page_replaces_only_its_triples() {
    let ctx = setup_empty_vault();
    let target = create_page(&ctx, "Target");
    let a = create_page(&ctx, "A");
    let b = create_page(&ctx, "B");
    let before = index_json(&ctx, &[])["triples"].as_u64().unwrap();

    // Both gain a link behind Cerbo's back; only A is indexed.
    for uuid in [&a, &b] {
        std::fs::write(page_md(&ctx, uuid), format!("[T](cerbo://{target})")).unwrap();
    }
    let summary = index_json(&ctx, &["--page", &a]);

    assert_eq!(summary["pages_processed"], 1);
    assert_eq!(summary["legacy_backrefs_removed"], 0);
    assert_eq!(
        summary["triples"].as_u64().unwrap(),
        before + 1,
        "only A's citation is new in the index: {summary}"
    );
}

#[test]
fn test_index_twice_gives_identical_files_and_answers() {
    let ctx = setup_empty_vault();
    let target = create_page(&ctx, "Target");
    let source = create_page(&ctx, "Source");
    std::fs::write(page_md(&ctx, &source), format!("[T](cerbo://{target}) [Alice]{{schema:Person}}")).unwrap();
    let annotations = ctx.config_dir.join("objects").join(&source).join("annotations.ttl");
    let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o } ORDER BY ?s ?p ?o";

    let first = index_json(&ctx, &[]);
    let first_annotations = std::fs::read(&annotations).unwrap();
    let first_answer = run_ok(&ctx, &["sparql", "--json", query]);

    let second = index_json(&ctx, &[]);
    assert_eq!(first, second);
    assert_eq!(std::fs::read(&annotations).unwrap(), first_annotations);
    assert_eq!(run_ok(&ctx, &["sparql", "--json", query]), first_answer);

    // The same for one page.
    let one = index_json(&ctx, &["--page", &source]);
    let one_annotations = std::fs::read(&annotations).unwrap();
    assert_eq!(index_json(&ctx, &["--page", &source]), one);
    assert_eq!(std::fs::read(&annotations).unwrap(), one_annotations);
    assert_eq!(run_ok(&ctx, &["sparql", "--json", query]), first_answer);
}

#[test]
fn test_index_link_added_then_removed_leaves_no_backlink() {
    let ctx = setup_empty_vault();
    let target = create_page(&ctx, "Target");
    let a = create_page(&ctx, "A");
    let original = index_json(&ctx, &[]);

    std::fs::write(page_md(&ctx, &a), format!("# A\n\n[T](cerbo://{target})")).unwrap();
    index_json(&ctx, &[]);
    assert_eq!(run_ok(&ctx, &["page", "backlinks", &target]), format!("{a}: A\n"));

    std::fs::write(page_md(&ctx, &a), "# A\n").unwrap();
    let last = index_json(&ctx, &[]);
    assert_eq!(run_ok(&ctx, &["page", "backlinks", &target]), "");
    assert_eq!(last["triples"], original["triples"], "{original} vs {last}");
}

#[test]
fn test_index_json_summary_has_semantic_fields() {
    let ctx = setup_empty_vault();
    create_page(&ctx, "Page");
    let summary = index_json(&ctx, &[]);
    assert!(summary["triples"].is_u64(), "{summary}");
    assert!(summary["legacy_backrefs_removed"].is_u64(), "{summary}");
}

// ── `cerbo sparql` (spec graph-query) ─────────────────────────────────────────

/// A vault with a page titled "Index" that mentions a place and a person in
/// the vault's own `ex:` vocabulary.
fn sparql_vault() -> (TestContext, String) {
    let ctx = setup();
    std::fs::create_dir_all(ctx.config_dir.join("objects")).unwrap();
    std::fs::write(
        ctx.config_dir.join("ontology-map.json"),
        r#"{"version":2,"prefixes":{"ex":{"namespace":"https://example.org/vocab#"}}}"#,
    )
    .unwrap();
    let index = create_page(&ctx, "Index");
    create_page(&ctx, "Second page");
    write_page(&ctx, &index, "# Index\n\n[Berlin]{schema:Place} and [Alice]{ex:Person}");
    (ctx, index)
}

#[test]
fn test_sparql_lists_page_titles_as_a_table() {
    let (ctx, _) = sparql_vault();
    let out = run_ok(&ctx, &["sparql", "SELECT ?t WHERE { ?p a schema:NoteDigitalDocument ; schema:name ?t } ORDER BY ?t"]);
    assert_eq!(out, "t\n-----------\nIndex\nSecond page\n");
}

#[test]
fn test_sparql_explicit_vault() {
    let (ctx, _) = sparql_vault();
    let elsewhere = TempDir::new().unwrap();
    let output = ctx.cmd()
        .args(["sparql", "--vault", ctx.tmp_dir.path().to_str().unwrap(), "ASK { ?s schema:name \"Second page\" }"])
        .current_dir(elsewhere.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "true\n");
}

#[test]
fn test_sparql_query_from_file_and_stdin() {
    let (ctx, _) = sparql_vault();
    let file = ctx.tmp_dir.path().join("places.rq");
    std::fs::write(&file, "SELECT ?name WHERE { ?x a schema:Place ; schema:name ?name }").unwrap();
    assert_eq!(run_ok(&ctx, &["sparql", "--file", file.to_str().unwrap()]), "name\n------\nBerlin\n");

    use std::io::Write;
    let mut child = ctx.cmd()
        .args(["sparql", "-"])
        .current_dir(ctx.tmp_dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"ASK { ?s ?p ?o }").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "true\n");

    // A query and --file together, or neither, is a usage error.
    assert!(!run(&ctx, &["sparql", "--file", file.to_str().unwrap(), "ASK {}"]).status.success());
    assert!(!run(&ctx, &["sparql"]).status.success());
}

#[test]
fn test_sparql_predeclared_prefixes() {
    let (ctx, _) = sparql_vault();
    // cerbo: and schema: without PREFIX lines; schema: is schema.org.
    assert_eq!(run_ok(&ctx, &["sparql", "ASK { ?s schema:name \"Index\" ; schema:mentions ?x . ?x a schema:Place }"]), "true\n");
    assert_eq!(
        run_ok(&ctx, &["sparql", "ASK { ?x a schema:Place } "]),
        run_ok(&ctx, &["sparql", "ASK { ?x a <https://schema.org/Place> }"])
    );
    assert_eq!(
        run_ok(&ctx, &["sparql", "SELECT ?c WHERE { BIND(cerbo:slug AS ?c) }"]),
        "c\n---------------------\ncerbo://ontology/slug\n"
    );
    // The vault map's ex: prefix.
    assert_eq!(
        run_ok(&ctx, &["sparql", "SELECT ?n WHERE { ?x a ex:Person ; schema:name ?n }"]),
        "n\n-----\nAlice\n"
    );
    assert_eq!(run_ok(&ctx, &["sparql", "ASK { ?x a <https://example.org/vocab#Person> }"]), "true\n");
}

#[test]
fn test_sparql_ask_and_construct() {
    let (ctx, index) = sparql_vault();
    assert_eq!(run_ok(&ctx, &["sparql", "ASK { ?s schema:name \"Index\" }"]), "true\n");
    assert_eq!(run_ok(&ctx, &["sparql", "ASK { ?s schema:name \"Missing\" }"]), "false\n");

    let construct = "CONSTRUCT { ?p schema:name ?t } WHERE { ?p a schema:NoteDigitalDocument ; schema:name ?t }";
    let turtle = run_ok(&ctx, &["sparql", construct]);
    let triples = cerbo_core::rdf::parse(&turtle).expect("CONSTRUCT output must be valid Turtle");
    assert_eq!(triples.len(), 2, "{turtle}");
    assert!(turtle.contains(&index), "{turtle}");

    let jsonld: serde_json::Value = serde_json::from_str(&run_ok(&ctx, &["sparql", "--json", construct]))
        .expect("CONSTRUCT --json must be JSON-LD");
    assert!(jsonld.to_string().contains(&format!("cerbo://objects/{index}")), "{jsonld}");
}

#[test]
fn test_sparql_select_as_json() {
    let (ctx, _) = sparql_vault();
    let out = run_ok(&ctx, &["sparql", "--json", "SELECT ?t WHERE { ?p a schema:NoteDigitalDocument ; schema:name ?t } ORDER BY ?t"]);
    let json: serde_json::Value = serde_json::from_str(&out).expect("stdout must be one JSON document");
    assert_eq!(json["head"]["vars"], serde_json::json!(["t"]));
    let titles: Vec<_> = json["results"]["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["t"]["value"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["Index", "Second page"]);

    let ask: serde_json::Value = serde_json::from_str(&run_ok(&ctx, &["sparql", "--json", "ASK { ?s ?p ?o }"])).unwrap();
    assert_eq!(ask["boolean"], true);
}

#[test]
fn test_sparql_rejects_updates() {
    let (ctx, _) = sparql_vault();
    let count = "SELECT (COUNT(*) AS ?n) WHERE { GRAPH ?g { ?s ?p ?o } }";
    let before = run_ok(&ctx, &["sparql", "--json", count]);

    let output = run(&ctx, &["sparql", "INSERT DATA { <a:b> <a:c> \"d\" }"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("vault data is changed through files"), "{stderr}");

    assert_eq!(run_ok(&ctx, &["sparql", "--json", count]), before, "the index is unchanged");
    assert_eq!(run_ok(&ctx, &["sparql", "ASK { <a:b> ?p ?o }"]), "false\n");
}

#[test]
fn test_sparql_reports_syntax_and_evaluation_errors() {
    let (ctx, _) = sparql_vault();
    let output = run(&ctx, &["sparql", "SELEC ?s WHERE { ?s ?p ?o }"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("SPARQL syntax error"), "{stderr}");
    assert!(stderr.contains("1:1"), "the position is reported: {stderr}");

    let output = run(&ctx, &["sparql", "SELECT * WHERE { SERVICE <http://example.org/sparql> { ?s ?p ?o } }"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("SPARQL evaluation error"), "{stderr}");
}

// ── `cerbo search`, `cerbo similar`, `cerbo index --embeddings` (spec semantic-search) ──

/// A proxy nobody listens on: any model download fails at once, so a test
/// proves the command needed no network.
const DEAD_PROXY: &str = "http://127.0.0.1:1";

/// Run cerbo inside the test vault with the word-counting mock embedder in
/// place of the model.
fn run_mock(ctx: &TestContext, args: &[&str]) -> std::process::Output {
    ctx.cmd()
        .args(args)
        .env(cerbo_core::semantic::MOCK_EMBEDDER_ENV, "1")
        .current_dir(ctx.tmp_dir.path())
        .output()
        .unwrap()
}

fn run_mock_ok(ctx: &TestContext, args: &[&str]) -> String {
    let output = run_mock(ctx, args);
    assert!(output.status.success(), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

/// Run cerbo with the real model loader, no `CERBO_MODEL_DIR` unless given,
/// and no way to reach the network.
fn run_offline(ctx: &TestContext, args: &[&str], model_dir: Option<&Path>) -> std::process::Output {
    let mut cmd = ctx.cmd();
    cmd.args(args)
        .env_remove(cerbo_core::semantic::MOCK_EMBEDDER_ENV)
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .env("ALL_PROXY", DEAD_PROXY)
        .env("HTTPS_PROXY", DEAD_PROXY)
        .current_dir(ctx.tmp_dir.path());
    match model_dir {
        Some(dir) => cmd.env(cerbo_core::semantic::model::MODEL_DIR_ENV, dir),
        None => cmd.env_remove(cerbo_core::semantic::model::MODEL_DIR_ENV),
    };
    cmd.output().unwrap()
}

/// `cerbo search`/`similar ... --json` with the mock embedder, parsed.
fn hits_json(ctx: &TestContext, args: &[&str]) -> Vec<serde_json::Value> {
    let mut args = args.to_vec();
    args.push("--json");
    let out = run_mock_ok(ctx, &args);
    serde_json::from_str::<serde_json::Value>(&out).expect("stdout must be only the JSON array")
        .as_array()
        .expect("a JSON array")
        .clone()
}

fn hit_uuids(hits: &[serde_json::Value]) -> Vec<String> {
    hits.iter().map(|h| h["uuid"].as_str().unwrap().to_string()).collect()
}

/// A page id for planted page `n`.
fn planted_uuid(n: usize) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

/// A vault with three pages, embedded with the mock embedder.
fn embedded_vault() -> (TestContext, [String; 3]) {
    let ctx = setup_empty_vault();
    let berlin = planted_uuid(1);
    let munich = planted_uuid(2);
    let bread = planted_uuid(3);
    plant_page(&ctx, &berlin, "Trip to Berlin", "# Trip to Berlin\n\nMuseums and the Spree in Berlin.");
    plant_page(&ctx, &munich, "Weekend in Munich", "# Weekend in Munich\n\nBeer gardens and museums.");
    plant_page(&ctx, &bread, "Sourdough", "# Sourdough\n\nFeed the starter 137 g of flour.");
    let summary: serde_json::Value =
        serde_json::from_str(&run_mock_ok(&ctx, &["index", "--embeddings", "--json"])).unwrap();
    assert_eq!(summary["embedded"], 3, "{summary}");
    (ctx, [berlin, munich, bread])
}

#[test]
fn test_index_embeddings_embeds_only_changed_pages() {
    let (ctx, [berlin, ..]) = embedded_vault();

    let summary: serde_json::Value =
        serde_json::from_str(&run_mock_ok(&ctx, &["index", "--embeddings", "--json"])).unwrap();
    assert_eq!(summary["embedded"], 0, "nothing changed: {summary}");

    std::fs::write(page_md(&ctx, &berlin), "# Trip to Berlin\n\nThe Spree, at night.").unwrap();
    let summary: serde_json::Value =
        serde_json::from_str(&run_mock_ok(&ctx, &["index", "--embeddings", "--json"])).unwrap();
    assert_eq!(summary["embedded"], 1, "exactly the edited page: {summary}");
    assert!(summary["triples"].is_u64(), "{summary}");
    assert!(summary["legacy_backrefs_removed"].is_u64(), "{summary}");

    // Plain output says it too; without --embeddings there is no such field.
    let out = run_mock_ok(&ctx, &["index", "--embeddings"]);
    assert!(out.contains("Embedded 0 pages"), "{out}");
    assert!(index_json(&ctx, &[]).get("embedded").is_none());
}

#[test]
fn test_search_before_embeddings_asks_for_index_embeddings() {
    let ctx = setup_empty_vault();
    plant_page(&ctx, &planted_uuid(1), "Trip", "# Trip\n\nA trip.");
    // The real loader and no network: the model is not even looked for.
    for args in [vec!["search", "trip"], vec!["similar", &planted_uuid(1)]] {
        let output = run_offline(&ctx, &args, None);
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("cerbo index --embeddings"), "{args:?}: {stderr}");
        assert!(!stderr.contains("model"), "no model was needed: {stderr}");
    }
}

#[test]
fn test_search_as_json_and_table() {
    let (ctx, [berlin, ..]) = embedded_vault();
    let hits = hits_json(&ctx, &["search", "Berlin museums"]);
    assert_eq!(hits.len(), 3, "{hits:?}");
    assert_eq!(hits[0]["uuid"], berlin.as_str());
    assert_eq!(hits[0]["title"], "Trip to Berlin");
    for hit in &hits {
        assert_eq!(hit.as_object().unwrap().len(), 3, "only uuid, title and score: {hit}");
        assert!(hit["score"].is_number(), "{hit}");
    }
    let scores: Vec<f64> = hits.iter().map(|h| h["score"].as_f64().unwrap()).collect();
    assert!(scores.windows(2).all(|w| w[0] >= w[1]), "best first: {scores:?}");

    let out = run_mock_ok(&ctx, &["search", "Berlin museums"]);
    let first = out.lines().next().unwrap();
    assert!(first.ends_with(&format!("  {berlin}: Trip to Berlin")), "{out}");
    assert_eq!(out.lines().count(), 3, "{out}");
}

#[test]
fn test_fresh_note_is_found_without_reindexing() {
    let (ctx, _) = embedded_vault();
    let fresh = create_page(&ctx, "Berlin again");
    write_page(&ctx, &fresh, "# Berlin again\n\nBerlin Berlin Berlin.");
    let hits = hits_json(&ctx, &["search", "Berlin"]);
    assert_eq!(hit_uuids(&hits)[0], fresh, "{hits:?}");
}

#[test]
fn test_search_limit_defaults_to_10() {
    let ctx = setup_empty_vault();
    for n in 1..=12 {
        plant_page(&ctx, &planted_uuid(n), &format!("Notes {n}"), "# Notes\n\nnotes");
    }
    run_mock_ok(&ctx, &["index", "--embeddings"]);
    assert_eq!(hits_json(&ctx, &["search", "notes"]).len(), 10);
    assert_eq!(hits_json(&ctx, &["search", "notes", "--limit", "3"]).len(), 3);
    assert_eq!(hits_json(&ctx, &["similar", &planted_uuid(1), "--limit", "3"]).len(), 3);
    assert!(!run_mock(&ctx, &["search", "notes", "--limit", "0"]).status.success());
}

#[test]
fn test_similar_excludes_the_page_and_rejects_unknown_ids() {
    let (ctx, [berlin, munich, _]) = embedded_vault();
    let hits = hits_json(&ctx, &["similar", &berlin]);
    let uuids = hit_uuids(&hits);
    assert!(!uuids.contains(&berlin), "{uuids:?}");
    assert_eq!(uuids.len(), 2);
    assert_eq!(uuids[0], munich, "the other page about museums comes first");

    for unknown in ["00000000-0000-0000-0000-000000000000", "not-a-uuid"] {
        let output = run_mock(&ctx, &["similar", unknown]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Object not found"), "{stderr}");
    }
}

#[test]
fn test_search_filters_by_type_and_annotation() {
    let (ctx, [berlin, munich, _]) = embedded_vault();
    write_page(&ctx, &berlin, "# Trip to Berlin\n\nMuseums in [Berlin]{schema:Place}, a city.");
    write_page(&ctx, &munich, "# Weekend in Munich\n\nA city with beer gardens.");

    let hits = hits_json(&ctx, &["search", "city", "--annotated", "schema:Place"]);
    assert_eq!(hit_uuids(&hits), vec![berlin.clone()], "only pages with a schema:Place annotation");

    assert_eq!(hits_json(&ctx, &["search", "city", "--type", "schema:NoteDigitalDocument"]).len(), 3);
    assert!(hits_json(&ctx, &["search", "city", "--type", "schema:WebPage"]).is_empty());
    let hits = hits_json(&ctx, &["similar", &munich, "--type", "schema:NoteDigitalDocument", "--annotated", "schema:Place"]);
    assert_eq!(hit_uuids(&hits), vec![berlin]);

    let output = run_mock(&ctx, &["search", "city", "--type", "nope:Thing"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid filter"), "{stderr}");
}

#[test]
fn test_filters_do_not_shrink_the_result_count() {
    let ctx = setup_empty_vault();
    // 137 pages match "city"; the 20 with schema:Place say more besides, so
    // they rank below all the others and the filter has to look further.
    let mut places = Vec::new();
    for n in 1..=137 {
        let uuid = planted_uuid(n);
        if n % 7 == 0 && places.len() < 20 {
            plant_page(&ctx, &uuid, &format!("Place {n}"), "[Somewhere]{schema:Place} city far away river");
            places.push(uuid);
        } else {
            plant_page(&ctx, &uuid, "City", "city");
        }
    }
    assert_eq!(places.len(), 19);
    let last = planted_uuid(137);
    std::fs::write(page_md(&ctx, &last), "[Somewhere]{schema:Place} city far away river").unwrap();
    places.push(last);
    run_mock_ok(&ctx, &["index", "--embeddings"]);

    assert!(
        hit_uuids(&hits_json(&ctx, &["search", "city", "--limit", "100"])).iter().all(|u| !places.contains(u)),
        "the 100 closest pages carry no schema:Place"
    );
    let hits = hits_json(&ctx, &["search", "city", "--annotated", "schema:Place", "--limit", "10"]);
    assert_eq!(hits.len(), 10, "{hits:?}");
    assert!(hit_uuids(&hits).iter().all(|u| places.contains(u)), "{hits:?}");
}

#[test]
fn test_similar_pages_are_linked_in_the_index_only() {
    let ctx = setup_empty_vault();
    let a = planted_uuid(1);
    let b = planted_uuid(2);
    plant_page(&ctx, &a, "Berlin", "# Berlin\n\nBerlin museums and the Spree.");
    plant_page(&ctx, &b, "Berlin", "# Berlin\n\nBerlin museums and the Spree river.");
    plant_page(&ctx, &planted_uuid(3), "Sourdough", "# Sourdough\n\nFlour and water.");
    run_mock_ok(&ctx, &["index", "--embeddings"]);

    let out = run_mock_ok(&ctx, &["sparql", "--json", "SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }"]);
    let results: serde_json::Value = serde_json::from_str(&out).unwrap();
    let pairs: std::collections::BTreeSet<(String, String)> = results["results"]["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| (b["a"]["value"].as_str().unwrap().to_string(), b["b"]["value"].as_str().unwrap().to_string()))
        .collect();
    let iri = |u: &str| format!("cerbo://objects/{u}");
    assert_eq!(pairs, [(iri(&a), iri(&b)), (iri(&b), iri(&a))].into_iter().collect(), "{out}");

    for entry in walkdir_files(ctx.tmp_dir.path()) {
        let text = std::fs::read(&entry).unwrap();
        let text = String::from_utf8_lossy(&text);
        assert!(!text.contains("similarTo") && !text.contains("Similarity"), "{}", entry.display());
    }
}

/// Every file under `dir`.
fn walkdir_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { stack.push(path) } else { files.push(path) }
        }
    }
    files
}

#[test]
fn test_model_dir_with_wrong_files_fails_without_a_download() {
    let ctx = setup_empty_vault();
    plant_page(&ctx, &planted_uuid(1), "Trip", "# Trip\n\nA trip.");
    let model_dir = TempDir::new().unwrap();
    for file in cerbo_core::semantic::model::MODEL_FILES {
        let path = model_dir.path().join(file.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "137").unwrap();
    }

    let output = run_offline(&ctx, &["index", "--embeddings"], Some(model_dir.path()));
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("embedding model unavailable"), "{stderr}");
    assert!(stderr.contains("model.onnx") && stderr.contains("checksum mismatch"), "{stderr}");
    assert!(!stderr.contains("download"), "no download is tried: {stderr}");
    assert!(model_dir.path().join("onnx/model.onnx").exists(), "the user's files are left alone");
}

#[test]
fn test_tampered_cached_model_file_is_deleted_and_named() {
    let ctx = setup_empty_vault();
    plant_page(&ctx, &planted_uuid(1), "Trip", "# Trip\n\nA trip.");
    let onnx = ctx
        .xdg_dir
        .path()
        .join("cache/cerbo/models")
        .join(cerbo_core::semantic::model::MODEL_COMMIT)
        .join(cerbo_core::semantic::model::MODEL_ONNX.path);
    std::fs::create_dir_all(onnx.parent().unwrap()).unwrap();
    std::fs::write(&onnx, "137").unwrap();

    let output = run_offline(&ctx, &["index", "--embeddings"], None);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("model.onnx") && stderr.contains("checksum mismatch"), "{stderr}");
    assert!(!onnx.exists(), "the tampered file is deleted");
}

#[test]
fn test_model_download_impossible() {
    let ctx = setup_empty_vault();
    plant_page(&ctx, &planted_uuid(1), "Trip", "# Trip\n\nA trip.");
    let output = run_offline(&ctx, &["index", "--embeddings"], None);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("embedding model unavailable"), "{stderr}");
    assert!(stderr.contains("cannot download"), "{stderr}");
}

/// The real model, provisioned through `CERBO_MODEL_DIR`, with no network.
/// Run with `CERBO_MODEL_DIR=<dir> cargo test -p cerbo -- --ignored real_model`.
fn real_model_vault() -> (TestContext, std::path::PathBuf, String) {
    let model_dir = std::path::PathBuf::from(
        std::env::var_os(cerbo_core::semantic::model::MODEL_DIR_ENV).expect("CERBO_MODEL_DIR is not set"),
    );
    let ctx = setup_empty_vault();
    let berlin = planted_uuid(1);
    plant_page(&ctx, &berlin, "Trip to Berlin", "# Trip to Berlin\n\nMuseums, the Spree and the Reichstag.");
    plant_page(&ctx, &planted_uuid(2), "Sourdough", "# Sourdough\n\nFeed the starter 137 g of flour every morning.");
    plant_page(&ctx, &planted_uuid(3), "Tax return", "# Tax return\n\nReceipts and forms for the accountant.");
    let output = run_offline(&ctx, &["index", "--embeddings", "--json"], Some(&model_dir));
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["embedded"], 3, "{summary}");
    (ctx, model_dir, berlin)
}

fn real_search(ctx: &TestContext, model_dir: &Path, text: &str) -> Vec<String> {
    let output = run_offline(ctx, &["search", text, "--json", "--limit", "1"], Some(model_dir));
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    hit_uuids(serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap().as_array().unwrap())
}

#[test]
#[ignore = "needs the real model in CERBO_MODEL_DIR and ONNX Runtime"]
fn real_model_synonym_and_cross_language_search_offline() {
    let (ctx, model_dir, berlin) = real_model_vault();
    assert_eq!(real_search(&ctx, &model_dir, "vacation in Germany"), vec![berlin.clone()]);
    assert_eq!(real_search(&ctx, &model_dir, "поездка в Берлин"), vec![berlin.clone()]);

    let output = run_offline(&ctx, &["similar", &berlin, "--json"], Some(&model_dir));
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let hits: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!hit_uuids(hits.as_array().unwrap()).contains(&berlin));
}

// ── Scopes, progress and a busy index (review fixes) ─────────────────────────

/// The registry ID of the vault at `root`, from `cerbo vault list --json`.
fn vault_id(ctx: &TestContext, root: &Path) -> String {
    let out = run_ok(ctx, &["vault", "list", "--json"]);
    let vaults: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    let root = root.canonicalize().unwrap();
    vaults
        .iter()
        .find(|v| Path::new(v["path"].as_str().unwrap()).canonicalize().ok().as_deref() == Some(root.as_path()))
        .and_then(|v| v["id"].as_str())
        .unwrap_or_else(|| panic!("{} is not registered: {out}", root.display()))
        .to_string()
}

/// Hold the index of the test vault, as another Cerbo process would.
fn hold_index(ctx: &TestContext) -> cerbo_core::semantic::IndexLock {
    let cache = ctx.xdg_dir.path().join("cache").join("cerbo");
    let location = cerbo_core::semantic::IndexLocation::for_vault(&cache, ctx.tmp_dir.path()).unwrap();
    std::fs::create_dir_all(&location.dir).unwrap();
    cerbo_core::semantic::IndexLock::acquire(&location.lock_path(), std::time::Duration::ZERO).unwrap()
}

#[test]
fn test_index_conflicting_scope_flags() {
    let ctx = setup_empty_vault();
    let uuid = create_page(&ctx, "Page");

    let output = run(&ctx, &["index", "--vault", ctx.tmp_dir.path().to_str().unwrap(), "--page", &uuid]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cannot specify both --vault and --page"), "{stderr}");
    assert!(!ctx.xdg_dir.path().join("cache").join("cerbo").join("index").exists(), "nothing may be indexed");
}

#[test]
fn test_index_vault_by_id() {
    let ctx = setup_empty_vault();
    create_page(&ctx, "Page");
    let id = vault_id(&ctx, ctx.tmp_dir.path());

    let output = ctx.cmd().args(["index", "--vault", &id]).current_dir("/tmp").output().unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Indexed 1 pages"));

    let output = ctx.cmd().args(["index", "--vault", "no-such-vault-137"]).current_dir("/tmp").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("vault not found"));
}

#[test]
fn test_index_without_arguments_indexes_every_vault() {
    let ctx = setup_empty_vault();
    let first = create_page(&ctx, "First");
    let other = TempDir::new().unwrap();
    std::fs::create_dir_all(other.path().join(".cerbo").join("objects")).unwrap();
    std::fs::write(other.path().join(".cerbo").join("ontology-map.json"), r#"{"version":2,"prefixes":{}}"#).unwrap();
    let output = ctx.cmd().args(["page", "create", "Second"]).current_dir(other.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let second = extract_uuid(&output.stdout);
    let neither = TempDir::new().unwrap();

    // From outside any vault: both registered vaults are indexed.
    let output = ctx.cmd().args(["index", "--json"]).current_dir(neither.path()).output().unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["pages_processed"], 2, "{json}");
    for (root, uuid) in [(ctx.tmp_dir.path(), &first), (other.path(), &second)] {
        let output = ctx
            .cmd()
            .args(["sparql", &format!("ASK {{ <cerbo://objects/{uuid}> ?p ?o }}")])
            .current_dir(root)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "true");
    }
    let indexes = std::fs::read_dir(ctx.xdg_dir.path().join("cache").join("cerbo").join("index")).unwrap().count();
    assert_eq!(indexes, 2);
}

#[test]
fn test_index_continues_past_a_failing_vault() {
    let ctx = setup_empty_vault();
    create_page(&ctx, "Held");
    let other = TempDir::new().unwrap();
    std::fs::create_dir_all(other.path().join(".cerbo").join("objects")).unwrap();
    std::fs::write(other.path().join(".cerbo").join("ontology-map.json"), r#"{"version":2,"prefixes":{}}"#).unwrap();
    let output = ctx.cmd().args(["page", "create", "Free"]).current_dir(other.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let free = extract_uuid(&output.stdout);
    let neither = TempDir::new().unwrap();
    // Another Cerbo process holds the first vault's index throughout.
    let held = hold_index(&ctx);

    let output = ctx
        .cmd()
        .args(["index", "--embeddings", "--json"])
        .env(cerbo_core::semantic::MOCK_EMBEDDER_ENV, "1")
        .current_dir(neither.path())
        .output()
        .unwrap();
    drop(held);

    assert!(!output.status.success(), "a failed vault fails the run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&format!("({}): ", ctx.tmp_dir.path().display())), "the failed vault is named: {stderr}");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("the summary is still printed");
    assert_eq!(json["vault_errors"], 1, "{json}");
    assert_eq!(json["embedded"], 1, "the other vault is embedded: {json}");
    let output = ctx
        .cmd()
        .args(["sparql", &format!("ASK {{ <cerbo://objects/{free}> ?p ?o }}")])
        .current_dir(other.path())
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "true", "the other vault is indexed");
}

#[test]
fn test_index_page_outside_its_vault() {
    let ctx = setup_empty_vault();
    let uuid = create_page(&ctx, "Page");
    let neither = TempDir::new().unwrap();

    let output = ctx.cmd().args(["index", "--page", &uuid, "--json"]).current_dir(neither.path()).output().unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["pages_processed"], 1, "{json}");
}

#[test]
fn test_build_progress_goes_to_stderr() {
    let ctx = setup_empty_vault();
    create_page(&ctx, "Page");

    let output = run(&ctx, &["sparql", "ASK { ?s ?p ?o }"]);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "true\n", "stdout must hold only the answer");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Building the semantic index (1 objects)..."), "{stderr}");
    assert!(stderr.contains("Semantic index ready ("), "{stderr}");

    // Built once: the next query reports no build.
    let output = run(&ctx, &["sparql", "ASK { ?s ?p ?o }"]);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Building"));
}

#[test]
fn test_index_busy_exits_non_zero() {
    let ctx = setup_empty_vault();
    let uuid = create_page(&ctx, "Page");
    let _held = hold_index(&ctx);

    let started = std::time::Instant::now();
    let output = run(&ctx, &["sparql", "ASK { ?s ?p ?o }"]);
    assert!(!output.status.success());
    assert!(started.elapsed() >= std::time::Duration::from_secs(5), "gave up before the retry limit");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("index busy"), "{stderr}");
    assert!(output.stdout.is_empty());

    for args in [vec!["page", "backlinks", uuid.as_str()], vec!["index"]] {
        let output = run(&ctx, &args);
        assert!(!output.status.success(), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("index busy"), "{args:?}: {stderr}");
    }
    // `cerbo index` still wrote the files.
    assert!(ctx.config_dir.join("objects").join(&uuid).join("annotations.ttl").exists());
}

#[test]
fn test_cli_waits_for_a_released_index() {
    let ctx = setup_empty_vault();
    create_page(&ctx, "Page");
    let held = hold_index(&ctx);
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1370));
        drop(held);
    });

    let output = run(&ctx, &["sparql", "ASK { ?s ?p ?o }"]);

    release.join().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "true\n");
}
