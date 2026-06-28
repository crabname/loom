use std::collections::HashMap;

use serde_json::Value;

use super::{FormField, KeyValueField, MultipartField, Request};

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub value: Value,
}

impl Variable {
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            value: Value::String(String::new()),
        }
    }

    pub fn from_strings(name: String, value: String) -> Self {
        Self {
            name,
            value: Value::String(value),
        }
    }

    pub fn display_value(&self) -> String {
        match &self.value {
            Value::String(text) => text.clone(),
            Value::Null => String::new(),
            other => other.to_string(),
        }
    }
}

pub fn default_variables() -> Vec<Variable> {
    vec![Variable::empty()]
}

/// Variable layers from lowest to highest priority (Bruno-style, excluding runtime/script vars).
#[derive(Debug, Clone, Default)]
pub struct VariableLayers<'a> {
    pub global: &'a [Variable],
    pub collection: &'a [Variable],
    pub environment: Option<&'a [Variable]>,
    /// Folder-level variables; empty until folder hierarchy exists.
    pub folder: &'a [Variable],
    pub request: &'a [Variable],
}

pub fn build_variable_pool(layers: VariableLayers<'_>) -> HashMap<String, String> {
    let mut pool = HashMap::new();

    for layer in [
        layers.global,
        layers.collection,
        layers.environment.unwrap_or(&[]),
        layers.folder,
        layers.request,
    ] {
        merge_variables(&mut pool, layer);
    }

    pool
}

fn merge_variables(pool: &mut HashMap<String, String>, variables: &[Variable]) {
    for variable in variables {
        let name = variable.name.trim();
        if name.is_empty() {
            continue;
        }
        pool.insert(name.to_string(), variable.display_value());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSpan {
    pub start: usize,
    pub end: usize,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableSource {
    Unresolved,
    WorkspaceVariable,
    CollectionVariable,
    WorkspaceEnvironment,
    CollectionEnvironment,
    FolderVariable,
    RequestVariable,
    Runtime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedVariable {
    pub name: String,
    pub value: Option<String>,
    pub source: VariableSource,
}

#[derive(Debug, Clone, Default)]
pub struct VariableResolveLabels {
    pub collection_name: Option<String>,
    pub folder_name: Option<String>,
    pub workspace_environment_name: Option<String>,
    pub collection_environment_name: Option<String>,
}

pub fn variable_at_offset(text: &str, offset: usize) -> Option<VariableSpan> {
    if offset > text.len() {
        return None;
    }

    let mut probes = vec![offset];
    if offset > 0 {
        probes.push(offset - 1);
    }
    if offset < text.len() {
        probes.push(offset + 1);
    }

    for probe in probes {
        if let Some(span) = variable_at_exact_offset(text, probe) {
            return Some(span);
        }
    }

    None
}

fn variable_at_exact_offset(text: &str, offset: usize) -> Option<VariableSpan> {
    if offset > text.len() {
        return None;
    }

    let mut search_from = 0;
    while let Some(relative_start) = text[search_from..].find("{{") {
        let start = search_from + relative_start;
        let inner_start = start + 2;
        let Some(relative_end) = text[inner_start..].find("}}") else {
            break;
        };
        let inner_end = inner_start + relative_end;
        let end = inner_end + 2;
        let name = text[inner_start..inner_end].trim().to_string();
        if (start..end).contains(&offset) && !name.is_empty() {
            return Some(VariableSpan { start, end, name });
        }
        search_from = end;
    }

    None
}

pub fn resolve_variable_source(
    name: &str,
    layers: VariableLayers<'_>,
    runtime: &HashMap<String, serde_json::Value>,
    labels: &VariableResolveLabels,
) -> ResolvedVariable {
    let name = name.trim();
    if name.is_empty() {
        return ResolvedVariable {
            name: String::new(),
            value: None,
            source: VariableSource::Unresolved,
        };
    }

    if runtime.contains_key(name) {
        return ResolvedVariable {
            name: name.to_string(),
            value: Some(runtime_value_to_string(&runtime[name])),
            source: VariableSource::Runtime,
        };
    }

    if let Some(value) = variable_value_in_slice(name, layers.request) {
        return ResolvedVariable {
            name: name.to_string(),
            value: Some(value),
            source: VariableSource::RequestVariable,
        };
    }

    if let Some(value) = variable_value_in_slice(name, layers.folder) {
        return ResolvedVariable {
            name: name.to_string(),
            value: Some(value),
            source: VariableSource::FolderVariable,
        };
    }

    if let Some(environment) = layers.environment
        && let Some(value) = variable_value_in_slice(name, environment)
    {
        let source = if labels.collection_environment_name.is_some() {
            VariableSource::CollectionEnvironment
        } else {
            VariableSource::WorkspaceEnvironment
        };
        return ResolvedVariable {
            name: name.to_string(),
            value: Some(value),
            source,
        };
    }

    if let Some(value) = variable_value_in_slice(name, layers.collection) {
        return ResolvedVariable {
            name: name.to_string(),
            value: Some(value),
            source: VariableSource::CollectionVariable,
        };
    }

    if let Some(value) = variable_value_in_slice(name, layers.global) {
        return ResolvedVariable {
            name: name.to_string(),
            value: Some(value),
            source: VariableSource::WorkspaceVariable,
        };
    }

    ResolvedVariable {
        name: name.to_string(),
        value: None,
        source: VariableSource::Unresolved,
    }
}

fn variable_value_in_slice(name: &str, variables: &[Variable]) -> Option<String> {
    variables
        .iter()
        .find(|variable| variable.name.trim() == name)
        .map(|variable| variable.display_value())
        .filter(|value| !value.is_empty())
}

pub fn format_variable_hover(resolved: &ResolvedVariable, labels: &VariableResolveLabels) -> String {
    let value_line = resolved
        .value
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|value| format!("`{value}`"))
        .unwrap_or_else(|| "*(unresolved)*".to_string());

    let source_line = match resolved.source {
        VariableSource::Unresolved => "Not defined in any scope".to_string(),
        VariableSource::WorkspaceVariable => "Workspace variable".to_string(),
        VariableSource::CollectionVariable => labels_line(
            "Collection variable",
            labels.collection_name.as_deref(),
        ),
        VariableSource::WorkspaceEnvironment => labels_line(
            "Workspace environment",
            labels.workspace_environment_name.as_deref(),
        ),
        VariableSource::CollectionEnvironment => labels_line(
            "Collection environment",
            labels
                .collection_environment_name
                .as_deref()
                .or(labels.collection_name.as_deref()),
        ),
        VariableSource::FolderVariable => {
            labels_line("Folder variable", labels.folder_name.as_deref())
        }
        VariableSource::RequestVariable => "Request variable".to_string(),
        VariableSource::Runtime => "Runtime (script)".to_string(),
    };

    format!("**{}**\n\n{value_line}\n\n{source_line}", resolved.name)
}

fn labels_line(prefix: &str, detail: Option<&str>) -> String {
    match detail {
        Some(detail) => format!("{prefix} · {detail}"),
        None => prefix.to_string(),
    }
}

fn runtime_value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub fn substitute_variables(text: &str, pool: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);
        rest = &rest[start + 2..];

        if let Some(end) = rest.find("}}") {
            let name = rest[..end].trim();
            if let Some(value) = pool.get(name) {
                result.push_str(value);
            } else {
                result.push_str("{{");
                result.push_str(name);
                result.push_str("}}");
            }
            rest = &rest[end + 2..];
        } else {
            result.push_str("{{");
            break;
        }
    }

    result.push_str(rest);
    result
}

pub fn substitute_key_value_fields(
    fields: &[KeyValueField],
    pool: &HashMap<String, String>,
) -> Vec<KeyValueField> {
    fields
        .iter()
        .map(|field| KeyValueField {
            enabled: field.enabled,
            name: substitute_variables(&field.name, pool),
            value: substitute_variables(&field.value, pool),
        })
        .collect()
}

pub fn substitute_form_fields(
    fields: &[FormField],
    pool: &HashMap<String, String>,
) -> Vec<FormField> {
    fields
        .iter()
        .map(|field| FormField {
            enabled: field.enabled,
            name: substitute_variables(&field.name, pool),
            value: substitute_variables(&field.value, pool),
        })
        .collect()
}

pub fn substitute_multipart_fields(
    fields: &[MultipartField],
    pool: &HashMap<String, String>,
) -> Vec<MultipartField> {
    fields
        .iter()
        .map(|field| MultipartField {
            enabled: field.enabled,
            name: substitute_variables(&field.name, pool),
            value: substitute_variables(&field.value, pool),
            field_type: field.field_type,
            content_type: substitute_variables(&field.content_type, pool),
        })
        .collect()
}

pub fn substitute_request(request: &Request, pool: &HashMap<String, String>) -> Request {
    Request {
        id: request.id,
        name: request.name.clone(),
        protocol: request.protocol,
        method: request.method,
        url: substitute_variables(&request.url, pool),
        query_params: substitute_key_value_fields(&request.query_params, pool),
        headers: substitute_key_value_fields(&request.headers, pool),
        body_type: request.body_type,
        body: substitute_variables(&request.body, pool),
        form_fields: substitute_form_fields(&request.form_fields, pool),
        multipart_fields: substitute_multipart_fields(&request.multipart_fields, pool),
        variables: request.variables.clone(),
        pre_request_script: request.pre_request_script.clone(),
        post_response_script: request.post_response_script.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_priority_order() {
        let global = vec![Variable::from_strings("key".into(), "global".into())];
        let collection = vec![Variable::from_strings("key".into(), "collection".into())];
        let request = vec![Variable::from_strings("key".into(), "request".into())];

        let pool = build_variable_pool(VariableLayers {
            global: &global,
            collection: &collection,
            environment: None,
            folder: &[],
            request: &request,
        });

        assert_eq!(pool.get("key").map(String::as_str), Some("request"));
    }

    #[test]
    fn substitutes_known_variables() {
        let mut pool = HashMap::new();
        pool.insert("baseUrl".into(), "https://api.example.com".into());

        assert_eq!(
            substitute_variables("{{baseUrl}}/users", &pool),
            "https://api.example.com/users"
        );
    }

    #[test]
    fn leaves_unknown_variables_literal() {
        let pool = HashMap::new();
        assert_eq!(
            substitute_variables("{{missing}}/users", &pool),
            "{{missing}}/users"
        );
    }

    #[test]
    fn finds_variable_at_offset() {
        let text = "https://{{baseUrl}}/users/{{userId}}";
        let span = variable_at_offset(text, 10).expect("baseUrl");
        assert_eq!(span.name, "baseUrl");
        assert_eq!(&text[span.start..span.end], "{{baseUrl}}");
    }

    #[test]
    fn finds_variable_at_adjacent_offset() {
        let text = "https://{{baseUrl}}/users";
        // Offset on the closing `}` of `{{baseUrl}}` — common hit-test edge.
        let span = variable_at_offset(text, 17).expect("baseUrl");
        assert_eq!(span.name, "baseUrl");
    }
}
