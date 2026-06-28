use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use boa_engine::native_function::NativeFunction;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{Context, JsData, JsNativeError, JsResult, JsValue, js_string};
use boa_gc::{Finalize, Trace};
use serde_json::Value;

use crate::domain::{KeyValueField, ResponseBody};
use crate::transport::HttpResponse;

#[derive(Debug, Clone)]
pub struct ScriptResponseSnapshot {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<KeyValueField>,
    pub body: ResponseBody,
    pub response_time_ms: u128,
    pub size_bytes: usize,
    pub url: String,
}

impl ScriptResponseSnapshot {
    pub fn from_http(response: &HttpResponse, url: String) -> Self {
        Self {
            status: response.status,
            status_text: response.status_text.clone(),
            headers: response.headers.clone(),
            body: response.body.clone(),
            response_time_ms: response.elapsed_ms,
            size_bytes: response.size_bytes,
            url,
        }
    }

    pub fn body_json_value(&self) -> Value {
        match &self.body {
            ResponseBody::Text(text) => parse_text_body_value(text),
            ResponseBody::Binary { size, content_type } => {
                let mut map = serde_json::Map::new();
                map.insert("_binary".into(), Value::Bool(true));
                map.insert("_size".into(), Value::Number((*size).into()));
                if let Some(content_type) = content_type {
                    map.insert("_contentType".into(), Value::String(content_type.clone()));
                }
                Value::Object(map)
            }
        }
    }

    pub fn headers_object(&self) -> HashMap<String, Value> {
        let mut headers = HashMap::new();
        for header in &self.headers {
            if !header.enabled || header.name.trim().is_empty() {
                continue;
            }
            headers.insert(
                header.name.clone(),
                Value::String(header.value.clone()),
            );
        }
        headers
    }

    pub fn get_header(&self, name: &str) -> Option<String> {
        self.headers.iter().find_map(|header| {
            if header.enabled && header.name.eq_ignore_ascii_case(name) {
                Some(header.value.clone())
            } else {
                None
            }
        })
    }
}

fn parse_text_body_value(text: &str) -> Value {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Value::String(String::new());
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return value;
    }

    Value::String(text.to_string())
}

#[derive(Clone, Trace, Finalize, JsData)]
pub struct ScriptResponseHandle {
    #[unsafe_ignore_trace]
    pub inner: Rc<RefCell<ScriptResponseSnapshot>>,
}

impl ScriptResponseHandle {
    pub fn new(snapshot: ScriptResponseSnapshot) -> Self {
        Self {
            inner: Rc::new(RefCell::new(snapshot)),
        }
    }
}

fn response_handle(context: &mut Context) -> JsResult<ScriptResponseHandle> {
    context
        .get_data::<ScriptResponseHandle>()
        .cloned()
        .ok_or_else(|| {
            JsNativeError::typ()
                .with_message("response is only available in post-response scripts")
                .into()
        })
}

pub fn attach_response(context: &mut Context, snapshot: ScriptResponseSnapshot) -> JsResult<()> {
    context.insert_data(ScriptResponseHandle::new(snapshot));
    register_res_global(context)
}

fn register_res_global(context: &mut Context) -> JsResult<()> {
    let get_status = NativeFunction::from_copy_closure(res_get_status);
    let get_status_text = NativeFunction::from_copy_closure(res_get_status_text);
    let get_header = NativeFunction::from_copy_closure(res_get_header);
    let get_headers = NativeFunction::from_copy_closure(res_get_headers);
    let get_body = NativeFunction::from_copy_closure(res_get_body);
    let get_response_time = NativeFunction::from_copy_closure(res_get_response_time);
    let get_url = NativeFunction::from_copy_closure(res_get_url);
    let get_size = NativeFunction::from_copy_closure(res_get_size);

    let handle = response_handle(context)?;
    let snapshot = handle.inner.borrow();
    let body_value = snapshot.body_json_value();
    let headers_value = Value::Object(snapshot.headers_object().into_iter().collect());
    let status = snapshot.status;
    let status_text = snapshot.status_text.clone();
    let response_time = snapshot.response_time_ms as f64;
    let url = snapshot.url.clone();
    drop(snapshot);

    let body_js = JsValue::from_json(&body_value, context)?;
    let headers_js = JsValue::from_json(&headers_value, context)?;

    let res = ObjectInitializer::new(context)
        .property(
            js_string!("status"),
            status,
            Attribute::CONFIGURABLE | Attribute::READONLY,
        )
        .property(
            js_string!("statusText"),
            js_string!(status_text.as_str()),
            Attribute::CONFIGURABLE | Attribute::READONLY,
        )
        .property(
            js_string!("body"),
            body_js,
            Attribute::CONFIGURABLE | Attribute::READONLY,
        )
        .property(
            js_string!("responseTime"),
            response_time,
            Attribute::CONFIGURABLE | Attribute::READONLY,
        )
        .property(
            js_string!("url"),
            js_string!(url.as_str()),
            Attribute::CONFIGURABLE | Attribute::READONLY,
        )
        .property(
            js_string!("headers"),
            headers_js,
            Attribute::CONFIGURABLE | Attribute::READONLY,
        )
        .function(get_status, js_string!("getStatus"), 0)
        .function(get_status_text, js_string!("getStatusText"), 0)
        .function(get_header, js_string!("getHeader"), 1)
        .function(get_headers, js_string!("getHeaders"), 0)
        .function(get_body, js_string!("getBody"), 0)
        .function(get_response_time, js_string!("getResponseTime"), 0)
        .function(get_url, js_string!("getUrl"), 0)
        .function(get_size, js_string!("getSize"), 0)
        .build();

    context.register_global_property(
        js_string!("res"),
        res,
        Attribute::CONFIGURABLE | Attribute::READONLY,
    )?;

    Ok(())
}

fn res_get_status(_: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::from(response_handle(context)?.inner.borrow().status))
}

fn res_get_status_text(_: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    Ok(js_string!(
        response_handle(context)?.inner.borrow().status_text.as_str()
    )
    .into())
}

fn res_get_header(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let name = read_string_arg(args, context, "header name")?;
    let value = response_handle(context)?
        .inner
        .borrow()
        .get_header(&name)
        .map_or(Value::Null, Value::String);
    JsValue::from_json(&value, context)
}

fn res_get_headers(_: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let headers = response_handle(context)?.inner.borrow().headers_object();
    JsValue::from_json(
        &Value::Object(headers.into_iter().collect()),
        context,
    )
}

fn res_get_body(_: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let body = response_handle(context)?.inner.borrow().body_json_value();
    JsValue::from_json(&body, context)
}

fn res_get_response_time(_: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::from(
        response_handle(context)?.inner.borrow().response_time_ms as f64,
    ))
}

fn res_get_url(_: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    Ok(js_string!(response_handle(context)?.inner.borrow().url.as_str()).into())
}

fn res_get_size(_: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let body = response_handle(context)?.inner.borrow().size_bytes as f64;
    JsValue::from_json(
        &serde_json::json!({
            "body": body,
            "total": body,
        }),
        context,
    )
}

fn read_string_arg(args: &[JsValue], context: &mut Context, label: &str) -> JsResult<String> {
    args.first()
        .ok_or_else(|| JsNativeError::typ().with_message(format!("{label} is required")))?
        .to_string(context)
        .map(|text| text.to_std_string_escaped())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::{run_post_response_script, run_pre_request_script, ScriptHostState};

    fn sample_response(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            status_text: "OK".into(),
            headers: vec![KeyValueField {
                enabled: true,
                name: "Content-Type".into(),
                value: "application/json".into(),
            }],
            body: ResponseBody::Text(body.into()),
            elapsed_ms: 42,
            size_bytes: body.len(),
        }
    }

    #[test]
    fn exposes_res_body_fields_in_post_response_script() {
        let response = sample_response(r#"{"id":7,"name":"Leanne Graham"}"#);

        let result = run_post_response_script(
            r#"if (res.status !== 200) {
  throw new Error("unexpected status");
}
host.setVar("activeUserId", String(res.body.id));
host.setVar("activeUserName", res.body.name);
host.setVar("contentType", res.getHeader("content-type"));"#,
            ScriptHostState::default(),
            &response,
            "https://jsonplaceholder.typicode.com/users/7".into(),
        )
        .expect("script should run");

        assert_eq!(
            result.runtime_vars.get("activeUserId").and_then(|v| v.as_str()),
            Some("7")
        );
        assert_eq!(
            result
                .runtime_vars
                .get("activeUserName")
                .and_then(|v| v.as_str()),
            Some("Leanne Graham")
        );
        assert_eq!(
            result
                .runtime_vars
                .get("contentType")
                .and_then(|v| v.as_str()),
            Some("application/json")
        );
        assert_eq!(result.console_logs.len(), 0);
    }

    #[test]
    fn captures_console_output_from_scripts() {
        let result = run_pre_request_script(
            r#"console.log("hello", { id: 7 });
console.warn("careful");
console.error("boom");"#,
            ScriptHostState::default(),
        )
        .expect("script should run");

        assert_eq!(result.console_logs.len(), 3);
        assert_eq!(result.console_logs[0].message, r#"hello {"id":7}"#);
        assert_eq!(result.console_logs[1].level.label(), "warn");
        assert_eq!(result.console_logs[2].level.label(), "error");
    }
}
