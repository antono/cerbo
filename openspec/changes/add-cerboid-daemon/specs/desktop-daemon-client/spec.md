# Spec Delta

## Purpose

The desktop app talks to the vault through the daemon, so it and agents see the same state.

## ADDED Requirements

### Requirement: App connects or starts the daemon

On start the app SHALL connect to the daemon. If none runs and the user's systemd socket is not available, it SHALL start one and wait until it accepts connections.

#### Scenario: Cold start
- **WHEN** the app starts and no daemon runs
- **THEN** the app SHALL start the daemon and then open its vault

### Requirement: App leaves a started daemon running

A daemon the app started SHALL keep running after the app quits, so agents can continue. The quit confirmation SHALL say that when an agent is working.

#### Scenario: Quit with agent working
- **WHEN** the user quits while an agent session is active
- **THEN** the confirmation SHALL say the agent keeps running in the background

### Requirement: Daemon unavailable

If the app cannot reach or start the daemon, it SHALL say so in a banner with a Retry action and SHALL NOT silently open the vault by itself while a daemon may still be running.

#### Scenario: Cannot start
- **WHEN** the daemon fails to start
- **THEN** the app SHALL show a banner explaining the failure with a Retry action

### Requirement: Live updates from agents

When a change event arrives for the open page, the app SHALL reload it, or, if the user has unsaved edits, SHALL show the existing external-change dialog instead of overwriting them.

#### Scenario: Clean page reloads
- **WHEN** an agent edits the open page and the user has no unsaved edits
- **THEN** the editor SHALL show the new content

#### Scenario: Unsaved edits
- **WHEN** an agent edits the open page while the user has unsaved edits
- **THEN** the external-change dialog SHALL appear
