use gpui::*;
use gpui_component::{notification::Notification, WindowExt as _};

use crate::domain::{BodyType, GrpcMethodInfo, RequestProtocol};
use crate::transport::{block_on, discover_grpc_methods, generate_grpc_request_template};

use super::LoomApp;

impl LoomApp {
    pub(super) fn active_protocol(&self) -> RequestProtocol {
        self.active_tab()
            .map(|tab| tab.protocol)
            .unwrap_or_default()
    }

    pub(super) fn discover_grpc_services(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_protocol() != RequestProtocol::Grpc {
            return;
        }

        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.capture_url_query_state(cx);

        let endpoint = self.url_input.read(cx).value().to_string();
        let view = cx.entity();

        cx.spawn_in(window, async move |_, window| {
            let result = block_on(discover_grpc_methods(&endpoint));
            window
                .update(|window, cx| {
                    view.update(cx, |app, cx| {
                        app.finish_grpc_discover(result, window, cx);
                    })
                })
                .ok();
        })
        .detach();
    }

    fn finish_grpc_discover(
        &mut self,
        result: Result<Vec<GrpcMethodInfo>, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(methods) => {
                self.grpc_discovered_methods = methods.clone();
                let labels = methods
                    .iter()
                    .map(|method| method.label().into())
                    .collect::<Vec<SharedString>>();
                let first_method = self.grpc_discovered_methods.first().cloned();
                self.grpc_method_select.update(cx, |select, cx| {
                    select.set_items(labels, window, cx);
                    if let Some(first) = &first_method {
                        select.set_selected_value(&first.label().into(), window, cx);
                    }
                });
                if let Some(first) = first_method {
                    self.apply_discovered_grpc_method(&first, window, cx);
                }
                if methods.is_empty() {
                    window.push_notification(
                        Notification::warning("No unary gRPC methods discovered").message(
                            "The server did not expose any unary methods via reflection",
                        ),
                        cx,
                    );
                } else {
                    window.push_notification(
                        Notification::success("gRPC services discovered")
                            .message(format!("Found {} unary methods", methods.len())),
                        cx,
                    );
                }
            }
            Err(error) => {
                window.push_notification(
                    Notification::error("gRPC discovery failed").message(error),
                    cx,
                );
            }
        }
        cx.notify();
    }

    pub(super) fn apply_discovered_grpc_method(
        &mut self,
        method: &GrpcMethodInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.active_tab_mut() {
            tab.grpc_service = method.service.clone();
            tab.grpc_method = method.method.clone();
        }
        self.grpc_service_input.update(cx, |input, cx| {
            input.set_value(method.service.clone(), window, cx);
        });
        self.grpc_method_input.update(cx, |input, cx| {
            input.set_value(method.method.clone(), window, cx);
        });
        self.sync_active_tab_to_collection(cx);
    }

    pub(super) fn fill_grpc_request_body_from_schema(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_protocol() != RequestProtocol::Grpc {
            return;
        }

        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        self.capture_url_query_state(cx);

        let endpoint = self.url_input.read(cx).value().to_string();
        let service = self.grpc_service_input.read(cx).value().to_string();
        let method = self.grpc_method_input.read(cx).value().to_string();

        if service.trim().is_empty() || method.trim().is_empty() {
            window.push_notification(
                Notification::warning("gRPC service and method are required").message(
                    "Set service and method before generating a request body template",
                ),
                cx,
            );
            return;
        }

        let view = cx.entity();
        cx.spawn_in(window, async move |_, window| {
            let result = block_on(generate_grpc_request_template(&endpoint, &service, &method));
            window
                .update(|window, cx| {
                    view.update(cx, |app, cx| {
                        app.finish_grpc_body_template(result, window, cx);
                    })
                })
                .ok();
        })
        .detach();
    }

    fn finish_grpc_body_template(
        &mut self,
        result: Result<String, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(template) => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.request_body = template.clone();
                    tab.body_type = BodyType::Json;
                }
                self.body_input.update(cx, |input, cx| {
                    input.set_highlighter("json", cx);
                    input.set_value(template, window, cx);
                });
                self.body_type_select.update(cx, |select, cx| {
                    select.set_selected_value(&BodyType::Json.label(), window, cx);
                });
                self.sync_active_tab_to_collection(cx);
                window.push_notification(
                    Notification::success("Request body filled from schema"),
                    cx,
                );
                cx.notify();
            }
            Err(error) => {
                window.push_notification(
                    Notification::error("Failed to build request template").message(error),
                    cx,
                );
            }
        }
    }
}
