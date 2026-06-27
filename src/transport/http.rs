use crate::domain::{
    classify_response_body, response_content_type, BodyType, FormField, HttpMethod, KeyValueField,
    MultipartField, MultipartFieldType, ResponseBody,
};

pub fn build_url_with_params(base_url: &str, params: &[KeyValueField]) -> Result<String, String> {
    let base = base_url.split('?').next().unwrap_or(base_url).trim();
    if base.is_empty() {
        return Err("URL is empty".into());
    }

    let mut url = reqwest::Url::parse(base).map_err(|e| e.to_string())?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.clear();
        for field in params {
            if field.enabled && !field.name.trim().is_empty() {
                pairs.append_pair(field.name.trim(), &field.value);
            }
        }
    }

    Ok(url.to_string())
}

pub async fn send_http_request(
    url: String,
    method: HttpMethod,
    query_params: Vec<KeyValueField>,
    headers: Vec<KeyValueField>,
    body_type: BodyType,
    raw_body: String,
    form_fields: Vec<FormField>,
    multipart_fields: Vec<MultipartField>,
) -> Result<HttpResponse, String> {
    let url = build_url_with_params(&url, &query_params)?;

    let client = reqwest::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let started = std::time::Instant::now();

    let request = match method {
        HttpMethod::Get => client.get(&url),
        HttpMethod::Post => client.post(&url),
        HttpMethod::Put => client.put(&url),
        HttpMethod::Patch => client.patch(&url),
        HttpMethod::Delete => client.delete(&url),
    };

    let request = apply_headers(request, &headers);

    let request = apply_body(request, body_type, raw_body, form_fields, multipart_fields).await?;

    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    let headers = parse_response_headers(response.headers());
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let content_type = response_content_type(&headers);
    let body = classify_response_body(&bytes, content_type.as_deref());
    let elapsed_ms = started.elapsed().as_millis();

    Ok(HttpResponse {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("Unknown").into(),
        headers,
        body,
        elapsed_ms,
    })
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<KeyValueField>,
    pub body: ResponseBody,
    pub elapsed_ms: u128,
}

fn parse_response_headers(headers: &reqwest::header::HeaderMap) -> Vec<KeyValueField> {
    let mut fields: Vec<KeyValueField> = headers
        .iter()
        .map(|(name, value)| KeyValueField {
            enabled: true,
            name: name.to_string(),
            value: value.to_str().unwrap_or("<invalid utf-8>").to_string(),
        })
        .collect();

    fields.sort_by(|a, b| a.name.cmp(&b.name));
    fields
}

fn apply_headers(
    request: reqwest::RequestBuilder,
    headers: &[KeyValueField],
) -> reqwest::RequestBuilder {
    let mut request = request;
    for field in headers {
        if field.enabled && !field.name.trim().is_empty() {
            request = request.header(field.name.trim(), field.value.as_str());
        }
    }
    request
}

async fn apply_body(
    request: reqwest::RequestBuilder,
    body_type: BodyType,
    raw_body: String,
    form_fields: Vec<FormField>,
    multipart_fields: Vec<MultipartField>,
) -> Result<reqwest::RequestBuilder, String> {
    match body_type {
        BodyType::None => Ok(request),
        BodyType::Json => {
            if raw_body.trim().is_empty() {
                return Ok(request);
            }
            Ok(request
                .header("Content-Type", "application/json")
                .body(raw_body))
        }
        BodyType::Xml => {
            if raw_body.trim().is_empty() {
                return Ok(request);
            }
            Ok(request
                .header("Content-Type", "application/xml")
                .body(raw_body))
        }
        BodyType::FormUrlEncoded => build_form_urlencoded(request, &form_fields),
        BodyType::Multipart => build_multipart(request, &multipart_fields).await,
    }
}

fn build_form_urlencoded(
    request: reqwest::RequestBuilder,
    fields: &[FormField],
) -> Result<reqwest::RequestBuilder, String> {
    let pairs: Vec<(String, String)> = fields
        .iter()
        .filter(|field| field.enabled && !field.name.trim().is_empty())
        .map(|field| (field.name.clone(), field.value.clone()))
        .collect();

    if pairs.is_empty() {
        return Err("Form-urlencoded body has no fields".into());
    }

    Ok(request.form(&pairs))
}

async fn build_multipart(
    request: reqwest::RequestBuilder,
    fields: &[MultipartField],
) -> Result<reqwest::RequestBuilder, String> {
    let mut form = reqwest::multipart::Form::new();
    let mut parts = 0;

    for field in fields {
        if !field.enabled || field.name.trim().is_empty() {
            continue;
        }

        let part = match field.field_type {
            MultipartFieldType::Text => {
                let mut part = reqwest::multipart::Part::text(field.value.clone());
                let mime = field.content_type.trim();
                if !mime.is_empty() {
                    part = part.mime_str(mime).map_err(|e| e.to_string())?;
                }
                part
            }
            MultipartFieldType::File => {
                let path = std::path::Path::new(field.value.trim());
                if field.value.trim().is_empty() {
                    return Err(format!("File path is empty for field '{}'", field.name));
                }
                if !path.exists() {
                    return Err(format!("File not found: {}", field.value.trim()));
                }
                let bytes = std::fs::read(path)
                    .map_err(|e| format!("Failed to read file {}: {e}", field.value.trim()))?;
                let file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file")
                    .to_string();
                let mut part = reqwest::multipart::Part::bytes(bytes).file_name(file_name);
                let mime = field.content_type.trim();
                if !mime.is_empty() {
                    part = part.mime_str(mime).map_err(|e| e.to_string())?;
                }
                part
            }
        };
        form = form.part(field.name.clone(), part);
        parts += 1;
    }

    if parts == 0 {
        return Err("Multipart body has no fields".into());
    }

    Ok(request.multipart(form))
}

