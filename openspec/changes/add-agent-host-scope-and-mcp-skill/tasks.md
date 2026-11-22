## 1. Settings and CLI

- [x] 1.1 Add `host` (`off|on`) and `host_paths` (`{path, mode}`) to the per-vault agent config in `core/src/agent.rs`, default `off`, with unknown-key tolerance
- [x] 1.2 Add `cerbo vault agent <vault> host [on|off]` and `host-path add|remove <path> [--write]`, with `--json`
- [x] 1.3 Refuse never-reachable paths and relative paths when adding; warn when an entry is `/` or a parent of the home folder
- [x] 1.4 Update `cli/man/cerbo.md` (commands and the AGENT PERMISSIONS section, including the isolation warning)

## 2. Host tools in the daemon

- [x] 2.1 Add RPC calls for list, read and write outside the vault in `core/src/rpc/mod.rs`; classify them for the write queue and change log
- [x] 2.2 Implement path resolution, the never-reachable set, the vault-inside refusal and list matching in one decision function, with unit tests for links, `..` and relative paths
- [x] 2.3 Route "ask" decisions through the existing approval queue with path, action and diff or size; apply the same deadline and rejection messages
- [x] 2.4 Write files with the durable-write helper; add a read size limit with offset chunking
- [x] 2.5 Log every host action (allowed, approved, rejected, timed out, refused) in the agent change log

## 3. MCP server

- [x] 3.1 Add `host_list`, `host_read`, `host_write` to `cli/src/mcp.rs`, listed only when host access is `on`; re-list on a settings change
- [x] 3.2 Add `ontology_list`, `ontology_types` (read) and `ontology_create`, `ontology_add_type` (write, hidden at `read`)
- [x] 3.3 Implement vault-owned ontology creation and class declaration in `core`, refusing bundled and imported ontologies and taken prefixes
- [x] 3.4 Extend the server instructions with the short method summary and a pointer to the skill

## 4. Agent skill

- [x] 4.1 Confirm where Claude Code and OpenCode read skills inside the agent home (done: the sandbox switches their own tools off, so the guide is an MCP tool instead, see design D8)
- [x] 4.2 Write the three guides (usage, management, ontology and RDF with the known ontologies): search choice, scores, links, safe editing with `base_version`, annotations, checking and creating types, SPARQL recipes
- [x] 4.3 Render each per session (only the sections the session may use) and serve it through the `guide` tool with a `topic` and as the resources `cerbo://guide/<topic>`; share the summary with the server instructions; keep one skill file per guide generated from the same texts
- [x] 4.4 Add a test that every tool named in the skill is registered by the server and that a `read` session gets no write sections

## 5. App

- [x] 5.1 Add host access controls and the path list to the agent settings
- [x] 5.2 Show the "host access on" marker and list in the agent panel
- [x] 5.3 Show host approval cards with real path, action and diff or size

## 6. Tests and docs

- [x] 6.1 Integration tests with the fake ACP agent: off, listed read, listed write, ask approved, ask rejected, timeout, link escape, never-reachable
- [x] 6.2 Test that tightening applies on the next call and rejects pending requests
- [x] 6.3 Ontology tool tests: add type then resolve an annotation; duplicate; imported ontology refused
- [x] 6.4 Update `docs/` with the host access explanation and run `openspec validate`

## 7. Index, trash and ontology import tools

- [x] 7.1 Core: `trash` module (list, restore the newest entry, refuse live UUIDs) and `ontology::import` with `check_import`, with unit tests
- [x] 7.2 RPC calls and daemon handlers for index status and rebuild, trash list and restore, ontology import; imports always ask and are checked before the card
- [x] 7.3 MCP tools `index_status`, `index_rebuild`, `list_trash`, `restore_page`, `ontology_import`; extend the management and ontology guides
- [x] 7.4 Integration tests: restore after delete, index status, import asks in an auto vault and refuses a local file
- [x] 7.5 Update the man page and `docs/` for the new tools
