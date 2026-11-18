# Spec Delta

## Purpose

The user decides, per vault, what an agent may do: only read, or manage the vault either freely or with approval for each change.

## ADDED Requirements

### Requirement: Permission levels

Each vault SHALL have one agent permission: `read` (read-only), `manage-auto` (the agent changes the vault and changes apply at once) or `manage-approve` (each change waits for approval). A vault with no setting SHALL be `read`.

#### Scenario: Default
- **WHEN** an agent connects to a vault that was never configured
- **THEN** it SHALL be able to read and SHALL NOT be able to change anything

### Requirement: Read-only blocks every change

At `read`, any write request SHALL be refused by the daemon, even if the client sends one. The refusal SHALL say the vault is read-only.

#### Scenario: Write attempt
- **WHEN** a client sends an edit for a read-only vault
- **THEN** the daemon SHALL refuse it and the page SHALL stay unchanged

### Requirement: Auto mode applies at once

At `manage-auto`, writes from the agent SHALL apply without asking. Each applied change SHALL be recorded in an agent change log with time, page and the agent's name.

#### Scenario: Auto edit
- **WHEN** an agent edits a page in a `manage-auto` vault
- **THEN** the edit SHALL apply and appear in the change log

### Requirement: Approve mode asks per change

At `manage-approve`, each write from the agent SHALL become a pending request showing what will change (for an edit, the difference) and SHALL apply only after a person approves. A rejection SHALL leave the vault unchanged and tell the agent the request was rejected.

#### Scenario: Approved
- **WHEN** a person approves a pending edit
- **THEN** the edit SHALL apply and the agent SHALL be told it succeeded

#### Scenario: Rejected
- **WHEN** a person rejects a pending edit
- **THEN** the page SHALL stay unchanged and the agent SHALL be told the request was rejected

### Requirement: Approved edits never overwrite newer changes

An approved edit replaces the whole page, so the daemon SHALL apply it only when the page is still the text it was when the agent asked. Otherwise it SHALL refuse with a conflict result, tell the agent to read the page again, and leave the page as it is.

#### Scenario: Page changed while waiting
- **WHEN** a person edits a page while an agent's edit of it waits, then approves the agent's edit
- **THEN** the agent's edit SHALL NOT apply, the person's edit SHALL remain and the agent SHALL be told about the conflict

### Requirement: Abandoned and excessive requests

A pending request whose agent connection closed SHALL be cancelled and SHALL never be applied. One agent connection SHALL have at most 137 requests waiting; a further change SHALL be refused at once with a message saying so.

#### Scenario: Agent killed while waiting
- **WHEN** an agent process ends while its change waits for approval
- **THEN** the request SHALL leave the queue and the change SHALL NOT apply, even if approved later

### Requirement: Scope of the guarantee

The documentation SHALL state that permissions apply to agents that work through `cerbo mcp`, that a process running as the user can bypass them, and that real isolation of agent processes is planned with the agent panel.

#### Scenario: Honest manual
- **WHEN** a user reads the AGENT PERMISSIONS section of the man page
- **THEN** it SHALL say plainly that a process of the same user can bypass the permission

### Requirement: Approvals work headless

A pending request SHALL stay in the daemon until a client answers it, including a client that connects later. A request with no answer within a time limit (default 10 minutes, configurable) SHALL be treated as rejected. A client that waits for the answer SHALL wait at least until that time limit, so an approval given late is never reported to the agent as a failure.

#### Scenario: Answered after the app opens
- **WHEN** an agent requests an edit while no app is open and the user opens the app 3 minutes later
- **THEN** the app SHALL show the pending request and approving it SHALL apply the edit

#### Scenario: Nobody answers
- **WHEN** no client answers within the time limit
- **THEN** the request SHALL be rejected and the agent told it timed out

### Requirement: Permission is set by the user, not the agent

The permission of a vault SHALL be changeable only by the user through the app or `cerbo` commands. No agent tool SHALL read or change it.

#### Scenario: Agent cannot raise its own access
- **WHEN** an agent in a read-only vault looks for a way to change the permission
- **THEN** no tool SHALL exist for it

### Requirement: Setting the permission

`cerbo vault agent <vault> read|auto|approve` SHALL set the level, and `cerbo vault agent <vault>` with no level SHALL print it. The change SHALL take effect for already-connected agents without a restart.

#### Scenario: Tighten while running
- **WHEN** the user sets a vault from `auto` to `read` while an agent is connected
- **THEN** the agent's next write SHALL be refused
