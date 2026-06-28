use gpui::*;
use gpui_component::{
    input::{InputEvent, InputState},
    select::{SelectEvent, SelectState},
    tree::TreeState,
    IndexPath,
};

use crate::domain::{BodyType, HttpMethod};
use crate::storage::AppPaths;
use std::collections::HashMap;

use super::startup::{first_open_request, load_startup_workspaces};
use super::ui::build_collection_tree_items;
use super::variable_hover::{configure_variable_code_editor, configure_variable_input, VariableHoverProvider};
use super::{menus, LoomApp, Tab};

pub(crate) const METHOD_LABELS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];
pub(crate) const BODY_LABELS: [&str; 5] = ["none", "JSON", "XML", "form-urlencoded", "multipart"];

impl LoomApp {
    pub fn open(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let app_paths = AppPaths::ensure().unwrap_or_else(|error| {
            eprintln!("failed to initialize app data directory: {error}");
            AppPaths::fallback()
        });
        let startup = load_startup_workspaces(&app_paths);
        let workspaces = startup.workspaces;
        let active_workspace = startup.active_workspace;
        let (request, tab_source) = first_open_request(&workspaces, active_workspace)
            .map(|(request, source)| (request, Some(source)))
            .unwrap_or_else(|| {
                (
                    workspaces[active_workspace]
                        .collections
                        .first()
                        .and_then(|collection| collection.requests.first())
                        .cloned()
                        .unwrap_or_else(|| {
                            crate::domain::Request::new("Untitled")
                        }),
                    None,
                )
            });
        let tab = Tab::from_request(0, &request, tab_source);

        let workspace_labels: Vec<SharedString> = workspaces
            .iter()
            .map(|workspace| workspace.name.clone().into())
            .collect();
        let workspace_select = cx.new(|cx| {
            SelectState::new(
                workspace_labels,
                Some(IndexPath::new(active_workspace)),
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

        let variable_hover = VariableHoverProvider::new();

        let url_input = cx.new(|cx| {
            configure_variable_input(
                InputState::new(window, cx)
                    .placeholder("https://api.example.com/endpoint")
                    .default_value(tab.url.clone()),
                variable_hover.clone(),
            )
        });

        let body_input = cx.new(|cx| {
            configure_variable_code_editor(
                InputState::new(window, cx).default_value(tab.request_body.clone()),
                variable_hover.clone(),
                "json",
            )
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
            TreeState::new(cx).items(build_collection_tree_items(
                &workspaces[active_workspace].collections,
            ))
        });

        let app_menu_bar = menus::new_app_menu_bar(cx);

        let mut app = Self {
            app_paths,
            workspaces,
            workspace_bindings: startup.bindings,
            workspace_collection_paths: startup.collection_paths,
            active_workspace,
            app_menu_bar,
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
            active_environment: None,
            runtime_vars: HashMap::new(),
            query_inputs: Vec::new(),
            header_inputs: Vec::new(),
            form_inputs: Vec::new(),
            multipart_inputs: Vec::new(),
            variable_inputs: Vec::new(),
            query_sync_guard: false,
            url_parse_debounce_seq: 0,
            autosave_debounce_seq: 0,
            query_param_subscriptions: Vec::new(),
            collections_tree,
            variable_hover,
            _subscriptions: Vec::new(),
        };

        app.variable_hover.attach(&cx.entity());
        app.wire_global_subscriptions(window, cx);
        app.wire_tree_subscription(cx);
        app.wire_workspace_subscription(window, cx);
        app.wire_environment_subscription(window, cx);
        app.active_environment = app.default_environment_ref();
        app.refresh_environment_select(window, cx);
        app.sync_collections_tree_selection(cx);
        app.reload_field_inputs(window, cx);
        app.sync_url_from_params(window, cx);

        app._subscriptions.push(cx.on_app_quit(|app, cx| {
            app.flush_workspace_edits(cx);
            app.autosave_active_workspace(cx);
            app.persist_app_state();
            async {}
        }));

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
