use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    list::ListItem,
    menu::PopupMenuItem,
    select::Select,
    tree::{tree, TreeItem},
    ActiveTheme as _, IconName, Sizable as _, StyledExt as _,
};

use crate::domain::{Collection, HttpMethod, RequestProtocol};
use crate::app::tab::TabSource;

use super::LoomApp;
use super::rename::RenameTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RequestTreeId {
    pub collection: usize,
    pub folder: Option<usize>,
    pub request: usize,
}

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

pub(crate) fn folder_tree_id(collection: usize, folder: usize) -> String {
    format!("folder:{collection}:{folder}")
}

pub(crate) fn request_tree_id(
    collection: usize,
    folder: Option<usize>,
    request: usize,
) -> SharedString {
    match folder {
        None => format!("request:{collection}:r:{request}").into(),
        Some(folder) => format!("request:{collection}:f:{folder}:{request}").into(),
    }
}

pub(crate) fn parse_collection_tree_id(id: &SharedString) -> Option<usize> {
    id.strip_prefix("collection:")?.parse().ok()
}

pub(crate) fn parse_folder_tree_id(id: &SharedString) -> Option<(usize, usize)> {
    let rest = id.strip_prefix("folder:")?;
    let (collection, folder) = rest.split_once(':')?;
    Some((collection.parse().ok()?, folder.parse().ok()?))
}

pub(crate) fn parse_request_tree_id(id: &SharedString) -> Option<RequestTreeId> {
    let rest = id.strip_prefix("request:")?;
    let (collection, rest) = rest.split_once(':')?;
    let collection = collection.parse().ok()?;

    if let Some(request) = rest.strip_prefix("r:") {
        return Some(RequestTreeId {
            collection,
            folder: None,
            request: request.parse().ok()?,
        });
    }

    let rest = rest.strip_prefix("f:")?;
    let (folder, request) = rest.split_once(':')?;
    Some(RequestTreeId {
        collection,
        folder: Some(folder.parse().ok()?),
        request: request.parse().ok()?,
    })
}

fn build_folder_tree_items(collection_index: usize, folder_index: usize, folder: &crate::domain::CollectionFolder) -> TreeItem {
    TreeItem::new(
        folder_tree_id(collection_index, folder_index),
        folder.name.clone(),
    )
    .expanded(folder.expanded)
    .children(folder.requests.iter().enumerate().map(|(request_index, request)| {
        TreeItem::new(
            request_tree_id(collection_index, Some(folder_index), request_index),
            request.name.clone(),
        )
    }))
}

fn build_collection_children(collection_index: usize, collection: &Collection) -> Vec<TreeItem> {
    let mut children = collection
        .folders
        .iter()
        .enumerate()
        .map(|(folder_index, folder)| {
            build_folder_tree_items(collection_index, folder_index, folder)
        })
        .collect::<Vec<_>>();

    children.extend(collection.requests.iter().enumerate().map(|(request_index, request)| {
        TreeItem::new(
            request_tree_id(collection_index, None, request_index),
            request.name.clone(),
        )
    }));

    children
}

pub(crate) fn build_collection_tree_items(collections: &[Collection]) -> Vec<TreeItem> {
    collections
        .iter()
        .enumerate()
        .map(|(collection_index, collection)| {
            TreeItem::new(collection_tree_id(collection_index), collection.name.clone())
                .expanded(collection.expanded)
                .children(build_collection_children(collection_index, collection))
        })
        .collect()
}

impl LoomApp {
    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let tree_view = view.clone();

        div()
            .w_full()
            .h_full()
            .min_w_0()
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
                        h_flex()
                            .flex_shrink_0()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .child("Workspace"),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("workspace-variables")
                                            .ghost()
                                            .xsmall()
                                            .label("Variables")
                                            .tooltip("Workspace variables")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.open_workspace_variables_dialog(window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("open-settings")
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::Settings)
                                            .tooltip("Application settings")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.open_settings_dialog(window, cx);
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .child(Select::new(&self.workspace_select)),
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .child("Collections"),
                            )
                            .child(
                                Button::new("add-collection")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Plus)
                                    .tooltip("New collection")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.add_collection(window, cx);
                                    })),
                            ),
                    )
                    .child(
                        tree(&self.collections_tree, {
                            let view = tree_view.clone();
                            move |ix, entry, selected, _window, cx| {
                            view.update(cx, |this, cx| {
                                let item = entry.item();
                                let is_folder = entry.is_folder();
                                let collection_index = parse_collection_tree_id(&item.id);
                                let folder_location = parse_folder_tree_id(&item.id);
                                let request_location = parse_request_tree_id(&item.id);

                                let icon = if collection_index.is_some() {
                                    IconName::Inbox
                                } else if folder_location.is_some() {
                                    if entry.is_expanded() {
                                        IconName::FolderOpen
                                    } else {
                                        IconName::Folder
                                    }
                                } else if let Some(location) = request_location {
                                    this.active_collections()
                                        .get(location.collection)
                                        .and_then(|collection| {
                                            collection.request_ref(location.folder, location.request)
                                        })
                                        .map(|request| protocol_icon(request.protocol))
                                        .unwrap_or(IconName::File)
                                } else {
                                    IconName::File
                                };

                                let is_active = request_location.is_some_and(|location| {
                                    this.active_tab().and_then(|tab| tab.source)
                                        == Some(TabSource {
                                            workspace: this.active_workspace,
                                            collection: location.collection,
                                            folder: location.folder,
                                            request: location.request,
                                        })
                                });

                                let label = if let Some(collection_index) = collection_index {
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_between()
                                        .gap_2()
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .child(icon)
                                                .child(item.label.clone()),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_0p5()
                                                .child(
                                                    Button::new(("add-folder", collection_index))
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::Folder)
                                                        .tooltip("New folder")
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                cx.stop_propagation();
                                                                this.add_folder_to_collection(
                                                                    collection_index,
                                                                    window,
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                )
                                                .child(
                                                    Button::new(("add-request", collection_index))
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::Plus)
                                                        .tooltip("New request")
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                cx.stop_propagation();
                                                                this.add_request_to_collection(
                                                                    collection_index,
                                                                    None,
                                                                    window,
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                )
                                                .child(
                                                    Button::new(("delete-collection", collection_index))
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::Delete)
                                                        .tooltip("Delete collection")
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                cx.stop_propagation();
                                                                this.delete_collection(
                                                                    collection_index,
                                                                    window,
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                ),
                                        )
                                        .into_any_element()
                                } else if let Some((collection_index, folder_index)) = folder_location {
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_between()
                                        .gap_2()
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .child(icon)
                                                .child(item.label.clone()),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_0p5()
                                                .child(
                                                    Button::new(format!(
                                                        "add-request-folder:{collection_index}:{folder_index}"
                                                    ))
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::Plus)
                                                        .tooltip("New request")
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                cx.stop_propagation();
                                                                this.add_request_to_collection(
                                                                    collection_index,
                                                                    Some(folder_index),
                                                                    window,
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                )
                                                .child(
                                                    Button::new(format!(
                                                        "delete-folder:{collection_index}:{folder_index}"
                                                    ))
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::Delete)
                                                        .tooltip("Delete folder")
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                cx.stop_propagation();
                                                                this.delete_folder_from_collection(
                                                                    collection_index,
                                                                    folder_index,
                                                                    window,
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                ),
                                        )
                                        .into_any_element()
                                } else if let Some(location) = request_location {
                                    let request = this
                                        .active_collections()
                                        .get(location.collection)
                                        .and_then(|collection| {
                                            collection.request_ref(location.folder, location.request)
                                        });

                                    if let Some(request) = request {
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .justify_between()
                                            .gap_2()
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .child(icon)
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(method_color(request.method))
                                                            .child(request.method.as_str()),
                                                    )
                                                    .child(div().text_sm().child(request.name.clone())),
                                            )
                                            .child(
                                                Button::new(format!(
                                                    "delete-request:{}:{}:{}",
                                                    location.collection,
                                                    location.folder.map_or("r".into(), |folder| folder.to_string()),
                                                    location.request
                                                ))
                                                    .ghost()
                                                    .xsmall()
                                                    .icon(IconName::Delete)
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            cx.stop_propagation();
                                                            this.delete_request_from_collection(
                                                                location.collection,
                                                                location.folder,
                                                                location.request,
                                                                window,
                                                                cx,
                                                            );
                                                        },
                                                    )),
                                            )
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
                                                if let Some(location) = parse_request_tree_id(&item_id) {
                                                    this.open_request_tab(
                                                        location.collection,
                                                        location.folder,
                                                        location.request,
                                                        window,
                                                        cx,
                                                    );
                                                }
                                            }
                                        }))
                                    })
                            })
                        }
                        })
                        .context_menu({
                            let view = tree_view;
                            move |_ix, entry, menu, _window, _cx| {
                                if let Some(collection_index) =
                                    parse_collection_tree_id(&entry.item().id)
                                {
                                    let view_for_rename = view.clone();
                                    let view_for_folder = view.clone();
                                    let view_for_new = view.clone();
                                    let view_for_import = view.clone();
                                    let view_for_variables = view.clone();
                                    let view_for_delete = view.clone();
                                    return menu
                                        .item(
                                            PopupMenuItem::new("Rename")
                                                .icon(IconName::Replace)
                                                .on_click(move |_, window, cx| {
                                                    view_for_rename.update(cx, |this, cx| {
                                                        this.open_rename_dialog(
                                                            RenameTarget::Collection {
                                                                collection: collection_index,
                                                            },
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("Variables")
                                                .icon(IconName::Inbox)
                                                .on_click(move |_, window, cx| {
                                                    view_for_variables.update(cx, |this, cx| {
                                                        this.open_collection_variables_dialog(
                                                            collection_index,
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("New Folder")
                                                .icon(IconName::Folder)
                                                .on_click(move |_, window, cx| {
                                                    view_for_folder.update(cx, |this, cx| {
                                                        this.add_folder_to_collection(
                                                            collection_index,
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("New Request")
                                                .icon(IconName::Plus)
                                                .on_click(move |_, window, cx| {
                                                    view_for_new.update(cx, |this, cx| {
                                                        this.add_request_to_collection(
                                                            collection_index,
                                                            None,
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("Import cURL")
                                                .icon(IconName::ArrowDown)
                                                .on_click(move |_, window, cx| {
                                                    view_for_import.update(cx, |this, cx| {
                                                        this.open_import_curl_dialog(
                                                            super::curl::CurlImportTarget::Collection {
                                                                collection: collection_index,
                                                                folder: None,
                                                            },
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("Delete")
                                                .icon(IconName::Delete)
                                                .on_click(move |_, window, cx| {
                                                    view_for_delete.update(cx, |this, cx| {
                                                        this.delete_collection(
                                                            collection_index,
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        );
                                }

                                if let Some((collection_index, folder_index)) =
                                    parse_folder_tree_id(&entry.item().id)
                                {
                                    let view_for_rename = view.clone();
                                    let view_for_new = view.clone();
                                    let view_for_import = view.clone();
                                    let view_for_variables = view.clone();
                                    let view_for_delete = view.clone();
                                    return menu
                                        .item(
                                            PopupMenuItem::new("Rename")
                                                .icon(IconName::Replace)
                                                .on_click(move |_, window, cx| {
                                                    view_for_rename.update(cx, |this, cx| {
                                                        this.open_rename_dialog(
                                                            RenameTarget::Folder {
                                                                collection: collection_index,
                                                                folder: folder_index,
                                                            },
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("Variables")
                                                .icon(IconName::Folder)
                                                .on_click(move |_, window, cx| {
                                                    view_for_variables.update(cx, |this, cx| {
                                                        this.open_folder_variables_dialog(
                                                            collection_index,
                                                            folder_index,
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("New Request")
                                                .icon(IconName::Plus)
                                                .on_click(move |_, window, cx| {
                                                    view_for_new.update(cx, |this, cx| {
                                                        this.add_request_to_collection(
                                                            collection_index,
                                                            Some(folder_index),
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("Import cURL")
                                                .icon(IconName::ArrowDown)
                                                .on_click(move |_, window, cx| {
                                                    view_for_import.update(cx, |this, cx| {
                                                        this.open_import_curl_dialog(
                                                            super::curl::CurlImportTarget::Collection {
                                                                collection: collection_index,
                                                                folder: Some(folder_index),
                                                            },
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        )
                                        .item(
                                            PopupMenuItem::new("Delete")
                                                .icon(IconName::Delete)
                                                .on_click(move |_, window, cx| {
                                                    view_for_delete.update(cx, |this, cx| {
                                                        this.delete_folder_from_collection(
                                                            collection_index,
                                                            folder_index,
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        );
                                }

                                let Some(location) = parse_request_tree_id(&entry.item().id) else {
                                    return menu;
                                };

                                let view_for_rename = view.clone();
                                let view_for_export = view.clone();
                                let view_for_delete = view.clone();
                                menu.item(
                                    PopupMenuItem::new("Rename")
                                        .icon(IconName::Replace)
                                        .on_click(move |_, window, cx| {
                                            view_for_rename.update(cx, |this, cx| {
                                                this.open_rename_dialog(
                                                    RenameTarget::Request {
                                                        collection: location.collection,
                                                        folder: location.folder,
                                                        request: location.request,
                                                    },
                                                    window,
                                                    cx,
                                                );
                                            });
                                        }),
                                )
                                .item(
                                    PopupMenuItem::new("Export cURL")
                                        .icon(IconName::Copy)
                                        .on_click({
                                            move |_, window, cx| {
                                                let curl = view_for_export.read(cx)
                                                    .collection_request_as_curl(
                                                        location.collection,
                                                        location.folder,
                                                        location.request,
                                                    );
                                                view_for_export.update(cx, |this, cx| {
                                                    this.open_export_curl_dialog(curl, window, cx);
                                                });
                                            }
                                        }),
                                )
                                .item(
                                    PopupMenuItem::new("Delete")
                                        .icon(IconName::Delete)
                                        .on_click(move |_, window, cx| {
                                            view_for_delete.update(cx, |this, cx| {
                                                this.delete_request_from_collection(
                                                    location.collection,
                                                    location.folder,
                                                    location.request,
                                                    window,
                                                    cx,
                                                );
                                            });
                                        }),
                                )
                            }
                        })
                        .flex_1()
                        .min_h_0(),
                    ),
            )
    }
}
