use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    dialog::{DialogAction, DialogButtonProps, DialogClose, DialogFooter},
    input::{Input, InputState},
    v_flex,
    ActiveTheme as _, WindowExt as _,
};

use crate::domain::{parse_curl, request_to_curl, Request};

use super::LoomApp;

#[derive(Clone, Copy)]
pub(super) enum CurlImportTarget {
    ActiveTab,
    Collection {
        collection: usize,
        folder: Option<usize>,
    },
}

struct CurlDialogPanel {
    input: Entity<InputState>,
    error: Option<String>,
    show_copy: bool,
    description: SharedString,
}

impl CurlDialogPanel {
    fn new_import(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .rows(12)
                    .placeholder("curl 'https://api.example.com/endpoint' …")
            });

            Self {
                input,
                error: None,
                show_copy: false,
                description: "Paste a cURL command copied from the browser or terminal, then click Import."
                    .into(),
            }
        })
    }

    fn new_export(window: &mut Window, cx: &mut App, curl: String) -> Entity<Self> {
        cx.new(|cx| {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .rows(12)
                    .default_value(curl)
            });

            Self {
                input,
                error: None,
                show_copy: true,
                description: "Select text and copy with ⌘C / Ctrl+C, or use the copy icon."
                    .into(),
            }
        })
    }

    fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }
}

impl Render for CurlDialogPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(div().text_sm().text_color(cx.theme().muted_foreground).child(self.description.clone()))
            .child({
                let input = Input::new(&self.input).h_full();
                if self.show_copy {
                    div().h(px(240.)).child(input.suffix(
                        Clipboard::new("export-curl-copy")
                            .tooltip("Copy to clipboard")
                            .value_fn({
                                let input = self.input.clone();
                                move |_, cx| input.read(cx).value()
                            }),
                    ))
                } else {
                    div().h(px(240.)).child(input)
                }
            })
            .when_some(self.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().red)
                        .child(error),
                )
            })
    }
}

impl LoomApp {
    pub(super) fn active_request_as_curl(&mut self, cx: &App) -> Result<String, String> {
        self.flush_field_inputs(cx);
        self.capture_editor_state(cx);
        let tab = self.active_tab().ok_or("No active request")?;
        let request = tab.to_request();
        let collection_index = tab.source.map(|source| source.collection);
        let folder_index = tab.source.and_then(|source| source.folder);
        let resolved = self.resolve_request_variables(&request, collection_index, folder_index);
        request_to_curl(&resolved)
    }

    pub(super) fn collection_request_as_curl(
        &self,
        collection: usize,
        folder: Option<usize>,
        request: usize,
    ) -> Result<String, String> {
        let request = self
            .active_collections()
            .get(collection)
            .and_then(|collection_data| collection_data.request_ref(folder, request))
            .ok_or_else(|| "Request not found".to_string())?;
        let resolved = self.resolve_request_variables(request, Some(collection), folder);
        request_to_curl(&resolved)
    }

    pub(super) fn open_import_curl_dialog(
        &mut self,
        target: CurlImportTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panel = CurlDialogPanel::new_import(window, cx);
        let panel_for_ok = panel.clone();
        let view = cx.entity();

        window.open_dialog(cx, move |dialog, _, _| {
            let import_buttons = DialogButtonProps::default()
                .ok_text("Import")
                .show_cancel(true)
                .on_ok({
                    let panel_for_ok = panel_for_ok.clone();
                    let view = view.clone();
                    move |_, window, cx| {
                        let curl = panel_for_ok.read(cx).input.read(cx).value().to_string();
                        if curl.trim().is_empty() {
                            panel_for_ok.update(cx, |panel, cx| {
                                panel.set_error(Some("Paste a cURL command first".into()));
                                cx.notify();
                            });
                            return false;
                        }

                        match parse_curl(&curl) {
                            Err(error) => {
                                panel_for_ok.update(cx, |panel, cx| {
                                    panel.set_error(Some(error));
                                    cx.notify();
                                });
                                false
                            }
                            Ok(request) => {
                                view.update(cx, |app, cx| {
                                    app.apply_imported_request(request, target, window, cx);
                                });
                                true
                            }
                        }
                    }
                });

            dialog
                .title("Import cURL")
                .w(px(640.))
                .child(panel.clone())
                .button_props(import_buttons)
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new()
                                .child(Button::new("cancel-import-curl").label("Cancel")),
                        )
                        .child(
                            DialogAction::new().child(
                                Button::new("import-curl")
                                    .primary()
                                    .label("Import"),
                            ),
                        ),
                )
        });
    }

    pub(super) fn open_export_curl_dialog(
        &mut self,
        curl: Result<String, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match curl {
            Ok(curl) => {
                let panel = CurlDialogPanel::new_export(window, cx, curl);

                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .title("Export cURL")
                        .w(px(640.))
                        .child(panel.clone())
                        .footer(
                            DialogFooter::new().child(
                                Button::new("close-export-curl")
                                    .primary()
                                    .label("Close")
                                    .on_click(|_, window, cx| {
                                        window.close_dialog(cx);
                                    }),
                            ),
                        )
                });
            }
            Err(error) => {
                window.push_notification(gpui_component::notification::Notification::error(error), cx);
            }
        }
    }

    fn apply_imported_request(
        &mut self,
        mut request: Request,
        target: CurlImportTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match target {
            CurlImportTarget::ActiveTab => {
                self.flush_field_inputs(cx);
                self.capture_editor_state(cx);
                self.apply_request_to_active_tab(&request, window, cx);
            }
            CurlImportTarget::Collection { collection, folder } => {
                let collection_data = self
                    .active_collections_mut()
                    .get_mut(collection)
                    .expect("collection exists");
                let request_count = match folder {
                    None => collection_data.requests.len(),
                    Some(folder_index) => collection_data
                        .folders
                        .get(folder_index)
                        .map(|folder| folder.requests.len())
                        .unwrap_or(0),
                };
                let number = request_count + 1;
                request.name = if number == 1 {
                    "Imported Request".into()
                } else {
                    format!("Imported Request {number}")
                };
                collection_data.expanded = true;
                let request_index = collection_data.push_request(folder, request);
                self.refresh_collections_tree(cx);
                self.open_request_tab(collection, folder, request_index, window, cx);
            }
        }
    }

    fn apply_request_to_active_tab(
        &mut self,
        request: &Request,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.active_tab_mut() {
            tab.url = request.url.clone();
            tab.method = request.method;
            tab.query_params = request.query_params.clone();
            tab.headers = request.headers.clone();
            tab.body_type = request.body_type;
            tab.request_body = request.body.clone();
            tab.form_fields = request.form_fields.clone();
            tab.multipart_fields = request.multipart_fields.clone();
            tab.variables = request.variables.clone();
        }

        self.sync_active_tab_to_collection(cx);
        self.reload_active_tab_inputs(window, cx);
        cx.notify();
    }
}
