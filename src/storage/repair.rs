use std::path::{Path, PathBuf};

use super::app_state::{AppState, WorkspaceRef};
use super::paths::AppPaths;

pub fn remove_workspace_refs(app_paths: &AppPaths, paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }

    let mut state = AppState::load(app_paths);
    let before = state.workspaces.len();
    state.workspaces.retain(|workspace_ref| match workspace_ref {
        WorkspaceRef::Local { path } => !paths.iter().any(|removed| removed == path),
        WorkspaceRef::Cloud { .. } => true,
    });

    if state.workspaces.len() != before {
        if state.active_workspace >= state.workspaces.len() {
            state.active_workspace = state.workspaces.len().saturating_sub(1);
        }
        state.save(app_paths)?;
    }

    Ok(())
}

pub fn remove_missing_workspace_refs(app_paths: &AppPaths) -> Result<Vec<PathBuf>, String> {
    let state = AppState::load(app_paths);
    let missing = state
        .workspaces
        .iter()
        .filter_map(|workspace_ref| {
            let WorkspaceRef::Local { path } = workspace_ref else {
                return None;
            };
            if path.is_dir() {
                None
            } else {
                Some(path.clone())
            }
        })
        .collect::<Vec<_>>();

    remove_workspace_refs(app_paths, &missing)?;
    Ok(missing)
}

pub fn clear_local_workspace_dir(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|error| {
            format!("failed to remove {}: {error}", path.display())
        })?;
    }
    Ok(())
}
