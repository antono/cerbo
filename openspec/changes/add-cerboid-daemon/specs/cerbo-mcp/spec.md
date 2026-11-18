# Spec Delta

## Purpose

Lets an agent CLI work with a vault through a small set of tools, with every call going through the daemon.

## ADDED Requirements

### Requirement: Stdio MCP server

`cerbo mcp --vault <name-or-path>` SHALL run an MCP server on stdin and stdout, for agent CLIs to start. It SHALL forward every tool call to the daemon and SHALL NOT open the vault itself. When the daemon is not running it SHALL start it, or fail with a clear message if it cannot.

#### Scenario: Agent starts the server
- **WHEN** an agent CLI starts `cerbo mcp --vault Notes`
- **THEN** the server SHALL list its tools and answer calls for the Notes vault only

#### Scenario: Daemon not running
- **WHEN** the daemon is down and the server starts
- **THEN** it SHALL start the daemon, or report why it could not

### Requirement: Read tools

The server SHALL offer tools to list pages, read a page, search pages by text, search pages by meaning, list backlinks and outgoing links of a page, list attachments, and run a SPARQL query. Read tools SHALL work at every permission level.

#### Scenario: Read a page
- **WHEN** the agent calls the read-page tool with a page UUID
- **THEN** it SHALL receive the page's title, content and metadata

#### Scenario: Unknown page
- **WHEN** the agent asks for a UUID that does not exist
- **THEN** the tool SHALL return an error result naming the UUID, not crash

### Requirement: Write tools

The server SHALL offer tools to create, edit, rename, move and delete a page, and to link pages. Deleting SHALL move the page to the vault's trash, never remove it for good. Write tools SHALL be subject to the vault's agent permission.

#### Scenario: Delete goes to trash
- **WHEN** the agent deletes a page and permission allows it
- **THEN** the page SHALL be in the trash and recoverable

### Requirement: Tools hidden by permission

In a read-only vault the server SHALL NOT list write tools.

#### Scenario: Read-only vault
- **WHEN** the vault is read-only and the agent lists tools
- **THEN** no create, edit, rename, move, delete or link tool SHALL appear

### Requirement: Scoped to one vault

A server instance SHALL expose exactly one vault. Page references from another vault SHALL NOT resolve. Its daemon connection SHALL be bound to that vault: the daemon SHALL refuse any call about another vault, show the connection only its own vault in the vault list, and send it only page change events of that vault.

#### Scenario: Other vault
- **WHEN** the agent passes the UUID of a page that lives in a different vault
- **THEN** the tool SHALL report that the page was not found
