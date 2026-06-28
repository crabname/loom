mod curl;
mod environment;
mod fields;
mod rename;
mod request;
mod response;
mod sidebar;
mod tab_bar;
mod url_bar;

pub(crate) use fields::{
    build_multipart_row_inputs, build_query_row_inputs, build_row_inputs, flush_multipart_rows,
    flush_rows, FieldTable, MultipartRowInputs, RowInputs,
};
pub(crate) use environment::{build_variable_row_inputs, flush_environment_variables};
pub(crate) use sidebar::{
    build_collection_tree_items, parse_collection_tree_id, parse_folder_tree_id, request_tree_id,
};

use gpui::*;
use gpui_component::{
    resizable::{h_resizable, resizable_panel, v_resizable},
    v_flex, Root, TitleBar,
};

use super::LoomApp;

impl Render for LoomApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("loom-root")
            .size_full()
            .flex()
            .flex_col()
            .key_context("LoomApp")
            .on_action(cx.listener(Self::on_open_workspace))
            .child(
                TitleBar::new()
                    .child(div().flex().items_center().child(self.app_menu_bar.clone())),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(
                        h_resizable("main-layout")
                            .child(
                                resizable_panel()
                                    .size(px(260.))
                                    .flex_none()
                                    .child(self.render_sidebar(cx)),
                            )
                            .child(
                                resizable_panel().child(
                                    v_flex()
                                        .size_full()
                                        .w_full()
                                        .min_h_0()
                                        .min_w_0()
                                        .child(self.render_tab_bar(cx))
                                        .child(self.render_url_bar(cx))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_h_0()
                                                .child(
                                                    v_resizable("editor-split")
                                                        .child(
                                                            resizable_panel()
                                                                .child(self.render_request_panel(cx)),
                                                        )
                                                        .child(
                                                            resizable_panel()
                                                                .child(self.render_response_panel(cx)),
                                                        ),
                                                ),
                                        ),
                                ),
                            ),
                    ),
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
