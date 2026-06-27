use gpui::*;
use gpui_component::{
    h_flex, v_flex,
    input::Input,
    scroll::ScrollableElement as _,
    select::Select,
    tab::TabBar,
    ActiveTheme as _,
};

use crate::domain::BodyType;
use crate::app::tab::RequestPanelTab;

use super::fields::FieldTable;
use super::ApiHelperApp;

impl ApiHelperApp {
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
                self.render_kv_table(FieldTable::QueryParams, &self.query_inputs, fields, cx)
                    .into_any_element()
            }
            RequestPanelTab::Headers => {
                let fields = self
                    .active_tab()
                    .map(|tab| tab.headers.as_slice())
                    .unwrap_or(&[]);
                self.render_kv_table(
                    FieldTable::RequestHeaders,
                    &self.header_inputs,
                    fields,
                    cx,
                )
                .into_any_element()
            }
            RequestPanelTab::Body => {
                let body_type = self
                    .active_tab()
                    .map(|tab| tab.body_type)
                    .unwrap_or(BodyType::None);

                let mut column = v_flex()
                    .gap_2()
                    .size_full()
                    .child(
                        h_flex()
                            .items_center()
                            .child(div().flex_1().text_sm().child("Body"))
                            .child(Select::new(&self.body_type_select).w(px(180.))),
                    );

                column = match body_type {
                    BodyType::FormUrlEncoded => {
                        let fields = self
                            .active_tab()
                            .map(|tab| tab.form_fields.as_slice())
                            .unwrap_or(&[]);
                        column.child(self.render_kv_table(
                            FieldTable::FormFields,
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
                            FieldTable::MultipartFields,
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
                    })
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        if let Some(tab) = this.active_tab_mut() {
                            tab.request_panel_tab = match index {
                                0 => RequestPanelTab::Params,
                                1 => RequestPanelTab::Headers,
                                _ => RequestPanelTab::Body,
                            };
                            cx.notify();
                        }
                    }))
                    .child("Params")
                    .child("Headers")
                    .child("Body"),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(panel_content),
            )
    }
}
