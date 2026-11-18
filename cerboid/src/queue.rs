//! The per-vault write queue (design D4).
//!
//! Every write of a vault is a job on one queue and runs alone, in the order
//! it was submitted, on a blocking thread. Reads never queue.

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

type Job = Box<dyn FnOnce() + Send>;

/// The queue was closed: the daemon is stopping or the vault was removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Closed;

pub struct WriteQueue {
    tx: std::sync::Mutex<Option<mpsc::UnboundedSender<Job>>>,
    worker: std::sync::Mutex<Option<JoinHandle<()>>>,
}

impl WriteQueue {
    /// Start the queue's worker on the current runtime.
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<Job>();
        let worker = tokio::spawn(async move {
            while let Some(job) = rx.recv().await {
                // A panicking job must not take the queue down with it; its
                // submitter sees the dropped result channel.
                let _ = tokio::task::spawn_blocking(job).await;
            }
        });
        Self { tx: std::sync::Mutex::new(Some(tx)), worker: std::sync::Mutex::new(Some(worker)) }
    }

    /// Put `job` at the end of the queue. The order of `submit` calls is the
    /// order of execution. The receiver yields the job's result; it fails when
    /// the job panicked.
    pub fn submit<T: Send + 'static>(
        &self,
        job: impl FnOnce() -> T + Send + 'static,
    ) -> Result<oneshot::Receiver<T>, Closed> {
        let (done_tx, done_rx) = oneshot::channel();
        let guard = self.tx.lock().unwrap_or_else(|e| e.into_inner());
        let tx = guard.as_ref().ok_or(Closed)?;
        tx.send(Box::new(move || {
            let _ = done_tx.send(job());
        }))
        .map_err(|_| Closed)?;
        Ok(done_rx)
    }

    /// Take no more jobs, finish the ones queued, and return.
    pub async fn close_and_drain(&self) {
        self.tx.lock().unwrap_or_else(|e| e.into_inner()).take();
        let worker = self.worker.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(worker) = worker {
            let _ = worker.await;
        }
    }
}

impl Default for WriteQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[tokio::test]
    async fn jobs_run_one_at_a_time_in_submission_order() {
        let queue = WriteQueue::new();
        let running = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(Mutex::new(Vec::new()));

        let mut results = Vec::new();
        for n in 0..20 {
            let running = Arc::clone(&running);
            let order = Arc::clone(&order);
            results.push(
                queue
                    .submit(move || {
                        assert_eq!(running.fetch_add(1, Ordering::SeqCst), 0, "two writes ran at once");
                        std::thread::sleep(Duration::from_millis(2));
                        order.lock().unwrap().push(n);
                        running.fetch_sub(1, Ordering::SeqCst);
                        n
                    })
                    .unwrap(),
            );
        }
        for (n, result) in results.into_iter().enumerate() {
            assert_eq!(result.await.unwrap(), n);
        }
        assert_eq!(*order.lock().unwrap(), (0..20).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn closing_finishes_queued_jobs_and_refuses_new_ones() {
        let queue = WriteQueue::new();
        let done = Arc::new(AtomicUsize::new(0));
        for _ in 0..5 {
            let done = Arc::clone(&done);
            queue
                .submit(move || {
                    std::thread::sleep(Duration::from_millis(20));
                    done.fetch_add(1, Ordering::SeqCst);
                })
                .unwrap();
        }

        queue.close_and_drain().await;
        assert_eq!(done.load(Ordering::SeqCst), 5, "queued writes must finish before the queue closes");
        assert!(queue.submit(|| ()).is_err());
    }

    #[tokio::test]
    async fn a_panicking_job_does_not_stop_the_queue() {
        let queue = WriteQueue::new();
        let bad = queue.submit(|| -> u32 { panic!("boom") }).unwrap();
        let good = queue.submit(|| 137).unwrap();

        assert!(bad.await.is_err());
        assert_eq!(good.await.unwrap(), 137);
    }
}
