use gpui::*;
use gpui_component::input::InputState;

use crate::domain::Variable;

use crate::app::ui::fields::RowInputs;

pub(crate) fn flush_environment_variables(
    variables: &mut [Variable],
    rows: &[RowInputs],
    cx: &App,
) {
    for (variable, row) in variables.iter_mut().zip(rows.iter()) {
        variable.name = row.name.read(cx).value().to_string();
        variable.value = serde_json::Value::String(row.value.read(cx).value().to_string());
    }
}

pub(crate) fn build_variable_row_inputs(
    window: &mut Window,
    cx: &mut App,
    variables: &[Variable],
) -> Vec<RowInputs> {
    variables
        .iter()
        .map(|variable| {
            let name = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Key")
                    .default_value(variable.name.clone())
            });
            let value = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Value")
                    .default_value(variable.display_value())
            });
            RowInputs { name, value }
        })
        .collect()
}
