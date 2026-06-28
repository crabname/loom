use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    input::Input,
    scroll::ScrollableElement as _,
    select::Select,
    tab::TabBar,
    ActiveTheme as _, IconName, Sizable as _,
};

use crate::domain::BodyType;

use crate::app::tab::{RequestPanelTab, RequestScriptSubTab};

use super::LoomApp;

impl LoomApp {
    pub(super) fn render_request_panel(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let panel_tab = self
            .active_tab()
            .map(|tab| tab.request_panel_tab)
            .unwrap_or(RequestPanelTab::Params);

        let panel_content: AnyElement = match panel_tab {
            RequestPanelTab::Params => {
                let fields = self
                    .active_tab()
                    .map(|tab| tab.query_params.as_slice())
                    .unwrap_or(&[]);
                self.render_kv_table(super::fields::FieldTable::QueryParams, &self.query_inputs, fields, cx)
                    .into_any_element()
            }
            RequestPanelTab::Headers => {
                let fields = self
                    .active_tab()
                    .map(|tab| tab.headers.as_slice())
                    .unwrap_or(&[]);
                self.render_kv_table(
                    super::fields::FieldTable::RequestHeaders,
                    &self.header_inputs,
                    fields,
                    cx,
                )
                .into_any_element()
            }
            RequestPanelTab::Vars => {
                self.render_request_variables(cx).into_any_element()
            }
            RequestPanelTab::Script => self.render_request_scripts(cx).into_any_element(),
            RequestPanelTab::Body => {
                let body_type = self
                    .active_tab()
                    .map(|tab| tab.body_type)
                    .unwrap_or(BodyType::None);

                let mut column = v_flex()
                    .gap_2()
                    .size_full()
                    .child({
                        let mut header = h_flex()
                            .items_center()
                            .gap_2()
                            .child(div().flex_1().text_sm().child("Body"))
                            .child(
                                Select::new(&self.body_type_select)
                                    .appearance(false)
                                    .small(),
                            );

                        if matches!(body_type, BodyType::Json | BodyType::Xml) {
                            header = header.child(
                                Button::new("format-body")
                                    .label("Format")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.format_request_body(window, cx);
                                    })),
                            );
                        }

                        header
                    });

                column = match body_type {
                    BodyType::FormUrlEncoded => {
                        let fields = self
                            .active_tab()
                            .map(|tab| tab.form_fields.as_slice())
                            .unwrap_or(&[]);
                        column.child(self.render_kv_table(
                            super::fields::FieldTable::FormFields,
                            &self.form_inputs,
                            fields,
                            cx,
                        ))
                    }
                    BodyType::Multipart => {
                        let fields = self
                            .active_tab()
                            .map(|tab| tab.multipart_fields.as_slice())
                            .unwrap_or(&[]);
                        column.child(self.render_multipart_table(
                            super::fields::FieldTable::MultipartFields,
                            &self.multipart_inputs,
                            fields,
                            cx,
                        ))
                    }
                    BodyType::None => column.child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(cx.theme().muted_foreground)
                            .child("No body for this request"),
                    ),
                    BodyType::Json | BodyType::Xml => {
                        column.child(div().flex_1().child(Input::new(&self.body_input).h_full()))
                    }
                };

                column.into_any_element()
            }
        };

        v_flex()
            .gap_2()
            .p_3()
            .size_full()
            .min_h_0()
            .bg(cx.theme().background)
            .child(
                TabBar::new("request-panel-tabs")
                    .flex_shrink_0()
                    .underline()
                    .selected_index(match panel_tab {
                        RequestPanelTab::Params => 0,
                        RequestPanelTab::Headers => 1,
                        RequestPanelTab::Body => 2,
                        RequestPanelTab::Vars => 3,
                        RequestPanelTab::Script => 4,
                    })
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        if let Some(tab) = this.active_tab_mut() {
                            tab.request_panel_tab = match index {
                                0 => RequestPanelTab::Params,
                                1 => RequestPanelTab::Headers,
                                2 => RequestPanelTab::Body,
                                3 => RequestPanelTab::Vars,
                                _ => RequestPanelTab::Script,
                            };
                            cx.notify();
                        }
                    }))
                    .child("Params")
                    .child("Headers")
                    .child("Body")
                    .child("Vars")
                    .child("Script"),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(panel_content),
            )
    }

    fn render_request_scripts(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let script_sub_tab = self
            .active_tab()
            .map(|tab| tab.request_script_sub_tab)
            .unwrap_or(RequestScriptSubTab::PreRequest);

        let editor = match script_sub_tab {
            RequestScriptSubTab::PreRequest => Input::new(&self.pre_request_script_input),
            RequestScriptSubTab::PostResponse => Input::new(&self.post_response_script_input),
        };

        v_flex()
            .gap_2()
            .size_full()
            .child(
                TabBar::new("request-script-sub-tabs")
                    .flex_shrink_0()
                    .underline()
                    .selected_index(match script_sub_tab {
                        RequestScriptSubTab::PreRequest => 0,
                        RequestScriptSubTab::PostResponse => 1,
                    })
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        if let Some(tab) = this.active_tab_mut() {
                            tab.request_script_sub_tab = match index {
                                0 => RequestScriptSubTab::PreRequest,
                                _ => RequestScriptSubTab::PostResponse,
                            };
                            cx.notify();
                        }
                    }))
                    .child("Pre-request")
                    .child("Post-response"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        "Pre-request: host.* variables. Post-response: res (status, body, headers) \
                         and console.log / warn / error — output appears in Response → Console.",
                    ),
            )
            .child(div().flex_1().min_h_0().child(editor.h_full()))
    }

    fn render_request_variables(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let variables = self
            .active_tab()
            .map(|tab| tab.variables.as_slice())
            .unwrap_or(&[]);

        let mut list = v_flex().gap_1().child(
            h_flex()
                .gap_2()
                .px_2()
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
            let Some(row) = self.variable_inputs.get(index) else {
                continue;
            };

            list = list.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .px_2()
                    .child(div().flex_1().child(Input::new(&row.name)))
                    .child(div().flex_1().child(Input::new(&row.value)))
                    .child(
                        Button::new(("request-variable-remove", index))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.remove_request_variable(index, window, cx);
                            })),
                    ),
            );
        }

        v_flex()
            .gap_2()
            .size_full()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Request variables override collection, environment, and global values."),
            )
            .child(list.flex_1().overflow_y_scrollbar())
            .child(
                Button::new("request-variable-add")
                    .ghost()
                    .small()
                    .icon(IconName::Plus)
                    .label("Add variable")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.add_request_variable(window, cx);
                    })),
            )
    }
}
