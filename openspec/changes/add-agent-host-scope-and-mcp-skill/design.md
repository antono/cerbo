# Design

## Context

An agent session runs under bubblewrap (`cerboid/src/sandbox.rs`): the vault is mounted read-only, the agent has its own home, and the agent's native file and shell tools are all denied (`CLAUDE_DENIED_TOOLS`, and the OpenCode equivalent). The agent changes the vault only through `cerbo mcp`, where the daemon checks the vault's `read | auto | approve` permission and runs approvals. Today no tool reaches outside the vault, and the MCP server has no ontology tools. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- Outside-vault access that is easy to explain: "these paths are open, everything else asks".
- Tightening takes effect on the next call, with no session restart.
- One enforcement point, as with vault permissions.
- An agent can find, annotate and extend types using only the MCP tools and the skill.

**Non-Goals:**
- Deleting, moving or executing anything outside the vault.
- Network or shell policy.
- Per-agent host lists; the list belongs to the vault.
- Changing how embeddings or search ranking work.
- Editing bundled or imported ontologies.

## Decisions

**D1. Host access goes through daemon tools, not through wider mounts.** The daemon performs the file operation (`host_list`, `host_read`, `host_write`) on behalf of the session and checks the list at call time. The sandbox stays as it is. Alternative: bind the allowed paths into bubblewrap and re-enable native Read/Write. Rejected: mounts are fixed when the session starts, so tightening would need a restart, "ask per path" cannot be done with mounts at all, and re-enabling native tools would remove the one place where everything is checked.

**D2. One decision function, per call.**

```
host call (path, action)
   │
   ▼
real = resolve(path)          ── relative path ──────────► refuse
   │
   ├─ in never-reachable set? ───────────────────────────► refuse (no card)
   │
   ├─ host access off? ──────────────────────────────────► refuse (tool not listed)
   │
   ├─ covered by list entry with enough mode? ───────────► do it, log
   │
   └─ otherwise ─────────────────────────────────────────► approval card ─► do it / reject / timeout
```

The card reuses the existing approval queue and deadline of the vault flow (D7 of `add-cerboid-daemon`), so a client that connects later still sees it. Approval covers that one call only; there is no "always allow", the user adds a path to the list for that.

**D3. Resolve, then act on the resolved file.** The daemon canonicalizes the path (`..`, links) and checks the result. To limit the gap between check and use, it opens the resolved path without following a final link and compares the opened file's location again before reading or writing. Writes go through the existing durable-write helper (temp file, then rename). Alternative: check the textual path only. Rejected: a link inside an allowed folder would escape it.

**D4. Never-reachable set is fixed in code and checked first.** It contains the daemon's data and runtime directories, agent homes and login files (the existing `protected` list in the sandbox spec), other vaults' `.cerbo` directories, and `~/.ssh`, `~/.gnupg`, `~/.aws`. It is checked before the list and before the approval, so neither a list entry nor a click can open it. Alternative: let the approval decide. Rejected: a tired click on a card is a weak guard for credentials.

**D5. Setting storage.** Host access and the list live in the same per-vault daemon config as the vault permission (`core/src/agent.rs`): `host = off|on`, `host_paths = [{path, mode}]`. The daemon reads it on each call, as with `get_permission`, so changes apply at once. The user sets it with `cerbo vault agent <vault> host …` and in the app; no agent tool reads or changes it.

**D6. The vault stays out of host tools.** A host tool call whose resolved path lies inside the session's vault is refused with a hint to use the vault tools. Otherwise a `write` entry covering a parent folder would let an agent bypass the vault's permission and its change log.

**D7. Ontology tools are thin.** `ontology_list`, `ontology_types` read the vault's ontology map and the `meta.ttl` class declarations. `ontology_create` and `ontology_add_type` follow what `cerbo import-ontology` and the vault import already do: a vault-owned ontology object with namespace `cerbo://vocab/<uuid>/`, a prefix in `.cerbo/ontology-map.json`, and a class declared in `meta.ttl`. They are write calls, so they pass the write queue, approval and change log like page edits. Bundled and imported ontologies are refused so that a vendor ontology is never silently altered.

**D8. Three guides, offered as a tool and as resources.** MCP gives a server exactly one `instructions` string in the initialize result ([MCP lifecycle](https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle)), so more than one text needs another primitive. MCP has three: tools are chosen by the model, resources are read-only documents the application attaches, and prompts are templates the user picks. The `rmcp` 3.11 we use supports resources and prompts. We use a tool and resources. The `guide` tool takes a `topic` and is the channel that always works: in the daemon's sandbox both agents have their own tools off (Claude is started with `tools: []`, OpenCode denies everything except `cerbo_*`), so the agent cannot call a client-side resource reader, and a skill file in the agent home could not be loaded either (Claude Code reads `$CLAUDE_CONFIG_DIR/skills/<name>/SKILL.md`, OpenCode `~/.config/opencode/skills/<name>/SKILL.md`; see [Claude Code skills](https://code.claude.com/docs/en/skills), [OpenCode skills](https://opencode.ai/docs/skills)). Resources `cerbo://guide/<topic>` carry the same texts for clients that show them (Claude Code lists resources with `@`, third-party sources say; unverified for OpenCode). Prompts are not used: a user-picked template does not fit text the model needs on its own.

The three sources are `cli/src/guide/{usage,management,ontology}.md`, with sections marked for write, not-write and host and tools written `{{tool:name}}`. They are rendered for the session's state (write allowed, host on), by the tool and by the resource read alike. A short summary in the server instructions names the topics. The same texts, with every section, are kept as `.agents/skills/cerbo-mcp-<topic>/SKILL.md` for agents that run outside the sandbox. Tests check that every tool a guide names is registered, that a read-only rendering names no write or host tool, that the summary lists every topic, and that the skill files equal their sources. The SPARQL questions in the ontology guide were run against a real vault when written.

**D9. Panel visibility.** The agent panel shows a persistent "host access on" marker with the list, and an approval card for a host call shows the real path, the action and, for writes, a diff (new file: its size).

## Risks / Trade-offs

- [`write` entry covers a large area, such as the whole home folder] → the CLI and app warn when an entry is a parent of the home folder or `/`, and the manual says plainly that this reduces isolation.
- [Check-then-use gap on a path] → D3; remaining risk is documented, as with the vault guarantee: a same-user process can already bypass everything.
- [Large files through MCP are heavy] → host read has a size limit (default 137 KB per call, chunked by offset) and the card shows the size before approving.
- [Guide drifts from the tools] → the tests in D8.
- [Agents ignore the guide] → server instructions carry the short summary, so the method reaches the agent even if it never calls the tool.
- [Approval fatigue for many outside paths] → the user can add the folder to the list; the card text says so.

## Migration Plan

1. Land after `add-cerboid-daemon` and `add-agent-panel`; archive them first so the deltas apply.
2. New settings default to `host = off`, so existing vaults behave as before.
3. Rollback: remove the host tools and the config keys; unknown keys are ignored by older daemons.

## Open Questions

- Default size limit for a host read (D-risks); easily changed later.
