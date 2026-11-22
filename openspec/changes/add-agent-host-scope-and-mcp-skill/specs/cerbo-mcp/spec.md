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

### Requirement: Instructions point to the guides

The server instructions (a single text, as MCP defines them) SHALL give a short summary of how to search and annotate and SHALL tell the agent to call the `guide` tool, with the topics `usage`, `management` and `ontology`, described in `agent-skill`. The `guide` tool SHALL be listed at every permission level.

#### Scenario: Instructions
- **WHEN** a client initializes the server
- **THEN** the instructions SHALL mention search by meaning, types, the guide tool and its three topics

### Requirement: Guides as resources

The server SHALL declare the resources capability and list one Markdown resource per guide, `cerbo://guide/usage`, `cerbo://guide/management` and `cerbo://guide/ontology`. Reading one SHALL return the guide rendered for the session's permission and host access, as the guide tool does. Other URIs SHALL be unknown.

#### Scenario: List
- **WHEN** a client lists resources
- **THEN** it SHALL receive the three guides with a title and a one-sentence description

#### Scenario: Read follows the permission
- **WHEN** the vault's permission changes from `read` to `auto` and the client reads `cerbo://guide/management` again
- **THEN** the second read SHALL describe the page-changing tools and the first SHALL NOT

### Requirement: Index tools

The server SHALL offer a read tool that reports whether the vault's semantic index is built, usable and current, how many pages there are and how many have embeddings, and a write tool that rebuilds the index from the vault's files without embeddings. The rebuild SHALL be subject to the vault's agent permission, hidden at `read`.

#### Scenario: Status after a rebuild
- **WHEN** the agent rebuilds the index and then asks for its status
- **THEN** the status SHALL say built, usable and not stale, with the vault's page count

#### Scenario: Read-only vault
- **WHEN** the vault is `read` and the agent lists tools
- **THEN** the status tool SHALL be listed and the rebuild tool SHALL NOT

### Requirement: Trash tools

The server SHALL offer a read tool that lists the trash (UUID, title, deletion time, newest first) and a write tool that puts a page back from it with its UUID. Restoring SHALL take the newest entry of that UUID and SHALL be refused, changing nothing, when a page with that UUID exists or the trash holds none. There SHALL be no tool that empties the trash or removes an entry.

#### Scenario: Delete and restore
- **WHEN** the agent deletes a page and then restores it
- **THEN** the page SHALL be live again with the same UUID and text, and links to it SHALL work

#### Scenario: Live page
- **WHEN** the agent restores a UUID whose page exists
- **THEN** the tool SHALL refuse and the page SHALL stay as it is

### Requirement: Ontology import always asks

The server SHALL offer a write tool that fetches an ontology file from an address and stores it as an Ontology object, optionally registering a prefix and namespace given together. It SHALL accept only http and https addresses, limit the size and the time of the fetch, and SHALL wait for a person's approval showing the address at every permission level that offers it, including `auto`. A bad address, a prefix in use or an invalid namespace SHALL be refused before anyone is asked.

#### Scenario: Auto vault still asks
- **WHEN** the vault is `auto` and the agent imports an ontology
- **THEN** a person SHALL be asked, and a rejection SHALL leave the vault unchanged

#### Scenario: Local file
- **WHEN** the agent passes a `file://` address
- **THEN** the tool SHALL refuse at once and no approval SHALL be shown

#### Scenario: Imported ontologies stay closed
- **WHEN** an ontology was imported
- **THEN** the ontology write tools SHALL refuse to add types to it
