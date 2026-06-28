use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Value as YamlValue;

use crate::domain::{Collection, CollectionFolder, Environment, Request, RequestProtocol};

use super::shared::{
    apply_oc_scripts, domain_body_to_oc, domain_headers_to_oc, domain_params_to_oc,
    domain_scripts_to_oc, domain_variables_to_oc, oc_body_to_domain, oc_headers_to_domain,
    oc_params_to_domain, oc_variables_to_domain, parse_http_method, push_warning, slugify_name,
    ImportResult, OcBody, OcKeyValue, OcMultipartPart, OcScript, OcVariable, OcVariableValue,
};

const ROOT_MANIFESTS: [&str; 2] = ["opencollection.yml", "bruno.yml"];

pub fn import_opencollection(path: &Path) -> Result<ImportResult, String> {
    if path.is_dir() {
        import_opencollection_dir(path)
    } else if path.is_file() {
        import_opencollection_file(path)
    } else {
        Err(format!("path not found: {}", path.display()))
    }
}

pub fn export_opencollection(collection: &Collection, path: &Path) -> Result<Vec<String>, String> {
    if path.exists() && !path.is_dir() {
        return Err(format!("export path is not a directory: {}", path.display()));
    }

    fs::create_dir_all(path).map_err(|error| error.to_string())?;
    let mut warnings = Vec::new();

    write_collection_manifest(path, collection)?;
    write_environments(path, collection)?;

    let mut used_names = HashSet::new();
    for request in &collection.requests {
        let file_name = unique_yaml_name(&request.name, &mut used_names);
        write_request_file(&path.join(format!("{file_name}.yml")), request, &mut warnings)?;
    }

    for folder in &collection.folders {
        export_folder(path, folder, &mut used_names, &mut warnings)?;
    }

    Ok(warnings)
}

fn import_opencollection_dir(path: &Path) -> Result<ImportResult, String> {
    let manifest_path = ROOT_MANIFESTS
        .iter()
        .map(|name| path.join(name))
        .find(|candidate| candidate.is_file());

    let mut warnings = Vec::new();
    let (name, collection_variables) = if let Some(manifest_path) = &manifest_path {
        let manifest = read_yaml_value(manifest_path)?;
        let name = manifest
            .get("info")
            .and_then(|info| info.get("name"))
            .and_then(YamlValue::as_str)
            .unwrap_or_else(|| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Imported Collection")
            })
            .to_string();
        let variables = manifest
            .get("request")
            .and_then(parse_request_defaults)
            .map(|defaults| defaults.variables)
            .unwrap_or_default();
        (name, variables)
    } else {
        push_warning(
            &mut warnings,
            "opencollection.yml not found; using folder name as collection name",
        );
        (
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Imported Collection")
                .to_string(),
            Vec::new(),
        )
    };

    let environments = load_environments(&path.join("environments"), &mut warnings);
    let (folders, requests) = parse_directory(path, &mut warnings)?;

    Ok(ImportResult {
        collection: Collection {
            id: crate::domain::EntityId::new(),
            name,
            expanded: true,
            variables: oc_variables_to_domain(&collection_variables, &mut warnings, "collection"),
            environments,
            folders,
            requests,
        },
        warnings,
    })
}

fn import_opencollection_file(path: &Path) -> Result<ImportResult, String> {
    let root = read_yaml_value(path)?;
    if root.get("items").is_some() {
        return import_bundled_collection(&root);
    }

    if root.get("info").is_some() {
        let mut warnings = Vec::new();
        let request = parse_request_document(&root, &mut warnings)?;
        let name = root
            .get("info")
            .and_then(|info| info.get("name"))
            .and_then(YamlValue::as_str)
            .unwrap_or("Imported Collection")
            .to_string();
        return Ok(ImportResult {
            collection: Collection {
                id: crate::domain::EntityId::new(),
                name,
                expanded: true,
                variables: crate::domain::default_variables(),
                environments: Vec::new(),
                folders: Vec::new(),
                requests: vec![request],
            },
            warnings,
        });
    }

    Err("unrecognized OpenCollection YAML file".into())
}

fn import_bundled_collection(root: &YamlValue) -> Result<ImportResult, String> {
    let mut warnings = Vec::new();
    let name = root
        .get("info")
        .and_then(|info| info.get("name"))
        .and_then(YamlValue::as_str)
        .unwrap_or("Imported Collection")
        .to_string();

    let collection_variables = root
        .get("request")
        .and_then(parse_request_defaults)
        .map(|defaults| defaults.variables)
        .unwrap_or_default();

    let environments = root
        .get("config")
        .and_then(|config| config.get("environments"))
        .and_then(YamlValue::as_sequence)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| parse_environment(entry, &mut warnings))
                .collect()
        })
        .unwrap_or_default();

    let mut folders = Vec::new();
    let mut requests = Vec::new();
    if let Some(items) = root.get("items").and_then(YamlValue::as_sequence) {
        for item in items {
            parse_bundled_item(item, &mut folders, &mut requests, &mut warnings);
        }
    }

    Ok(ImportResult {
        collection: Collection {
            id: crate::domain::EntityId::new(),
            name,
            expanded: true,
            variables: oc_variables_to_domain(&collection_variables, &mut warnings, "collection"),
            environments,
            folders,
            requests,
        },
        warnings,
    })
}

fn parse_bundled_item(
    item: &YamlValue,
    folders: &mut Vec<CollectionFolder>,
    requests: &mut Vec<Request>,
    warnings: &mut Vec<String>,
) {
    let item_type = item
        .get("info")
        .and_then(|info| info.get("type"))
        .and_then(YamlValue::as_str);

    match item_type {
        Some("folder") => {
            let name = item
                .get("info")
                .and_then(|info| info.get("name"))
                .and_then(YamlValue::as_str)
                .unwrap_or("Folder")
                .to_string();
            let variables = item
                .get("request")
                .and_then(parse_request_defaults)
                .map(|defaults| defaults.variables)
                .unwrap_or_default();
            let mut folder_requests = Vec::new();
            let mut nested_folders = Vec::new();
            if let Some(items) = item.get("items").and_then(YamlValue::as_sequence) {
                for child in items {
                    parse_bundled_item(child, &mut nested_folders, &mut folder_requests, warnings);
                }
            }
            if !nested_folders.is_empty() {
                push_warning(
                    warnings,
                    format!("nested folders inside `{name}` were flattened into the collection root"),
                );
                folders.extend(nested_folders);
            }
            folders.push(CollectionFolder {
                id: crate::domain::EntityId::new(),
                name,
                expanded: true,
                variables: oc_variables_to_domain(&variables, warnings, "folder"),
                requests: folder_requests,
            });
        }
        Some("http") => match parse_request_document(item, warnings) {
            Ok(request) => requests.push(request),
            Err(error) => push_warning(warnings, error),
        },
        Some(other) => push_warning(warnings, format!("skipped bundled item type `{other}`")),
        None => push_warning(warnings, "skipped bundled item without info.type"),
    }
}

fn parse_directory(
    dir: &Path,
    warnings: &mut Vec<String>,
) -> Result<(Vec<CollectionFolder>, Vec<Request>), String> {
    let mut requests = Vec::new();
    let mut folders = Vec::new();

    let mut entries = fs::read_dir(dir)
        .map_err(|error| format!("failed to read {}: {error}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in &entries {
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_file() {
            if !file_name.ends_with(".yml") && !file_name.ends_with(".yaml") {
                continue;
            }
            if is_reserved_file(&file_name) {
                continue;
            }
            if dir.join("folder.yml").is_file() && file_name == "folder.yml" {
                continue;
            }
            let doc = read_yaml_value(&entry.path())?;
            let item_type = doc
                .get("info")
                .and_then(|info| info.get("type"))
                .and_then(YamlValue::as_str);
            if item_type == Some("folder") {
                continue;
            }
            if item_type == Some("http") {
                match parse_request_document(&doc, warnings) {
                    Ok(request) => requests.push(request),
                    Err(error) => push_warning(warnings, error),
                }
            } else if let Some(other) = item_type {
                push_warning(
                    warnings,
                    format!("skipped `{}` with type `{other}`", entry.path().display()),
                );
            }
        }
    }

    for entry in entries {
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if !file_type.is_dir() {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().into_owned();
        if dir_name == "environments" {
            continue;
        }
        let child_path = entry.path();
        let folder_manifest = child_path.join("folder.yml");
        if !folder_manifest.is_file() {
            push_warning(
                warnings,
                format!(
                    "skipped directory `{}` without folder.yml",
                    child_path.display()
                ),
            );
            continue;
        }
        let doc = read_yaml_value(&folder_manifest)?;
        let name = doc
            .get("info")
            .and_then(|info| info.get("name"))
            .and_then(YamlValue::as_str)
            .unwrap_or(&dir_name)
            .to_string();
        let variables = doc
            .get("request")
            .and_then(parse_request_defaults)
            .map(|defaults| defaults.variables)
            .unwrap_or_default();
        let (nested_folders, nested_requests) = parse_directory(&child_path, warnings)?;
        if !nested_folders.is_empty() {
            push_warning(
                warnings,
                format!("nested folders inside `{name}` were flattened into the collection root"),
            );
            folders.extend(nested_folders);
        }
        folders.push(CollectionFolder {
            id: crate::domain::EntityId::new(),
            name,
            expanded: true,
            variables: oc_variables_to_domain(&variables, warnings, "folder"),
            requests: nested_requests,
        });
    }

    Ok((folders, requests))
}

fn parse_request_document(doc: &YamlValue, warnings: &mut Vec<String>) -> Result<Request, String> {
    let info = doc.get("info").ok_or("request is missing info section")?;
    let item_type = info
        .get("type")
        .and_then(YamlValue::as_str)
        .ok_or("request is missing info.type")?;
    if item_type != "http" {
        return Err(format!("unsupported request type `{item_type}`"));
    }

    let name = info
        .get("name")
        .and_then(YamlValue::as_str)
        .unwrap_or("Imported Request")
        .to_string();

    let http = doc
        .get("http")
        .ok_or("HTTP request is missing http section")?;

    if http.get("auth").is_some() {
        push_warning(warnings, format!("auth settings in `{name}` were not imported"));
    }

    let method = http
        .get("method")
        .and_then(YamlValue::as_str)
        .unwrap_or("GET");
    let url = http
        .get("url")
        .and_then(YamlValue::as_str)
        .unwrap_or_default()
        .to_string();

    let headers = http
        .get("headers")
        .and_then(parse_key_values)
        .unwrap_or_default();
    let params = http
        .get("params")
        .and_then(parse_key_values)
        .unwrap_or_default();
    let body = http
        .get("body")
        .map(parse_body)
        .transpose()?
        .unwrap_or(OcBody::None);

    let runtime_variables = doc
        .get("runtime")
        .and_then(parse_request_defaults)
        .map(|defaults| defaults.variables)
        .unwrap_or_default();
    let scripts = doc
        .get("runtime")
        .and_then(|runtime| runtime.get("scripts"))
        .and_then(parse_scripts)
        .unwrap_or_default();

    let (body_type, raw_body, form_fields, multipart_fields) =
        oc_body_to_domain(&body, warnings);

    let mut request = Request {
        id: crate::domain::EntityId::new(),
        name,
        protocol: RequestProtocol::Http,
        method: parse_http_method(method, warnings, "request"),
        url,
        query_params: oc_params_to_domain(&params, warnings),
        headers: oc_headers_to_domain(&headers, warnings, "request"),
        body_type,
        body: raw_body,
        form_fields,
        multipart_fields,
        variables: oc_variables_to_domain(&runtime_variables, warnings, "request"),
        pre_request_script: String::new(),
        post_response_script: String::new(),
        tests_script: String::new(),
    };
    apply_oc_scripts(&mut request, &scripts, warnings);
    Ok(request)
}

struct RequestDefaults {
    variables: Vec<OcVariable>,
}

fn parse_request_defaults(value: &YamlValue) -> Option<RequestDefaults> {
    Some(RequestDefaults {
        variables: value
            .get("variables")
            .and_then(parse_variables)
            .unwrap_or_default(),
    })
}

fn parse_environment(value: &YamlValue, warnings: &mut Vec<String>) -> Option<Environment> {
    let name = value.get("name").and_then(YamlValue::as_str)?;
    let variables = value
        .get("variables")
        .and_then(parse_variables)
        .unwrap_or_default();
    Some(Environment {
        id: crate::domain::EntityId::new(),
        name: name.to_string(),
        variables: oc_variables_to_domain(&variables, warnings, "environment"),
    })
}

fn load_environments(path: &Path, warnings: &mut Vec<String>) -> Vec<Environment> {
    if !path.is_dir() {
        return Vec::new();
    }

    let mut environments = Vec::new();
    let mut entries = fs::read_dir(path)
        .map_err(|error| {
            push_warning(
                warnings,
                format!("failed to read environments in {}: {error}", path.display()),
            );
        })
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
        if !(file_name.ends_with(".yml") || file_name.ends_with(".yaml")) {
            continue;
        }
        match read_yaml_value(&path)
            .ok()
            .and_then(|doc| parse_environment(&doc, warnings))
        {
            Some(environment) => environments.push(environment),
            None => push_warning(warnings, format!("skipped environment file {}", path.display())),
        }
    }

    environments
}

fn parse_variables(value: &YamlValue) -> Option<Vec<OcVariable>> {
    let sequence = value.as_sequence()?;
    Some(
        sequence
            .iter()
            .filter_map(parse_variable)
            .collect::<Vec<_>>(),
    )
}

fn parse_variable(value: &YamlValue) -> Option<OcVariable> {
    let name = value.get("name").and_then(YamlValue::as_str)?.to_string();
    let disabled = value
        .get("disabled")
        .and_then(YamlValue::as_bool)
        .unwrap_or(false);
    let variable_value = if let Some(text) = value.get("value").and_then(YamlValue::as_str) {
        OcVariableValue::Plain(text.to_string())
    } else if let Some(typed) = value.get("value") {
        parse_typed_variable_value(typed)?
    } else {
        OcVariableValue::Plain(String::new())
    };
    Some(OcVariable {
        name,
        value: variable_value,
        disabled,
    })
}

fn parse_typed_variable_value(value: &YamlValue) -> Option<OcVariableValue> {
    let var_type = value.get("type").and_then(YamlValue::as_str)?.to_string();
    let data = value
        .get("data")
        .and_then(YamlValue::as_str)
        .unwrap_or_default()
        .to_string();
    Some(OcVariableValue::Typed { var_type, data })
}

fn parse_key_values(value: &YamlValue) -> Option<Vec<OcKeyValue>> {
    let sequence = value.as_sequence()?;
    Some(
        sequence
            .iter()
            .filter_map(|entry| {
                let name = entry.get("name").and_then(YamlValue::as_str)?.to_string();
                Some(OcKeyValue {
                    name,
                    value: entry
                        .get("value")
                        .and_then(YamlValue::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    disabled: entry
                        .get("disabled")
                        .and_then(YamlValue::as_bool)
                        .unwrap_or(false),
                    param_type: entry
                        .get("type")
                        .and_then(YamlValue::as_str)
                        .map(str::to_string),
                })
            })
            .collect(),
    )
}

fn parse_scripts(value: &YamlValue) -> Option<Vec<OcScript>> {
    let sequence = value.as_sequence()?;
    Some(
        sequence
            .iter()
            .filter_map(|entry| {
                Some(OcScript {
                    script_type: entry.get("type").and_then(YamlValue::as_str)?.to_string(),
                    code: entry
                        .get("code")
                        .and_then(YamlValue::as_str)
                        .unwrap_or_default()
                        .to_string(),
                })
            })
            .collect(),
    )
}

fn parse_body(value: &YamlValue) -> Result<OcBody, String> {
    let body_type = value
        .get("type")
        .and_then(YamlValue::as_str)
        .ok_or("body is missing type")?;
    match body_type {
        "json" | "xml" | "text" | "sparql" => Ok(OcBody::Raw {
            body_type: body_type.to_string(),
            data: value
                .get("data")
                .and_then(YamlValue::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        "form-urlencoded" => {
            let fields = value
                .get("data")
                .and_then(parse_key_values)
                .unwrap_or_default();
            Ok(OcBody::FormUrlEncoded(fields))
        }
        "multipart-form" => {
            let parts = value
                .get("data")
                .and_then(|data| data.as_sequence())
                .map(|sequence| {
                    sequence
                        .iter()
                        .filter_map(|entry| {
                            Some(OcMultipartPart {
                                name: entry.get("name").and_then(YamlValue::as_str)?.to_string(),
                                part_type: entry
                                    .get("type")
                                    .and_then(YamlValue::as_str)
                                    .unwrap_or("text")
                                    .to_string(),
                                value: entry
                                    .get("value")
                                    .and_then(value_to_string)
                                    .unwrap_or_default(),
                                content_type: entry
                                    .get("contentType")
                                    .and_then(YamlValue::as_str)
                                    .unwrap_or_default()
                                    .to_string(),
                                disabled: entry
                                    .get("disabled")
                                    .and_then(YamlValue::as_bool)
                                    .unwrap_or(false),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Ok(OcBody::Multipart(parts))
        }
        other => Err(format!("unsupported body type `{other}`")),
    }
}

fn value_to_string(value: &YamlValue) -> Option<String> {
    match value {
        YamlValue::String(text) => Some(text.clone()),
        YamlValue::Sequence(items) => Some(
            items
                .iter()
                .filter_map(YamlValue::as_str)
                .collect::<Vec<_>>()
                .join(","),
        ),
        YamlValue::Number(number) => Some(number.to_string()),
        YamlValue::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn write_collection_manifest(path: &Path, collection: &Collection) -> Result<(), String> {
    let variables = domain_variables_to_oc(&collection.variables);
    let mut manifest = serde_yaml::Mapping::new();
    manifest.insert(
        YamlValue::String("opencollection".into()),
        YamlValue::String("1.0.0".into()),
    );
    let mut info = serde_yaml::Mapping::new();
    info.insert(
        YamlValue::String("name".into()),
        YamlValue::String(collection.name.clone()),
    );
    manifest.insert(YamlValue::String("info".into()), YamlValue::Mapping(info));
    if !variables.is_empty() {
        manifest.insert(
            YamlValue::String("request".into()),
            YamlValue::Mapping(request_defaults_mapping(&variables)),
        );
    }
    write_yaml(path.join("opencollection.yml"), &YamlValue::Mapping(manifest))
}

fn write_environments(path: &Path, collection: &Collection) -> Result<(), String> {
    if collection.environments.is_empty() {
        return Ok(());
    }
    let env_dir = path.join("environments");
    fs::create_dir_all(&env_dir).map_err(|error| error.to_string())?;
    let mut used_names = HashSet::new();
    for environment in &collection.environments {
        let file_name = unique_yaml_name(&environment.name, &mut used_names);
        let mut mapping = serde_yaml::Mapping::new();
        mapping.insert(
            YamlValue::String("name".into()),
            YamlValue::String(environment.name.clone()),
        );
        mapping.insert(
            YamlValue::String("variables".into()),
            YamlValue::Sequence(variables_to_yaml(&domain_variables_to_oc(
                &environment.variables,
            ))),
        );
        write_yaml(env_dir.join(format!("{file_name}.yml")), &YamlValue::Mapping(mapping))?;
    }
    Ok(())
}

fn export_folder(
    root: &Path,
    folder: &CollectionFolder,
    used_names: &mut HashSet<String>,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let dir_name = unique_dir_name(&folder.name, root, used_names);
    let folder_path = root.join(&dir_name);
    fs::create_dir_all(&folder_path).map_err(|error| error.to_string())?;

    let mut folder_manifest = serde_yaml::Mapping::new();
    let mut info = serde_yaml::Mapping::new();
    info.insert(
        YamlValue::String("name".into()),
        YamlValue::String(folder.name.clone()),
    );
    info.insert(
        YamlValue::String("type".into()),
        YamlValue::String("folder".into()),
    );
    folder_manifest.insert(YamlValue::String("info".into()), YamlValue::Mapping(info));
    let variables = domain_variables_to_oc(&folder.variables);
    if !variables.is_empty() {
        folder_manifest.insert(
            YamlValue::String("request".into()),
            YamlValue::Mapping(request_defaults_mapping(&variables)),
        );
    }
    write_yaml(
        folder_path.join("folder.yml"),
        &YamlValue::Mapping(folder_manifest),
    )?;

    let mut used_request_names = HashSet::new();
    for request in &folder.requests {
        let file_name = unique_yaml_name(&request.name, &mut used_request_names);
        write_request_file(
            &folder_path.join(format!("{file_name}.yml")),
            request,
            warnings,
        )?;
    }
    Ok(())
}

fn write_request_file(
    path: &Path,
    request: &Request,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    if request.protocol != RequestProtocol::Http {
        push_warning(
            warnings,
            format!("skipped non-HTTP request `{}` during export", request.name),
        );
        return Ok(());
    }

    let mut mapping = serde_yaml::Mapping::new();
    let mut info = serde_yaml::Mapping::new();
    info.insert(
        YamlValue::String("name".into()),
        YamlValue::String(request.name.clone()),
    );
    info.insert(
        YamlValue::String("type".into()),
        YamlValue::String("http".into()),
    );
    mapping.insert(YamlValue::String("info".into()), YamlValue::Mapping(info));

    let mut http = serde_yaml::Mapping::new();
    http.insert(
        YamlValue::String("method".into()),
        YamlValue::String(request.method.as_str().into()),
    );
    http.insert(
        YamlValue::String("url".into()),
        YamlValue::String(request.url.clone()),
    );
    let headers = domain_headers_to_oc(&request.headers);
    if !headers.is_empty() {
        http.insert(
            YamlValue::String("headers".into()),
            YamlValue::Sequence(key_values_to_yaml(&headers)),
        );
    }
    let params = domain_params_to_oc(&request.query_params);
    if !params.is_empty() {
        http.insert(
            YamlValue::String("params".into()),
            YamlValue::Sequence(key_values_to_yaml(&params)),
        );
    }
    if let Some(body) = body_to_yaml(&domain_body_to_oc(
        request.body_type,
        &request.body,
        &request.form_fields,
        &request.multipart_fields,
    )) {
        http.insert(YamlValue::String("body".into()), body);
    }
    mapping.insert(YamlValue::String("http".into()), YamlValue::Mapping(http));

    let variables = domain_variables_to_oc(&request.variables);
    let scripts = domain_scripts_to_oc(request);
    if !variables.is_empty() || !scripts.is_empty() {
        let mut runtime = serde_yaml::Mapping::new();
        if !variables.is_empty() {
            runtime.insert(
                YamlValue::String("variables".into()),
                YamlValue::Sequence(variables_to_yaml(&variables)),
            );
        }
        if !scripts.is_empty() {
            runtime.insert(
                YamlValue::String("scripts".into()),
                YamlValue::Sequence(scripts_to_yaml(&scripts)),
            );
        }
        mapping.insert(
            YamlValue::String("runtime".into()),
            YamlValue::Mapping(runtime),
        );
    }

    let mut settings = serde_yaml::Mapping::new();
    settings.insert(
        YamlValue::String("encodeUrl".into()),
        YamlValue::Bool(true),
    );
    settings.insert(
        YamlValue::String("timeout".into()),
        YamlValue::Number(0.into()),
    );
    mapping.insert(
        YamlValue::String("settings".into()),
        YamlValue::Mapping(settings),
    );

    write_yaml(path.to_path_buf(), &YamlValue::Mapping(mapping))
}

fn request_defaults_mapping(variables: &[OcVariable]) -> serde_yaml::Mapping {
    let mut mapping = serde_yaml::Mapping::new();
    mapping.insert(
        YamlValue::String("variables".into()),
        YamlValue::Sequence(variables_to_yaml(variables)),
    );
    mapping
}

fn variables_to_yaml(variables: &[OcVariable]) -> Vec<YamlValue> {
    variables
        .iter()
        .map(|variable| {
            let mut mapping = serde_yaml::Mapping::new();
            mapping.insert(
                YamlValue::String("name".into()),
                YamlValue::String(variable.name.clone()),
            );
            mapping.insert(
                YamlValue::String("value".into()),
                match &variable.value {
                    OcVariableValue::Plain(text) => YamlValue::String(text.clone()),
                    OcVariableValue::Typed { var_type, data } => {
                        let mut typed = serde_yaml::Mapping::new();
                        typed.insert(
                            YamlValue::String("type".into()),
                            YamlValue::String(var_type.clone()),
                        );
                        typed.insert(
                            YamlValue::String("data".into()),
                            YamlValue::String(data.clone()),
                        );
                        YamlValue::Mapping(typed)
                    }
                },
            );
            YamlValue::Mapping(mapping)
        })
        .collect()
}

fn key_values_to_yaml(values: &[OcKeyValue]) -> Vec<YamlValue> {
    values
        .iter()
        .map(|value| {
            let mut mapping = serde_yaml::Mapping::new();
            mapping.insert(
                YamlValue::String("name".into()),
                YamlValue::String(value.name.clone()),
            );
            mapping.insert(
                YamlValue::String("value".into()),
                YamlValue::String(value.value.clone()),
            );
            if value.disabled {
                mapping.insert(YamlValue::String("disabled".into()), YamlValue::Bool(true));
            }
            if let Some(param_type) = &value.param_type {
                mapping.insert(
                    YamlValue::String("type".into()),
                    YamlValue::String(param_type.clone()),
                );
            }
            YamlValue::Mapping(mapping)
        })
        .collect()
}

fn scripts_to_yaml(scripts: &[OcScript]) -> Vec<YamlValue> {
    scripts
        .iter()
        .map(|script| {
            let mut mapping = serde_yaml::Mapping::new();
            mapping.insert(
                YamlValue::String("type".into()),
                YamlValue::String(script.script_type.clone()),
            );
            mapping.insert(
                YamlValue::String("code".into()),
                YamlValue::String(script.code.clone()),
            );
            YamlValue::Mapping(mapping)
        })
        .collect()
}

fn body_to_yaml(body: &OcBody) -> Option<YamlValue> {
    match body {
        OcBody::None => None,
        OcBody::Raw { body_type, data } => {
            let mut mapping = serde_yaml::Mapping::new();
            mapping.insert(
                YamlValue::String("type".into()),
                YamlValue::String(body_type.clone()),
            );
            mapping.insert(
                YamlValue::String("data".into()),
                YamlValue::String(data.clone()),
            );
            Some(YamlValue::Mapping(mapping))
        }
        OcBody::FormUrlEncoded(fields) => {
            let mut mapping = serde_yaml::Mapping::new();
            mapping.insert(
                YamlValue::String("type".into()),
                YamlValue::String("form-urlencoded".into()),
            );
            mapping.insert(
                YamlValue::String("data".into()),
                YamlValue::Sequence(key_values_to_yaml(fields)),
            );
            Some(YamlValue::Mapping(mapping))
        }
        OcBody::Multipart(parts) => {
            let mut mapping = serde_yaml::Mapping::new();
            mapping.insert(
                YamlValue::String("type".into()),
                YamlValue::String("multipart-form".into()),
            );
            mapping.insert(
                YamlValue::String("data".into()),
                YamlValue::Sequence(
                    parts
                        .iter()
                        .map(|part| {
                            let mut part_mapping = serde_yaml::Mapping::new();
                            part_mapping.insert(
                                YamlValue::String("name".into()),
                                YamlValue::String(part.name.clone()),
                            );
                            part_mapping.insert(
                                YamlValue::String("type".into()),
                                YamlValue::String(part.part_type.clone()),
                            );
                            part_mapping.insert(
                                YamlValue::String("value".into()),
                                YamlValue::String(part.value.clone()),
                            );
                            if !part.content_type.is_empty() {
                                part_mapping.insert(
                                    YamlValue::String("contentType".into()),
                                    YamlValue::String(part.content_type.clone()),
                                );
                            }
                            if part.disabled {
                                part_mapping.insert(
                                    YamlValue::String("disabled".into()),
                                    YamlValue::Bool(true),
                                );
                            }
                            YamlValue::Mapping(part_mapping)
                        })
                        .collect(),
                ),
            );
            Some(YamlValue::Mapping(mapping))
        }
    }
}

fn unique_yaml_name(name: &str, used: &mut HashSet<String>) -> String {
    let base = slugify_name(name);
    let mut candidate = base.clone();
    let mut index = 2;
    while used.contains(&candidate) {
        candidate = format!("{base}-{index}");
        index += 1;
    }
    used.insert(candidate.clone());
    candidate
}

fn unique_dir_name(name: &str, root: &Path, used: &mut HashSet<String>) -> String {
    let base = slugify_name(name);
    let mut candidate = base.clone();
    let mut index = 2;
    while used.contains(&candidate) || root.join(&candidate).exists() {
        candidate = format!("{base}-{index}");
        index += 1;
    }
    used.insert(candidate.clone());
    candidate
}

fn is_reserved_file(file_name: &str) -> bool {
    ROOT_MANIFESTS.contains(&file_name)
}

fn read_yaml_value(path: &Path) -> Result<YamlValue, String> {
    let content = fs::read_to_string(path).map_err(|error| {
        format!("failed to read {}: {error}", path.display())
    })?;
    serde_yaml::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_yaml(path: PathBuf, value: &YamlValue) -> Result<(), String> {
    let content = serde_yaml::to_string(value).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_minimal_opencollection_directory() {
        let temp = std::env::temp_dir().join(format!(
            "loom-oc-import-{}",
            crate::domain::EntityId::new()
        ));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("users")).expect("mkdir");
        fs::write(
            temp.join("opencollection.yml"),
            r#"opencollection: "1.0.0"
info:
  name: Demo API
request:
  variables:
    - name: baseUrl
      value: https://example.com
"#,
        )
        .expect("write manifest");
        fs::write(
            temp.join("users/folder.yml"),
            r#"info:
  name: Users
  type: folder
"#,
        )
        .expect("write folder");
        fs::write(
            temp.join("users/list.yml"),
            r#"info:
  name: List Users
  type: http
http:
  method: GET
  url: "{{baseUrl}}/users"
  params:
    - name: page
      value: "1"
      type: query
"#,
        )
        .expect("write request");

        let imported = import_opencollection(&temp).expect("import");
        assert_eq!(imported.collection.name, "Demo API");
        assert_eq!(imported.collection.folders.len(), 1);
        assert_eq!(imported.collection.folders[0].requests.len(), 1);
        assert_eq!(
            imported.collection.folders[0].requests[0].query_params[0].name,
            "page"
        );

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn round_trips_exported_collection() {
        let mut collection = Collection::new("Sample");
        let mut request = Request::new("Ping");
        request.url = "https://example.com".into();
        request.method = crate::domain::HttpMethod::Get;
        collection.requests.push(request);

        let temp = std::env::temp_dir().join(format!(
            "loom-oc-export-{}",
            crate::domain::EntityId::new()
        ));
        let _ = fs::remove_dir_all(&temp);
        export_opencollection(&collection, &temp).expect("export");
        let imported = import_opencollection(&temp).expect("import");
        assert_eq!(imported.collection.name, "Sample");
        assert_eq!(imported.collection.requests.len(), 1);
        assert_eq!(imported.collection.requests[0].url, "https://example.com");
        let _ = fs::remove_dir_all(temp);
    }
}
