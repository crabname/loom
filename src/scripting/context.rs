use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use boa_engine::JsData;
use boa_gc::{Finalize, Trace};
use serde_json::Value;

use crate::domain::Variable;

/// Mutable script state exposed to JavaScript via the `host` global.
#[derive(Debug, Default)]
pub struct ScriptHostState {
    pub runtime_vars: HashMap<String, Value>,
    pub env_vars: HashMap<String, Value>,
    pub workspace_env_vars: HashMap<String, Value>,
    pub runtime_dirty: bool,
    pub env_dirty: bool,
}

impl ScriptHostState {
    pub fn from_parts(
        runtime_vars: HashMap<String, Value>,
        env_vars: HashMap<String, Value>,
        workspace_env_vars: HashMap<String, Value>,
    ) -> Self {
        Self {
            runtime_vars,
            env_vars,
            workspace_env_vars,
            runtime_dirty: false,
            env_dirty: false,
        }
    }
}

#[derive(Clone, Trace, Finalize, JsData)]
pub struct ScriptHostHandle {
    #[unsafe_ignore_trace]
    pub inner: Rc<RefCell<ScriptHostState>>,
}

impl ScriptHostHandle {
    pub fn new(state: ScriptHostState) -> Self {
        Self {
            inner: Rc::new(RefCell::new(state)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptConsoleLevel {
    Log,
    Debug,
    Info,
    Warn,
    Error,
}

impl ScriptConsoleLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Log => "log",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptConsoleEntry {
    pub level: ScriptConsoleLevel,
    pub message: String,
}

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct ConsoleLogStore {
    #[unsafe_ignore_trace]
    pub logs: Rc<RefCell<Vec<ScriptConsoleEntry>>>,
}

impl Default for ConsoleLogStore {
    fn default() -> Self {
        Self {
            logs: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ScriptResult {
    pub runtime_vars: HashMap<String, Value>,
    pub env_vars: HashMap<String, Value>,
    pub runtime_dirty: bool,
    pub env_dirty: bool,
    pub console_logs: Vec<ScriptConsoleEntry>,
}

impl ScriptResult {
    pub fn from_handle(handle: &ScriptHostHandle, console_logs: Vec<ScriptConsoleEntry>) -> Self {
        let state = handle.inner.borrow();
        Self {
            runtime_vars: state.runtime_vars.clone(),
            env_vars: state.env_vars.clone(),
            runtime_dirty: state.runtime_dirty,
            env_dirty: state.env_dirty,
            console_logs,
        }
    }
}

pub fn variables_to_map(variables: &[Variable]) -> HashMap<String, Value> {
    variables
        .iter()
        .filter(|variable| !variable.name.trim().is_empty())
        .map(|variable| (variable.name.clone(), variable.value.clone()))
        .collect()
}

pub fn map_to_variables(map: &HashMap<String, Value>) -> Vec<Variable> {
    let mut variables: Vec<Variable> = map
        .iter()
        .map(|(name, value)| Variable {
            name: name.clone(),
            value: value.clone(),
        })
        .collect();
    variables.sort_by(|left, right| left.name.cmp(&right.name));
    variables
}

pub fn merge_runtime_vars(
    pool: &mut HashMap<String, String>,
    runtime_vars: &HashMap<String, Value>,
) {
    for (name, value) in runtime_vars {
        pool.insert(name.clone(), json_value_to_string(value));
    }
}

pub fn json_value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
