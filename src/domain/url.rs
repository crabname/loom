use super::{default_key_value_fields, KeyValueField};

pub fn base_url(url: &str) -> String {
    url.split_once('?')
        .map(|(base, _)| base)
        .unwrap_or(url)
        .trim()
        .to_string()
}

pub fn split_query_params(url: &str) -> (String, Vec<KeyValueField>) {
    let base = base_url(url);
    let Some((_, query)) = url.split_once('?') else {
        return (base, default_key_value_fields());
    };

    let mut query_params = Vec::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        query_params.push(KeyValueField {
            enabled: true,
            name: percent_decode(name),
            value: percent_decode(value),
        });
    }

    if query_params.is_empty() {
        (base, default_key_value_fields())
    } else {
        (base, query_params)
    }
}

pub fn ensure_trailing_empty_row(params: &mut Vec<KeyValueField>) {
    if params.is_empty() || !is_empty_row(params.last().expect("checked above")) {
        params.push(KeyValueField::empty());
    }
}

pub fn format_request_url(url: &str, params: &[KeyValueField]) -> String {
    let base = base_url(url);
    match build_url_with_params(&base, params) {
        Ok(url) => url,
        Err(_) if base.is_empty() => format_query_suffix(params),
        Err(_) => base,
    }
}

pub fn build_url_with_params(url: &str, params: &[KeyValueField]) -> Result<String, String> {
    let base = base_url(url);
    if base.is_empty() {
        return Err("URL is empty".into());
    }

    let has_params = params
        .iter()
        .any(|field| field.enabled && !field.name.trim().is_empty());
    if !has_params {
        return Ok(base);
    }

    let mut url = reqwest::Url::parse(&base).map_err(|e| e.to_string())?;
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

fn format_query_suffix(params: &[KeyValueField]) -> String {
    let enabled: Vec<_> = params
        .iter()
        .filter(|field| field.enabled && !field.name.trim().is_empty())
        .collect();
    if enabled.is_empty() {
        return String::new();
    }

    let mut suffix = String::from('?');
    for (index, field) in enabled.iter().enumerate() {
        if index > 0 {
            suffix.push('&');
        }
        suffix.push_str(&percent_encode(field.name.trim()));
        suffix.push('=');
        suffix.push_str(&percent_encode(&field.value));
    }
    suffix
}

fn is_empty_row(field: &KeyValueField) -> bool {
    field.name.trim().is_empty() && field.value.trim().is_empty()
}

pub fn query_params_equal(left: &[KeyValueField], right: &[KeyValueField]) -> bool {
    fn meaningful(params: &[KeyValueField]) -> Vec<(&str, &str, bool)> {
        params
            .iter()
            .filter(|field| !is_empty_row(field))
            .map(|field| (field.name.as_str(), field.value.as_str(), field.enabled))
            .collect()
    }

    meaningful(left) == meaningful(right)
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

pub(crate) fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or(""),
                16,
            )
        {
            decoded.push(byte as char);
            index += 3;
            continue;
        }
        decoded.push(bytes[index] as char);
        index += 1;
    }
    decoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_encoded_query_params() {
        let (base, params) = split_query_params("https://example.com/search?q=hello%20world&page=2");
        assert_eq!(base, "https://example.com/search");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "q");
        assert_eq!(params[0].value, "hello world");
        assert_eq!(params[1].name, "page");
        assert_eq!(params[1].value, "2");
    }

    #[test]
    fn formats_base_only_url() {
        let url = format_request_url(
            "https://example.com/users",
            &[KeyValueField {
                enabled: true,
                name: String::new(),
                value: String::new(),
            }],
        );
        assert_eq!(url, "https://example.com/users");
    }

    #[test]
    fn formats_request_url_with_encoding() {
        let url = format_request_url(
            "https://example.com/search",
            &[
                KeyValueField {
                    enabled: true,
                    name: "q".into(),
                    value: "hello world".into(),
                },
                KeyValueField {
                    enabled: true,
                    name: "page".into(),
                    value: "2".into(),
                },
            ],
        );
        assert_eq!(url, "https://example.com/search?q=hello+world&page=2");
    }

    #[test]
    fn build_url_skips_disabled_params() {
        let url = build_url_with_params(
            "https://example.com/items",
            &[
                KeyValueField {
                    enabled: true,
                    name: "a".into(),
                    value: "1".into(),
                },
                KeyValueField {
                    enabled: false,
                    name: "b".into(),
                    value: "2".into(),
                },
            ],
        )
        .unwrap();
        assert_eq!(url, "https://example.com/items?a=1");
    }
}
