pub mod http;
mod runtime;

pub use http::{send_http_request, HttpRequestBody, HttpRequestResult, HttpResponse};
pub(crate) use runtime::block_on;
