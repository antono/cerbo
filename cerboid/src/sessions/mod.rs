//! Agent sessions (design D1-D7, D9): the daemon starts an agent CLI in its
//! sandbox, speaks ACP to it, keeps the feed of what happens in a store outside
//! the vault and tells subscribed clients.
//!
//! A [`Manager`] owns every session of the daemon. A session is a stored feed
//! ([`store`]) plus, while its agent runs, a [`Live`]: the ACP client, the
//! session's own socket and the task that turns what the agent says into
//! feed events. Windows and the CLI are viewers: they call the session API of
//! the daemon and listen to [`Event::Session`].
//!
//! One task per live session ("the driver") is the only writer of the agent's
//! output into the feed, so the feed has the order the agent spoke in, and a
//! turn's end always comes after its last chunk.

pub mod permission;
pub mod store;

use crate::acp::{
    AcpClient, AcpError, AcpEvent, AcpEvents, AgentCapabilities, ClientOptions, ContentBlock, McpServer, PermissionRequest,
    ResourceContents, SessionNotification, SessionUpdate, StopReason, ToolCallContent,
};
use crate::agents::{self, AgentRegistry, HostEnv};
use crate::approvals::{self, Approvals};
use crate::sandbox::{self, Confinement, Launch, RealFs, SessionSpec};
use cerbo_core::agent;
use cerbo_core::daemon_paths::DaemonPaths;
use cerbo_core::rpc::{
    self, AgentKind, ApprovalOutcome, Event, PageRef, ResumeHow, RpcError, SessionEvent, SessionEventBody, SessionInfo, SessionState,
};
use cerbo_core::CerboContext;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use store::{LogWriter, Meta, Store};
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;

/// How long a found agent list is trusted before the next `agent_list` looks
/// again (an agent may have been installed meanwhile).
const REGISTRY_TTL: Duration = Duration::from_secs(30);
/// The title of a session before its first message.
const DEFAULT_TITLE: &str = "New session";
/// The longest title taken from a first message, in characters.
const TITLE_CHARS: usize = 80;
/// How much of an earlier conversation is handed to an agent that cannot load
/// its own session, in characters (the end of it is kept).
const TRANSCRIPT_CHARS: usize = 12_000;
/// The longest tool input or output shown in a feed row, in characters.
const SUMMARY_CHARS: usize = 400;

/// The parts of the environment the session manager looks at; tests give
/// their own.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The real user's environment: `PATH`, home, data directory.
    pub host: HostEnv,
    /// `CERBO_BWRAP`, the bubblewrap to use.
    pub bwrap_env: Option<OsString>,
    /// `CERBO_BIN`, the `cerbo` an agent runs as its MCP server.
    pub cerbo_bin_env: Option<OsString>,
    /// The daemon's own executable (to find `cerbo` beside it).
    pub current_exe: Option<PathBuf>,
    /// The directory of the per-session socket directories.
    pub sockets_dir: PathBuf,
}

impl Settings {
    pub fn from_process(paths: &DaemonPaths) -> Self {
        Self {
            host: HostEnv::from_process(),
            bwrap_env: std::env::var_os("CERBO_BWRAP"),
            cerbo_bin_env: std::env::var_os("CERBO_BIN"),
            current_exe: std::env::current_exe().ok(),
            sockets_dir: paths.dir.join("sessions"),
        }
    }

    fn data_root(&self) -> Result<PathBuf, String> {
        self.host
            .data_home()
            .map(|d| d.join("cerbo"))
            .ok_or_else(|| "neither XDG_DATA_HOME nor HOME is set: no place for the session store".to_string())
    }
}

/// The vault a session works in.
#[derive(Debug, Clone)]
pub struct VaultRef {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
}

/// A session's socket: while it lives the listener runs; dropping it stops
/// the listener, closes its connections and removes the socket directory.
pub struct SocketGuard {
    dir: PathBuf,
    task: JoinHandle<()>,
    close: watch::Sender<bool>,
}

impl SocketGuard {
    pub fn new(dir: PathBuf, task: JoinHandle<()>, close: watch::Sender<bool>) -> Self {
        Self { dir, task, close }
    }
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        self.close.send_replace(true);
        self.task.abort();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Opens the socket of a session: `(session id, vault id, socket directory)`.
/// The daemon's server provides it; the directory is created by it.
pub type SocketOpener<'a> = dyn Fn(&str, &str, &Path) -> Result<SocketGuard, String> + Send + Sync + 'a;

// ── Sessions ─────────────────────────────────────────────────────────────────

struct Inner {
    meta: Meta,
    log: LogWriter,
}

struct Session {
    id: String,
    vault: String,
    vault_name: String,
    agent: AgentKind,
    inner: Mutex<Inner>,
    live: Mutex<Option<Arc<Live>>>,
    /// Set when the session is being ended on purpose, so the agent's exit is
    /// not reported as a failure.
    ending: AtomicBool,
}

impl Session {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn info(&self) -> SessionInfo {
        self.lock().meta.info()
    }

    fn live(&self) -> Option<Arc<Live>> {
        self.live.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// What runs while a session's agent does.
struct Live {
    client: Arc<AcpClient>,
    /// The agent's own id of the session.
    acp_session: String,
    caps: AgentCapabilities,
    socket: Mutex<Option<SocketGuard>>,
    /// Tool permission cards of this session that wait for a person.
    tool_approvals: Mutex<HashSet<u64>>,
    /// A transcript of an earlier conversation, sent with the next message
    /// (resume without `session/load`).
    context: Mutex<Option<String>>,
    done: mpsc::UnboundedSender<PromptDone>,
}

enum PromptDone {
    Finished(Result<StopReason, AcpError>),
}

/// Why a session did not start, for the feed.
struct StartFailure {
    reason: String,
    stderr_tail: Option<String>,
}

impl StartFailure {
    fn of(error: &AcpError, client: Option<&AcpClient>) -> Self {
        let (reason, tail) = match error {
            AcpError::Exited(info) => {
                let text = info.to_string();
                let reason = text.split("; stderr:").next().unwrap_or(&text).to_string();
                (reason, Some(info.stderr_tail.clone()))
            }
            other => (other.to_string(), client.map(AcpClient::stderr_tail)),
        };
        Self { reason, stderr_tail: tail.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) }
    }

    fn plain(reason: impl Into<String>) -> Self {
        Self { reason: reason.into(), stderr_tail: None }
    }
}

/// The session manager of one daemon.
pub struct Manager {
    settings: Settings,
    ctx: CerboContext,
    approvals: Arc<Approvals>,
    events: broadcast::Sender<Event>,
    approval_timeout: Option<Duration>,
    store: Store,
    registry: Mutex<Option<(Instant, Arc<AgentRegistry>)>>,
    sessions: Mutex<HashMap<String, Arc<Session>>>,
    stopping: AtomicBool,
}

fn failed(message: impl Into<String>) -> RpcError {
    RpcError::failed(message)
}

fn login_hint(kind: AgentKind) -> &'static str {
    match kind {
        AgentKind::Opencode => "run `opencode auth login` in a terminal",
        AgentKind::Claude => "run `claude` in a terminal and sign in with /login",
    }
}

fn agent_title(kind: AgentKind) -> &'static str {
    match kind {
        AgentKind::Opencode => "OpenCode",
        AgentKind::Claude => "Claude Code",
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push('…');
    out
}

fn summarize_value(value: &Value) -> String {
    let text = match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    truncate(&text, SUMMARY_CHARS)
}

/// The title of a session from its first message: the first non-empty line.
pub fn title_of(text: &str) -> String {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    if line.is_empty() { DEFAULT_TITLE.to_string() } else { truncate(line, TITLE_CHARS) }
}

/// A text summary of a stored feed, to give a new agent session as context:
/// the messages in order, the end of it when long.
pub fn transcript_summary(events: &[SessionEvent]) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut agent_open = false;
    for event in events {
        match &event.body {
            SessionEventBody::UserMessage { text, .. } => {
                lines.push(format!("User: {text}"));
                agent_open = false;
            }
            SessionEventBody::AgentText { text } => {
                if agent_open {
                    if let Some(last) = lines.last_mut() {
                        last.push_str(text);
                    }
                } else {
                    lines.push(format!("Agent: {text}"));
                    agent_open = true;
                }
            }
            SessionEventBody::PageChanged { path } => {
                lines.push(format!("(the agent changed the page {path})"));
                agent_open = false;
            }
            _ => {}
        }
    }
    let joined = lines.join("\n");
    let count = joined.chars().count();
    let body = if count > TRANSCRIPT_CHARS {
        let tail: String = joined.chars().skip(count - TRANSCRIPT_CHARS).collect();
        format!("…{tail}")
    } else {
        joined
    };
    format!("This is the earlier conversation of this session, kept for context. Continue from it.\n\n{body}\n\n---\n")
}

impl Manager {
    pub fn new(
        settings: Settings,
        ctx: CerboContext,
        approvals: Arc<Approvals>,
        events: broadcast::Sender<Event>,
        approval_timeout: Option<Duration>,
    ) -> Arc<Self> {
        let store_root = settings.data_root().map(|d| d.join("sessions")).unwrap_or_else(|_| settings.sockets_dir.join("no-store"));
        let manager = Arc::new(Self {
            settings,
            ctx,
            approvals: Arc::clone(&approvals),
            events,
            approval_timeout,
            store: Store::new(store_root),
            registry: Mutex::new(None),
            sessions: Mutex::new(HashMap::new()),
            stopping: AtomicBool::new(false),
        });
        // Approval cards of a session go into its feed from here, whoever
        // asked: the agent's tools (this module) or its vault writes (server).
        let weak: Weak<Manager> = Arc::downgrade(&manager);
        approvals.set_observer(Box::new(move |event, session| {
            let (Some(manager), Some(session)) = (weak.upgrade(), session) else { return };
            let Some(session) = manager.known(session) else { return };
            match event {
                Event::ApprovalRequested { approval } => {
                    manager.emit(&session, SessionEventBody::ApprovalRequested { approval: Box::new(approval.clone()) });
                }
                Event::ApprovalResolved { approval, outcome, .. } => {
                    manager.emit(&session, SessionEventBody::ApprovalResolved { approval_id: *approval, outcome: *outcome });
                }
                _ => {}
            }
        }));
        manager
    }

    fn known(&self, id: &str) -> Option<Arc<Session>> {
        self.sessions.lock().unwrap_or_else(|e| e.into_inner()).get(id).cloned()
    }

    fn approval_wait(&self) -> Duration {
        self.approval_timeout.unwrap_or_else(|| agent::approval_timeout(&self.ctx))
    }

    // ── The feed ─────────────────────────────────────────────────────────────

    /// Append an event to a session's feed and tell subscribers; the two happen
    /// under one lock, so subscribers see the feed's order.
    fn emit(&self, session: &Session, body: SessionEventBody) -> Option<SessionEvent> {
        let mut inner = session.lock();
        let persist_meta = matches!(
            body,
            SessionEventBody::UserMessage { .. } | SessionEventBody::TurnEnded { .. } | SessionEventBody::State { .. } | SessionEventBody::Resumed { .. }
        );
        if let SessionEventBody::UserMessage { text, .. } = &body
            && inner.meta.title == DEFAULT_TITLE
        {
            inner.meta.title = title_of(text);
        }
        let event = match inner.log.append(body) {
            Ok(event) => event,
            Err(e) => {
                eprintln!("cerboid: cannot write the feed of session {}: {e}", session.id);
                return None;
            }
        };
        inner.meta.updated_at = event.at.clone();
        if persist_meta && let Err(e) = self.store.write_meta(&inner.meta) {
            eprintln!("cerboid: cannot write the index of session {}: {e}", session.id);
        }
        let _ = self.events.send(Event::Session { vault: session.vault.clone(), session: session.id.clone(), event: event.clone() });
        Some(event)
    }

    fn set_state(&self, session: &Session, state: SessionState, reason: Option<String>, stderr_tail: Option<String>) {
        {
            let mut inner = session.lock();
            if inner.meta.state == state && reason.is_none() {
                return;
            }
            inner.meta.state = state;
        }
        self.emit(session, SessionEventBody::State { state, reason, stderr_tail });
    }

    /// An agent write applied to a page, from the session's own connection.
    pub fn page_changed(&self, session: &str, page: &str) {
        if let Some(session) = self.known(session) {
            self.emit(&session, SessionEventBody::PageChanged { path: page.to_string() });
        }
    }

    // ── The registry ─────────────────────────────────────────────────────────

    async fn registry(self: &Arc<Self>, fresh: bool) -> Result<Arc<AgentRegistry>, RpcError> {
        if let Some((at, registry)) = self.registry.lock().unwrap_or_else(|e| e.into_inner()).clone()
            && !fresh
            && at.elapsed() < REGISTRY_TTL
        {
            return Ok(registry);
        }
        let this = Arc::clone(self);
        let detected = tokio::task::spawn_blocking(move || {
            let probe_home = this.settings.data_root()?.join("agents").join("_probe");
            let section = agent::agent_section(&this.ctx);
            Ok::<_, String>(Arc::new(AgentRegistry::detect(&section, &this.settings.host, &probe_home)))
        })
        .await
        .map_err(|_| failed("looking for the agents crashed"))?
        .map_err(failed)?;
        *self.registry.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), Arc::clone(&detected)));
        Ok(detected)
    }

    /// The answer of `agent_list`.
    pub async fn agent_list(self: &Arc<Self>) -> Result<Value, RpcError> {
        let registry = self.registry(true).await?;
        serde_json::to_value(registry.infos()).map_err(|e| failed(e.to_string()))
    }

    // ── Create, resume ───────────────────────────────────────────────────────

    /// Start a session in `vault`, or, with `resume_from`, continue an earlier
    /// one of the same vault. Returns once the agent is ready (or has failed:
    /// then the session is `failed` and its feed says why). A session that
    /// cannot be tried at all (agent missing or not signed in, no sandbox) is an
    /// error and leaves nothing behind.
    pub async fn create(
        self: &Arc<Self>,
        vault: VaultRef,
        agent: AgentKind,
        resume_from: Option<String>,
        open_socket: &SocketOpener<'_>,
    ) -> Result<SessionInfo, RpcError> {
        if self.stopping.load(Ordering::SeqCst) {
            return Err(RpcError::new(rpc::CODE_SHUTTING_DOWN, "the daemon is stopping"));
        }
        // A session to continue: same vault, not running.
        let old = match &resume_from {
            Some(id) => {
                let old = self.find_meta(id).ok_or_else(|| failed(format!("no such session: {id}")))?;
                if old.vault != vault.id {
                    return Err(failed("that session belongs to another vault"));
                }
                if old.agent != agent {
                    return Err(failed(format!("that session ran with {}; resume it with the same agent", agent_title(old.agent))));
                }
                if self.known(id).and_then(|s| s.live()).is_some() {
                    return Err(failed("that session is still running"));
                }
                Some(old)
            }
            None => None,
        };

        // What must be true before anything is made.
        let registry = self.registry(false).await?;
        let entry = registry.get(agent).ok_or_else(|| failed("unknown agent"))?.clone();
        let info = entry.info();
        if !info.available {
            return Err(failed(format!(
                "{} is not available: {}",
                agent_title(agent),
                info.reason.unwrap_or_else(|| "unknown reason".to_string())
            )));
        }
        if info.logged_in == Some(false) {
            return Err(failed(format!("{} is not signed in: {}", agent_title(agent), login_hint(agent))));
        }
        let binary = entry.binary.clone().ok_or_else(|| failed("the agent has no binary"))?;
        let section = agent::agent_section(&self.ctx);
        let mode = section.sandbox_mode().map_err(failed)?;
        let bwrap_env = self.settings.bwrap_env.clone();
        let confinement = tokio::task::spawn_blocking(move || sandbox::confinement(mode, bwrap_env))
            .await
            .map_err(|_| failed("checking the sandbox crashed"))?
            .map_err(failed)?;
        let cerbo_bin = sandbox::resolve_cerbo_bin(self.settings.cerbo_bin_env.clone(), self.settings.current_exe.clone()).map_err(failed)?;
        let agents_root = sandbox::agents_root(&self.settings.host).map_err(failed)?;
        let agent_home = sandbox::prepare_home(&agents_root, agent).map_err(failed)?;
        let vault_root = std::fs::canonicalize(&vault.root).map_err(|e| failed(format!("{}: {e}", vault.root.display())))?;

        let id = old.as_ref().map_or_else(|| uuid::Uuid::new_v4().hyphenated().to_string(), |m| m.id.clone());
        let socket_dir = self.settings.sockets_dir.join(&id);
        let spec = SessionSpec {
            kind: agent,
            agent_binary: binary,
            vault: vault_root.clone(),
            socket_dir: socket_dir.clone(),
            agent_home,
            cerbo_bin: cerbo_bin.clone(),
            login_file: entry.login_file.clone().filter(|f| f.is_file()),
            real_home: self.settings.host.home.clone(),
            host_path: self.settings.host.path.clone(),
        };
        let launch = sandbox::plan(&confinement, &spec, &RealFs).map_err(failed)?;

        // The record. From here a failure is part of the session.
        let session = self.open_session(&vault, agent, &id, old, &launch)?;
        self.set_state(&session, SessionState::Starting, None, None);
        let resume = resume_from.is_some();
        match self.start(&session, &launch, &confinement, &vault_root, &cerbo_bin, &socket_dir, open_socket, resume).await {
            Ok(()) => {}
            Err(failure) => {
                self.set_state(&session, SessionState::Failed, Some(failure.reason), failure.stderr_tail);
            }
        }
        Ok(session.info())
    }

    /// The id of the vault a session belongs to.
    pub fn vault_of(&self, session: &str) -> Option<String> {
        self.find_meta(session).map(|m| m.vault)
    }

    fn find_meta(&self, id: &str) -> Option<Meta> {
        if let Some(session) = self.known(id) {
            return Some(session.lock().meta.clone());
        }
        self.store.find(id)
    }

    /// The in-memory session for `id` (new, or loaded from the store for a
    /// resume), with its feed open for appending.
    fn open_session(&self, vault: &VaultRef, agent: AgentKind, id: &str, old: Option<Meta>, launch: &Launch) -> Result<Arc<Session>, RpcError> {
        let log = self.store.open_log(&vault.id, id).map_err(failed)?;
        let now = store::now();
        let meta = match old {
            Some(mut meta) => {
                meta.sandboxed = launch.sandboxed;
                meta.warning = launch.warning.clone();
                meta.updated_at = now;
                meta
            }
            None => Meta {
                id: id.to_string(),
                vault: vault.id.clone(),
                agent,
                title: DEFAULT_TITLE.to_string(),
                state: SessionState::Starting,
                created_at: now.clone(),
                updated_at: now,
                resumable: false,
                sandboxed: launch.sandboxed,
                warning: launch.warning.clone(),
                acp_session_id: None,
            },
        };
        self.store.write_meta(&meta).map_err(failed)?;
        let session = Arc::new(Session {
            id: id.to_string(),
            vault: vault.id.clone(),
            vault_name: vault.name.clone(),
            agent,
            inner: Mutex::new(Inner { meta, log }),
            live: Mutex::new(None),
            ending: AtomicBool::new(false),
        });
        self.sessions.lock().unwrap_or_else(|e| e.into_inner()).insert(id.to_string(), Arc::clone(&session));
        Ok(session)
    }

    #[allow(clippy::too_many_arguments)]
    async fn start(
        self: &Arc<Self>,
        session: &Arc<Session>,
        launch: &Launch,
        confinement: &Confinement,
        vault_root: &Path,
        cerbo_bin: &Path,
        socket_dir: &Path,
        open_socket: &SocketOpener<'_>,
        resume: bool,
    ) -> Result<(), StartFailure> {
        let guard = open_socket(&session.id, &session.vault, socket_dir).map_err(|e| StartFailure::plain(format!("cannot open the session socket: {e}")))?;
        let command = tokio::process::Command::from(launch.command());
        let (client, mut events) = AcpClient::spawn(command, ClientOptions::default()).map_err(|e| StartFailure::of(&e, None))?;
        let client = Arc::new(client);
        let init = client.initialize().await.map_err(|e| StartFailure::of(&e, Some(&client)))?;
        let caps = init.capabilities;

        // The MCP server entry: the cerbo binary, bound to the vault, reaching
        // the daemon through the session's socket (as seen from the agent).
        let socket_path = match confinement {
            Confinement::Bwrap(_) => Path::new(sandbox::SANDBOX_SOCKET_DIR).join(sandbox::SESSION_SOCKET_NAME),
            Confinement::None => socket_dir.join(sandbox::SESSION_SOCKET_NAME),
        };
        let mcp = McpServer::stdio(
            "cerbo",
            cerbo_bin.to_string_lossy(),
            vec!["mcp".into(), "--vault".into(), vault_root.to_string_lossy().into_owned()],
            vec![("CERBO_SOCKET".into(), socket_path.to_string_lossy().into_owned())],
        );
        let meta = (session.agent == AgentKind::Claude).then(sandbox::claude_session_meta);

        let stored_acp = session.lock().meta.acp_session_id.clone();
        let mut context = None;
        let mut how = ResumeHow::Transcript;
        let mut acp_session = None;
        if resume && caps.load_session && let Some(old_id) = stored_acp {
            match client.load_session_with_meta(&old_id, vault_root, std::slice::from_ref(&mcp), meta.as_ref()).await {
                Ok(_) => {
                    // The agent replays its history as updates: it is in the
                    // feed already.
                    drain_updates(&mut events);
                    acp_session = Some(old_id);
                    how = ResumeHow::Load;
                }
                Err(AcpError::Exited(info)) => return Err(StartFailure::of(&AcpError::Exited(info), None)),
                Err(_) => {}
            }
        }
        let acp_session = match acp_session {
            Some(id) => id,
            None => {
                let created = client
                    .new_session_with_meta(vault_root, std::slice::from_ref(&mcp), meta.as_ref())
                    .await
                    .map_err(|e| StartFailure::of(&e, Some(&client)))?;
                if resume {
                    let past = self.store.read_events(&session.vault, &session.id, None, usize::MAX);
                    context = Some(transcript_summary(&past));
                }
                created.session_id
            }
        };

        {
            let mut inner = session.lock();
            inner.meta.resumable = caps.load_session;
            inner.meta.acp_session_id = Some(acp_session.clone());
        }
        let (done_tx, done_rx) = mpsc::unbounded_channel();
        let live = Arc::new(Live {
            client,
            acp_session,
            caps,
            socket: Mutex::new(Some(guard)),
            tool_approvals: Mutex::new(HashSet::new()),
            context: Mutex::new(context),
            done: done_tx,
        });
        *session.live.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::clone(&live));
        tokio::spawn(Arc::clone(self).drive(Arc::clone(session), live, events, done_rx));
        if resume {
            self.emit(session, SessionEventBody::Resumed { how });
        }
        self.set_state(session, SessionState::Idle, None, None);
        Ok(())
    }

    // ── The driver ───────────────────────────────────────────────────────────

    async fn drive(self: Arc<Self>, session: Arc<Session>, live: Arc<Live>, mut events: AcpEvents, mut done: mpsc::UnboundedReceiver<PromptDone>) {
        loop {
            tokio::select! {
                biased;
                event = events.recv() => match event {
                    Some(AcpEvent::Exited(info)) => {
                        self.on_exit(&session, &live, &info);
                        return;
                    }
                    Some(event) => self.handle_event(&session, &live, event),
                    None => return,
                },
                Some(PromptDone::Finished(result)) = done.recv() => {
                    // Everything the agent said before the turn ended is in
                    // the stream by now: write it first.
                    while let Ok(event) = events.try_recv() {
                        if let AcpEvent::Exited(info) = event {
                            self.on_exit(&session, &live, &info);
                            return;
                        }
                        self.handle_event(&session, &live, event);
                    }
                    self.finish_turn(&session, result);
                }
            }
        }
    }

    fn handle_event(self: &Arc<Self>, session: &Arc<Session>, live: &Arc<Live>, event: AcpEvent) {
        match event {
            AcpEvent::Update(note) => self.on_update(session, live, note),
            AcpEvent::Permission(request) => {
                tokio::spawn(Arc::clone(self).on_permission(Arc::clone(session), Arc::clone(live), request));
            }
            AcpEvent::Refused { .. } | AcpEvent::Notification { .. } | AcpEvent::Warning(_) | AcpEvent::Exited(_) => {}
        }
    }

    fn on_update(&self, session: &Session, live: &Live, note: SessionNotification) {
        if note.session_id != live.acp_session {
            return;
        }
        match note.update {
            SessionUpdate::AgentMessageChunk { content, .. } => {
                if let Some(text) = content.as_text() {
                    self.emit(session, SessionEventBody::AgentText { text: text.to_string() });
                }
            }
            SessionUpdate::AgentThoughtChunk { content, .. } => {
                if let Some(text) = content.as_text() {
                    self.emit(session, SessionEventBody::AgentThought { text: text.to_string() });
                }
            }
            SessionUpdate::ToolCall(call) => {
                self.emit(
                    session,
                    SessionEventBody::ToolCall {
                        id: call.tool_call_id,
                        title: call.title,
                        kind: call.kind.as_ref().map_or("other", |k| k.as_str()).to_string(),
                        status: call.status.as_ref().map_or("pending", |s| s.as_str()).to_string(),
                        input_summary: call.raw_input.as_ref().map(summarize_value),
                    },
                );
            }
            SessionUpdate::ToolCallUpdate(update) => {
                let text = update
                    .content
                    .as_ref()
                    .and_then(|c| {
                        c.iter().find_map(|c| match c {
                            ToolCallContent::Content { content } => content.as_text().map(str::to_string),
                            _ => None,
                        })
                    })
                    .or_else(|| update.raw_output.as_ref().map(summarize_value))
                    .map(|t| truncate(&t, SUMMARY_CHARS));
                self.emit(
                    session,
                    SessionEventBody::ToolUpdate {
                        id: update.tool_call_id,
                        status: update.status.as_ref().map_or("in_progress", |s| s.as_str()).to_string(),
                        output_summary: text,
                    },
                );
            }
            // History replay, plans, mode and usage updates: not part of the feed.
            SessionUpdate::UserMessageChunk { .. } | SessionUpdate::Plan { .. } | SessionUpdate::Other { .. } => {}
        }
    }

    fn finish_turn(&self, session: &Session, result: Result<StopReason, AcpError>) {
        match result {
            Ok(reason) => {
                self.emit(session, SessionEventBody::TurnEnded { stop_reason: reason.as_str().to_string() });
                self.set_state(session, SessionState::Idle, None, None);
            }
            // The agent is gone: the driver reports that.
            Err(AcpError::Exited(_)) => {}
            Err(e) => {
                self.emit(session, SessionEventBody::Error { message: e.to_string() });
                self.emit(session, SessionEventBody::TurnEnded { stop_reason: "error".to_string() });
                self.set_state(session, SessionState::Idle, None, None);
            }
        }
    }

    /// The agent process ended. If nobody asked for it, the session failed;
    /// there is no retry.
    fn on_exit(&self, session: &Session, live: &Live, info: &crate::acp::ExitInfo) {
        if session.ending.load(Ordering::SeqCst) {
            return;
        }
        self.cancel_approvals_of(session, live, ApprovalOutcome::Cancelled);
        *session.live.lock().unwrap_or_else(|e| e.into_inner()) = None;
        live.socket.lock().unwrap_or_else(|e| e.into_inner()).take();
        let text = info.to_string();
        let reason = text.split("; stderr:").next().unwrap_or(&text).to_string();
        let tail = Some(info.stderr_tail.trim().to_string()).filter(|t| !t.is_empty());
        self.set_state(session, SessionState::Failed, Some(reason), tail);
    }

    /// End every approval of a session that still waits: its tool cards and the
    /// vault writes of its agent.
    fn cancel_approvals_of(&self, session: &Session, live: &Live, outcome: ApprovalOutcome) {
        live.tool_approvals.lock().unwrap_or_else(|e| e.into_inner()).clear();
        for pending in self.approvals.list(Some(&session.vault)) {
            if pending.session.as_deref() != Some(session.id.as_str()) {
                continue;
            }
            match outcome {
                ApprovalOutcome::Rejected => {
                    let _ = self.approvals.answer(pending.id, false);
                }
                _ => {
                    self.approvals.cancel(pending.id);
                }
            }
        }
    }

    // ── Tool permission (D3) ─────────────────────────────────────────────────

    async fn on_permission(self: Arc<Self>, session: Arc<Session>, live: Arc<Live>, request: PermissionRequest) {
        if request.session_id != live.acp_session {
            request.cancel();
            return;
        }
        let mode = agent::get_permission(&self.ctx, &session.vault);
        if permission::decide(mode, &request.tool_call) == permission::Decision::Allow
            && let Some(option) = permission::option_for(ApprovalOutcome::Approved, &request.options)
        {
            let _ = request.select(&option);
            return;
        }
        let tool = &request.tool_call;
        let title = tool.title.clone().or_else(|| tool.name.clone()).unwrap_or_else(|| "a tool".to_string());
        let diff = tool.content.as_ref().and_then(|c| {
            c.iter().find_map(|c| match c {
                ToolCallContent::Diff { old_text, new_text, .. } => Some(crate::ops::unified_diff(old_text.as_deref().unwrap_or(""), new_text)),
                _ => None,
            })
        });
        let timeout = self.approval_wait();
        let (id, rx) = self.approvals.request(
            approvals::Request {
                vault: session.vault.clone(),
                vault_name: session.vault_name.clone(),
                agent: agents::name_of(session.agent).to_string(),
                action: "tool".to_string(),
                page: None,
                summary: format!("{} wants to use: {}", agent_title(session.agent), truncate(&title, SUMMARY_CHARS)),
                diff,
                session: Some(session.id.clone()),
                tool_title: Some(truncate(&title, SUMMARY_CHARS)),
                tool_kind: Some(permission::kind_word(tool.kind.as_ref())),
            },
            timeout,
        );
        live.tool_approvals.lock().unwrap_or_else(|e| e.into_inner()).insert(id);
        let outcome = self.approvals.wait(id, rx, timeout).await;
        live.tool_approvals.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
        match permission::option_for(outcome, &request.options) {
            Some(option) => {
                let _ = request.select(&option);
            }
            None => request.cancel(),
        }
    }

    // ── Messages ─────────────────────────────────────────────────────────────

    /// Send a message to the agent. Answers at once; the turn runs on.
    pub fn send(
        self: &Arc<Self>,
        session: &str,
        text: String,
        context: Vec<PageRef>,
        read_page: &dyn Fn(&str) -> Option<String>,
    ) -> Result<(), RpcError> {
        let session = self.known(session).ok_or_else(|| self.missing(session))?;
        let live = session.live().ok_or_else(|| failed("the session has ended; start a new one or resume it"))?;
        {
            let mut inner = session.lock();
            match inner.meta.state {
                SessionState::Working => return Err(failed("the agent is still working on the previous message")),
                SessionState::Idle => inner.meta.state = SessionState::Working,
                _ => return Err(failed("the session is not ready for a message")),
            }
        }
        let blocks = build_prompt(&text, &context, &live.caps, live.context.lock().unwrap_or_else(|e| e.into_inner()).take(), read_page);
        self.emit(&session, SessionEventBody::UserMessage { text, context });
        self.emit(&session, SessionEventBody::State { state: SessionState::Working, reason: None, stderr_tail: None });
        let done = live.done.clone();
        tokio::spawn(async move {
            let result = live.client.prompt(&live.acp_session, &blocks).await;
            let _ = done.send(PromptDone::Finished(result));
        });
        Ok(())
    }

    fn missing(&self, id: &str) -> RpcError {
        if self.store.find(id).is_some() {
            failed("the session is not running (it ended, or the daemon restarted); resume it or start a new one")
        } else {
            failed(format!("no such session: {id}"))
        }
    }

    /// Cancel the agent's current turn; the session stays open.
    pub fn cancel(self: &Arc<Self>, session: &str) -> Result<(), RpcError> {
        let session = self.known(session).ok_or_else(|| self.missing(session))?;
        let live = session.live().ok_or_else(|| failed("the session has ended"))?;
        if session.lock().meta.state != SessionState::Working {
            return Ok(());
        }
        // The cards of the turn end first; then the agent is told to stop.
        let waiting: Vec<u64> = live.tool_approvals.lock().unwrap_or_else(|e| e.into_inner()).drain().collect();
        for id in waiting {
            self.approvals.cancel(id);
        }
        live.client.cancel(&live.acp_session);
        Ok(())
    }

    // ── End, delete, list ────────────────────────────────────────────────────

    /// End a session: its waiting approvals are rejected, the agent process
    /// stops, the socket goes. Ending an ended session does nothing.
    pub async fn end(self: &Arc<Self>, session: &str) -> Result<(), RpcError> {
        let session = self.known(session).ok_or_else(|| self.missing(session))?;
        self.end_session(&session, ApprovalOutcome::Rejected).await;
        Ok(())
    }

    async fn end_session(&self, session: &Arc<Session>, how: ApprovalOutcome) {
        session.ending.store(true, Ordering::SeqCst);
        let live = session.live.lock().unwrap_or_else(|e| e.into_inner()).take();
        let Some(live) = live else {
            let state = session.lock().meta.state;
            if !matches!(state, SessionState::Ended | SessionState::Failed) {
                self.set_state(session, SessionState::Ended, None, None);
            }
            return;
        };
        self.cancel_approvals_of(session, &live, how);
        live.client.shutdown().await;
        live.socket.lock().unwrap_or_else(|e| e.into_inner()).take();
        self.set_state(session, SessionState::Ended, None, None);
    }

    /// Delete an ended or failed session and its feed.
    pub fn delete(&self, session: &str) -> Result<(), RpcError> {
        let meta = self.find_meta(session).ok_or_else(|| failed(format!("no such session: {session}")))?;
        if self.known(session).and_then(|s| s.live()).is_some() {
            return Err(failed("the session is running: end it first"));
        }
        self.sessions.lock().unwrap_or_else(|e| e.into_inner()).remove(session);
        self.store.delete(&meta.vault, &meta.id).map_err(failed)
    }

    /// The sessions of a vault (all without), newest first. A session whose
    /// agent is not running here (the daemon restarted since) shows as ended.
    pub fn list(&self, vault: Option<&str>) -> Vec<SessionInfo> {
        self.store
            .list(vault)
            .into_iter()
            .map(|stored| {
                if let Some(session) = self.known(&stored.id) {
                    let running = session.live().is_some();
                    let mut info = session.info();
                    if !running && matches!(info.state, SessionState::Starting | SessionState::Idle | SessionState::Working) {
                        info.state = SessionState::Ended;
                    }
                    info
                } else {
                    let mut info = stored.info();
                    if matches!(info.state, SessionState::Starting | SessionState::Idle | SessionState::Working) {
                        info.state = SessionState::Ended;
                    }
                    info
                }
            })
            .collect()
    }

    /// A session's stored feed after `after_seq`, at most `limit` events.
    pub fn events(&self, session: &str, after_seq: Option<u64>, limit: usize) -> Result<Vec<SessionEvent>, RpcError> {
        let meta = self.find_meta(session).ok_or_else(|| failed(format!("no such session: {session}")))?;
        Ok(self.store.read_events(&meta.vault, &meta.id, after_seq, limit))
    }

    /// The daemon stops: every agent ends, waiting approvals are cancelled.
    pub async fn shutdown(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        let sessions: Vec<Arc<Session>> = self.sessions.lock().unwrap_or_else(|e| e.into_inner()).values().cloned().collect();
        for session in sessions {
            self.end_session(&session, ApprovalOutcome::Cancelled).await;
        }
    }
}

/// Discard what is waiting in the event stream (the replay of `session/load`).
fn drain_updates(events: &mut AcpEvents) {
    while let Ok(event) = events.try_recv() {
        // An exit ends the stream anyway; the driver sees the closed channel.
        let _ = event;
    }
}

/// The blocks of a prompt: an earlier transcript (resume), the text, and each
/// attached page as embedded text where the agent takes it, else as a link.
fn build_prompt(text: &str, context: &[PageRef], caps: &AgentCapabilities, transcript: Option<String>, read_page: &dyn Fn(&str) -> Option<String>) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();
    if let Some(transcript) = transcript {
        blocks.push(ContentBlock::text(transcript));
    }
    blocks.push(ContentBlock::text(text));
    for page in context {
        let uri = format!("cerbo://{}", page.path);
        let embedded = caps.prompt.embedded_context.then(|| read_page(&page.path)).flatten();
        match embedded {
            Some(content) => blocks.push(ContentBlock::Resource {
                resource: ResourceContents::Text { uri, text: content, mime_type: Some("text/markdown".to_string()) },
            }),
            None => blocks.push(ContentBlock::ResourceLink {
                name: page.title.clone(),
                uri,
                mime_type: Some("text/markdown".to_string()),
                size: None,
                title: Some(page.title.clone()),
                description: None,
            }),
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(seq: u64, body: SessionEventBody) -> SessionEvent {
        SessionEvent { seq, at: "t".into(), body }
    }

    #[test]
    fn titles_come_from_the_first_line() {
        assert_eq!(title_of("\n  Plan the trip  \nsecond"), "Plan the trip");
        assert_eq!(title_of("   "), DEFAULT_TITLE);
        let long = "x".repeat(200);
        assert_eq!(title_of(&long).chars().count(), TITLE_CHARS + 1);
    }

    #[test]
    fn the_transcript_joins_chunks_and_skips_the_noise() {
        let events = vec![
            ev(1, SessionEventBody::UserMessage { text: "hi".into(), context: vec![] }),
            ev(2, SessionEventBody::AgentText { text: "hel".into() }),
            ev(3, SessionEventBody::AgentText { text: "lo".into() }),
            ev(4, SessionEventBody::AgentThought { text: "secret".into() }),
            ev(5, SessionEventBody::PageChanged { path: "p1".into() }),
            ev(6, SessionEventBody::UserMessage { text: "bye".into(), context: vec![] }),
        ];
        let text = transcript_summary(&events);
        assert!(text.contains("User: hi\nAgent: hello\n(the agent changed the page p1)\nUser: bye"), "{text}");
        assert!(!text.contains("secret"));
    }

    #[test]
    fn a_long_transcript_keeps_its_end() {
        let events = vec![
            ev(1, SessionEventBody::UserMessage { text: "a".repeat(TRANSCRIPT_CHARS), context: vec![] }),
            ev(2, SessionEventBody::UserMessage { text: "the end".into(), context: vec![] }),
        ];
        let text = transcript_summary(&events);
        assert!(text.contains("the end"));
        assert!(text.contains('…'));
    }

    #[test]
    fn pages_are_embedded_only_where_the_agent_takes_them() {
        let page = PageRef { path: "p137".into(), title: "Vendor".into() };
        let read = |_: &str| Some("body".to_string());
        let mut caps = AgentCapabilities::default();
        let links = build_prompt("hi", std::slice::from_ref(&page), &caps, None, &read);
        assert_eq!(links.len(), 2);
        assert!(matches!(&links[1], ContentBlock::ResourceLink { uri, .. } if uri == "cerbo://p137"));
        caps.prompt.embedded_context = true;
        let embedded = build_prompt("hi", std::slice::from_ref(&page), &caps, Some("old".into()), &read);
        assert_eq!(embedded.len(), 3);
        assert!(matches!(&embedded[2], ContentBlock::Resource { resource: ResourceContents::Text { text, .. } } if text == "body"));
        // A page that cannot be read falls back to a link.
        let none = |_: &str| None;
        let fallback = build_prompt("hi", std::slice::from_ref(&page), &caps, None, &none);
        assert!(matches!(&fallback[1], ContentBlock::ResourceLink { .. }));
    }
}
