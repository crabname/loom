use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use anyhow::Result;
use gpui::{App, Entity, Task, WeakEntity, Window};
use gpui_component::input::{HoverProvider, Rope, RopeExt};
use lsp_types::{Hover, HoverContents, MarkedString, Range as LspRange};

use crate::domain::{
    format_variable_hover, resolve_variable_source, variable_at_offset, VariableLayers,
    VariableResolveLabels,
};

use super::LoomApp;

pub struct VariableHoverProvider {
    app: RefCell<WeakEntity<LoomApp>>,
}

impl VariableHoverProvider {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            app: RefCell::new(WeakEntity::new_invalid()),
        })
    }

    pub fn attach(&self, app: &Entity<LoomApp>) {
        *self.app.borrow_mut() = app.downgrade();
    }
}

impl HoverProvider for VariableHoverProvider {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Option<Hover>>> {
        let Some(app) = self.app.borrow().upgrade() else {
            return Task::ready(Ok(None));
        };

        let text_value = text.to_string();
        let Some(span) = variable_at_offset(&text_value, offset) else {
            return Task::ready(Ok(None));
        };

        let Some((layers, labels)) = app.read(cx).variable_layers_for_active_tab() else {
            return Task::ready(Ok(None));
        };

        let runtime = app.read(cx).runtime_vars.clone();
        let resolved = resolve_variable_source(&span.name, layers, &runtime, &labels);
        let contents = format_variable_hover(&resolved, &labels);
        let range = byte_range_to_lsp_range(text, span.start..span.end);

        Task::ready(Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(contents)),
            range: Some(range),
        })))
    }
}

pub fn configure_variable_input(
    mut input: gpui_component::input::InputState,
    provider: Rc<VariableHoverProvider>,
) -> gpui_component::input::InputState {
    // Plain single-line input is enough for variable hover; code_editor adds
    // syntax parsing overhead and narrower hit-test bounds for short values.
    input.lsp.hover_provider = Some(provider);
    input
}

pub fn configure_variable_code_editor(
    mut input: gpui_component::input::InputState,
    provider: Rc<VariableHoverProvider>,
    language: impl Into<gpui::SharedString>,
) -> gpui_component::input::InputState {
    input = input
        .multi_line(true)
        .rows(12)
        .code_editor(language)
        .searchable(true);
    input.lsp.hover_provider = Some(provider);
    input
}

fn byte_range_to_lsp_range(text: &Rope, range: Range<usize>) -> LspRange {
    let start = text.offset_to_position(range.start);
    let end = text.offset_to_position(range.end);
    LspRange {
        start: lsp_types::Position::new(start.line, start.character),
        end: lsp_types::Position::new(end.line, end.character),
    }
}

impl LoomApp {
    pub(super) fn variable_layers_for_active_tab(
        &self,
    ) -> Option<(VariableLayers<'_>, VariableResolveLabels)> {
        let workspace = self.workspaces.get(self.active_workspace)?;
        let tab = self.active_tab()?;
        let source = tab.source;

        let collection = source.and_then(|source| {
            workspace
                .collections
                .get(source.collection)
        });
        let folder_variables = source
            .and_then(|source| {
                source
                    .folder
                    .and_then(|folder_index| collection?.folders.get(folder_index))
            })
            .map(|folder| folder.variables.as_slice())
            .unwrap_or(&[]);

        let layers = VariableLayers {
            global: &workspace.variables,
            collection: collection
                .map(|collection| collection.variables.as_slice())
                .unwrap_or(&[]),
            environment: self.active_environment_variables(),
            folder: folder_variables,
            request: &tab.variables,
        };

        Some((layers, self.variable_resolve_labels()))
    }

    fn variable_resolve_labels(&self) -> VariableResolveLabels {
        let mut labels = VariableResolveLabels::default();
        let workspace = &self.workspaces[self.active_workspace];

        if let Some(source) = self.active_tab().and_then(|tab| tab.source)
            && let Some(collection) = workspace.collections.get(source.collection)
        {
            labels.collection_name = Some(collection.name.clone());
            if let Some(folder_index) = source.folder
                && let Some(folder) = collection.folders.get(folder_index)
            {
                labels.folder_name = Some(folder.name.clone());
            }
        }

        let Some(environment_ref) = self.active_environment else {
            return labels;
        };

        match environment_ref.scope {
            crate::domain::EnvironmentScope::Workspace => {
                labels.workspace_environment_name = workspace
                    .environments
                    .get(environment_ref.index)
                    .map(|environment| environment.name.clone());
            }
            crate::domain::EnvironmentScope::Collection(collection_index) => {
                if let Some(collection) = workspace.collections.get(collection_index)
                    && let Some(environment) = collection.environments.get(environment_ref.index)
                {
                    labels.collection_environment_name = Some(environment.name.clone());
                }
            }
        }

        labels
    }
}
