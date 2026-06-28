use std::collections::HashMap;

use serde_json::Value;

use crate::domain::{
    default_form_fields, default_key_value_fields, default_multipart_fields, default_variables,
    BodyType, FormField, HttpMethod, KeyValueField, MultipartField, Request, RequestProtocol,
    RequestTimingBreakdown, ResponseBody, ResponseBodyView, Variable,
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
    Vars,
    Script,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestScriptSubTab {
    PreRequest,
    PostResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabSource {
    pub workspace: usize,
    pub collection: usize,
    pub folder: Option<usize>,
    pub request: usize,
}

#[derive(Debug, Clone)]
pub struct WorkspaceSession {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub active_environment: Option<crate::domain::EnvironmentRef>,
    pub runtime_vars: HashMap<String, Value>,
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
    pub request_script_sub_tab: RequestScriptSubTab,
    pub body_type: BodyType,
    pub request_body: String,
    pub form_fields: Vec<FormField>,
    pub multipart_fields: Vec<MultipartField>,
    pub variables: Vec<Variable>,
    pub pre_request_script: String,
    pub post_response_script: String,
    pub response_panel_tab: ResponsePanelTab,
    pub response_body: ResponseBody,
    pub response_body_view: ResponseBodyView,
    pub response_headers: Vec<KeyValueField>,
    pub response_http_status: Option<u16>,
    pub response_status_text: Option<String>,
    pub response_elapsed_ms: Option<u128>,
    pub response_timing: Option<RequestTimingBreakdown>,
    pub response_size_bytes: Option<usize>,
    pub response_error: Option<String>,
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
            request_script_sub_tab: RequestScriptSubTab::PreRequest,
            body_type: request.body_type,
            request_body: request.body.clone(),
            form_fields: request.form_fields.clone(),
            multipart_fields: request.multipart_fields.clone(),
            variables: request.variables.clone(),
            pre_request_script: request.pre_request_script.clone(),
            post_response_script: request.post_response_script.clone(),
            response_panel_tab: ResponsePanelTab::Body,
            response_body: ResponseBody::empty(),
            response_body_view: ResponseBodyView::Raw,
            response_headers: Vec::new(),
            response_http_status: None,
            response_status_text: None,
            response_elapsed_ms: None,
            response_timing: None,
            response_size_bytes: None,
            response_error: None,
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
            request_script_sub_tab: RequestScriptSubTab::PreRequest,
            body_type: BodyType::None,
            request_body: String::new(),
            form_fields: default_form_fields(),
            multipart_fields: default_multipart_fields(),
            variables: default_variables(),
            pre_request_script: String::new(),
            post_response_script: String::new(),
            response_panel_tab: ResponsePanelTab::Body,
            response_body: ResponseBody::empty(),
            response_body_view: ResponseBodyView::Raw,
            response_headers: Vec::new(),
            response_http_status: None,
            response_status_text: None,
            response_elapsed_ms: None,
            response_timing: None,
            response_size_bytes: None,
            response_error: None,
            loading: false,
        }
    }

    pub fn to_request(&self) -> Request {
        Request {
            name: self.title.clone(),
            protocol: RequestProtocol::Http,
            method: self.method,
            url: self.url.clone(),
            query_params: self.query_params.clone(),
            headers: self.headers.clone(),
            body_type: self.body_type,
            body: self.request_body.clone(),
            form_fields: self.form_fields.clone(),
            multipart_fields: self.multipart_fields.clone(),
            variables: self.variables.clone(),
            pre_request_script: self.pre_request_script.clone(),
            post_response_script: self.post_response_script.clone(),
        }
    }
}
