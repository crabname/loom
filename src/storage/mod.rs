mod app_state;
mod local;
mod paths;
mod yaml;

pub use app_state::{AppState, WorkspaceRef};
pub use local::{
    default_workspace_collection_paths, LoadedWorkspace, LocalStorageProvider,
};
pub use paths::AppPaths;

pub trait StorageProvider {
    fn load_workspace(&self, path: &std::path::Path) -> Result<LoadedWorkspace, String>;

    fn save_workspace(
        &self,
        path: &std::path::Path,
        workspace: &crate::domain::Workspace,
        collection_paths: &mut Vec<String>,
    ) -> Result<(), String>;
}

impl StorageProvider for LocalStorageProvider {
    fn load_workspace(&self, path: &std::path::Path) -> Result<LoadedWorkspace, String> {
        Self::load_workspace(path)
    }

    fn save_workspace(
        &self,
        path: &std::path::Path,
        workspace: &crate::domain::Workspace,
        collection_paths: &mut Vec<String>,
    ) -> Result<(), String> {
        Self::save_workspace(path, workspace, collection_paths)
    }
}
