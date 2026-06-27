use gpui::*;
use gpui_component::{
    input::InputState,
    select::{SelectEvent, SelectState},
    IndexPath,
};

use crate::domain::{Environment, Variable};

use crate::app::ui::fields::RowInputs;
use super::super::variables::{build_variable_row_inputs, flush_environment_variables};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EnvironmentManagerTab {
    WorkspaceEnv,
    CollectionEnv,
    GlobalVars,
    CollectionVars,
    FolderVars,
}

impl EnvironmentManagerTab {
    pub(super) fn is_environment(self) -> bool {
        matches!(self, Self::WorkspaceEnv | Self::CollectionEnv)
    }

    pub(super) fn uses_collection_picker(self) -> bool {
        matches!(self, Self::CollectionEnv | Self::CollectionVars | Self::FolderVars)
    }

    pub(super) fn uses_folder_picker(self) -> bool {
        matches!(self, Self::FolderVars)
    }
}

pub(crate) struct EnvironmentsManagerPanel {
    pub(super) tab: EnvironmentManagerTab,
    pub(super) workspace_environments: Vec<Environment>,
    pub(super) workspace_variables: Vec<Variable>,
    pub(super) collection_names: Vec<SharedString>,
    pub(super) collection_environments: Vec<Vec<Environment>>,
    pub(super) collection_variables: Vec<Vec<Variable>>,
    pub(super) folder_names: Vec<Vec<SharedString>>,
    pub(super) folder_variables: Vec<Vec<Vec<Variable>>>,
    pub(super) collection_index: usize,
    pub(super) folder_index: usize,
    pub(super) selected_index: usize,
    pub(super) name_input: Entity<InputState>,
    pub(super) variable_rows: Vec<RowInputs>,
    pub(super) collection_select: Option<Entity<SelectState<Vec<SharedString>>>>,
    pub(super) folder_select: Option<Entity<SelectState<Vec<SharedString>>>>,
    pub(super) _subscriptions: Vec<Subscription>,
}

impl EnvironmentsManagerPanel {
    pub(crate) fn new(
        window: &mut Window,
        cx: &mut App,
        workspace_environments: Vec<Environment>,
        workspace_variables: Vec<Variable>,
        collection_names: Vec<SharedString>,
        collection_environments: Vec<Vec<Environment>>,
        collection_variables: Vec<Vec<Variable>>,
        folder_names: Vec<Vec<SharedString>>,
        folder_variables: Vec<Vec<Vec<Variable>>>,
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

        let initial_folder_names = folder_names.first().cloned().unwrap_or_default();
        let folder_select = (!initial_folder_names.is_empty()).then(|| {
            cx.new(|cx| {
                SelectState::new(
                    initial_folder_names,
                    Some(IndexPath::default()),
                    window,
                    cx,
                )
            })
        });

        let panel = Self {
            tab: EnvironmentManagerTab::WorkspaceEnv,
            workspace_environments,
            workspace_variables,
            collection_names,
            collection_environments,
            collection_variables,
            folder_names,
            folder_variables,
            collection_index: 0,
            folder_index: 0,
            selected_index: 0,
            name_input,
            variable_rows,
            collection_select,
            folder_select,
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

        if let Some(select) = entity.read(cx).folder_select.clone() {
            entity.update(cx, |panel, cx| {
                panel._subscriptions.push(cx.subscribe_in(&select, window, {
                    move |panel, _, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                        let SelectEvent::Confirm(Some(name)) = event else {
                            return;
                        };
                        let Some(index) = panel
                            .folder_names
                            .get(panel.collection_index)
                            .and_then(|folders| {
                                folders.iter().position(|folder| folder == name)
                            })
                        else {
                            return;
                        };
                        panel.switch_folder(index, window, cx);
                    }
                }));
            });
        }

        entity
    }

    pub(super) fn refresh_folder_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let folder_names = self
            .folder_names
            .get(self.collection_index)
            .cloned()
            .unwrap_or_default();

        if folder_names.is_empty() {
            self.folder_select = None;
            self.folder_index = 0;
            return;
        }

        self.folder_index = self.folder_index.min(folder_names.len().saturating_sub(1));

        if let Some(select) = &self.folder_select {
            select.update(cx, |select, cx| {
                select.set_items(folder_names.clone(), window, cx);
                select.set_selected_value(&folder_names[self.folder_index], window, cx);
            });
        } else {
            let selected = self.folder_index;
            self.folder_select = Some(cx.new(|cx| {
                SelectState::new(
                    folder_names.clone(),
                    Some(IndexPath::default().row(selected)),
                    window,
                    cx,
                )
            }));
            let select = self.folder_select.clone().expect("folder select exists");
            self._subscriptions.push(cx.subscribe_in(&select, window, {
                move |panel, _, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                    let SelectEvent::Confirm(Some(name)) = event else {
                        return;
                    };
                    let Some(index) = panel
                        .folder_names
                        .get(panel.collection_index)
                        .and_then(|folders| folders.iter().position(|folder| folder == name))
                    else {
                        return;
                    };
                    panel.switch_folder(index, window, cx);
                }
            }));
        }
    }

    pub(super) fn current_environments(&self) -> &[Environment] {
        match self.tab {
            EnvironmentManagerTab::WorkspaceEnv => &self.workspace_environments,
            EnvironmentManagerTab::CollectionEnv => self
                .collection_environments
                .get(self.collection_index)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    fn current_environments_mut(&mut self) -> &mut Vec<Environment> {
        match self.tab {
            EnvironmentManagerTab::WorkspaceEnv => &mut self.workspace_environments,
            EnvironmentManagerTab::CollectionEnv => {
                &mut self.collection_environments[self.collection_index]
            }
            _ => panic!("not an environment tab"),
        }
    }

    pub(super) fn current_scope_variables(&self) -> &[Variable] {
        match self.tab {
            EnvironmentManagerTab::GlobalVars => &self.workspace_variables,
            EnvironmentManagerTab::CollectionVars => self
                .collection_variables
                .get(self.collection_index)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            EnvironmentManagerTab::FolderVars => self
                .folder_variables
                .get(self.collection_index)
                .and_then(|folders| folders.get(self.folder_index))
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            EnvironmentManagerTab::WorkspaceEnv => self
                .current_environments()
                .get(self.selected_index)
                .map(|environment| environment.variables.as_slice())
                .unwrap_or(&[]),
            EnvironmentManagerTab::CollectionEnv => self
                .current_environments()
                .get(self.selected_index)
                .map(|environment| environment.variables.as_slice())
                .unwrap_or(&[]),
        }
    }

    fn has_folder_variables_scope(&self) -> bool {
        self.folder_variables
            .get(self.collection_index)
            .is_some_and(|folders| folders.get(self.folder_index).is_some())
    }

    fn current_scope_variables_mut(&mut self) -> &mut Vec<Variable> {
        match self.tab {
            EnvironmentManagerTab::GlobalVars => &mut self.workspace_variables,
            EnvironmentManagerTab::CollectionVars => {
                &mut self.collection_variables[self.collection_index]
            }
            EnvironmentManagerTab::FolderVars => {
                &mut self.folder_variables[self.collection_index][self.folder_index]
            }
            EnvironmentManagerTab::WorkspaceEnv => {
                let index = self.selected_index;
                &mut self.current_environments_mut()[index].variables
            }
            EnvironmentManagerTab::CollectionEnv => {
                let index = self.selected_index;
                &mut self.current_environments_mut()[index].variables
            }
        }
    }

    fn flush_selected(&mut self, cx: &App) {
        if self.tab.is_environment() {
            let selected_index = self.selected_index;
            let name = self.name_input.read(cx).value().to_string();
            let rows = self.variable_rows.clone();
            let Some(environment) = self.current_environments_mut().get_mut(selected_index) else {
                return;
            };

            environment.name = name;
            flush_environment_variables(&mut environment.variables, &rows, cx);
        } else if self.tab == EnvironmentManagerTab::FolderVars && !self.has_folder_variables_scope() {
            // No folder selected.
        } else {
            let rows = self.variable_rows.clone();
            let variables = self.current_scope_variables_mut();
            flush_environment_variables(variables, &rows, cx);
        }
    }

    fn reload_selected_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab.is_environment() {
            let environment = self
                .current_environments()
                .get(self.selected_index)
                .cloned()
                .unwrap_or_else(|| Environment::new("New Environment"));

            self.name_input.update(cx, |input, cx| {
                input.set_value(environment.name.clone(), window, cx);
            });
            self.variable_rows =
                build_variable_row_inputs(window, cx, &environment.variables);
        } else if self.tab == EnvironmentManagerTab::FolderVars && !self.has_folder_variables_scope() {
            self.variable_rows.clear();
        } else {
            let variables = self.current_scope_variables();
            if variables.is_empty() {
                self.current_scope_variables_mut().push(Variable::empty());
            }
            self.variable_rows =
                build_variable_row_inputs(window, cx, self.current_scope_variables());
        }
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
        if tab.uses_folder_picker() {
            self.refresh_folder_select(window, cx);
        }
        self.reload_selected_inputs(window, cx);
        cx.notify();
    }

    fn switch_collection(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.collection_environments.len() || index == self.collection_index {
            return;
        }

        self.flush_selected(cx);
        self.collection_index = index;
        self.folder_index = 0;
        self.selected_index = 0;
        self.refresh_folder_select(window, cx);
        self.reload_selected_inputs(window, cx);
        cx.notify();
    }

    fn switch_folder(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let folder_count = self
            .folder_names
            .get(self.collection_index)
            .map(Vec::len)
            .unwrap_or(0);
        if index >= folder_count || index == self.folder_index {
            return;
        }

        self.flush_selected(cx);
        self.folder_index = index;
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
        self.current_scope_variables_mut().push(Variable::empty());
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
        let variables = self.current_scope_variables_mut();
        if variables.len() > 1 {
            variables.remove(index);
        } else {
            variables[0] = Variable::empty();
        }
        self.reload_selected_inputs(window, cx);
        cx.notify();
    }

    pub(crate) fn take_state(
        &mut self,
        cx: &App,
    ) -> (
        Vec<Environment>,
        Vec<Vec<Environment>>,
        Vec<Variable>,
        Vec<Vec<Variable>>,
        Vec<Vec<Vec<Variable>>>,
    ) {
        self.flush_selected(cx);
        (
            std::mem::take(&mut self.workspace_environments),
            std::mem::take(&mut self.collection_environments),
            std::mem::take(&mut self.workspace_variables),
            std::mem::take(&mut self.collection_variables),
            std::mem::take(&mut self.folder_variables),
        )
    }
}
