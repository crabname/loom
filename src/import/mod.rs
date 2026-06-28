mod opencollection;
mod postman;
mod shared;

use std::path::Path;

pub use shared::ImportResult;

use opencollection::{export_opencollection, import_opencollection};
use postman::{export_postman_json, import_postman};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionFormat {
    OpenCollection,
    Postman,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportTarget {
    OpenCollectionFolder,
    PostmanFile,
}

pub fn detect_format(path: &Path) -> Option<CollectionFormat> {
    if path.is_dir() {
        return Some(CollectionFormat::OpenCollection);
    }

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "json" => Some(CollectionFormat::Postman),
        "yml" | "yaml" => Some(CollectionFormat::OpenCollection),
        _ if path.is_file() => None,
        _ => None,
    }
}

pub fn import_collection(path: &Path) -> Result<ImportResult, String> {
    let format = detect_format(path).ok_or_else(|| {
        format!("unsupported import path: {}", path.display())
    })?;

    match format {
        CollectionFormat::OpenCollection => import_opencollection(path),
        CollectionFormat::Postman => import_postman(path),
    }
}

pub fn export_collection(
    collection: &crate::domain::Collection,
    target: ExportTarget,
    path: &Path,
) -> Result<Vec<String>, String> {
    match target {
        ExportTarget::OpenCollectionFolder => export_opencollection(collection, path),
        ExportTarget::PostmanFile => {
            let (json, warnings) = export_postman_json(collection)?;
            std::fs::write(path, json).map_err(|error| error.to_string())?;
            Ok(warnings)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_json_as_postman() {
        let path = Path::new("/tmp/demo.postman_collection.json");
        assert_eq!(detect_format(path), Some(CollectionFormat::Postman));
    }

    #[test]
    fn detects_yaml_as_opencollection() {
        let path = Path::new("/tmp/demo.yml");
        assert_eq!(detect_format(path), Some(CollectionFormat::OpenCollection));
    }
}
