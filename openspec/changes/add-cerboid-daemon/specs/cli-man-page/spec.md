## ADDED Requirements

### Requirement: Man page documents daemon and MCP commands

The man page SHALL document `cerbo daemon start|stop|status`, `cerbo mcp --vault`, and `cerbo vault agent`, with their arguments, the three agent permission levels (`read`, `auto`, `approve`), and a short example of registering `cerbo mcp` with an agent CLI.

#### Scenario: Commands listed
- **WHEN** the user runs `man cerbo`
- **THEN** the COMMANDS section SHALL describe `daemon`, `mcp` and `vault agent`

#### Scenario: Permission levels explained
- **WHEN** the user reads the `vault agent` entry
- **THEN** it SHALL explain `read`, `auto` and `approve`
