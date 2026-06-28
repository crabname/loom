use std::path::PathBuf;

use crate::storage::{AppState, WorkspaceRef};

use super::{LoomApp, WorkspaceBinding};

impl LoomApp {
    pub(super) fn persist_app_state(&self) {
        let local_refs = self.persistable_local_refs();
        if local_refs.is_empty() {
            return;
        }

        let cloud_refs = AppState::load(&self.app_paths)
            .cloud_workspaces()
            .cloned()
            .collect::<Vec<_>>();

        let mut workspaces = local_refs;
        workspaces.extend(cloud_refs);

        let state = AppState {
            version: 1,
            active_workspace: self.persistable_active_index(&workspaces),
            workspaces,
        };

        if let Err(error) = state.save(&self.app_paths) {
            eprintln!("failed to save app state: {error}");
        }
    }

    pub(super) fn drop_ephemeral_workspaces(&mut self, keep: Option<usize>) {
        if !self
            .workspace_bindings
            .iter()
            .any(|binding| matches!(binding, WorkspaceBinding::Ephemeral))
        {
            return;
        }

        let active_binding = self.workspace_bindings[self.active_workspace].clone();
        let mut next_workspaces = Vec::new();
        let mut next_bindings = Vec::new();
        let mut next_collection_paths = Vec::new();

        for index in 0..self.workspaces.len() {
            let is_ephemeral = matches!(self.workspace_bindings[index], WorkspaceBinding::Ephemeral);
            if is_ephemeral && keep != Some(index) {
                continue;
            }

            next_workspaces.push(self.workspaces[index].clone());
            next_bindings.push(self.workspace_bindings[index].clone());
            next_collection_paths.push(self.workspace_collection_paths[index].clone());
        }

        self.workspaces = next_workspaces;
        self.workspace_collection_paths = next_collection_paths;

        self.active_workspace = next_bindings
            .iter()
            .position(|binding| binding == &active_binding)
            .unwrap_or(0);
        self.workspace_bindings = next_bindings;
    }

    pub(super) fn managed_workspace_path_for(&self, index: usize) -> PathBuf {
        let name = self.workspaces[index].name.clone();
        let exclude = self.workspace_bindings[index].local_path();
        self.app_paths
            .unique_managed_workspace_path(&name, exclude)
    }

    fn persistable_local_refs(&self) -> Vec<WorkspaceRef> {
        self.workspace_bindings
            .iter()
            .filter_map(|binding| match binding {
                WorkspaceBinding::Local(path) => Some(WorkspaceRef::Local {
                    path: path.clone(),
                }),
                _ => None,
            })
            .collect()
    }

    fn persistable_active_index(&self, workspaces: &[WorkspaceRef]) -> usize {
        match &self.workspace_bindings[self.active_workspace] {
            WorkspaceBinding::Local(path) => workspaces
                .iter()
                .position(|workspace_ref| {
                    matches!(workspace_ref, WorkspaceRef::Local { path: stored } if stored == path)
                })
                .unwrap_or(0),
            WorkspaceBinding::Cloud(binding) => workspaces
                .iter()
                .position(|workspace_ref| {
                    matches!(
                        workspace_ref,
                        WorkspaceRef::Cloud { id, .. } if id == &binding.id
                    )
                })
                .unwrap_or(0),
            WorkspaceBinding::Ephemeral => 0,
        }
    }
}
