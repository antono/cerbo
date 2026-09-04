# Agent Instructions

## Guessing vs Asking

Never guess. Always ask perplexity or user if you cannot decide.

## Nix Development Environment

The project uses Nix Flakes for managing dependencies and development
environments.

```bash
# Enter the development shell
nix develop

# List all available outputs (packages, devShells)
nix flake show
```

Agents should load `direnv` if they know how (e.g. by running `direnv allow` or
using a tool that supports it) as the project includes a `.envrc` file.

## Package Management

**ALWAYS use bun** for all package management tasks (installing, adding,
removing dependencies). Run them inside the Nix development shell:

```bash
# Native bun commands (inside devshell)
bun install
bun add <package>
bun add -d <package> # for dev dependencies

# Or via nix develop --command (from outside)
nix develop --command bash -c "bun install"
```

## Bun Nix Manifest

When `package.json` or `bun.lock` changes, regenerate `nix/bun.nix` from the
current lockfile with:

```bash
nix develop --command bunx bun2nix -o nix/bun.nix
```

Use `bun install` first if the lockfile needs to be refreshed, then rerun the
command above so the generated Nix manifest stays in sync.

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
# Run the CLI tool for vault management (inside devshell)
cargo run -p cerbo -- vault list
cargo run -p cerbo -- vault add 'My Vault' /path/to/vault

# Or via nix develop --command (from outside)
nix develop --command bash -c "cargo run -p cerbo -- vault list"
```

### CLI API Changes

Any change to the CLI API **MUST** be reflected in the man page. When adding,
removing, or modifying CLI commands or arguments, update the man page
accordingly. The man page ships with the `cerbo` CLI tool.

**Primary source: `cli/man/cerbo.md`** — edit this Markdown file only.
`cli/man/cerbo.1` is generated at build time by `build.rs` using `mandown`
and is **not committed to git** (listed in `.gitignore`). Never edit `cerbo.1`
directly; changes will be lost on the next build.

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

## Session Completion

When ending a work session:

1. **Run quality gates** (if code changed) - Tests, linters, builds
2. **Clean up** - Clear stashes, prune remote branches
3. **Hand off** - Provide context for next session
