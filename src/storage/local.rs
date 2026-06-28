use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::domain::{Collection, CollectionFolder, Environment, Request, Variable, Workspace};

use super::yaml::{
    collection_from_parts, serializable_variables, workspace_from_parts, CollectionFile,
    CollectionRef, EnvironmentFile, FolderFile, RequestFile, VariablesFile, WorkspaceFile,
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
}

pub struct LocalStorageProvider;

impl LocalStorageProvider {
    pub fn load_workspace(path: &Path) -> Result<LoadedWorkspace, String> {
        let workspace_file = read_yaml::<WorkspaceFile>(&path.join(WORKSPACE_FILE))?;

        let variables = if path.join(VARIABLES_FILE).exists() {
            read_yaml::<VariablesFile>(&path.join(VARIABLES_FILE))?
                .variables
                .into_iter()
                .map(Into::into)
                .collect()
        } else {
            Vec::new()
        };

        let environments = load_environments(&path.join(ENVIRONMENTS_DIR))?;

        let collection_refs = if workspace_file.collections.is_empty() {
            discover_collections(path)?
        } else {
            workspace_file.collections
        };

        let collection_paths = collection_refs
            .iter()
            .map(|collection_ref| collection_ref.path.clone())
            .collect::<Vec<_>>();

        let mut collections = Vec::new();
        for collection_ref in collection_refs {
            let collection_path = path.join(&collection_ref.path);
            collections.push(load_collection(&collection_path)?);
        }

        Ok(LoadedWorkspace {
            workspace: workspace_from_parts(
                workspace_file.name,
                variables,
                environments,
                collections,
            ),
            collection_paths,
        })
    }

    pub fn save_workspace(
        path: &Path,
        workspace: &Workspace,
        collection_paths: &mut Vec<String>,
    ) -> Result<(), String> {
        fs::create_dir_all(path).map_err(|error| error.to_string())?;

        ensure_collection_paths(workspace, collection_paths);

        write_yaml(
            &path.join(WORKSPACE_FILE),
            &WorkspaceFile {
                version: 1,
                name: workspace.name.clone(),
                collections: collection_paths
                    .iter()
                    .map(|collection_path| CollectionRef {
                        path: collection_path.clone(),
                    })
                    .collect(),
            },
        )?;

        save_variables(&path.join(VARIABLES_FILE), &workspace.variables)?;
        save_environments(&path.join(ENVIRONMENTS_DIR), &workspace.environments)?;

        for (collection, collection_path) in workspace.collections.iter().zip(collection_paths) {
            save_collection(&path.join(collection_path), collection)?;
        }

        Ok(())
    }
}

fn ensure_collection_paths(workspace: &Workspace, collection_paths: &mut Vec<String>) {
    if collection_paths.len() < workspace.collections.len() {
        collection_paths.resize(workspace.collections.len(), String::new());
    }

    for (index, collection) in workspace.collections.iter().enumerate() {
        if collection_paths[index].is_empty() {
            collection_paths[index] = format!("{COLLECTIONS_DIR}/{}", slugify(&collection.name));
        }
    }
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

    let mut used = HashSet::new();
    for environment in environments {
        let file_name = format!("{}.yml", unique_slug(&slugify(&environment.name), &mut used));
        write_yaml(&path.join(file_name), &EnvironmentFile::from(environment))?;
    }

    Ok(())
}

fn save_collection(path: &Path, collection: &Collection) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| error.to_string())?;

    write_yaml(&path.join(COLLECTION_FILE), &CollectionFile::from(collection))?;
    save_environments(&path.join(ENVIRONMENTS_DIR), &collection.environments)?;

    let mut used = HashSet::new();
    for request in &collection.requests {
        let file_name = format!("{}.yml", unique_slug(&slugify(&request.name), &mut used));
        save_request(&path.join(file_name), request)?;
    }

    for folder in &collection.folders {
        save_folder(path, folder)?;
    }

    Ok(())
}

fn save_folder(collection_path: &Path, folder: &CollectionFolder) -> Result<(), String> {
    let folder_path = collection_path.join(slugify(&folder.name));
    fs::create_dir_all(&folder_path).map_err(|error| error.to_string())?;
    write_yaml(&folder_path.join(FOLDER_FILE), &FolderFile::from(folder))?;

    let mut used = HashSet::new();
    for request in &folder.requests {
        let file_name = format!("{}.yml", unique_slug(&slugify(&request.name), &mut used));
        save_request(&folder_path.join(file_name), request)?;
    }

    Ok(())
}

fn save_request(path: &Path, request: &Request) -> Result<(), String> {
    write_yaml(path, &RequestFile::from(request))
}

fn discover_collections(workspace_path: &Path) -> Result<Vec<CollectionRef>, String> {
    let collections_root = workspace_path.join(COLLECTIONS_DIR);
    if !collections_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut refs = Vec::new();
    for entry in fs::read_dir(&collections_root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry.file_type().map_err(|error| error.to_string())?.is_dir() {
            continue;
        }

        let collection_path = entry.path();
        if collection_path.join(COLLECTION_FILE).is_file() {
            let relative = collection_path
                .strip_prefix(workspace_path)
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            refs.push(CollectionRef { path: relative });
        }
    }

    refs.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(refs)
}

fn load_collection(collection_path: &Path) -> Result<Collection, String> {
    let collection_file = read_yaml::<CollectionFile>(&collection_path.join(COLLECTION_FILE))?;

    let variables = collection_file
        .variables
        .into_iter()
        .map(Into::into)
        .collect::<Vec<Variable>>();

    let environments = load_environments(&collection_path.join(ENVIRONMENTS_DIR))?;

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
            folders.push(load_folder(&entry_path)?);
            continue;
        }

        if file_type.is_file() && entry_path.extension().is_some_and(|ext| ext == "yml") {
            requests.push(load_request(&entry_path)?);
        }
    }

    folders.sort_by(|left, right| left.name.cmp(&right.name));
    requests.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(collection_from_parts(
        collection_file.name,
        variables,
        environments,
        folders,
        requests,
    ))
}

fn load_folder(folder_path: &Path) -> Result<CollectionFolder, String> {
    let mut folder: CollectionFolder =
        read_yaml::<FolderFile>(&folder_path.join(FOLDER_FILE))?.into();

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
            folder.requests.push(load_request(&entry_path)?);
        }
    }

    folder.requests.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(folder)
}

fn load_request(path: &Path) -> Result<Request, String> {
    Ok(read_yaml::<RequestFile>(path)?.into())
}

fn load_environments(path: &Path) -> Result<Vec<Environment>, String> {
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
            environments.push(read_yaml::<EnvironmentFile>(&entry_path)?.into());
        }
    }

    environments.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(environments)
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

fn unique_slug(base: &str, used: &mut HashSet<String>) -> String {
    let base = if base.is_empty() { "untitled" } else { base };
    let mut slug = base.to_string();
    let mut counter = 2;

    while used.contains(&slug) {
        slug = format!("{base}-{counter}");
        counter += 1;
    }

    used.insert(slug.clone());
    slug
}

pub fn default_collection_paths(workspace: &Workspace) -> Vec<String> {
    workspace
        .collections
        .iter()
        .map(|collection| format!("{COLLECTIONS_DIR}/{}", slugify(&collection.name)))
        .collect()
}

pub fn default_workspace_collection_paths(workspaces: &[Workspace]) -> Vec<Vec<String>> {
    workspaces
        .iter()
        .map(default_collection_paths)
        .collect()
}
