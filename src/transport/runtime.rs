use std::sync::OnceLock;

use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub(crate) fn block_on<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    RUNTIME
        .get_or_init(|| Runtime::new().expect("tokio runtime"))
        .block_on(future)
}
