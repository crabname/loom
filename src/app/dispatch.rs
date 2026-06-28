use std::collections::HashMap;

use gpui::*;

use crate::domain::{
    build_variable_pool, substitute_request, EnvironmentScope, Request, RequestTimingBreakdown,
    ResponseBody, ResponseBodyView, Variable, VariableLayers,
};
use crate::scripting::{
    map_to_variables, merge_runtime_vars, run_post_response_script, run_pre_request_script,
    variables_to_map, ScriptHostState, ScriptResult,
};
use crate::transport::{block_on, send_http_request, HttpRequestBody, HttpRequestResult};

use super::ApiHelperApp;

impl ApiHelperApp {
    fn active_environment_variables(&self) -> Option<&[Variable]> {
        let environment_ref = self.active_environment?;
        let workspace = self.workspaces.get(self.active_workspace)?;

        match environment_ref.scope {
            EnvironmentScope::Workspace => workspace
                .environments
                .get(environment_ref.index)
                .map(|environment| environment.variables.as_slice()),
            EnvironmentScope::Collection(collection_index) => workspace
                .collections
                .get(collection_index)
                .and_then(|collection| collection.environments.get(environment_ref.index))
                .map(|environment| environment.variables.as_slice()),
        }
    }

    fn active_environment_variables_mut(&mut self) -> Option<&mut Vec<Variable>> {
        let environment_ref = self.active_environment?;
        let workspace = self.workspaces.get_mut(self.active_workspace)?;

        match environment_ref.scope {
            EnvironmentScope::Workspace => workspace
                .environments
                .get_mut(environment_ref.index)
                .map(|environment| &mut environment.variables),
            EnvironmentScope::Collection(collection_index) => workspace
                .collections
                .get_mut(collection_index)
                .and_then(|collection| collection.environments.get_mut(environment_ref.index))
                .map(|environment| &mut environment.variables),
        }
    }

    fn workspace_environment_fallback_map(&self) -> HashMap<String, serde_json::Value> {
        let Some(workspace) = self.workspaces.get(self.active_workspace) else {
            return HashMap::new();
        };

        let Some(environment_ref) = self.active_environment else {
            return HashMap::new();
        };

        if matches!(environment_ref.scope, EnvironmentScope::Workspace) {
            return HashMap::new();
        }

        workspace
            .environments
            .first()
            .map(|environment| variables_to_map(&environment.variables))
            .unwrap_or_default()
    }

    pub(super) fn build_script_host_state(&self) -> ScriptHostState {
        ScriptHostState::from_parts(
            self.runtime_vars.clone(),
            variables_to_map(self.active_environment_variables().unwrap_or(&[])),
            self.workspace_environment_fallback_map(),
        )
    }

    pub(super) fn apply_script_result(&mut self, result: &ScriptResult) {
        if result.runtime_dirty {
            self.runtime_vars = result.runtime_vars.clone();
        }

        if result.env_dirty {
            if let Some(variables) = self.active_environment_variables_mut() {
                *variables = map_to_variables(&result.env_vars);
            }
        }
    }

    fn variable_pool_for_source(
        &self,
        collection_index: Option<usize>,
        folder_index: Option<usize>,
        request_variables: &[Variable],
        runtime_vars: &HashMap<String, serde_json::Value>,
    ) -> std::collections::HashMap<String, String> {
        let workspace = &self.workspaces[self.active_workspace];
        let collection = collection_index.and_then(|index| workspace.collections.get(index));
        let collection_variables = collection
            .map(|collection| collection.variables.as_slice())
            .unwrap_or(&[]);
        let folder_variables = collection
            .and_then(|collection| {
                folder_index.and_then(|index| collection.folders.get(index))
            })
            .map(|folder| folder.variables.as_slice())
            .unwrap_or(&[]);

        let mut pool = build_variable_pool(VariableLayers {
            global: &workspace.variables,
            collection: collection_variables,
            environment: self.active_environment_variables(),
            folder: folder_variables,
            request: request_variables,
        });
        merge_runtime_vars(&mut pool, runtime_vars);
        pool
    }

    pub(super) fn resolve_request_variables(
        &self,
        request: &Request,
        collection_index: Option<usize>,
        folder_index: Option<usize>,
    ) -> Request {
        let pool = self.variable_pool_for_source(
            collection_index,
            folder_index,
            &request.variables,
            &self.runtime_vars,
        );
        substitute_request(request, &pool)
    }

    fn fail_request_with_script_error(
        &mut self,
        tab_id: usize,
        message: String,
        timing: Option<RequestTimingBreakdown>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };

        tab.loading = false;
        tab.response_http_status = None;
        tab.response_status_text = None;
        tab.response_elapsed_ms = timing.map(|timing| timing.total_ms());
        tab.response_timing = timing;
        tab.response_size_bytes = None;
        tab.response_error = Some(message.clone());
        tab.response_body = ResponseBody::Text(message);
        tab.response_body_view = ResponseBodyView::Raw;
        tab.response_headers.clear();

        if self
            .tabs
            .get(self.active_tab)
            .is_some_and(|tab| tab.id == tab_id)
        {
            self.reload_response_body_input(window, cx);
        }
        cx.notify();
    }

    pub(super) fn send_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
            tab.response_http_status = None;
            tab.response_status_text = None;
            tab.response_elapsed_ms = None;
            tab.response_timing = None;
            tab.response_size_bytes = None;
            tab.response_error = None;
            tab.response_body = ResponseBody::empty();
            tab.response_body_view = ResponseBodyView::Raw;
            tab.response_headers.clear();
        }
        self.sync_active_tab_to_collection(cx);
        self.reload_response_body_input(window, cx);
        cx.notify();

        let tab_id = self.tabs[self.active_tab].id;
        let tab = self.tabs[self.active_tab].clone();
        let collection_index = tab.source.map(|source| source.collection);
        let folder_index = tab.source.and_then(|source| source.folder);
        let pre_request_script = tab.pre_request_script.clone();
        let post_response_script = tab.post_response_script.clone();
        let mut timing = RequestTimingBreakdown::default();

        if !pre_request_script.trim().is_empty() {
            let script_started = std::time::Instant::now();
            let host_state = self.build_script_host_state();
            match run_pre_request_script(&pre_request_script, host_state) {
                Ok(result) => self.apply_script_result(&result),
                Err(error) => {
                    timing.pre_request_script_ms = script_started.elapsed().as_millis();
                    self.fail_request_with_script_error(
                        tab_id,
                        format!("Pre-request script error: {error}"),
                        Some(timing),
                        window,
                        cx,
                    );
                    return;
                }
            }
            timing.pre_request_script_ms = script_started.elapsed().as_millis();
        }

        let resolved = self.resolve_request_variables(
            &tab.to_request(),
            collection_index,
            folder_index,
        );

        let url = resolved.url;
        let method = resolved.method;
        let query_params = resolved.query_params;
        let headers = resolved.headers;
        let body_type = resolved.body_type;
        let request_body = resolved.body;
        let form_fields = resolved.form_fields;
        let multipart_fields = resolved.multipart_fields;

        cx.spawn_in(window, async move |this, cx| {
            let result = block_on(send_http_request(
                url,
                method,
                query_params,
                headers,
                HttpRequestBody {
                    body_type,
                    raw_body: request_body,
                    form_fields,
                    multipart_fields,
                },
            ));

            cx.update(|window, app| {
                this.update(app, |app, cx| {
                    app.finish_request(
                        tab_id,
                        result,
                        post_response_script,
                        timing,
                        window,
                        cx,
                    );
                    cx.notify();
                })
                .ok();
            })
            .ok();
        })
        .detach();

        let _ = window;
    }

    fn finish_request(
        &mut self,
        tab_id: usize,
        result: HttpRequestResult,
        post_response_script: String,
        mut timing: RequestTimingBreakdown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab_index) = self.tabs.iter().position(|tab| tab.id == tab_id) else {
            return;
        };

        timing.http = result.timing;
        let response_for_script = result.response.as_ref().ok().cloned();

        {
            let tab = &mut self.tabs[tab_index];
            tab.loading = false;
            match &result.response {
                Ok(response) => {
                    tab.response_http_status = Some(response.status);
                    tab.response_status_text = Some(response.status_text.clone());
                    tab.response_error = None;
                    tab.response_body = response.body.clone();
                    tab.response_body_view = ResponseBodyView::Raw;
                    tab.response_headers = response.headers.clone();
                    tab.response_size_bytes = Some(response.size_bytes);
                }
                Err(error) => {
                    tab.response_http_status = None;
                    tab.response_status_text = None;
                    tab.response_size_bytes = None;
                    tab.response_error = Some(error.clone());
                    tab.response_body = ResponseBody::Text(error.clone());
                    tab.response_body_view = ResponseBodyView::Raw;
                    tab.response_headers.clear();
                }
            }
        }

        if !post_response_script.trim().is_empty() {
            if let Some(response) = response_for_script {
                let script_started = std::time::Instant::now();
                let host_state = self.build_script_host_state();
                match run_post_response_script(&post_response_script, host_state, &response) {
                    Ok(script_result) => self.apply_script_result(&script_result),
                    Err(error) => {
                        self.tabs[tab_index].response_error =
                            Some(format!("Post-response script error: {error}"));
                    }
                }
                timing.post_response_script_ms = script_started.elapsed().as_millis();
            }
        }

        let tab = &mut self.tabs[tab_index];
        tab.response_timing = Some(timing);
        tab.response_elapsed_ms = Some(timing.total_ms());

        if self
            .tabs
            .get(self.active_tab)
            .is_some_and(|tab| tab.id == tab_id)
        {
            self.reload_response_body_input(window, cx);
        }
    }

    pub(super) fn pick_multipart_file(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select a file".into()),
        });

        cx.spawn_in(window, async move |this, cx| {
            let path = match path.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let Some(path) = path else {
                return;
            };

            cx.update(|window, app| {
                this.update(app, |app, cx| {
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
