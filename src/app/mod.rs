mod collections;
mod dispatch;
mod editor;
mod field_tables;
mod init;
mod tab;
mod tab_actions;
mod ui;
mod url_sync;
mod workspace;

use gpui::*;
use gpui_component::{
    input::InputState,
    select::SelectState,
    tree::TreeState,
};

use crate::domain::{EnvironmentRef, Workspace};

use tab::{Tab, TabSource, WorkspaceSession};

use ui::{MultipartRowInputs, RowInputs};

pub struct ApiHelperApp {
    pub(super) workspaces: Vec<Workspace>,
    pub(super) active_workspace: usize,
    pub(super) workspace_sessions: Vec<Option<WorkspaceSession>>,
    pub(super) tabs: Vec<Tab>,
    pub(super) active_tab: usize,
    pub(super) next_tab_id: usize,

    pub(super) url_input: Entity<InputState>,
    pub(super) body_input: Entity<InputState>,
    pub(super) response_body_input: Entity<InputState>,
    pub(super) method_select: Entity<SelectState<Vec<&'static str>>>,
    pub(super) body_type_select: Entity<SelectState<Vec<&'static str>>>,
    pub(super) workspace_select: Entity<SelectState<Vec<SharedString>>>,
    pub(super) environment_select: Entity<SelectState<Vec<SharedString>>>,
    pub(super) active_environment: Option<EnvironmentRef>,

    pub(super) query_inputs: Vec<RowInputs>,
    pub(super) header_inputs: Vec<RowInputs>,
    pub(super) form_inputs: Vec<RowInputs>,
    pub(super) multipart_inputs: Vec<MultipartRowInputs>,
    pub(super) variable_inputs: Vec<RowInputs>,

    pub(super) query_sync_guard: bool,
    pub(super) url_parse_debounce_seq: u64,
    pub(super) query_param_subscriptions: Vec<Subscription>,

    pub(super) collections_tree: Entity<TreeState>,
    pub(super) _subscriptions: Vec<Subscription>,
}
