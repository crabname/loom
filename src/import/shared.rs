use serde_json::Value;

use crate::domain::{
    default_form_fields, default_key_value_fields, default_multipart_fields, default_variables,
    BodyType, FormField, HttpMethod, MultipartField, MultipartFieldType, Request, Variable,
};

pub struct ImportResult {
    pub collection: crate::domain::Collection,
    pub warnings: Vec<String>,
}

pub fn push_warning(warnings: &mut Vec<String>, message: impl Into<String>) {
    warnings.push(message.into());
}

pub fn oc_variable_value(value: &OcVariableValue) -> Value {
    match value {
        OcVariableValue::Plain(text) => Value::String(text.clone()),
        OcVariableValue::Typed { data, .. } => parse_typed_value(data),
    }
}

#[derive(Debug, Clone)]
pub enum OcVariableValue {
    Plain(String),
    Typed { var_type: String, data: String },
}

#[derive(Debug, Clone)]
pub struct OcVariable {
    pub name: String,
    pub value: OcVariableValue,
    pub disabled: bool,
}

pub fn oc_variables_to_domain(
    variables: &[OcVariable],
    warnings: &mut Vec<String>,
    context: &str,
) -> Vec<Variable> {
    let mut result = Vec::new();
    for variable in variables {
        if variable.name.is_empty() {
            continue;
        }
        if variable.disabled {
            push_warning(
                warnings,
                format!("skipped disabled {context} variable `{}`", variable.name),
            );
            continue;
        }
        result.push(Variable {
            name: variable.name.clone(),
            value: oc_variable_value(&variable.value),
        });
    }
    normalize_variables(result)
}

pub fn domain_variables_to_oc(variables: &[Variable]) -> Vec<OcVariable> {
    variables
        .iter()
        .filter(|variable| {
            !variable.name.is_empty() || !variable.display_value().is_empty()
        })
        .map(|variable| OcVariable {
            name: variable.name.clone(),
            value: OcVariableValue::Plain(variable.display_value()),
            disabled: false,
        })
        .collect()
}

fn parse_typed_value(data: &str) -> Value {
    if let Ok(value) = serde_json::from_str::<Value>(data) {
        return value;
    }
    Value::String(data.to_string())
}

pub fn normalize_variables(mut variables: Vec<Variable>) -> Vec<Variable> {
    variables.retain(|variable| !variable.name.is_empty() || !variable.display_value().is_empty());
    if variables.is_empty() {
        return default_variables();
    }
    variables.push(Variable::empty());
    variables
}

pub fn oc_headers_to_domain(
    headers: &[OcKeyValue],
    warnings: &mut Vec<String>,
    context: &str,
) -> Vec<FormField> {
    let mut fields = Vec::new();
    for header in headers {
        if header.name.is_empty() {
            continue;
        }
        fields.push(FormField {
            enabled: !header.disabled,
            name: header.name.clone(),
            value: header.value.clone(),
        });
    }
    if fields.is_empty() {
        return default_key_value_fields();
    }
    if fields.iter().all(|field| !field.enabled) {
        push_warning(warnings, format!("all {context} headers are disabled"));
    }
    fields.push(FormField::empty());
    fields
}

pub fn domain_headers_to_oc(headers: &[FormField]) -> Vec<OcKeyValue> {
    headers
        .iter()
        .filter(|field| !field.name.is_empty() || !field.value.is_empty())
        .map(|field| OcKeyValue {
            name: field.name.clone(),
            value: field.value.clone(),
            disabled: !field.enabled,
            param_type: None,
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct OcKeyValue {
    pub name: String,
    pub value: String,
    pub disabled: bool,
    pub param_type: Option<String>,
}

pub fn oc_params_to_domain(
    params: &[OcKeyValue],
    warnings: &mut Vec<String>,
) -> Vec<FormField> {
    let mut fields = Vec::new();
    for param in params {
        if param.name.is_empty() {
            continue;
        }
        match param.param_type.as_deref() {
            Some("query") | None => fields.push(FormField {
                enabled: !param.disabled,
                name: param.name.clone(),
                value: param.value.clone(),
            }),
            Some("path") => push_warning(
                warnings,
                format!(
                    "path param `{}` is not stored separately; keep it in the URL",
                    param.name
                ),
            ),
            Some(other) => push_warning(
                warnings,
                format!("unsupported param type `{other}` for `{}`", param.name),
            ),
        }
    }
    if fields.is_empty() {
        return default_key_value_fields();
    }
    fields.push(FormField::empty());
    fields
}

pub fn domain_params_to_oc(params: &[FormField]) -> Vec<OcKeyValue> {
    params
        .iter()
        .filter(|field| !field.name.is_empty() || !field.value.is_empty())
        .map(|field| OcKeyValue {
            name: field.name.clone(),
            value: field.value.clone(),
            disabled: !field.enabled,
            param_type: Some("query".into()),
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct OcScript {
    pub script_type: String,
    pub code: String,
}

pub fn apply_oc_scripts(request: &mut Request, scripts: &[OcScript], warnings: &mut Vec<String>) {
    for script in scripts {
        match script.script_type.as_str() {
            "before-request" => request.pre_request_script = script.code.clone(),
            "after-response" => request.post_response_script = script.code.clone(),
            "tests" => request.tests_script = script.code.clone(),
            "hooks" => push_warning(warnings, "hook scripts are not supported yet"),
            other => push_warning(warnings, format!("unsupported script type `{other}`")),
        }
    }
}

pub fn domain_scripts_to_oc(request: &Request) -> Vec<OcScript> {
    let mut scripts = Vec::new();
    if !request.pre_request_script.trim().is_empty() {
        scripts.push(OcScript {
            script_type: "before-request".into(),
            code: request.pre_request_script.clone(),
        });
    }
    if !request.post_response_script.trim().is_empty() {
        scripts.push(OcScript {
            script_type: "after-response".into(),
            code: request.post_response_script.clone(),
        });
    }
    if !request.tests_script.trim().is_empty() {
        scripts.push(OcScript {
            script_type: "tests".into(),
            code: request.tests_script.clone(),
        });
    }
    scripts
}

#[derive(Debug, Clone)]
pub enum OcBody {
    None,
    Raw { body_type: String, data: String },
    FormUrlEncoded(Vec<OcKeyValue>),
    Multipart(Vec<OcMultipartPart>),
}

#[derive(Debug, Clone)]
pub struct OcMultipartPart {
    pub name: String,
    pub part_type: String,
    pub value: String,
    pub content_type: String,
    pub disabled: bool,
}

pub fn oc_body_to_domain(body: &OcBody, warnings: &mut Vec<String>) -> (BodyType, String, Vec<FormField>, Vec<MultipartField>) {
    match body {
        OcBody::None => (
            BodyType::None,
            String::new(),
            default_form_fields(),
            default_multipart_fields(),
        ),
        OcBody::Raw { body_type, data } => {
            let body_type = match body_type.as_str() {
                "json" => BodyType::Json,
                "xml" => BodyType::Xml,
                "text" | "sparql" => {
                    push_warning(
                        warnings,
                        format!("raw body type `{body_type}` stored as plain text body"),
                    );
                    BodyType::None
                }
                other => {
                    push_warning(warnings, format!("unsupported raw body type `{other}`"));
                    BodyType::None
                }
            };
            (body_type, data.clone(), default_form_fields(), default_multipart_fields())
        }
        OcBody::FormUrlEncoded(fields) => {
            let mut form_fields = Vec::new();
            for field in fields {
                if field.name.is_empty() {
                    continue;
                }
                form_fields.push(FormField {
                    enabled: !field.disabled,
                    name: field.name.clone(),
                    value: field.value.clone(),
                });
            }
            if form_fields.is_empty() {
                form_fields = default_form_fields();
            } else {
                form_fields.push(FormField::empty());
            }
            (
                BodyType::FormUrlEncoded,
                String::new(),
                form_fields,
                default_multipart_fields(),
            )
        }
        OcBody::Multipart(parts) => {
            let mut multipart_fields = Vec::new();
            for part in parts {
                if part.name.is_empty() {
                    continue;
                }
                let field_type = match part.part_type.as_str() {
                    "text" => MultipartFieldType::Text,
                    "file" => MultipartFieldType::File,
                    other => {
                        push_warning(
                            warnings,
                            format!("multipart part `{}` has unsupported type `{other}`", part.name),
                        );
                        MultipartFieldType::Text
                    }
                };
                multipart_fields.push(MultipartField {
                    enabled: !part.disabled,
                    name: part.name.clone(),
                    value: part.value.clone(),
                    field_type,
                    content_type: part.content_type.clone(),
                });
            }
            if multipart_fields.is_empty() {
                multipart_fields = default_multipart_fields();
            } else {
                multipart_fields.push(MultipartField::empty());
            }
            (
                BodyType::Multipart,
                String::new(),
                default_form_fields(),
                multipart_fields,
            )
        }
    }
}

pub fn domain_body_to_oc(
    body_type: BodyType,
    body: &str,
    form_fields: &[FormField],
    multipart_fields: &[MultipartField],
) -> OcBody {
    match body_type {
        BodyType::None => OcBody::None,
        BodyType::Json => OcBody::Raw {
            body_type: "json".into(),
            data: body.to_string(),
        },
        BodyType::Xml => OcBody::Raw {
            body_type: "xml".into(),
            data: body.to_string(),
        },
        BodyType::FormUrlEncoded => OcBody::FormUrlEncoded(
            form_fields
                .iter()
                .filter(|field| !field.name.is_empty() || !field.value.is_empty())
                .map(|field| OcKeyValue {
                    name: field.name.clone(),
                    value: field.value.clone(),
                    disabled: !field.enabled,
                    param_type: None,
                })
                .collect(),
        ),
        BodyType::Multipart => OcBody::Multipart(
            multipart_fields
                .iter()
                .filter(|field| {
                    !field.name.is_empty() || !field.value.is_empty() || !field.content_type.is_empty()
                })
                .map(|field| OcMultipartPart {
                    name: field.name.clone(),
                    part_type: field.field_type.label().into(),
                    value: field.value.clone(),
                    content_type: field.content_type.clone(),
                    disabled: !field.enabled,
                })
                .collect(),
        ),
    }
}

pub fn parse_http_method(method: &str, warnings: &mut Vec<String>, context: &str) -> HttpMethod {
    HttpMethod::from_label(method).unwrap_or_else(|| {
        push_warning(
            warnings,
            format!("unsupported HTTP method `{method}` in {context}; defaulting to GET"),
        );
        HttpMethod::Get
    })
}

pub fn slugify_name(name: &str) -> String {
    let slug = name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase();
    if slug.is_empty() {
        "request".into()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_strips_unsafe_characters() {
        assert_eq!(slugify_name("Get Orders!"), "get-orders");
    }
}
