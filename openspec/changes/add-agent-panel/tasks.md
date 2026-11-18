## 1. ACP in the daemon

- [x] 1.1 ACP client over stdio: initialize, new session, prompt, cancel, streamed updates, permission requests
- [x] 1.2 Agent registry for OpenCode and Claude Code: find command, report unavailable with reason
- [x] 1.3 Process supervision: start, stop, crash reporting with the tail of stderr
- [ ] 1.4 Pass `cerbo mcp --vault <v>` as the session's MCP server; set working directory to the vault; apply each CLI's file-tool restrictions

## 1b. Sandbox (D8–D10)

- [x] 1b.1 bwrap argument builder, probe at session start, clear error when it fails; `agent.sandbox = "none"` opt-out with a warning
- [ ] 1b.2 Per-session socket: agent-only, bound to the vault, removed on end; `cerbo mcp` reads `CERBO_SOCKET` (the `cerbo mcp` half is done; the daemon-side socket needs the session manager)
- [x] 1b.3 Agent homes with generated config (built-in tools denied, `cerbo` MCP allowed) and the login file bound in
- [ ] 1b.4 Nix: ship bubblewrap with cerboid (`CERBO_BWRAP`); tests for direct write refused, main socket hidden, hello as app refused (the first two are done in `cerboid/tests/sandbox_bwrap.rs`; the last needs the per-session socket)

## 1c. Desktop connection (subset of add-cerboid-daemon 7.1–7.2)

- [x] 1c.1 Daemon client in `src-tauri` for session calls and events only: connect, start if absent, reconnect
- [x] 1c.2 Daemon-unavailable state in the Agent tab with Retry

## 2. Sessions

- [ ] 2.1 Event model and append-only per-vault session store outside the vault (owner-only permissions)
- [ ] 2.2 Session API: create, send, cancel, end, list, delete, subscribe; resume or transcript-as-context
- [ ] 2.3 Map agent tool-permission requests to approval cards under the vault's mode (D3)
- [x] 2.4 Replay tests from recorded ACP logs (no real agent needed)

## 3. CLI

- [ ] 3.1 `cerbo agent run --vault [--agent] [--json] "<prompt>"` with exit codes for success, rejection, timeout, failure
- [ ] 3.2 `cerbo agent list` and `cerbo agent stop <session>`
- [ ] 3.3 Update `cli/man/cerbo.md`

## 4. Agent tab

- [x] 4.1 Session store on the frontend: subscribe to events, build the feed from the event list
- [x] 4.2 Feed: messages, streaming text, tool-call rows, scroll behaviour, virtualization
- [x] 4.3 Header: agent picker, connection state, New session, Sessions list
- [x] 4.4 Composer: Enter/Shift+Enter, page context chip, `@` mentions, cancel while working
- [x] 4.5 Approval cards with diff; waiting mark on the tab icon
- [x] 4.6 Permission control with confirmation when widening
- [x] 4.7 Changed-page links and refresh of the open page
- [x] 4.8 Empty and error states (no agent, daemon unreachable)

## 5. Verification

- [ ] 5.1 e2e with a scripted fake ACP agent: chat, tool rows, approve and reject, close and reopen the window mid-session
- [ ] 5.2 Headless e2e: `cerbo agent run` on read, auto and approve vaults, including the approval timeout
- [ ] 5.3 Run cargo tests, vitest and `nix build .#cerbo .#cerbo-desktop`
