use cerbo_core::context::CoreContext;
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::CerboContext;
use cerboid::{Config, ShutdownCtl, Started};
use tokio::signal::unix::{signal, SignalKind};

fn main() {
    if let Err(e) = real_main() {
        eprintln!("cerboid: {e}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => {}
        Some("--version" | "-V") => {
            println!("cerboid {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some("--help" | "-h") => {
            println!(
                "cerboid {}\n\nThe Cerbo daemon: one process per user that owns every registered vault.\n\
                 Usage: cerboid\n\nIt listens on $XDG_RUNTIME_DIR/cerbo/cerboid.sock. A second instance exits at once.",
                env!("CARGO_PKG_VERSION")
            );
            return Ok(());
        }
        Some(other) => return Err(format!("unknown argument: {other} (try --help)")),
    }

    let ctx: CerboContext = CoreContext::new()?.into();
    let paths = DaemonPaths::for_user(&ctx.cache_dir);
    let config = Config::new(ctx, paths.clone());

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string())?;
    runtime.block_on(async move {
        let ctl = ShutdownCtl::new();
        let on_signal = ctl.clone();
        let mut term = signal(SignalKind::terminate()).map_err(|e| e.to_string())?;
        let mut int = signal(SignalKind::interrupt()).map_err(|e| e.to_string())?;
        tokio::spawn(async move {
            tokio::select! {
                _ = term.recv() => {}
                _ = int.recv() => {}
            }
            on_signal.trigger();
        });

        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let result = cerboid::run(config, ctl, Some(started_tx)).await;
        if let Ok(Ok(Started::AlreadyRunning)) = started_rx.try_recv() {
            eprintln!("cerboid is already running ({})", paths.socket.display());
        }
        result
    })
}
