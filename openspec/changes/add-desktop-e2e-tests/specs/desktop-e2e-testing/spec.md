# Spec Delta

## Purpose

Defines how the desktop application is exercised end to end as a real binary,
so startup, IPC and webview regressions are caught before release.

## ADDED Requirements

### Requirement: E2E suite drives the real desktop binary
The e2e suite SHALL launch the built desktop application binary and interact
with its webview through the WebDriver protocol, without mocking backend
commands.

#### Scenario: Suite launches the built app
- **WHEN** a developer runs the e2e command after building the desktop app
- **THEN** the desktop app window is started and the tests interact with its UI

#### Scenario: Missing binary
- **WHEN** the e2e command runs and the desktop binary has not been built
- **THEN** the run fails before any test with a message naming the missing binary

### Requirement: E2E runs are isolated from user data
Each e2e session SHALL use its own temporary configuration and cache
directories and its own copy of any vault it opens, and SHALL NOT read or
modify the developer's registered vaults or settings.

#### Scenario: Developer config untouched
- **WHEN** the e2e suite runs on a machine with registered vaults
- **THEN** the developer's vault registry, UI settings and vault files are unchanged afterwards

#### Scenario: Fixture vault is a copy
- **WHEN** a test opens the fixture vault and the app writes to it
- **THEN** the repository's `test-vault/` directory is unchanged

### Requirement: E2E runs headless
The e2e command SHALL run without a visible display by starting a virtual X
server when needed.

#### Scenario: No display available
- **WHEN** the e2e command runs in a session without `DISPLAY`
- **THEN** the tests run against a virtual display and report results normally

### Requirement: Smoke scenarios
The e2e suite SHALL include smoke tests covering first launch without vaults
and launch with an existing vault.

#### Scenario: First launch shows empty state
- **WHEN** the app starts with no registered vaults
- **THEN** the window shows the prompt to add a vault

#### Scenario: Launch with a vault lists its pages
- **WHEN** the app starts with the fixture vault registered
- **THEN** the page list shows the fixture vault's pages
</content>
