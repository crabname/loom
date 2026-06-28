use gpui::*;

use crate::domain::{
    ensure_trailing_empty_row, format_request_url, query_params_equal, split_query_params,
    RequestProtocol,
};

use super::LoomApp;

impl LoomApp {
    pub(super) fn on_query_param_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_url_from_params(window, cx);
        cx.notify();
    }

    pub(super) fn sync_url_from_params(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.query_sync_guard {
            return;
        }

        self.flush_field_inputs(cx);

        let Some(tab) = self.active_tab() else {
            return;
        };
        let display_url = format_request_url(&tab.url, &tab.query_params);
        let current = self.url_input.read(cx).value().to_string();
        if current == display_url {
            return;
        }

        self.query_sync_guard = true;
        self.url_input.update(cx, |input, cx| {
            input.set_value(display_url, window, cx);
        });
        self.query_sync_guard = false;
        self.sync_active_tab_to_collection(cx);
    }

    pub(super) fn schedule_url_parse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.url_parse_debounce_seq += 1;
        let seq = self.url_parse_debounce_seq;
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(200))
                .await;
            cx.update(|window, app| {
                this.update(app, |app, cx| {
                    if app.url_parse_debounce_seq != seq {
                        return;
                    }
                    app.apply_url_to_query_params(window, cx);
                })
                .ok();
            })
            .ok();
        })
        .detach();
    }

    fn apply_url_to_query_params(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.query_sync_guard {
            return;
        }

        self.flush_field_inputs(cx);

        let full_url = self.url_input.read(cx).value().to_string();
        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        if tab.protocol == RequestProtocol::Grpc {
            if tab.url != full_url {
                tab.url = full_url;
                self.sync_active_tab_to_collection(cx);
                cx.notify();
            }
            return;
        }

        let (base, mut parsed) = split_query_params(&full_url);
        ensure_trailing_empty_row(&mut parsed);

        let base_changed = tab.url != base;
        let params_changed = !query_params_equal(&tab.query_params, &parsed);
        if !base_changed && !params_changed {
            return;
        }

        tab.url = base;
        tab.query_params = parsed;
        self.reload_field_inputs(window, cx);
        self.sync_active_tab_to_collection(cx);
        cx.notify();
    }

    pub(super) fn capture_url_query_state(&mut self, cx: &App) {
        if self.query_sync_guard {
            return;
        }

        let full_url = self.url_input.read(cx).value().to_string();
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            if tab.protocol == RequestProtocol::Grpc {
                tab.url = full_url;
                return;
            }
            let (base, mut parsed) = split_query_params(&full_url);
            ensure_trailing_empty_row(&mut parsed);
            tab.url = base;
            tab.query_params = parsed;
        }
    }
}
