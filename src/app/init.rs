use gpui::*;
use gpui_component::{
    input::{InputEvent, InputState},
    select::{SelectEvent, SelectState},
    tree::TreeState,
    IndexPath,
};

use crate::domain::{
    demo_workspaces, BodyType, EnvironmentRef, EnvironmentScope, HttpMethod,
};
use std::collections::HashMap;

use super::ui::build_collection_tree_items;
use super::{ApiHelperApp, Tab, TabSource};

pub(crate) const METHOD_LABELS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];
pub(crate) const BODY_LABELS: [&str; 5] = ["none", "JSON", "XML", "form-urlencoded", "multipart"];

impl ApiHelperApp {
    pub fn open(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspaces = demo_workspaces();
        let request = workspaces[0].collections[0].requests[0].clone();
        let tab = Tab::from_request(
            0,
            &request,
            Some(TabSource {
                workspace: 0,
                collection: 0,
                folder: None,
                request: 0,
            }),
        );

        let workspace_labels: Vec<SharedString> = workspaces
            .iter()
            .map(|workspace| workspace.name.clone().into())
            .collect();
        let workspace_select = cx.new(|cx| {
            SelectState::new(
                workspace_labels,
                Some(IndexPath::default()),
                window,
                cx,
            )
        });

        let environment_select = cx.new(|cx| {
            SelectState::new(Vec::<SharedString>::new(), None, window, cx)
        });

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

        let pre_request_script_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .code_editor("javascript")
                .searchable(true)
                .placeholder("// Runs before the request is sent")
                .default_value(tab.pre_request_script.clone())
        });

        let post_response_script_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .code_editor("javascript")
                .searchable(true)
                .placeholder("// Runs after the response is received")
                .default_value(tab.post_response_script.clone())
        });

        let response_body_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .code_editor("text")
                .searchable(true)
                .context_menu(false)
                .default_value(String::new())
        });

        let collections_tree = cx.new(|cx| {
            TreeState::new(cx).items(build_collection_tree_items(&workspaces[0].collections))
        });

        let workspace_count = workspaces.len();

        let mut app = Self {
            workspaces,
            active_workspace: 0,
            workspace_sessions: vec![None; workspace_count],
            tabs: vec![tab],
            active_tab: 0,
            next_tab_id: 1,
            url_input,
            body_input,
            pre_request_script_input,
            post_response_script_input,
            response_body_input,
            method_select,
            body_type_select,
            workspace_select,
            environment_select,
            active_environment: Some(EnvironmentRef {
                scope: EnvironmentScope::Workspace,
                index: 0,
            }),
            runtime_vars: HashMap::new(),
            query_inputs: Vec::new(),
            header_inputs: Vec::new(),
            form_inputs: Vec::new(),
            multipart_inputs: Vec::new(),
            variable_inputs: Vec::new(),
            query_sync_guard: false,
            url_parse_debounce_seq: 0,
            query_param_subscriptions: Vec::new(),
            collections_tree,
            _subscriptions: Vec::new(),
        };

        app.wire_global_subscriptions(window, cx);
        app.wire_tree_subscription(cx);
        app.wire_workspace_subscription(window, cx);
        app.wire_environment_subscription(window, cx);
        app.refresh_environment_select(window, cx);
        app.sync_collections_tree_selection(cx);
        app.reload_field_inputs(window, cx);
        app.sync_url_from_params(window, cx);
        cx.notify();
        app
    }

    fn wire_global_subscriptions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._subscriptions.push(cx.subscribe_in(&self.url_input, window, {
            move |this, _, _: &InputEvent, window, cx| {
                if this.query_sync_guard {
                    return;
                }
                this.schedule_url_parse(window, cx);
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

        self._subscriptions.push(cx.subscribe_in(&self.pre_request_script_input, window, {
            move |this, _, _: &InputEvent, _, cx| {
                let script = this.pre_request_script_input.read(cx).value().to_string();
                if let Some(tab) = this.active_tab_mut() {
                    tab.pre_request_script = script;
                }
                this.sync_active_tab_to_collection(cx);
            }
        }));

        self._subscriptions.push(cx.subscribe_in(&self.post_response_script_input, window, {
            move |this, _, _: &InputEvent, _, cx| {
                let script = this.post_response_script_input.read(cx).value().to_string();
                if let Some(tab) = this.active_tab_mut() {
                    tab.post_response_script = script;
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

    pub(super) fn wire_query_param_subscriptions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.query_param_subscriptions.clear();
        for row in &self.query_inputs {
            for input in [&row.name, &row.value] {
                let input = input.clone();
                self.query_param_subscriptions.push(cx.subscribe_in(&input, window, {
                    move |this, _, _: &InputEvent, window, cx| {
                        this.on_query_param_changed(window, cx);
                    }
                }));
            }
        }
    }
}
