use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    h_flex, v_flex,
    list::ListItem,
    tree::{tree, TreeItem},
    ActiveTheme as _, IconName, StyledExt as _,
};

use crate::domain::{Collection, HttpMethod, RequestProtocol};
use crate::app::tab::TabSource;

use super::ApiHelperApp;

fn protocol_icon(protocol: RequestProtocol) -> IconName {
    match protocol {
        RequestProtocol::Http => IconName::Globe,
        RequestProtocol::Grpc => IconName::Network,
    }
}

fn method_color(method: HttpMethod) -> gpui::Hsla {
    match method {
        HttpMethod::Get => gpui::rgb(0x609eff).into(),
        HttpMethod::Post => gpui::rgb(0x4acc8f).into(),
        HttpMethod::Put => gpui::rgb(0xfc9f30).into(),
        HttpMethod::Patch => gpui::rgb(0x4fe3c2).into(),
        HttpMethod::Delete => gpui::rgb(0xfa3d3d).into(),
    }
}

pub(crate) fn collection_tree_id(collection: usize) -> String {
    format!("collection:{collection}")
}

pub(crate) fn request_tree_id(collection: usize, request: usize) -> SharedString {
    format!("request:{collection}:{request}").into()
}

pub(crate) fn parse_collection_tree_id(id: &SharedString) -> Option<usize> {
    id.strip_prefix("collection:")?.parse().ok()
}

fn parse_request_tree_id(id: &SharedString) -> Option<(usize, usize)> {
    let rest = id.strip_prefix("request:")?;
    let (collection, request) = rest.split_once(':')?;
    Some((collection.parse().ok()?, request.parse().ok()?))
}

pub(crate) fn build_collection_tree_items(collections: &[Collection]) -> Vec<TreeItem> {
    collections
        .iter()
        .enumerate()
        .map(|(collection_index, collection)| {
            TreeItem::new(collection_tree_id(collection_index), collection.name.clone())
                .expanded(collection.expanded)
                .children(collection.requests.iter().enumerate().map(
                    |(request_index, request)| {
                        TreeItem::new(
                            request_tree_id(collection_index, request_index),
                            request.name.clone(),
                        )
                    },
                ))
        })
        .collect()
}

impl ApiHelperApp {
    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();

        div()
            .w(px(260.))
            .h_full()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                v_flex()
                    .gap_1()
                    .p_2()
                    .size_full()
                    .min_h_0()
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_sm()
                            .font_semibold()
                            .child("Collections"),
                    )
                    .child(
                        tree(&self.collections_tree, move |ix, entry, selected, _window, cx| {
                            view.update(cx, |this, cx| {
                                let item = entry.item();
                                let is_folder = entry.is_folder();
                                let icon = if is_folder {
                                    if entry.is_expanded() {
                                        IconName::FolderOpen
                                    } else {
                                        IconName::Folder
                                    }
                                } else if let Some((collection_index, request_index)) =
                                    parse_request_tree_id(&item.id)
                                {
                                    this.collections
                                        .get(collection_index)
                                        .and_then(|collection| {
                                            collection.requests.get(request_index)
                                        })
                                        .map(|request| protocol_icon(request.protocol))
                                        .unwrap_or(IconName::File)
                                } else {
                                    IconName::File
                                };

                                let is_active = !is_folder
                                    && parse_request_tree_id(&item.id).is_some_and(|source| {
                                        this.active_tab().and_then(|tab| tab.source)
                                            == Some(TabSource {
                                                collection: source.0,
                                                request: source.1,
                                            })
                                    });

                                let label = if is_folder {
                                    h_flex()
                                        .gap_2()
                                        .child(icon)
                                        .child(item.label.clone())
                                        .into_any_element()
                                } else if let Some((collection_index, request_index)) =
                                    parse_request_tree_id(&item.id)
                                {
                                    let request = this
                                        .collections
                                        .get(collection_index)
                                        .and_then(|collection| {
                                            collection.requests.get(request_index)
                                        });

                                    if let Some(request) = request {
                                        h_flex()
                                            .gap_2()
                                            .child(icon)
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(method_color(request.method))
                                                    .child(request.method.as_str()),
                                            )
                                            .child(div().text_sm().child(request.name.clone()))
                                            .into_any_element()
                                    } else {
                                        h_flex()
                                            .gap_2()
                                            .child(icon)
                                            .child(item.label.clone())
                                            .into_any_element()
                                    }
                                } else {
                                    h_flex()
                                        .gap_2()
                                        .child(icon)
                                        .child(item.label.clone())
                                        .into_any_element()
                                };

                                ListItem::new(ix)
                                    .w_full()
                                    .selected(selected || is_active)
                                    .pl(px(16.) * entry.depth() + px(12.))
                                    .child(label)
                                    .when(!is_folder, |list_item| {
                                        list_item.on_click(cx.listener({
                                            let item_id = item.id.clone();
                                            move |this, _, window, cx| {
                                                if let Some((collection, request)) =
                                                    parse_request_tree_id(&item_id)
                                                {
                                                    this.open_request_tab(
                                                        collection,
                                                        request,
                                                        window,
                                                        cx,
                                                    );
                                                }
                                            }
                                        }))
                                    })
                            })
                        })
                        .flex_1()
                        .min_h_0(),
                    ),
            )
    }
}
