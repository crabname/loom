use gpui::*;

use super::{LoomApp, Tab, TabSource};

impl LoomApp {
    pub(super) fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    pub(super) fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub(super) fn switch_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.tabs.len() || index == self.active_tab {
            return;
        }

        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.sync_active_tab_to_collection(cx);

        self.active_tab = index;
        self.reload_active_tab_inputs(window, cx);
        self.refresh_environment_select(window, cx);
        self.sync_collections_tree_selection(cx);
        cx.notify();
    }

    pub(super) fn ensure_open_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.tabs.is_empty() {
            return;
        }

        let first_request = self
            .active_collections()
            .iter()
            .enumerate()
            .find_map(|(collection_index, collection)| {
                collection
                    .first_request_location()
                    .map(|(folder, request_index)| (collection_index, folder, request_index))
            });

        if let Some((collection_index, folder, request_index)) = first_request {
            let request_data = self.active_collections()[collection_index]
                .request_ref(folder, request_index)
                .expect("request exists")
                .clone();
            let id = self.next_tab_id;
            self.next_tab_id += 1;
            self.tabs.push(Tab::from_request(
                id,
                &request_data,
                Some(TabSource {
                    workspace: self.active_workspace,
                    collection: collection_index,
                    folder,
                    request: request_index,
                }),
            ));
            self.active_tab = 0;
            self.reload_active_tab_inputs(window, cx);
            self.refresh_environment_select(window, cx);
            return;
        }

        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(Tab::empty(id, "Request 1"));
        self.active_tab = 0;
        self.reload_active_tab_inputs(window, cx);
        self.refresh_environment_select(window, cx);
    }

    pub(super) fn open_request_tab(
        &mut self,
        collection: usize,
        folder: Option<usize>,
        request: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.sync_active_tab_to_collection(cx);

        if let Some(index) = self.tabs.iter().position(|tab| {
            tab.source
                == Some(TabSource {
                    workspace: self.active_workspace,
                    collection,
                    folder,
                    request,
                })
        }) {
            self.active_tab = index;
            self.reload_active_tab_inputs(window, cx);
            self.refresh_environment_select(window, cx);
            self.sync_collections_tree_selection(cx);
            cx.notify();
            return;
        }

        let request_data = self.active_collections()[collection]
            .request_ref(folder, request)
            .expect("request exists")
            .clone();
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(Tab::from_request(
            id,
            &request_data,
            Some(TabSource {
                workspace: self.active_workspace,
                collection,
                folder,
                request,
            }),
        ));
        self.active_tab = self.tabs.len() - 1;
        self.reload_active_tab_inputs(window, cx);
        self.refresh_environment_select(window, cx);
        self.sync_collections_tree_selection(cx);
        cx.notify();
    }

    pub(super) fn add_empty_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.sync_active_tab_to_collection(cx);

        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let number = self.tabs.len() + 1;
        self.tabs.push(Tab::empty(id, format!("Request {number}")));
        self.active_tab = self.tabs.len() - 1;
        self.reload_active_tab_inputs(window, cx);
        self.refresh_environment_select(window, cx);
        self.sync_collections_tree_selection(cx);
        cx.notify();
    }

    pub(super) fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 {
            return;
        }

        self.flush_field_inputs(cx);
        self.sync_active_tab_to_collection(cx);
        self.tabs.remove(index);

        if self.active_tab >= index && self.active_tab > 0 {
            self.active_tab -= 1;
        }
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }

        self.reload_active_tab_inputs(window, cx);
        self.refresh_environment_select(window, cx);
        self.sync_collections_tree_selection(cx);
        cx.notify();
    }
}
