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

### Requirement: The agent runs in a sandbox

The daemon SHALL start every agent process inside a bubblewrap sandbox in which the vault is read-only, the rest of the user's home is hidden except the agent's login file, and the main daemon socket is not reachable. The agent SHALL be able to change pages only through the session's own socket.

#### Scenario: Direct write is refused
- **WHEN** the agent tries to write a file in the vault folder with its own tools
- **THEN** the write SHALL fail and the page SHALL stay unchanged

#### Scenario: No bubblewrap
- **WHEN** `bwrap` is missing or user namespaces are disabled
- **THEN** starting a session SHALL fail with a message naming the cause

#### Scenario: Sandbox turned off
- **WHEN** the user set `agent.sandbox = "none"` in the daemon config
- **THEN** sessions SHALL start without the sandbox and each SHALL show a warning

### Requirement: The session socket is agent-only

Every connection on a session's socket SHALL count as an agent bound to the session's vault, whatever its hello says.

#### Scenario: Agent cannot pose as the app
- **WHEN** a process in the sandbox connects to the session socket and says it is the app
- **THEN** the daemon SHALL treat it as an agent of that vault and refuse approval answers

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
