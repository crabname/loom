use crate::domain::Workspace;
use crate::storage::{
    default_workspace_collection_paths, remove_missing_workspace_refs, remove_workspace_refs,
    AppPaths, AppState, LocalStorageProvider, WorkspaceRef,
};

use super::{TabSource, WorkspaceBinding};

pub struct StartupWorkspaces {
    pub workspaces: Vec<Workspace>,
    pub bindings: Vec<WorkspaceBinding>,
    pub collection_paths: Vec<Vec<String>>,
    pub active_workspace: usize,
    pub startup_warnings: Vec<String>,
}

pub fn load_startup_workspaces(app_paths: &AppPaths) -> StartupWorkspaces {
    let mut startup_warnings = match remove_missing_workspace_refs(app_paths) {
        Ok(missing) => missing
            .into_iter()
            .map(|path| {
                format!(
                    "removed missing workspace from app state: {}",
                    path.display()
                )
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            eprintln!("failed to repair app state: {error}");
            vec![format!("failed to repair app state: {error}")]
        }
    };

    let app_state = AppState::load(app_paths);
    if app_state.workspaces.is_empty() {
        startup_warnings.extend(demo_startup().startup_warnings);
        return StartupWorkspaces {
            startup_warnings,
            ..demo_startup()
        };
    }

    let mut workspaces = Vec::new();
    let mut bindings = Vec::new();
    let mut collection_paths = Vec::new();
    let mut local_paths = Vec::new();
    let mut failed_paths = Vec::new();

    for workspace_ref in &app_state.workspaces {
        let WorkspaceRef::Local { path } = workspace_ref else {
            continue;
        };

        match LocalStorageProvider::load_workspace(path) {
            Ok(loaded) => {
                startup_warnings.extend(loaded.warnings);
                local_paths.push(path.clone());
                workspaces.push(loaded.workspace);
                bindings.push(WorkspaceBinding::Local(path.clone()));
                collection_paths.push(loaded.collection_paths);
            }
            Err(error) => {
                let message = format!(
                    "failed to load workspace {}: {error}",
                    path.display()
                );
                eprintln!("{message}");
                startup_warnings.push(message);
                failed_paths.push(path.clone());
            }
        }
    }

    if !failed_paths.is_empty()
        && let Err(error) = remove_workspace_refs(app_paths, &failed_paths)
    {
        startup_warnings.push(format!("failed to clean app state: {error}"));
    }

    if workspaces.is_empty() {
        let demo = demo_startup();
        startup_warnings.extend(demo.startup_warnings);
        return StartupWorkspaces {
            startup_warnings,
            ..demo
        };
    }

    let active_workspace = app_state
        .active_workspace
        .min(workspaces.len().saturating_sub(1));
    let active_path = local_paths.get(active_workspace);
    let active_workspace = active_path
        .and_then(|path| {
            bindings.iter().position(|binding| {
                matches!(binding, WorkspaceBinding::Local(stored) if stored == path)
            })
        })
        .unwrap_or(0);

    StartupWorkspaces {
        workspaces,
        bindings,
        collection_paths,
        active_workspace,
        startup_warnings,
    }
}

fn demo_startup() -> StartupWorkspaces {
    let workspaces = crate::demo::demo_workspaces();
    let workspace_count = workspaces.len();

    StartupWorkspaces {
        collection_paths: default_workspace_collection_paths(&workspaces),
        bindings: vec![WorkspaceBinding::Ephemeral; workspace_count],
        active_workspace: 0,
        workspaces,
        startup_warnings: Vec::new(),
    }
}

pub fn first_open_request(
    workspaces: &[Workspace],
    active_workspace: usize,
) -> Option<(crate::domain::Request, TabSource)> {
    let workspace = workspaces.get(active_workspace)?;

    for (collection_index, collection) in workspace.collections.iter().enumerate() {
        if let Some(request) = collection.requests.first() {
            return Some((
                request.clone(),
                TabSource {
                    workspace: active_workspace,
                    collection: collection_index,
                    folder: None,
                    request: 0,
                },
            ));
        }

        for (folder_index, folder) in collection.folders.iter().enumerate() {
            if let Some(request) = folder.requests.first() {
                return Some((
                    request.clone(),
                    TabSource {
                        workspace: active_workspace,
                        collection: collection_index,
                        folder: Some(folder_index),
                        request: 0,
                    },
                ));
            }
        }
    }

    None
}
