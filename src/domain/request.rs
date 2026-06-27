use super::{
    default_form_fields, default_key_value_fields, default_multipart_fields, FormField,
    KeyValueField, MultipartField,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RequestProtocol {
    #[default]
    Http,
    Grpc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyType {
    None,
    Json,
    Xml,
    FormUrlEncoded,
    Multipart,
}

impl BodyType {
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Json,
        Self::Xml,
        Self::FormUrlEncoded,
        Self::Multipart,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Json => "JSON",
            Self::Xml => "XML",
            Self::FormUrlEncoded => "form-urlencoded",
            Self::Multipart => "multipart",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|body_type| body_type.label() == label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub const ALL: [Self; 5] = [
        Self::Get,
        Self::Post,
        Self::Put,
        Self::Patch,
        Self::Delete,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|method| method.as_str() == label)
    }
}

impl Request {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            protocol: RequestProtocol::default(),
            method: HttpMethod::Get,
            url: String::new(),
            query_params: default_key_value_fields(),
            headers: default_key_value_fields(),
            body_type: BodyType::None,
            body: String::new(),
            form_fields: default_form_fields(),
            multipart_fields: default_multipart_fields(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    pub name: String,
    pub protocol: RequestProtocol,
    pub method: HttpMethod,
    pub url: String,
    pub query_params: Vec<KeyValueField>,
    pub headers: Vec<KeyValueField>,
    pub body_type: BodyType,
    pub body: String,
    pub form_fields: Vec<FormField>,
    pub multipart_fields: Vec<MultipartField>,
}

#[derive(Debug, Clone)]
pub struct Collection {
    pub name: String,
    pub expanded: bool,
    pub requests: Vec<Request>,
}
