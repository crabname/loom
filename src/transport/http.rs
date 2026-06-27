use crate::domain::{
    build_url_with_params, classify_response_body, format_response_size, response_content_type,
    BodyType, FormField, HttpMethod, KeyValueField, MultipartField, MultipartFieldType,
    ResponseBody,
};

/// Maximum response body size kept in memory (10 MiB).
const MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct HttpRequestBody {
    pub body_type: BodyType,
    pub raw_body: String,
    pub form_fields: Vec<FormField>,
    pub multipart_fields: Vec<MultipartField>,
}

pub async fn send_http_request(
    url: String,
    method: HttpMethod,
    query_params: Vec<KeyValueField>,
    headers: Vec<KeyValueField>,
    body: HttpRequestBody,
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

    let request = apply_body(request, body).await?;

    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    let headers = parse_response_headers(response.headers());
    let bytes = read_response_bytes_limited(response, MAX_RESPONSE_BYTES).await?;
    let content_type = response_content_type(&headers);
    let body = classify_response_body(&bytes, content_type.as_deref());
    let elapsed_ms = started.elapsed().as_millis();

    Ok(HttpResponse {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("Unknown").into(),
        headers,
        body,
        elapsed_ms,
        size_bytes: bytes.len(),
    })
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<KeyValueField>,
    pub body: ResponseBody,
    pub elapsed_ms: u128,
    pub size_bytes: usize,
}

fn response_too_large_error(reported_size: usize, limit: usize) -> String {
    format!(
        "Response body exceeds the {} limit (reported size: {})",
        format_response_size(limit),
        format_response_size(reported_size),
    )
}

async fn read_response_bytes_limited(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    if let Some(len) = response.content_length() {
        let len = len as usize;
        if len > max_bytes {
            return Err(response_too_large_error(len, max_bytes));
        }
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if body.len() + chunk.len() > max_bytes {
            return Err(response_too_large_error(body.len() + chunk.len(), max_bytes));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
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
    body: HttpRequestBody,
) -> Result<reqwest::RequestBuilder, String> {
    match body.body_type {
        BodyType::None => Ok(request),
        BodyType::Json => {
            if body.raw_body.trim().is_empty() {
                return Ok(request);
            }
            Ok(request
                .header("Content-Type", "application/json")
                .body(body.raw_body))
        }
        BodyType::Xml => {
            if body.raw_body.trim().is_empty() {
                return Ok(request);
            }
            Ok(request
                .header("Content-Type", "application/xml")
                .body(body.raw_body))
        }
        BodyType::FormUrlEncoded => build_form_urlencoded(request, &body.form_fields),
        BodyType::Multipart => build_multipart(request, &body.multipart_fields).await,
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

