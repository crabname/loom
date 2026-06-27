mod fields;
mod request;
mod response;
mod sidebar;
mod tab_bar;
mod url_bar;

pub(crate) use fields::{
    build_multipart_row_inputs, build_row_inputs, flush_multipart_rows, flush_rows, FieldTable,
    MultipartRowInputs, RowInputs,
};
pub(crate) use sidebar::{
    build_collection_tree_items, parse_collection_tree_id, request_tree_id,
};

use gpui::*;
use gpui_component::{
    resizable::{h_resizable, resizable_panel, v_resizable},
    v_flex,
};

use super::ApiHelperApp;

impl Render for ApiHelperApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(
                h_resizable("main-layout")
                    .child(
                        resizable_panel()
                            .size(px(260.))
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
            )
    }
}
