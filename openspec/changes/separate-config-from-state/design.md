# Design: Separate Config from State

## Context

Current architecture:
- `$XDG_CONFIG_DIR/cerbo/vaults.json` stores both persistent config (vault list) and ephemeral state (active_vault_id, last_open_page per vault)
- Format: JSON
- No per-vault cache currently used

Problems:
- Ephemeral state (last page, active vault) pollutes config directory — should be in cache
- JSON is hard to hand-edit compared to TOML
- No separation between user config and per-session state

## Goals / Non-Goals

**Goals:**
- Config in TOML format at `$XDG_CONFIG_DIR/cerbo/`
- Ephemeral state at `$XDG_CACHE_DIR/cerbo/state.toml`
- Per-vault cache at `$XDG_CACHE_DIR/cerbo/<vault_id>/`
- UI settings in separate `$XDG_CONFIG_DIR/cerbo/ui.toml`
- Auto-migration from old JSON config on startup

**Non-Goals:**
- Not implementing a full state management system (simple TOML read/write)
- Not adding encryption (future concern)
- Not cloud sync (future)

## Decisions

### 1. File Structure

| File | Location | Persists | Contains |
|------|----------|----------|-----------|
| `config.toml` | `$XDG_CONFIG_DIR/cerbo/` | Yes | Vaults list |
| `ui.toml` | `$XDG_CONFIG_DIR/cerbo/` | Yes | Theme, font size, sidebar width |
| `state.toml` | `$XDG_CACHE_DIR/cerbo/` | No | active_vault, last_open_page |
| `<vault_id>/` | `$XDG_CACHE_DIR/cerbo/` | No | Search index, render cache |

**Rationale:** Follows XDG base spec correctly. Cache is for transient data.

### 2. Config Loading Order

```
on startup:
1. Load config.toml (vaults)
2. Load ui.toml (UI settings)  [optional, use defaults if missing]
3. Load state.toml          [optional, init empty if missing]
4. Check for old vaults.json → if found, migrate and delete
```

### 3. Migration Strategy

**When:** On startup, before any other config load

**Steps:**
1. Check if `$XDG_CONFIG_DIR/cerbo/vaults.json` exists
2. If yes:
   - Parse JSON
   - Write config.toml (vaults array)
   - Write state.toml (active_vault_id, per-vault last_open_page)
   - Write ui.toml (any UI settings found in JSON, else defaults)
   - Delete old vaults.json
3. Log migration completed

**Rollback:** None needed — one-way migration from JSON to TOML.

### 4. State Persistence

**Write-through:** Save state.toml immediately on change (active vault, last page) — no debouncing for v1.

### 5. Config Modules in Core

```rust
// core/src/config.rs
pub mod config {
    pub fn load_config(ctx: &CerboContext) -> Result<Config, String>
    pub fn save_config(ctx: &CerboContext, config: &Config) -> Result<(), String>
    pub fn load_ui(ctx: &CerboContext) -> Result<UiSettings, String>
    pub fn save_ui(ctx: &CerboContext, ui: &UiSettings) -> Result<(), String>
    pub fn load_state(ctx: &CerboContext) -> Result<State, String>
    pub fn save_state(ctx: &CerboContext, state: &State) -> Result<(), String>
}

mod migration {
    pub fn migrate_if_needed(ctx: &CerboContext) -> Result<bool, String>  // returns true if migrated
    // TODO: Remove after v0.3.0 - migration code
}
```

Data models:
```rust
#[derive(Serialize, Deserialize)]
struct Config {
    vaults: Vec<Vault>,
}

#[derive(Serialize, Deserialize)]
struct UiSettings {
    theme: Option<String>,
    font_size: Option<u8>,
    sidebar_width: Option<u16>,
}

#[derive(Serialize, Deserialize)]
struct State {
    active_vault_id: Option<String>,
    vault_states: HashMap<String, VaultState>,
}

struct VaultState {
    last_open_page: Option<String>,
}
```

### 6. Serde TOML

Add to `core/Cargo.toml`:
```toml
serde_toml = "0.8"
```

Alternative considered: `toml` crate directly — rejected because `serde_toml` integrates with `Serialize/Deserialize`.

## Risks / Trade-offs

- [Risk] First upgrade may lose state if migration fails
  - [Mitigation] Migration logs all steps, validates JSON before writing TOML
- [Risk] Users with hand-edited JSON lose changes
  - [Mitigation] Migration reads old format, writes new — preserves data
- [Risk] Cache directory grows unbounded
  - [Mitigation] Future: add cache eviction (out of scope for v1)

## Migration Code TODO

```rust
// TODO: Remove this migration code after v0.3.0 release
// The old vaults.json format was deprecated in v0.2.0.
// Remove this block after all users have upgraded (estimated: 2025-Q2).
```

## Open Questions

- Q: Should state.toml use `serde_hjson` or manual `toml`? → Use `toml::to_string` for simplicity
- Q: Should we keep vaults list in separate file per vault? → No, single config.toml is simpler