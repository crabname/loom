use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    input::Input,
    scroll::ScrollableElement as _,
    select::Select,
    tab::TabBar,
    ActiveTheme as _, Disableable as _, IconName, Sizable as _,
};

use super::panel::{EnvironmentManagerTab, EnvironmentsManagerPanel};

impl EnvironmentsManagerPanel {
    pub(super) fn render_variable_editor(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let variables = self.current_scope_variables();
        let mut variables_list = v_flex().gap_1().child(
            h_flex()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Key"),
                )
                .child(
                    div()
                        .flex_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Value"),
                )
                .child(div().w(px(28.))),
        );

        for (index, _variable) in variables.iter().enumerate() {
            let Some(row) = self.variable_rows.get(index) else {
                continue;
            };

            variables_list = variables_list.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().child(Input::new(&row.name)))
                    .child(div().flex_1().child(Input::new(&row.value)))
                    .child(
                        Button::new(("manager-variable-remove", index))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.remove_variable_row(index, window, cx);
                            })),
                    ),
            );
        }

        v_flex()
            .gap_1()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Variables"),
            )
            .child(variables_list.flex_1().overflow_y_scrollbar())
            .child(
                Button::new("manager-variable-add")
                    .ghost()
                    .small()
                    .icon(IconName::Plus)
                    .label("Add variable")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.add_variable_row(window, cx);
                    })),
            )
    }
}

impl Render for EnvironmentsManagerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let environments = self.current_environments();
        let has_selection = self.tab.is_environment()
            && environments.get(self.selected_index).is_some();
        let can_delete = self.tab.is_environment() && !environments.is_empty();
        let folder_vars_available = !matches!(self.tab, EnvironmentManagerTab::FolderVars)
            || self
                .folder_names
                .get(self.collection_index)
                .is_some_and(|folders| !folders.is_empty());

        let mut env_list = v_flex().gap_1().w(px(200.)).flex_shrink_0();

        if self.tab.is_environment() {
            if environments.is_empty() {
                env_list = env_list.child(
                    div()
                        .px_2()
                        .py_4()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("No environments"),
                );
            } else {
                for (index, environment) in environments.iter().enumerate() {
                    let selected = index == self.selected_index;
                    env_list = env_list.child(
                        div()
                            .id(("manager-env", index))
                            .px_2()
                            .py_1p5()
                            .rounded_md()
                            .cursor_pointer()
                            .when(selected, |this| this.bg(cx.theme().accent))
                            .when(!selected, |this| {
                                this.hover(|style| style.bg(cx.theme().muted))
                            })
                            .text_sm()
                            .child(environment.name.clone())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.select_environment(index, window, cx);
                            })),
                    );
                }
            }
        }

        let mut editor = v_flex().flex_1().min_w_0().gap_2();

        if self.tab.is_environment() {
            editor = editor.child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Name"),
                    )
                    .child(Input::new(&self.name_input)),
            );
        }

        if (self.tab.is_environment() && has_selection)
            || (!self.tab.is_environment() && folder_vars_available)
        {
            editor = editor.child(self.render_variable_editor(cx));
        } else if matches!(self.tab, EnvironmentManagerTab::FolderVars) {
            editor = editor.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("No folders in this collection"),
            );
        }

        v_flex()
            .gap_3()
            .child(
                TabBar::new("environment-manager-tabs")
                    .underline()
                    .selected_index(match self.tab {
                        EnvironmentManagerTab::WorkspaceEnv => 0,
                        EnvironmentManagerTab::CollectionEnv => 1,
                        EnvironmentManagerTab::GlobalVars => 2,
                        EnvironmentManagerTab::CollectionVars => 3,
                        EnvironmentManagerTab::FolderVars => 4,
                    })
                    .on_click(cx.listener(|this, index: &usize, window, cx| {
                        let tab = match index {
                            0 => EnvironmentManagerTab::WorkspaceEnv,
                            1 => EnvironmentManagerTab::CollectionEnv,
                            2 => EnvironmentManagerTab::GlobalVars,
                            3 => EnvironmentManagerTab::CollectionVars,
                            _ => EnvironmentManagerTab::FolderVars,
                        };
                        this.switch_tab(tab, window, cx);
                    }))
                    .child("Workspace env")
                    .child("Collection env")
                    .child("Global vars")
                    .child("Collection vars")
                    .child("Folder vars"),
            )
            .when(self.tab.uses_collection_picker() && self.collection_select.is_some(), |this| {
                this.child(
                    div()
                        .w(px(240.))
                        .child(Select::new(self.collection_select.as_ref().unwrap())),
                )
            })
            .when(
                self.tab.uses_folder_picker()
                    && folder_vars_available
                    && self.folder_select.is_some(),
                |this| {
                    this.child(
                        div()
                            .w(px(240.))
                            .child(Select::new(self.folder_select.as_ref().unwrap())),
                    )
                },
            )
            .child(
                h_flex()
                    .gap_3()
                    .h(px(320.))
                    .when(self.tab.is_environment(), |this| {
                        this.child(env_list.overflow_y_scrollbar())
                    })
                    .child(editor),
            )
            .when(self.tab.is_environment(), |this| {
                this.child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("manager-env-add")
                                .ghost()
                                .small()
                                .icon(IconName::Plus)
                                .label("Add environment")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.add_environment(window, cx);
                                })),
                        )
                        .child(
                            Button::new("manager-env-delete")
                                .ghost()
                                .small()
                                .icon(IconName::Delete)
                                .label("Delete")
                                .disabled(!can_delete)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.delete_selected(window, cx);
                                })),
                        ),
                )
            })
    }
}
