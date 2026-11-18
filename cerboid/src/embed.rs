//! One thread that owns the embedding model.
//!
//! The model is not `Send` and slow to load, so the daemon loads it once, on
//! first use, in a thread of its own; semantic searches borrow it from there.

use cerbo_core::semantic::LazyEmbedder;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// Builds the embedder inside the worker thread. The default is the real
/// model; tests pass a mock.
pub type EmbedderFactory = Arc<dyn Fn(&Path) -> LazyEmbedder + Send + Sync>;

/// The real model, from the cache directory.
pub fn default_factory() -> EmbedderFactory {
    Arc::new(cerbo_core::semantic::default_embedder)
}

type Job = Box<dyn FnOnce(&mut LazyEmbedder) + Send>;

pub struct EmbedWorker {
    tx: Mutex<Option<mpsc::Sender<Job>>>,
    factory: EmbedderFactory,
    cache_dir: PathBuf,
}

impl EmbedWorker {
    /// The thread starts on first use.
    pub fn new(factory: EmbedderFactory, cache_dir: PathBuf) -> Self {
        Self { tx: Mutex::new(None), factory, cache_dir }
    }

    /// Run `f` with the embedder and return its result. Blocks until done.
    pub fn run<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut LazyEmbedder) -> T + Send + 'static,
    ) -> Result<T, String> {
        let (result_tx, result_rx) = mpsc::channel();
        let job: Job = Box::new(move |embedder| {
            let _ = result_tx.send(f(embedder));
        });
        {
            let mut tx = self.tx.lock().unwrap_or_else(|e| e.into_inner());
            let sender = tx.get_or_insert_with(|| self.spawn());
            sender.send(job).map_err(|_| "the embedding worker stopped".to_string())?;
        }
        result_rx.recv().map_err(|_| "the embedding worker failed".to_string())
    }

    fn spawn(&self) -> mpsc::Sender<Job> {
        let (tx, rx) = mpsc::channel::<Job>();
        let factory = Arc::clone(&self.factory);
        let cache_dir = self.cache_dir.clone();
        std::thread::spawn(move || {
            let mut embedder = factory(&cache_dir);
            while let Ok(job) = rx.recv() {
                job(&mut embedder);
            }
        });
        tx
    }
}
