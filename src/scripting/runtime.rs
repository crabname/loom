use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::{Context, JsValue, Source, js_string};
use boa_gc::{Finalize, Trace};
use serde_json::Value;

use super::context::{
    ConsoleLogStore, ScriptConsoleEntry, ScriptConsoleLevel, ScriptHostHandle, ScriptHostState,
    ScriptResult,
};
use super::host::{attach_host, format_script_error};
use super::response::{attach_response, ScriptResponseSnapshot};

#[derive(Trace, Finalize)]
struct ConsoleCaptures {
    #[unsafe_ignore_trace]
    logs: Rc<RefCell<Vec<ScriptConsoleEntry>>>,
    #[unsafe_ignore_trace]
    level: ScriptConsoleLevel,
}

pub fn run_script(
    script: &str,
    state: ScriptHostState,
    response: Option<ScriptResponseSnapshot>,
) -> Result<ScriptResult, String> {
    let trimmed = script.trim();
    if trimmed.is_empty() {
        return Ok(ScriptResult::default());
    }

    let handle = ScriptHostHandle::new(state);
    let mut context = Context::default();

    attach_host(&mut context, handle.clone()).map_err(|error| error.to_string())?;
    if let Some(snapshot) = response {
        attach_response(&mut context, snapshot).map_err(|error| error.to_string())?;
    }
    let console_logs = register_console(&mut context);

    let source = Source::from_bytes(trimmed.as_bytes());
    let result = context
        .eval(source)
        .map_err(|error| format_script_error(error));

    match result {
        Ok(_) => Ok(ScriptResult::from_handle(
            &handle,
            console_logs.borrow().clone(),
        )),
        Err(message) => Err(message),
    }
}

fn register_console(context: &mut Context) -> Rc<RefCell<Vec<ScriptConsoleEntry>>> {
    let store = ConsoleLogStore::default();
    let logs = store.logs.clone();
    context.insert_data(store);

    let console = boa_engine::object::ObjectInitializer::new(context)
        .function(console_fn(logs.clone(), ScriptConsoleLevel::Log), js_string!("log"), 1)
        .function(
            console_fn(logs.clone(), ScriptConsoleLevel::Debug),
            js_string!("debug"),
            1,
        )
        .function(
            console_fn(logs.clone(), ScriptConsoleLevel::Info),
            js_string!("info"),
            1,
        )
        .function(
            console_fn(logs.clone(), ScriptConsoleLevel::Warn),
            js_string!("warn"),
            1,
        )
        .function(
            console_fn(logs.clone(), ScriptConsoleLevel::Error),
            js_string!("error"),
            1,
        )
        .build();

    let _ = context.register_global_property(
        js_string!("console"),
        console,
        boa_engine::property::Attribute::CONFIGURABLE | boa_engine::property::Attribute::WRITABLE,
    );

    logs
}

fn console_fn(
    logs: Rc<RefCell<Vec<ScriptConsoleEntry>>>,
    level: ScriptConsoleLevel,
) -> boa_engine::NativeFunction {
    boa_engine::native_function::NativeFunction::from_copy_closure_with_captures(
        |_: &JsValue,
         args: &[JsValue],
         captures: &ConsoleCaptures,
         context: &mut Context| {
            let mut parts = Vec::with_capacity(args.len());
            for arg in args {
                parts.push(format_console_arg(arg, context));
            }
            captures.logs.borrow_mut().push(ScriptConsoleEntry {
                level: captures.level,
                message: parts.join(" "),
            });
            Ok(JsValue::undefined())
        },
        ConsoleCaptures { logs, level },
    )
}

fn format_console_arg(value: &JsValue, context: &mut Context) -> String {
    if let Ok(Some(json)) = value.to_json(context) {
        return match json {
            Value::String(text) => text,
            Value::Null => "null".into(),
            other => serde_json::to_string(&other).unwrap_or_else(|_| other.to_string()),
        };
    }

    value
        .to_string(context)
        .map(|text| text.to_std_string_escaped())
        .unwrap_or_else(|_| "<value>".into())
}
