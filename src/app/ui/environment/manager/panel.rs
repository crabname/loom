use gpui::*;
use gpui_component::{
    input::InputState,
    select::{SelectEvent, SelectState},
    IndexPath,
};

use crate::domain::Environment;

use crate::app::ui::fields::RowInputs;
use super::super::variables::{build_variable_row_inputs, flush_environment_variables};

type EnvironmentManagerState = (Vec<Environment>, Vec<Vec<Environment>>);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EnvironmentManagerTab {
    WorkspaceEnv,
    CollectionEnv,
}

impl EnvironmentManagerTab {
    pub(super) fn uses_collection_picker(self) -> bool {
        matches!(self, Self::CollectionEnv)
    }
}

pub(crate) struct EnvironmentsManagerPanel {
    pub(super) tab: EnvironmentManagerTab,
    pub(super) workspace_environments: Vec<Environment>,
    pub(super) collection_names: Vec<SharedString>,
    pub(super) collection_environments: Vec<Vec<Environment>>,
    pub(super) collection_index: usize,
    pub(super) selected_index: usize,
    pub(super) name_input: Entity<InputState>,
    pub(super) variable_rows: Vec<RowInputs>,
    pub(super) collection_select: Option<Entity<SelectState<Vec<SharedString>>>>,
    pub(super) _subscriptions: Vec<Subscription>,
}

impl EnvironmentsManagerPanel {
    pub(crate) fn new(
        window: &mut Window,
        cx: &mut App,
        workspace_environments: Vec<Environment>,
        collection_names: Vec<SharedString>,
        collection_environments: Vec<Vec<Environment>>,
    ) -> Entity<Self> {
        let initial_env = workspace_environments
            .first()
            .cloned()
            .unwrap_or_else(|| Environment::new("New Environment"));

        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Environment name")
                .default_value(initial_env.name.clone())
        });

        let variable_rows = build_variable_row_inputs(window, cx, &initial_env.variables);

        let collection_select = (!collection_names.is_empty()).then(|| {
            cx.new(|cx| {
                SelectState::new(
                    collection_names.clone(),
                    Some(IndexPath::default()),
                    window,
                    cx,
                )
            })
        });

        let panel = Self {
            tab: EnvironmentManagerTab::WorkspaceEnv,
            workspace_environments,
            collection_names,
            collection_environments,
            collection_index: 0,
            selected_index: 0,
            name_input,
            variable_rows,
            collection_select,
            _subscriptions: Vec::new(),
        };

        let entity = cx.new(|_| panel);

        if let Some(select) = entity.read(cx).collection_select.clone() {
            entity.update(cx, |panel, cx| {
                panel._subscriptions.push(cx.subscribe_in(&select, window, {
                    move |panel, _, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                        let SelectEvent::Confirm(Some(name)) = event else {
                            return;
                        };
                        let Some(index) = panel
                            .collection_names
                            .iter()
                            .position(|collection| collection == name)
                        else {
                            return;
                        };
                        panel.switch_collection(index, window, cx);
                    }
                }));
            });
        }

        entity
    }

    pub(super) fn current_environments(&self) -> &[Environment] {
        match self.tab {
            EnvironmentManagerTab::WorkspaceEnv => &self.workspace_environments,
            EnvironmentManagerTab::CollectionEnv => self
                .collection_environments
                .get(self.collection_index)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        }
    }

    fn current_environments_mut(&mut self) -> &mut Vec<Environment> {
        match self.tab {
            EnvironmentManagerTab::WorkspaceEnv => &mut self.workspace_environments,
            EnvironmentManagerTab::CollectionEnv => {
                &mut self.collection_environments[self.collection_index]
            }
        }
    }

    fn current_environment_variables_mut(&mut self) -> &mut Vec<crate::domain::Variable> {
        let index = self.selected_index;
        &mut self.current_environments_mut()[index].variables
    }

    fn flush_selected(&mut self, cx: &App) {
        let selected_index = self.selected_index;
        let name = self.name_input.read(cx).value().to_string();
        let rows = self.variable_rows.clone();
        let Some(environment) = self.current_environments_mut().get_mut(selected_index) else {
            return;
        };

        environment.name = name;
        flush_environment_variables(&mut environment.variables, &rows, cx);
    }

    fn reload_selected_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let environment = self
            .current_environments()
            .get(self.selected_index)
            .cloned()
            .unwrap_or_else(|| Environment::new("New Environment"));

        self.name_input.update(cx, |input, cx| {
            input.set_value(environment.name.clone(), window, cx);
        });
        self.variable_rows = build_variable_row_inputs(window, cx, &environment.variables);
    }

    pub(super) fn select_environment(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index == self.selected_index {
            return;
        }

        self.flush_selected(cx);
        self.selected_index = index;
        self.reload_selected_inputs(window, cx);
        cx.notify();
    }

    pub(super) fn switch_tab(&mut self, tab: EnvironmentManagerTab, window: &mut Window, cx: &mut Context<Self>) {
        if tab == self.tab {
            return;
        }

        self.flush_selected(cx);
        self.tab = tab;
        self.selected_index = 0;
        self.reload_selected_inputs(window, cx);
        cx.notify();
    }

    fn switch_collection(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.collection_environments.len() || index == self.collection_index {
            return;
        }

        self.flush_selected(cx);
        self.collection_index = index;
        self.selected_index = 0;
        self.reload_selected_inputs(window, cx);
        cx.notify();
    }

    pub(super) fn add_environment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush_selected(cx);

        let environments = self.current_environments_mut();
        let number = environments.len() + 1;
        let name = if number == 1 {
            "New Environment".into()
        } else {
            format!("New Environment {number}")
        };
        environments.push(Environment::new(name));
        self.selected_index = environments.len() - 1;
        self.reload_selected_inputs(window, cx);
        cx.notify();
    }

    pub(super) fn delete_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush_selected(cx);

        let selected_index = self.selected_index;
        let environments = self.current_environments_mut();
        if environments.is_empty() {
            return;
        }

        if environments.len() > 1 {
            environments.remove(selected_index);
            self.selected_index = selected_index.min(environments.len().saturating_sub(1));
        } else {
            environments[0] = Environment::new("New Environment");
        }

        self.reload_selected_inputs(window, cx);
        cx.notify();
    }

    pub(super) fn add_variable_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush_selected(cx);
        self.current_environment_variables_mut().push(crate::domain::Variable::empty());
        self.reload_selected_inputs(window, cx);
        cx.notify();
    }

    pub(super) fn remove_variable_row(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.flush_selected(cx);
        let variables = self.current_environment_variables_mut();
        if variables.len() > 1 {
            variables.remove(index);
        } else {
            variables[0] = crate::domain::Variable::empty();
        }
        self.reload_selected_inputs(window, cx);
        cx.notify();
    }

    pub(crate) fn take_state(&mut self, cx: &App) -> EnvironmentManagerState {
        self.flush_selected(cx);
        (
            std::mem::take(&mut self.workspace_environments),
            std::mem::take(&mut self.collection_environments),
        )
    }
}
