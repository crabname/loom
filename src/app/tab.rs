use crate::domain::{
    default_form_fields, default_key_value_fields, default_multipart_fields, BodyType,
    FormField, HttpMethod, KeyValueField, MultipartField, Request,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponsePanelTab {
    Body,
    Headers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestPanelTab {
    Params,
    Headers,
    Body,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabSource {
    pub collection: usize,
    pub request: usize,
}

#[derive(Debug, Clone)]
pub struct Tab {
    pub id: usize,
    pub title: String,
    pub source: Option<TabSource>,
    pub url: String,
    pub method: HttpMethod,
    pub query_params: Vec<KeyValueField>,
    pub headers: Vec<KeyValueField>,
    pub request_panel_tab: RequestPanelTab,
    pub body_type: BodyType,
    pub request_body: String,
    pub form_fields: Vec<FormField>,
    pub multipart_fields: Vec<MultipartField>,
    pub response_panel_tab: ResponsePanelTab,
    pub response_body: String,
    pub response_headers: Vec<KeyValueField>,
    pub response_status: Option<String>,
    pub loading: bool,
}

impl Tab {
    pub fn from_request(id: usize, request: &Request, source: Option<TabSource>) -> Self {
        Self {
            id,
            title: request.name.clone(),
            source,
            url: request.url.clone(),
            method: request.method,
            query_params: request.query_params.clone(),
            headers: request.headers.clone(),
            request_panel_tab: RequestPanelTab::Body,
            body_type: request.body_type,
            request_body: request.body.clone(),
            form_fields: request.form_fields.clone(),
            multipart_fields: request.multipart_fields.clone(),
            response_panel_tab: ResponsePanelTab::Body,
            response_body: String::new(),
            response_headers: Vec::new(),
            response_status: None,
            loading: false,
        }
    }

    pub fn empty(id: usize, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            source: None,
            url: String::new(),
            method: HttpMethod::Get,
            query_params: default_key_value_fields(),
            headers: default_key_value_fields(),
            request_panel_tab: RequestPanelTab::Body,
            body_type: BodyType::None,
            request_body: String::new(),
            form_fields: default_form_fields(),
            multipart_fields: default_multipart_fields(),
            response_panel_tab: ResponsePanelTab::Body,
            response_body: String::new(),
            response_headers: Vec::new(),
            response_status: None,
            loading: false,
        }
    }
}
