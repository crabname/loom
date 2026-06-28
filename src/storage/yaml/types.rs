use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{
    default_key_value_fields, default_multipart_fields, default_variables,
    BodyType, Collection, CollectionFolder, Environment, FormField, HttpMethod, MultipartField,
    MultipartFieldType, Request, RequestProtocol, Variable, Workspace,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceFile {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub collections: Vec<CollectionRef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionRef {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VariablesFile {
    #[serde(default)]
    pub variables: Vec<VariableFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentFile {
    pub name: String,
    #[serde(default)]
    pub variables: Vec<VariableFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionFile {
    pub name: String,
    #[serde(default)]
    pub variables: Vec<VariableFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FolderFile {
    pub name: String,
    #[serde(default)]
    pub variables: Vec<VariableFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestFile {
    pub name: String,
    #[serde(default)]
    pub protocol: ProtocolFile,
    pub method: MethodFile,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub query_params: Vec<FieldFile>,
    #[serde(default)]
    pub headers: Vec<FieldFile>,
    #[serde(default)]
    pub body_type: BodyTypeFile,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub form_fields: Vec<FieldFile>,
    #[serde(default)]
    pub multipart_fields: Vec<MultipartFieldFile>,
    #[serde(default)]
    pub variables: Vec<VariableFile>,
    #[serde(default)]
    pub pre_request_script: String,
    #[serde(default)]
    pub post_response_script: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ProtocolFile {
    #[default]
    Http,
    Grpc,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum MethodFile {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum BodyTypeFile {
    #[default]
    None,
    Json,
    Xml,
    #[serde(rename = "form_urlencoded")]
    FormUrlEncoded,
    Multipart,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldFile {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MultipartFieldFile {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub field_type: MultipartFieldTypeFile,
    #[serde(default)]
    pub content_type: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum MultipartFieldTypeFile {
    #[default]
    Text,
    File,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VariableFile {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: Value,
}

fn default_true() -> bool {
    true
}

impl From<ProtocolFile> for RequestProtocol {
    fn from(value: ProtocolFile) -> Self {
        match value {
            ProtocolFile::Http => Self::Http,
            ProtocolFile::Grpc => Self::Grpc,
        }
    }
}

impl From<MethodFile> for HttpMethod {
    fn from(value: MethodFile) -> Self {
        match value {
            MethodFile::GET => Self::Get,
            MethodFile::POST => Self::Post,
            MethodFile::PUT => Self::Put,
            MethodFile::PATCH => Self::Patch,
            MethodFile::DELETE => Self::Delete,
        }
    }
}

impl From<BodyTypeFile> for BodyType {
    fn from(value: BodyTypeFile) -> Self {
        match value {
            BodyTypeFile::None => Self::None,
            BodyTypeFile::Json => Self::Json,
            BodyTypeFile::Xml => Self::Xml,
            BodyTypeFile::FormUrlEncoded => Self::FormUrlEncoded,
            BodyTypeFile::Multipart => Self::Multipart,
        }
    }
}

impl From<MultipartFieldTypeFile> for MultipartFieldType {
    fn from(value: MultipartFieldTypeFile) -> Self {
        match value {
            MultipartFieldTypeFile::Text => Self::Text,
            MultipartFieldTypeFile::File => Self::File,
        }
    }
}

impl From<VariableFile> for Variable {
    fn from(value: VariableFile) -> Self {
        Self {
            name: value.name,
            value: value.value,
        }
    }
}

impl From<FieldFile> for FormField {
    fn from(value: FieldFile) -> Self {
        Self {
            enabled: value.enabled,
            name: value.name,
            value: value.value,
        }
    }
}

impl From<MultipartFieldFile> for MultipartField {
    fn from(value: MultipartFieldFile) -> Self {
        Self {
            enabled: value.enabled,
            name: value.name,
            value: value.value,
            field_type: value.field_type.into(),
            content_type: value.content_type,
        }
    }
}

impl From<EnvironmentFile> for Environment {
    fn from(value: EnvironmentFile) -> Self {
        Self {
            name: value.name,
            variables: normalize_variables(value.variables.into_iter().map(Into::into).collect()),
        }
    }
}

impl From<RequestFile> for Request {
    fn from(value: RequestFile) -> Self {
        Self {
            name: value.name,
            protocol: value.protocol.into(),
            method: value.method.into(),
            url: value.url,
            query_params: normalize_fields(value.query_params.into_iter().map(Into::into).collect()),
            headers: normalize_fields(value.headers.into_iter().map(Into::into).collect()),
            body_type: value.body_type.into(),
            body: value.body,
            form_fields: normalize_fields(value.form_fields.into_iter().map(Into::into).collect()),
            multipart_fields: normalize_multipart_fields(
                value
                    .multipart_fields
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            ),
            variables: normalize_variables(value.variables.into_iter().map(Into::into).collect()),
            pre_request_script: value.pre_request_script,
            post_response_script: value.post_response_script,
        }
    }
}

impl From<FolderFile> for CollectionFolder {
    fn from(value: FolderFile) -> Self {
        Self {
            name: value.name,
            expanded: true,
            variables: normalize_variables(value.variables.into_iter().map(Into::into).collect()),
            requests: Vec::new(),
        }
    }
}

pub fn normalize_fields(mut fields: Vec<FormField>) -> Vec<FormField> {
    fields.retain(|field| !field.name.is_empty() || !field.value.is_empty());
    if fields.is_empty() {
        return default_key_value_fields();
    }
    fields.push(FormField::empty());
    fields
}

pub fn normalize_multipart_fields(mut fields: Vec<MultipartField>) -> Vec<MultipartField> {
    fields.retain(|field| {
        !field.name.is_empty() || !field.value.is_empty() || !field.content_type.is_empty()
    });
    if fields.is_empty() {
        return default_multipart_fields();
    }
    fields.push(MultipartField::empty());
    fields
}

pub fn normalize_variables(mut variables: Vec<Variable>) -> Vec<Variable> {
    variables.retain(|variable| !variable.name.is_empty() || !variable.display_value().is_empty());
    if variables.is_empty() {
        return default_variables();
    }
    variables.push(Variable::empty());
    variables
}

pub fn workspace_from_parts(
    name: String,
    variables: Vec<Variable>,
    environments: Vec<Environment>,
    collections: Vec<Collection>,
) -> Workspace {
    Workspace {
        name,
        variables: normalize_variables(variables),
        environments,
        collections,
    }
}

pub fn collection_from_parts(
    name: String,
    variables: Vec<Variable>,
    environments: Vec<Environment>,
    folders: Vec<CollectionFolder>,
    requests: Vec<Request>,
) -> Collection {
    Collection {
        name,
        expanded: true,
        variables: normalize_variables(variables),
        environments,
        folders,
        requests,
    }
}

pub fn serializable_variables(variables: &[Variable]) -> Vec<VariableFile> {
    variables
        .iter()
        .filter(|variable| {
            !variable.name.is_empty() || !variable.display_value().is_empty()
        })
        .map(|variable| VariableFile {
            name: variable.name.clone(),
            value: variable.value.clone(),
        })
        .collect()
}

pub fn serializable_fields(fields: &[FormField]) -> Vec<FieldFile> {
    fields
        .iter()
        .filter(|field| !field.name.is_empty() || !field.value.is_empty())
        .map(|field| FieldFile {
            enabled: field.enabled,
            name: field.name.clone(),
            value: field.value.clone(),
        })
        .collect()
}

pub fn serializable_multipart_fields(fields: &[MultipartField]) -> Vec<MultipartFieldFile> {
    fields
        .iter()
        .filter(|field| {
            !field.name.is_empty() || !field.value.is_empty() || !field.content_type.is_empty()
        })
        .map(|field| MultipartFieldFile {
            enabled: field.enabled,
            name: field.name.clone(),
            value: field.value.clone(),
            field_type: field.field_type.into(),
            content_type: field.content_type.clone(),
        })
        .collect()
}

impl From<RequestProtocol> for ProtocolFile {
    fn from(value: RequestProtocol) -> Self {
        match value {
            RequestProtocol::Http => Self::Http,
            RequestProtocol::Grpc => Self::Grpc,
        }
    }
}

impl From<HttpMethod> for MethodFile {
    fn from(value: HttpMethod) -> Self {
        match value {
            HttpMethod::Get => Self::GET,
            HttpMethod::Post => Self::POST,
            HttpMethod::Put => Self::PUT,
            HttpMethod::Patch => Self::PATCH,
            HttpMethod::Delete => Self::DELETE,
        }
    }
}

impl From<BodyType> for BodyTypeFile {
    fn from(value: BodyType) -> Self {
        match value {
            BodyType::None => Self::None,
            BodyType::Json => Self::Json,
            BodyType::Xml => Self::Xml,
            BodyType::FormUrlEncoded => Self::FormUrlEncoded,
            BodyType::Multipart => Self::Multipart,
        }
    }
}

impl From<MultipartFieldType> for MultipartFieldTypeFile {
    fn from(value: MultipartFieldType) -> Self {
        match value {
            MultipartFieldType::Text => Self::Text,
            MultipartFieldType::File => Self::File,
        }
    }
}

impl From<&Environment> for EnvironmentFile {
    fn from(value: &Environment) -> Self {
        Self {
            name: value.name.clone(),
            variables: serializable_variables(&value.variables),
        }
    }
}

impl From<&Collection> for CollectionFile {
    fn from(value: &Collection) -> Self {
        Self {
            name: value.name.clone(),
            variables: serializable_variables(&value.variables),
        }
    }
}

impl From<&CollectionFolder> for FolderFile {
    fn from(value: &CollectionFolder) -> Self {
        Self {
            name: value.name.clone(),
            variables: serializable_variables(&value.variables),
        }
    }
}

impl From<&Request> for RequestFile {
    fn from(value: &Request) -> Self {
        Self {
            name: value.name.clone(),
            protocol: value.protocol.into(),
            method: value.method.into(),
            url: value.url.clone(),
            query_params: serializable_fields(&value.query_params),
            headers: serializable_fields(&value.headers),
            body_type: value.body_type.into(),
            body: value.body.clone(),
            form_fields: serializable_fields(&value.form_fields),
            multipart_fields: serializable_multipart_fields(&value.multipart_fields),
            variables: serializable_variables(&value.variables),
            pre_request_script: value.pre_request_script.clone(),
            post_response_script: value.post_response_script.clone(),
        }
    }
}
