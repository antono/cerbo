# Spec Delta

## Purpose

An agent that works with a vault through `cerbo mcp` is given a clear, short guide on how to find things, how to annotate and how to create new types.

## ADDED Requirements

### Requirement: Three guides

The server SHALL offer three guides, each a separate text: `usage` (finding and reading: when to search by meaning, by text and with SPARQL, how to read scores, following links), `management` (changing the vault: reading before editing, passing the version as `base_version`, folders, links, delete as trash, approvals, files outside the vault), and `ontology` (annotations, the known ontologies, what the RDF graph holds, useful SPARQL questions, looking at vocabularies, and how and when to create a new type, including what only the user can do). Each SHALL be useful on its own and SHALL point to the others.

#### Scenario: Choosing a search
- **WHEN** an agent reads the usage guide to find notes "about Berlin" in any language
- **THEN** the guide SHALL point it to search by meaning first and to text search for exact words

#### Scenario: Creating a type
- **WHEN** an agent needs a type no ontology offers
- **THEN** the ontology guide SHALL tell it to look for an existing type, then to add a new one to a vault-owned ontology, and to use it in annotations afterwards

#### Scenario: Known ontologies
- **WHEN** an agent reads the ontology guide
- **THEN** it SHALL find the built-in prefixes (`schema`, `foaf`, `dcterms`, `skos`, `rdf`, `rdfs`, `owl`, `xsd`) with what each is for, and the advice to follow the vault's own habit

#### Scenario: Queries work
- **WHEN** an agent runs a SPARQL question printed in a guide
- **THEN** it SHALL be valid and answer what the guide says it answers

### Requirement: Skill matches the tools

Every tool the skill names SHALL exist in the server, and the skill SHALL NOT name a write tool as available when the vault is read-only. A test SHALL fail when the skill names a tool that the server does not offer.

#### Scenario: Renamed tool
- **WHEN** a tool is renamed and the skill still uses the old name
- **THEN** the test SHALL fail

### Requirement: Delivered through the MCP server

The MCP server instructions are one text, so they SHALL contain a short summary of the method and SHALL name the three guides. The `guide` tool SHALL take a `topic` (`usage`, `management` or `ontology`; `usage` without one) and return that guide, in every vault and at every permission level; an unknown topic SHALL be an error that lists the real ones. The server SHALL also offer each guide as a resource `cerbo://guide/<topic>` (Markdown), for clients that show or attach resources, with the same text as the tool. This SHALL work for agents in the daemon's sandbox, whose own file and skill tools are switched off, which is why the tool is the channel that is always there. Each guide, with every section, SHALL also be kept in the repository as its own skill file for agents that run outside the sandbox.

#### Scenario: Agent in the sandbox
- **WHEN** a sandboxed agent starts a session
- **THEN** the server instructions SHALL name the guides and the guide tool SHALL return each one without any file tool

#### Scenario: Resource
- **WHEN** a client reads `cerbo://guide/management`
- **THEN** it SHALL receive the management guide rendered for the session, the same as the guide tool; an unknown guide URI SHALL be a protocol error

#### Scenario: Skill file is current
- **WHEN** the guide text changes
- **THEN** a test SHALL fail until the repository's skill file is regenerated

### Requirement: Skill follows the permissions

The text the `guide` tool and the resources return SHALL describe only what that session may do: write and ontology-write tools SHALL be described only when offered, and host tools only when host access is `on`. The summary in the server instructions SHALL name only tools that exist at every level.

#### Scenario: Read-only vault
- **WHEN** the vault is `read`
- **THEN** the delivered text SHALL NOT describe any tool that changes the vault
