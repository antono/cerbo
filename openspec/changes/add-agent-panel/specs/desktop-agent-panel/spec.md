# Spec Delta

## Purpose

The Agent tab lets the user talk to an agent about the open vault and decide what it may change.

## ADDED Requirements

### Requirement: Chat feed

The Agent tab SHALL show the session as a feed: the user's messages, the agent's replies as they stream in, and one compact row per tool call with its name, short arguments and result state (running, done, failed). A long feed SHALL stay scrolled to the newest message unless the user scrolled up.

#### Scenario: Streaming reply
- **WHEN** the agent is answering
- **THEN** its reply text SHALL appear as it arrives

#### Scenario: Tool call row
- **WHEN** the agent searches pages
- **THEN** a row SHALL show the tool, its query and, when done, the result count

### Requirement: Header controls

The tab header SHALL show the agent picker (OpenCode, Claude Code), the connection state, a New session action and a Sessions list. Changing the agent SHALL start a new session.

#### Scenario: Switch agent
- **WHEN** the user picks Claude Code
- **THEN** a new session SHALL start with Claude Code

### Requirement: Composer

The composer SHALL send on Enter and add a new line on Shift+Enter. It SHALL show the open page as a context chip that the user can remove. Typing `@` SHALL offer vault pages to add as context. While the agent works, the send action SHALL become a cancel action. While the agent is still starting, the send action SHALL be disabled.

#### Scenario: Mention a page
- **WHEN** the user types `@ven` and chooses "Vendor onboarding"
- **THEN** the page SHALL be added to the message as context

#### Scenario: Window fell behind
- **WHEN** the daemon tells the window it dropped events for it (`resync`)
- **THEN** the panel SHALL read the sessions, the open feed and the pending approvals again, so nothing stays missing

#### Scenario: Remove page context
- **WHEN** the user removes the open-page chip and sends
- **THEN** the message SHALL be sent without that page as context

### Requirement: Approval cards

In `approve` mode, each pending change SHALL appear in the feed as a card showing the page and the difference, with Approve and Reject actions. Answering SHALL update the card to show the outcome. A card for a request that timed out SHALL show that.

#### Scenario: Approve an edit
- **WHEN** the user presses Approve on an edit card
- **THEN** the edit SHALL apply, the card SHALL show Applied, and the agent SHALL continue

#### Scenario: Pending while the panel was closed
- **WHEN** a request arrived while the panel was hidden
- **THEN** the Agent tab icon SHALL show a waiting mark and the card SHALL be there when the tab opens

### Requirement: Permission control

The tab SHALL show the vault's agent permission and let the user switch between read-only, auto and approve. Switching to a wider permission (read-only to auto or approve, approve to auto) SHALL ask for confirmation.

#### Scenario: Widen asks
- **WHEN** the user switches from approve to auto
- **THEN** a confirmation SHALL explain that edits will apply without asking

### Requirement: Changed pages are visible

When the agent changes a page, the feed SHALL link to it, and opening it SHALL show the new content. If the page is open and clean it SHALL refresh by itself.

#### Scenario: Open the changed page
- **WHEN** the user clicks the page link in an applied-change row
- **THEN** the page SHALL open with the agent's edit

### Requirement: Empty and error states

With no agent installed the tab SHALL explain how to install one and which commands Cerbo looks for. With the daemon unreachable it SHALL show the daemon-unavailable message with Retry.

#### Scenario: No agent installed
- **WHEN** neither agent is found
- **THEN** the tab SHALL explain what to install and offer a recheck action
