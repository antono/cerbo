# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd prime` for full
workflow context.

## Building the App

**ALWAYS use devenv tasks** for all build commands:

```bash
# Enter the devenv shell (for ad-hoc commands)
devenv shell

# Run defined tasks (inside the shell)
devenv tasks run app:dev        # Desktop App (Tauri hot reload)
devenv tasks run app:build      # Desktop production build
devenv tasks run app:check      # Workspace type check (cargo check)
devenv tasks run cli:build      # CLI tool build (cerbo binary)
devenv tasks run core:test      # Run core logic unit tests

# Frontend commands (inside the shell)
devenv tasks run frontend:dev    # Vite dev server (hot reload)
devenv tasks run frontend:build   # Vite production build
devenv tasks run frontend:check   # TypeScript check
```

### Alternative: run commands in a one-liner

```bash
devenv shell --no-tui -c "devenv tasks run app:check"
devenv shell --no-tui -c "devenv tasks run cli:build"
devenv shell --no-tui -c "devenv tasks run core:test"
```

### Rust Workspace

The backend is split into three crates:
- `core`: Shared domain logic (indexing, renames, CRUD).
- `cli`: Standalone `cerbo` binary.
- `src-tauri`: Desktop GUI (`cerbo-desktop`).

### CLI usage (cerbo)

```bash
# Vault management
cargo run -p cerbo -- vault list
cargo run -p cerbo -- vault add "My Vault" /path/to/vault

# Page management
cargo run -p cerbo -- page list <vault-id>
cargo run -p cerbo -- page create <vault-id> "New Page"

# Headless watcher
cargo run -p cerbo -- watch
```

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
