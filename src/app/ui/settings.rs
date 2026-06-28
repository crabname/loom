use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dialog::{DialogAction, DialogButtonProps, DialogClose, DialogFooter},
    h_flex, v_flex,
    notification::Notification,
    ActiveTheme as _, StyledExt as _, WindowExt as _,
};

use crate::storage::{clear_local_workspace_dir, default_collection_paths, BootstrapConfig};

use super::super::{LoomApp, WorkspaceBinding};

impl LoomApp {
    pub(super) fn on_open_settings(
        &mut self,
        _: &super::super::OpenSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_settings_dialog(window, cx);
    }

    pub(crate) fn show_startup_warnings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.startup_warnings.is_empty() {
            return;
        }

        let preview = self
            .startup_warnings
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let suffix = if self.startup_warnings.len() > 3 {
            format!("\n…and {} more", self.startup_warnings.len() - 3)
        } else {
            String::new()
        };

        window.push_notification(
            Notification::warning("Some workspace data could not be loaded")
                .message(format!("{preview}{suffix}")),
            cx,
        );
        self.startup_warnings.clear();
    }

    pub(super) fn open_settings_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let data_root = self.app_paths.root.display().to_string();
        let default_root = BootstrapConfig::default_data_root().display().to_string();
        let is_custom = self.app_paths.uses_custom_root();
        let workspace_name = self
            .workspaces
            .get(self.active_workspace)
            .map(|workspace| workspace.name.clone())
            .unwrap_or_else(|| "Untitled".into());
        let local_path = self.workspace_bindings[self.active_workspace]
            .local_path()
            .map(|path| path.display().to_string());
        let is_ephemeral =
            matches!(self.workspace_bindings[self.active_workspace], WorkspaceBinding::Ephemeral);
        let view = cx.entity();

        window.open_dialog(cx, move |dialog, _, cx| {
            let mut data_section = v_flex()
                .gap_2()
                .child(div().text_sm().font_semibold().child("Data directory"))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            "Application data, workspaces, and app state are stored here. \
                             Restart the app after changing this path.",
                        ),
                )
                .child(
                    div()
                        .text_sm()
                        .font_family("monospace")
                        .child(data_root.clone()),
                )
                .child(
                    h_flex().gap_2().child(
                        Button::new("change-data-dir")
                            .label("Change…")
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    view.update(cx, |app, cx| {
                                        app.prompt_change_data_directory(window, cx);
                                    });
                                }
                            }),
                    ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("Default: {default_root}")),
                );

            if is_custom {
                data_section = data_section.child(
                    Button::new("reset-data-dir")
                        .label("Reset to default")
                        .on_click({
                            let view = view.clone();
                            move |_, window, cx| {
                                view.update(cx, |app, cx| {
                                    app.reset_data_directory(window, cx);
                                });
                            }
                        }),
                );
            }

            let mut workspace_section = v_flex()
                .gap_2()
                .child(div().text_sm().font_semibold().child("Current workspace"))
                .child(div().text_sm().child(workspace_name.clone()));

            if let Some(path) = local_path.clone() {
                workspace_section = workspace_section.child(
                    div()
                        .text_xs()
                        .font_family("monospace")
                        .child(path),
                );
            }

            if is_ephemeral {
                workspace_section = workspace_section.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Demo workspace — not saved to disk until edited."),
                );
            }

            workspace_section = workspace_section
                .child(
                    Button::new("clear-workspace-data")
                        .label("Clear workspace data")
                        .on_click({
                            let view = view.clone();
                            move |_, window, cx| {
                                view.update(cx, |app, cx| {
                                    app.confirm_clear_workspace_data(window, cx);
                                });
                            }
                        }),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            "Deletes the workspace folder from disk and resets to the demo \
                             collection. This cannot be undone.",
                        ),
                );

            dialog
                .title("Settings")
                .w(px(640.))
                .child(v_flex().gap_4().p_1().child(data_section).child(workspace_section))
                .footer(
                    DialogFooter::new().child(
                        DialogClose::new().child(Button::new("close-settings").label("Close")),
                    ),
                )
        });
    }

    fn prompt_change_data_directory(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Select application data directory".into()),
        });
        let view = cx.entity();

        cx.spawn_in(window, async move |_, cx| {
            let Some(path) = paths
                .await
                .ok()
                .and_then(|result| result.ok())
                .and_then(|paths| paths)
                .and_then(|mut paths| paths.pop())
            else {
                return;
            };

            cx.update(|window, cx| {
                view.update(cx, |app, cx| {
                    app.apply_data_directory_change(path, window, cx);
                })
            })
            .ok();
        })
        .detach();
    }

    fn apply_data_directory_change(
        &mut self,
        path: std::path::PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match BootstrapConfig::set_data_root(path) {
            Ok(()) => {
                window.push_notification(
                    Notification::info("Data directory updated")
                        .message("Restart the application to use the new data folder."),
                    cx,
                );
            }
            Err(error) => {
                window.push_notification(
                    Notification::error("Failed to update data directory").message(error),
                    cx,
                );
            }
        }
    }

    fn reset_data_directory(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match BootstrapConfig::reset_data_root() {
            Ok(()) => {
                window.push_notification(
                    Notification::info("Data directory reset")
                        .message("Restart the application to use the default data folder."),
                    cx,
                );
            }
            Err(error) => {
                window.push_notification(
                    Notification::error("Failed to reset data directory").message(error),
                    cx,
                );
            }
        }
    }

    fn confirm_clear_workspace_data(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity();
        window.open_dialog(cx, move |dialog, _, _| {
            let clear_buttons = DialogButtonProps::default()
                .ok_text("Clear")
                .show_cancel(true)
                .on_ok({
                    let view = view.clone();
                    move |_, window, cx| {
                        view.update(cx, |app, cx| {
                            app.clear_active_workspace_data(window, cx);
                        });
                        true
                    }
                });

            dialog
                .title("Clear workspace data?")
                .child(
                    div().text_sm().child(
                        "This permanently deletes the workspace folder on disk and resets the \
                         current workspace to the demo collection.",
                    ),
                )
                .button_props(clear_buttons)
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new()
                                .child(Button::new("cancel-clear-workspace").label("Cancel")),
                        )
                        .child(
                            DialogAction::new().child(
                                Button::new("confirm-clear-workspace")
                                    .danger()
                                    .label("Clear"),
                            ),
                        ),
                )
        });
    }

    fn clear_active_workspace_data(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() {
            return;
        }

        self.flush_workspace_edits(cx);

        let index = self.active_workspace;
        if let Some(path) = self.workspace_bindings[index].local_path() {
            if let Err(error) = clear_local_workspace_dir(path) {
                window.push_notification(
                    Notification::error("Failed to clear workspace").message(error),
                    cx,
                );
                return;
            }
            self.workspace_bindings[index] = WorkspaceBinding::Ephemeral;
        }

        let demo_workspace = crate::demo::demo_workspaces()
            .into_iter()
            .next()
            .expect("demo workspace");

        self.workspaces[index] = demo_workspace.clone();
        self.workspace_collection_paths[index] = default_collection_paths(&demo_workspace);
        self.persist_app_state();
        self.refresh_collections_tree(cx);
        self.reset_workspace_ui(window, cx);
        self.sync_collections_tree_selection(cx);

        window.push_notification(
            Notification::info("Workspace cleared").message(
                "Workspace data was removed. The demo collection is loaded in memory.",
            ),
            cx,
        );
        cx.notify();
    }
}
