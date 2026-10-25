use std::path::PathBuf;
use xdg::BaseDirectories;

#[derive(Debug, Clone)]
pub struct CoreContext {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl CoreContext {
    pub fn new() -> Result<Self, String> {
        let xdg = BaseDirectories::with_prefix("cerbo");
        Ok(Self {
            config_dir: xdg
                .get_config_home()
                .ok_or("XDG directories: config home not found")?,
            cache_dir: xdg
                .get_cache_home()
                .ok_or("XDG directories: cache home not found")?,
        })
    }
}
