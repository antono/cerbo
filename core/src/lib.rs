use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CerboContext {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}

pub mod index;
pub mod page;
pub mod paths;
pub mod rename;
pub mod slug;
pub mod vault;
