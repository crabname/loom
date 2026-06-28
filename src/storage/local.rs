use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::domain::{Collection, CollectionFolder, EntityId, Environment, Request, Variable, Workspace};

use super::yaml::{
    collection_from_parts, serializable_variables, workspace_from_parts, CollectionFile,
    CollectionRef, EnvironmentFile, FolderFile, RequestFile, VariablesFile, WorkspaceFile,
    WORKSPACE_FORMAT_VERSION,
};

const WORKSPACE_FILE: &str = "workspace.yml";
const VARIABLES_FILE: &str = "variables.yml";
const COLLECTION_FILE: &str = "collection.yml";
const FOLDER_FILE: &str = "folder.yml";
const ENVIRONMENTS_DIR: &str = "environments";
const COLLECTIONS_DIR: &str = "collections";

pub struct LoadedWorkspace {
    pub workspace: Workspace,
    pub collection_paths: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct LocalStorageProvider;

impl LocalStorageProvider {
    pub fn load_workspace(path: &Path) -> Result<LoadedWorkspace, String> {
        if !path.is_dir() {
            return Err(format!("workspace folder not found: {}", path.display()));
        }

        if !path.join(WORKSPACE_FILE).is_file() {
            return Err(format!(
                "workspace is missing {WORKSPACE_FILE} in {}",
                path.display()
            ));
        }

        let mut warnings = Vec::new();
        let workspace_file = read_yaml::<WorkspaceFile>(&path.join(WORKSPACE_FILE))?;
        if workspace_file.version != WORKSPACE_FORMAT_VERSION {
            return Err(format!(
                "unsupported workspace format version {} in {} (expected {WORKSPACE_FORMAT_VERSION})",
                workspace_file.version,
                path.display()
            ));
        }

        let variables = match path.join(VARIABLES_FILE).exists() {
            true => match read_yaml::<VariablesFile>(&path.join(VARIABLES_FILE)) {
                Ok(file) => file
                    .variables
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                Err(error) => {
                    push_warning(
                        &mut warnings,
                        format!("skipped {VARIABLES_FILE} in {}: {error}", path.display()),
                    );
                    Vec::new()
                }
            },
            false => Vec::new(),
        };

        let environments = load_environments(&path.join(ENVIRONMENTS_DIR), &mut warnings)?;

        let mut collections = Vec::new();
        let mut collection_paths = Vec::new();
        let collection_refs = workspace_file.collections;
        let had_collections = !collection_refs.is_empty();
        for collection_ref in collection_refs {
            let collection_path = collection_dir_path(collection_ref.id);
            let full_path = path.join(&collection_path);
            match load_collection(&full_path, collection_ref.id, &mut warnings) {
                Ok(collection) => {
                    collections.push(collection);
                    collection_paths.push(collection_path);
                }
                Err(error) => push_warning(
                    &mut warnings,
                    format!(
                        "skipped collection {} in {}: {error}",
                        collection_ref.id,
                        full_path.display()
                    ),
                ),
            }
        }

        if collections.is_empty() && had_collections {
            warnings.push(format!(
                "workspace {} loaded with no collections; check collections/ for damaged files",
                path.display()
            ));
        }

        Ok(LoadedWorkspace {
            workspace: workspace_from_parts(
                workspace_file.name,
                variables,
                environments,
                collections,
            ),
            collection_paths,
            warnings,
        })
    }

    pub fn save_workspace(
        path: &Path,
        workspace: &Workspace,
        collection_paths: &mut Vec<String>,
    ) -> Result<(), String> {
        fs::create_dir_all(path).map_err(|error| error.to_string())?;

        sync_collection_paths(workspace, collection_paths);

        write_yaml(
            &path.join(WORKSPACE_FILE),
            &WorkspaceFile {
                version: WORKSPACE_FORMAT_VERSION,
                name: workspace.name.clone(),
                collections: workspace
                    .collections
                    .iter()
                    .map(CollectionRef::from_collection)
                    .collect(),
            },
        )?;

        save_variables(&path.join(VARIABLES_FILE), &workspace.variables)?;
        save_environments(&path.join(ENVIRONMENTS_DIR), &workspace.environments)?;

        let mut keep_collections = HashSet::new();
        for (collection, collection_path) in workspace.collections.iter().zip(collection_paths.iter()) {
            save_collection(&path.join(collection_path), collection)?;
            keep_collections.insert(
                collection_path
                    .strip_prefix(&format!("{COLLECTIONS_DIR}/"))
                    .unwrap_or(collection_path)
                    .to_string(),
            );
        }

        prune_stale_collection_dirs(&path.join(COLLECTIONS_DIR), &keep_collections)?;
        Ok(())
    }
}

fn collection_dir_path(id: EntityId) -> String {
    format!("{COLLECTIONS_DIR}/{id}")
}

fn sync_collection_paths(workspace: &Workspace, collection_paths: &mut Vec<String>) {
    collection_paths.clear();
    collection_paths.extend(
        workspace
            .collections
            .iter()
            .map(|collection| collection_dir_path(collection.id)),
    );
}

fn entity_file_name(id: EntityId) -> String {
    format!("{id}.yml")
}

fn save_variables(path: &Path, variables: &[Variable]) -> Result<(), String> {
    let variables = serializable_variables(variables);
    if variables.is_empty() {
        if path.is_file() {
            fs::remove_file(path).map_err(|error| error.to_string())?;
        }
        return Ok(());
    }

    write_yaml(
        path,
        &VariablesFile {
            variables,
        },
    )
}

fn save_environments(path: &Path, environments: &[Environment]) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| error.to_string())?;

    let mut keep = HashSet::new();
    for environment in environments {
        let file_name = entity_file_name(environment.id);
        write_yaml(&path.join(&file_name), &EnvironmentFile::from(environment))?;
        keep.insert(file_name);
    }

    prune_stale_yaml_files(path, &keep)
}

fn save_collection(path: &Path, collection: &Collection) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| error.to_string())?;

    write_yaml(&path.join(COLLECTION_FILE), &CollectionFile::from(collection))?;
    save_environments(&path.join(ENVIRONMENTS_DIR), &collection.environments)?;

    let mut keep_files = HashSet::from([COLLECTION_FILE.to_string()]);
    let mut keep_dirs = HashSet::new();
    for request in &collection.requests {
        let file_name = entity_file_name(request.id);
        save_request(&path.join(&file_name), request)?;
        keep_files.insert(file_name);
    }

    for folder in &collection.folders {
        save_folder(path, folder)?;
        keep_dirs.insert(folder.id.to_string());
    }

    prune_stale_yaml_files(path, &keep_files)?;
    prune_stale_subdirs(path, &keep_dirs, &[ENVIRONMENTS_DIR])
}

fn save_folder(collection_path: &Path, folder: &CollectionFolder) -> Result<(), String> {
    let folder_path = collection_path.join(folder.id.to_string());
    fs::create_dir_all(&folder_path).map_err(|error| error.to_string())?;
    write_yaml(&folder_path.join(FOLDER_FILE), &FolderFile::from(folder))?;

    let mut keep = HashSet::from([FOLDER_FILE.to_string()]);
    for request in &folder.requests {
        let file_name = entity_file_name(request.id);
        save_request(&folder_path.join(&file_name), request)?;
        keep.insert(file_name);
    }

    prune_stale_yaml_files(&folder_path, &keep)
}

fn save_request(path: &Path, request: &Request) -> Result<(), String> {
    write_yaml(path, &RequestFile::from(request))
}

fn prune_stale_yaml_files(dir: &Path, keep: &HashSet<String>) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            continue;
        }

        let file_name = entry
            .file_name()
            .to_string_lossy()
            .into_owned();
        if file_name.ends_with(".yml") && !keep.contains(&file_name) {
            fs::remove_file(entry.path()).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

fn prune_stale_subdirs(
    dir: &Path,
    keep: &HashSet<String>,
    reserved: &[&str],
) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().into_owned();
        if reserved.contains(&dir_name.as_str()) || keep.contains(&dir_name) {
            continue;
        }

        fs::remove_dir_all(entry.path()).map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn prune_stale_collection_dirs(collections_root: &Path, keep: &HashSet<String>) -> Result<(), String> {
    prune_stale_subdirs(collections_root, keep, &[])
}

fn push_warning(warnings: &mut Vec<String>, message: String) {
    eprintln!("{message}");
    warnings.push(message);
}

fn load_collection(
    collection_path: &Path,
    expected_id: EntityId,
    warnings: &mut Vec<String>,
) -> Result<Collection, String> {
    if !collection_path.is_dir() {
        return Err(format!(
            "collection folder not found: {}",
            collection_path.display()
        ));
    }

    let collection_file = read_yaml::<CollectionFile>(&collection_path.join(COLLECTION_FILE))?;
    if collection_file.id != expected_id {
        return Err(format!(
            "collection id mismatch in {}: expected {expected_id}, found {}",
            collection_path.display(),
            collection_file.id
        ));
    }

    let variables = collection_file
        .variables
        .into_iter()
        .map(Into::into)
        .collect::<Vec<Variable>>();

    let environments =
        load_environments(&collection_path.join(ENVIRONMENTS_DIR), warnings)?;

    let mut folders = Vec::new();
    let mut requests = Vec::new();

    for entry in fs::read_dir(collection_path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let entry_path = entry.path();
        let file_name = entry_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();

        if file_name == COLLECTION_FILE || file_name == ENVIRONMENTS_DIR {
            continue;
        }

        if file_type.is_dir() {
            match load_folder(&entry_path, warnings) {
                Ok(folder) => folders.push(folder),
                Err(error) => push_warning(
                    warnings,
                    format!("skipped folder {}: {error}", entry_path.display()),
                ),
            }
            continue;
        }

        if file_type.is_file() && entry_path.extension().is_some_and(|ext| ext == "yml") {
            match load_request(&entry_path) {
                Ok(request) => requests.push(request),
                Err(error) => push_warning(
                    warnings,
                    format!("skipped request {}: {error}", entry_path.display()),
                ),
            }
        }
    }

    folders.sort_by(|left, right| left.name.cmp(&right.name));
    requests.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(collection_from_parts(
        collection_file.id,
        collection_file.name,
        variables,
        environments,
        folders,
        requests,
    ))
}

fn load_folder(folder_path: &Path, warnings: &mut Vec<String>) -> Result<CollectionFolder, String> {
    let mut folder: CollectionFolder =
        read_yaml::<FolderFile>(&folder_path.join(FOLDER_FILE))?.into();

    let expected_dir = folder.id.to_string();
    let actual_dir = folder_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if actual_dir != expected_dir {
        return Err(format!(
            "folder directory name mismatch in {}: expected {expected_dir}, found {actual_dir}",
            folder_path.display()
        ));
    }

    for entry in fs::read_dir(folder_path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let entry_path = entry.path();
        let file_name = entry_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();

        if file_name == FOLDER_FILE {
            continue;
        }

        if file_type.is_file() && entry_path.extension().is_some_and(|ext| ext == "yml") {
            match load_request(&entry_path) {
                Ok(request) => folder.requests.push(request),
                Err(error) => push_warning(
                    warnings,
                    format!("skipped request {}: {error}", entry_path.display()),
                ),
            }
        }
    }

    folder.requests.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(folder)
}

fn load_request(path: &Path) -> Result<Request, String> {
    let request: Request = read_yaml::<RequestFile>(path)?.into();
    let expected_file = entity_file_name(request.id);
    let actual_file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if actual_file != expected_file {
        return Err(format!(
            "request file name mismatch in {}: expected {expected_file}, found {actual_file}",
            path.display()
        ));
    }
    Ok(request)
}

fn load_environments(path: &Path, warnings: &mut Vec<String>) -> Result<Vec<Environment>, String> {
    if !path.is_dir() {
        return Ok(Vec::new());
    }

    let mut environments: Vec<Environment> = Vec::new();
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            continue;
        }

        let entry_path = entry.path();
        if entry_path.extension().is_some_and(|ext| ext == "yml") {
            match load_environment(&entry_path) {
                Ok(environment) => environments.push(environment),
                Err(error) => push_warning(
                    warnings,
                    format!("skipped environment {}: {error}", entry_path.display()),
                ),
            }
        }
    }

    environments.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(environments)
}

fn load_environment(path: &Path) -> Result<Environment, String> {
    let environment: Environment = read_yaml::<EnvironmentFile>(path)?.into();
    let expected_file = entity_file_name(environment.id);
    let actual_file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if actual_file != expected_file {
        return Err(format!(
            "environment file name mismatch in {}: expected {expected_file}, found {actual_file}",
            path.display()
        ));
    }
    Ok(environment)
}

fn read_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path).map_err(|error| {
        format!("failed to read {}: {error}", path.display())
    })?;
    serde_yaml::from_str(&content).map_err(|error| {
        format!("failed to parse {}: {error}", path.display())
    })
}

fn write_yaml<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let content = serde_yaml::to_string(value).map_err(|error| {
        format!("failed to serialize {}: {error}", path.display())
    })?;
    fs::write(path, content).map_err(|error| {
        format!("failed to write {}: {error}", path.display())
    })
}

pub fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut last_hyphen = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_hyphen = false;
        } else if !last_hyphen && !slug.is_empty() {
            slug.push('-');
            last_hyphen = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "untitled".into()
    } else {
        slug
    }
}

pub fn remove_collection_dir(workspace_path: &Path, collection_path: &str) -> Result<(), String> {
    let path = workspace_path.join(collection_path);
    if path.is_dir() {
        fs::remove_dir_all(&path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn remove_folder_dir(
    workspace_path: &Path,
    collection_path: &str,
    folder_id: EntityId,
) -> Result<(), String> {
    let path = workspace_path.join(collection_path).join(folder_id.to_string());
    if path.is_dir() {
        fs::remove_dir_all(&path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn default_collection_paths(workspace: &Workspace) -> Vec<String> {
    workspace
        .collections
        .iter()
        .map(|collection| collection_dir_path(collection.id))
        .collect()
}

pub fn default_workspace_collection_paths(workspaces: &[Workspace]) -> Vec<Vec<String>> {
    workspaces
        .iter()
        .map(default_collection_paths)
        .collect()
}
