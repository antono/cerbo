use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CerboContext {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}

pub mod annotation;
pub mod config;
pub mod context;
pub mod index;
pub mod links;
pub mod migration;
pub mod object;
pub mod page;
pub mod paths;
pub mod state;
pub mod ui_settings;
pub mod vault;

#[cfg(any(test, feature = "test-utils"))]
pub mod fixtures;
