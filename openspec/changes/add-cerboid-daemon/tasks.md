## 1. Protocol and crate

- [ ] 1.1 Add the `cerboid` crate to the workspace and wire it into `nix/pkgs.nix` as a package output
- [ ] 1.2 Define RPC request, response and event types in `core` (newline-delimited JSON) with round-trip tests
- [ ] 1.3 Add protocol version and the connect handshake with tests for a mismatch

## 2. Daemon core

- [ ] 2.1 Single-instance lock, owner-only socket directory, stale socket replacement, with tests
- [ ] 2.2 Vault registry: serve every vault in the list and pick up `vault add`/`remove` while running
- [ ] 2.3 Per-vault write queue using the durable-write helpers; concurrency test with two writers
- [ ] 2.4 Read endpoints: pages, attachments, links, backlinks, outgoing links, text and semantic search, SPARQL
- [ ] 2.5 Write endpoints: create, edit, rename, move, trash, link
- [ ] 2.6 Hold the app lock shared for the daemon's lifetime; test that automatic migration is skipped
- [ ] 2.7 File watcher and change events to subscribers, including outside edits
- [ ] 2.8 Graceful stop: finish queued writes, then exit

## 3. Permissions

- [ ] 3.1 Store `read | auto | approve` per vault, default `read`; apply changes to connected agents at once
- [ ] 3.2 Enforce on every agent write; refuse at `read` with a clear message
- [ ] 3.3 Approval queue: diff, deadline (default 10 min, configurable), answer from any client, rejection and timeout results
- [ ] 3.4 Change log of applied agent writes (time, page, agent name)
- [ ] 3.5 `cerbo vault agent <vault> [read|auto|approve]`

## 4. MCP shim

- [ ] 4.1 `cerbo mcp --vault` stdio server forwarding to the daemon; start the daemon if it is down
- [ ] 4.2 Read tools and their error results
- [ ] 4.3 Write tools, hidden at `read`; deletes go to trash
- [ ] 4.4 Integration test driving the server with a scripted MCP client against a temporary vault

## 5. CLI

- [ ] 5.1 `cerbo daemon start|stop|status [--json]`
- [ ] 5.2 Daemon-first path for existing commands with direct fallback when no daemon runs
- [ ] 5.3 Update `cli/man/cerbo.md` for `daemon`, `mcp`, `vault agent` and the permission levels

## 6. Nix and systemd

- [ ] 6.1 Export `homeManagerModules.default` with `services.cerbo.daemon.{enable,package}`
- [ ] 6.2 Define `systemd.user.services.cerboid` and `systemd.user.sockets.cerboid` (restart on failure, socket activation)
- [ ] 6.3 Add `vaults.<name>.agent` and apply it at service start
- [ ] 6.4 NixOS VM or `home-manager` build check that the unit is generated and disabled by default

## 7. Desktop client (staged)

- [ ] 7.1 Daemon client in `src-tauri`: connect, start if absent (not when socket activation exists), handshake, reconnect
- [ ] 7.2 Daemon-unavailable banner with Retry
- [ ] 7.3 Move page, attachment and link commands behind the client; reload or show the external-change dialog on change events
- [ ] 7.4 Move index, backlinks, similar notes and search behind the client
- [ ] 7.5 Remove the direct vault path from the app
- [ ] 7.6 Quit confirmation mentions a still-running agent; the daemon outlives the app

## 8. Verification

- [ ] 8.1 Update and extend e2e tests (start with and without a running daemon, agent edit reaches the open page)
- [ ] 8.2 Run cargo tests, vitest and `nix build .#cerbo .#cerbo-desktop`
