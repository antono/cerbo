//! The agent sandbox with a real `bwrap` (design D8). Skipped, with a message,
//! when `CERBO_BWRAP` is not set or a test sandbox cannot start here (for
//! example user namespaces are disabled, as in the Nix build sandbox).
//!
//! The "agent" is this very test binary, started inside the sandbox with the
//! argument `acp` (the arguments of OpenCode's ACP mode), which libtest takes
//! as a name filter: only [`acp_sandbox_child`] runs there. It reports what it
//! sees in a file of the session socket directory; the parent asserts on it.
//! No test name other than the child's may contain "acp".

use cerbo_core::rpc::AgentKind;
use cerboid::agents::HostEnv;
use cerboid::sandbox::{self, Confinement, RealFs, SessionSpec};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

const REPORT: &str = "report.txt";
const EXPECT: &str = "expect.txt";

/// Runs inside the sandbox only (when `CERBO_SOCKET` is set by the daemon's plan).
#[test]
fn acp_sandbox_child() {
    let Some(socket) = std::env::var_os("CERBO_SOCKET") else { return };
    let dir = Path::new(&socket).parent().unwrap().to_path_buf();
    let expect = std::fs::read_to_string(dir.join(EXPECT)).unwrap();
    let mut lines = expect.lines();
    let real_home = lines.next().unwrap_or("").to_string();
    let runtime_socket = lines.next().unwrap_or("").to_string();
    let main_socket = lines.next().unwrap_or("").to_string();

    let mut report = String::new();
    let mut say = |k: &str, v: String| report.push_str(&format!("{k}={v}\n"));

    let cwd = std::env::current_dir().unwrap();
    say("cwd", cwd.display().to_string());
    say("vault_note_readable", std::fs::read_to_string(cwd.join("note.md")).is_ok().to_string());
    let write = std::fs::write(cwd.join("new.md"), "x");
    say("vault_write_errno", write.err().and_then(|e| e.raw_os_error()).map_or("none".into(), |n| n.to_string()));
    // Directories that lead to a bound path exist (empty) even under a hidden
    // /home; anything else in the real home would be a leak.
    let exe = std::env::current_exe().unwrap();
    let leaks: Vec<String> = std::fs::read_dir(&real_home)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| !exe.starts_with(p) && !cwd.starts_with(p)).map(|p| p.display().to_string()).collect())
        .unwrap_or_default();
    say("real_home_leaks", leaks.join(","));
    say("runtime_socket_visible", (!runtime_socket.is_empty() && Path::new(&runtime_socket).exists()).to_string());
    say("main_socket_visible", Path::new(&main_socket).exists().to_string());
    say("tmp_is_private", (!Path::new("/tmp/outside-marker-137").exists()).to_string());

    let home = std::env::var("HOME").unwrap_or_default();
    say("home", home.clone());
    say("home_writable", std::fs::write(Path::new(&home).join("probe"), "x").is_ok().to_string());
    let login = Path::new(&home).join(".local/share/opencode/auth.json");
    say("login_content", std::fs::read_to_string(&login).unwrap_or_default().trim().to_string());
    say("login_writable", std::fs::OpenOptions::new().write(true).open(&login).is_ok().to_string());

    let mut names: Vec<String> = std::env::vars().map(|(k, _)| k).collect();
    names.sort();
    say("env", names.join(","));

    let greeting = UnixStream::connect(&socket).and_then(|mut s| {
        let mut text = String::new();
        s.read_to_string(&mut text)?;
        Ok(text)
    });
    say("socket", greeting.unwrap_or_else(|e| format!("error: {e}")).trim().to_string());

    std::fs::write(dir.join(REPORT), report).unwrap();
}

fn bwrap_or_skip() -> Option<PathBuf> {
    let Some(path) = std::env::var_os("CERBO_BWRAP").filter(|v| !v.is_empty()) else {
        eprintln!(
            "SKIPPED: CERBO_BWRAP is not set (the devshell sets it; or export the bin/bwrap of \
             `nix build nixpkgs#bubblewrap --print-out-paths`)"
        );
        return None;
    };
    match sandbox::confinement(cerbo_core::agent::SandboxMode::Bwrap, Some(path)) {
        Ok(Confinement::Bwrap(path)) => Some(path),
        Ok(Confinement::None) => unreachable!(),
        Err(e) => {
            eprintln!("SKIPPED: bwrap cannot sandbox here: {e}");
            None
        }
    }
}

#[test]
fn the_agent_sees_a_read_only_vault_a_private_world_and_only_its_session_socket() {
    let Some(bwrap) = bwrap_or_skip() else { return };

    let tmp = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(tmp.path()).unwrap();
    let vault = base.join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::write(vault.join("note.md"), "# note").unwrap();

    // The daemon's main socket, which must stay invisible, and the user's real one if any.
    let main_dir = base.join("run/cerbo");
    std::fs::create_dir_all(&main_dir).unwrap();
    let main_socket = main_dir.join("cerboid.sock");
    let _main = UnixListener::bind(&main_socket).unwrap();
    let runtime_socket = std::env::var_os("XDG_RUNTIME_DIR")
        .map(|d| Path::new(&d).join("cerbo/cerboid.sock").display().to_string())
        .unwrap_or_default();
    std::fs::write("/tmp/outside-marker-137", "x").unwrap();

    // The session socket directory with a listener that greets once.
    let socket_dir = base.join("sock");
    std::fs::create_dir_all(&socket_dir).unwrap();
    let session_listener = UnixListener::bind(socket_dir.join(sandbox::SESSION_SOCKET_NAME)).unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = session_listener.accept().unwrap();
        stream.write_all(b"hello-137\n").unwrap();
    });

    let real_home = std::env::var("HOME").unwrap_or_default();
    std::fs::write(socket_dir.join(EXPECT), format!("{real_home}\n{runtime_socket}\n{}\n", main_socket.display())).unwrap();

    let login = base.join("real-auth.json");
    std::fs::write(&login, "{\"token\":\"137\"}").unwrap();
    let cerbo_dir = base.join("cerbo-pkg/bin");
    std::fs::create_dir_all(&cerbo_dir).unwrap();
    std::fs::write(cerbo_dir.join("cerbo"), "#!/bin/sh\n").unwrap();

    let env = HostEnv::from_process();
    let agent_home = sandbox::prepare_home(&base.join("agents"), AgentKind::Opencode).unwrap();
    let spec = SessionSpec {
        kind: AgentKind::Opencode,
        agent_binary: std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap(),
        vault: vault.clone(),
        socket_dir: socket_dir.clone(),
        agent_home,
        cerbo_bin: cerbo_dir.join("cerbo"),
        login_file: Some(login.clone()),
        real_home: env.home.clone(),
        host_path: env.path.clone(),
        protected: Vec::new(),
    };
    let launch = sandbox::plan(&Confinement::Bwrap(bwrap), &spec, &RealFs).unwrap();
    assert!(launch.sandboxed);
    let output = launch
        .command()
        // Whatever the daemon's own environment holds must not reach the agent.
        .env("LEAK_137", "secret")
        .output()
        .unwrap();
    server.join().unwrap();
    std::fs::remove_file("/tmp/outside-marker-137").ok();
    assert!(
        output.status.success(),
        "the sandboxed child failed: {}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );

    let report = std::fs::read_to_string(socket_dir.join(REPORT)).expect("the child wrote no report");
    let get = |k: &str| {
        report
            .lines()
            .find_map(|l| l.strip_prefix(&format!("{k}=")))
            .unwrap_or_else(|| panic!("no {k} in {report}"))
            .to_string()
    };

    assert_eq!(get("cwd"), vault.display().to_string(), "the vault is the working directory at its real path");
    assert_eq!(get("vault_note_readable"), "true");
    assert_eq!(get("vault_write_errno"), "30", "writing into the vault is EROFS: {report}");
    assert!(!vault.join("new.md").exists());
    assert_eq!(get("real_home_leaks"), "", "nothing of the real home is visible: {report}");
    assert_eq!(get("runtime_socket_visible"), "false", "{report}");
    assert_eq!(get("main_socket_visible"), "false", "{report}");
    assert_eq!(get("tmp_is_private"), "true");
    assert_eq!(get("socket"), "hello-137", "the session socket is connectable: {report}");
    assert_eq!(get("home"), "/home/agent");
    assert_eq!(get("home_writable"), "true");
    assert_eq!(get("login_content"), "{\"token\":\"137\"}");
    assert_eq!(get("login_writable"), "true");
    let names = get("env");
    assert!(!names.contains("LEAK_137"), "the environment is cleared: {names}");
    assert!(
        names.split(',').all(|n| {
            matches!(n, "PATH" | "HOME" | "LANG" | "PWD" | "SHLVL" | "_" | "CERBO_SOCKET" | "CERBO_BIN")
                || n.starts_with("XDG_")
                || n.starts_with("OPENCODE_")
        }),
        "unexpected variables: {names}"
    );
}

#[test]
fn a_missing_bwrap_refuses_the_session_with_the_reason() {
    use cerbo_core::agent::SandboxMode;
    let err = sandbox::confinement(SandboxMode::Bwrap, Some("/nonexistent/bwrap".into())).unwrap_err();
    assert!(err.contains("CERBO_BWRAP"), "{err}");
}
