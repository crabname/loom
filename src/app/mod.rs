mod tab;
mod ui;

use gpui::*;
use gpui_component::{
    input::{InputEvent, InputState},
    select::{SelectEvent, SelectState},
    tree::{TreeEvent, TreeItem, TreeState},
    IndexPath, WindowExt,
};

use crate::domain::{
    demo_collections, format_body, BodyType, Collection, FormField, HttpMethod, KeyValueField,
    MultipartField, Request, ResponseBody, ResponseBodyView,
};
use crate::transport::{send_http_request, HttpResponse};

use tab::{Tab, TabSource};

use ui::{
    build_collection_tree_items, build_multipart_row_inputs, build_row_inputs, flush_multipart_rows,
    flush_rows, request_tree_id, FieldTable, MultipartRowInputs, RowInputs,
};

const METHOD_LABELS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];
const BODY_LABELS: [&str; 5] = ["none", "JSON", "XML", "form-urlencoded", "multipart"];

pub struct ApiHelperApp {
    collections: Vec<Collection>,
    tabs: Vec<Tab>,
    active_tab: usize,
    next_tab_id: usize,

    url_input: Entity<InputState>,
    body_input: Entity<InputState>,
    method_select: Entity<SelectState<Vec<&'static str>>>,
    body_type_select: Entity<SelectState<Vec<&'static str>>>,

    query_inputs: Vec<RowInputs>,
    header_inputs: Vec<RowInputs>,
    form_inputs: Vec<RowInputs>,
    multipart_inputs: Vec<MultipartRowInputs>,

    collections_tree: Entity<TreeState>,
    _subscriptions: Vec<Subscription>,
}

impl ApiHelperApp {
    pub fn open(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let collections = demo_collections();
        let request = collections[0].requests[0].clone();
        let tab = Tab::from_request(0, &request, Some(TabSource { collection: 0, request: 0 }));

        let method_select = cx.new(|cx| {
            SelectState::new(
                METHOD_LABELS.to_vec(),
                Some(IndexPath::default()),
                window,
                cx,
            )
        });

        let body_type_select = cx.new(|cx| {
            SelectState::new(
                BODY_LABELS.to_vec(),
                Some(IndexPath::default().row(1)),
                window,
                cx,
            )
        });

        let url_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("https://api.example.com/endpoint")
                .default_value(tab.url.clone())
        });

        let body_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .code_editor("json")
                .searchable(true)
                .default_value(tab.request_body.clone())
        });

        let collections_tree = cx.new(|cx| {
            TreeState::new(cx).items(build_collection_tree_items(&collections))
        });

        let mut app = Self {
            collections,
            tabs: vec![tab],
            active_tab: 0,
            next_tab_id: 1,
            url_input,
            body_input,
            method_select,
            body_type_select,
            query_inputs: Vec::new(),
            header_inputs: Vec::new(),
            form_inputs: Vec::new(),
            multipart_inputs: Vec::new(),
            collections_tree,
            _subscriptions: Vec::new(),
        };

        app.wire_global_subscriptions(window, cx);
        app.wire_tree_subscription(cx);
        app.sync_collections_tree_selection(cx);
        app.reload_field_inputs(window, cx);
        cx.notify();
        app
    }

    fn wire_global_subscriptions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._subscriptions.push(cx.subscribe_in(&self.url_input, window, {
            move |this, _, _: &InputEvent, _, cx| {
                let url = this.url_input.read(cx).value().to_string();
                if let Some(tab) = this.active_tab_mut() {
                    tab.url = url;
                }
                this.sync_active_tab_to_collection(cx);
            }
        }));

        self._subscriptions.push(cx.subscribe_in(&self.body_input, window, {
            move |this, _, _: &InputEvent, _, cx| {
                let body = this.body_input.read(cx).value().to_string();
                if let Some(tab) = this.active_tab_mut() {
                    tab.request_body = body;
                }
                this.sync_active_tab_to_collection(cx);
            }
        }));

        self._subscriptions.push(cx.subscribe_in(&self.method_select, window, {
            move |this, _, event: &SelectEvent<Vec<&'static str>>, _, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                if let Some(method) = HttpMethod::from_label(value) {
                    if let Some(tab) = this.active_tab_mut() {
                        tab.method = method;
                    }
                    this.sync_active_tab_to_collection(cx);
                    cx.notify();
                }
            }
        }));

        self._subscriptions.push(cx.subscribe_in(&self.body_type_select, window, {
            move |this, _, event: &SelectEvent<Vec<&'static str>>, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                if let Some(body_type) = BodyType::from_label(value) {
                    if let Some(tab) = this.active_tab_mut() {
                        tab.body_type = body_type;
                    }
                    this.sync_active_tab_to_collection(cx);
                    this.reload_body_input(window, cx);
                    cx.notify();
                }
            }
        }));
    }

    fn wire_tree_subscription(&mut self, cx: &mut Context<Self>) {
        self._subscriptions.push(cx.subscribe(&self.collections_tree, {
            |this, _, event: &TreeEvent, _| {
                let (TreeEvent::Expanded(id) | TreeEvent::Collapsed(id)) = event;
                if let Some(collection_index) = ui::parse_collection_tree_id(id)
                    && let Some(collection) = this.collections.get_mut(collection_index) {
                        collection.expanded = matches!(event, TreeEvent::Expanded(_));
                    }
            }
        }));
    }

    fn refresh_collections_tree(&mut self, cx: &mut Context<Self>) {
        let items = build_collection_tree_items(&self.collections);
        self.collections_tree.update(cx, |tree, cx| {
            tree.set_items(items, cx);
        });
    }

    fn sync_collections_tree_selection(&mut self, cx: &mut Context<Self>) {
        let selected_item = self.active_tab().and_then(|tab| {
            tab.source.map(|source| {
                TreeItem::new(
                    request_tree_id(source.collection, source.request),
                    "",
                )
            })
        });

        self.collections_tree.update(cx, |tree, cx| {
            match &selected_item {
                Some(item) => tree.set_selected_item(Some(item), cx),
                None => tree.set_selected_item(None, cx),
            }
        });
    }

    fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    fn sync_active_tab_to_collection(&mut self, cx: &mut Context<Self>) {
        let tab = match self.tabs.get(self.active_tab) {
            Some(tab) => tab.clone(),
            None => return,
        };
        let Some(source) = tab.source else {
            return;
        };

        if let Some(request) = self
            .collections
            .get_mut(source.collection)
            .and_then(|collection| collection.requests.get_mut(source.request))
        {
            request.url = tab.url;
            request.method = tab.method;
            request.query_params = tab.query_params;
            request.headers = tab.headers;
            request.body_type = tab.body_type;
            request.body = tab.request_body;
            request.form_fields = tab.form_fields;
            request.multipart_fields = tab.multipart_fields;
        }

        let _ = cx;
    }

    fn flush_field_inputs(&mut self, cx: &App) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };

        flush_rows(&mut tab.query_params, &self.query_inputs, cx);
        flush_rows(&mut tab.headers, &self.header_inputs, cx);
        flush_rows(&mut tab.form_fields, &self.form_inputs, cx);
        flush_multipart_rows(&mut tab.multipart_fields, &self.multipart_inputs, cx);
    }

    fn reload_field_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tabs.get(self.active_tab).cloned();
        let Some(tab) = tab else {
            return;
        };

        self.query_inputs = build_row_inputs(window, cx, &tab.query_params);
        self.header_inputs = build_row_inputs(window, cx, &tab.headers);
        self.form_inputs = build_row_inputs(window, cx, &tab.form_fields);
        self.multipart_inputs = build_multipart_row_inputs(window, cx, &tab.multipart_fields);
    }

    fn reload_active_tab_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tabs.get(self.active_tab).cloned();
        let Some(tab) = tab else {
            return;
        };

        self.url_input.update(cx, |input, cx| {
            input.set_value(tab.url.clone(), window, cx);
        });
        self.method_select.update(cx, |select, cx| {
            select.set_selected_value(&tab.method.as_str(), window, cx);
        });
        self.body_type_select.update(cx, |select, cx| {
            select.set_selected_value(&tab.body_type.label(), window, cx);
        });
        self.reload_body_input(window, cx);
        self.reload_field_inputs(window, cx);
    }

    fn reload_body_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tabs.get(self.active_tab).cloned();
        let Some(tab) = tab else {
            return;
        };

        self.body_input.update(cx, |input, cx| {
            input.set_value(tab.request_body.clone(), window, cx);
        });
    }

    fn format_request_body(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(body_type) = self.active_tab().map(|tab| tab.body_type) else {
            return;
        };

        let body = self.body_input.read(cx).value().to_string();
        match format_body(body_type, &body) {
            Ok(formatted) => {
                self.body_input.update(cx, |input, cx| {
                    input.set_value(formatted.clone(), window, cx);
                });
                if let Some(tab) = self.active_tab_mut() {
                    tab.request_body = formatted;
                }
                self.sync_active_tab_to_collection(cx);
                cx.notify();
            }
            Err(error) => {
                window.push_notification(gpui_component::notification::Notification::error(error), cx);
            }
        }
    }

    fn capture_editor_state(&mut self, cx: &App) {
        let url = self.url_input.read(cx).value().to_string();
        let body = self.body_input.read(cx).value().to_string();
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.url = url;
            tab.request_body = body;
        }
    }

    fn switch_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.tabs.len() || index == self.active_tab {
            return;
        }

        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.sync_active_tab_to_collection(cx);

        self.active_tab = index;
        self.reload_active_tab_inputs(window, cx);
        self.sync_collections_tree_selection(cx);
        cx.notify();
    }

    fn add_request_to_collection(
        &mut self,
        collection: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(collection_data) = self.collections.get_mut(collection) else {
            return;
        };

        collection_data.expanded = true;
        let number = collection_data.requests.len() + 1;
        let name = if number == 1 {
            "New Request".into()
        } else {
            format!("New Request {number}")
        };
        let request_index = collection_data.requests.len();
        collection_data
            .requests
            .push(Request::new(name));

        self.refresh_collections_tree(cx);
        self.open_request_tab(collection, request_index, window, cx);
    }

    fn delete_request_from_collection(
        &mut self,
        collection: usize,
        request: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.collections.get(collection).is_none_or(|c| request >= c.requests.len()) {
            return;
        }

        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.sync_active_tab_to_collection(cx);

        let active_tab_id = self.tabs.get(self.active_tab).map(|tab| tab.id);

        self.tabs.retain(|tab| {
            tab.source != Some(TabSource { collection, request })
        });

        for tab in &mut self.tabs {
            if let Some(source) = &mut tab.source {
                if source.collection == collection && source.request > request {
                    source.request -= 1;
                }
            }
        }

        self.collections[collection].requests.remove(request);
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
        cx.notify();
    }

    fn ensure_open_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.tabs.is_empty() {
            return;
        }

        for (collection_index, collection) in self.collections.iter().enumerate() {
            if let Some(request_data) = collection.requests.first() {
                let id = self.next_tab_id;
                self.next_tab_id += 1;
                self.tabs.push(Tab::from_request(
                    id,
                    request_data,
                    Some(TabSource {
                        collection: collection_index,
                        request: 0,
                    }),
                ));
                self.active_tab = 0;
                self.reload_active_tab_inputs(window, cx);
                return;
            }
        }

        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(Tab::empty(id, "Request 1"));
        self.active_tab = 0;
        self.reload_active_tab_inputs(window, cx);
    }

    fn open_request_tab(
        &mut self,
        collection: usize,
        request: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.sync_active_tab_to_collection(cx);

        if let Some(index) = self.tabs.iter().position(|tab| {
            tab.source == Some(TabSource { collection, request })
        }) {
            self.active_tab = index;
            self.reload_active_tab_inputs(window, cx);
            self.sync_collections_tree_selection(cx);
            cx.notify();
            return;
        }

        let request_data = self.collections[collection].requests[request].clone();
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(Tab::from_request(
            id,
            &request_data,
            Some(TabSource { collection, request }),
        ));
        self.active_tab = self.tabs.len() - 1;
        self.reload_active_tab_inputs(window, cx);
        self.sync_collections_tree_selection(cx);
        cx.notify();
    }

    fn add_empty_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.sync_active_tab_to_collection(cx);

        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let number = self.tabs.len() + 1;
        self.tabs.push(Tab::empty(id, format!("Request {number}")));
        self.active_tab = self.tabs.len() - 1;
        self.reload_active_tab_inputs(window, cx);
        self.sync_collections_tree_selection(cx);
        cx.notify();
    }

    fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
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
        self.sync_collections_tree_selection(cx);
        cx.notify();
    }

    fn send_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        if tab.loading {
            return;
        }

        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        if let Some(tab) = self.active_tab_mut() {
            tab.loading = true;
            tab.response_status = Some("Sending…".into());
            tab.response_body = ResponseBody::empty();
            tab.response_body_view = ResponseBodyView::Raw;
            tab.response_headers.clear();
        }
        self.sync_active_tab_to_collection(cx);
        cx.notify();

        let tab_id = self.tabs[self.active_tab].id;
        let url = self.tabs[self.active_tab].url.clone();
        let method = self.tabs[self.active_tab].method;
        let query_params = self.tabs[self.active_tab].query_params.clone();
        let headers = self.tabs[self.active_tab].headers.clone();
        let body_type = self.tabs[self.active_tab].body_type;
        let body = self.tabs[self.active_tab].request_body.clone();
        let form_fields = self.tabs[self.active_tab].form_fields.clone();
        let multipart_fields = self.tabs[self.active_tab].multipart_fields.clone();

        cx.spawn(async move |this, cx| {
            let result = tokio::runtime::Runtime::new()
                .expect("tokio runtime")
                .block_on(send_http_request(
                    url,
                    method,
                    query_params,
                    headers,
                    body_type,
                    body,
                    form_fields,
                    multipart_fields,
                ));

            this.update(cx, |app, cx| {
                app.finish_request(tab_id, result);
                cx.notify();
            })
            .ok();
        })
        .detach();

        let _ = window;
    }

    fn finish_request(&mut self, tab_id: usize, result: Result<HttpResponse, String>) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };

        tab.loading = false;
        match result {
            Ok(response) => {
                tab.response_status = Some(format!(
                    "{} {} · {} ms",
                    response.status, response.status_text, response.elapsed_ms
                ));
                tab.response_body = response.body;
                tab.response_body_view = ResponseBodyView::Raw;
                tab.response_headers = response.headers;
            }
            Err(error) => {
                tab.response_status = Some(format!("Error · {error}"));
                tab.response_body = ResponseBody::Text(error);
                tab.response_body_view = ResponseBodyView::Raw;
                tab.response_headers.clear();
            }
        }
    }

    fn toggle_field(
        &mut self,
        table: FieldTable,
        index: usize,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        match table {
            FieldTable::QueryParams => {
                if let Some(field) = tab.query_params.get_mut(index) {
                    field.enabled = enabled;
                }
            }
            FieldTable::RequestHeaders => {
                if let Some(field) = tab.headers.get_mut(index) {
                    field.enabled = enabled;
                }
            }
            FieldTable::FormFields => {
                if let Some(field) = tab.form_fields.get_mut(index) {
                    field.enabled = enabled;
                }
            }
            FieldTable::MultipartFields => {
                if let Some(field) = tab.multipart_fields.get_mut(index) {
                    field.enabled = enabled;
                }
            }
        }

        self.sync_active_tab_to_collection(cx);
        cx.notify();
    }

    fn remove_field(
        &mut self,
        table: FieldTable,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.flush_field_inputs(cx);

        if let Some(tab) = self.active_tab_mut() {
            match table {
                FieldTable::QueryParams => {
                    if tab.query_params.len() > 1 {
                        tab.query_params.remove(index);
                    } else {
                        tab.query_params[0] = KeyValueField::empty();
                    }
                }
                FieldTable::RequestHeaders => {
                    if tab.headers.len() > 1 {
                        tab.headers.remove(index);
                    } else {
                        tab.headers[0] = KeyValueField::empty();
                    }
                }
                FieldTable::FormFields => {
                    if tab.form_fields.len() > 1 {
                        tab.form_fields.remove(index);
                    } else {
                        tab.form_fields[0] = FormField::empty();
                    }
                }
                FieldTable::MultipartFields => {
                    if tab.multipart_fields.len() > 1 {
                        tab.multipart_fields.remove(index);
                    } else {
                        tab.multipart_fields[0] = MultipartField::empty();
                    }
                }
            }
        }

        self.reload_field_inputs(window, cx);
        cx.notify();
    }

    fn add_field(&mut self, table: FieldTable, window: &mut Window, cx: &mut Context<Self>) {
        self.flush_field_inputs(cx);

        if let Some(tab) = self.active_tab_mut() {
            match table {
                FieldTable::QueryParams => tab.query_params.push(KeyValueField::empty()),
                FieldTable::RequestHeaders => tab.headers.push(KeyValueField::empty()),
                FieldTable::FormFields => tab.form_fields.push(FormField::empty()),
                FieldTable::MultipartFields => tab.multipart_fields.push(MultipartField::empty()),
            }
        }

        self.reload_field_inputs(window, cx);
        cx.notify();
    }

    fn pick_multipart_file(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let path = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select a file".into()),
        });

        cx.spawn_in(window, async move |this, window| {
            let path = match path.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let Some(path) = path else {
                return;
            };

            window
                .update(|window, cx| {
                    this.update(cx, |app, cx| {
                        let path_str = path.to_string_lossy().to_string();
                        if let Some(row) = app.multipart_inputs.get(index) {
                            row.value.update(cx, |input, cx| {
                                input.set_value(path_str.clone(), window, cx);
                            });
                        }
                        if let Some(field) = app
                            .active_tab_mut()
                            .and_then(|tab| tab.multipart_fields.get_mut(index))
                        {
                            field.value = path_str;
                        }
                        app.sync_active_tab_to_collection(cx);
                        cx.notify();
                    })
                    .ok();
                })
                .ok();
        })
        .detach();
    }
}
