use super::{
    default_form_fields, default_key_value_fields, default_multipart_fields, default_variables,
    format_request_url, split_query_params, BodyType, FormField, HttpMethod, KeyValueField,
    MultipartField, MultipartFieldType, Request, RequestProtocol,
};

pub fn request_to_curl(request: &Request) -> Result<String, String> {
    if request.protocol != RequestProtocol::Http {
        return Err("Only HTTP requests can be exported to cURL".into());
    }

    let url = format_request_url(&request.url, &request.query_params);
    let mut parts = vec!["curl".to_string()];

    if request.method != HttpMethod::Get {
        parts.push(format!("-X {}", request.method.as_str()));
    }

    parts.push(shell_single_quoted(&url));

    for header in enabled_fields(&request.headers) {
        parts.push("-H".into());
        parts.push(shell_single_quoted(&format!("{}: {}", header.name, header.value)));
    }

    match request.body_type {
        BodyType::None => {}
        BodyType::Json | BodyType::Xml => {
            if !request.body.trim().is_empty() {
                if !has_header(&request.headers, "content-type") {
                    let content_type = match request.body_type {
                        BodyType::Json => "application/json",
                        BodyType::Xml => "application/xml",
                        _ => unreachable!(),
                    };
                    parts.push("-H".into());
                    parts.push(shell_single_quoted(&format!("Content-Type: {content_type}")));
                }
                parts.push("--data-raw".into());
                parts.push(shell_single_quoted(&request.body));
            }
        }
        BodyType::FormUrlEncoded => {
            if !has_header(&request.headers, "content-type") {
                parts.push("-H".into());
                parts.push(shell_single_quoted("Content-Type: application/x-www-form-urlencoded"));
            }
            for field in enabled_fields(&request.form_fields) {
                parts.push("--data-urlencode".into());
                parts.push(shell_single_quoted(&format!("{}={}", field.name, field.value)));
            }
        }
        BodyType::Multipart => {
            for field in enabled_multipart_fields(&request.multipart_fields) {
                parts.push("-F".into());
                let value = if field.field_type == MultipartFieldType::File {
                    format!("{}=@{}", field.name, field.value)
                } else {
                    format!("{}={}", field.name, field.value)
                };
                parts.push(shell_single_quoted(&value));
            }
        }
    }

    Ok(parts.join(" \\\n  "))
}

pub fn parse_curl(curl: &str) -> Result<Request, String> {
    let args = tokenize_curl(curl)?;
    if args.is_empty() {
        return Err("Empty cURL command".into());
    }

    let mut method = HttpMethod::Get;
    let mut url = String::new();
    let mut headers = Vec::new();
    let mut data_parts = Vec::new();
    let mut form_parts = Vec::new();
    let mut use_get = false;
    let mut data_urlencoded = false;

    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "-X" | "--request" => {
                let value = next_arg(&args, &mut index, arg)?;
                method = HttpMethod::from_label(&value.to_ascii_uppercase())
                    .ok_or_else(|| format!("Unsupported HTTP method: {value}"))?;
            }
            "-H" | "--header" => {
                let value = next_arg(&args, &mut index, arg)?;
                let (name, header_value) = split_header(&value)?;
                headers.push(KeyValueField {
                    enabled: true,
                    name,
                    value: header_value,
                });
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                data_parts.push(next_arg(&args, &mut index, arg)?);
            }
            "--data-urlencode" => {
                data_urlencoded = true;
                data_parts.push(next_arg(&args, &mut index, arg)?);
            }
            "-F" | "--form" => {
                form_parts.push(next_arg(&args, &mut index, arg)?);
            }
            "-G" | "--get" => {
                use_get = true;
            }
            "--url" => {
                url = next_arg(&args, &mut index, arg)?;
            }
            "--compressed" | "-k" | "--insecure" | "-L" | "--location" | "-s" | "--silent"
            | "-i" | "--include" | "-v" | "--verbose" | "-b" | "--cookie" | "-c"
            | "--cookie-jar" | "-A" | "--user-agent" | "-u" | "--user" | "--connect-timeout"
            | "--max-time" | "-m" | "--proxy" | "-x" | "--http1.1" | "--http2" | "--http3"
            | "-0" | "--http1.0" | "--tlsv1.2" | "--tlsv1.3" | "--cacert" | "--cert"
            | "--key" | "--pass" | "--pinnedpubkey" | "--resolve" | "--retry" | "--retry-delay"
            | "--retry-max-time" | "--speed-limit" | "--speed-time" | "--limit-rate"
            | "--globoff" | "--noproxy" | "--path-as-is" | "--aws-sigv4" | "--oauth2-bearer"
            | "--json" => {
                let _ = next_arg(&args, &mut index, arg).ok();
            }
            value if is_url(value) => {
                if url.is_empty() {
                    url = value.to_string();
                }
            }
            value if !value.starts_with('-') => {}
            _ => {
                let _ = next_arg(&args, &mut index, arg).ok();
            }
        }
        index += 1;
    }

    if url.is_empty() {
        return Err("cURL command is missing a URL".into());
    }

    let (base_url, mut query_params) = split_query_params(&url);
    if use_get && !data_parts.is_empty() {
        for part in &data_parts {
            if let Some((name, value)) = part.split_once('=') {
                query_params.push(KeyValueField {
                    enabled: true,
                    name: name.to_string(),
                    value: value.to_string(),
                });
            } else {
                query_params.push(KeyValueField {
                    enabled: true,
                    name: part.clone(),
                    value: String::new(),
                });
            }
        }
        data_parts.clear();
    }

    if !data_parts.is_empty() && method == HttpMethod::Get && !use_get {
        method = HttpMethod::Post;
    }

    let content_type = headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case("content-type"))
        .map(|header| header.value.to_ascii_lowercase());

    let (body_type, body, form_fields, multipart_fields) = if !form_parts.is_empty() {
        let multipart_fields = form_parts
            .into_iter()
            .map(parse_multipart_field)
            .collect::<Result<Vec<_>, _>>()?;
        (BodyType::Multipart, String::new(), default_form_fields(), multipart_fields)
    } else if data_urlencoded || content_type.as_deref() == Some("application/x-www-form-urlencoded") {
        let form_fields = data_parts
            .into_iter()
            .map(parse_form_field)
            .collect::<Result<Vec<_>, _>>()?;
        (BodyType::FormUrlEncoded, String::new(), form_fields, default_multipart_fields())
    } else if data_parts.is_empty() {
        (BodyType::None, String::new(), default_form_fields(), default_multipart_fields())
    } else {
        let body = data_parts.join("&");
        let body_type = match content_type.as_deref() {
            Some(ct) if ct.contains("json") => BodyType::Json,
            Some(ct) if ct.contains("xml") => BodyType::Xml,
            Some(ct) if ct.contains("x-www-form-urlencoded") => {
                let form_fields = parse_form_body(&body)?;
                return Ok(Request {
                    name: "Imported Request".into(),
                    protocol: RequestProtocol::Http,
                    method,
                    url: base_url,
                    query_params,
                    headers,
                    body_type: BodyType::FormUrlEncoded,
                    body: String::new(),
                    form_fields,
                    multipart_fields: default_multipart_fields(),
                    variables: default_variables(),
                });
            }
            _ => BodyType::Json,
        };
        (body_type, body, default_form_fields(), default_multipart_fields())
    };

    Ok(Request {
        name: "Imported Request".into(),
        protocol: RequestProtocol::Http,
        method,
        url: base_url,
        query_params,
        headers,
        body_type,
        body,
        form_fields,
        multipart_fields,
        variables: default_variables(),
    })
}

fn tokenize_curl(input: &str) -> Result<Vec<String>, String> {
    let mut normalized = input.trim().to_string();
    if normalized.is_empty() {
        return Err("Empty cURL command".into());
    }

    if let Some(rest) = normalized.strip_prefix("curl ") {
        normalized = rest.to_string();
    } else if normalized.eq_ignore_ascii_case("curl") {
        normalized.clear();
    }

    normalized = normalized.replace("\\\r\n", " ");
    normalized = normalized.replace("\\\n", " ");
    normalized = normalized.replace('\n', " ");
    normalized = normalized.replace('\r', " ");

    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in normalized.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' && quote.is_none() {
            escaped = true;
            continue;
        }

        match quote {
            Some('\'') if ch == '\'' => quote = None,
            Some('"') if ch == '"' => quote = None,
            Some(_) => current.push(ch),
            None if ch == '\'' || ch == '"' => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            None => current.push(ch),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    Ok(args)
}

fn next_arg(args: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    let next = args.get(*index + 1).ok_or_else(|| format!("Missing value for {flag}"))?;
    *index += 1;
    Ok(next.clone())
}

fn split_header(value: &str) -> Result<(String, String), String> {
    let (name, header_value) = value
        .split_once(':')
        .ok_or_else(|| format!("Invalid header: {value}"))?;
    Ok((name.trim().to_string(), header_value.trim().to_string()))
}

fn parse_form_field(part: String) -> Result<FormField, String> {
    let (name, value) = part
        .split_once('=')
        .ok_or_else(|| format!("Invalid form field: {part}"))?;
    Ok(FormField {
        enabled: true,
        name: name.to_string(),
        value: value.to_string(),
    })
}

fn parse_form_body(body: &str) -> Result<Vec<FormField>, String> {
    let mut fields = Vec::new();
    for pair in body.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        fields.push(FormField {
            enabled: true,
            name: super::url::percent_decode(name),
            value: super::url::percent_decode(value),
        });
    }
    if fields.is_empty() {
        Ok(default_form_fields())
    } else {
        Ok(fields)
    }
}

fn parse_multipart_field(part: String) -> Result<MultipartField, String> {
    let (name, value) = part
        .split_once('=')
        .ok_or_else(|| format!("Invalid multipart field: {part}"))?;
    if let Some(path) = value.strip_prefix('@') {
        Ok(MultipartField {
            enabled: true,
            name: name.to_string(),
            value: path.to_string(),
            field_type: MultipartFieldType::File,
            content_type: String::new(),
        })
    } else {
        Ok(MultipartField {
            enabled: true,
            name: name.to_string(),
            value: value.to_string(),
            field_type: MultipartFieldType::Text,
            content_type: String::new(),
        })
    }
}

fn enabled_fields(fields: &[KeyValueField]) -> impl Iterator<Item = &KeyValueField> {
    fields
        .iter()
        .filter(|field| field.enabled && !field.name.trim().is_empty())
}

fn enabled_multipart_fields(
    fields: &[MultipartField],
) -> impl Iterator<Item = &MultipartField> {
    fields
        .iter()
        .filter(|field| field.enabled && !field.name.trim().is_empty())
}

fn has_header(headers: &[KeyValueField], name: &str) -> bool {
    headers.iter().any(|header| {
        header.enabled && header.name.eq_ignore_ascii_case(name)
    })
}

fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn shell_single_quoted(value: &str) -> String {
    if value.contains('\'') {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    } else {
        format!("'{value}'")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_simple_get() {
        let request = Request {
            name: "Test".into(),
            protocol: RequestProtocol::Http,
            method: HttpMethod::Get,
            url: "https://example.com/users".into(),
            query_params: default_key_value_fields(),
            headers: default_key_value_fields(),
            body_type: BodyType::None,
            body: String::new(),
            form_fields: default_form_fields(),
            multipart_fields: default_multipart_fields(),
            variables: default_variables(),
        };

        let curl = request_to_curl(&request).unwrap();
        assert!(curl.contains("curl"));
        assert!(curl.contains("'https://example.com/users'"));
        assert!(!curl.contains("-X"));
    }

    #[test]
  fn exports_post_with_json() {
        let request = Request {
            name: "Create".into(),
            protocol: RequestProtocol::Http,
            method: HttpMethod::Post,
            url: "https://example.com/users".into(),
            query_params: default_key_value_fields(),
            headers: vec![KeyValueField {
                enabled: true,
                name: "Authorization".into(),
                value: "Bearer token".into(),
            }],
            body_type: BodyType::Json,
            body: r#"{"name":"Alice"}"#.into(),
            form_fields: default_form_fields(),
            multipart_fields: default_multipart_fields(),
            variables: default_variables(),
        };

        let curl = request_to_curl(&request).unwrap();
        assert!(curl.contains("-X POST"));
        assert!(curl.contains("Authorization: Bearer token"));
        assert!(curl.contains("--data-raw"));
        assert!(curl.contains(r#"{"name":"Alice"}"#));
    }

    #[test]
    fn round_trips_post_json() {
        let curl = r#"curl -X POST 'https://example.com/users' \
  -H 'Authorization: Bearer token' \
  -H 'Content-Type: application/json' \
  --data-raw '{"name":"Alice"}'"#;

        let request = parse_curl(curl).unwrap();
        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "https://example.com/users");
        assert_eq!(request.body_type, BodyType::Json);
        assert_eq!(request.body, r#"{"name":"Alice"}"#);
        assert!(request.headers.iter().any(|header| {
            header.name == "Authorization" && header.value == "Bearer token"
        }));
    }

    #[test]
    fn parses_url_query_params() {
        let curl = "curl 'https://example.com/search?q=rust&page=2'";
        let request = parse_curl(curl).unwrap();
        assert_eq!(request.url, "https://example.com/search");
        assert_eq!(request.query_params.len(), 2);
        assert_eq!(request.query_params[0].name, "q");
        assert_eq!(request.query_params[0].value, "rust");
        assert_eq!(request.query_params[1].name, "page");
        assert_eq!(request.query_params[1].value, "2");
    }

    #[test]
    fn parses_multipart_form() {
        let curl = r#"curl -F 'name=Alice' -F 'avatar=@/tmp/picture.png' 'https://example.com/upload'"#;
        let request = parse_curl(curl).unwrap();
        assert_eq!(request.body_type, BodyType::Multipart);
        assert_eq!(request.multipart_fields.len(), 2);
        assert_eq!(request.multipart_fields[0].field_type, MultipartFieldType::Text);
        assert_eq!(request.multipart_fields[1].field_type, MultipartFieldType::File);
    }

    #[test]
    fn round_trips_exported_request() {
        let request = Request {
            name: "Search".into(),
            protocol: RequestProtocol::Http,
            method: HttpMethod::Get,
            url: "https://example.com/search".into(),
            query_params: vec![
                KeyValueField {
                    enabled: true,
                    name: "q".into(),
                    value: "rust".into(),
                },
                KeyValueField {
                    enabled: true,
                    name: "page".into(),
                    value: "2".into(),
                },
            ],
            headers: default_key_value_fields(),
            body_type: BodyType::None,
            body: String::new(),
            form_fields: default_form_fields(),
            multipart_fields: default_multipart_fields(),
            variables: default_variables(),
        };

        let curl = request_to_curl(&request).unwrap();
        let parsed = parse_curl(&curl).unwrap();
        assert_eq!(parsed.method, request.method);
        assert_eq!(parsed.url, request.url);
        assert_eq!(parsed.query_params.len(), 2);
        assert_eq!(parsed.query_params[0].value, "rust");
    }
}
