use std::process::Command;
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
