use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    select::Select,
    ActiveTheme as _, Disableable as _,
};

use super::ApiHelperApp;

impl ApiHelperApp {
    pub(super) fn render_url_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let loading = self.active_tab().is_some_and(|tab| tab.loading);

        h_flex()
            .w_full()
            .gap_2()
            .p_2()
            .flex_shrink_0()
            .items_center()
            .bg(cx.theme().muted)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .w(px(110.))
                    .flex_shrink_0()
                    .child(Select::new(&self.method_select)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&self.url_input)),
            )
            .child(
                Button::new("send")
                    .primary()
                    .flex_shrink_0()
                    .label(if loading { "Sending…" } else { "Send" })
                    .disabled(loading)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.send_request(window, cx);
                    })),
            )
    }
}
