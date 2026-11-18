//! `cerbo agent`: run an agent headless, list running sessions, stop one
//! (design D7).
//!
//! These commands are thin clients of the daemon's session API: the agent
//! process belongs to the daemon, never to this program. `agent run` starts a
//! session, sends the prompt, waits for the turn to end and prints the answer.

use crate::daemon;
use cerbo_core::rpc::launch;
use cerbo_core::rpc::{
    AgentKind, ApprovalOutcome, Call, ClientKind, Event, SessionEvent, SessionEventBody, SessionInfo, SessionState, VaultInfo,
};
use cerbo_core::CerboContext;
use serde_json::json;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

/// The command succeeded.
pub const EXIT_OK: i32 = 0;
/// The command failed: no agent, the session could not start or crashed, the
/// turn ended in an error, or the daemon could not be used.
pub const EXIT_FAILED: i32 = 1;
/// The turn finished, but a change or a tool the agent asked for was rejected
/// (by a person, or by the approval deadline).
pub const EXIT_REJECTED: i32 = 3;
/// `--timeout` ran out before the turn ended.
pub const EXIT_TIMED_OUT: i32 = 4;

/// How long one wait for an event lasts before the deadline is checked again.
const POLL: Duration = Duration::from_millis(250);
/// How long the start of the agent may take before `agent run` gives up.
const START_LIMIT: Duration = Duration::from_secs(180);
/// How often the start looks at the session's state itself, in case an event
/// was dropped.
const START_LOOK: Duration = Duration::from_secs(2);

/// The signal (`SIGINT` or `SIGTERM`) that asked `agent run` to stop; 0 for none.
static SIGNALLED: AtomicI32 = AtomicI32::new(0);

#[cfg(unix)]
extern "C" fn on_signal(signal: libc::c_int) {
    SIGNALLED.store(signal, Ordering::SeqCst);
}

/// Take `SIGINT` and `SIGTERM` as a request to stop: the command then cancels
/// the turn, ends the session and exits with `128 + signal`, instead of dying
/// with the session left to run in the daemon.
#[cfg(unix)]
fn catch_signals() {
    // SAFETY: the handler only stores into an atomic, which is async-signal-safe.
    unsafe {
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
    }
}

#[cfg(not(unix))]
fn catch_signals() {}

fn signalled() -> Option<i32> {
    match SIGNALLED.load(Ordering::SeqCst) {
        0 => None,
        signal => Some(signal),
    }
}

/// Ends the session when dropped, on whatever path the command leaves: an
/// error, a panic, or a signal handled by the loop. The daemon also ends an
/// `ephemeral` session when the connection closes; this is the prompt way.
struct EndOnExit<'a> {
    ctx: &'a CerboContext,
    session: Option<String>,
}

impl EndOnExit<'_> {
    /// The session was ended already.
    fn disarm(&mut self) {
        self.session = None;
    }
}

impl Drop for EndOnExit<'_> {
    fn drop(&mut self) {
        if let Some(session) = self.session.take()
            && let Ok(mut client) = connect(self.ctx)
        {
            let _ = client.call(Call::SessionEnd { session });
        }
    }
}

/// What `cerbo agent run` was asked.
pub struct RunOptions {
    pub vault: String,
    pub agent: AgentKind,
    pub json: bool,
    pub timeout: Option<Duration>,
    pub prompt: String,
}

fn connect(ctx: &CerboContext) -> Result<cerbo_core::rpc::client::Client, String> {
    if std::env::var_os(daemon::NO_DAEMON_ENV).is_some_and(|v| !v.is_empty()) {
        return Err("agent sessions need the daemon, and CERBO_NO_DAEMON is set".to_string());
    }
    launch::connect_or_start(&daemon::paths(ctx), ClientKind::Cli, Some("cerbo"), None).map_err(|e| e.message)
}

fn word(agent: AgentKind) -> &'static str {
    match agent {
        AgentKind::Opencode => "opencode",
        AgentKind::Claude => "claude",
    }
}

fn running(state: SessionState) -> bool {
    matches!(state, SessionState::Starting | SessionState::Idle | SessionState::Working)
}

fn state_word(state: SessionState) -> &'static str {
    match state {
        SessionState::Starting => "starting",
        SessionState::Idle => "idle",
        SessionState::Working => "working",
        SessionState::Ended => "ended",
        SessionState::Failed => "failed",
    }
}

/// The stored feed of a session.
fn feed(client: &mut cerbo_core::rpc::client::Client, session: &str) -> Result<Vec<SessionEvent>, String> {
    let value = client
        .call(Call::SessionEvents { session: session.to_string(), after_seq: None, limit: 1_000_000 })
        .map_err(|e| e.message)?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

/// Why a session failed, from its feed.
fn failure_reason(feed: &[SessionEvent]) -> String {
    for event in feed.iter().rev() {
        if let SessionEventBody::State { state: SessionState::Failed, reason, stderr_tail } = &event.body {
            let mut text = reason.clone().unwrap_or_else(|| "the agent failed".to_string());
            if let Some(tail) = stderr_tail {
                text.push_str(&format!("\nagent stderr:\n{tail}"));
            }
            return text;
        }
    }
    "the agent failed".to_string()
}

/// The last state a feed names.
fn last_state(feed: &[SessionEvent]) -> Option<SessionState> {
    feed.iter().rev().find_map(|e| match &e.body {
        SessionEventBody::State { state, .. } => Some(*state),
        _ => None,
    })
}

/// What the last turn of a feed amounts to.
struct Outcome {
    text: String,
    stop_reason: Option<String>,
    rejected: usize,
    error: Option<String>,
}

fn outcome_of(feed: &[SessionEvent]) -> Outcome {
    let start = feed.iter().rposition(|e| matches!(e.body, SessionEventBody::UserMessage { .. })).map_or(0, |i| i + 1);
    let mut outcome = Outcome { text: String::new(), stop_reason: None, rejected: 0, error: None };
    for event in &feed[start..] {
        match &event.body {
            SessionEventBody::AgentText { text } => outcome.text.push_str(text),
            SessionEventBody::TurnEnded { stop_reason } => outcome.stop_reason = Some(stop_reason.clone()),
            SessionEventBody::ApprovalResolved { outcome: ApprovalOutcome::Rejected | ApprovalOutcome::TimedOut, .. } => {
                outcome.rejected += 1;
            }
            SessionEventBody::Error { message } => outcome.error = Some(message.clone()),
            _ => {}
        }
    }
    outcome
}

/// Wait until the session's agent has started. Creating a session answers at
/// once; the agent is ready when the session says `idle`.
fn wait_started(client: &mut cerbo_core::rpc::client::Client, id: &str) -> Result<(), String> {
    let began = Instant::now();
    let mut looked = Instant::now();
    loop {
        if let Some(signal) = signalled() {
            return Err(format!("interrupted by signal {signal} while the agent started"));
        }
        if began.elapsed() >= START_LIMIT {
            return Err(format!("the agent did not start within {} seconds", START_LIMIT.as_secs()));
        }
        let mut check = looked.elapsed() >= START_LOOK;
        match client.next_event(POLL)? {
            Some(Event::Session { session, event, .. }) if session == id => {
                if let SessionEventBody::State { state, .. } = event.body {
                    match state {
                        SessionState::Idle => return Ok(()),
                        SessionState::Failed | SessionState::Ended => check = true,
                        _ => {}
                    }
                }
            }
            // Events were lost: look at the session itself.
            Some(Event::Resync) => check = true,
            _ => {}
        }
        if check {
            looked = Instant::now();
            let value = client.call(Call::SessionList { vault: None }).map_err(|e| e.message)?;
            let sessions: Vec<SessionInfo> = serde_json::from_value(value).map_err(|e| e.to_string())?;
            match sessions.iter().find(|s| s.id == id).map(|s| s.state) {
                Some(SessionState::Idle | SessionState::Working) => return Ok(()),
                Some(SessionState::Failed) => return Err(failure_reason(&feed(client, id)?)),
                Some(SessionState::Ended) => return Err("the session ended while the agent started".to_string()),
                _ => {}
            }
        }
    }
}

/// `cerbo agent run`. Returns the exit code; `Err` is a failure to even try
/// (exit 1 as well, with the message). The session ends on every way out.
pub fn run(ctx: &CerboContext, options: RunOptions) -> Result<i32, String> {
    let vault = daemon::find_vault(ctx, &options.vault)?;
    let mut client = connect(ctx)?;
    catch_signals();
    // Listen before anything starts, so no event of the turn is missed.
    client.call(Call::Subscribe { vault: Some(vault.id.clone()) }).map_err(|e| e.message)?;
    // `ephemeral`: the daemon ends the session when this connection closes, so
    // even a command that is killed outright leaves no agent running.
    let value = client
        .call(Call::SessionCreate { vault: vault.id.clone(), agent: options.agent, resume_from: None, ephemeral: true })
        .map_err(|e| e.message)?;
    let info: SessionInfo = serde_json::from_value(value).map_err(|e| e.to_string())?;
    let mut guard = EndOnExit { ctx, session: Some(info.id.clone()) };
    if let Some(warning) = &info.warning {
        eprintln!("warning: {warning}");
    }
    if let Err(why) = wait_started(&mut client, &info.id) {
        if let Some(signal) = signalled() {
            eprintln!("Error: interrupted");
            return Ok(128 + signal);
        }
        return Err(format!("the {} session did not start: {why}", word(options.agent)));
    }
    client
        .call(Call::SessionSend { session: info.id.clone(), text: options.prompt.clone(), context: Vec::new() })
        .map_err(|e| e.message)?;

    let deadline = options.timeout.map(|t| Instant::now() + t);
    let mut timed_out = false;
    let mut ended_badly: Option<String> = None;
    let mut interrupted: Option<i32> = None;
    'wait: loop {
        if let Some(signal) = signalled() {
            interrupted = Some(signal);
            break;
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            timed_out = true;
            break;
        }
        match client.next_event(POLL)? {
            Some(Event::Session { session, event, .. }) if session == info.id => match event.body {
                SessionEventBody::TurnEnded { .. } => break 'wait,
                SessionEventBody::State { state: SessionState::Failed, .. } => {
                    ended_badly = Some("failed".to_string());
                    break 'wait;
                }
                SessionEventBody::State { state: SessionState::Ended, .. } => {
                    ended_badly = Some("ended".to_string());
                    break 'wait;
                }
                _ => {}
            },
            // Events were lost: the feed says whether the turn ended meanwhile.
            Some(Event::Resync) => {
                let stored = feed(&mut client, &info.id)?;
                match last_state(&stored) {
                    Some(SessionState::Failed) => {
                        ended_badly = Some("failed".to_string());
                        break 'wait;
                    }
                    Some(SessionState::Ended) => {
                        ended_badly = Some("ended".to_string());
                        break 'wait;
                    }
                    _ => {}
                }
                if outcome_of(&stored).stop_reason.is_some() {
                    break 'wait;
                }
            }
            _ => {}
        }
    }
    if timed_out || interrupted.is_some() {
        let _ = client.call(Call::SessionCancel { session: info.id.clone() });
    }
    // The session was this command's: it ends with it. The transcript stays.
    if interrupted.is_some() {
        let _ = client.call(Call::SessionEnd { session: info.id.clone() });
        guard.disarm();
        eprintln!("Error: interrupted; the agent's turn was cancelled and the session ended");
        return Ok(128 + interrupted.unwrap_or(0));
    }
    let stored = feed(&mut client, &info.id)?;
    let _ = client.call(Call::SessionEnd { session: info.id.clone() });
    guard.disarm();
    let outcome = outcome_of(&stored);

    let code = if timed_out {
        EXIT_TIMED_OUT
    } else if ended_badly.is_some() || outcome.error.is_some() || !matches!(outcome.stop_reason.as_deref(), Some("end_turn" | "max_tokens" | "max_turn_requests")) {
        EXIT_FAILED
    } else if outcome.rejected > 0 {
        EXIT_REJECTED
    } else {
        EXIT_OK
    };

    if options.json {
        let failure = ended_badly.as_ref().map(|_| failure_reason(&stored));
        println!(
            "{}",
            json!({
                "session": info.id,
                "vault": vault.id,
                "agent": word(options.agent),
                "exit_code": code,
                "stop_reason": outcome.stop_reason,
                "rejected": outcome.rejected,
                "timed_out": timed_out,
                "error": outcome.error.clone().or(failure),
                "text": outcome.text,
                "events": stored,
            })
        );
        return Ok(code);
    }
    match code {
        EXIT_TIMED_OUT => eprintln!("Error: the agent did not finish within the time limit; its turn was cancelled"),
        EXIT_FAILED => {
            let why = match (&ended_badly, &outcome.error, &outcome.stop_reason) {
                (Some(_), _, _) => failure_reason(&stored),
                (None, Some(message), _) => message.clone(),
                (None, None, Some(reason)) => format!("the turn ended with: {reason}"),
                (None, None, None) => "the turn did not finish".to_string(),
            };
            eprintln!("Error: {why}");
        }
        _ => {}
    }
    if !outcome.text.trim().is_empty() {
        println!("{}", outcome.text.trim_end());
    }
    if code == EXIT_REJECTED {
        eprintln!(
            "Error: {} request{} the agent made {} rejected (by a person, or because nobody answered in time); \
             the vault is unchanged by {}",
            outcome.rejected,
            if outcome.rejected == 1 { "" } else { "s" },
            if outcome.rejected == 1 { "was" } else { "were" },
            if outcome.rejected == 1 { "it" } else { "them" },
        );
    }
    Ok(code)
}

/// `cerbo agent list`: the running sessions, or all with `all`.
pub fn list(ctx: &CerboContext, all: bool, json: bool) -> Result<(), String> {
    let mut client = daemon::running_client(ctx)?.ok_or("cerboid is not running, so no agent session runs")?;
    let value = client.call(Call::SessionList { vault: None }).map_err(|e| e.message)?;
    let mut sessions: Vec<SessionInfo> = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if !all {
        sessions.retain(|s| running(s.state));
    }
    if json {
        println!("{}", serde_json::to_string(&sessions).map_err(|e| e.to_string())?);
        return Ok(());
    }
    if sessions.is_empty() {
        println!("{}", if all { "No agent sessions" } else { "No running agent sessions" });
        return Ok(());
    }
    let vaults: Vec<VaultInfo> = client
        .call(Call::Vaults)
        .ok()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    for s in sessions {
        let vault = vaults.iter().find(|v| v.id == s.vault).map_or(s.vault.as_str(), |v| v.name.as_str());
        println!("{}  {}  {}  {}  {}", s.id, vault, word(s.agent), state_word(s.state), s.title);
    }
    Ok(())
}

/// `cerbo agent stop <session>`: the id, or the start of one.
pub fn stop(ctx: &CerboContext, session: &str, json: bool) -> Result<(), String> {
    let mut client = daemon::running_client(ctx)?.ok_or("cerboid is not running, so no agent session runs")?;
    let value = client.call(Call::SessionList { vault: None }).map_err(|e| e.message)?;
    let sessions: Vec<SessionInfo> = serde_json::from_value(value).map_err(|e| e.to_string())?;
    let matching: Vec<&SessionInfo> = if sessions.iter().any(|s| s.id == session) {
        sessions.iter().filter(|s| s.id == session).collect()
    } else {
        sessions.iter().filter(|s| !session.is_empty() && s.id.starts_with(session)).collect()
    };
    let target = match matching.as_slice() {
        [one] => *one,
        [] => return Err(format!("no agent session {session} (see `cerbo agent list --all`)")),
        _ => return Err(format!("{session} matches several sessions; give more of the id")),
    };
    client.call(Call::SessionEnd { session: target.id.clone() }).map_err(|e| e.message)?;
    if json {
        println!("{}", json!({ "success": true, "session": target.id }));
    } else {
        println!("Stopped session {}", target.id);
    }
    Ok(())
}
