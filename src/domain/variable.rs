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
}
