use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::native_function::NativeFunction;
use boa_engine::object::{FunctionObjectBuilder, ObjectInitializer};
use boa_engine::property::Attribute;
use boa_engine::{Context, JsData, JsError, JsNativeError, JsResult, JsValue, Source, js_string};
use boa_gc::{Finalize, Trace};
use serde_json::Value;

use super::context::{ScriptHostState, ScriptResult};
use super::host::format_script_error;
use super::response::ScriptResponseSnapshot;
use super::runtime::init_script_context;
use crate::transport::HttpResponse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResultEntry {
    pub description: String,
    pub status: TestStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TestScriptResult {
    pub script_result: ScriptResult,
    pub test_results: Vec<TestResultEntry>,
}

#[derive(Clone, Trace, Finalize, JsData)]
struct TestResultsStore {
    #[unsafe_ignore_trace]
    results: Rc<RefCell<Vec<TestResultEntry>>>,
}

#[derive(Clone, Trace, Finalize)]
struct ActualCapture {
    actual: JsValue,
}

pub fn run_tests_script(
    script: &str,
    state: ScriptHostState,
    response: &HttpResponse,
    request_url: String,
) -> Result<TestScriptResult, String> {
    let trimmed = script.trim();
    if trimmed.is_empty() {
        return Ok(TestScriptResult {
            script_result: ScriptResult::default(),
            test_results: Vec::new(),
        });
    }

    let snapshot = ScriptResponseSnapshot::from_http(response, request_url);
    let (mut context, handle, console_logs) = init_script_context(state, Some(snapshot))?;
    attach_test_api(&mut context)?;

    let source = Source::from_bytes(trimmed.as_bytes());
    context
        .eval(source)
        .map_err(format_script_error)?;

    let test_results = context
        .get_data::<TestResultsStore>()
        .map(|store| store.results.borrow().clone())
        .unwrap_or_default();

    Ok(TestScriptResult {
        script_result: ScriptResult::from_handle(&handle, console_logs.borrow().clone()),
        test_results,
    })
}

fn attach_test_api(context: &mut Context) -> Result<(), String> {
    let results = Rc::new(RefCell::new(Vec::<TestResultEntry>::new()));
    context.insert_data(TestResultsStore {
        results: results.clone(),
    });

    register_test(context, results).map_err(|error| error.to_string())?;
    register_expect(context).map_err(|error| error.to_string())?;
    Ok(())
}

fn register_test(context: &mut Context, results: Rc<RefCell<Vec<TestResultEntry>>>) -> JsResult<()> {
    let test_fn = NativeFunction::from_copy_closure_with_captures(
        |_: &JsValue, args: &[JsValue], captures: &TestResultsStore, context: &mut Context| {
            let description = read_string_arg(args, context, "test description")?;
            let callback = args.get(1).ok_or_else(|| {
                JsNativeError::typ().with_message("test callback is required")
            })?;
            let Some(callable) = callback.as_callable() else {
                return Err(JsNativeError::typ()
                    .with_message("test callback must be a function")
                    .into());
            };

            match callable.call(&JsValue::undefined(), &[], context) {
                Ok(_) => {
                    captures.results.borrow_mut().push(TestResultEntry {
                        description,
                        status: TestStatus::Pass,
                        error: None,
                    });
                }
                Err(error) => {
                    let message = format_test_error(&error, context);
                    captures.results.borrow_mut().push(TestResultEntry {
                        description,
                        status: TestStatus::Fail,
                        error: Some(message),
                    });
                }
            }

            Ok(JsValue::undefined())
        },
        TestResultsStore { results },
    );

    let test_callable = FunctionObjectBuilder::new(context.realm(), test_fn).build();
    context.register_global_property(
        js_string!("test"),
        test_callable,
        Attribute::CONFIGURABLE | Attribute::WRITABLE,
    )?;

    Ok(())
}

fn register_expect(context: &mut Context) -> JsResult<()> {
    let expect_fn = NativeFunction::from_copy_closure(|_: &JsValue, args: &[JsValue], context: &mut Context| {
        let actual = args.first().cloned().unwrap_or(JsValue::undefined());
        build_expect_value(actual, context)
    });

    let expect_callable = FunctionObjectBuilder::new(context.realm(), expect_fn).build();
    context.register_global_property(
        js_string!("expect"),
        expect_callable,
        Attribute::CONFIGURABLE | Attribute::WRITABLE,
    )?;

    Ok(())
}

fn build_expect_value(actual: JsValue, context: &mut Context) -> JsResult<JsValue> {
    let equal = NativeFunction::from_copy_closure_with_captures(
        |_: &JsValue, args: &[JsValue], capture: &ActualCapture, context: &mut Context| {
            let expected = args.first().cloned().unwrap_or(JsValue::undefined());
            assert_values_equal(&capture.actual, &expected, context)?;
            Ok(JsValue::undefined())
        },
        ActualCapture { actual: actual.clone() },
    );

    let type_check = NativeFunction::from_copy_closure_with_captures(
        |_: &JsValue, args: &[JsValue], capture: &ActualCapture, context: &mut Context| {
            let type_name = read_string_arg(args, context, "type name")?;
            assert_type(&capture.actual, &type_name, context)?;
            Ok(JsValue::undefined())
        },
        ActualCapture { actual: actual.clone() },
    );

    let property_check = NativeFunction::from_copy_closure_with_captures(
        |_: &JsValue, args: &[JsValue], capture: &ActualCapture, context: &mut Context| {
            let property = read_string_arg(args, context, "property name")?;
            assert_has_property(&capture.actual, &property, context)?;
            Ok(JsValue::undefined())
        },
        ActualCapture { actual: actual.clone() },
    );

    let be = ObjectInitializer::new(context)
        .function(type_check.clone(), js_string!("a"), 1)
        .function(type_check, js_string!("an"), 1)
        .build();

    let have = ObjectInitializer::new(context)
        .function(property_check, js_string!("property"), 1)
        .build();

    let to = ObjectInitializer::new(context)
        .function(equal, js_string!("equal"), 1)
        .property(js_string!("be"), be, Attribute::CONFIGURABLE | Attribute::READONLY)
        .property(js_string!("have"), have, Attribute::CONFIGURABLE | Attribute::READONLY)
        .build();

    Ok(ObjectInitializer::new(context)
        .property(js_string!("to"), to, Attribute::CONFIGURABLE | Attribute::READONLY)
        .build()
        .into())
}

fn assert_values_equal(actual: &JsValue, expected: &JsValue, context: &mut Context) -> JsResult<()> {
    if values_equal(actual, expected, context) {
        return Ok(());
    }

    Err(assertion_error(
        format!(
            "expected {} to equal {}",
            format_value(actual, context),
            format_value(expected, context),
        ),
        context,
    ))
}

fn assert_type(actual: &JsValue, type_name: &str, context: &mut Context) -> JsResult<()> {
    if value_matches_type(actual, type_name, context) {
        return Ok(());
    }

    Err(assertion_error(
        format!(
            "expected {} to be a(n) {type_name}",
            format_value(actual, context),
        ),
        context,
    ))
}

fn assert_has_property(actual: &JsValue, property: &str, context: &mut Context) -> JsResult<()> {
    let Some(object) = actual.as_object() else {
        return Err(assertion_error(
            format!(
                "expected {} to have property \"{property}\"",
                format_value(actual, context),
            ),
            context,
        ));
    };

    if object.has_property(js_string!(property), context)? {
        return Ok(());
    }

    Err(assertion_error(
        format!(
            "expected {} to have property \"{property}\"",
            format_value(actual, context),
        ),
        context,
    ))
}

fn values_equal(left: &JsValue, right: &JsValue, context: &mut Context) -> bool {
    if left.strict_equals(right) {
        return true;
    }

    match (value_to_json(left, context), value_to_json(right, context)) {
        (Ok(Some(left_json)), Ok(Some(right_json))) => left_json == right_json,
        _ => false,
    }
}

fn value_matches_type(value: &JsValue, type_name: &str, _context: &mut Context) -> bool {
    match type_name {
        "string" => value.is_string(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "undefined" => value.is_undefined(),
        "null" => value.is_null(),
        "array" => value.as_object().is_some_and(|object| object.is_array()),
        "object" => value
            .as_object()
            .is_some_and(|object| !value.is_null() && !object.is_array()),
        _ => false,
    }
}

fn value_to_json(value: &JsValue, context: &mut Context) -> JsResult<Option<Value>> {
    value.to_json(context)
}

fn format_value(value: &JsValue, context: &mut Context) -> String {
    if let Ok(Some(json)) = value.to_json(context) {
        return match json {
            Value::String(text) => format!("\"{text}\""),
            Value::Null => "null".into(),
            other => other.to_string(),
        };
    }

    value
        .to_string(context)
        .map(|text| text.to_std_string_escaped())
        .unwrap_or_else(|_| "<value>".into())
}

fn assertion_error(message: String, _context: &mut Context) -> JsError {
    JsNativeError::error().with_message(message).into()
}

fn format_test_error(error: &JsError, _context: &mut Context) -> String {
    error.to_string()
}

fn read_string_arg(args: &[JsValue], context: &mut Context, label: &str) -> JsResult<String> {
    args.first()
        .ok_or_else(|| JsNativeError::typ().with_message(format!("{label} is required")))?
        .to_string(context)
        .map(|text| text.to_std_string_escaped())
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::domain::{KeyValueField, ResponseBody};
    use crate::scripting::ScriptHostState;
    use crate::transport::HttpResponse;

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
    fn runs_passing_and_failing_tests() {
        let response = sample_response(r#"{"id":7,"name":"Leanne Graham"}"#);

        let result = run_tests_script(
            r#"test("status is 200", function () {
  expect(res.status).to.equal(200);
});

test("name is set", function () {
  expect(res.body.name).to.equal("Leanne Graham");
});

test("fails on purpose", function () {
  expect(res.body.id).to.equal(99);
});"#,
            ScriptHostState::default(),
            &response,
            "https://example.com/users/7".into(),
        )
        .expect("tests should run");

        assert_eq!(result.test_results.len(), 3);
        assert_eq!(result.test_results[0].status, TestStatus::Pass);
        assert_eq!(result.test_results[1].status, TestStatus::Pass);
        assert_eq!(result.test_results[2].status, TestStatus::Fail);
        assert!(result.test_results[2].error.is_some());
    }

    #[test]
    fn supports_type_and_property_assertions() {
        let response = sample_response(r#"{"items":[1,2]}"#);

        let result = run_tests_script(
            r#"test("body is object", function () {
  expect(res.body).to.be.an("object");
  expect(res.body).to.have.property("items");
  expect(res.body.items).to.be.an("array");
});"#,
            ScriptHostState::default(),
            &response,
            "https://example.com".into(),
        )
        .expect("tests should run");

        assert_eq!(result.test_results.len(), 1);
        assert_eq!(result.test_results[0].status, TestStatus::Pass);
    }
}
