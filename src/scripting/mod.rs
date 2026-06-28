mod context;
mod host;
mod runtime;

pub use context::{
    map_to_variables, merge_runtime_vars, variables_to_map, ScriptHostState, ScriptResult,
};
pub use runtime::run_script;

use crate::transport::HttpResponse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptPhase {
    PreRequest,
    PostResponse,
}

pub fn run_pre_request_script(
    script: &str,
    state: ScriptHostState,
) -> Result<ScriptResult, String> {
    run_script(script, state)
}

pub fn run_post_response_script(
    script: &str,
    state: ScriptHostState,
    _response: &HttpResponse,
) -> Result<ScriptResult, String> {
    let _ = ScriptPhase::PostResponse;
    run_script(script, state)
}
