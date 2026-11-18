//! The desktop's link to the daemon, for agent sessions only (change
//! `add-agent-panel`, task group 1c; a subset of `add-cerboid-daemon` 7.1-7.2).
//!
//! Page and index commands stay in-process (that move is `add-cerboid-daemon`
//! 7.3 and later). This module keeps two kinds of connection to the daemon:
//!
//! - one **event connection**, owned by a thread that connects (starting the
//!   daemon when none answers), subscribes, forwards what it hears to the
//!   webview and, when the connection breaks, reconnects with backoff;
//! - a small **pool** of call connections, used by the Tauri commands, so a
//!   slow call (starting an agent) never holds up an approval answer.
//!
//! The webview hears two Tauri events: `agent-connection` (a
//! [`ConnectionStatus`]) and `agent-event` (a daemon [`Event`] of the kinds
//! `session`, `approval_requested`, `approval_resolved`, `resync`). Everything else the
//! daemon says is of no interest to the Agent tab and is dropped here.

use crate::get_context;
use cerbo_core::agent as agent_registry;
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::client::Client;
use cerbo_core::rpc::launch::connect_or_start;
use cerbo_core::rpc::{AgentKind, Call, ClientKind, Event, PageRef, RpcError, CODE_CLIENT_TIMEOUT};
use serde::Serialize;
use serde_json::Value;
use std::str::FromStr;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

/// Tauri event: the connection to the daemon changed ([`ConnectionStatus`]).
pub const CONNECTION_EVENT: &str = "agent-connection";
/// Tauri event: the daemon sent a session or approval [`Event`].
pub const EVENT_EVENT: &str = "agent-event";

/// The name the daemon logs and shows in approval prompts for this client.
const CLIENT_NAME: &str = "cerbo-desktop";

/// How the event thread paces itself.
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    /// First wait after a failed connection.
    pub backoff_start: Duration,
    /// The wait never grows beyond this.
    pub backoff_max: Duration,
    /// How long the pump waits for an event before it checks the connection.
    pub tick: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            backoff_start: Duration::from_millis(500),
            backoff_max: Duration::from_secs(10),
            tick: Duration::from_secs(2),
        }
    }
}

/// The wait after a connection that failed after `previous`.
fn next_backoff(previous: Duration, timing: &Timing) -> Duration {
    (previous * 2).min(timing.backoff_max)
}

/// Whether the desktop can talk to the daemon right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectionStatus {
    /// `connecting`, `connected` or `unavailable`.
    pub state: &'static str,
    /// Why the daemon is unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daemon_version: Option<String>,
}

impl ConnectionStatus {
    fn connecting() -> Self {
        Self { state: "connecting", message: None, daemon_version: None }
    }
    fn connected(version: String) -> Self {
        Self { state: "connected", message: None, daemon_version: Some(version) }
    }
    fn unavailable(message: String) -> Self {
        Self { state: "unavailable", message: Some(message), daemon_version: None }
    }
}

/// What the event thread reports.
#[derive(Debug, Clone, PartialEq)]
pub enum Notice {
    Connection(ConnectionStatus),
    Event(Event),
}

type Connector = Arc<dyn Fn(&DaemonPaths) -> Result<Client, RpcError> + Send + Sync>;
type Sink = Arc<dyn Fn(Notice) + Send + Sync>;

struct Shared {
    paths: DaemonPaths,
    connect: Connector,
    status: Mutex<ConnectionStatus>,
    /// Idle call connections.
    pool: Mutex<Vec<Client>>,
    /// Wakes the event thread out of its backoff wait.
    wake: Mutex<Sender<()>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The managed state of the app: one link to the daemon.
#[derive(Clone)]
pub struct DaemonLink {
    shared: Arc<Shared>,
}

impl DaemonLink {
    /// Connect to the daemon at `paths` with `connect`, report to `sink`, and
    /// keep doing so until the process ends. Returns at once.
    pub fn spawn(paths: DaemonPaths, connect: Connector, sink: Sink, timing: Timing) -> Self {
        let (wake_tx, wake_rx) = channel();
        let shared = Arc::new(Shared {
            paths,
            connect,
            status: Mutex::new(ConnectionStatus::connecting()),
            pool: Mutex::new(Vec::new()),
            wake: Mutex::new(wake_tx),
        });
        let thread_shared = shared.clone();
        std::thread::Builder::new()
            .name("daemon-events".into())
            .spawn(move || event_loop(thread_shared, sink, wake_rx, timing))
            .expect("spawn the daemon event thread");
        Self { shared }
    }

    /// The link of the real app: the user's daemon, started if absent, with
    /// the webview as the sink.
    pub fn for_app(app: &AppHandle) -> Result<Self, String> {
        let ctx = get_context(app)?;
        let paths = DaemonPaths::for_user(&ctx.cache_dir);
        let connect: Connector =
            Arc::new(|paths| connect_or_start(paths, ClientKind::App, Some(CLIENT_NAME), None));
        let handle = app.clone();
        let sink: Sink = Arc::new(move |notice| {
            let result = match &notice {
                Notice::Connection(status) => handle.emit(CONNECTION_EVENT, status),
                Notice::Event(event) => handle.emit(EVENT_EVENT, event),
            };
            if let Err(e) = result {
                eprintln!("Warning: could not tell the window about the daemon: {e}");
            }
        });
        Ok(Self::spawn(paths, connect, sink, Timing::default()))
    }

    pub fn status(&self) -> ConnectionStatus {
        lock(&self.shared.status).clone()
    }

    /// Skip the backoff wait: connect again now.
    pub fn retry(&self) {
        // A closed channel means the thread is gone, which only happens at exit.
        let _ = lock(&self.shared.wake).send(());
    }

    /// One call on a pooled connection (a new one when none is idle).
    pub fn call(&self, call: Call) -> Result<Value, RpcError> {
        let pooled = lock(&self.shared.pool).pop();
        let mut client = match pooled {
            Some(client) => client,
            None => (self.shared.connect)(&self.shared.paths)?,
        };
        match client.call(call) {
            Ok(value) => {
                lock(&self.shared.pool).push(client);
                Ok(value)
            }
            Err(error) => {
                // The daemon answered (a refusal, an unknown session): the
                // connection is fine. Anything else may have broken it.
                if !is_transport_failure(&error) {
                    lock(&self.shared.pool).push(client);
                }
                Err(error)
            }
        }
    }
}

/// An error that says the connection itself failed, not the call.
fn is_transport_failure(error: &RpcError) -> bool {
    error.code == CODE_CLIENT_TIMEOUT
        || error.message.starts_with("send:")
        || error.message == "the daemon closed the connection"
}

fn set_status(shared: &Shared, sink: &Sink, status: ConnectionStatus) {
    {
        let mut current = lock(&shared.status);
        if *current == status {
            return;
        }
        *current = status.clone();
    }
    sink(Notice::Connection(status));
}

/// Whether the Agent tab wants to hear of `event`.
fn forwarded(event: &Event) -> bool {
    matches!(event, Event::Session { .. } | Event::ApprovalRequested { .. } | Event::ApprovalResolved { .. } | Event::Resync)
}

fn event_loop(shared: Arc<Shared>, sink: Sink, wake: Receiver<()>, timing: Timing) {
    let mut delay = timing.backoff_start;
    loop {
        set_status(&shared, &sink, ConnectionStatus::connecting());
        let failure = connect_and_pump(&shared, &sink, &timing);
        // Connections made before the break may be dead as well.
        lock(&shared.pool).clear();
        set_status(&shared, &sink, ConnectionStatus::unavailable(failure));
        match wake.recv_timeout(delay) {
            Ok(()) => delay = timing.backoff_start,
            Err(RecvTimeoutError::Timeout) => delay = next_backoff(delay, &timing),
            Err(RecvTimeoutError::Disconnected) => return,
        }
        // Retries queued while we waited count once.
        while wake.try_recv().is_ok() {}
    }
}

/// Connect, subscribe and forward events until the connection breaks or
/// cannot be made; the answer says why. The caller handles backoff.
fn connect_and_pump(shared: &Shared, sink: &Sink, timing: &Timing) -> String {
    pump(shared, sink, timing).unwrap_err_or_else()
}

/// `Result` with no success type: the pump only ever ends with a reason.
trait ReasonOnly {
    fn unwrap_err_or_else(self) -> String;
}

impl ReasonOnly for Result<std::convert::Infallible, String> {
    fn unwrap_err_or_else(self) -> String {
        match self {
            Ok(never) => match never {},
            Err(reason) => reason,
        }
    }
}

fn pump(shared: &Shared, sink: &Sink, timing: &Timing) -> Result<std::convert::Infallible, String> {
    let mut client = (shared.connect)(&shared.paths).map_err(|e| e.message)?;
    client.call(Call::Subscribe { vault: None }).map_err(|e| format!("subscribe: {}", e.message))?;
    set_status(shared, sink, ConnectionStatus::connected(client.welcome().daemon_version.clone()));
    loop {
        match client.next_event(timing.tick) {
            Ok(Some(event)) => {
                if forwarded(&event) {
                    sink(Notice::Event(event));
                }
            }
            // Nothing came: either quiet, or the daemon is gone (the reader
            // cannot tell the two apart), so ask.
            Ok(None) => {
                client
                    .call_with_timeout(Call::Ping, Duration::from_secs(5))
                    .map_err(|e| format!("the daemon stopped answering: {}", e.message))?;
            }
            Err(e) => return Err(e),
        }
    }
}

// ── Tauri commands ───────────────────────────────────────────────────────────

/// What a command reports on failure: the daemon's code and message, so the
/// webview can tell "no daemon" from "the daemon said no".
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl From<RpcError> for CommandError {
    fn from(e: RpcError) -> Self {
        Self { code: e.code, message: e.message }
    }
}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self { code: "failed".into(), message }
    }
}

async fn call(link: State<'_, DaemonLink>, call: Call) -> Result<Value, CommandError> {
    let link = link.inner().clone();
    tauri::async_runtime::spawn_blocking(move || link.call(call))
        .await
        .map_err(|e| CommandError::from(format!("the call did not finish: {e}")))?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn agent_list(link: State<'_, DaemonLink>) -> Result<Value, CommandError> {
    call(link, Call::AgentList {}).await
}

#[tauri::command]
pub async fn session_create(
    link: State<'_, DaemonLink>,
    vault: String,
    agent: AgentKind,
    resume_from: Option<String>,
) -> Result<Value, CommandError> {
    call(link, Call::SessionCreate { vault, agent, resume_from, ephemeral: false }).await
}

#[tauri::command]
pub async fn session_send(
    link: State<'_, DaemonLink>,
    session: String,
    text: String,
    context: Vec<PageRef>,
) -> Result<Value, CommandError> {
    call(link, Call::SessionSend { session, text, context }).await
}

#[tauri::command]
pub async fn session_cancel(link: State<'_, DaemonLink>, session: String) -> Result<Value, CommandError> {
    call(link, Call::SessionCancel { session }).await
}

#[tauri::command]
pub async fn session_end(link: State<'_, DaemonLink>, session: String) -> Result<Value, CommandError> {
    call(link, Call::SessionEnd { session }).await
}

#[tauri::command]
pub async fn session_list(link: State<'_, DaemonLink>, vault: Option<String>) -> Result<Value, CommandError> {
    call(link, Call::SessionList { vault }).await
}

#[tauri::command]
pub async fn session_delete(link: State<'_, DaemonLink>, session: String) -> Result<Value, CommandError> {
    call(link, Call::SessionDelete { session }).await
}

#[tauri::command]
pub async fn session_events(
    link: State<'_, DaemonLink>,
    session: String,
    after_seq: Option<u64>,
    limit: usize,
) -> Result<Value, CommandError> {
    call(link, Call::SessionEvents { session, after_seq, limit }).await
}

#[tauri::command]
pub async fn approval_list(link: State<'_, DaemonLink>, vault: Option<String>) -> Result<Value, CommandError> {
    call(link, Call::ApprovalList { vault }).await
}

#[tauri::command]
pub async fn approval_answer(
    link: State<'_, DaemonLink>,
    approval: u64,
    approve: bool,
) -> Result<Value, CommandError> {
    call(link, Call::ApprovalAnswer { approval, approve }).await
}

/// The agent permission of a vault: `read`, `auto` or `approve`. Read from
/// the registry file, as `cerbo vault agent` does: the daemon looks at the
/// file on its next write, so there is no call for it.
#[tauri::command]
pub fn vault_agent_permission_get(app: AppHandle, vault_id: String) -> Result<String, CommandError> {
    let ctx = get_context(&app)?;
    known_vault(&ctx, &vault_id)?;
    Ok(agent_registry::get_permission(&ctx, &vault_id).as_str().to_string())
}

/// Set the agent permission of a vault; answers the level now in force.
#[tauri::command]
pub fn vault_agent_permission_set(
    app: AppHandle,
    vault_id: String,
    level: String,
) -> Result<String, CommandError> {
    let ctx = get_context(&app)?;
    known_vault(&ctx, &vault_id)?;
    let level = agent_registry::AgentPermission::from_str(&level)?;
    agent_registry::set_permission(&ctx, &vault_id, level)?;
    Ok(agent_registry::get_permission(&ctx, &vault_id).as_str().to_string())
}

fn known_vault(ctx: &cerbo_core::CerboContext, vault_id: &str) -> Result<(), CommandError> {
    cerbo_core::vault::get_vault_path(ctx, vault_id)
        .map(|_| ())
        .ok_or_else(|| CommandError::from(format!("vault not found: {vault_id}")))
}

#[tauri::command]
pub fn daemon_status(link: State<'_, DaemonLink>) -> ConnectionStatus {
    link.status()
}

#[tauri::command]
pub fn daemon_retry(link: State<'_, DaemonLink>) -> ConnectionStatus {
    link.retry();
    link.status()
}

/// Start the link at app start. A failure to find the user's directories only
/// means the Agent tab shows no daemon; the app runs on.
pub fn start(app: &AppHandle) {
    match DaemonLink::for_app(app) {
        Ok(link) => {
            app.manage(link);
        }
        Err(e) => {
            eprintln!("Warning: no link to the daemon: {e}");
            let paths = DaemonPaths::in_dir(std::env::temp_dir().join("cerbo-no-daemon"));
            let failure = e.clone();
            let connect: Connector = Arc::new(move |_| Err(RpcError::failed(failure.clone())));
            let sink: Sink = Arc::new(|_| {});
            app.manage(DaemonLink::spawn(paths, connect, sink, Timing::default()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cerbo_core::rpc::{
        ApprovalOutcome, ClientHello, Request, ServerMessage, SessionEvent, SessionEventBody, PROTOCOL_VERSION,
    };
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;

    fn fast() -> Timing {
        Timing {
            backoff_start: Duration::from_millis(20),
            backoff_max: Duration::from_millis(80),
            tick: Duration::from_millis(40),
        }
    }

    fn session_event(seq: u64) -> Event {
        Event::Session {
            vault: "v".into(),
            session: "s".into(),
            event: SessionEvent {
                seq,
                at: "2026-01-01T00:00:00Z".into(),
                body: SessionEventBody::AgentText { text: format!("hello {seq}") },
            },
        }
    }

    fn write_line<T: Serialize>(stream: &mut std::os::unix::net::UnixStream, message: &T) {
        let mut line = serde_json::to_vec(message).unwrap();
        line.push(b'\n');
        stream.write_all(&line).unwrap();
    }

    /// A daemon that welcomes each client, answers every call with null and,
    /// after the answer to `subscribe`, sends `events`. The first connection
    /// is dropped once the events are sent; later ones stay open.
    fn fake_daemon(listener: UnixListener, events: Vec<Event>) {
        std::thread::spawn(move || {
            for (n, stream) in listener.incoming().enumerate() {
                let Ok(mut stream) = stream else { return };
                let events = events.clone();
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    let _hello: ClientHello = serde_json::from_str(line.trim_end()).unwrap();
                    write_line(
                        &mut stream,
                        &ServerMessage::Welcome {
                            protocol: PROTOCOL_VERSION,
                            daemon_version: "fake".into(),
                            pid: 1,
                        },
                    );
                    loop {
                        line.clear();
                        if reader.read_line(&mut line).unwrap_or(0) == 0 {
                            return;
                        }
                        let request: Request = serde_json::from_str(line.trim_end()).unwrap();
                        write_line(&mut stream, &ServerMessage::ok(request.id, Value::Null));
                        if matches!(request.call, Call::Subscribe { .. }) {
                            for event in &events {
                                write_line(&mut stream, &ServerMessage::Event { event: event.clone() });
                            }
                            if n == 0 {
                                return; // drop the connection: the daemon "restarts"
                            }
                        }
                    }
                });
            }
        });
    }

    fn collect(link_notices: &Receiver<Notice>, count: usize) -> Vec<Notice> {
        (0..count)
            .map(|_| link_notices.recv_timeout(Duration::from_secs(5)).expect("a notice in time"))
            .collect()
    }

    #[test]
    fn forwards_events_and_reconnects_after_the_daemon_drops() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DaemonPaths::in_dir(dir.path().to_path_buf());
        let listener = UnixListener::bind(&paths.socket).unwrap();
        fake_daemon(listener, vec![session_event(1)]);

        let (tx, rx) = channel();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |notice| {
            let _ = lock(&tx).send(notice);
        });
        let connect: Connector =
            Arc::new(|paths| Client::connect(&paths.socket, ClientKind::App, Some("test")).map_err(|e| RpcError::failed(e.to_string())));
        let link = DaemonLink::spawn(paths, connect, sink, fast());

        // The first "connecting" is the initial status and is not announced.
        let notices = collect(&rx, 6);
        let states: Vec<&str> = notices
            .iter()
            .map(|n| match n {
                Notice::Connection(c) => c.state,
                Notice::Event(_) => "event",
            })
            .collect();
        assert_eq!(states, ["connected", "event", "unavailable", "connecting", "connected", "event"]);
        assert_eq!(link.status().state, "connected");
    }

    #[test]
    fn calls_use_a_second_connection_and_return_the_answer() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DaemonPaths::in_dir(dir.path().to_path_buf());
        let listener = UnixListener::bind(&paths.socket).unwrap();
        fake_daemon(listener, vec![]);
        let connect: Connector =
            Arc::new(|paths| Client::connect(&paths.socket, ClientKind::App, Some("test")).map_err(|e| RpcError::failed(e.to_string())));
        let link = DaemonLink::spawn(paths, connect, Arc::new(|_| {}), fast());
        assert_eq!(link.call(Call::Ping).unwrap(), Value::Null);
        assert_eq!(link.call(Call::Ping).unwrap(), Value::Null);
        assert_eq!(lock(&link.shared.pool).len(), 1, "the connection is reused");
    }

    #[test]
    fn an_unreachable_daemon_is_reported_and_retried() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DaemonPaths::in_dir(dir.path().to_path_buf());
        let connect: Connector = Arc::new(|_| Err(RpcError::failed("no daemon here")));
        let (tx, rx) = channel();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |notice| {
            let _ = lock(&tx).send(notice);
        });
        let link = DaemonLink::spawn(paths, connect, sink, fast());
        let first = collect(&rx, 1);
        assert_eq!(first, vec![Notice::Connection(ConnectionStatus::unavailable("no daemon here".into()))]);
        link.retry();
        assert_eq!(link.status().state, "unavailable");
        assert!(link.call(Call::Ping).is_err());
    }

    #[test]
    fn backoff_doubles_up_to_the_maximum() {
        let t = Timing::default();
        let mut d = t.backoff_start;
        let mut seen = vec![d];
        for _ in 0..6 {
            d = next_backoff(d, &t);
            seen.push(d);
        }
        assert_eq!(seen[1], Duration::from_secs(1));
        assert_eq!(*seen.last().unwrap(), Duration::from_secs(10));
    }

    #[test]
    fn only_session_and_approval_events_are_forwarded() {
        assert!(forwarded(&session_event(1)));
        assert!(forwarded(&Event::ApprovalResolved { vault: "v".into(), approval: 137, outcome: ApprovalOutcome::Approved }));
        assert!(!forwarded(&Event::VaultsChanged));
        assert!(forwarded(&Event::Resync), "a window that fell behind must be told");
    }

    #[test]
    fn transport_failures_are_told_from_refusals() {
        assert!(is_transport_failure(&RpcError::failed("the daemon closed the connection")));
        assert!(is_transport_failure(&RpcError::new(CODE_CLIENT_TIMEOUT, "slow")));
        assert!(!is_transport_failure(&RpcError::failed("no such session")));
    }
}
