# Spec Delta

## Purpose

The daemon runs agent sessions so they continue without the window and can be reopened later.

## ADDED Requirements

### Requirement: Supported agents

The daemon SHALL be able to run OpenCode and Claude Code as ACP agents, found by their command on the user's `PATH` or by a configured path. An agent that is not installed SHALL be shown as unavailable with the reason.

#### Scenario: Agent missing
- **WHEN** `opencode` is not installed
- **THEN** OpenCode SHALL be listed as unavailable and Claude Code SHALL still be usable

### Requirement: A session works on one vault

A session SHALL be tied to one vault and given that vault only, through `cerbo mcp`. The agent's own process SHALL run with the vault's folder as its working directory and with the vault's agent permission in force.

#### Scenario: Read-only session
- **WHEN** a session starts on a read-only vault
- **THEN** the agent SHALL be able to read the vault and every write SHALL be refused

### Requirement: Sessions survive the window

A session SHALL keep running when the app window closes or the app quits. Reopening the app SHALL show the running session with everything said so far.

#### Scenario: Close and return
- **WHEN** the user closes the app while the agent works and opens it again
- **THEN** the panel SHALL show the same session and its new messages

### Requirement: Session history

Each session SHALL be stored with its messages, tool calls and outcomes, listed per vault, newest first, with a title taken from its first message. The user SHALL be able to delete a session.

#### Scenario: Reopen an earlier session
- **WHEN** the user picks an earlier session from the list
- **THEN** its messages SHALL be shown and the user SHALL be able to continue it if the agent supports resuming, and otherwise start a new one with its transcript as context

### Requirement: Stop and cancel

The user SHALL be able to cancel the agent's current turn and to end a session. Ending SHALL stop the agent process. Pending approvals of the session SHALL be rejected.

#### Scenario: Cancel a turn
- **WHEN** the user cancels while the agent is working
- **THEN** the agent SHALL stop the current turn and the session SHALL stay open

### Requirement: Agent failures are reported

If the agent process exits or fails to start, the session SHALL show the reason (including the tail of its error output) and offer to start a new session. It SHALL NOT retry by itself.

#### Scenario: Agent crashes
- **WHEN** the agent process exits during a turn
- **THEN** the session SHALL show that it stopped, with the reason

### Requirement: Headless run

`cerbo agent run --vault <vault> [--agent opencode|claude] "<prompt>"` SHALL start a session, send the prompt, wait for the turn to finish, print the agent's final answer, and exit 0 on success. Under `approve`, approvals SHALL follow the daemon's approval rules (wait for any client, reject at the deadline); under `read` and `auto` it SHALL need no person. `--json` SHALL print the transcript as JSON.

#### Scenario: Cron job
- **WHEN** a cron job runs `cerbo agent run --vault Notes "Summarize today's inbox page"` on an `auto` vault with no app open
- **THEN** the command SHALL finish, print the summary, and any page changes SHALL be in the change log

#### Scenario: Approve mode with nobody there
- **WHEN** the same command runs on an `approve` vault and no client answers
- **THEN** the request SHALL be rejected at the deadline and the command SHALL report the rejection and exit non-zero

### Requirement: List and stop from the CLI

`cerbo agent list` SHALL show running sessions with vault, agent and state. `cerbo agent stop <session>` SHALL end one.

#### Scenario: Stop a runaway session
- **WHEN** the user runs `cerbo agent stop` for a running session
- **THEN** the session SHALL end and its agent process SHALL stop
