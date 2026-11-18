# Spec Delta

## Purpose

One background process per user owns every registered vault, so the app, the CLI and agents never write to the same files at once.

## ADDED Requirements

### Requirement: One daemon per user

At most one `cerboid` SHALL run per user. A second instance started while one runs SHALL exit at once without error, and say that the daemon is already running.

#### Scenario: Second start
- **WHEN** `cerboid` is started while another `cerboid` of the same user runs
- **THEN** the second process SHALL exit with status 0 and leave the first untouched

#### Scenario: Crash recovery
- **WHEN** the daemon died without cleaning up
- **THEN** the next start SHALL succeed and replace the stale socket

### Requirement: Local socket for the owner only

The daemon SHALL listen on a Unix socket in the user's runtime directory with permissions that allow only that user. It SHALL NOT listen on any network address.

#### Scenario: Other users blocked
- **WHEN** another user of the machine tries to connect
- **THEN** the connection SHALL be refused by file permissions

### Requirement: Daemon owns vault access

While the daemon runs, reads and writes of vault pages, attachments, links, annotations, the semantic index and SPARQL queries SHALL be served by the daemon. Writes SHALL go through one queue per vault, applied in the order received, with the same durability as direct writes.

#### Scenario: Two writers
- **WHEN** the desktop app and an agent each save a different page at the same time
- **THEN** both saves SHALL be applied and neither SHALL be lost or corrupted

### Requirement: Daemon serves every registered vault

The daemon SHALL serve all vaults in the user's vault list and pick up vaults added or removed while it runs, without a restart.

#### Scenario: Vault added while running
- **WHEN** the user adds a vault with `cerbo vault add`
- **THEN** clients SHALL be able to use it through the daemon without restarting it

### Requirement: Change events

The daemon SHALL tell connected clients when a page changes, whether the change came from a client, an agent or an outside edit of the files, naming the vault and page.

#### Scenario: Agent edit reaches the app
- **WHEN** an agent saves a page that is open in the desktop app
- **THEN** the app SHALL receive a change event for that page

### Requirement: Keeps automatic rewrites away

While running, the daemon SHALL hold the same lock a running desktop app holds, so automatic vocabulary migrations skip the vault.

#### Scenario: Migration skipped
- **WHEN** the daemon runs and the CLI would start an automatic vocabulary migration
- **THEN** the migration SHALL be skipped and nothing written

### Requirement: Daemon commands

`cerbo daemon start` SHALL run the daemon in the background and return once it accepts connections. `cerbo daemon stop` SHALL stop it after finishing queued writes. `cerbo daemon status` SHALL print whether it runs, its version and the vaults it serves, and exit non-zero when it does not run. `--json` SHALL be accepted by `status`.

#### Scenario: Status when stopped
- **WHEN** the daemon is not running and the user runs `cerbo daemon status`
- **THEN** the command SHALL say it is not running and exit non-zero

#### Scenario: Stop finishes writes
- **WHEN** the user runs `cerbo daemon stop` while a write is queued
- **THEN** the write SHALL finish before the daemon exits

### Requirement: Version match

A client SHALL check the daemon's protocol version when it connects. On a mismatch it SHALL tell the user to restart the daemon and SHALL NOT send requests.

#### Scenario: Old daemon after upgrade
- **WHEN** the app is upgraded while an older daemon still runs
- **THEN** the app SHALL report the mismatch and ask to restart the daemon
