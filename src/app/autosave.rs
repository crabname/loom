use std::time::Duration;

use gpui::*;

use crate::storage::LocalStorageProvider;

use super::{ApiHelperApp, WorkspaceBinding};

const AUTOSAVE_DEBOUNCE_MS: u64 = 800;

impl ApiHelperApp {
    pub(super) fn flush_workspace_edits(&mut self, cx: &mut Context<Self>) {
        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.sync_active_tab_to_collection_quiet(cx);
    }

    pub(super) fn schedule_autosave(&mut self, cx: &mut Context<Self>) {
        self.autosave_debounce_seq += 1;
        let seq = self.autosave_debounce_seq;

        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(AUTOSAVE_DEBOUNCE_MS))
                .await;

            this.update(cx, |app, cx| {
                if app.autosave_debounce_seq != seq {
                    return;
                }
                app.autosave_workspace_at(cx, app.active_workspace);
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn autosave_active_workspace(&mut self, cx: &mut Context<Self>) {
        self.autosave_debounce_seq += 1;
        self.autosave_workspace_at(cx, self.active_workspace);
    }

    pub(super) fn autosave_workspace_at(&mut self, cx: &mut Context<Self>, index: usize) {
        if index >= self.workspaces.len() {
            return;
        }

        if !self.ensure_workspace_persisted(index) {
            return;
        }

        let Some(path) = self.workspace_bindings[index].local_path().map(|path| path.to_path_buf())
        else {
            return;
        };

        let workspace = self.workspaces[index].clone();
        let mut collection_paths = self.workspace_collection_paths[index].clone();

        match LocalStorageProvider::save_workspace(&path, &workspace, &mut collection_paths) {
            Ok(()) => {
                self.workspace_collection_paths[index] = collection_paths;
            }
            Err(error) => {
                eprintln!(
                    "autosave failed for {}: {error}",
                    path.display()
                );
            }
        }

        let _ = cx;
    }

    fn ensure_workspace_persisted(&mut self, index: usize) -> bool {
        if index >= self.workspaces.len() {
            return false;
        }

        if matches!(self.workspace_bindings[index], WorkspaceBinding::Ephemeral) {
            self.drop_ephemeral_workspaces(Some(index));
            if index >= self.workspaces.len() {
                return false;
            }
            let path = self.managed_workspace_path_for(index);
            self.workspace_bindings[index] = WorkspaceBinding::Local(path);
            self.persist_app_state();
        }

        self.workspace_bindings[index].local_path().is_some()
    }
}
