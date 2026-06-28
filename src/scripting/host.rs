use boa_engine::native_function::NativeFunction;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{Context, JsError, JsNativeError, JsResult, JsValue, js_string};
use serde_json::Value;

use super::context::ScriptHostHandle;

pub fn attach_host(context: &mut Context, handle: ScriptHostHandle) -> JsResult<()> {
    context.insert_data(handle);
    register_host_global(context)
}

fn host_handle(context: &mut Context) -> JsResult<ScriptHostHandle> {
    context
        .get_data::<ScriptHostHandle>()
        .cloned()
        .ok_or_else(|| {
            JsNativeError::typ()
                .with_message("script host state is not initialized")
                .into()
        })
}

fn read_var_name(args: &[JsValue], context: &mut Context) -> JsResult<String> {
    let name = args
        .first()
        .ok_or_else(|| JsNativeError::typ().with_message("variable name is required"))?
        .to_string(context)?;
    let name = name.to_std_string_escaped();
    if name.is_empty() {
        return Err(JsNativeError::typ()
            .with_message("variable name is required")
            .into());
    }
    if !is_valid_var_name(&name) {
        return Err(JsNativeError::typ()
            .with_message(format!(
                "variable name \"{name}\" contains invalid characters; \
                 use only letters, digits, \"-\", \"_\", and \".\""
            ))
            .into());
    }
    Ok(name)
}

fn is_valid_var_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn register_host_global(context: &mut Context) -> JsResult<()> {
    let get_var = NativeFunction::from_copy_closure(host_get_var);
    let set_var = NativeFunction::from_copy_closure(host_set_var);
    let get_env_var = NativeFunction::from_copy_closure(host_get_env_var);
    let set_env_var = NativeFunction::from_copy_closure(host_set_env_var);

    let host = ObjectInitializer::new(context)
        .function(get_var, js_string!("getVar"), 1)
        .function(set_var, js_string!("setVar"), 2)
        .function(get_env_var, js_string!("getEnvVar"), 1)
        .function(set_env_var, js_string!("setEnvVar"), 2)
        .build();

    context.register_global_property(
        js_string!("host"),
        host,
        Attribute::CONFIGURABLE | Attribute::WRITABLE,
    )?;

    Ok(())
}

fn host_get_var(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let handle = host_handle(context)?;
    let name = read_var_name(args, context)?;
    let state = handle.inner.borrow();
    let value = state.runtime_vars.get(&name).cloned().unwrap_or(Value::Null);
    JsValue::from_json(&value, context)
}

fn host_set_var(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let handle = host_handle(context)?;
    let name = read_var_name(args, context)?;
    let value = args.get(1).ok_or_else(|| {
        JsNativeError::typ().with_message("variable value is required")
    })?;
    let value = js_to_json_value(value, context)?;
    let mut state = handle.inner.borrow_mut();
    state.runtime_vars.insert(name, value);
    state.runtime_dirty = true;
    Ok(JsValue::undefined())
}

fn host_get_env_var(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let handle = host_handle(context)?;
    let name = read_var_name(args, context)?;
    let state = handle.inner.borrow();
    let value = state
        .env_vars
        .get(&name)
        .or_else(|| state.workspace_env_vars.get(&name))
        .cloned()
        .unwrap_or(Value::Null);
    JsValue::from_json(&value, context)
}

fn host_set_env_var(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let handle = host_handle(context)?;
    let name = read_var_name(args, context)?;
    let value = args.get(1).ok_or_else(|| {
        JsNativeError::typ().with_message("variable value is required")
    })?;
    let value = js_to_json_value(value, context)?;
    let mut state = handle.inner.borrow_mut();
    state.env_vars.insert(name, value);
    state.env_dirty = true;
    Ok(JsValue::undefined())
}

fn js_to_json_value(value: &JsValue, context: &mut Context) -> JsResult<Value> {
    Ok(value.to_json(context)?.unwrap_or(Value::Null))
}

pub fn format_script_error(error: JsError) -> String {
    error.to_string()
}
