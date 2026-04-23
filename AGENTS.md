# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd prime` for full
workflow context.

## Nix Development Environment

The project uses Nix Flakes for managing dependencies and development environments.

```bash
# Enter the development shell
nix develop

# List all available outputs (packages, devShells)
nix flake show
```

## Package Management

**ALWAYS use bun** for all package management tasks (installing, adding, removing dependencies). Run them inside the Nix development shell:

```bash
nix develop --command bash -c "bun install"
nix develop --command bash -c "bun add <package>"
nix develop --command bash -c "bun add -d <package>" # for dev dependencies
```

## Building the App

**ALWAYS use nix build** for all build commands:

```bash
# Build the CLI tool
nix build .#cerbo

# Build the Desktop App
nix build .#cerbo-desktop

# Run build checks
nix build .#cerbo .#cerbo-desktop
```

### Rust Workspace

The backend is split into three crates:
- `core`: Shared domain logic (indexing, renames, CRUD).
- `cli`: Standalone `cerbo` binary.
- `src-tauri`: Desktop GUI (`cerbo-desktop`).

### CLI usage (cerbo)

**ALWAYS run cargo commands inside `nix develop`**.

```bash
# Run the CLI tool for vault management
nix develop --command bash -c "cargo run -p cerbo -- vault list"
nix develop --command bash -c "cargo run -p cerbo -- vault add 'My Vault' /path/to/vault"

# Run page management commands
nix develop --command bash -c "cargo run -p cerbo -- page list <vault-id>"
nix develop --command bash -c "cargo run -p cerbo -- page create <vault-id> 'New Page'"

# Run headless watcher
nix develop --command bash -c "cargo run -p cerbo -- watch"
```

## Debugging with Chrome DevTools MCP

To debug the Tauri app using AI-driven browser tools (Chrome DevTools MCP), use the specialized debug script:

```bash
# Start the app with remote debugging ports enabled (9222)
nix develop --command bash -c "npm run dev-debug"
```

Once the app is running:
1. Use `mcp_chrome-devtools_list_pages` to find the app's webview.
2. Use `mcp_chrome-devtools_take_snapshot` or `mcp_chrome-devtools_evaluate_script` to inspect the UI state.
3. Note: On Windows (WebView2), this uses the standard Chrome DevTools Protocol (CDP). On Linux (WebKitGTK), it enables the WebKit inspector server on port 9222.

## Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work atomically
bd close <id>         # Complete work
bd dolt push          # Push beads data to remote
```

## Non-Interactive Shell Commands

**ALWAYS use non-interactive flags** with file operations to avoid hanging on
confirmation prompts.

Shell commands like `cp`, `mv`, and `rm` may be aliased to include `-i`
(interactive) mode on some systems, causing the agent to hang indefinitely
waiting for y/n input.

**Use these forms instead:**

```bash
# Force overwrite without prompting
cp -f source dest           # NOT: cp source dest
mv -f source dest           # NOT: mv source dest
rm -f file                  # NOT: rm file

# For recursive operations
rm -rf directory            # NOT: rm -r directory
cp -rf source dest          # NOT: cp -r source dest
```

**Other commands that may prompt:**

- `scp` - use `-o BatchMode=yes` for non-interactive
- `ssh` - use `-o BatchMode=yes` to fail instead of prompting
- `apt-get` - use `-y` flag

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->

## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full
workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown
  TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT
complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs
   follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**

- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

<!-- END BEADS INTEGRATION -->
