use std::fs;
use std::path::{Path, PathBuf};

use super::local::slugify;

const APP_DIR_NAME: &str = "loom";

pub struct AppPaths {
    #[allow(dead_code)]
    pub root: PathBuf,
    pub app_state: PathBuf,
    pub local_workspaces: PathBuf,
    pub cloud: PathBuf,
}

impl AppPaths {
    pub fn ensure() -> Result<Self, String> {
        let root = dirs::data_dir()
            .ok_or("could not determine application data directory")?
            .join(APP_DIR_NAME);

        let paths = Self {
            app_state: root.join("app-state.yml"),
            local_workspaces: root.join("local").join("workspaces"),
            cloud: root.join("cloud"),
            root,
        };

        fs::create_dir_all(&paths.local_workspaces).map_err(|error| error.to_string())?;
        fs::create_dir_all(paths.cloud.join("cache")).map_err(|error| error.to_string())?;

        Ok(paths)
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
        let _ = std::fs::create_dir_all(root.join("local").join("workspaces"));
        let _ = std::fs::create_dir_all(root.join("cloud").join("cache"));

        Self {
            root: root.clone(),
            app_state: root.join("app-state.yml"),
            local_workspaces: root.join("local").join("workspaces"),
            cloud: root.join("cloud"),
        }
    }
}
