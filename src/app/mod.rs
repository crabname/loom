mod app_state;
mod autosave;
mod collection_import;
mod collections;
mod dispatch;
mod editor;
mod field_tables;
mod init;
pub mod menus;
mod startup;
mod tab;
mod tab_actions;
mod ui;
mod url_sync;
mod variable_hover;
mod workspace;
mod workspace_binding;
mod workspace_storage;

use gpui::*;
use gpui_component::{
    input::InputState,
    menu::AppMenuBar,
    select::SelectState,
    tree::TreeState,
};
use serde_json::Value;
use std::collections::HashMap;

use crate::domain::{EnvironmentRef, Workspace};
use crate::storage::AppPaths;

use tab::{Tab, TabSource};
use workspace_binding::WorkspaceBinding;

use ui::{MultipartRowInputs, RowInputs};

pub use menus::{ImportCollection, OpenSettings, OpenWorkspace};

pub struct LoomApp {
    pub(super) app_paths: AppPaths,
    pub(super) workspaces: Vec<Workspace>,
    pub(super) workspace_bindings: Vec<WorkspaceBinding>,
    pub(super) workspace_collection_paths: Vec<Vec<String>>,
    pub(super) active_workspace: usize,
    pub(super) app_menu_bar: Entity<AppMenuBar>,
    pub(super) tabs: Vec<Tab>,
    pub(super) active_tab: usize,
    pub(super) next_tab_id: usize,

    pub(super) url_input: Entity<InputState>,
    pub(super) body_input: Entity<InputState>,
    pub(super) pre_request_script_input: Entity<InputState>,
    pub(super) post_response_script_input: Entity<InputState>,
    pub(super) tests_script_input: Entity<InputState>,
    pub(super) response_body_input: Entity<InputState>,
    pub(super) method_select: Entity<SelectState<Vec<&'static str>>>,
    pub(super) body_type_select: Entity<SelectState<Vec<&'static str>>>,
    pub(super) workspace_select: Entity<SelectState<Vec<SharedString>>>,
    pub(super) environment_select: Entity<SelectState<Vec<SharedString>>>,
    pub(super) active_environment: Option<EnvironmentRef>,
    pub(super) runtime_vars: HashMap<String, Value>,

    pub(super) query_inputs: Vec<RowInputs>,
    pub(super) header_inputs: Vec<RowInputs>,
    pub(super) form_inputs: Vec<RowInputs>,
    pub(super) multipart_inputs: Vec<MultipartRowInputs>,
    pub(super) variable_inputs: Vec<RowInputs>,

    pub(super) query_sync_guard: bool,
    pub(super) url_parse_debounce_seq: u64,
    pub(super) autosave_debounce_seq: u64,
    pub(super) query_param_subscriptions: Vec<Subscription>,

    pub(super) collections_tree: Entity<TreeState>,
    pub(super) variable_hover: std::rc::Rc<variable_hover::VariableHoverProvider>,
    pub(super) startup_warnings: Vec<String>,
    pub(super) _subscriptions: Vec<Subscription>,
}
