use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dialog::{DialogAction, DialogButtonProps, DialogClose, DialogFooter},
    WindowExt as _,
};

use crate::domain::Variable;

use super::variables_panel::VariablesPanel;
use super::LoomApp;

#[derive(Clone, Copy)]
enum ScopeVariablesTarget {
    Workspace,
    Collection(usize),
    Folder { collection: usize, folder: usize },
}

impl LoomApp {
    fn open_scope_variables_dialog(
        &mut self,
        title: String,
        variables: Vec<Variable>,
        target: ScopeVariablesTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panel = VariablesPanel::new(window, cx, variables);
        let panel_for_ok = panel.clone();
        let view = cx.entity();
        let dialog_title = title.clone();

        window.open_dialog(cx, move |dialog, _, _| {
            let save_buttons = DialogButtonProps::default()
                .ok_text("Save")
                .show_cancel(true)
                .on_ok({
                    let panel_for_ok = panel_for_ok.clone();
                    let view = view.clone();
                    move |_, _, cx| {
                        let variables =
                            panel_for_ok.update(cx, |panel, cx| panel.take_variables(cx));

                        view.update(cx, |app, cx| match target {
                            ScopeVariablesTarget::Workspace => {
                                app.apply_workspace_variables(variables, cx);
                            }
                            ScopeVariablesTarget::Collection(collection_index) => {
                                app.apply_collection_variables(collection_index, variables, cx);
                            }
                            ScopeVariablesTarget::Folder {
                                collection,
                                folder,
                            } => {
                                app.apply_folder_variables(collection, folder, variables, cx);
                            }
                        });

                        true
                    }
                });

            dialog
                .title(dialog_title.clone())
                .w(px(480.))
                .child(panel.clone())
                .button_props(save_buttons)
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new()
                                .child(Button::new("cancel-scope-variables").label("Cancel")),
                        )
                        .child(
                            DialogAction::new().child(
                                Button::new("save-scope-variables")
                                    .primary()
                                    .label("Save"),
                            ),
                        ),
                )
        });
    }

    pub(in crate::app::ui) fn open_workspace_variables_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspace_name = self.workspaces[self.active_workspace].name.clone();
        let variables = self.workspaces[self.active_workspace].variables.clone();
        let title = format!("Workspace variables · {workspace_name}");

        self.open_scope_variables_dialog(
            title,
            variables,
            ScopeVariablesTarget::Workspace,
            window,
            cx,
        );
    }

    pub(in crate::app::ui) fn open_collection_variables_dialog(
        &mut self,
        collection_index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(collection) = self.active_collections().get(collection_index) else {
            return;
        };

        let collection_name = collection.name.clone();
        let variables = collection.variables.clone();
        let title = format!("Collection variables · {collection_name}");

        self.open_scope_variables_dialog(
            title,
            variables,
            ScopeVariablesTarget::Collection(collection_index),
            window,
            cx,
        );
    }

    pub(in crate::app::ui) fn open_folder_variables_dialog(
        &mut self,
        collection_index: usize,
        folder_index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(collection) = self.active_collections().get(collection_index) else {
            return;
        };
        let Some(folder) = collection.folders.get(folder_index) else {
            return;
        };

        let folder_name = folder.name.clone();
        let collection_name = collection.name.clone();
        let variables = folder.variables.clone();
        let title = format!("Folder variables · {collection_name} / {folder_name}");

        self.open_scope_variables_dialog(
            title,
            variables,
            ScopeVariablesTarget::Folder {
                collection: collection_index,
                folder: folder_index,
            },
            window,
            cx,
        );
    }
}
