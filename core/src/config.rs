use crate::paths;
use crate::{vault::Vault, CerboContext};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub vaults: Vec<Vault>,
}

fn config_path(ctx: &CerboContext) -> Result<PathBuf, String> {
    Ok(paths::config_dir(ctx.config_dir.clone())?.join("vaults.toml"))
}

pub fn load_config(ctx: &CerboContext) -> Result<Config, String> {
    let p = config_path(ctx)?;
    if !p.exists() {
        return Ok(Config::default());
    }
    let raw = std::fs::read_to_string(&p).map_err(|e| format!("load_config read: {e}"))?;
    toml::from_str(&raw).map_err(|e| format!("load_config parse: {e}"))
}

pub fn save_config(ctx: &CerboContext, config: &Config) -> Result<(), String> {
    let p = config_path(ctx)?;
    let tmp = p.with_extension("toml.tmp");
    let raw = toml::to_string_pretty(config).map_err(|e| format!("save_config serialize: {e}"))?;
    std::fs::write(&tmp, raw).map_err(|e| format!("save_config write tmp: {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("save_config rename: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn save_config_writes_config_toml_in_config_dir() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let ctx = CerboContext {
            config_dir: config_dir.clone(),
            cache_dir: tmp.path().join("cache"),
        };

        save_config(&ctx, &Config::default()).unwrap();

        assert!(config_dir.join("vaults.toml").exists());
    }
}
