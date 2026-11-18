# Spec Delta

## Purpose

Lets a Nix user turn the daemon on with one option and have systemd keep it running.

## ADDED Requirements

### Requirement: Flake exports a Home Manager module

The flake SHALL export `homeManagerModules.default`. Importing it SHALL add the option `services.cerbo.daemon.enable`, default `false`, and change nothing while it is `false`.

#### Scenario: Disabled by default
- **WHEN** a configuration imports the module without enabling it
- **THEN** no package, service or file SHALL be added

### Requirement: Enabling runs a systemd user service

With `services.cerbo.daemon.enable = true`, the module SHALL install `cerboid` and the `cerbo` CLI and define a systemd user service `cerboid` that starts with the user's session, restarts on failure, and is stopped when the user logs out.

#### Scenario: Service after switch
- **WHEN** the user enables the option and switches the Home Manager configuration
- **THEN** `systemctl --user status cerboid` SHALL show it running

#### Scenario: Restart on crash
- **WHEN** the daemon process is killed
- **THEN** systemd SHALL start it again

### Requirement: Package option

The module SHALL offer `services.cerbo.daemon.package`, defaulting to the flake's package, so users can pin another build.

#### Scenario: Custom package
- **WHEN** the user sets the package option
- **THEN** the service SHALL run that package's `cerboid`

### Requirement: Socket activation

The module SHALL also define a systemd user socket for `cerboid`, so the first client to connect starts the daemon if it is not running.

#### Scenario: Started on demand
- **WHEN** the daemon is stopped and `cerbo mcp` connects to the socket
- **THEN** systemd SHALL start `cerboid` and the connection SHALL be served

### Requirement: Declarative vault permissions

The module SHALL offer `services.cerbo.daemon.vaults.<name>.agent` with values `read`, `auto` and `approve`, default `read`, applied when the service starts. A permission set by the user through `cerbo` commands SHALL be overwritten only by a later switch that changes the option.

#### Scenario: Permission from config
- **WHEN** the user sets `vaults.Notes.agent = "approve"` and switches
- **THEN** `cerbo vault agent Notes` SHALL print `approve`

### Requirement: Service does not break the app

When the service runs, the desktop app SHALL use it and SHALL NOT start a second daemon.

#### Scenario: App with service running
- **WHEN** the service is active and the user opens the app
- **THEN** the app SHALL connect to the existing daemon
