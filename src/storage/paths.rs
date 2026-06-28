use std::path::{Path, PathBuf};

use super::bootstrap::{ensure_data_root, BootstrapConfig};
use super::local::slugify;

pub struct AppPaths {
    pub root: PathBuf,
    pub app_state: PathBuf,
    pub local_workspaces: PathBuf,
    #[allow(dead_code)]
    pub cloud: PathBuf,
}

impl AppPaths {
    pub fn ensure() -> Result<Self, String> {
        let root = BootstrapConfig::effective_data_root();
        ensure_data_root(&root)?;

        Ok(Self::from_root(root))
    }

    pub fn from_root(root: PathBuf) -> Self {
        Self {
            app_state: root.join("app-state.yml"),
            local_workspaces: root.join("local").join("workspaces"),
            cloud: root.join("cloud"),
            root,
        }
    }

    pub fn uses_custom_root(&self) -> bool {
        self.root != BootstrapConfig::default_data_root()
    }

    pub fn managed_workspace_path(&self, name: &str) -> PathBuf {
        self.local_workspaces.join(slugify(name))
    }

    pub fn unique_managed_workspace_path(&self, name: &str, exclude: Option<&Path>) -> PathBuf {
        let base = self.managed_workspace_path(name);
        if !base.exists() || exclude.is_some_and(|path| path == base) {
            return base;
        }

        let slug = slugify(name);
        for index in 2.. {
            let candidate = self.local_workspaces.join(format!("{slug}-{index}"));
            if !candidate.exists() || exclude.is_some_and(|path| path == candidate) {
                return candidate;
            }
        }

        base
    }

    pub fn fallback() -> Self {
        let root = std::env::temp_dir().join("loom");
        let _ = ensure_data_root(&root);
        Self::from_root(root)
    }
}
