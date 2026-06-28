use gpui::*;
use gpui_component::select::SelectEvent;

use crate::domain::{Environment, EnvironmentRef, EnvironmentScope, Variable};

use super::ApiHelperApp;

impl ApiHelperApp {
    pub(super) fn wire_workspace_subscription(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._subscriptions.push(cx.subscribe_in(&self.workspace_select, window, {
            move |this, _, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let Some(index) = this
                    .workspaces
                    .iter()
                    .position(|workspace| workspace.name == *value)
                else {
                    return;
                };
                this.switch_workspace(index, window, cx);
            }
        }));
    }

    fn reset_workspace_ui(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.clear();
        self.active_tab = 0;
        self.active_environment = self.default_environment_ref();
        self.runtime_vars.clear();
        self.ensure_open_tab(window, cx);
        self.refresh_environment_select(window, cx);
    }

    pub(super) fn switch_workspace(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.workspaces.len() || index == self.active_workspace {
            return;
        }

        self.flush_workspace_edits(cx);
        self.autosave_workspace_at(cx, self.active_workspace);
        self.active_workspace = index;
        self.refresh_collections_tree(cx);
        self.reset_workspace_ui(window, cx);
        self.sync_collections_tree_selection(cx);
        self.persist_app_state();
        cx.notify();
    }

    pub(super) fn default_environment_ref(&self) -> Option<EnvironmentRef> {
        if self.workspaces[self.active_workspace]
            .environments
            .first()
            .is_some()
        {
            return Some(EnvironmentRef {
                scope: EnvironmentScope::Workspace,
                index: 0,
            });
        }

        for (collection_index, collection) in self.workspaces[self.active_workspace]
            .collections
            .iter()
            .enumerate()
        {
            if collection.environments.first().is_some() {
                return Some(EnvironmentRef {
                    scope: EnvironmentScope::Collection(collection_index),
                    index: 0,
                });
            }
        }

        None
    }

    fn environment_entries(&self) -> Vec<(SharedString, EnvironmentRef)> {
        let workspace = &self.workspaces[self.active_workspace];
        let mut entries = Vec::new();

        for (index, environment) in workspace.environments.iter().enumerate() {
            entries.push((
                environment.name.clone().into(),
                EnvironmentRef {
                    scope: EnvironmentScope::Workspace,
                    index,
                },
            ));
        }

        for (collection_index, collection) in workspace.collections.iter().enumerate() {
            for (index, environment) in collection.environments.iter().enumerate() {
                entries.push((
                    format!("{} / {}", collection.name, environment.name).into(),
                    EnvironmentRef {
                        scope: EnvironmentScope::Collection(collection_index),
                        index,
                    },
                ));
            }
        }

        entries
    }

    pub(super) fn refresh_environment_select(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entries = self.environment_entries();
        let labels: Vec<SharedString> = entries.iter().map(|(label, _)| label.clone()).collect();

        if self
            .active_environment
            .is_some_and(|environment_ref| {
                !entries
                    .iter()
                    .any(|(_, entry)| *entry == environment_ref)
            })
        {
            self.active_environment = entries.first().map(|(_, entry)| *entry);
        }

        let selected_label = self.active_environment.and_then(|environment_ref| {
            entries
                .iter()
                .find(|(_, entry)| *entry == environment_ref)
                .map(|(label, _)| label.clone())
        });

        self.environment_select.update(cx, |select, cx| {
            select.set_items(labels, window, cx);
            if let Some(label) = selected_label {
                select.set_selected_value(&label, window, cx);
            } else {
                select.set_selected_index(None, window, cx);
            }
        });
    }

    pub(super) fn wire_environment_subscription(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._subscriptions.push(cx.subscribe_in(&self.environment_select, window, {
            move |this, _, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let Some(environment_ref) = this
                    .environment_entries()
                    .into_iter()
                    .find(|(label, _)| label == value)
                    .map(|(_, entry)| entry)
                else {
                    return;
                };
                this.select_environment(environment_ref, window, cx);
            }
        }));
    }

    fn select_environment(
        &mut self,
        environment_ref: EnvironmentRef,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_environment == Some(environment_ref) {
            return;
        }

        self.active_environment = Some(environment_ref);
        cx.notify();
    }

    pub(super) fn apply_environments_manager(
        &mut self,
        workspace_environments: Vec<Environment>,
        collection_environments: Vec<Vec<Environment>>,
        workspace_variables: Vec<Variable>,
        collection_variables: Vec<Vec<Variable>>,
        folder_variables: Vec<Vec<Vec<Variable>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspace = &mut self.workspaces[self.active_workspace];
        workspace.environments = workspace_environments;
        workspace.variables = workspace_variables;

        for (collection, ((environments, variables), folders)) in workspace
            .collections
            .iter_mut()
            .zip(
                collection_environments
                    .into_iter()
                    .zip(collection_variables)
                    .zip(folder_variables),
            )
        {
            collection.environments = environments;
            collection.variables = variables;
            for (folder, variables) in collection.folders.iter_mut().zip(folders) {
                folder.variables = variables;
            }
        }

        self.reconcile_active_environment();
        self.refresh_environment_select(window, cx);
        self.autosave_active_workspace(cx);
        cx.notify();
    }

    fn reconcile_active_environment(&mut self) {
        let entries = self.environment_entries();
        if self
            .active_environment
            .is_some_and(|environment_ref| {
                entries
                    .iter()
                    .any(|(_, entry)| *entry == environment_ref)
            })
        {
            return;
        }

        self.active_environment = entries.first().map(|(_, entry)| *entry);
    }
}
