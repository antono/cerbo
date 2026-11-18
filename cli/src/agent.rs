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

/// `cerbo agent run`. Returns the exit code; `Err` is a failure to even try
/// (exit 1 as well, with the message).
pub fn run(ctx: &CerboContext, options: RunOptions) -> Result<i32, String> {
    let vault = daemon::find_vault(ctx, &options.vault)?;
    let mut client = connect(ctx)?;
    // Listen before anything starts, so no event of the turn is missed.
    client.call(Call::Subscribe { vault: Some(vault.id.clone()) }).map_err(|e| e.message)?;
    let value = client
        .call(Call::SessionCreate { vault: vault.id.clone(), agent: options.agent, resume_from: None })
        .map_err(|e| e.message)?;
    let info: SessionInfo = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if let Some(warning) = &info.warning {
        eprintln!("warning: {warning}");
    }
    if info.state == SessionState::Failed {
        let reason = failure_reason(&feed(&mut client, &info.id)?);
        return Err(format!("the {} session did not start: {reason}", word(options.agent)));
    }
    client
        .call(Call::SessionSend { session: info.id.clone(), text: options.prompt.clone(), context: Vec::new() })
        .map_err(|e| e.message)?;

    let deadline = options.timeout.map(|t| Instant::now() + t);
    let mut timed_out = false;
    let mut ended_badly: Option<String> = None;
    'wait: loop {
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
            _ => {}
        }
    }
    if timed_out {
        let _ = client.call(Call::SessionCancel { session: info.id.clone() });
    }
    let stored = feed(&mut client, &info.id)?;
    // The session was this command's: it ends with it. The transcript stays.
    let _ = client.call(Call::SessionEnd { session: info.id.clone() });
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
