use gpui::*;
use gpui_component::{
    h_flex, v_flex,
    input::Input,
    scroll::ScrollableElement as _,
    tab::TabBar,
    text::html,
    tooltip::Tooltip,
    ActiveTheme as _,
};

use crate::app::tab::ResponsePanelTab;
use crate::domain::{
    format_binary_body_message, format_response_size, is_html_content, response_content_type,
    KeyValueField, RequestTimingBreakdown, ResponseBody, ResponseBodyView,
};
use crate::scripting::{ScriptConsoleEntry, ScriptConsoleLevel, TestResultEntry, TestStatus};

use super::LoomApp;

fn console_level_color(level: ScriptConsoleLevel, cx: &App) -> Hsla {
    match level {
        ScriptConsoleLevel::Error => cx.theme().red,
        ScriptConsoleLevel::Warn => cx.theme().yellow,
        ScriptConsoleLevel::Info => cx.theme().blue,
        ScriptConsoleLevel::Debug => cx.theme().muted_foreground,
        ScriptConsoleLevel::Log => cx.theme().foreground,
    }
}

fn render_script_console(entries: &[ScriptConsoleEntry], cx: &App) -> AnyElement {
    if entries.is_empty() {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(cx.theme().muted_foreground)
            .child("Script console output will appear here after running pre/post scripts")
            .into_any_element();
    }

    let mut list = v_flex().gap_1();
    for entry in entries {
        list = list.child(
            h_flex()
                .gap_2()
                .px_2()
                .py_1()
                .rounded(px(4.))
                .hover(|style| style.bg(cx.theme().muted))
                .child(
                    div()
                        .w(px(44.))
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(console_level_color(entry.level, cx))
                        .child(entry.level.label()),
                )
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .child(entry.message.clone()),
                ),
        );
    }

    v_flex()
        .size_full()
        .min_h_0()
        .child(list.flex_1().overflow_y_scrollbar())
        .into_any_element()
}

fn render_test_results(entries: &[TestResultEntry], cx: &App) -> AnyElement {
    if entries.is_empty() {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(cx.theme().muted_foreground)
            .child("Test results will appear here after sending a request with tests")
            .into_any_element();
    }

    let passed = entries
        .iter()
        .filter(|entry| entry.status == TestStatus::Pass)
        .count();
    let failed = entries.len() - passed;

    let mut list = v_flex()
        .gap_2()
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(format!("{passed} passed, {failed} failed")),
        );

    for entry in entries {
        let (icon, color) = match entry.status {
            TestStatus::Pass => ("✓", cx.theme().green),
            TestStatus::Fail => ("✗", cx.theme().red),
        };

        let mut row = v_flex()
            .gap_1()
            .px_2()
            .py_1()
            .rounded(px(4.))
            .child(
                h_flex()
                    .gap_2()
                    .items_start()
                    .child(
                        div()
                            .text_sm()
                            .text_color(color)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(icon),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .child(entry.description.clone()),
                    ),
            );

        if let Some(error) = &entry.error {
            row = row.child(
                div()
                    .pl_6()
                    .text_xs()
                    .text_color(cx.theme().red)
                    .child(error.clone()),
            );
        }

        list = list.child(row);
    }

    v_flex()
        .size_full()
        .min_h_0()
        .child(list.flex_1().overflow_y_scrollbar())
        .into_any_element()
}

struct ResponseStatusInfo<'a> {
    loading: bool,
    http_status: Option<u16>,
    status_text: Option<&'a str>,
    elapsed_ms: Option<u128>,
    timing: Option<RequestTimingBreakdown>,
    size_bytes: Option<usize>,
    error: Option<&'a str>,
}

fn timing_span_color(index: usize, cx: &App) -> Hsla {
    match index {
        0 => cx.theme().chart_1,
        1 => cx.theme().chart_2,
        2 => cx.theme().chart_3,
        3 => cx.theme().chart_4,
        4 => cx.theme().chart_5,
        _ => cx.theme().accent,
    }
}

fn render_timing_waterfall(timing: RequestTimingBreakdown, cx: &App) -> AnyElement {
    let spans = timing.visible_spans();
    let total = timing.total_ms().max(1) as f32;
    const BAR_WIDTH: f32 = 260.0;

    let mut bar = h_flex()
        .h(px(10.))
        .w(px(BAR_WIDTH))
        .rounded(px(3.))
        .overflow_hidden()
        .bg(cx.theme().muted);

    for (index, (_, ms)) in spans.iter().enumerate() {
        let width = ((*ms as f32 / total) * BAR_WIDTH).max(if *ms > 0 { 2.0 } else { 0.0 });
        bar = bar.child(
            div()
                .h_full()
                .w(px(width))
                .bg(timing_span_color(index, cx)),
        );
    }

    let mut legend = v_flex().gap_1();
    for (index, (label, ms)) in spans.into_iter().enumerate() {
        legend = legend.child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .w(px(8.))
                        .h(px(8.))
                        .rounded(px(2.))
                        .bg(timing_span_color(index, cx)),
                )
                .child(div().text_xs().flex_1().child(label))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{ms} ms")),
                ),
        );
    }

    v_flex()
        .gap_2()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Request timing"),
        )
        .child(bar)
        .child(legend)
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!("Total {} ms", timing.total_ms())),
        )
        .into_any_element()
}

fn response_status_color(status: u16, cx: &Context<LoomApp>) -> Hsla {
    match status {
        200..=299 => cx.theme().green,
        300..=399 => cx.theme().blue,
        400..=499 => cx.theme().yellow,
        _ => cx.theme().red,
    }
}

impl LoomApp {
    fn render_response_status(
        &self,
        status: &ResponseStatusInfo<'_>,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        if status.loading {
            return div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Sending…")
                .into_any_element();
        }

        if let Some(error) = status.error {
            return div()
                .text_xs()
                .text_color(cx.theme().red)
                .child(format!("Error · {error}"))
                .into_any_element();
        }

        if let Some(http_status) = status.http_status {
            let color = response_status_color(http_status, cx);
            let status_text = status.status_text.unwrap_or("Unknown");

            let mut row = h_flex().gap_1().items_center();
            row = row.child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color)
                    .child(format!("{http_status}")),
            );
            row = row.child(
                div()
                    .text_xs()
                    .text_color(color)
                    .child(status_text.to_string()),
            );

            if let Some(ms) = status.elapsed_ms {
                let elapsed = div()
                    .id("response-elapsed")
                    .cursor_pointer()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("· {ms} ms"));

                row = row.child(if let Some(timing) = status.timing {
                    elapsed.tooltip(move |window, cx| {
                        Tooltip::element(move |_, cx| render_timing_waterfall(timing, cx))
                            .build(window, cx)
                    })
                } else {
                    elapsed
                });
            }

            if let Some(size) = status.size_bytes {
                row = row.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("· {}", format_response_size(size))),
                );
            }

            return row.into_any_element();
        }

        div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child("Response will appear here after sending a request")
            .into_any_element()
    }

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
        cx: &Context<Self>,
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

        div()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .child(Input::new(&self.response_body_input).h_full())
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
        let (
            panel_tab,
            body_view,
            loading,
            http_status,
            status_text,
            elapsed_ms,
            timing,
            size_bytes,
            error,
            body,
            headers,
            script_console,
            test_results,
        ) = self
            .active_tab()
            .map(|tab| {
                (
                    tab.response_panel_tab,
                    tab.response_body_view,
                    tab.loading,
                    tab.response_http_status,
                    tab.response_status_text.clone(),
                    tab.response_elapsed_ms,
                    tab.response_timing,
                    tab.response_size_bytes,
                    tab.response_error.clone(),
                    tab.response_body.clone(),
                    tab.response_headers.clone(),
                    tab.script_console.clone(),
                    tab.test_results.clone(),
                )
            })
            .unwrap_or((
                ResponsePanelTab::Body,
                ResponseBodyView::Raw,
                false,
                None,
                None,
                None,
                None,
                None,
                None,
                ResponseBody::empty(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ));

        let html_preview_available = matches!(&body, ResponseBody::Text(text) if {
            let content_type = response_content_type(&headers);
            is_html_content(content_type.as_deref(), text)
        });

        let status = ResponseStatusInfo {
            loading,
            http_status,
            status_text: status_text.as_deref(),
            elapsed_ms,
            timing,
            size_bytes,
            error: error.as_deref(),
        };

        let content: AnyElement = match panel_tab {
            ResponsePanelTab::Body => {
                self.render_response_body(&body, &headers, body_view, cx)
            }
            ResponsePanelTab::Headers => self.render_response_headers(&headers, cx),
            ResponsePanelTab::Console => render_script_console(&script_console, cx),
            ResponsePanelTab::Tests => render_test_results(&test_results, cx),
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
                    .child(self.render_response_status(&status, cx)),
            )
            .child(
                TabBar::new("response-panel-tabs")
                    .flex_shrink_0()
                    .underline()
                    .selected_index(match panel_tab {
                        ResponsePanelTab::Body => 0,
                        ResponsePanelTab::Headers => 1,
                        ResponsePanelTab::Console => 2,
                        ResponsePanelTab::Tests => 3,
                    })
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        if let Some(tab) = this.active_tab_mut() {
                            tab.response_panel_tab = match *index {
                                0 => ResponsePanelTab::Body,
                                1 => ResponsePanelTab::Headers,
                                2 => ResponsePanelTab::Console,
                                _ => ResponsePanelTab::Tests,
                            };
                            cx.notify();
                        }
                    }))
                    .child("Body")
                    .child("Headers")
                    .child("Console")
                    .child("Tests"),
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
