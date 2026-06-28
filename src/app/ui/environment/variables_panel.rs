use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    input::Input,
    scroll::ScrollableElement as _,
    ActiveTheme as _, IconName, Sizable as _,
};

use crate::domain::Variable;

use crate::app::ui::fields::RowInputs;
use super::variables::{build_variable_row_inputs, flush_environment_variables};

pub(crate) struct VariablesPanel {
    variables: Vec<Variable>,
    variable_rows: Vec<RowInputs>,
}

impl VariablesPanel {
    pub(crate) fn new(window: &mut Window, cx: &mut App, variables: Vec<Variable>) -> Entity<Self> {
        let mut variables = variables;
        if variables.is_empty() {
            variables.push(Variable::empty());
        }
        let variable_rows = build_variable_row_inputs(window, cx, &variables);

        cx.new(|_| Self {
            variables,
            variable_rows,
        })
    }

    fn flush(&mut self, cx: &App) {
        flush_environment_variables(&mut self.variables, &self.variable_rows, cx);
    }

    fn reload_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.variables.is_empty() {
            self.variables.push(Variable::empty());
        }
        self.variable_rows = build_variable_row_inputs(window, cx, &self.variables);
    }

    pub(crate) fn add_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush(cx);
        self.variables.push(Variable::empty());
        self.reload_rows(window, cx);
        cx.notify();
    }

    pub(crate) fn remove_row(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.flush(cx);
        if self.variables.len() > 1 {
            self.variables.remove(index);
        } else {
            self.variables[0] = Variable::empty();
        }
        self.reload_rows(window, cx);
        cx.notify();
    }

    pub(crate) fn take_variables(&mut self, cx: &App) -> Vec<Variable> {
        self.flush(cx);
        std::mem::take(&mut self.variables)
    }
}

impl Render for VariablesPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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

        for (index, row) in self.variable_rows.iter().enumerate() {
            variables_list = variables_list.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().child(Input::new(&row.name)))
                    .child(div().flex_1().child(Input::new(&row.value)))
                    .child(
                        Button::new(("scope-variable-remove", index))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.remove_row(index, window, cx);
                            })),
                    ),
            );
        }

        v_flex()
            .gap_2()
            .child(variables_list.flex_1().overflow_y_scrollbar())
            .child(
                Button::new("scope-variable-add")
                    .ghost()
                    .small()
                    .icon(IconName::Plus)
                    .label("Add variable")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.add_row(window, cx);
                    })),
            )
    }
}
