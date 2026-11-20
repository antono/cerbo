# Spec Delta

## Purpose

Adds ontology and host tools to the MCP server. Applies on top of `cerbo-mcp` from `add-cerboid-daemon`, which has to be archived first.

## ADDED Requirements

### Requirement: Ontology read tools

The server SHALL offer tools to list the vault's ontologies with their prefixes and to list the types (classes) and properties of one ontology with labels and descriptions where they exist. They SHALL work at every permission level.

#### Scenario: List types
- **WHEN** the agent asks for the types of the `schema` ontology
- **THEN** it SHALL receive type names with their labels

#### Scenario: Unknown prefix
- **WHEN** the agent asks for a prefix the vault does not map
- **THEN** the tool SHALL return an error result naming the prefix

### Requirement: Ontology write tools

The server SHALL offer a tool to add a type to a vault-owned ontology (name, label, optional description and optional parent type) and a tool to create a vault-owned ontology with a prefix. These tools SHALL be subject to the vault's agent permission, SHALL NOT be listed at `read`, and at `approve` SHALL wait for a person's approval showing what will be added. Adding a type that already exists SHALL return the existing type without changing anything.

#### Scenario: Add a type
- **WHEN** the agent adds the type `Recipe` to a vault-owned ontology and permission allows it
- **THEN** `[Pancakes]{ex:Recipe}` SHALL resolve in that vault afterwards

#### Scenario: Read-only vault
- **WHEN** the vault is `read` and the agent lists tools
- **THEN** no ontology write tool SHALL appear

#### Scenario: Duplicate
- **WHEN** the agent adds a type that the ontology already declares
- **THEN** nothing SHALL change and the result SHALL name the existing type

### Requirement: Bundled and imported ontologies are not edited

The ontology write tools SHALL refuse to change an ontology that is bundled (Schema.org, FOAF) or was imported from a URL or an external vault. The refusal SHALL tell the agent to add its type to a vault-owned ontology.

#### Scenario: Add to Schema.org
- **WHEN** the agent tries to add a type to `schema`
- **THEN** the tool SHALL refuse and name the alternative

### Requirement: Host tools

When host access is `on` (see `agent-host-access`), the server SHALL offer tools to list a directory, read a file and write a file outside the vault, and SHALL send each call through the daemon's host-access checks. Their descriptions SHALL say that a path outside the allowed list asks a person and that the call may wait.

#### Scenario: Call outside the list
- **WHEN** the agent reads a file outside every allowed path
- **THEN** the call SHALL wait for approval and return the content only after it

### Requirement: Instructions point to the skill

The server instructions SHALL give a short summary of how to search and annotate and SHALL name the skill described in `agent-skill`.

#### Scenario: Instructions
- **WHEN** a client initializes the server
- **THEN** the instructions SHALL mention search by meaning, types and the skill
