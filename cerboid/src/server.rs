//! The socket server: accept connections, run the handshake, dispatch calls,
//! stream events, and stop gracefully.

use crate::approvals::{self, Approvals};
use crate::embed::{EmbedWorker, EmbedderFactory};
use crate::instance::{self, Acquired};
use crate::ops::{self, OpResult, Written};
use crate::registry::{Registry, VaultHandle, WatchSettings};
use crate::watch::WriteTracker;
use cerbo_core::app_lock::AppLock;
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::agent::{self, AgentPermission, ChangeEntry};
use cerbo_core::rpc::{
    self, ApprovalOutcome, Call, ClientHello, ClientKind, Event, Hello, Origin, Request, RpcError, ServerMessage,
    StatusInfo, VaultInfo, PROTOCOL_VERSION,
};
use cerbo_core::CerboContext;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, watch, Mutex, Notify};

/// The longest line a client may send. A page is sent whole, so this is
/// generous; it only bounds what one bad client can make the daemon buffer.
const MAX_LINE: usize = 32 * 1024 * 1024;
/// How long a stop waits for in-flight requests after the queues drained.
const DRAIN_LIMIT: Duration = Duration::from_secs(30);

/// How the daemon is set up.
#[derive(Clone)]
pub struct Config {
    /// Global config (`vaults.toml`) and cache (indexes, app lock) locations.
    pub ctx: CerboContext,
    pub paths: DaemonPaths,
    /// How often the vault list is checked for changes.
    pub registry_poll: Duration,
    /// How long a vault must be quiet before its change events are sent.
    pub event_quiet: Duration,
    pub embedder: EmbedderFactory,
    /// How long an approval request waits for an answer. `None` reads
    /// `approval_timeout_secs` of `agents.toml` at each request (default 10
    /// minutes).
    pub approval_timeout: Option<Duration>,
}

impl Config {
    pub fn new(ctx: CerboContext, paths: DaemonPaths) -> Self {
        Self {
            ctx,
            paths,
            registry_poll: Duration::from_millis(500),
            event_quiet: Duration::from_millis(100),
            embedder: crate::embed::default_factory(),
            approval_timeout: None,
        }
    }
}

/// What a start found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Started {
    /// This process is the daemon now and accepts connections.
    Running,
    /// Another daemon runs; this one has nothing to do.
    AlreadyRunning,
}

/// Asks a running daemon to stop; cloneable and usable from any thread.
#[derive(Clone)]
pub struct ShutdownCtl {
    tx: Arc<watch::Sender<bool>>,
}

impl ShutdownCtl {
    pub fn new() -> Self {
        Self { tx: Arc::new(watch::channel(false).0) }
    }

    pub fn trigger(&self) {
        self.tx.send_replace(true);
    }

    pub fn is_triggered(&self) -> bool {
        *self.tx.borrow()
    }

    async fn triggered(&self) {
        let mut rx = self.tx.subscribe();
        let _ = rx.wait_for(|stopped| *stopped).await;
    }
}

impl Default for ShutdownCtl {
    fn default() -> Self {
        Self::new()
    }
}

/// Counts requests that have not been answered yet.
struct Inflight {
    count: AtomicUsize,
    idle: Notify,
}

struct InflightGuard(Arc<Inflight>);

impl Inflight {
    fn enter(self: &Arc<Self>) -> InflightGuard {
        self.count.fetch_add(1, Ordering::SeqCst);
        InflightGuard(Arc::clone(self))
    }

    async fn wait_idle(&self) {
        loop {
            let notified = self.idle.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.count.load(Ordering::SeqCst) == 0 {
                return;
            }
            notified.await;
        }
    }
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        if self.0.count.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.0.idle.notify_waiters();
        }
    }
}

type Fut = Pin<Box<dyn Future<Output = OpResult> + Send>>;

fn ready(result: OpResult) -> Fut {
    Box::pin(async move { result })
}

struct Shared {
    ctx: CerboContext,
    approvals: Arc<Approvals>,
    approval_timeout: Option<Duration>,
    registry: Registry,
    events: broadcast::Sender<Event>,
    tracker: Arc<WriteTracker>,
    embed: Arc<EmbedWorker>,
    started: Instant,
    shutdown: ShutdownCtl,
    inflight: Arc<Inflight>,
    close_connections: watch::Sender<bool>,
}

/// Run the daemon until it is told to stop. `started` hears whether this
/// process became the daemon, as soon as that is known.
pub async fn run(
    config: Config,
    ctl: ShutdownCtl,
    started: Option<std::sync::mpsc::Sender<Result<Started, String>>>,
) -> Result<(), String> {
    let tell = |outcome: Result<Started, String>| {
        if let Some(tx) = &started {
            let _ = tx.send(outcome);
        }
    };
    match serve(config, ctl, &tell).await {
        Ok(()) => Ok(()),
        Err(e) => {
            tell(Err(e.clone()));
            Err(e)
        }
    }
}

async fn serve(
    config: Config,
    ctl: ShutdownCtl,
    tell: &dyn Fn(Result<Started, String>),
) -> Result<(), String> {
    let instance = match instance::acquire(&config.paths)? {
        Acquired::Instance(instance) => instance,
        Acquired::AlreadyRunning => {
            tell(Ok(Started::AlreadyRunning));
            return Ok(());
        }
    };

    // A running daemon keeps automatic vault rewrites away, as a running app
    // does (spec "Keeps automatic rewrites away"). Held until this function ends.
    let cache_dir = config.ctx.cache_dir.clone();
    let _app_lock = tokio::task::spawn_blocking(move || AppLock::shared(&cache_dir))
        .await
        .map_err(|e| e.to_string())??;

    let (events, _) = broadcast::channel(1024);
    let tracker = Arc::new(WriteTracker::default());
    let registry = Registry::new(
        config.ctx.clone(),
        WatchSettings { events: events.clone(), tracker: Arc::clone(&tracker), quiet: config.event_quiet },
    )?;
    let listener = UnixListener::from_std(instance.bind()?).map_err(|e| e.to_string())?;

    let shared = Arc::new(Shared {
        ctx: config.ctx.clone(),
        approvals: Arc::new(Approvals::new(events.clone())),
        approval_timeout: config.approval_timeout,
        registry,
        events,
        tracker,
        embed: Arc::new(EmbedWorker::new(Arc::clone(&config.embedder), config.ctx.cache_dir.clone())),
        started: Instant::now(),
        shutdown: ctl.clone(),
        inflight: Arc::new(Inflight { count: AtomicUsize::new(0), idle: Notify::new() }),
        close_connections: watch::channel(false).0,
    });
    tell(Ok(Started::Running));

    let poller = tokio::spawn(poll_registry(Arc::clone(&shared), config.registry_poll));

    loop {
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => { tokio::spawn(connection(Arc::clone(&shared), stream)); }
                Err(e) => eprintln!("cerboid: accept failed: {e}"),
            },
            () = ctl.triggered() => break,
        }
    }

    // Stop: no new connections; queued writes finish; answers are delivered;
    // then connections close and the socket goes away (spec "Stop finishes
    // writes").
    drop(listener);
    poller.abort();
    // Requests nobody answered end now; their agents are told the daemon stopped.
    shared.approvals.cancel_all();
    shared.registry.close_all().await;
    if tokio::time::timeout(DRAIN_LIMIT, shared.inflight.wait_idle()).await.is_err() {
        eprintln!("cerboid: stopping with requests still unanswered");
    }
    shared.close_connections.send_replace(true);
    // Let connection tasks drop their sockets before the socket file goes.
    tokio::time::sleep(Duration::from_millis(20)).await;
    drop(instance);
    Ok(())
}

async fn poll_registry(shared: Arc<Shared>, every: Duration) {
    loop {
        tokio::time::sleep(every).await;
        let this = Arc::clone(&shared);
        let refreshed = tokio::task::spawn_blocking(move || this.registry.refresh()).await;
        // The registry announces a change itself.
        if let Err(e) = refreshed.unwrap_or_else(|_| Err("the vault list check crashed".to_string())) {
            eprintln!("cerboid: cannot read the vault list: {e}");
        }
    }
}

// ── Connections ──────────────────────────────────────────────────────────────

enum Line {
    Eof,
    TooLong,
    Text(String),
}

async fn read_line<R: AsyncBufRead + Unpin>(reader: &mut R, buf: &mut Vec<u8>) -> std::io::Result<Line> {
    buf.clear();
    let n = (&mut *reader).take(MAX_LINE as u64 + 1).read_until(b'\n', buf).await?;
    if n == 0 {
        return Ok(Line::Eof);
    }
    if buf.len() > MAX_LINE {
        return Ok(Line::TooLong);
    }
    Ok(Line::Text(String::from_utf8_lossy(buf).trim_end().to_string()))
}

/// Resolves when the daemon closes its connections. A function of its own so
/// the watch guard is gone before the `select!` result crosses an await.
async fn wait_closing(rx: &mut watch::Receiver<bool>) {
    let _ = rx.wait_for(|closing| *closing).await;
}

#[derive(Clone)]
struct Writer(Arc<Mutex<OwnedWriteHalf>>);

impl Writer {
    async fn send(&self, message: &ServerMessage) {
        let Ok(mut line) = serde_json::to_vec(message) else { return };
        line.push(b'\n');
        let mut half = self.0.lock().await;
        // A client that went away is not an error worth reporting.
        let _ = half.write_all(&line).await;
    }
}

async fn connection(shared: Arc<Shared>, stream: UnixStream) {
    let (read_half, write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let writer = Writer(Arc::new(Mutex::new(write_half)));
    let mut buf = Vec::new();

    // Handshake: the first line must be a hello with our protocol version.
    let client = match read_line(&mut reader, &mut buf).await {
        Ok(Line::Text(text)) => match serde_json::from_str::<ClientHello>(&text) {
            Ok(hello) => match rpc::check_protocol(hello.hello.protocol) {
                Ok(()) => hello.hello,
                Err(error) => {
                    writer.send(&ServerMessage::Error { error }).await;
                    return;
                }
            },
            Err(e) => {
                let error = RpcError::new(rpc::CODE_BAD_REQUEST, format!("expected a hello message: {e}"));
                writer.send(&ServerMessage::Error { error }).await;
                return;
            }
        },
        _ => return,
    };
    writer
        .send(&ServerMessage::Welcome {
            protocol: PROTOCOL_VERSION,
            daemon_version: env!("CARGO_PKG_VERSION").to_string(),
            pid: std::process::id(),
        })
        .await;

    let mut close = shared.close_connections.subscribe();
    let mut subscription: Option<tokio::task::JoinHandle<()>> = None;

    loop {
        let text = tokio::select! {
            line = read_line(&mut reader, &mut buf) => match line {
                Ok(Line::Text(text)) => text,
                Ok(Line::TooLong) => {
                    let error = RpcError::new(rpc::CODE_BAD_REQUEST, "message too long");
                    writer.send(&ServerMessage::Error { error }).await;
                    break;
                }
                Ok(Line::Eof) | Err(_) => break,
            },
            () = wait_closing(&mut close) => break,
        };
        if text.is_empty() {
            continue;
        }
        let request = match serde_json::from_str::<Request>(&text) {
            Ok(request) => request,
            Err(e) => {
                let error = RpcError::new(rpc::CODE_BAD_REQUEST, format!("unreadable request: {e}"));
                // Answer under the request's id when it has one.
                let id = serde_json::from_str::<Value>(&text).ok().and_then(|v| v["id"].as_u64());
                let message = match id {
                    Some(id) => ServerMessage::err(id, error),
                    None => ServerMessage::Error { error },
                };
                writer.send(&message).await;
                continue;
            }
        };
        let Request { id, call } = request;
        let guard = shared.inflight.enter();

        let outcome: Result<Fut, RpcError> = match call {
            // An agent never stops the daemon or sees or answers approvals:
            // the person decides what the agent may do.
            Call::Stop | Call::ApprovalList { .. } | Call::ApprovalAnswer { .. } | Call::AgentChanges { .. }
                if client.client == ClientKind::Agent =>
            {
                Err(RpcError::new(rpc::CODE_FORBIDDEN, "agent connections cannot make this call"))
            }
            Call::Subscribe { vault } => subscribe(&shared, &writer, vault, &mut subscription),
            Call::Stop => {
                // The guard is held until the answer is written, and the stop
                // waits for every guard, so the answer is delivered first.
                shared.shutdown.trigger();
                Ok(ready(Ok(Value::Null)))
            }
            other => shared.begin(other, &client),
        };
        let writer = writer.clone();
        tokio::spawn(async move {
            let message = match outcome {
                Ok(fut) => match fut.await {
                    Ok(result) => ServerMessage::ok(id, result),
                    Err(error) => ServerMessage::err(id, error),
                },
                Err(error) => ServerMessage::err(id, error),
            };
            writer.send(&message).await;
            drop(guard);
        });
    }
    if let Some(task) = subscription {
        task.abort();
    }
}

/// Start (or retarget) this connection's event stream. The receiver exists
/// before the answer goes out, so no event after the answer is missed.
fn subscribe(
    shared: &Arc<Shared>,
    writer: &Writer,
    vault: Option<String>,
    subscription: &mut Option<tokio::task::JoinHandle<()>>,
) -> Result<Fut, RpcError> {
    let wanted = match vault {
        Some(selector) => Some(
            shared
                .registry
                .resolve(&selector)
                .ok_or_else(|| RpcError::new(rpc::CODE_UNKNOWN_VAULT, format!("unknown vault: {selector}")))?
                .info
                .id
                .clone(),
        ),
        None => None,
    };
    if let Some(old) = subscription.take() {
        old.abort();
    }
    let mut events = shared.events.subscribe();
    let writer = writer.clone();
    let mut close = shared.close_connections.subscribe();
    *subscription = Some(tokio::spawn(async move {
        loop {
            tokio::select! {
                event = events.recv() => match event {
                    Ok(event) => {
                        let wanted_here = match (&wanted, event.vault()) {
                            (Some(wanted), Some(vault)) => wanted == vault,
                            _ => true,
                        };
                        if wanted_here {
                            writer.send(&ServerMessage::Event { event }).await;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                () = wait_closing(&mut close) => break,
            }
        }
    }));
    Ok(ready(Ok(Value::Null)))
}

// ── Dispatch ─────────────────────────────────────────────────────────────────

impl Shared {
    /// Start `call`. Writes are put on their vault's queue before this
    /// returns, so requests from one connection are applied in the order they
    /// arrived.
    fn begin(self: &Arc<Self>, call: Call, client: &Hello) -> Result<Fut, RpcError> {
        match call {
            Call::Ping => Ok(ready(Ok(json!("pong")))),
            Call::Status => Ok(ready(self.status())),
            Call::Vaults => Ok(ready(serde_json::to_value(self.vault_infos()).map_err(|e| RpcError::failed(e.to_string())))),
            Call::Stop | Call::Subscribe { .. } => unreachable!("handled by the connection"),
            Call::ApprovalList { vault } => {
                let id = match vault {
                    Some(selector) => Some(self.resolve(&selector)?.info.id.clone()),
                    None => None,
                };
                let list = self.approvals.list(id.as_deref());
                Ok(ready(serde_json::to_value(list).map_err(|e| RpcError::failed(e.to_string()))))
            }
            Call::ApprovalAnswer { approval, approve } => {
                Ok(ready(self.approvals.answer(approval, approve).map(|()| Value::Null)))
            }
            call => {
                let selector = call.vault().expect("every other call names a vault").to_string();
                let vault = self.resolve(&selector)?;
                match &call {
                    Call::Permission { .. } => {
                        let level = agent::get_permission(&self.ctx, &vault.info.id);
                        return Ok(ready(Ok(json!({ "permission": level.as_str() }))));
                    }
                    Call::AgentChanges { limit, .. } => {
                        let changes = agent::read_changes(&self.ctx, &vault.info.id, *limit).map_err(RpcError::failed)?;
                        return Ok(ready(serde_json::to_value(changes).map_err(|e| RpcError::failed(e.to_string()))));
                    }
                    _ => {}
                }
                if call.is_write() { self.begin_write(vault, call, client) } else { Ok(self.begin_read(vault, call)) }
            }
        }
    }

    fn resolve(&self, selector: &str) -> Result<Arc<VaultHandle>, RpcError> {
        self.registry
            .resolve(selector)
            .ok_or_else(|| RpcError::new(rpc::CODE_UNKNOWN_VAULT, format!("unknown vault: {selector}")))
    }

    fn vault_infos(&self) -> Vec<VaultInfo> {
        self.registry.list().iter().map(|v| v.info.clone()).collect()
    }

    fn status(&self) -> OpResult {
        let info = StatusInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            protocol: PROTOCOL_VERSION,
            pid: std::process::id(),
            uptime_secs: self.started.elapsed().as_secs(),
            memory_kb: resident_memory_kb(),
            vaults: self.vault_infos(),
        };
        serde_json::to_value(info).map_err(|e| RpcError::failed(e.to_string()))
    }

    fn begin_read(&self, vault: Arc<VaultHandle>, call: Call) -> Fut {
        let embed = Arc::clone(&self.embed);
        let task = tokio::task::spawn_blocking(move || read(&vault, &embed, call));
        Box::pin(async move { task.await.unwrap_or_else(|_| Err(RpcError::failed("the read crashed"))) })
    }

    /// A write. App and CLI connections go straight to the vault's queue; an
    /// agent's is judged by the vault's permission (design D6), read afresh so
    /// a change applies to connected agents at once.
    fn begin_write(self: &Arc<Self>, vault: Arc<VaultHandle>, call: Call, client: &Hello) -> Result<Fut, RpcError> {
        if self.shutdown.is_triggered() {
            return Err(RpcError::new(rpc::CODE_SHUTTING_DOWN, "the daemon is stopping and takes no more writes"));
        }
        if client.client != ClientKind::Agent {
            return self.queue_write(vault, call, client.client.into(), None);
        }
        let agent_name = client.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| "agent".to_string());
        match agent::get_permission(&self.ctx, &vault.info.id) {
            AgentPermission::Read => Err(read_only(&vault)),
            AgentPermission::Auto => self.queue_write(vault, call, Origin::Agent, Some(AgentLog::new(agent_name, false))),
            AgentPermission::Approve => {
                let preview = ops::preview(&vault, &call);
                let timeout =
                    self.approval_timeout.unwrap_or_else(|| agent::approval_timeout(&self.ctx));
                let (id, rx) = self.approvals.request(
                    approvals::Request {
                        vault: vault.info.id.clone(),
                        vault_name: vault.info.name.clone(),
                        agent: agent_name.clone(),
                        action: call.action().unwrap_or("change").to_string(),
                        page: preview.page,
                        summary: preview.summary,
                        diff: preview.diff,
                    },
                    timeout,
                );
                let this = Arc::clone(self);
                Ok(Box::pin(async move {
                    match this.approvals.wait(id, rx, timeout).await {
                        ApprovalOutcome::Approved => {
                            // The user may have tightened the vault while this waited.
                            if !agent::get_permission(&this.ctx, &vault.info.id).can_write() {
                                return Err(read_only(&vault));
                            }
                            match this.queue_write(vault, call, Origin::Agent, Some(AgentLog::new(agent_name, true))) {
                                Ok(done) => done.await,
                                Err(e) => Err(e),
                            }
                        }
                        ApprovalOutcome::Rejected => {
                            Err(RpcError::new(rpc::CODE_REJECTED, "the change was rejected by the user; the vault is unchanged"))
                        }
                        ApprovalOutcome::TimedOut => Err(RpcError::new(
                            rpc::CODE_TIMED_OUT,
                            "nobody answered the approval request in time, so it counts as rejected; the vault is unchanged",
                        )),
                        ApprovalOutcome::Cancelled => {
                            Err(RpcError::new(rpc::CODE_SHUTTING_DOWN, "the daemon stopped before the change was approved"))
                        }
                    }
                }))
            }
        }
    }

    /// Put a write on its vault's queue. With `log`, an applied change is
    /// appended to the agent change log from the queue, right after it ran.
    fn queue_write(
        &self,
        vault: Arc<VaultHandle>,
        call: Call,
        origin: Origin,
        log: Option<AgentLog>,
    ) -> Result<Fut, RpcError> {
        let tracker = Arc::clone(&self.tracker);
        let ctx = self.ctx.clone();
        let job_vault = Arc::clone(&vault);
        let queued = vault.queue.submit(move || {
            let id = job_vault.info.id.clone();
            let action = call.action().unwrap_or("change");
            // Attribute the file events before the files change.
            if let Some(page) = target_page(&call) {
                tracker.note(&id, &page, origin);
            }
            let Written { page, result } = write(&job_vault, call);
            if let Some(page) = &page {
                tracker.note(&id, page, origin);
                if let Some(log) = &log {
                    let entry = ChangeEntry {
                        time: approvals::now_secs(),
                        vault: id.clone(),
                        page: page.clone(),
                        agent: log.agent.clone(),
                        action: action.to_string(),
                        approved: log.approved,
                    };
                    if let Err(e) = agent::append_change(&ctx, &entry) {
                        eprintln!("cerboid: cannot record an agent change: {e}");
                    }
                }
            }
            result
        });
        let done = queued
            .map_err(|_| RpcError::new(rpc::CODE_SHUTTING_DOWN, "the daemon is stopping and takes no more writes"))?;
        Ok(Box::pin(async move { done.await.unwrap_or_else(|_| Err(RpcError::failed("the write crashed"))) }))
    }
}

/// The daemon's resident set size from `/proc` (Linux); none elsewhere.
fn resident_memory_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

/// Who an applied write is recorded for.
struct AgentLog {
    agent: String,
    approved: bool,
}

impl AgentLog {
    fn new(agent: String, approved: bool) -> Self {
        Self { agent, approved }
    }
}

fn read_only(vault: &VaultHandle) -> RpcError {
    RpcError::new(
        rpc::CODE_READ_ONLY,
        format!(
            "the vault \"{}\" is read-only for agents, so this change was refused; only the user can allow agents to change it",
            vault.info.name
        ),
    )
}

/// The page a write names up front, when it names one.
fn target_page(call: &Call) -> Option<String> {
    let id = match call {
        Call::PageEdit { page, .. }
        | Call::PageRename { page, .. }
        | Call::PageMove { page, .. }
        | Call::PageTrash { page, .. } => page,
        Call::LinkAdd { from, .. } => from,
        _ => return None,
    };
    ops::page_id(id).ok()
}

fn read(vault: &VaultHandle, embed: &EmbedWorker, call: Call) -> OpResult {
    match call {
        Call::PageList { .. } => ops::page_list(vault),
        Call::PageRead { page, .. } => ops::page_read(vault, &page),
        Call::AttachmentList { page, .. } => ops::attachment_list(vault, page.as_deref()),
        Call::AttachmentInfo { attachment, .. } => ops::attachment_info(vault, &attachment),
        Call::Links { .. } => ops::links(vault),
        Call::Backlinks { page, .. } => ops::backlinks(vault, &page),
        Call::OutgoingLinks { page, .. } => ops::outgoing_links(vault, &page),
        Call::SearchText { query, limit, .. } => ops::search_text(vault, &query, limit),
        Call::SearchSemantic { query, limit, types, annotated, .. } => {
            ops::search_semantic(vault, embed, query, limit, types, annotated)
        }
        Call::Similar { page, limit, types, annotated, .. } => {
            ops::similar(vault, embed, &page, limit, types, annotated)
        }
        Call::Sparql { query, .. } => ops::sparql(vault, &query),
        other => Err(RpcError::failed(format!("not a read: {other:?}"))),
    }
}

fn write(vault: &VaultHandle, call: Call) -> Written {
    match call {
        Call::PageCreate { title, content, path, .. } => ops::page_create(vault, title, content, path),
        Call::PageEdit { page, content, .. } => ops::page_edit(vault, &page, content),
        Call::PageRename { page, title, .. } => ops::page_rename(vault, &page, title),
        Call::PageMove { page, path, .. } => ops::page_move(vault, &page, path),
        Call::PageTrash { page, .. } => ops::page_trash(vault, &page),
        Call::LinkAdd { from, to, .. } => ops::link_add(vault, &from, &to),
        other => Written { page: None, result: Err(RpcError::failed(format!("not a write: {other:?}"))) },
    }
}
