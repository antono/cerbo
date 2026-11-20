# Spec Delta

## Purpose

Makes clear how vault permission relates to host access. Applies on top of `agent-vault-permissions` from `add-cerboid-daemon`.

## ADDED Requirements

### Requirement: Vault permission does not grant host access

Raising a vault's agent permission SHALL NOT enable host access, and enabling host access SHALL NOT raise the vault's permission. An approval at `auto` or `approve` for a vault change SHALL NOT count as approval of any action outside the vault.

#### Scenario: Auto does not open the disk
- **WHEN** a vault is set to `auto` and host access is `off`
- **THEN** the agent SHALL have no host tools and no access outside the vault

### Requirement: Scope of the guarantee names host access

The AGENT PERMISSIONS section of the man page SHALL describe host access, its list and its approval rule, and SHALL say that it reduces isolation.

#### Scenario: Honest manual
- **WHEN** a user reads the AGENT PERMISSIONS section
- **THEN** it SHALL describe `host` and say that listed `write` paths are open to the agent without asking
