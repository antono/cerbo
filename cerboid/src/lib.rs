//! `cerboid`, the Cerbo daemon: one process per user that owns every
//! registered vault. The desktop app, the `cerbo` CLI and agents are its
//! clients; they talk to it over a Unix socket with the protocol of
//! [`cerbo_core::rpc`].
//!
//! The logic lives in `cerbo-core`; this crate hosts it: the socket and
//! single-instance lock ([`instance`]), the vault registry ([`registry`]), a
//! write queue per vault ([`queue`]), the read and write endpoints
//! ([`ops`]), the file watcher that feeds change events ([`watch`]) and the
//! server that ties them together ([`server`]).

pub mod acp;
pub mod agents;
pub mod approvals;
pub mod embed;
pub mod instance;
pub mod ops;
pub mod queue;
pub mod registry;
pub mod sandbox;
pub mod server;
pub mod sessions;
pub mod watch;

pub use server::{run, Config, ShutdownCtl, Started};

use std::thread::JoinHandle;

/// A daemon running on a thread of this process, with a runtime of its own.
/// For tests and for embedding; the `cerboid` binary calls [`run`] directly.
pub struct Running {
    ctl: ShutdownCtl,
    thread: Option<JoinHandle<Result<(), String>>>,
}

impl Running {
    /// Ask the daemon to stop; queued writes finish first. Does not wait.
    pub fn request_stop(&self) {
        self.ctl.trigger();
    }

    /// Wait until the daemon has stopped.
    pub fn join(mut self) -> Result<(), String> {
        match self.thread.take() {
            Some(thread) => thread.join().map_err(|_| "the daemon thread panicked".to_string())?,
            None => Ok(()),
        }
    }

    /// [`Self::request_stop`], then [`Self::join`].
    pub fn stop(self) -> Result<(), String> {
        self.request_stop();
        self.join()
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.ctl.trigger();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Start a daemon on a new thread and return once it accepts connections.
/// `Ok(None)` when another daemon of this user already runs.
pub fn spawn(config: Config) -> Result<Option<Running>, String> {
    let ctl = ShutdownCtl::new();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let thread_ctl = ctl.clone();
    let thread = std::thread::Builder::new()
        .name("cerboid".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("cannot start the runtime: {e}"))?;
            runtime.block_on(run(config, thread_ctl, Some(started_tx)))
        })
        .map_err(|e| e.to_string())?;

    match started_rx.recv() {
        Ok(Ok(Started::Running)) => Ok(Some(Running { ctl, thread: Some(thread) })),
        Ok(Ok(Started::AlreadyRunning)) => {
            let _ = thread.join();
            Ok(None)
        }
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        // The thread ended without saying anything: its result says why.
        Err(_) => match thread.join() {
            Ok(Err(e)) => Err(e),
            _ => Err("the daemon ended during start-up".to_string()),
        },
    }
}
