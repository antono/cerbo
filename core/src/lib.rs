use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CerboContext {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}

/// Vault-scoped context for operations within a specific vault
/// Similar to how Git operates within a repository
#[derive(Debug, Clone)]
pub struct VaultContext {
    pub vault_path: PathBuf,  // Directory containing .cerbo/
    pub global: CerboContext,  // Global config/cache access
}

impl VaultContext {
    /// Create a VaultContext from an explicit vault path
    pub fn from_path(vault_path: PathBuf) -> Result<Self, String> {
        let cerbo_dir = vault_path.join(".cerbo");
        if !cerbo_dir.exists() {
            return Err(format!("Not a Cerbo vault (no .cerbo/ directory): {}", vault_path.display()));
        }
        
        let global = context::CoreContext::new()?.into();
        Ok(Self { vault_path, global })
    }
    
    /// Discover vault from current working directory (walks up like Git)
    pub fn from_cwd() -> Result<Self, String> {
        let cwd = std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?;
        
        Self::discover_from(&cwd)
    }
    
    /// Walk up from given directory to find .cerbo/
    fn discover_from(start: &std::path::Path) -> Result<Self, String> {
        let mut current = start;
        
        loop {
            let cerbo_dir = current.join(".cerbo");
            if cerbo_dir.exists() && cerbo_dir.is_dir() {
                return Self::from_path(current.to_path_buf());
            }
            
            match current.parent() {
                Some(parent) => current = parent,
                None => return Err(format!(
                    "Not inside a Cerbo vault (no .cerbo/ found). \
                    Run 'cerbo vault init' or use --vault <path>"
                )),
            }
        }
    }
    
    /// Get the .cerbo directory path
    pub fn cerbo_dir(&self) -> PathBuf {
        self.vault_path.join(".cerbo")
    }
    
    /// Get the objects directory path
    pub fn objects_dir(&self) -> PathBuf {
        self.cerbo_dir().join("objects")
    }
    
    /// Get path to a specific object
    pub fn object_path(&self, uuid: &str) -> PathBuf {
        self.objects_dir().join(uuid)
    }
}

impl From<context::CoreContext> for CerboContext {
    fn from(core: context::CoreContext) -> Self {
        Self {
            config_dir: core.config_dir,
            cache_dir: core.cache_dir,
        }
    }
}

pub mod config;
pub mod context;
pub mod annotations;
pub mod index;
pub mod metadata_index;
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
