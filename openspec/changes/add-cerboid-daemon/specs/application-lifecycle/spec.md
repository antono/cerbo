## ADDED Requirements

### Requirement: Daemon connection at startup

The app SHALL connect to the `cerboid` daemon before it opens a vault, and SHALL show the vault only once the connection works. The details of finding and starting the daemon are defined by the `desktop-daemon-client` capability.

#### Scenario: Vault opens after connection
- **WHEN** the app starts and the daemon accepts the connection
- **THEN** the app SHALL open the last used vault through the daemon

#### Scenario: No connection
- **WHEN** the app cannot connect to or start the daemon
- **THEN** the app SHALL NOT show a vault and SHALL show the daemon-unavailable banner
