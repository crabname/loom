use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::paths::AppPaths;

const APP_STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub version: u32,
    pub active_workspace: usize,
    pub workspaces: Vec<WorkspaceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceRef {
    Local {
        path: PathBuf,
    },
    Cloud {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        organization_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
    },
}

impl AppState {
    pub fn empty() -> Self {
        Self {
            version: APP_STATE_VERSION,
            active_workspace: 0,
            workspaces: Vec::new(),
        }
    }

    pub fn load(app_paths: &AppPaths) -> Self {
        if !app_paths.app_state.is_file() {
            return Self::empty();
        }

        let content = match fs::read_to_string(&app_paths.app_state) {
            Ok(content) => content,
            Err(_) => return Self::empty(),
        };

        match serde_yaml::from_str::<AppState>(&content) {
            Ok(state) if state.version == APP_STATE_VERSION => state,
            _ => Self::empty(),
        }
    }

    pub fn save(&self, app_paths: &AppPaths) -> Result<(), String> {
        if let Some(parent) = app_paths.app_state.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let content = serde_yaml::to_string(self).map_err(|error| error.to_string())?;
        fs::write(&app_paths.app_state, content).map_err(|error| error.to_string())
    }

    pub fn cloud_workspaces(&self) -> impl Iterator<Item = &WorkspaceRef> {
        self.workspaces
            .iter()
            .filter(|workspace| matches!(workspace, WorkspaceRef::Cloud { .. }))
    }
}
