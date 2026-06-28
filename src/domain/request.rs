use super::{
    default_form_fields, default_key_value_fields, default_multipart_fields, default_variables,
    EntityId, Environment, FormField, KeyValueField, MultipartField, Variable,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RequestProtocol {
    #[default]
    Http,
    Grpc,
}

impl RequestProtocol {
    pub const ALL: [Self; 2] = [Self::Http, Self::Grpc];

    pub fn label(self) -> &'static str {
        match self {
            Self::Http => "HTTP",
            Self::Grpc => "gRPC",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|protocol| protocol.label().eq_ignore_ascii_case(label))
    }

    pub fn list_badge(self, http_method: HttpMethod) -> &'static str {
        match self {
            Self::Http => http_method.as_str(),
            Self::Grpc => "gRPC",
        }
    }
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
            id: EntityId::new(),
            name: name.into(),
            protocol: RequestProtocol::default(),
            method: HttpMethod::Get,
            url: String::new(),
            grpc_service: String::new(),
            grpc_method: String::new(),
            query_params: default_key_value_fields(),
            headers: default_key_value_fields(),
            body_type: BodyType::None,
            body: String::new(),
            form_fields: default_form_fields(),
            multipart_fields: default_multipart_fields(),
            variables: default_variables(),
            pre_request_script: String::new(),
            post_response_script: String::new(),
            tests_script: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    pub id: EntityId,
    pub name: String,
    pub protocol: RequestProtocol,
    pub method: HttpMethod,
    pub url: String,
    pub grpc_service: String,
    pub grpc_method: String,
    pub query_params: Vec<KeyValueField>,
    pub headers: Vec<KeyValueField>,
    pub body_type: BodyType,
    pub body: String,
    pub form_fields: Vec<FormField>,
    pub multipart_fields: Vec<MultipartField>,
    pub variables: Vec<Variable>,
    pub pre_request_script: String,
    pub post_response_script: String,
    pub tests_script: String,
}

#[derive(Debug, Clone)]
pub struct CollectionFolder {
    pub id: EntityId,
    pub name: String,
    pub expanded: bool,
    pub variables: Vec<Variable>,
    pub requests: Vec<Request>,
}

impl CollectionFolder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: EntityId::new(),
            name: name.into(),
            expanded: true,
            variables: default_variables(),
            requests: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Collection {
    pub id: EntityId,
    pub name: String,
    pub expanded: bool,
    pub variables: Vec<Variable>,
    pub environments: Vec<Environment>,
    pub folders: Vec<CollectionFolder>,
    pub requests: Vec<Request>,
}

impl Collection {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: EntityId::new(),
            name: name.into(),
            expanded: true,
            variables: default_variables(),
            environments: Vec::new(),
            folders: Vec::new(),
            requests: Vec::new(),
        }
    }

    pub fn request_ref(&self, folder: Option<usize>, index: usize) -> Option<&Request> {
        match folder {
            None => self.requests.get(index),
            Some(folder_index) => self
                .folders
                .get(folder_index)
                .and_then(|folder| folder.requests.get(index)),
        }
    }

    pub fn request_mut(&mut self, folder: Option<usize>, index: usize) -> Option<&mut Request> {
        match folder {
            None => self.requests.get_mut(index),
            Some(folder_index) => self
                .folders
                .get_mut(folder_index)
                .and_then(|folder| folder.requests.get_mut(index)),
        }
    }

    pub fn push_request(&mut self, folder: Option<usize>, request: Request) -> usize {
        match folder {
            None => {
                let index = self.requests.len();
                self.requests.push(request);
                index
            }
            Some(folder_index) => {
                let folder = &mut self.folders[folder_index];
                folder.expanded = true;
                let index = folder.requests.len();
                folder.requests.push(request);
                index
            }
        }
    }

    pub fn remove_request(&mut self, folder: Option<usize>, index: usize) {
        match folder {
            None => {
                self.requests.remove(index);
            }
            Some(folder_index) => {
                self.folders[folder_index].requests.remove(index);
            }
        }
    }

    pub fn remove_folder(&mut self, index: usize) {
        self.folders.remove(index);
    }

    pub fn first_request_location(&self) -> Option<(Option<usize>, usize)> {
        if !self.requests.is_empty() {
            return Some((None, 0));
        }

        for (folder_index, folder) in self.folders.iter().enumerate() {
            if !folder.requests.is_empty() {
                return Some((Some(folder_index), 0));
            }
        }

        None
    }
}
