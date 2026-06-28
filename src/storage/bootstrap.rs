use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const BOOTSTRAP_VERSION: u32 = 1;
const BOOTSTRAP_FILE: &str = "bootstrap.yml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_root: Option<PathBuf>,
}

impl BootstrapConfig {
    pub fn load() -> Self {
        let path = Self::bootstrap_file();
        if !path.is_file() {
            return Self::default();
        }

        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                eprintln!("failed to read {}: {error}", path.display());
                return Self::default();
            }
        };

        match serde_yaml::from_str::<Self>(&content) {
            Ok(config) if config.version == BOOTSTRAP_VERSION => config,
            Err(error) => {
                eprintln!("failed to parse {}: {error}", path.display());
                Self::default()
            }
            _ => {
                eprintln!("unsupported bootstrap version in {}", path.display());
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::bootstrap_file();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let content = serde_yaml::to_string(self).map_err(|error| error.to_string())?;
        fs::write(path, content).map_err(|error| error.to_string())
    }

    pub fn fixed_config_dir() -> PathBuf {
        dirs::data_dir()
            .map(|dir| dir.join("loom"))
            .unwrap_or_else(|| std::env::temp_dir().join("loom"))
    }

    pub fn bootstrap_file() -> PathBuf {
        Self::fixed_config_dir().join(BOOTSTRAP_FILE)
    }

    pub fn default_data_root() -> PathBuf {
        Self::fixed_config_dir()
    }

    pub fn effective_data_root() -> PathBuf {
        let config = Self::load();
        config
            .data_root
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(Self::default_data_root)
    }

    pub fn set_data_root(path: PathBuf) -> Result<(), String> {
        Self {
            version: BOOTSTRAP_VERSION,
            data_root: Some(path),
        }
        .save()
    }

    pub fn reset_data_root() -> Result<(), String> {
        Self {
            version: BOOTSTRAP_VERSION,
            data_root: None,
        }
        .save()
    }
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            version: BOOTSTRAP_VERSION,
            data_root: None,
        }
    }
}

pub fn ensure_data_root(root: &Path) -> Result<(), String> {
    fs::create_dir_all(root.join("local").join("workspaces"))
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(root.join("cloud").join("cache")).map_err(|error| error.to_string())?;
    Ok(())
}
