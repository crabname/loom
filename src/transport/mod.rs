pub mod grpc;
pub mod http;
mod runtime;

pub use grpc::{discover_grpc_methods, generate_grpc_request_template, send_grpc_request};
pub use http::{send_http_request, HttpRequestBody, HttpRequestResult, HttpResponse};
pub(crate) use runtime::block_on;
