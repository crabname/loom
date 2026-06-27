mod manager;
mod variables;

pub(crate) use variables::{build_variable_row_inputs, flush_environment_variables};

use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dialog::{DialogAction, DialogButtonProps, DialogClose, DialogFooter},
    h_flex,
    select::Select,
    IconName, Sizable as _, WindowExt as _,
};

use crate::domain::Variable;

use manager::EnvironmentsManagerPanel;
use super::ApiHelperApp;

impl ApiHelperApp {
    pub(super) fn render_environment_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_1()
            .items_center()
            .child(
                div()
                    .w(px(200.))
                    .min_w_0()
                    .child(Select::new(&self.environment_select)),
            )
            .child(
                Button::new("environment-manage")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Settings)
                    .tooltip("Manage environments and variables")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_environments_manager_dialog(window, cx);
                    })),
            )
    }

    pub(super) fn open_environments_manager_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspace = &self.workspaces[self.active_workspace];
        let folder_names: Vec<Vec<SharedString>> = workspace
            .collections
            .iter()
            .map(|collection| {
                collection
                    .folders
                    .iter()
                    .map(|folder| folder.name.clone().into())
                    .collect()
            })
            .collect();
        let folder_variables: Vec<Vec<Vec<Variable>>> = workspace
            .collections
            .iter()
            .map(|collection| {
                collection
                    .folders
                    .iter()
                    .map(|folder| folder.variables.clone())
                    .collect()
            })
            .collect();
        let panel = EnvironmentsManagerPanel::new(
            window,
            cx,
            workspace.environments.clone(),
            workspace.variables.clone(),
            workspace
                .collections
                .iter()
                .map(|collection| collection.name.clone().into())
                .collect(),
            workspace
                .collections
                .iter()
                .map(|collection| collection.environments.clone())
                .collect(),
            workspace
                .collections
                .iter()
                .map(|collection| collection.variables.clone())
                .collect(),
            folder_names,
            folder_variables,
        );
        let panel_for_ok = panel.clone();
        let view = cx.entity();

        window.open_dialog(cx, move |dialog, _, _| {
            let save_buttons = DialogButtonProps::default()
                .ok_text("Save")
                .show_cancel(true)
                .on_ok({
                    let panel_for_ok = panel_for_ok.clone();
                    let view = view.clone();
                    move |_, window, cx| {
                        let (
                            workspace_environments,
                            collection_environments,
                            workspace_variables,
                            collection_variables,
                            folder_variables,
                        ) = panel_for_ok.update(cx, |panel, cx| panel.take_state(cx));

                        view.update(cx, |app, cx| {
                            app.apply_environments_manager(
                                workspace_environments,
                                collection_environments,
                                workspace_variables,
                                collection_variables,
                                folder_variables,
                                window,
                                cx,
                            );
                        });

                        true
                    }
                });

            dialog
                .title("Manage environments and variables")
                .w(px(720.))
                .child(panel.clone())
                .button_props(save_buttons)
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new()
                                .child(Button::new("cancel-environments-manager").label("Cancel")),
                        )
                        .child(
                            DialogAction::new().child(
                                Button::new("save-environments-manager")
                                    .primary()
                                    .label("Save"),
                            ),
                        ),
                )
        });
    }
}
