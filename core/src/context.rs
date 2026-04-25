use std::path::PathBuf;
use xdg::BaseDirectories;

#[derive(Debug, Clone)]
pub struct CoreContext {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl CoreContext {
    pub fn new() -> Result<Self, String> {
        let xdg =
            BaseDirectories::with_prefix("cerbo").map_err(|e| format!("XDG directories: {e}"))?;
        Ok(Self {
            config_dir: xdg.get_config_home(),
            cache_dir: xdg.get_cache_home(),
        })
    }
}
