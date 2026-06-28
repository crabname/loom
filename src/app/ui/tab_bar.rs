use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    tab::{Tab as TabChip, TabBar},
    ActiveTheme as _, IconName, Sizable as _,
};

use super::LoomApp;

impl LoomApp {
    pub(super) fn render_tab_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut bar = TabBar::new("request-tabs")
            .selected_index(self.active_tab)
            .on_click(cx.listener(|this, index: &usize, window, cx| {
                this.switch_tab(*index, window, cx);
            }));

        for (index, tab) in self.tabs.iter().enumerate() {
            let title = format!("{} {}", tab.method.as_str(), tab.title);
            bar = bar.child(
                TabChip::new()
                    .label(title)
                    .suffix(
                        Button::new(("close", index))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.close_tab(index, window, cx);
                            })),
                    ),
            );
        }

        v_flex()
            .w_full()
            .flex_shrink_0()
            .bg(cx.theme().sidebar)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .items_center()
                    .justify_end()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(self.render_environment_bar(cx)),
            )
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .p_2()
                    .items_center()
                    .child(div().flex_1().min_w_0().child(bar))
                    .child(
                        Button::new("new-tab")
                            .ghost()
                            .icon(IconName::Plus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_empty_tab(window, cx);
                            })),
                    ),
            )
    }
}
