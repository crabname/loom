use std::path::PathBuf;

use gpui::*;
use gpui_component::{notification::Notification, WindowExt as _};

use crate::import::{export_collection, import_collection, ExportTarget};
use crate::storage::default_collection_paths;

use super::LoomApp;

impl LoomApp {
    pub(super) fn on_import_collection(
        &mut self,
        _: &super::ImportCollection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.prompt_import_collection(window, cx);
    }

    pub(super) fn prompt_import_collection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: Some("Select a Postman collection (.json) or OpenCollection folder/file".into()),
        });

        let view = cx.entity();
        cx.spawn_in(window, async move |_, window| {
            let path = match paths.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let Some(path) = path else {
                return;
            };

            let result = import_collection(&path);
            window
                .update(|window, cx| {
                    view.update(cx, |app, cx| {
                        app.finish_import_collection(path, result, window, cx);
                    })
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn prompt_export_collection(
        &mut self,
        collection_index: usize,
        target: ExportTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(collection) = self.active_collections().get(collection_index) else {
            return;
        };

        let collection = collection.clone();
        let default_name = match target {
            ExportTarget::PostmanFile => format!("{}.postman_collection.json", slug_file_name(&collection.name)),
            ExportTarget::OpenCollectionFolder => slug_file_name(&collection.name),
        };

        let view = cx.entity();
        match target {
            ExportTarget::OpenCollectionFolder => {
                let paths = cx.prompt_for_paths(PathPromptOptions {
                    files: false,
                    directories: true,
                    multiple: false,
                    prompt: Some("Select a folder for the OpenCollection export".into()),
                });
                cx.spawn_in(window, async move |_, window| {
                    let parent = match paths.await {
                        Ok(Ok(Some(paths))) => paths.into_iter().next(),
                        _ => None,
                    };
                    let Some(parent) = parent else {
                        return;
                    };

                    let export_path = parent.join(&default_name);
                    let result = export_collection(&collection, target, &export_path);
                    window
                        .update(|window, cx| {
                            view.update(cx, |app, cx| {
                                app.finish_export_collection(export_path, target, result, window, cx);
                            })
                        })
                        .ok();
                })
                .detach();
            }
            ExportTarget::PostmanFile => {
                let home = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
                let path = cx.prompt_for_new_path(&home, Some(&default_name));
                cx.spawn_in(window, async move |_, window| {
                    let Some(export_path) = path
                        .await
                        .ok()
                        .and_then(|result| result.ok())
                        .flatten()
                    else {
                        return;
                    };

                    let result = export_collection(&collection, target, &export_path);
                    window
                        .update(|window, cx| {
                            view.update(cx, |app, cx| {
                                app.finish_export_collection(export_path, target, result, window, cx);
                            })
                        })
                        .ok();
                })
                .detach();
            }
        }
    }

    fn finish_import_collection(
        &mut self,
        source_path: PathBuf,
        result: Result<crate::import::ImportResult, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(imported) => {
                self.integrate_imported_collection(imported.collection);
                self.refresh_collections_tree(cx);
                self.refresh_environment_select(window, cx);
                self.autosave_active_workspace(cx);

                if imported.warnings.is_empty() {
                    window.push_notification(
                        Notification::success("Collection imported").message(format!(
                            "Imported from {}",
                            source_path.display()
                        )),
                        cx,
                    );
                } else {
                    let preview = imported
                        .warnings
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                    window.push_notification(
                        Notification::warning("Collection imported with warnings")
                            .message(preview),
                        cx,
                    );
                }
            }
            Err(error) => {
                window.push_notification(
                    Notification::error("Failed to import collection").message(error),
                    cx,
                );
            }
        }
        cx.notify();
    }

    fn finish_export_collection(
        &mut self,
        export_path: PathBuf,
        target: ExportTarget,
        result: Result<Vec<String>, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(warnings) => {
                let format_label = match target {
                    ExportTarget::OpenCollectionFolder => "OpenCollection",
                    ExportTarget::PostmanFile => "Postman",
                };
                if warnings.is_empty() {
                    window.push_notification(
                        Notification::success(format!("Collection exported as {format_label}"))
                            .message(export_path.display().to_string()),
                        cx,
                    );
                } else {
                    let preview = warnings
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                    window.push_notification(
                        Notification::warning(format!(
                            "Collection exported as {format_label} with warnings"
                        ))
                        .message(preview),
                        cx,
                    );
                }
            }
            Err(error) => {
                window.push_notification(
                    Notification::error("Failed to export collection").message(error),
                    cx,
                );
            }
        }
        cx.notify();
    }

    fn integrate_imported_collection(&mut self, collection: crate::domain::Collection) {
        self.active_collections_mut().push(collection);
        let collection_path = default_collection_paths(&self.workspaces[self.active_workspace])
            .last()
            .expect("collection was just added")
            .clone();
        self.workspace_collection_paths[self.active_workspace].push(collection_path);
    }
}

fn slug_file_name(name: &str) -> String {
    let slug = name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase();
    if slug.is_empty() {
        "collection".into()
    } else {
        slug
    }
}
