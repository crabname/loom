use std::path::{Path, PathBuf};

use gpui::*;
use gpui_component::{notification::Notification, WindowExt as _};

use crate::storage::LocalStorageProvider;

use super::{ApiHelperApp, OpenWorkspace, WorkspaceBinding};

impl ApiHelperApp {
    pub(super) fn on_open_workspace(
        &mut self,
        _: &OpenWorkspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.prompt_and_open_workspace(window, cx);
    }

    fn prompt_and_open_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Select a workspace folder".into()),
        });

        let view = cx.entity();
        cx.spawn_in(window, async move |_, window| {
            let Some(path) = paths
                .await
                .ok()
                .and_then(|result| result.ok())
                .and_then(|paths| paths)
                .and_then(|mut paths| paths.pop())
            else {
                return;
            };

            let result = LocalStorageProvider::load_workspace(&path);
            window
                .update(|window, cx| {
                    view.update(cx, |app, cx| {
                        app.finish_open_workspace(path, result, window, cx);
                    })
                })
                .ok();
        })
        .detach();
    }

    fn finish_open_workspace(
        &mut self,
        path: PathBuf,
        result: Result<crate::storage::LoadedWorkspace, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(loaded) => {
                self.integrate_loaded_workspace(path, loaded, window, cx);
            }
            Err(error) => {
                window.push_notification(
                    Notification::error("Failed to open workspace").message(error),
                    cx,
                );
            }
        }
    }

    fn integrate_loaded_workspace(
        &mut self,
        path: PathBuf,
        loaded: crate::storage::LoadedWorkspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.workspaces.is_empty() {
            self.flush_workspace_edits(cx);
            self.autosave_workspace_at(cx, self.active_workspace);
        }

        self.drop_ephemeral_workspaces(None);

        if let Some(index) = self.workspace_index_for_path(&path) {
            self.workspaces[index] = loaded.workspace;
            self.workspace_collection_paths[index] = loaded.collection_paths;
            self.switch_workspace(index, window, cx);
            self.persist_app_state();
            return;
        }

        self.workspaces.push(loaded.workspace);
        self.workspace_bindings.push(WorkspaceBinding::Local(path));
        self.workspace_collection_paths.push(loaded.collection_paths);

        let index = self.workspaces.len() - 1;
        self.refresh_workspace_select(window, cx);
        self.switch_workspace(index, window, cx);
        self.persist_app_state();
    }

    fn workspace_index_for_path(&self, path: &Path) -> Option<usize> {
        self.workspace_bindings.iter().position(|binding| {
            binding
                .local_path()
                .is_some_and(|stored_path| stored_path == path)
        })
    }

    pub(super) fn refresh_workspace_select(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let labels: Vec<SharedString> = self
            .workspaces
            .iter()
            .map(|workspace| workspace.name.clone().into())
            .collect();
        let selected = labels
            .get(self.active_workspace)
            .cloned()
            .or_else(|| labels.first().cloned());

        self.workspace_select.update(cx, |select, cx| {
            select.set_items(labels, window, cx);
            if let Some(label) = selected {
                select.set_selected_value(&label, window, cx);
            }
        });
    }
}
