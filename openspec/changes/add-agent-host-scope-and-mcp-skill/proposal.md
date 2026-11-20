# Proposal

## Why

The agent works well inside a vault, but two gaps remain. First, it can never touch files outside the vault: the bubblewrap sandbox and the `read` / `auto` / `approve` levels stop at the vault boundary, so tasks like "import these notes from `~/Documents`" or "save the result next to my project" are impossible. Second, agents get only a one-paragraph MCP instruction, and the MCP server has no ontology tools at all, so an agent cannot use semantic search well or extend the vocabulary when a note fits no existing type.

## What Changes

- Add a **host access** setting, separate from the vault permission: `vault-only` (default, today's behaviour) or `host`. In `host` mode the agent may read and write files outside the vault, under rules to be fixed in design (working name: `host-scoped` / `computer-scoped`).
- Host access is set only by the user (app or `cerbo` command), never by an agent tool, and each outside-vault action still goes through the existing approval card unless the user chose otherwise.
- Scope: a list of allowed paths (each `read` or `write`) opens without asking; every other path asks a person per action. The sandbox is unchanged: all outside access goes through daemon-run host tools, so no extra mounts and nothing to restart when the user tightens it.
- Add **ontology tools** to `cerbo mcp`: list ontologies and their types, and register a new type or ontology for annotations. These are write tools, hidden at `read` and subject to approval at `approve`.
- Ship an **agent skill** that teaches: when to use semantic search versus text search and SPARQL, how to read scores, how annotations `[Text]{prefix:Type}` work, how to check existing types first and create a new one only when needed. It reaches the agent through the MCP server instructions and as a skill file in the agent's sandbox home.

## Capabilities

### New Capabilities
- `agent-host-access`: the user-controlled setting that lets an agent act on files outside the vault, its scopes, approval behaviour and sandbox mounts.
- `agent-skill`: the skill text for agents working through `cerbo mcp`, how it is delivered, and the guarantee that it matches the tools actually offered.

### Modified Capabilities
- `cerbo-mcp`: new ontology read and write tools; server instructions point to the skill. (Defined in the in-flight `add-cerboid-daemon`; this change needs a delta on it once archived.)
- `agent-vault-permissions`: states that vault permission and host access are independent, and that host access never raises vault permission. (Same in-flight origin.)
- `ontology-management`: a type or prefix can be registered by an agent through the daemon, not only by `cerbo import-ontology`.

## Impact

- Code: `cerboid` (host tool handling, approvals), `core/src/agent.rs` (new setting), `core/src/rpc/mod.rs` (new calls), `cli/src/mcp.rs` (tools, instructions), `cli/man/cerbo.md` (new command and man section), desktop agent panel (setting UI and warning).
- Docs: the "scope of the guarantee" text must say that host mode weakens isolation.
- Depends on `add-cerboid-daemon` and `add-agent-panel` landing first.

## Non-goals

- No network or shell allowlists; only file scope changes.
- No agent-writable permission settings.
- No new embedding model or ranking changes.
- No Windows or macOS sandbox work.
