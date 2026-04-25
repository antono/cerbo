use crate::{config, state, ui_settings, vault, CerboContext};

// TODO: Remove after v0.3.0
pub fn migrate_if_needed(ctx: &CerboContext) -> Result<bool, String> {
    let legacy = ctx.config_dir.join("vaults.json");
    if !legacy.exists() {
        return Ok(false);
    }

    let raw = std::fs::read_to_string(&legacy).map_err(|e| format!("migrate read legacy: {e}"))?;
    let legacy_file: vault::VaultsFile =
        serde_json::from_str(&raw).map_err(|e| format!("migrate parse legacy: {e}"))?;

    let config = config::Config {
        vaults: legacy_file.vaults,
    };
    config::save_config(ctx, &config)?;
    ui_settings::save_ui_settings(ctx, &ui_settings::UiSettings::default())?;
    state::save_state(ctx, &state::State::default())?;

    std::fs::remove_file(&legacy).map_err(|e| format!("migrate remove legacy: {e}"))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn migrate_legacy_vaults_json_to_toml_files() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let cache_dir = tmp.path().join("cache");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&cache_dir).unwrap();

        let legacy = vault::VaultsFile {
            vaults: vec![vault::Vault {
                id: "vault-1".into(),
                name: "Vault One".into(),
                path: tmp.path().join("vault-1"),
            }],
        };
        let legacy_path = config_dir.join("vaults.json");
        std::fs::write(&legacy_path, serde_json::to_string_pretty(&legacy).unwrap()).unwrap();

        let ctx = CerboContext {
            config_dir: config_dir.clone(),
            cache_dir: cache_dir.clone(),
        };

        let migrated = migrate_if_needed(&ctx).unwrap();
        assert!(migrated);
        assert!(!legacy_path.exists());
        assert!(config_dir.join("vaults.toml").exists());
        assert!(config_dir.join("ui.toml").exists());
        assert!(cache_dir.join("state.toml").exists());

        let config = config::load_config(&ctx).unwrap();
        assert_eq!(config.vaults.len(), 1);
        assert_eq!(config.vaults[0].id, "vault-1");
    }

    #[test]
    fn migrate_legacy_json_creates_state_file() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let cache_dir = tmp.path().join("cache");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&cache_dir).unwrap();

        let legacy = vault::VaultsFile {
            vaults: vec![vault::Vault {
                id: "vault-1".into(),
                name: "Vault One".into(),
                path: tmp.path().join("vault-1"),
            }],
        };
        std::fs::write(
            config_dir.join("vaults.json"),
            serde_json::to_string_pretty(&legacy).unwrap(),
        )
        .unwrap();

        let ctx = CerboContext {
            config_dir,
            cache_dir: cache_dir.clone(),
        };

        migrate_if_needed(&ctx).unwrap();

        assert!(cache_dir.join("state.toml").exists());
    }
}
