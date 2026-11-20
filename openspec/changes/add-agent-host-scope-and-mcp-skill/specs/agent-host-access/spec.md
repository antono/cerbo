# Spec Delta

## Purpose

The user can let an agent work with files outside the vault: freely inside a list of paths the user chose, and only with a person's approval anywhere else.

## ADDED Requirements

### Requirement: Host access is a separate setting

Each vault SHALL have a host access setting, `off` or `on`, independent of its agent permission (`read`, `auto`, `approve`). A vault with no setting SHALL be `off`. Host access SHALL NOT change what the agent may do inside the vault, and the vault permission SHALL NOT change what the agent may do outside it.

#### Scenario: Default
- **WHEN** an agent works in a vault whose host access was never set
- **THEN** it SHALL have no way to read, list or write any file outside the vault

#### Scenario: Independent axes
- **WHEN** a vault is `read` with host access `on`
- **THEN** the agent SHALL still be unable to change the vault, and SHALL be able to use host access as set

### Requirement: Allowed paths

With host access `on`, the user SHALL be able to keep a list of allowed paths, each marked `read` or `write` (`write` includes `read`). A listed path SHALL cover the directory or file and everything below it. The agent SHALL read and list files under a listed path, and write files under a `write` path, without asking.

#### Scenario: Read inside a listed path
- **WHEN** `~/Documents/inbox` is listed as `read` and the agent reads a file under it
- **THEN** the agent SHALL get the content with no approval request

#### Scenario: Write under a read-only path
- **WHEN** `~/Documents/inbox` is listed as `read` and the agent tries to write a file under it
- **THEN** the write SHALL fail and the file SHALL stay unchanged

#### Scenario: Write under a write path
- **WHEN** `~/Projects/site/out` is listed as `write` and the agent writes a file under it
- **THEN** the file SHALL be written with no approval request

### Requirement: Anything else asks

With host access `on`, every read, list or write of a path that no list entry covers SHALL become a pending request that shows the path and the action (for a write, the difference or the size) and SHALL proceed only after a person approves. This SHALL hold at every vault permission level, including `auto`. A rejection or an expired request SHALL leave the file untouched and tell the agent which of the two happened.

#### Scenario: Approved read
- **WHEN** the agent reads `~/Downloads/report.pdf`, which no entry covers, and a person approves
- **THEN** the agent SHALL receive the content

#### Scenario: Rejected write
- **WHEN** the agent writes `~/notes.txt`, which no entry covers, and a person rejects
- **THEN** no file SHALL be written and the agent SHALL be told the request was rejected

#### Scenario: Auto vault still asks
- **WHEN** the vault is `auto` and the agent reads a path outside every list entry
- **THEN** a person SHALL be asked

#### Scenario: Nobody answers
- **WHEN** no client answers within the approval time limit
- **THEN** the request SHALL be treated as rejected and the agent told it timed out

### Requirement: Paths are resolved before they are checked

A path SHALL be checked after `..` segments and symbolic links are resolved to the real location. A link inside an allowed path that points outside it SHALL be treated as the outside path. A relative path SHALL be refused.

#### Scenario: Link out of an allowed path
- **WHEN** `~/Documents/inbox` is allowed and contains a link to `~/.config`
- **THEN** reading through that link SHALL be handled as a read of `~/.config`, not as an allowed read

### Requirement: Some paths are never reachable

The following SHALL be unreachable by the agent in every mode, even if listed or approved: Cerbo's own private directories (the daemon's data and runtime directories, the agents' homes and login files), the `.cerbo` directory of any vault other than the session's own, and the user's credential stores (at least `~/.ssh`, `~/.gnupg` and `~/.aws`). Adding such a path to the list SHALL be refused with a message naming the reason.

#### Scenario: Credentials
- **WHEN** the user tries to allow `~/.ssh`
- **THEN** the command SHALL fail and the list SHALL not change

#### Scenario: Approval cannot open it
- **WHEN** the agent asks to read `~/.ssh/id_ed25519` and a person would approve
- **THEN** the request SHALL be refused before any card is shown

### Requirement: Host tools offered only when on

The MCP server SHALL list the host tools (list a directory, read a file, write a file) only when host access is `on`. There SHALL be no tool to delete, move or run anything outside the vault.

#### Scenario: Off
- **WHEN** host access is `off` and the agent lists tools
- **THEN** no host tool SHALL appear

#### Scenario: No delete
- **WHEN** host access is `on`
- **THEN** no tool SHALL exist to delete or execute anything outside the vault

### Requirement: Set by the user, never by the agent

Host access and its list SHALL be changeable only by the user through the app or `cerbo vault agent <vault> host on|off` and `cerbo vault agent <vault> host-path add|remove <path> [--write]`. `cerbo vault agent <vault> host` SHALL print the setting and the list. No agent tool SHALL read or change them.

#### Scenario: Agent cannot widen its own access
- **WHEN** an agent looks for a way to add a path to the list
- **THEN** no tool SHALL exist for it

### Requirement: Tightening takes effect at once

Turning host access `off`, or removing or lowering a path, SHALL apply to the agent's next call at once, without ending its session, and pending requests that the change forbids SHALL be rejected. Widening SHALL also apply to the next call; the host tools SHALL appear or disappear from the tool list without a restart.

#### Scenario: Remove a path
- **WHEN** the user removes `~/Projects/site/out` while an agent session is running
- **THEN** the agent's next write there SHALL ask a person instead of proceeding

#### Scenario: Turn off
- **WHEN** the user sets host access `off` while a request waits for approval
- **THEN** the request SHALL be rejected and no file SHALL be touched

### Requirement: Host actions are recorded

Every host action SHALL be added to the agent change log with time, agent name, action, real path and outcome (done, rejected, timed out, refused), including actions allowed by the list.

#### Scenario: Allowed write is logged
- **WHEN** the agent writes a 137 KB file under a `write` path
- **THEN** the log SHALL show the write with its real path and the outcome "done"

### Requirement: Host access is visible

While host access is `on`, the agent panel SHALL show that fact and the list. The documentation SHALL say plainly that `on` reduces isolation: the agent can read and change everything under listed `write` paths, and what a person approves is approved for that single action only.

#### Scenario: Panel indicator
- **WHEN** a vault has host access `on`
- **THEN** the agent panel SHALL show an indicator and the allowed paths
