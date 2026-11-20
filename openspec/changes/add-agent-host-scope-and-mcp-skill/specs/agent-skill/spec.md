# Spec Delta

## Purpose

An agent that works with a vault through `cerbo mcp` is given a clear, short guide on how to find things, how to annotate and how to create new types.

## ADDED Requirements

### Requirement: Skill covers the working method

The skill SHALL explain, for an agent that has only the MCP tools: when to search by meaning, by text and with SPARQL, and how to combine them; how to read search scores and what to do when results are weak; how pages link to each other with `cerbo://<uuid>`; how annotations `[Text]{prefix:Type}` work; how to check the existing types first; and how and when to create a new type. It SHALL tell the agent to read a page before editing it and to pass the version it read.

#### Scenario: Choosing a search
- **WHEN** an agent reads the skill to find notes "about Berlin" in any language
- **THEN** the skill SHALL point it to search by meaning first and to text search for exact words

#### Scenario: Creating a type
- **WHEN** an agent needs a type no ontology offers
- **THEN** the skill SHALL tell it to look for an existing type, then to add a new one to a vault-owned ontology, and to use it in annotations afterwards

### Requirement: Skill matches the tools

Every tool the skill names SHALL exist in the server, and the skill SHALL NOT name a write tool as available when the vault is read-only. A test SHALL fail when the skill names a tool that the server does not offer.

#### Scenario: Renamed tool
- **WHEN** a tool is renamed and the skill still uses the old name
- **THEN** the test SHALL fail

### Requirement: Delivered two ways

The server instructions SHALL contain a short summary of the method and SHALL name the skill. The full skill SHALL also be placed in the agent's home inside the sandbox in the form each supported agent reads as a skill. A session started without the skill file SHALL still work with the server instructions alone.

#### Scenario: Agent sees it
- **WHEN** a session starts for a supported agent
- **THEN** its home SHALL contain the skill and the server instructions SHALL point to it

### Requirement: Skill follows the permissions

The text delivered to an agent SHALL describe only what that session may do: write and ontology-write tools SHALL be described only when offered, and host tools only when host access is `on`.

#### Scenario: Read-only vault
- **WHEN** the vault is `read`
- **THEN** the delivered text SHALL NOT describe any tool that changes the vault
