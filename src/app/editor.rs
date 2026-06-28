use gpui::*;
use gpui_component::WindowExt;

use crate::domain::{format_body, format_request_url, ResponseBody};

use super::ui::{
    build_multipart_row_inputs, build_row_inputs, build_variable_row_inputs,
    flush_environment_variables, flush_multipart_rows, flush_rows,
};
use super::ApiHelperApp;

impl ApiHelperApp {
    pub(super) fn flush_field_inputs(&mut self, cx: &App) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };

        flush_rows(&mut tab.query_params, &self.query_inputs, cx);
        flush_rows(&mut tab.headers, &self.header_inputs, cx);
        flush_rows(&mut tab.form_fields, &self.form_inputs, cx);
        flush_multipart_rows(&mut tab.multipart_fields, &self.multipart_inputs, cx);
        flush_environment_variables(&mut tab.variables, &self.variable_inputs, cx);
    }

    pub(super) fn reload_field_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tabs.get(self.active_tab).cloned();
        let Some(tab) = tab else {
            return;
        };

        self.query_inputs = build_row_inputs(window, cx, &tab.query_params);
        self.header_inputs = build_row_inputs(window, cx, &tab.headers);
        self.form_inputs = build_row_inputs(window, cx, &tab.form_fields);
        self.multipart_inputs = build_multipart_row_inputs(window, cx, &tab.multipart_fields);
        self.variable_inputs = build_variable_row_inputs(window, cx, &tab.variables);
        self.wire_query_param_subscriptions(window, cx);
    }

    pub(super) fn reload_active_tab_inputs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab = self.tabs.get(self.active_tab).cloned();
        let Some(tab) = tab else {
            return;
        };

        self.url_input.update(cx, |input, cx| {
            input.set_value(format_request_url(&tab.url, &tab.query_params), window, cx);
        });
        self.method_select.update(cx, |select, cx| {
            select.set_selected_value(&tab.method.as_str(), window, cx);
        });
        self.body_type_select.update(cx, |select, cx| {
            select.set_selected_value(&tab.body_type.label(), window, cx);
        });
        self.reload_body_input(window, cx);
        self.reload_script_inputs(window, cx);
        self.reload_response_body_input(window, cx);
        self.reload_field_inputs(window, cx);
    }

    pub(super) fn reload_response_body_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tabs.get(self.active_tab).cloned();
        let Some(tab) = tab else {
            return;
        };

        let (text, language) = match &tab.response_body {
            ResponseBody::Text(text) => {
                let language = crate::domain::response_body_language(
                    crate::domain::response_content_type(&tab.response_headers).as_deref(),
                    text,
                )
                .unwrap_or("text");
                (text.clone(), language)
            }
            ResponseBody::Binary { .. } => (String::new(), "text"),
        };

        self.response_body_input.update(cx, |input, cx| {
            input.set_highlighter(language, cx);
            input.set_value(text, window, cx);
        });
    }

    pub(super) fn reload_body_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tabs.get(self.active_tab).cloned();
        let Some(tab) = tab else {
            return;
        };

        self.body_input.update(cx, |input, cx| {
            input.set_value(tab.request_body.clone(), window, cx);
        });
    }

    pub(super) fn format_request_body(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn capture_editor_state(&mut self, cx: &App) {
        self.flush_field_inputs(cx);
        self.capture_url_query_state(cx);
        let body = self.body_input.read(cx).value().to_string();
        let pre_request_script = self.pre_request_script_input.read(cx).value().to_string();
        let post_response_script = self.post_response_script_input.read(cx).value().to_string();
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.request_body = body;
            tab.pre_request_script = pre_request_script;
            tab.post_response_script = post_response_script;
        }
    }

    pub(super) fn reload_script_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tabs.get(self.active_tab).cloned();
        let Some(tab) = tab else {
            return;
        };

        self.pre_request_script_input.update(cx, |input, cx| {
            input.set_value(tab.pre_request_script.clone(), window, cx);
        });
        self.post_response_script_input.update(cx, |input, cx| {
            input.set_value(tab.post_response_script.clone(), window, cx);
        });
    }
}
