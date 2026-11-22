//! The socket server: accept connections, run the handshake, dispatch calls,
//! stream events, and stop gracefully.

use crate::approvals::{self, Approvals};
use crate::embed::{EmbedWorker, EmbedderFactory};
use crate::instance::{self, Acquired};
use crate::ops::{self, OpResult, Written};
use crate::registry::{Registry, VaultHandle, WatchSettings};
use crate::sessions::{self, Manager, SocketGuard, VaultRef};
use crate::watch::WriteTracker;
use cerbo_core::app_lock::AppLock;
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::agent::{self, AgentPermission, ChangeEntry};
use cerbo_core::host_access::{self, Action as HostAction, Decision as HostDecision};
use cerbo_core::rpc::{
    self, AdminAction, AdminInfo, AdminReply, AdminRequest, ApprovalOutcome, Call, ClientHello, ClientKind, Event, Hello, Origin, Request, RpcError, ServerMessage,
    StatusInfo, VaultInfo, MAX_PENDING_PER_AGENT, PROTOCOL_VERSION,
};
use cerbo_core::CerboContext;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
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
    /// How many approval requests one agent connection may have waiting;
    /// more are refused at once. Default [`MAX_PENDING_PER_AGENT`].
    pub max_pending_per_agent: usize,
    /// Called on the queue's thread at the start of every write job, before
    /// anything is checked or changed. For tests, to hold the queue still.
    pub before_write: Option<Arc<dyn Fn() + Send + Sync>>,
    /// What the agent session manager looks at: the user's environment, the
    /// bubblewrap and the `cerbo` binary. Tests give their own.
    pub agents: sessions::Settings,
}

impl Config {
    pub fn new(ctx: CerboContext, paths: DaemonPaths) -> Self {
        let agents = sessions::Settings::from_process(&paths);
        Self {
            ctx,
            paths,
            registry_poll: Duration::from_millis(500),
            event_quiet: Duration::from_millis(100),
            embedder: crate::embed::default_factory(),
            approval_timeout: None,
            max_pending_per_agent: MAX_PENDING_PER_AGENT,
            before_write: None,
            agents,
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
    /// The daemon's own runtime directory (socket and lock): never reachable
    /// through host access.
    daemon_dir: std::path::PathBuf,
    approvals: Arc<Approvals>,
    sessions: Arc<Manager>,
    approval_timeout: Option<Duration>,
    max_pending_per_agent: usize,
    /// Approval requests of agents that wait for a person, counted per session
    /// (agents on a session socket) or per vault (agents on the main socket).
    pending: Arc<PendingCounts>,
    before_write: Option<Arc<dyn Fn() + Send + Sync>>,
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
    let mut instance = match instance::acquire(&config.paths)? {
        Acquired::Instance(instance) => instance,
        Acquired::AlreadyRunning => {
            // Started by systemd for its socket, yet another daemon holds the
            // lock: exiting 0 would leave a socket nobody serves (and the
            // service not restarted), so this is a failure.
            if instance::socket_activated() {
                return Err(format!(
                    "another cerboid holds {} although systemd started this one for the socket; \
                     nobody would serve it",
                    config.paths.lock.display()
                ));
            }
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
    // Under systemd socket activation the socket already exists and belongs
    // to systemd: use it as it is, never unlink and rebind it.
    let std_listener = match instance::inherited_listener()? {
        Some(listener) => {
            instance.leave_socket_to_systemd();
            listener
        }
        None => instance.bind()?,
    };
    let listener = UnixListener::from_std(std_listener).map_err(|e| e.to_string())?;

    let approvals = Arc::new(Approvals::new(events.clone()));
    let sessions = Manager::new(config.agents.clone(), config.ctx.clone(), Arc::clone(&approvals), events.clone(), config.approval_timeout);
    let shared = Arc::new(Shared {
        ctx: config.ctx.clone(),
        daemon_dir: config.paths.dir.clone(),
        approvals,
        sessions,
        approval_timeout: config.approval_timeout,
        max_pending_per_agent: config.max_pending_per_agent,
        pending: Arc::new(PendingCounts::default()),
        before_write: config.before_write.clone(),
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
                Ok((stream, _)) => { tokio::spawn(connection(Arc::clone(&shared), stream, None)); }
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
    // Agents end with the daemon; their waiting requests are cancelled.
    shared.sessions.shutdown().await;
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

/// The action of a first line that is an administration line, if it is one.
fn admin_line(text: &str) -> Option<AdminAction> {
    serde_json::from_str::<AdminRequest>(text).ok().map(|request| request.admin)
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

/// What a connection is: who it says it is, the vault an agent is bound to,
/// and the means to notice that it went away.
struct Conn {
    hello: Hello,
    /// The session an agent connection belongs to, when it came in on a
    /// session socket.
    session: Option<String>,
    /// Id of the one vault an agent connection may touch; none for the app
    /// and the CLI.
    bound: Option<String>,
    /// Becomes true when the connection ends.
    closed: watch::Receiver<bool>,
    /// Sessions this connection made with `ephemeral`: they end with it.
    owned: Arc<std::sync::Mutex<Vec<String>>>,
}

/// How many approval requests wait for a person, by key. An agent can open
/// any number of connections, so the count is not a connection's: it is the
/// session's (agents on a session socket) or the vault's (agents on the main
/// socket).
#[derive(Default)]
struct PendingCounts(std::sync::Mutex<HashMap<String, usize>>);

/// One waiting approval request; released when dropped.
struct PendingSlot {
    counts: Arc<PendingCounts>,
    key: String,
}

impl PendingSlot {
    fn take(counts: &Arc<PendingCounts>, key: String, max: usize) -> Option<Self> {
        let mut map = counts.0.lock().unwrap_or_else(|e| e.into_inner());
        let n = map.entry(key.clone()).or_insert(0);
        if *n >= max {
            return None;
        }
        *n += 1;
        Some(Self { counts: Arc::clone(counts), key })
    }
}

impl Drop for PendingSlot {
    fn drop(&mut self) {
        let mut map = self.counts.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(n) = map.get_mut(&self.key) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                map.remove(&self.key);
            }
        }
    }
}

fn forbidden(message: impl Into<String>) -> RpcError {
    RpcError::new(rpc::CODE_FORBIDDEN, message)
}

/// What a session's socket forces on every connection that comes in on it
/// (design D9).
#[derive(Clone)]
struct SessionBinding {
    session: String,
    /// Id of the session's vault.
    vault: String,
    /// Becomes true when the session ends.
    ended: watch::Receiver<bool>,
}

/// Resolves when the connection's session ends; never for a connection on the
/// daemon's main socket.
async fn wait_session_end(binding: &mut Option<SessionBinding>) {
    match binding {
        Some(binding) => wait_closing(&mut binding.ended).await,
        None => std::future::pending::<()>().await,
    }
}

async fn connection(shared: Arc<Shared>, stream: UnixStream, mut forced: Option<SessionBinding>) {
    let (read_half, write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let writer = Writer(Arc::new(Mutex::new(write_half)));
    let mut buf = Vec::new();

    // Handshake: the first line must be a hello with our protocol version.
    let client = match read_line(&mut reader, &mut buf).await {
        Ok(Line::Text(text)) => match serde_json::from_str::<ClientHello>(&text) {
            Err(_) if forced.is_some() && admin_line(&text).is_some() => {
                // Nothing on a session's socket may stop or probe the daemon.
                let error = forbidden("administration is not available on an agent session socket");
                writer.send(&ServerMessage::Error { error }).await;
                return;
            }
            Err(_) if admin_line(&text).is_some() => {
                // Version-independent administration: answered whatever the
                // client's protocol version, so a stale daemon can be stopped.
                let action = admin_line(&text).expect("just checked");
                let info = AdminInfo {
                    protocol: PROTOCOL_VERSION,
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    pid: std::process::id(),
                    uptime_secs: shared.started.elapsed().as_secs(),
                };
                // Answer first: a stop closes connections once it ran.
                let mut line = serde_json::to_vec(&AdminReply { admin: info }).unwrap_or_default();
                line.push(b'\n');
                let _ = writer.0.lock().await.write_all(&line).await;
                if action == AdminAction::Stop {
                    shared.shutdown.trigger();
                }
                return;
            }
            Ok(hello) => match rpc::check_protocol(hello.hello.protocol) {
                Ok(()) => {
                    let mut hello = hello.hello;
                    // On a session socket every connection is an agent of the
                    // session's vault, whatever its hello says.
                    if let Some(binding) = &forced {
                        hello.client = ClientKind::Agent;
                        hello.vault = Some(binding.vault.clone());
                    }
                    hello
                }
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
    // An agent is bound to exactly one vault, named in its hello.
    let bound = if client.client == ClientKind::Agent {
        match shared.bind_agent(&client) {
            Ok(id) => Some(id),
            Err(error) => {
                writer.send(&ServerMessage::Error { error }).await;
                return;
            }
        }
    } else {
        None
    };
    writer
        .send(&ServerMessage::Welcome {
            protocol: PROTOCOL_VERSION,
            daemon_version: env!("CARGO_PKG_VERSION").to_string(),
            pid: std::process::id(),
        })
        .await;

    let (closed_tx, closed_rx) = watch::channel(false);
    let session = forced.as_ref().map(|b| b.session.clone());
    let conn = Conn { hello: client, session, bound, closed: closed_rx, owned: Arc::new(std::sync::Mutex::new(Vec::new())) };
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
            () = wait_session_end(&mut forced) => break,
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

        let refusal = shared.agent_refusal(&conn, &call);
        let outcome: Result<Fut, RpcError> = match call {
            // An agent never stops the daemon or sees or answers approvals,
            // and only touches the vault it is bound to: the person decides
            // what the agent may do.
            _ if refusal.is_some() => Err(refusal.expect("just checked")),
            Call::Subscribe { vault } => {
                subscribe(&shared, &writer, vault, conn.bound.as_deref(), &mut subscription)
            }
            Call::Stop => {
                // The guard is held until the answer is written, and the stop
                // waits for every guard, so the answer is delivered first.
                shared.shutdown.trigger();
                Ok(ready(Ok(Value::Null)))
            }
            other => shared.begin(other, &conn),
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
    // Whatever this connection still has waiting for a person is dropped: its
    // agent can no longer be told the answer, so it must never be applied.
    closed_tx.send_replace(true);
    if let Some(task) = subscription {
        task.abort();
    }
    // Sessions made for this connection alone end with it.
    let owned: Vec<String> = std::mem::take(&mut *conn.owned.lock().unwrap_or_else(|e| e.into_inner()));
    for id in owned {
        let sessions = Arc::clone(&shared.sessions);
        tokio::spawn(async move {
            let _ = sessions.end(&id).await;
        });
    }
}

/// Start (or retarget) this connection's event stream. The receiver exists
/// before the answer goes out, so no event after the answer is missed.
fn subscribe(
    shared: &Arc<Shared>,
    writer: &Writer,
    vault: Option<String>,
    agent_vault: Option<&str>,
    subscription: &mut Option<tokio::task::JoinHandle<()>>,
) -> Result<Fut, RpcError> {
    // An agent hears the page events of its own vault and nothing else.
    let page_events_only = agent_vault.is_some();
    let vault = agent_vault.map(str::to_string).or(vault);
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
                        } && (!page_events_only || matches!(event, Event::PageChanged { .. }));
                        if wanted_here {
                            writer.send(&ServerMessage::Event { event }).await;
                        }
                    }
                    // Events were dropped for this connection: it cannot know
                    // what it missed, so it is told to read the state again.
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        if !page_events_only {
                            writer.send(&ServerMessage::Event { event: Event::Resync }).await;
                        }
                    }
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
    fn begin(self: &Arc<Self>, call: Call, conn: &Conn) -> Result<Fut, RpcError> {
        match call {
            Call::Ping => Ok(ready(Ok(json!("pong")))),
            Call::Status => Ok(ready(self.status())),
            Call::Vaults => {
                let mut vaults = self.vault_infos();
                if let Some(bound) = &conn.bound {
                    vaults.retain(|v| &v.id == bound);
                }
                Ok(ready(serde_json::to_value(vaults).map_err(|e| RpcError::failed(e.to_string()))))
            }
            Call::Stop | Call::Subscribe { .. } => unreachable!("handled by the connection"),
            Call::AgentList {} => {
                let sessions = Arc::clone(&self.sessions);
                Ok(Box::pin(async move { sessions.agent_list().await }))
            }
            Call::SessionCreate { vault, agent, resume_from, ephemeral } => {
                let handle = self.resolve(&vault)?;
                let vault = VaultRef { id: handle.info.id.clone(), name: handle.info.name.clone(), root: handle.root.clone() };
                let this = Arc::clone(self);
                let owned = Arc::clone(&conn.owned);
                let closed = conn.closed.clone();
                Ok(Box::pin(async move {
                    let opener: sessions::SocketOpener = {
                        let this = Arc::clone(&this);
                        Arc::new(move |session: &str, vault: &str, dir: &Path| this.open_session_socket(session, vault, dir))
                    };
                    let info = this.sessions.create(vault, agent, resume_from, opener).await?;
                    if ephemeral {
                        // The session belongs to this connection: it ends with
                        // it. A connection that closed meanwhile ends it now.
                        owned.lock().unwrap_or_else(|e| e.into_inner()).push(info.id.clone());
                        if *closed.borrow() {
                            let _ = this.sessions.end(&info.id).await;
                        }
                    }
                    serde_json::to_value(info).map_err(|e| RpcError::failed(e.to_string()))
                }))
            }
            Call::SessionSend { session, text, context } => {
                let vault = self.sessions.vault_of(&session).and_then(|id| self.registry.resolve(&id));
                let read_page = move |page: &str| -> Option<String> {
                    let vault = vault.as_ref()?;
                    let page = ops::page_read(vault, page).ok()?;
                    page["content"].as_str().map(str::to_string)
                };
                Ok(ready(self.sessions.send(&session, text, context, &read_page).map(|()| Value::Null)))
            }
            Call::SessionCancel { session } => Ok(ready(self.sessions.cancel(&session).map(|()| Value::Null))),
            Call::SessionEnd { session } => {
                let sessions = Arc::clone(&self.sessions);
                Ok(Box::pin(async move { sessions.end(&session).await.map(|()| Value::Null) }))
            }
            Call::SessionList { vault } => {
                let id = match vault {
                    Some(selector) => Some(self.resolve(&selector)?.info.id.clone()),
                    None => None,
                };
                let list = self.sessions.list(id.as_deref());
                Ok(ready(serde_json::to_value(list).map_err(|e| RpcError::failed(e.to_string()))))
            }
            Call::SessionDelete { session } => Ok(ready(self.sessions.delete(&session).map(|()| Value::Null))),
            Call::SessionEvents { session, after_seq, limit } => {
                let events = self.sessions.events(&session, after_seq, limit)?;
                Ok(ready(serde_json::to_value(events).map_err(|e| RpcError::failed(e.to_string()))))
            }
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
                        let host = agent::get_host(&self.ctx, &vault.info.id).enabled;
                        let wait = self.approval_timeout.unwrap_or_else(|| agent::approval_timeout(&self.ctx));
                        return Ok(ready(Ok(json!({
                            "permission": level.as_str(),
                            "approval_timeout_secs": wait.as_secs(),
                            "host": host,
                        }))));
                    }
                    c if c.is_host() => return self.begin_host(vault, call, conn),
                    Call::AgentChanges { limit, .. } => {
                        let changes = agent::read_changes(&self.ctx, &vault.info.id, *limit).map_err(RpcError::failed)?;
                        return Ok(ready(serde_json::to_value(changes).map_err(|e| RpcError::failed(e.to_string()))));
                    }
                    _ => {}
                }
                if call.is_write() { self.begin_write(vault, call, conn) } else { Ok(self.begin_read(vault, call)) }
            }
        }
    }

    /// Listen on a session's own socket (design D9): the directory is private,
    /// the socket owner-only, and every connection on it is an agent of the
    /// session's vault. The listener stops and the directory goes when the
    /// returned guard is dropped.
    fn open_session_socket(self: &Arc<Self>, session: &str, vault: &str, dir: &Path) -> Result<SocketGuard, String> {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        let err = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
        if let Some(parent) = dir.parent() {
            std::fs::DirBuilder::new().recursive(true).mode(0o700).create(parent).map_err(|e| err(parent, e))?;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).map_err(|e| err(parent, e))?;
        }
        // A directory left by a session that did not end cleanly is replaced.
        let _ = std::fs::remove_dir_all(dir);
        std::fs::DirBuilder::new().mode(0o700).create(dir).map_err(|e| err(dir, e))?;
        let path = dir.join(crate::sandbox::SESSION_SOCKET_NAME);
        let listener = std::os::unix::net::UnixListener::bind(&path).map_err(|e| err(&path, e))?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).map_err(|e| err(&path, e))?;
        listener.set_nonblocking(true).map_err(|e| err(&path, e))?;
        let listener = UnixListener::from_std(listener).map_err(|e| err(&path, e))?;
        let (ended_tx, ended_rx) = watch::channel(false);
        let binding = SessionBinding { session: session.to_string(), vault: vault.to_string(), ended: ended_rx };
        let shared = Arc::clone(self);
        let task = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        tokio::spawn(connection(Arc::clone(&shared), stream, Some(binding.clone())));
                    }
                    Err(e) => {
                        eprintln!("cerboid: a session socket cannot accept: {e}");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        });
        Ok(SocketGuard::new(dir.to_path_buf(), task, ended_tx))
    }

    /// The vault an agent's hello binds it to: its id.
    fn bind_agent(&self, hello: &Hello) -> Result<String, RpcError> {
        let selector = hello.vault.as_deref().filter(|v| !v.is_empty()).ok_or_else(|| {
            RpcError::new(rpc::CODE_BAD_REQUEST, "an agent connection must name the vault it works in, in its hello")
        })?;
        Ok(self.resolve(selector)?.info.id.clone())
    }

    /// Why an agent connection may not make `call`, if it may not.
    fn agent_refusal(&self, conn: &Conn, call: &Call) -> Option<RpcError> {
        let bound = conn.bound.as_deref()?;
        if call.is_session_call() {
            return Some(forbidden("agent connections cannot use agent sessions"));
        }
        if matches!(
            call,
            Call::Stop | Call::Status | Call::ApprovalList { .. } | Call::ApprovalAnswer { .. } | Call::AgentChanges { .. }
        ) {
            return Some(forbidden("agent connections cannot make this call"));
        }
        let named = match call {
            Call::Subscribe { vault } => vault.as_deref(),
            other => other.vault(),
        };
        let named = named?;
        match self.registry.resolve(named) {
            Some(vault) if vault.info.id == bound => None,
            _ => Some(forbidden("this agent connection is bound to another vault and cannot use this one")),
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

    /// The places host access never reaches on this machine, for this vault.
    fn host_guard(&self, vault: &VaultHandle) -> cerbo_core::host_access::Guard {
        let env = crate::agents::HostEnv::from_process();
        let mut protected = vec![self.ctx.config_dir.clone(), self.ctx.cache_dir.clone(), self.daemon_dir.clone()];
        if let Some(data) = env.data_home() {
            protected.push(data.join("cerbo"));
            protected.push(data.join("opencode/auth.json"));
        }
        let claude = env.claude_config_dir.clone().or_else(|| env.home.as_ref().map(|h| h.join(".claude")));
        if let Some(claude) = claude {
            protected.push(claude.join(".credentials.json"));
        }
        let others = self.registry.list().iter().filter(|v| v.info.id != vault.info.id).map(|v| v.root.clone()).collect();
        agent::host_guard(vault.root.clone(), others, protected)
    }

    /// A host call (design D2): judged by the vault's host access setting,
    /// read afresh on every call so a change applies to the next one. Covered
    /// by the list: done at once. Not covered: a person approves this one
    /// action. Never reachable or off: refused.
    fn begin_host(self: &Arc<Self>, vault: Arc<VaultHandle>, call: Call, conn: &Conn) -> Result<Fut, RpcError> {
        if conn.bound.is_none() {
            return Err(forbidden("host calls are for agent connections"));
        }
        if self.shutdown.is_triggered() {
            return Err(RpcError::new(rpc::CODE_SHUTTING_DOWN, "the daemon is stopping"));
        }
        let (path, action) = match &call {
            Call::HostList { path, .. } => (path.clone(), HostAction::List),
            Call::HostRead { path, .. } => (path.clone(), HostAction::Read),
            Call::HostWrite { path, .. } => (path.clone(), HostAction::Write),
            _ => unreachable!("begin_host takes host calls"),
        };
        let agent_name = conn.hello.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| "agent".to_string());
        let real = host_access::resolve(&path, action).map_err(|e| RpcError::new(rpc::CODE_BAD_REQUEST, e))?;
        let settings = agent::get_host(&self.ctx, &vault.info.id);
        let guard = self.host_guard(&vault);
        let log = {
            let ctx = self.ctx.clone();
            let vault_id = vault.info.id.clone();
            let agent_name = agent_name.clone();
            let real = real.clone();
            move |approved: bool, outcome: &str| log_host(&ctx, &vault_id, &agent_name, action, &real, approved, outcome)
        };
        match host_access::decide(&settings, &guard, &real, action) {
            HostDecision::Refuse(why) => {
                log(false, "refused");
                Err(RpcError::new(rpc::CODE_FORBIDDEN, why))
            }
            HostDecision::Allow => {
                let task = tokio::task::spawn_blocking(move || run_host(&real, &call));
                Ok(Box::pin(async move {
                    let result = task.await.unwrap_or_else(|_| Err(RpcError::failed("the host call crashed")));
                    log(false, if result.is_ok() { "done" } else { "failed" });
                    result
                }))
            }
            HostDecision::Ask => {
                let (key, whose) = match &conn.session {
                    Some(session) => (format!("session:{session}"), "this session"),
                    None => (format!("vault:{}", vault.info.id), "this vault"),
                };
                let slot = PendingSlot::take(&self.pending, key, self.max_pending_per_agent).ok_or_else(|| {
                    RpcError::new(
                        rpc::CODE_TOO_MANY_PENDING,
                        format!(
                            "{whose} already has {} requests waiting for approval, the most it may have; \
                             wait until some are answered",
                            self.max_pending_per_agent
                        ),
                    )
                })?;
                let summary = match &call {
                    Call::HostWrite { content, .. } => match host_access::existing_size(&real) {
                        Some(old) => format!("Replace the file {} ({old} bytes) with {} bytes", real.display(), content.len()),
                        None => format!("Create the file {} ({} bytes)", real.display(), content.len()),
                    },
                    Call::HostRead { .. } => format!("Read the file {}", real.display()),
                    _ => format!("List the folder {}", real.display()),
                };
                let timeout = self.approval_timeout.unwrap_or_else(|| agent::approval_timeout(&self.ctx));
                let (id, rx) = self.approvals.request(
                    approvals::Request {
                        vault: vault.info.id.clone(),
                        vault_name: vault.info.name.clone(),
                        agent: agent_name,
                        action: format!("host {}", action.as_str()),
                        page: None,
                        summary,
                        diff: None,
                        session: conn.session.clone(),
                        tool_title: None,
                        tool_kind: None,
                    },
                    timeout,
                );
                let this = Arc::clone(self);
                let mut closed = conn.closed.clone();
                let path_given = path;
                Ok(Box::pin(async move {
                    let _slot = slot;
                    let outcome = tokio::select! {
                        outcome = this.approvals.wait(id, rx, timeout) => outcome,
                        () = wait_closing(&mut closed) => {
                            this.approvals.cancel(id);
                            ApprovalOutcome::Cancelled
                        }
                    };
                    match outcome {
                        ApprovalOutcome::Approved => {
                            if *closed.borrow() {
                                log(true, "cancelled");
                                return Err(RpcError::new(rpc::CODE_SHUTTING_DOWN, "the agent went away before the action was done"));
                            }
                            // The user may have tightened host access while this
                            // waited, and the path may lead somewhere else now.
                            let settings = agent::get_host(&this.ctx, &vault.info.id);
                            let same = host_access::resolve(&path_given, action).ok().as_deref() == Some(real.as_path());
                            let verdict = if same {
                                host_access::decide(&settings, &this.host_guard(&vault), &real, action)
                            } else {
                                HostDecision::Refuse("the path changed while the request waited".into())
                            };
                            if let HostDecision::Refuse(why) = verdict {
                                log(true, "refused");
                                return Err(RpcError::new(rpc::CODE_FORBIDDEN, why));
                            }
                            let task = tokio::task::spawn_blocking({
                                let real = real.clone();
                                move || run_host(&real, &call)
                            });
                            let result = task.await.unwrap_or_else(|_| Err(RpcError::failed("the host call crashed")));
                            log(true, if result.is_ok() { "done" } else { "failed" });
                            result
                        }
                        ApprovalOutcome::Rejected => {
                            log(false, "rejected");
                            Err(RpcError::new(rpc::CODE_REJECTED, "the request was rejected by the user; nothing was touched"))
                        }
                        ApprovalOutcome::TimedOut => {
                            log(false, "timed_out");
                            Err(RpcError::new(
                                rpc::CODE_TIMED_OUT,
                                "nobody answered the approval request in time, so it counts as rejected; nothing was touched",
                            ))
                        }
                        ApprovalOutcome::Cancelled => {
                            log(false, "cancelled");
                            Err(RpcError::new(
                                rpc::CODE_SHUTTING_DOWN,
                                "the request was cancelled before it was approved (the daemon stopped or the agent went away); nothing was touched",
                            ))
                        }
                    }
                }))
            }
        }
    }

    fn begin_read(&self, vault: Arc<VaultHandle>, call: Call) -> Fut {
        let embed = Arc::clone(&self.embed);
        let task = tokio::task::spawn_blocking(move || read(&vault, &embed, call));
        Box::pin(async move { task.await.unwrap_or_else(|_| Err(RpcError::failed("the read crashed"))) })
    }

    /// A write. App and CLI connections go straight to the vault's queue; an
    /// agent's is judged by the vault's permission (design D6), read afresh so
    /// a change applies to connected agents at once.
    fn begin_write(self: &Arc<Self>, vault: Arc<VaultHandle>, call: Call, conn: &Conn) -> Result<Fut, RpcError> {
        let client = &conn.hello;
        if self.shutdown.is_triggered() {
            return Err(RpcError::new(rpc::CODE_SHUTTING_DOWN, "the daemon is stopping and takes no more writes"));
        }
        // A bad import is refused before a person is asked about it.
        if let Call::OntologyImport { url, prefix, namespace, .. } = &call {
            ops::check_ontology_import(&vault, url, prefix.as_deref(), namespace.as_deref())?;
        }
        // The text the caller says its edit was written against, if it said.
        let given = match &call {
            Call::PageEdit { base_version: Some(version), .. } => Some(ops::parse_version(version).ok_or_else(|| {
                RpcError::new(
                    rpc::CODE_BAD_REQUEST,
                    "base_version is not a page version: pass the `version` that read_page returned",
                )
            })?),
            _ => None,
        };
        if client.client != ClientKind::Agent {
            return self.queue_write(vault, call, client.client.into(), None, given, None);
        }
        let agent_name = client.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| "agent".to_string());
        // A call that reaches outside the machine asks a person even at `auto`.
        let level = match agent::get_permission(&self.ctx, &vault.info.id) {
            AgentPermission::Auto if call.always_asks() => AgentPermission::Approve,
            other => other,
        };
        match level {
            AgentPermission::Read => Err(read_only(&vault)),
            AgentPermission::Auto => {
                let closed = Some(conn.closed.clone());
                let log = AgentLog::new(agent_name, false, conn.session.clone());
                self.queue_write(vault, call, Origin::Agent, Some(log), given, closed)
            }
            AgentPermission::Approve => {
                // The cap is the session's (or, on the main socket, the
                // vault's), not the connection's: an agent can open many.
                let (key, whose) = match &conn.session {
                    Some(session) => (format!("session:{session}"), "this session"),
                    None => (format!("vault:{}", vault.info.id), "this vault"),
                };
                let slot = PendingSlot::take(&self.pending, key, self.max_pending_per_agent).ok_or_else(|| {
                    RpcError::new(
                        rpc::CODE_TOO_MANY_PENDING,
                        format!(
                            "{whose} already has {} changes waiting for approval, the most it may have; \
                             wait until some are answered",
                            self.max_pending_per_agent
                        ),
                    )
                })?;
                // What the page was when the agent read it, if it said; else
                // when it asked: the approved edit replaces the whole page, so
                // it must not land on another text.
                let base = given.or_else(|| match &call {
                    Call::PageEdit { page, .. } => ops::page_fingerprint(&vault, page),
                    _ => None,
                });
                let mut closed = conn.closed.clone();
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
                        session: conn.session.clone(),
                        tool_title: None,
                        tool_kind: None,
                    },
                    timeout,
                );
                let this = Arc::clone(self);
                let session = conn.session.clone();
                Ok(Box::pin(async move {
                    let _slot = slot;
                    // The agent hanging up ends the request: nobody can be
                    // told the answer, so it is never applied.
                    let outcome = tokio::select! {
                        outcome = this.approvals.wait(id, rx, timeout) => outcome,
                        () = wait_closing(&mut closed) => {
                            this.approvals.cancel(id);
                            ApprovalOutcome::Cancelled
                        }
                    };
                    match outcome {
                        ApprovalOutcome::Approved => {
                            let gone = *closed.borrow();
                            if gone {
                                return Err(RpcError::new(rpc::CODE_SHUTTING_DOWN, "the agent went away before the change was applied"));
                            }
                            // The user may have tightened the vault while this waited.
                            if !agent::get_permission(&this.ctx, &vault.info.id).can_write() {
                                return Err(read_only(&vault));
                            }
                            let log = Some(AgentLog::new(agent_name, true, session));
                            match this.queue_write(vault, call, Origin::Agent, log, base, Some(closed)) {
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
                        ApprovalOutcome::Cancelled => Err(RpcError::new(
                            rpc::CODE_SHUTTING_DOWN,
                            "the change was cancelled before it was approved (the daemon stopped or the agent went away); the vault is unchanged",
                        )),
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
        base: Option<u64>,
        closed: Option<watch::Receiver<bool>>,
    ) -> Result<Fut, RpcError> {
        let tracker = Arc::clone(&self.tracker);
        let ctx = self.ctx.clone();
        let job_vault = Arc::clone(&vault);
        let before_write = self.before_write.clone();
        let sessions = Arc::clone(&self.sessions);
        let queued = vault.queue.submit(move || {
            if let Some(hook) = &before_write {
                hook();
            }
            // The requester hanging up while this waited in the queue ends
            // the write: nobody can be told how it went, so it is not applied.
            // Checked here, right before writing, not only when it was queued.
            if closed.is_some_and(|closed| *closed.borrow()) {
                return Err(RpcError::new(
                    rpc::CODE_SHUTTING_DOWN,
                    "the agent went away before the change was applied, so it was not applied",
                ));
            }
            let id = job_vault.info.id.clone();
            let action = call.action().unwrap_or("change");
            // Attribute the file events before the files change.
            if let Some(page) = target_page(&call) {
                tracker.note(&id, &page, origin);
            }
            let Written { page, result } = write(&job_vault, call, base);
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
                        outcome: None,
                    };
                    if let Err(e) = agent::append_change(&ctx, &entry) {
                        eprintln!("cerboid: cannot record an agent change: {e}");
                    }
                    // The page change also goes into the feed of the session whose
                    // agent made it, once it is applied.
                    if result.is_ok()
                        && let Some(session) = &log.session
                    {
                        sessions.page_changed(session, page);
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
    /// The agent session the change was made in, if any.
    session: Option<String>,
}

impl AgentLog {
    fn new(agent: String, approved: bool, session: Option<String>) -> Self {
        Self { agent, approved, session }
    }
}

/// Do the file operation of a host call on the resolved path.
fn run_host(real: &Path, call: &Call) -> OpResult {
    let done = match call {
        Call::HostList { .. } => host_access::list_dir(real),
        Call::HostRead { offset, limit, .. } => host_access::read_file(real, *offset, *limit),
        Call::HostWrite { content, .. } => host_access::write_file(real, content),
        _ => return Err(RpcError::failed("not a host call")),
    };
    done.map_err(RpcError::failed)
}

/// Record a host action in the agent change log.
fn log_host(
    ctx: &CerboContext,
    vault: &str,
    agent: &str,
    action: HostAction,
    real: &Path,
    approved: bool,
    outcome: &str,
) {
    let entry = ChangeEntry {
        time: approvals::now_secs(),
        vault: vault.to_string(),
        page: real.display().to_string(),
        agent: agent.to_string(),
        action: format!("host_{}", action.as_str()),
        approved,
        outcome: Some(outcome.to_string()),
    };
    if let Err(e) = agent::append_change(ctx, &entry) {
        eprintln!("cerboid: cannot record a host action: {e}");
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
        Call::IndexStatus { .. } => ops::index_status(vault),
        Call::TrashList { .. } => ops::trash_list(vault),
        Call::OntologyList { .. } => ops::ontology_list(vault),
        Call::OntologyTypes { prefix, .. } => ops::ontology_types(vault, &prefix),
        other => Err(RpcError::failed(format!("not a read: {other:?}"))),
    }
}

fn write(vault: &VaultHandle, call: Call, base: Option<u64>) -> Written {
    match call {
        Call::PageCreate { title, content, path, .. } => ops::page_create(vault, title, content, path),
        Call::PageEdit { page, content, .. } => ops::page_edit_if(vault, &page, content, base),
        Call::PageRename { page, title, .. } => ops::page_rename(vault, &page, title),
        Call::PageMove { page, path, .. } => ops::page_move(vault, &page, path),
        Call::PageTrash { page, .. } => ops::page_trash(vault, &page),
        Call::LinkAdd { from, to, .. } => ops::link_add(vault, &from, &to),
        Call::IndexRebuild { .. } => ops::index_rebuild(vault),
        Call::PageRestore { page, .. } => ops::page_restore(vault, &page),
        Call::OntologyImport { url, prefix, namespace, .. } => {
            ops::ontology_import(vault, &url, prefix.as_deref(), namespace.as_deref())
        }
        Call::OntologyCreate { prefix, title, .. } => ops::ontology_create(vault, &prefix, title.as_deref()),
        Call::OntologyAddType { prefix, name, label, comment, parent, .. } => {
            ops::ontology_add_type(vault, &prefix, &name, label.as_deref(), comment.as_deref(), parent.as_deref())
        }
        other => Written { page: None, result: Err(RpcError::failed(format!("not a write: {other:?}"))) },
    }
}
