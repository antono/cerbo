## Context

Currently, Cerbo uses different approaches for path resolution:
- **CLI**: Uses `directories::ProjectDirs` from the `directories` crate
- **Desktop**: Uses Tauri's `app.path()` API

Neither strictly follows the XDG Base Directory Specification. The XDG spec is the standard for Unix-like systems and respects environment variables like `$XDG_CONFIG_HOME`, `$XDG_CACHE_HOME`.

## Goals / Non-Goals

**Goals:**
- Use `xdg` crate for proper XDG Base Directory Specification compliance
- Provide `cerbo info` command showing all config/cache paths
- Show vault list with page counts in the info command
- Support `--info` flag in desktop app

**Non-Goals:**
- Migrate existing data from old locations (users can manually move if needed)
- Add data directory (only config and cache needed for now)

## Decisions

### 1: Use `xdg` crate over `directories::ProjectDirs`

**Rationale:** The `xdg` crate is purpose-built for XDG spec compliance and provides:
- Proper handling of `$XDG_CONFIG_HOME`, `$XDG_CACHE_HOME` env vars
- Fallback to defaults (`~/.config`, `~/.cache`) when env vars not set
- Cross-platform with Unix-first design

**Alternative considered:** Keep using `directories::ProjectDirs`. This works but isn't strict XDG.

### 2: Single `CoreContext` in core crate for path resolution

**Rationale:** Both CLI and desktop need paths. Centralize in core with `CoreContext::new()` that uses `xdg::BaseDirectories`.

```rust
// In core/src/context.rs
pub struct CoreContext {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl CoreContext {
    pub fn new() -> Result<Self, String> {
        let xdg = BaseDirectories::with_prefix("cerbo")
            .map_err(|e| format!("XDG directories: {e}"))?;
        Ok(Self {
            config_dir: xdg.get_config_home(),
            cache_dir: xdg.get_cache_home(),
        })
    }
}
```

### 3: Info command output format

```
Config:  ~/.config/cerbo
Cache:  ~/.cache/cerbo

Vaults: 2 registered
├── my-vault (./vaults/my-vault) - 42 pages
└── work-notes (./notes/work) - 156 pages
```

## Risks / Trade-offs

- **Migration**: Existing CLI users have data in old `ProjectDirs` location. No auto-migration (users can move manually).
- **Windows**: The `xdg` crate is primarily Unix-focused. May need adjustment for Windows cross-compile, but config/cache paths work similarly.

## Open Questions

- Should `--info` in desktop open a dialog or print to stderr like CLI?