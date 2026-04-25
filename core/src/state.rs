use crate::CerboContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub active_vault_id: Option<String>,
    pub vault_states: HashMap<String, VaultState>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultState {
    pub last_open_page: Option<String>,
}

fn state_path(ctx: &CerboContext) -> Result<PathBuf, String> {
    std::fs::create_dir_all(&ctx.cache_dir).map_err(|e| format!("state_dir mkdir: {e}"))?;
    Ok(ctx.cache_dir.join("state.toml"))
}

pub fn load_state(ctx: &CerboContext) -> Result<State, String> {
    let p = state_path(ctx)?;
    if !p.exists() {
        return Ok(State::default());
    }
    let raw = std::fs::read_to_string(&p).map_err(|e| format!("load_state read: {e}"))?;
    toml::from_str(&raw).map_err(|e| format!("load_state parse: {e}"))
}

pub fn save_state(ctx: &CerboContext, state: &State) -> Result<(), String> {
    let p = state_path(ctx)?;
    let tmp = p.with_extension("toml.tmp");
    let raw = toml::to_string_pretty(state).map_err(|e| format!("save_state serialize: {e}"))?;
    std::fs::write(&tmp, raw).map_err(|e| format!("save_state write tmp: {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("save_state rename: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn save_state_writes_state_toml_in_cache_dir() {
        let tmp = TempDir::new().unwrap();
        let ctx = CerboContext {
            config_dir: tmp.path().join("config"),
            cache_dir: tmp.path().join("cache"),
        };

        save_state(&ctx, &State::default()).unwrap();

        assert!(ctx.cache_dir.join("state.toml").exists());
    }
}
