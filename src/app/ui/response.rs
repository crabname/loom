use gpui::*;
use gpui_component::{
    h_flex, v_flex,
    scroll::ScrollableElement as _,
    tab::TabBar,
    text::{html, markdown},
    ActiveTheme as _,
};

use crate::app::tab::ResponsePanelTab;
use crate::domain::{
    format_binary_body_message, is_html_content, response_body_language, response_content_type,
    KeyValueField, ResponseBody, ResponseBodyView,
};

use super::ApiHelperApp;

fn response_body_markdown(body: &str, language: Option<&str>) -> String {
    match language {
        Some(lang) => format!("```{lang}\n{body}\n```"),
        None => format!("```\n{body}\n```"),
    }
}

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

    fn render_binary_body(
        &self,
        size: usize,
        content_type: Option<&str>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let message = format_binary_body_message(size, content_type);

        v_flex()
            .size_full()
            .gap_2()
            .justify_center()
            .items_center()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Binary data"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(message),
            )
            .into_any_element()
    }

    fn render_text_body(
        &self,
        body: &str,
        headers: &[KeyValueField],
        body_view: ResponseBodyView,
        _cx: &Context<Self>,
    ) -> AnyElement {
        let content_type = response_content_type(headers);
        let is_html = is_html_content(content_type.as_deref(), body);

        if is_html && body_view == ResponseBodyView::Preview {
            return div()
                .size_full()
                .min_h_0()
                .child(
                    html(body)
                        .selectable(true)
                        .scrollable(true),
                )
                .into_any_element();
        }

        let language = response_body_language(content_type.as_deref(), body);
        div()
            .size_full()
            .min_h_0()
            .child(
                markdown(response_body_markdown(body, language))
                    .selectable(true)
                    .scrollable(true),
            )
            .into_any_element()
    }

    fn render_response_body(
        &self,
        body: &ResponseBody,
        headers: &[KeyValueField],
        body_view: ResponseBodyView,
        cx: &Context<Self>,
    ) -> AnyElement {
        if body.is_empty() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child("Response body will appear here after sending a request")
                .into_any_element();
        }

        match body {
            ResponseBody::Binary { size, content_type } => {
                self.render_binary_body(*size, content_type.as_deref(), cx)
            }
            ResponseBody::Text(text) => self.render_text_body(text, headers, body_view, cx),
        }
    }

    pub(super) fn render_response_panel(&self, cx: &Context<Self>) -> impl IntoElement + use<> {
        let (panel_tab, body_view, status, body, headers) = self
            .active_tab()
            .map(|tab| {
                (
                    tab.response_panel_tab,
                    tab.response_body_view,
                    tab.response_status.clone(),
                    tab.response_body.clone(),
                    tab.response_headers.clone(),
                )
            })
            .unwrap_or((
                ResponsePanelTab::Body,
                ResponseBodyView::Raw,
                None,
                ResponseBody::empty(),
                Vec::new(),
            ));

        let status = status
            .unwrap_or_else(|| "Response will appear here after sending a request".into());

        let html_preview_available = matches!(&body, ResponseBody::Text(text) if {
            let content_type = response_content_type(&headers);
            is_html_content(content_type.as_deref(), text)
        });

        let content: AnyElement = match panel_tab {
            ResponsePanelTab::Body => {
                self.render_response_body(&body, &headers, body_view, cx)
            }
            ResponsePanelTab::Headers => self.render_response_headers(&headers, cx),
        };

        let mut panel = v_flex()
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
            );

        if panel_tab == ResponsePanelTab::Body && html_preview_available {
            panel = panel.child(
                TabBar::new("response-body-view-tabs")
                    .flex_shrink_0()
                    .selected_index(match body_view {
                        ResponseBodyView::Raw => 0,
                        ResponseBodyView::Preview => 1,
                    })
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        if let Some(tab) = this.active_tab_mut() {
                            tab.response_body_view = if *index == 0 {
                                ResponseBodyView::Raw
                            } else {
                                ResponseBodyView::Preview
                            };
                            cx.notify();
                        }
                    }))
                    .child("Raw")
                    .child("Preview"),
            );
        }

        panel
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(content),
            )
    }
}
