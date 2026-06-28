use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceBinding {
    Ephemeral,
    Local(PathBuf),
    #[allow(dead_code)]
    Cloud(CloudWorkspaceBinding),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudWorkspaceBinding {
    pub id: String,
    pub organization_id: Option<String>,
    pub display_name: String,
}

impl WorkspaceBinding {
    #[allow(dead_code)]
    pub fn is_persisted(&self) -> bool {
        !matches!(self, Self::Ephemeral)
    }

    pub fn local_path(&self) -> Option<&Path> {
        match self {
            Self::Local(path) => Some(path),
            _ => None,
        }
    }
}
