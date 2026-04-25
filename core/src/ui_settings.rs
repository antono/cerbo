use crate::paths;
use crate::CerboContext;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSettings {
    pub theme: Option<String>,
    pub font_size: Option<u8>,
    pub sidebar_width: Option<u16>,
    pub right_sidebar_visible: Option<bool>,
    pub window_bounds: Option<WindowBounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    pub width: f64,
    pub height: f64,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: None,
            font_size: None,
            sidebar_width: None,
            right_sidebar_visible: None,
            window_bounds: None,
        }
    }
}

fn ui_settings_path(ctx: &CerboContext) -> Result<PathBuf, String> {
    Ok(paths::config_dir(ctx.config_dir.clone())?.join("ui.toml"))
}

pub fn load_ui_settings(ctx: &CerboContext) -> Result<UiSettings, String> {
    let p = ui_settings_path(ctx)?;
    if !p.exists() {
        return Ok(UiSettings::default());
    }
    let raw = std::fs::read_to_string(&p).map_err(|e| format!("load_ui_settings read: {e}"))?;
    toml::from_str(&raw).map_err(|e| format!("load_ui_settings parse: {e}"))
}

pub fn save_ui_settings(ctx: &CerboContext, settings: &UiSettings) -> Result<(), String> {
    let p = ui_settings_path(ctx)?;
    let tmp = p.with_extension("toml.tmp");
    let raw =
        toml::to_string_pretty(settings).map_err(|e| format!("save_ui_settings serialize: {e}"))?;
    std::fs::write(&tmp, raw).map_err(|e| format!("save_ui_settings write tmp: {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("save_ui_settings rename: {e}"))?;
    Ok(())
}

pub fn merge_ui_settings(
    base: &UiSettings,
    theme: Option<String>,
    font_size: Option<u8>,
    sidebar_width: Option<u16>,
    right_sidebar_visible: Option<bool>,
    window_bounds: Option<WindowBounds>,
) -> UiSettings {
    UiSettings {
        theme: theme.or_else(|| base.theme.clone()),
        font_size: font_size.or(base.font_size),
        sidebar_width: sidebar_width.or(base.sidebar_width),
        right_sidebar_visible: right_sidebar_visible.or(base.right_sidebar_visible),
        window_bounds: window_bounds.or_else(|| base.window_bounds.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn save_ui_settings_writes_ui_toml_in_config_dir() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let ctx = CerboContext {
            config_dir: config_dir.clone(),
            cache_dir: tmp.path().join("cache"),
        };

        save_ui_settings(&ctx, &UiSettings::default()).unwrap();

        assert!(config_dir.join("ui.toml").exists());
    }

    #[test]
    fn save_ui_settings_persists_sidebar_width() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let ctx = CerboContext {
            config_dir: config_dir.clone(),
            cache_dir: tmp.path().join("cache"),
        };

        let settings = UiSettings {
            theme: Some("dark".into()),
            font_size: Some(16),
            sidebar_width: Some(320),
            right_sidebar_visible: Some(false),
            window_bounds: None,
        };
        save_ui_settings(&ctx, &settings).unwrap();

        let loaded = load_ui_settings(&ctx).unwrap();
        assert_eq!(loaded.sidebar_width, Some(320));
        assert_eq!(loaded.right_sidebar_visible, Some(false));
    }
}
