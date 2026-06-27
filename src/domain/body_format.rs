use crate::domain::BodyType;

pub fn format_json(body: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(body.trim()).map_err(|error| error.to_string())?;
    serde_json::to_string_pretty(&value).map_err(|error| error.to_string())
}

pub fn format_xml(body: &str) -> Result<String, String> {
    use xmltree::{Element, EmitterConfig};

    let root = Element::parse(body.trim().as_bytes()).map_err(|error| error.to_string())?;
    let mut output = Vec::new();
    root.write_with_config(
        &mut output,
        EmitterConfig::new().perform_indent(true),
    )
    .map_err(|error| error.to_string())?;
    String::from_utf8(output).map_err(|error| error.to_string())
}

pub fn format_body(body_type: BodyType, body: &str) -> Result<String, String> {
    match body_type {
        BodyType::Json => format_json(body),
        BodyType::Xml => format_xml(body),
        _ => Err("Formatting is only supported for JSON and XML bodies".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_json() {
        let formatted = format_json(r#"{"a":1,"b":[2,3]}"#).unwrap();
        assert_eq!(formatted, "{\n  \"a\": 1,\n  \"b\": [\n    2,\n    3\n  ]\n}");
    }

    #[test]
    fn formats_xml() {
        let formatted = format_xml("<root><item>value</item></root>").unwrap();
        assert!(formatted.contains("<root>"));
        assert!(formatted.contains("<item>value</item>"));
    }
}
