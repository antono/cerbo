//! The approval queue (design D7): agent changes waiting for a person in
//! `approve` mode.
//!
//! A request lives here, not in any client, so it survives until someone
//! answers, including a client that connects later (it lists the queue). One
//! that nobody answers in time ends as rejected, so a headless job finishes
//! instead of hanging.

use cerbo_core::rpc::{ApprovalInfo, ApprovalOutcome, Event, RpcError};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, oneshot};

/// What the requester supplies; the queue adds the id and the times.
pub struct Request {
    pub vault: String,
    pub vault_name: String,
    pub agent: String,
    pub action: String,
    pub page: Option<String>,
    pub summary: String,
    pub diff: Option<String>,
    /// The agent session the request belongs to, when it comes from one.
    pub session: Option<String>,
    /// For a request to run an agent's own tool: its title and kind.
    pub tool_title: Option<String>,
    pub tool_kind: Option<String>,
}

struct Pending {
    info: ApprovalInfo,
    tx: oneshot::Sender<ApprovalOutcome>,
}

/// Told, synchronously, of every request and every end of a request, with the
/// session the request belongs to (if any). The session manager uses it to
/// write approval cards into a session's feed without ever missing one.
pub type Observer = Box<dyn Fn(&Event, Option<&str>) + Send + Sync>;

pub struct Approvals {
    next: AtomicU64,
    pending: Mutex<BTreeMap<u64, Pending>>,
    events: broadcast::Sender<Event>,
    observer: OnceLock<Observer>,
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

impl Approvals {
    pub fn new(events: broadcast::Sender<Event>) -> Self {
        Self { next: AtomicU64::new(1), pending: Mutex::new(BTreeMap::new()), events, observer: OnceLock::new() }
    }

    /// Set the observer; only the first call counts.
    pub fn set_observer(&self, observer: Observer) {
        let _ = self.observer.set(observer);
    }

    fn announce(&self, event: Event, session: Option<&str>) {
        if let Some(observer) = self.observer.get() {
            observer(&event, session);
        }
        let _ = self.events.send(event);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<u64, Pending>> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Queue a request and tell subscribers. Await the outcome with
    /// [`Self::wait`].
    pub fn request(&self, request: Request, timeout: Duration) -> (u64, oneshot::Receiver<ApprovalOutcome>) {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let requested = now_secs();
        let info = ApprovalInfo {
            id,
            vault: request.vault,
            vault_name: request.vault_name,
            agent: request.agent,
            action: request.action,
            session: request.session,
            tool_title: request.tool_title,
            tool_kind: request.tool_kind,
            page: request.page,
            summary: request.summary,
            diff: request.diff,
            requested,
            deadline: requested + timeout.as_secs(),
        };
        let (tx, rx) = oneshot::channel();
        self.lock().insert(id, Pending { info: info.clone(), tx });
        let session = info.session.clone();
        self.announce(Event::ApprovalRequested { approval: info }, session.as_deref());
        (id, rx)
    }

    /// Wait for the outcome of request `id`; at `timeout` it ends as timed out.
    pub async fn wait(
        &self,
        id: u64,
        mut rx: oneshot::Receiver<ApprovalOutcome>,
        timeout: Duration,
    ) -> ApprovalOutcome {
        tokio::select! {
            outcome = &mut rx => outcome.unwrap_or(ApprovalOutcome::Cancelled),
            () = tokio::time::sleep(timeout) => {
                // An answer may have won the race; then the receiver holds it.
                self.resolve(id, ApprovalOutcome::TimedOut);
                rx.await.unwrap_or(ApprovalOutcome::Cancelled)
            }
        }
    }

    /// End request `id` with `outcome`. False when it was already over.
    fn resolve(&self, id: u64, outcome: ApprovalOutcome) -> bool {
        let Some(pending) = self.lock().remove(&id) else { return false };
        self.announce(
            Event::ApprovalResolved { vault: pending.info.vault.clone(), approval: id, outcome },
            pending.info.session.as_deref(),
        );
        let _ = pending.tx.send(outcome);
        true
    }

    /// End request `id` as cancelled: its agent went away, so it must never
    /// be applied. False when it was already over.
    pub fn cancel(&self, id: u64) -> bool {
        self.resolve(id, ApprovalOutcome::Cancelled)
    }

    /// A person's answer.
    pub fn answer(&self, id: u64, approve: bool) -> Result<(), RpcError> {
        let outcome = if approve { ApprovalOutcome::Approved } else { ApprovalOutcome::Rejected };
        if self.resolve(id, outcome) {
            Ok(())
        } else {
            Err(RpcError::failed(format!("no pending approval {id}: it was answered, timed out or never existed")))
        }
    }

    /// The queue, oldest first, for one vault id or all.
    pub fn list(&self, vault: Option<&str>) -> Vec<ApprovalInfo> {
        self.lock().values().map(|p| p.info.clone()).filter(|i| vault.is_none_or(|v| i.vault == v)).collect()
    }

    /// End every request as cancelled (the daemon is stopping).
    pub fn cancel_all(&self) {
        let ids: Vec<u64> = self.lock().keys().copied().collect();
        for id in ids {
            self.resolve(id, ApprovalOutcome::Cancelled);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn sample(vault: &str) -> Request {
        Request {
            vault: vault.into(),
            vault_name: "Notes".into(),
            agent: "opencode".into(),
            action: "edit".into(),
            page: Some("p".into()),
            summary: "Edit".into(),
            diff: None,
            session: None,
            tool_title: None,
            tool_kind: None,
        }
    }

    fn queue() -> (Arc<Approvals>, broadcast::Receiver<Event>) {
        let (tx, rx) = broadcast::channel(16);
        (Arc::new(Approvals::new(tx)), rx)
    }

    #[tokio::test]
    async fn an_approval_reaches_the_waiter_and_leaves_the_queue() {
        let (queue, mut events) = queue();
        let (id, rx) = queue.request(sample("a"), Duration::from_secs(60));
        assert_eq!(queue.list(None).len(), 1);
        assert!(matches!(events.recv().await.unwrap(), Event::ApprovalRequested { .. }));

        queue.answer(id, true).unwrap();
        assert_eq!(queue.wait(id, rx, Duration::from_secs(60)).await, ApprovalOutcome::Approved);
        assert!(queue.list(None).is_empty());
        assert!(matches!(
            events.recv().await.unwrap(),
            Event::ApprovalResolved { outcome: ApprovalOutcome::Approved, .. }
        ));
        assert!(queue.answer(id, true).is_err(), "answering twice is an error");
    }

    #[tokio::test]
    async fn a_cancelled_request_leaves_the_queue_and_cannot_be_approved() {
        let (queue, mut events) = queue();
        let (id, rx) = queue.request(sample("a"), Duration::from_secs(60));
        assert!(matches!(events.recv().await.unwrap(), Event::ApprovalRequested { .. }));
        assert!(queue.cancel(id));
        assert!(queue.list(None).is_empty());
        assert!(queue.answer(id, true).is_err(), "a cancelled request cannot be approved");
        assert!(!queue.cancel(id));
        assert_eq!(rx.await.unwrap(), ApprovalOutcome::Cancelled);
        assert!(matches!(
            events.recv().await.unwrap(),
            Event::ApprovalResolved { outcome: ApprovalOutcome::Cancelled, .. }
        ));
    }

    #[tokio::test]
    async fn no_answer_in_time_is_a_timeout() {
        let (queue, _events) = queue();
        let (id, rx) = queue.request(sample("a"), Duration::from_millis(20));
        assert_eq!(queue.wait(id, rx, Duration::from_millis(20)).await, ApprovalOutcome::TimedOut);
        assert!(queue.list(None).is_empty());
    }

    #[tokio::test]
    async fn the_list_filters_by_vault_and_cancel_all_empties_it() {
        let (queue, _events) = queue();
        let (_, rx_a) = queue.request(sample("a"), Duration::from_secs(60));
        let _b = queue.request(sample("b"), Duration::from_secs(60));
        assert_eq!(queue.list(Some("a")).len(), 1);
        assert_eq!(queue.list(None).len(), 2);
        queue.cancel_all();
        assert!(queue.list(None).is_empty());
        assert_eq!(rx_a.await.unwrap(), ApprovalOutcome::Cancelled);
    }
}
