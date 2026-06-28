use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::{Context, Source, js_string};
use boa_gc::{Finalize, Trace};

use super::context::{ConsoleLogStore, ScriptHostHandle, ScriptHostState, ScriptResult};
use super::host::{attach_host, format_script_error};

#[derive(Trace, Finalize)]
struct ConsoleCaptures {
    #[unsafe_ignore_trace]
    logs: Rc<RefCell<Vec<String>>>,
}

pub fn run_script(script: &str, state: ScriptHostState) -> Result<ScriptResult, String> {
    let trimmed = script.trim();
    if trimmed.is_empty() {
        return Ok(ScriptResult::default());
    }

    let handle = ScriptHostHandle::new(state);
    let mut context = Context::default();

    attach_host(&mut context, handle.clone()).map_err(|error| error.to_string())?;
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

fn register_console(context: &mut Context) -> Rc<RefCell<Vec<String>>> {
    let store = ConsoleLogStore::default();
    let logs = store.logs.clone();
    context.insert_data(store);

    let log = boa_engine::native_function::NativeFunction::from_copy_closure_with_captures(
        |_: &boa_engine::JsValue,
         args: &[boa_engine::JsValue],
         captures: &ConsoleCaptures,
         context: &mut Context| {
            let mut parts = Vec::with_capacity(args.len());
            for arg in args {
                parts.push(
                    arg.to_string(context)
                        .map(|text| text.to_std_string_escaped())
                        .unwrap_or_else(|_| "<value>".into()),
                );
            }
            captures.logs.borrow_mut().push(parts.join(" "));
            Ok(boa_engine::JsValue::undefined())
        },
        ConsoleCaptures { logs: logs.clone() },
    );

    let console = boa_engine::object::ObjectInitializer::new(context)
        .function(log, js_string!("log"), 1)
        .build();

    let _ = context.register_global_property(
        js_string!("console"),
        console,
        boa_engine::property::Attribute::CONFIGURABLE | boa_engine::property::Attribute::WRITABLE,
    );

    logs
}
