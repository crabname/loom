use crate::domain::KeyValueField;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseBody {
    Text(String),
    Binary {
        size: usize,
        content_type: Option<String>,
    },
}

impl ResponseBody {
    pub fn empty() -> Self {
        Self::Text(String::new())
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Text(text) => text.is_empty(),
            Self::Binary { .. } => false,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Binary { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseBodyView {
    Raw,
    Preview,
}

pub fn response_content_type(headers: &[KeyValueField]) -> Option<String> {
    headers.iter().find_map(|header| {
        if header.name.eq_ignore_ascii_case("content-type") {
            let content_type = header
                .value
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            if content_type.is_empty() {
                None
            } else {
                Some(content_type)
            }
        } else {
            None
        }
    })
}

pub fn is_binary_content_type(content_type: &str) -> bool {
    let content_type = content_type.trim().to_ascii_lowercase();
    if content_type.starts_with("text/") {
        return false;
    }

    if matches!(
        content_type.as_str(),
        "application/json"
            | "application/ld+json"
            | "application/xml"
            | "application/javascript"
            | "application/ecmascript"
            | "application/x-javascript"
            | "application/problem+json"
            | "application/problem+xml"
            | "application/xhtml+xml"
    ) {
        return false;
    }

    content_type.starts_with("image/")
        || content_type.starts_with("video/")
        || content_type.starts_with("audio/")
        || content_type.starts_with("font/")
        || matches!(
            content_type.as_str(),
            "application/octet-stream"
                | "application/pdf"
                | "application/zip"
                | "application/gzip"
                | "application/x-gzip"
                | "application/x-tar"
                | "application/vnd.ms-excel"
                | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                | "application/msword"
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )
}

pub fn is_html_content(content_type: Option<&str>, body: &str) -> bool {
    if let Some(content_type) = content_type {
        if content_type.contains("html") {
            return true;
        }
        if content_type.contains("xml") {
            return false;
        }
    }

    let trimmed = body.trim_start();
    trimmed.starts_with("<!DOCTYPE")
        || trimmed.starts_with("<!doctype")
        || trimmed.starts_with("<html")
        || trimmed.starts_with("<HTML")
}

pub fn response_body_language(
    content_type: Option<&str>,
    body: &str,
) -> Option<&'static str> {
    if let Some(content_type) = content_type {
        if content_type.contains("json") {
            return Some("json");
        }
        if content_type.contains("xml") {
            return Some("html");
        }
        if content_type.contains("html") {
            return Some("html");
        }
    }

    let trimmed = body.trim_start();
    if (trimmed.starts_with('{') || trimmed.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
    {
        return Some("json");
    }
    if trimmed.starts_with("<?xml")
        || (trimmed.starts_with('<') && trimmed.contains('>') && !is_html_content(None, body))
    {
        return Some("html");
    }

    None
}

pub fn classify_response_body(
    bytes: &[u8],
    content_type: Option<&str>,
) -> ResponseBody {
    if let Some(content_type) = content_type {
        if is_binary_content_type(content_type) {
            return ResponseBody::Binary {
                size: bytes.len(),
                content_type: Some(content_type.to_string()),
            };
        }
    }

    match std::str::from_utf8(bytes) {
        Ok(text) => ResponseBody::Text(prettify_text_body(text, content_type)),
        Err(_) => ResponseBody::Binary {
            size: bytes.len(),
            content_type: content_type.map(str::to_string),
        },
    }
}

fn prettify_text_body(body: &str, content_type: Option<&str>) -> String {
    let should_prettify_json = content_type.is_none_or(|content_type| content_type.contains("json"));
    if should_prettify_json {
        crate::domain::format_json(body).unwrap_or_else(|_| body.to_string())
    } else {
        body.to_string()
    }
}

pub fn format_binary_body_message(size: usize, content_type: Option<&str>) -> String {
    let size_label = format_binary_size(size);
    match content_type {
        Some(content_type) => format!("Binary response ({content_type}, {size_label})"),
        None => format!("Binary response ({size_label})"),
    }
}

fn format_binary_size(size: usize) -> String {
    const KIB: f64 = 1024.0;
    let size = size as f64;
    if size < KIB {
        format!("{size} B")
    } else if size < KIB * KIB {
        format!("{:.1} KB", size / KIB)
    } else if size < KIB * KIB * KIB {
        format!("{:.1} MB", size / (KIB * KIB))
    } else {
        format!("{:.1} GB", size / (KIB * KIB * KIB))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_binary_image() {
        let body = classify_response_body(&[0x89, 0x50, 0x4E, 0x47], Some("image/png"));
        assert_eq!(
            body,
            ResponseBody::Binary {
                size: 4,
                content_type: Some("image/png".into()),
            }
        );
    }

    #[test]
    fn classifies_invalid_utf8_as_binary() {
        let body = classify_response_body(&[0xFF, 0xFE, 0xFD], None);
        assert!(matches!(body, ResponseBody::Binary { size: 3, .. }));
    }

    #[test]
    fn classifies_json_text() {
        let body = classify_response_body(br#"{"a":1}"#, Some("application/json"));
        assert_eq!(body.text(), Some("{\n  \"a\": 1\n}"));
    }

    #[test]
    fn detects_html_content() {
        assert!(is_html_content(Some("text/html"), ""));
        assert!(is_html_content(None, "<!DOCTYPE html><html></html>"));
        assert!(!is_html_content(Some("application/xml"), "<root/>"));
    }
}
