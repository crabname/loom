use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dialog::{DialogAction, DialogButtonProps, DialogClose, DialogFooter},
    input::{Input, InputState},
    v_flex,
    ActiveTheme as _, WindowExt as _,
};

use super::LoomApp;

#[derive(Clone, Copy)]
pub(super) enum RenameTarget {
    Collection { collection: usize },
    Folder { collection: usize, folder: usize },
    Request {
        collection: usize,
        folder: Option<usize>,
        request: usize,
    },
}

struct RenameDialogPanel {
    input: Entity<InputState>,
    error: Option<String>,
}

impl RenameDialogPanel {
    fn new(window: &mut Window, cx: &mut App, current_name: &str) -> Entity<Self> {
        cx.new(|cx| {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Name")
                    .default_value(current_name)
            });

            Self {
                input,
                error: None,
            }
        })
    }

    fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }
}

impl Render for RenameDialogPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(Input::new(&self.input))
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
    fn rename_target_name(&self, target: RenameTarget) -> Option<String> {
        match target {
            RenameTarget::Collection { collection } => self
                .active_collections()
                .get(collection)
                .map(|collection| collection.name.clone()),
            RenameTarget::Folder { collection, folder } => self
                .active_collections()
                .get(collection)
                .and_then(|collection| collection.folders.get(folder))
                .map(|folder| folder.name.clone()),
            RenameTarget::Request {
                collection,
                folder,
                request,
            } => self
                .active_collections()
                .get(collection)
                .and_then(|collection| collection.request_ref(folder, request))
                .map(|request| request.name.clone()),
        }
    }

    pub(super) fn open_rename_dialog(
        &mut self,
        target: RenameTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(current_name) = self.rename_target_name(target) else {
            return;
        };

        let title = match target {
            RenameTarget::Collection { .. } => "Rename Collection",
            RenameTarget::Folder { .. } => "Rename Folder",
            RenameTarget::Request { .. } => "Rename Request",
        };

        let panel = RenameDialogPanel::new(window, cx, &current_name);
        let panel_for_ok = panel.clone();
        let view = cx.entity();

        window.open_dialog(cx, move |dialog, _, _| {
            let rename_buttons = DialogButtonProps::default()
                .ok_text("Rename")
                .show_cancel(true)
                .on_ok({
                    let panel_for_ok = panel_for_ok.clone();
                    let view = view.clone();
                    move |_, window, cx| {
                        let name = panel_for_ok.read(cx).input.read(cx).value().to_string();
                        if name.trim().is_empty() {
                            panel_for_ok.update(cx, |panel, cx| {
                                panel.set_error(Some("Name cannot be empty".into()));
                                cx.notify();
                            });
                            return false;
                        }

                        view.update(cx, |app, cx| {
                            app.apply_rename(target, name, window, cx);
                        });
                        true
                    }
                });

            dialog
                .title(title)
                .w(px(400.))
                .child(panel.clone())
                .button_props(rename_buttons)
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new()
                                .child(Button::new("cancel-rename").label("Cancel")),
                        )
                        .child(
                            DialogAction::new().child(
                                Button::new("confirm-rename")
                                    .primary()
                                    .label("Rename"),
                            ),
                        ),
                )
        });
    }

    fn apply_rename(
        &mut self,
        target: RenameTarget,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match target {
            RenameTarget::Collection { collection } => {
                self.rename_collection(collection, name, window, cx);
            }
            RenameTarget::Folder { collection, folder } => {
                self.rename_folder(collection, folder, name, cx);
            }
            RenameTarget::Request {
                collection,
                folder,
                request,
            } => self.rename_request(collection, folder, request, name, cx),
        }
    }
}
