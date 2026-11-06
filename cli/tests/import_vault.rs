//! `cerbo import-vault`: target flags, the interactive prompt, the reports.
//!
//! The import rules themselves (links, triples, staging) are tested in
//! `cerbo_core::vault_import`; these tests drive the binary.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

const NO_TARGET: &str = "no target vault: pass --vault <ID> or --new <PATH>";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/import")
        .join(name)
}

/// A private XDG home and a working directory outside any vault.
struct Env {
    xdg: TempDir,
    work: TempDir,
}

impl Env {
    fn new() -> Self {
        let env = Env {
            xdg: TempDir::new().unwrap(),
            work: TempDir::new().unwrap(),
        };
        // The first run writes the default config files; later snapshots
        // then only see what import-vault itself changes.
        assert!(env.run(&["vault", "list"]).status.success());
        env
    }

    fn cmd(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerbo"));
        cmd.env("XDG_CONFIG_HOME", self.xdg.path().join("config"))
            .env("XDG_CACHE_HOME", self.xdg.path().join("cache"))
            .current_dir(self.work.path())
            .stdin(Stdio::null());
        // `--new` bundles Schema.org and FOAF over the network; a dead proxy
        // makes that fail at once (a warning) instead of reaching out.
        for var in [
            "http_proxy",
            "https_proxy",
            "HTTPS_PROXY",
            "all_proxy",
            "ALL_PROXY",
        ] {
            cmd.env(var, "http://127.0.0.1:9");
        }
        cmd.env_remove("no_proxy").env_remove("NO_PROXY");
        cmd
    }

    fn run(&self, args: &[&str]) -> Output {
        self.cmd().args(args).output().unwrap()
    }

    fn import(&self, source: &str, args: &[&str]) -> Output {
        let dir = fixture(source);
        let mut all = vec!["import-vault", dir.to_str().unwrap()];
        all.extend_from_slice(args);
        self.run(&all)
    }

    /// A vault made by hand (no network-bound `cerbo init`), registered as
    /// `name`. Returns its id and path.
    fn offline_vault(&self, name: &str) -> (String, PathBuf) {
        let root = self.work.path().join(name);
        std::fs::create_dir_all(root.join(".cerbo/objects")).unwrap();
        std::fs::write(
            root.join(".cerbo/ontology-map.json"),
            r#"{"version":2,"prefixes":{}}"#,
        )
        .unwrap();
        let out = self.run(&["vault", "add", name, root.to_str().unwrap(), "--json"]);
        assert!(out.status.success(), "{}", stderr(&out));
        let vault: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        (vault["id"].as_str().unwrap().to_string(), root)
    }

    fn registry(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        snapshot(&self.xdg.path().join("config"))
    }

    /// Titles of every object `cerbo page list` shows; an import adds its
    /// pages plus its Ontology object.
    fn titles(&self, vault_id: &str) -> Vec<String> {
        let out = self.run(&["page", "list", "--vault", vault_id, "--json"]);
        assert!(out.status.success(), "{}", stderr(&out));
        serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout)
            .unwrap()
            .iter()
            .map(|p| p["title"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    /// Objects listed, less the import ontologies.
    fn page_count(&self, vault_id: &str) -> usize {
        self.titles(vault_id)
            .iter()
            .filter(|t| !t.starts_with("Tolaria: ") && !t.starts_with("Obsidian: "))
            .count()
    }
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", stdout(out)))
}

/// A copy of `from` at `to`, so a test may nest things in a source.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let dest = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

/// Every file under `dir` with its content; a missing dir is empty.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.insert(path.clone(), std::fs::read(&path).unwrap_or_default());
            }
        }
    }
    files
}

// ── Usage ─────────────────────────────────────────────────────────────────────

#[test]
fn help_lists_import_vault_next_to_import() {
    let env = Env::new();
    let help = stdout(&env.run(&["--help"]));
    for command in ["import ", "import-ontology", "import-vault"] {
        assert!(help.contains(command), "{command} missing from:\n{help}");
    }
}

#[test]
fn vault_and_new_are_exclusive() {
    let env = Env::new();
    let (id, vault) = env.offline_vault("Main");
    let before = (env.registry(), snapshot(&vault));
    let new = env.work.path().join("fresh");
    let out = env.import("tolaria", &["--vault", &id, "--new", new.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("cannot be used with"),
        "{}",
        stderr(&out)
    );
    assert!(!new.exists());
    assert_eq!((env.registry(), snapshot(&vault)), before);
}

#[test]
fn name_needs_new() {
    let env = Env::new();
    let out = env.import("tolaria", &["--name", "Notes", "--dry-run"]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
}

#[test]
fn unknown_format_is_a_usage_error() {
    let env = Env::new();
    let out = env.import("tolaria", &["--from", "logseq", "--dry-run"]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("expected obsidian or tolaria"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn unknown_vault_id_fails_and_writes_nothing() {
    let env = Env::new();
    let before = env.registry();
    let out = env.import("tolaria", &["--vault", "3f2a"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("Vault not found: 3f2a"),
        "{}",
        stderr(&out)
    );
    assert_eq!(env.registry(), before);
}

#[test]
fn no_terminal_and_no_target_fails_and_writes_nothing() {
    let env = Env::new();
    let (_, vault) = env.offline_vault("Main");
    let before = (env.registry(), snapshot(&vault));
    let out = env.import("tolaria", &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains(NO_TARGET), "{}", stderr(&out));
    assert!(
        !stderr(&out).contains("Import into which vault"),
        "no prompt without a terminal"
    );
    assert_eq!((env.registry(), snapshot(&vault)), before);
}

#[test]
fn failures_inside_an_unregistered_vault_register_nothing() {
    let env = Env::new();
    let here = env.work.path().join("here");
    std::fs::create_dir_all(here.join(".cerbo/objects")).unwrap();
    let before = env.registry();
    let dir = fixture("tolaria");
    for extra in [&["--vault", "bogus"][..], &[][..]] {
        let out = env
            .cmd()
            .current_dir(&here)
            .arg("import-vault")
            .arg(&dir)
            .args(extra)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
        assert_eq!(
            env.registry(),
            before,
            "{extra:?} registered the current vault"
        );
    }
}

#[test]
fn target_inside_the_source_is_refused() {
    let env = Env::new();
    let src = env.work.path().join("src");
    copy_tree(&fixture("obsidian"), &src);
    // A registered vault that lives inside the source.
    std::fs::create_dir_all(src.join("vault/.cerbo/objects")).unwrap();
    std::fs::write(
        src.join("vault/.cerbo/ontology-map.json"),
        r#"{"version":2,"prefixes":{}}"#,
    )
    .unwrap();
    let out = env.run(&[
        "vault",
        "add",
        "Inside",
        src.join("vault").to_str().unwrap(),
        "--json",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    let id = json(&out)["id"].as_str().unwrap().to_string();
    let before = (env.registry(), snapshot(&src));

    let inner = src.join("inner");
    for args in [vec!["--new", inner.to_str().unwrap()], vec!["--vault", &id]] {
        let mut all = vec!["import-vault", src.to_str().unwrap()];
        all.extend(args.iter().copied());
        let out = env.run(&all);
        assert_eq!(out.status.code(), Some(1), "{args:?}: {}", stdout(&out));
        assert!(
            stderr(&out).contains("inside the source folder"),
            "{}",
            stderr(&out)
        );
        assert!(!inner.exists());
        assert_eq!(
            (env.registry(), snapshot(&src)),
            before,
            "{args:?} wrote something"
        );
    }
}

#[test]
fn json_errors_are_json() {
    let env = Env::new();
    let out = env.import("tolaria", &["--json"]);
    assert_eq!(out.status.code(), Some(1));
    let err = json(&out);
    assert_eq!(err["error"], true);
    assert_eq!(err["message"], NO_TARGET);
}

#[test]
fn undetectable_folder_is_refused() {
    let env = Env::new();
    let empty = TempDir::new().unwrap();
    let out = env.run(&["import-vault", empty.path().to_str().unwrap(), "--dry-run"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("cannot detect vault format, pass --from obsidian|tolaria"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn new_vault_in_a_non_empty_folder_is_refused() {
    let env = Env::new();
    let target = env.work.path().join("busy");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("notes.txt"), "mine").unwrap();
    let before = env.registry();
    let out = env.import("tolaria", &["--new", target.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("not empty"), "{}", stderr(&out));
    assert!(!target.join(".cerbo").exists());
    assert_eq!(env.registry(), before);
}

// ── Dry run ───────────────────────────────────────────────────────────────────

#[test]
fn dry_run_json_reports_and_writes_nothing() {
    let env = Env::new();
    let (_, vault) = env.offline_vault("Main");
    let before = (snapshot(env.xdg.path()), snapshot(&vault));

    let out = env.import("tolaria", &["--dry-run", "--json"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stderr(&out).contains("Import into which vault"));
    let report = json(&out);
    assert_eq!(report["dryRun"], true);
    assert_eq!(report["vault"], serde_json::Value::Null);
    assert_eq!(report["format"], "tolaria");
    assert_eq!(report["pages"], 10);
    assert_eq!(report["attachments"], 2);
    assert_eq!(report["skipped"], serde_json::json!(["views/devices.yml"]));
    assert_eq!(
        report["unreferenced"],
        serde_json::json!(["attachments/firmware.zip"])
    );
    assert_eq!(
        report["unresolved"].as_array().unwrap().len(),
        2,
        "{report}"
    );
    for field in ["relations", "ambiguous", "renamedSlugs"] {
        assert!(report.get(field).is_some(), "missing {field}");
    }

    assert_eq!((snapshot(env.xdg.path()), snapshot(&vault)), before);
}

#[test]
fn dry_run_human_report_lists_what_needs_attention() {
    let env = Env::new();
    let out = env.import("obsidian", &["--dry-run"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("Dry run: would import 8 pages and 1 attachment"),
        "{text}"
    );
    assert!(text.contains("Obsidian, .obsidian/ folder"), "{text}");
    assert!(text.contains("Skipped (1):\n  plan.canvas"), "{text}");
    assert!(
        text.contains("Unreferenced files, not imported (1):\n  attachments/unused.pdf"),
        "{text}"
    );
    assert!(
        text.contains("Unresolved links (1):\n  Welcome.md: [[Missing Note]]"),
        "{text}"
    );
    assert!(text.contains("Ambiguous links (2):"), "{text}");
    assert!(
        text.contains("notes/slug-test.md: slug-test -> slug-test-2"),
        "{text}"
    );
}

#[test]
fn dry_run_from_overrides_detection() {
    let env = Env::new();
    let out = env.import("obsidian", &["--from", "Tolaria", "--dry-run", "--json"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let report = json(&out);
    assert_eq!(report["format"], "tolaria");
    assert_eq!(report["formatReason"], "--from");
}

// ── Import ────────────────────────────────────────────────────────────────────

#[test]
fn tolaria_import_into_a_registered_vault() {
    let env = Env::new();
    let (id, _) = env.offline_vault("Main");

    let out = env.import("tolaria", &["--vault", &id, "--json"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let report = json(&out);
    assert_eq!(report["dryRun"], false);
    assert_eq!(report["vault"]["id"], id.as_str());
    assert_eq!(report["vault"]["name"], "Main");
    assert_eq!(report["pages"], 10);
    assert_eq!(report["attachments"], 2);
    assert_eq!(report["relations"], 4);
    assert_eq!(report["ontology"]["prefix"], "tolaria");
    let unresolved: Vec<String> = report["unresolved"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| {
            format!(
                "{}: {}",
                u["note"].as_str().unwrap(),
                u["link"].as_str().unwrap()
            )
        })
        .collect();
    assert!(
        unresolved.contains(&"3d-printing.md: [[Voron Config]]".to_string()),
        "{unresolved:?}"
    );
    assert_eq!(env.page_count(&id), 10);

    // A second run is refused and names the first import's date.
    let again = env.import("tolaria", &["--vault", &id]);
    assert_eq!(again.status.code(), Some(1));
    assert!(
        stderr(&again).contains("already imported into this vault on"),
        "{}",
        stderr(&again)
    );
    assert_eq!(env.page_count(&id), 10);

    // --force makes a second, separate copy with its own prefix.
    let forced = env.import("tolaria", &["--vault", &id, "--force", "--json"]);
    assert!(forced.status.success(), "{}", stderr(&forced));
    assert_eq!(json(&forced)["ontology"]["prefix"], "tolaria-2");
    assert_eq!(env.page_count(&id), 20);
}

#[test]
fn obsidian_import_prints_the_human_report() {
    let env = Env::new();
    let (id, _) = env.offline_vault("Main");

    let out = env.import("obsidian", &["--vault", &id]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(&format!(
            "Imported 8 pages and 1 attachment (70 B) into \"Main\" ({})",
            id
        )),
        "{text}"
    );
    assert!(
        text.contains("Unresolved links (1):\n  Welcome.md: [[Missing Note]]"),
        "{text}"
    );
    assert_eq!(env.page_count(&id), 8);
}

#[test]
fn new_creates_registers_and_imports() {
    let env = Env::new();
    let target = env.work.path().join("cerbo/notes");

    let out = env.import(
        "tolaria",
        &[
            "--new",
            target.to_str().unwrap(),
            "--name",
            "Notes",
            "--json",
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let report = json(&out);
    assert_eq!(report["vault"]["name"], "Notes");
    assert!(target.join(".cerbo/objects").is_dir());
    assert!(
        std::fs::read_to_string(target.join(".gitignore"))
            .unwrap()
            .contains("/.cerbo/trash/")
    );

    let vaults: Vec<serde_json::Value> =
        serde_json::from_slice(&env.run(&["vault", "list", "--json"]).stdout).unwrap();
    let notes = vaults
        .iter()
        .find(|v| v["name"] == "Notes")
        .expect("vault registered");
    assert_eq!(notes["id"], report["vault"]["id"]);
    let titles = env.titles(notes["id"].as_str().unwrap());
    assert!(
        titles.contains(&"Tolaria: tolaria".to_string()),
        "{titles:?}"
    );
    assert!(titles.contains(&"Devices".to_string()), "{titles:?}");
}

#[test]
fn new_is_not_created_when_planning_fails() {
    let env = Env::new();
    let empty = TempDir::new().unwrap();
    let target = env.work.path().join("never");
    let before = env.registry();
    let out = env.run(&[
        "import-vault",
        empty.path().to_str().unwrap(),
        "--new",
        target.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        !target.exists(),
        "no vault is created for a source that cannot be planned"
    );
    assert_eq!(env.registry(), before);
}

// ── Interactive prompt ────────────────────────────────────────────────────────

/// A pseudo-terminal: (master, slave). `None` where the system has none.
#[cfg(unix)]
fn open_pty() -> Option<(std::fs::File, std::fs::File)> {
    use std::os::fd::FromRawFd;
    use std::os::unix::fs::OpenOptionsExt;

    // SAFETY: plain libc calls on a descriptor this function owns; `buf` is
    // large enough for any pts path and ptsname_r NUL-terminates it.
    unsafe {
        let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
        if master < 0 {
            return None;
        }
        let master_file = std::fs::File::from_raw_fd(master);
        if libc::grantpt(master) != 0 || libc::unlockpt(master) != 0 {
            return None;
        }
        let mut buf = [0 as libc::c_char; 128];
        if libc::ptsname_r(master, buf.as_mut_ptr(), buf.len()) != 0 {
            return None;
        }
        let name = std::ffi::CStr::from_ptr(buf.as_ptr())
            .to_str()
            .ok()?
            .to_string();
        let slave = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOCTTY)
            .open(name)
            .ok()?;
        Some((master_file, slave))
    }
}

#[cfg(unix)]
#[test]
fn prompt_on_a_terminal_imports_into_the_chosen_vault() {
    use std::io::Write;

    let env = Env::new();
    let (first, _) = env.offline_vault("First");
    let (second, _) = env.offline_vault("Second");
    let Some((mut master, slave)) = open_pty() else {
        eprintln!("skipped: no pseudo-terminal available");
        return;
    };

    let dir = fixture("tolaria");
    let child = env
        .cmd()
        .args(["import-vault", dir.to_str().unwrap()])
        .stdin(Stdio::from(slave))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The terminal buffers the line until the prompt reads it.
    master.write_all(b"2\n").unwrap();
    let out = child.wait_with_output().unwrap();
    drop(master);

    assert!(out.status.success(), "{}", stderr(&out));
    let prompt = stderr(&out);
    assert!(prompt.contains("Import into which vault?"), "{prompt}");
    assert!(prompt.contains("  1. First ("), "{prompt}");
    assert!(prompt.contains("  2. Second ("), "{prompt}");
    assert!(prompt.contains("  3. Create a new vault"), "{prompt}");
    assert!(
        stdout(&out).contains("Imported 10 pages"),
        "{}",
        stdout(&out)
    );
    assert_eq!(env.page_count(&second), 10);
    assert_eq!(env.page_count(&first), 0);
}
