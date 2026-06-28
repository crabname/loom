mod context;
mod host;
mod response;
mod runtime;

pub use context::{
    map_to_variables, merge_runtime_vars, variables_to_map, ScriptConsoleEntry, ScriptConsoleLevel,
    ScriptHostState, ScriptResult,
};
pub use response::ScriptResponseSnapshot;
pub use runtime::run_script;

use crate::transport::HttpResponse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptPhase {
    #[allow(dead_code)]
    PreRequest,
    PostResponse,
}

pub fn run_pre_request_script(
    script: &str,
    state: ScriptHostState,
) -> Result<ScriptResult, String> {
    run_script(script, state, None)
}

pub fn run_post_response_script(
    script: &str,
    state: ScriptHostState,
    response: &HttpResponse,
    request_url: String,
) -> Result<ScriptResult, String> {
    let _ = ScriptPhase::PostResponse;
    let snapshot = ScriptResponseSnapshot::from_http(response, request_url);
    run_script(script, state, Some(snapshot))
}
