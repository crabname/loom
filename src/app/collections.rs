use gpui::*;
use gpui_component::tree::{TreeEvent, TreeItem};

use crate::domain::{Collection, CollectionFolder, EnvironmentRef, EnvironmentScope, Request};
use crate::storage::{remove_collection_dir, remove_folder_dir};

use super::ui::{
    build_collection_tree_items, parse_collection_tree_id, parse_folder_tree_id, request_tree_id,
};
use super::{ApiHelperApp, TabSource};

impl ApiHelperApp {
    pub(super) fn active_collections(&self) -> &[Collection] {
        &self.workspaces[self.active_workspace].collections
    }

    pub(super) fn active_collections_mut(&mut self) -> &mut Vec<Collection> {
        &mut self.workspaces[self.active_workspace].collections
    }

    pub(super) fn wire_tree_subscription(&mut self, cx: &mut Context<Self>) {
        self._subscriptions.push(cx.subscribe(&self.collections_tree, {
            |this, _, event: &TreeEvent, _| {
                let (TreeEvent::Expanded(id) | TreeEvent::Collapsed(id)) = event;
                let expanded = matches!(event, TreeEvent::Expanded(_));

                if let Some(collection_index) = parse_collection_tree_id(id)
                    && let Some(collection) =
                        this.active_collections_mut().get_mut(collection_index)
                {
                    collection.expanded = expanded;
                    return;
                }

                if let Some((collection_index, folder_index)) = parse_folder_tree_id(id)
                    && let Some(folder) = this
                        .active_collections_mut()
                        .get_mut(collection_index)
                        .and_then(|collection| collection.folders.get_mut(folder_index))
                {
                    folder.expanded = expanded;
                }
            }
        }));
    }

    pub(super) fn refresh_collections_tree(&mut self, cx: &mut Context<Self>) {
        let items = build_collection_tree_items(self.active_collections());
        self.collections_tree.update(cx, |tree, cx| {
            tree.set_items(items, cx);
        });
    }

    pub(super) fn sync_collections_tree_selection(&mut self, cx: &mut Context<Self>) {
        let selected_item = self.active_tab().and_then(|tab| {
            tab.source.and_then(|source| {
                if source.workspace != self.active_workspace {
                    return None;
                }
                Some(TreeItem::new(
                    request_tree_id(source.collection, source.folder, source.request),
                    "",
                ))
            })
        });

        self.collections_tree.update(cx, |tree, cx| {
            match &selected_item {
                Some(item) => tree.set_selected_item(Some(item), cx),
                None => tree.set_selected_item(None, cx),
            }
        });
    }

    pub(super) fn sync_active_tab_to_collection_quiet(&mut self, cx: &mut Context<Self>) {
        let tab = match self.tabs.get(self.active_tab) {
            Some(tab) => tab.clone(),
            None => return,
        };
        let Some(source) = tab.source else {
            return;
        };

        if let Some(request) = self
            .workspaces
            .get_mut(source.workspace)
            .and_then(|workspace| workspace.collections.get_mut(source.collection))
            .and_then(|collection| collection.request_mut(source.folder, source.request))
        {
            request.url = tab.url;
            request.method = tab.method;
            request.query_params = tab.query_params;
            request.headers = tab.headers;
            request.body_type = tab.body_type;
            request.body = tab.request_body;
            request.form_fields = tab.form_fields;
            request.multipart_fields = tab.multipart_fields;
            request.variables = tab.variables;
            request.pre_request_script = tab.pre_request_script.clone();
            request.post_response_script = tab.post_response_script.clone();
        }

        let _ = cx;
    }

    pub(super) fn sync_active_tab_to_collection(&mut self, cx: &mut Context<Self>) {
        self.sync_active_tab_to_collection_quiet(cx);
        self.schedule_autosave(cx);
    }

    pub(super) fn add_collection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let number = self.active_collections().len() + 1;
        let name = if number == 1 {
            "New Collection".into()
        } else {
            format!("New Collection {number}")
        };

        self.active_collections_mut().push(Collection::new(name));
        self.workspace_collection_paths[self.active_workspace].push(String::new());

        self.refresh_collections_tree(cx);
        self.refresh_environment_select(window, cx);
        self.autosave_active_workspace(cx);
        cx.notify();
    }

    pub(super) fn delete_collection(
        &mut self,
        collection: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_collections().get(collection).is_none() {
            return;
        }

        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.sync_active_tab_to_collection(cx);

        let collection_path = self
            .workspace_collection_paths
            .get(self.active_workspace)
            .and_then(|paths| paths.get(collection))
            .cloned();
        let active_tab_id = self.tabs.get(self.active_tab).map(|tab| tab.id);

        if let Some(EnvironmentRef { scope, index }) = self.active_environment {
            if let EnvironmentScope::Collection(collection_index) = scope {
                if collection_index == collection {
                    self.active_environment = None;
                } else if collection_index > collection {
                    self.active_environment = Some(EnvironmentRef {
                        scope: EnvironmentScope::Collection(collection_index - 1),
                        index,
                    });
                }
            }
        }

        self.tabs.retain(|tab| {
            tab.source.is_none_or(|source| {
                source.workspace != self.active_workspace || source.collection != collection
            })
        });

        for tab in &mut self.tabs {
            if let Some(source) = &mut tab.source
                && source.workspace == self.active_workspace
                && source.collection > collection
            {
                source.collection -= 1;
            }
        }

        self.active_collections_mut().remove(collection);
        self.workspace_collection_paths[self.active_workspace].remove(collection);

        if let (Some(workspace_path), Some(collection_path)) = (
            self.workspace_bindings[self.active_workspace].local_path(),
            collection_path.as_deref(),
        ) {
            let _ = remove_collection_dir(workspace_path, collection_path);
        }

        self.ensure_open_tab(window, cx);

        if let Some(active_tab_id) = active_tab_id {
            self.active_tab = self
                .tabs
                .iter()
                .position(|tab| tab.id == active_tab_id)
                .unwrap_or_else(|| self.tabs.len().saturating_sub(1));
        }

        self.refresh_collections_tree(cx);
        self.reload_active_tab_inputs(window, cx);
        self.sync_collections_tree_selection(cx);
        self.refresh_environment_select(window, cx);
        self.autosave_active_workspace(cx);
        cx.notify();
    }

    pub(super) fn add_folder_to_collection(
        &mut self,
        collection: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(collection_data) = self.active_collections_mut().get_mut(collection) else {
            return;
        };

        collection_data.expanded = true;
        let number = collection_data.folders.len() + 1;
        let name = if number == 1 {
            "New Folder".into()
        } else {
            format!("New Folder {number}")
        };
        collection_data.folders.push(CollectionFolder::new(name));

        self.refresh_collections_tree(cx);
        self.autosave_active_workspace(cx);
        cx.notify();
        let _ = window;
    }

    pub(super) fn add_request_to_collection(
        &mut self,
        collection: usize,
        folder: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(collection_data) = self.active_collections_mut().get_mut(collection) else {
            return;
        };

        collection_data.expanded = true;
        let request_count = match folder {
            None => collection_data.requests.len(),
            Some(folder_index) => collection_data
                .folders
                .get(folder_index)
                .map(|folder| folder.requests.len())
                .unwrap_or(0),
        };
        let number = request_count + 1;
        let name = if number == 1 {
            "New Request".into()
        } else {
            format!("New Request {number}")
        };
        let request_index = collection_data.push_request(folder, Request::new(name));

        self.refresh_collections_tree(cx);
        self.open_request_tab(collection, folder, request_index, window, cx);
        self.autosave_active_workspace(cx);
    }

    pub(super) fn delete_request_from_collection(
        &mut self,
        collection: usize,
        folder: Option<usize>,
        request: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .active_collections()
            .get(collection)
            .is_none_or(|collection_data| collection_data.request_ref(folder, request).is_none())
        {
            return;
        }

        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.sync_active_tab_to_collection(cx);

        let active_tab_id = self.tabs.get(self.active_tab).map(|tab| tab.id);

        self.tabs.retain(|tab| {
            tab.source
                != Some(TabSource {
                    workspace: self.active_workspace,
                    collection,
                    folder,
                    request,
                })
        });

        for tab in &mut self.tabs {
            if let Some(source) = &mut tab.source
                && source.workspace == self.active_workspace
                && source.collection == collection
                && source.folder == folder
                && source.request > request
            {
                source.request -= 1;
            }
        }

        self.active_collections_mut()[collection].remove_request(folder, request);
        self.ensure_open_tab(window, cx);

        if let Some(active_tab_id) = active_tab_id {
            self.active_tab = self
                .tabs
                .iter()
                .position(|tab| tab.id == active_tab_id)
                .unwrap_or_else(|| self.tabs.len().saturating_sub(1));
        }

        self.refresh_collections_tree(cx);
        self.reload_active_tab_inputs(window, cx);
        self.sync_collections_tree_selection(cx);
        self.autosave_active_workspace(cx);
        cx.notify();
    }

    pub(super) fn delete_folder_from_collection(
        &mut self,
        collection: usize,
        folder: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .active_collections()
            .get(collection)
            .is_none_or(|collection_data| collection_data.folders.get(folder).is_none())
        {
            return;
        }

        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.sync_active_tab_to_collection(cx);

        let collection_path = self
            .workspace_collection_paths
            .get(self.active_workspace)
            .and_then(|paths| paths.get(collection))
            .cloned();
        let folder_name = self.active_collections()[collection].folders[folder].name.clone();
        let active_tab_id = self.tabs.get(self.active_tab).map(|tab| tab.id);

        self.tabs.retain(|tab| {
            tab.source.is_none_or(|source| {
                source.workspace != self.active_workspace
                    || source.collection != collection
                    || source.folder != Some(folder)
            })
        });

        for tab in &mut self.tabs {
            if let Some(source) = &mut tab.source
                && source.workspace == self.active_workspace
                && source.collection == collection
            {
                if let Some(folder_index) = source.folder
                    && folder_index > folder
                {
                    source.folder = Some(folder_index - 1);
                }
            }
        }

        self.active_collections_mut()[collection].remove_folder(folder);

        if let (Some(workspace_path), Some(collection_path)) = (
            self.workspace_bindings[self.active_workspace].local_path(),
            collection_path.as_deref(),
        ) {
            let _ = remove_folder_dir(workspace_path, collection_path, &folder_name);
        }

        self.ensure_open_tab(window, cx);

        if let Some(active_tab_id) = active_tab_id {
            self.active_tab = self
                .tabs
                .iter()
                .position(|tab| tab.id == active_tab_id)
                .unwrap_or_else(|| self.tabs.len().saturating_sub(1));
        }

        self.refresh_collections_tree(cx);
        self.reload_active_tab_inputs(window, cx);
        self.sync_collections_tree_selection(cx);
        self.autosave_active_workspace(cx);
        cx.notify();
    }

    pub(super) fn rename_collection(
        &mut self,
        collection: usize,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }

        let Some(collection_data) = self.active_collections_mut().get_mut(collection) else {
            return;
        };

        collection_data.name = name.to_string();
        self.refresh_collections_tree(cx);
        self.refresh_environment_select(window, cx);
        self.autosave_active_workspace(cx);
        cx.notify();
    }

    pub(super) fn rename_folder(
        &mut self,
        collection: usize,
        folder: usize,
        name: String,
        cx: &mut Context<Self>,
    ) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }

        let Some(folder_data) = self
            .active_collections_mut()
            .get_mut(collection)
            .and_then(|collection_data| collection_data.folders.get_mut(folder))
        else {
            return;
        };

        folder_data.name = name.to_string();
        self.refresh_collections_tree(cx);
        self.autosave_active_workspace(cx);
        cx.notify();
    }

    pub(super) fn rename_request(
        &mut self,
        collection: usize,
        folder: Option<usize>,
        request: usize,
        name: String,
        cx: &mut Context<Self>,
    ) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }

        let Some(request_data) = self
            .active_collections_mut()
            .get_mut(collection)
            .and_then(|collection_data| collection_data.request_mut(folder, request))
        else {
            return;
        };

        request_data.name = name.to_string();

        let source = TabSource {
            workspace: self.active_workspace,
            collection,
            folder,
            request,
        };
        for tab in &mut self.tabs {
            if tab.source == Some(source) {
                tab.title = name.to_string();
            }
        }

        self.refresh_collections_tree(cx);
        self.autosave_active_workspace(cx);
        cx.notify();
    }
}
