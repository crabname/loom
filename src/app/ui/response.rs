use gpui::*;
use gpui_component::{
    h_flex, v_flex,
    scroll::ScrollableElement as _,
    tab::TabBar,
    text::markdown,
    ActiveTheme as _,
};

use crate::domain::KeyValueField;
use crate::app::tab::ResponsePanelTab;

use super::ApiHelperApp;

impl ApiHelperApp {
    fn render_response_headers(&self, headers: &[KeyValueField], cx: &Context<Self>) -> AnyElement {
        if headers.is_empty() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child("Response headers will appear here after sending a request")
                .into_any_element();
        }

        let header_row = h_flex()
            .flex_shrink_0()
            .gap_2()
            .px_2()
            .pb_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .w(px(220.))
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
            );

        let mut rows = v_flex().gap_1();
        for header in headers {
            rows = rows.child(
                h_flex()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .child(div().w(px(220.)).text_sm().child(header.name.clone()))
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(header.value.clone()),
                    ),
            );
        }

        let list = v_flex().gap_1().child(header_row).child(rows);

        v_flex()
            .size_full()
            .min_h_0()
            .child(list.flex_1().overflow_y_scrollbar())
            .into_any_element()
    }

    pub(super) fn render_response_panel(&self, cx: &Context<Self>) -> impl IntoElement + use<> {
        let (panel_tab, status, body, headers) = self
            .active_tab()
            .map(|tab| {
                (
                    tab.response_panel_tab,
                    tab.response_status.clone(),
                    tab.response_body.clone(),
                    tab.response_headers.clone(),
                )
            })
            .unwrap_or((ResponsePanelTab::Body, None, String::new(), Vec::new()));

        let status = status
            .unwrap_or_else(|| "Response will appear here after sending a request".into());

        let content: AnyElement = match panel_tab {
            ResponsePanelTab::Body => {
                if body.is_empty() {
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(cx.theme().muted_foreground)
                        .child("Response body will appear here after sending a request")
                        .into_any_element()
                } else {
                    div()
                        .size_full()
                        .min_h_0()
                        .child(
                            markdown(format!("```\n{body}\n```"))
                                .selectable(true)
                                .scrollable(true),
                        )
                        .into_any_element()
                }
            }
            ResponsePanelTab::Headers => self.render_response_headers(&headers, cx),
        };

        v_flex()
            .gap_2()
            .p_3()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .flex_shrink_0()
                    .items_center()
                    .child(div().text_sm().child("Response"))
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(status),
                    ),
            )
            .child(
                TabBar::new("response-panel-tabs")
                    .flex_shrink_0()
                    .underline()
                    .selected_index(match panel_tab {
                        ResponsePanelTab::Body => 0,
                        ResponsePanelTab::Headers => 1,
                    })
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        if let Some(tab) = this.active_tab_mut() {
                            tab.response_panel_tab = if *index == 0 {
                                ResponsePanelTab::Body
                            } else {
                                ResponsePanelTab::Headers
                            };
                            cx.notify();
                        }
                    }))
                    .child("Body")
                    .child("Headers"),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(content),
            )
    }
}
