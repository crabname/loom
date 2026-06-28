use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::domain::{
    BodyType, Collection, CollectionFolder, FormField, HttpMethod, MultipartField, Request,
    RequestProtocol,
};

use super::shared::{
    normalize_variables, parse_http_method, push_warning, ImportResult,
};

const POSTMAN_SCHEMA_V21: &str =
    "https://schema.getpostman.com/json/collection/v2.1.0/collection.json";

#[derive(Debug, Clone)]
struct ParsedPostmanRequest {
    method: HttpMethod,
    url: String,
    query_params: Vec<FormField>,
    headers: Vec<FormField>,
    body_type: BodyType,
    body: String,
    form_fields: Vec<FormField>,
    multipart_fields: Vec<MultipartField>,
}

#[derive(Debug, Clone)]
struct ParsedPostmanBody {
    body_type: BodyType,
    body: String,
    form_fields: Vec<FormField>,
    multipart_fields: Vec<MultipartField>,
}

pub fn import_postman(path: &Path) -> Result<ImportResult, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    import_postman_str(&content)
}

pub fn import_postman_str(content: &str) -> Result<ImportResult, String> {
    let root: Value = serde_json::from_str(content).map_err(|error| error.to_string())?;
    let mut warnings = Vec::new();

    if !is_postman_collection(&root) {
        return Err(
            "file is not a Postman collection (expected info.schema or info._postman_schema)".into(),
        );
    }

    let name = root
        .get("info")
        .and_then(|info| info.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("Imported Collection")
        .to_string();

    let collection_variables = root
        .get("variable")
        .and_then(parse_postman_variables)
        .unwrap_or_default();

    let mut folders = Vec::new();
    let mut requests = Vec::new();
    if let Some(items) = root.get("item").and_then(Value::as_array) {
        for item in items {
            parse_postman_item(item, &mut folders, &mut requests, &mut warnings);
        }
    }

    Ok(ImportResult {
        collection: Collection {
            id: crate::domain::EntityId::new(),
            name,
            expanded: true,
            variables: postman_variables_to_domain(&collection_variables, &mut warnings, "collection"),
            environments: Vec::new(),
            folders,
            requests,
        },
        warnings,
    })
}

pub fn export_postman(collection: &Collection) -> Result<(Value, Vec<String>), String> {
    let mut warnings = Vec::new();
    let mut items = Vec::new();

    for request in &collection.requests {
        if let Some(item) = request_to_postman_item(request, &mut warnings) {
            items.push(item);
        }
    }

    for folder in &collection.folders {
        items.push(folder_to_postman_item(folder, &mut warnings));
    }

    let mut info = Map::new();
    info.insert("name".into(), Value::String(collection.name.clone()));
    info.insert(
        "schema".into(),
        Value::String(POSTMAN_SCHEMA_V21.into()),
    );

    let mut root = Map::new();
    root.insert("info".into(), Value::Object(info));
    root.insert("item".into(), Value::Array(items));

    let variables = domain_variables_to_postman(&collection.variables);
    if !variables.is_empty() {
        root.insert("variable".into(), Value::Array(variables));
    }

    Ok((Value::Object(root), warnings))
}

pub fn export_postman_json(collection: &Collection) -> Result<(String, Vec<String>), String> {
    let (value, warnings) = export_postman(collection)?;
    let json = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
    Ok((json, warnings))
}

fn parse_postman_item(
    item: &Value,
    folders: &mut Vec<CollectionFolder>,
    requests: &mut Vec<Request>,
    warnings: &mut Vec<String>,
) {
    if item.get("request").is_some() {
        match parse_postman_request_item(item, warnings) {
            Ok(request) => requests.push(request),
            Err(error) => push_warning(warnings, error),
        }
        return;
    }

    if let Some(children) = item.get("item").and_then(Value::as_array) {
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Folder")
            .to_string();
        let variables = item
            .get("variable")
            .and_then(parse_postman_variables)
            .unwrap_or_default();
        let mut folder_requests = Vec::new();
        let mut nested_folders = Vec::new();
        for child in children {
            parse_postman_item(child, &mut nested_folders, &mut folder_requests, warnings);
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
            variables: postman_variables_to_domain(&variables, warnings, "folder"),
            requests: folder_requests,
        });
        return;
    }

    push_warning(warnings, "skipped Postman item without request or folder children");
}

fn parse_postman_request_item(item: &Value, warnings: &mut Vec<String>) -> Result<Request, String> {
    let request_value = item
        .get("request")
        .ok_or("Postman item is missing request")?;
    let name = item
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| request_value.get("description").and_then(Value::as_str))
        .unwrap_or("Imported Request")
        .to_string();

    let parsed = if let Some(url) = request_value.as_str() {
        ParsedPostmanRequest {
            method: HttpMethod::Get,
            url: url.to_string(),
            query_params: crate::domain::default_key_value_fields(),
            headers: crate::domain::default_key_value_fields(),
            body_type: BodyType::None,
            body: String::new(),
            form_fields: crate::domain::default_form_fields(),
            multipart_fields: crate::domain::default_multipart_fields(),
        }
    } else {
        parse_postman_request_object(request_value, warnings)?
    };

    let variables = item
        .get("variable")
        .and_then(parse_postman_variables)
        .unwrap_or_default();

    let mut request = Request {
        id: crate::domain::EntityId::new(),
        name,
        protocol: RequestProtocol::Http,
        method: parsed.method,
        url: parsed.url,
        query_params: parsed.query_params,
        headers: parsed.headers,
        body_type: parsed.body_type,
        body: parsed.body,
        form_fields: parsed.form_fields,
        multipart_fields: parsed.multipart_fields,
        variables: postman_variables_to_domain(&variables, warnings, "request"),
        pre_request_script: String::new(),
        post_response_script: String::new(),
        tests_script: String::new(),
    };

    if let Some(events) = item.get("event").and_then(Value::as_array) {
        apply_postman_events(&mut request, events, warnings);
    }

    Ok(request)
}

fn parse_postman_request_object(
    request: &Value,
    warnings: &mut Vec<String>,
) -> Result<ParsedPostmanRequest, String> {
    if request.get("auth").is_some() {
        push_warning(warnings, "request auth settings were not imported");
    }

    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("GET");
    let method = parse_http_method(method, warnings, "request");

    let (url, query_params) = parse_postman_url(request.get("url"), warnings);
    let headers = request
        .get("header")
        .map(|header| parse_postman_headers(header, warnings))
        .unwrap_or_else(crate::domain::default_key_value_fields);

    let parsed_body = request
        .get("body")
        .map(|body| parse_postman_body(body, warnings))
        .transpose()?
        .unwrap_or_else(|| ParsedPostmanBody {
            body_type: BodyType::None,
            body: String::new(),
            form_fields: crate::domain::default_form_fields(),
            multipart_fields: crate::domain::default_multipart_fields(),
        });

    Ok(ParsedPostmanRequest {
        method,
        url,
        query_params,
        headers,
        body_type: parsed_body.body_type,
        body: parsed_body.body,
        form_fields: parsed_body.form_fields,
        multipart_fields: parsed_body.multipart_fields,
    })
}

fn parse_postman_url(
    value: Option<&Value>,
    warnings: &mut Vec<String>,
) -> (String, Vec<crate::domain::FormField>) {
    let Some(value) = value else {
        return (String::new(), crate::domain::default_key_value_fields());
    };

    if let Some(url) = value.as_str() {
        return (url.to_string(), crate::domain::default_key_value_fields());
    }

    let raw = value
        .get("raw")
        .and_then(Value::as_str)
        .filter(|url| !url.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| build_postman_url(value));

    if raw.is_empty() {
        push_warning(warnings, "request URL could not be resolved from Postman format");
    }

    let query = value
        .get("query")
        .and_then(parse_postman_key_values)
        .map(|fields| postman_key_values_to_domain(fields, warnings, "query params"))
        .unwrap_or_else(crate::domain::default_key_value_fields);
    (raw, query)
}

fn build_postman_url(value: &Value) -> String {
    let protocol = value
        .get("protocol")
        .and_then(Value::as_str)
        .unwrap_or("https");
    let host = value
        .get("host")
        .map(postman_url_host)
        .filter(|host| !host.is_empty())
        .or_else(|| {
            value
                .get("host")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();
    if host.is_empty() {
        return String::new();
    }

    let mut url = format!("{protocol}://{host}");
    if let Some(port) = value.get("port").and_then(Value::as_str)
        && !port.is_empty()
    {
        url.push(':');
        url.push_str(port);
    }

    let path = value
        .get("path")
        .map(postman_url_path)
        .filter(|path| !path.is_empty())
        .or_else(|| value.get("path").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default();
    if !path.is_empty() {
        if path.starts_with('/') {
            url.push_str(&path);
        } else {
            url.push('/');
            url.push_str(&path);
        }
    }

    url
}

fn postman_url_host(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("."),
        _ => String::new(),
    }
}

fn postman_url_path(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("/"),
        _ => String::new(),
    }
}

#[derive(Debug, Clone)]
struct PostmanKeyValue {
    key: String,
    value: String,
    disabled: bool,
}

fn is_postman_collection(root: &Value) -> bool {
    let Some(info) = root.get("info") else {
        return false;
    };

    for field in ["schema", "_postman_schema"] {
        if let Some(schema) = info.get(field).and_then(Value::as_str)
            && (schema.contains("postman.com") || schema.contains("schema.getpostman.com"))
        {
            return true;
        }
    }

    info.get("name").is_some() && root.get("item").is_some()
}

fn json_value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        other => other.to_string(),
    }
}

fn parse_postman_headers(
    value: &Value,
    warnings: &mut Vec<String>,
) -> Vec<crate::domain::FormField> {
    if let Some(text) = value.as_str() {
        return postman_key_values_to_domain(parse_header_string(text), warnings, "request headers");
    }

    parse_postman_key_values(value)
        .map(|fields| postman_key_values_to_domain(fields, warnings, "request headers"))
        .unwrap_or_else(crate::domain::default_key_value_fields)
}

fn parse_header_string(text: &str) -> Vec<PostmanKeyValue> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_end_matches('\r').trim();
            if line.is_empty() {
                return None;
            }
            let (key, value) = line.split_once(':')?;
            Some(PostmanKeyValue {
                key: key.trim().to_string(),
                value: value.trim().to_string(),
                disabled: false,
            })
        })
        .collect()
}

fn parse_postman_key_values(value: &Value) -> Option<Vec<PostmanKeyValue>> {
    let sequence = value.as_array()?;
    Some(
        sequence
            .iter()
            .filter_map(|entry| {
                let key = entry
                    .get("key")
                    .or_else(|| entry.get("name"))
                    .and_then(Value::as_str)?
                    .to_string();
                Some(PostmanKeyValue {
                    key,
                    value: entry
                        .get("value")
                        .map(json_value_to_string)
                        .unwrap_or_default(),
                    disabled: entry
                        .get("disabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                })
            })
            .collect(),
    )
}

fn postman_key_values_to_domain(
    values: Vec<PostmanKeyValue>,
    warnings: &mut Vec<String>,
    context: &str,
) -> Vec<crate::domain::FormField> {
    let mut fields = Vec::new();
    for value in values {
        if value.key.is_empty() {
            continue;
        }
        fields.push(crate::domain::FormField {
            enabled: !value.disabled,
            name: value.key,
            value: value.value,
        });
    }
    if fields.is_empty() {
        return crate::domain::default_key_value_fields();
    }
    if fields.iter().all(|field| !field.enabled) {
        push_warning(warnings, format!("all {context} are disabled"));
    }
    fields.push(crate::domain::FormField::empty());
    fields
}

fn parse_postman_body(body: &Value, warnings: &mut Vec<String>) -> Result<ParsedPostmanBody, String> {
    let mode = body
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("raw");

    match mode {
        "raw" => {
            let raw = body
                .get("raw")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let language = body
                .get("options")
                .and_then(|options| options.get("raw"))
                .and_then(|raw| raw.get("language"))
                .and_then(Value::as_str)
                .unwrap_or("json");
            let body_type = match language {
                "json" => BodyType::Json,
                "xml" => BodyType::Xml,
                other => {
                    push_warning(
                        warnings,
                        format!("raw body language `{other}` stored as plain text"),
                    );
                    BodyType::None
                }
            };
            Ok(ParsedPostmanBody {
                body_type,
                body: raw,
                form_fields: crate::domain::default_form_fields(),
                multipart_fields: crate::domain::default_multipart_fields(),
            })
        }
        "urlencoded" => {
            let fields = body
                .get("urlencoded")
                .and_then(parse_postman_key_values)
                .map(|values| postman_key_values_to_domain(values, warnings, "form fields"))
                .unwrap_or_else(crate::domain::default_form_fields);
            Ok(ParsedPostmanBody {
                body_type: BodyType::FormUrlEncoded,
                body: String::new(),
                form_fields: fields,
                multipart_fields: crate::domain::default_multipart_fields(),
            })
        }
        "formdata" => {
            let multipart_fields = body
                .get("formdata")
                .and_then(Value::as_array)
                .map(|entries| parse_postman_formdata(entries, warnings))
                .unwrap_or_else(crate::domain::default_multipart_fields);
            Ok(ParsedPostmanBody {
                body_type: BodyType::Multipart,
                body: String::new(),
                form_fields: crate::domain::default_form_fields(),
                multipart_fields,
            })
        }
        "file" | "graphql" => Err(format!("unsupported Postman body mode `{mode}`")),
        other => Err(format!("unsupported Postman body mode `{other}`")),
    }
}

fn parse_postman_formdata(
    entries: &[Value],
    warnings: &mut Vec<String>,
) -> Vec<crate::domain::MultipartField> {
    let mut fields = Vec::new();
    for entry in entries {
        let name = entry
            .get("key")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if name.is_empty() {
            continue;
        }
        let field_type = match entry.get("type").and_then(Value::as_str).unwrap_or("text") {
            "text" => crate::domain::MultipartFieldType::Text,
            "file" => crate::domain::MultipartFieldType::File,
            other => {
                push_warning(
                    warnings,
                    format!("multipart field `{name}` has unsupported type `{other}`"),
                );
                crate::domain::MultipartFieldType::Text
            }
        };
        fields.push(crate::domain::MultipartField {
            enabled: !entry
                .get("disabled")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            name,
            value: entry
                .get("value")
                .or_else(|| entry.get("src"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            field_type,
            content_type: entry
                .get("contentType")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        });
    }
    if fields.is_empty() {
        crate::domain::default_multipart_fields()
    } else {
        fields.push(crate::domain::MultipartField::empty());
        fields
    }
}

#[derive(Debug, Clone)]
struct PostmanVariable {
    key: String,
    value: String,
    disabled: bool,
}

fn parse_postman_variables(value: &Value) -> Option<Vec<PostmanVariable>> {
    let sequence = value.as_array()?;
    Some(
        sequence
            .iter()
            .filter_map(|entry| {
                let key = entry
                    .get("key")
                    .or_else(|| entry.get("name"))
                    .or_else(|| entry.get("id"))
                    .and_then(Value::as_str)?
                    .to_string();
                Some(PostmanVariable {
                    key,
                    value: entry
                        .get("value")
                        .map(json_value_to_string)
                        .unwrap_or_default(),
                    disabled: entry
                        .get("disabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                })
            })
            .collect(),
    )
}

fn postman_variables_to_domain(
    variables: &[PostmanVariable],
    warnings: &mut Vec<String>,
    context: &str,
) -> Vec<crate::domain::Variable> {
    let mut result = Vec::new();
    for variable in variables {
        if variable.key.is_empty() {
            continue;
        }
        if variable.disabled {
            push_warning(
                warnings,
                format!("skipped disabled {context} variable `{}`", variable.key),
            );
            continue;
        }
        result.push(crate::domain::Variable::from_strings(
            variable.key.clone(),
            variable.value.clone(),
        ));
    }
    normalize_variables(result)
}

fn domain_variables_to_postman(variables: &[crate::domain::Variable]) -> Vec<Value> {
    variables
        .iter()
        .filter(|variable| {
            !variable.name.is_empty() || !variable.display_value().is_empty()
        })
        .map(|variable| {
            json!({
                "key": variable.name,
                "value": variable.display_value(),
                "type": "string"
            })
        })
        .collect()
}

fn apply_postman_events(request: &mut Request, events: &[Value], warnings: &mut Vec<String>) {
    for event in events {
        let listen = event
            .get("listen")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(script) = event.get("script") else {
            continue;
        };

        if script.is_string() {
            push_warning(
                warnings,
                format!(
                    "skipped script reference `{}` on `{}`",
                    script.as_str().unwrap_or_default(),
                    request.name
                ),
            );
            continue;
        }

        let code = script
            .get("exec")
            .map(|exec| match exec {
                Value::Array(lines) => lines
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("\n"),
                Value::String(text) => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        if code.is_empty() {
            continue;
        }

        match listen {
            "prerequest" => request.pre_request_script = code,
            "test" => request.tests_script = code,
            other => push_warning(warnings, format!("unsupported Postman event `{other}`")),
        }
    }
}

fn request_to_postman_item(request: &Request, warnings: &mut Vec<String>) -> Option<Value> {
    if request.protocol != RequestProtocol::Http {
        push_warning(
            warnings,
            format!("skipped non-HTTP request `{}` during export", request.name),
        );
        return None;
    }

    let mut request_object = Map::new();
    request_object.insert("method".into(), Value::String(request.method.as_str().into()));
    request_object.insert(
        "url".into(),
        Value::String(request.url.clone()),
    );
    let headers = domain_key_values_to_postman(&request.headers);
    if !headers.is_empty() {
        request_object.insert("header".into(), Value::Array(headers));
    }
    if let Some(body) = domain_body_to_postman(request) {
        request_object.insert("body".into(), body);
    }

    let mut item = Map::new();
    item.insert("name".into(), Value::String(request.name.clone()));
    item.insert("request".into(), Value::Object(request_object));

    let variables = domain_variables_to_postman(&request.variables);
    if !variables.is_empty() {
        item.insert("variable".into(), Value::Array(variables));
    }

    let mut events = Vec::new();
    if !request.pre_request_script.trim().is_empty() {
        events.push(postman_event("prerequest", &request.pre_request_script));
    }
    if !request.tests_script.trim().is_empty() {
        events.push(postman_event("test", &request.tests_script));
    }
    if !events.is_empty() {
        item.insert("event".into(), Value::Array(events));
    }

    Some(Value::Object(item))
}

fn folder_to_postman_item(folder: &CollectionFolder, warnings: &mut Vec<String>) -> Value {
    let mut children = Vec::new();
    for request in &folder.requests {
        if let Some(item) = request_to_postman_item(request, warnings) {
            children.push(item);
        }
    }

    let mut item = Map::new();
    item.insert("name".into(), Value::String(folder.name.clone()));
    item.insert("item".into(), Value::Array(children));

    let variables = domain_variables_to_postman(&folder.variables);
    if !variables.is_empty() {
        item.insert("variable".into(), Value::Array(variables));
    }

    Value::Object(item)
}

fn domain_key_values_to_postman(fields: &[crate::domain::FormField]) -> Vec<Value> {
    fields
        .iter()
        .filter(|field| !field.name.is_empty() || !field.value.is_empty())
        .map(|field| {
            let mut entry = Map::new();
            entry.insert("key".into(), Value::String(field.name.clone()));
            entry.insert("value".into(), Value::String(field.value.clone()));
            if !field.enabled {
                entry.insert("disabled".into(), Value::Bool(true));
            }
            Value::Object(entry)
        })
        .collect()
}

fn domain_body_to_postman(request: &Request) -> Option<Value> {
    match request.body_type {
        crate::domain::BodyType::None => None,
        crate::domain::BodyType::Json => Some(json!({
            "mode": "raw",
            "raw": request.body,
            "options": { "raw": { "language": "json" } }
        })),
        crate::domain::BodyType::Xml => Some(json!({
            "mode": "raw",
            "raw": request.body,
            "options": { "raw": { "language": "xml" } }
        })),
        crate::domain::BodyType::FormUrlEncoded => Some(json!({
            "mode": "urlencoded",
            "urlencoded": domain_key_values_to_postman(&request.form_fields)
        })),
        crate::domain::BodyType::Multipart => Some(json!({
            "mode": "formdata",
            "formdata": request.multipart_fields.iter()
                .filter(|field| !field.name.is_empty() || !field.value.is_empty() || !field.content_type.is_empty())
                .map(|field| {
                    json!({
                        "key": field.name,
                        "value": field.value,
                        "type": field.field_type.label(),
                        "disabled": !field.enabled,
                        "contentType": field.content_type
                    })
                })
                .collect::<Vec<_>>()
        })),
    }
}

fn postman_event(listen: &str, code: &str) -> Value {
    json!({
        "listen": listen,
        "script": {
            "type": "text/javascript",
            "exec": code.lines().collect::<Vec<_>>()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_simple_postman_collection() {
        let json = r#"{
            "info": {
                "name": "Demo",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [{
                "name": "Hello",
                "request": {
                    "method": "POST",
                    "header": [{ "key": "Content-Type", "value": "application/json" }],
                    "body": {
                        "mode": "raw",
                        "raw": "{\"ok\":true}",
                        "options": { "raw": { "language": "json" } }
                    },
                    "url": "https://example.com"
                },
                "event": [{
                    "listen": "test",
                    "script": { "type": "text/javascript", "exec": ["pm.test('ok', () => {});"] }
                }]
            }]
        }"#;

        let imported = import_postman_str(json).expect("import");
        assert_eq!(imported.collection.name, "Demo");
        assert_eq!(imported.collection.requests.len(), 1);
        assert_eq!(imported.collection.requests[0].method, HttpMethod::Post);
        assert!(imported.collection.requests[0].tests_script.contains("pm.test"));
    }

    #[test]
    fn imports_postman_v2_fixture() {
        let json = include_str!("../../collection-v2.json");
        let imported = import_postman_str(json).expect("import v2 collection");
        assert_eq!(imported.collection.name, "HTTP Status Messages");
        assert_eq!(imported.collection.requests.len(), 2);
        assert!(!imported.collection.folders.is_empty());
        assert!(imported
            .collection
            .variables
            .iter()
            .any(|variable| variable.name == "var-1"));
    }

    #[test]
    fn exports_postman_collection() {
        let mut collection = Collection::new("Demo");
        let mut request = Request::new("Hello");
        request.url = "https://example.com".into();
        collection.requests.push(request);

        let (json, _) = export_postman_json(&collection).expect("export");
        let imported = import_postman_str(&json).expect("import");
        assert_eq!(imported.collection.requests[0].url, "https://example.com");
    }
}
