use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex, v_flex,
    input::{Input, InputState},
    select::{Select, SelectState},
    scroll::ScrollableElement as _,
    ActiveTheme as _, IconName, IndexPath, Sizable as _,
};

use crate::domain::{KeyValueField, MultipartField, MultipartFieldType};

use super::ApiHelperApp;

pub(crate) struct RowInputs {
    name: Entity<InputState>,
    value: Entity<InputState>,
}

const MULTIPART_TYPE_LABELS: [&str; 2] = ["text", "file"];

pub(crate) struct MultipartRowInputs {
    pub(crate) name: Entity<InputState>,
    pub(crate) value: Entity<InputState>,
    pub(crate) content_type: Entity<InputState>,
    pub(crate) field_type: Entity<SelectState<Vec<&'static str>>>,
}

#[derive(Clone, Copy)]
pub(crate) enum FieldTable {
    QueryParams,
    RequestHeaders,
    FormFields,
    MultipartFields,
}

impl FieldTable {
    fn enabled_id(self) -> &'static str {
        match self {
            Self::QueryParams => "query-params-enabled",
            Self::RequestHeaders => "request-headers-enabled",
            Self::FormFields => "form-fields-enabled",
            Self::MultipartFields => "multipart-fields-enabled",
        }
    }

    fn remove_id(self) -> &'static str {
        match self {
            Self::QueryParams => "query-params-remove",
            Self::RequestHeaders => "request-headers-remove",
            Self::FormFields => "form-fields-remove",
            Self::MultipartFields => "multipart-fields-remove",
        }
    }

    fn add_id(self) -> &'static str {
        match self {
            Self::QueryParams => "query-params-add",
            Self::RequestHeaders => "request-headers-add",
            Self::FormFields => "form-fields-add",
            Self::MultipartFields => "multipart-fields-add",
        }
    }
}

impl ApiHelperApp {
    pub(super) fn render_kv_table<F: KvFieldView>(
        &self,
        table: FieldTable,
        rows: &[RowInputs],
        fields: &[F],
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<F> {
        let mut list = v_flex().gap_1().child(
            h_flex()
                .gap_2()
                .px_2()
                .child(div().w(px(24.)))
                .child(div().flex_1().text_xs().text_color(cx.theme().muted_foreground).child("Key"))
                .child(div().flex_1().text_xs().text_color(cx.theme().muted_foreground).child("Value"))
                .child(div().w(px(28.))),
        );

        for (index, field) in fields.iter().enumerate() {
            let Some(row) = rows.get(index) else {
                continue;
            };
            list = list.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .px_2()
                    .child(
                        Checkbox::new((table.enabled_id(), index))
                            .checked(field.enabled())
                            .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                this.toggle_field(table, index, *checked, cx);
                            })),
                    )
                    .child(div().flex_1().child(Input::new(&row.name)))
                    .child(div().flex_1().child(Input::new(&row.value)))
                    .child(
                        Button::new((table.remove_id(), index))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.remove_field(table, index, window, cx);
                            })),
                    ),
            );
        }

        v_flex()
            .gap_2()
            .size_full()
            .child(list.flex_1().overflow_y_scrollbar())
            .child(
                Button::new(table.add_id())
                    .ghost()
                    .small()
                    .icon(IconName::Plus)
                    .label("+ Add row")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.add_field(table, window, cx);
                    })),
            )
    }

    pub(super) fn render_multipart_table(
        &self,
        table: FieldTable,
        rows: &[MultipartRowInputs],
        fields: &[MultipartField],
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut list = v_flex().gap_1().child(
            h_flex()
                .gap_2()
                .px_2()
                .child(div().w(px(24.)))
                .child(
                    div()
                        .w(px(120.))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Key"),
                )
                .child(
                    div()
                        .w(px(72.))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Type"),
                )
                .child(
                    div()
                        .flex_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Value"),
                )
                .child(
                    div()
                        .w(px(140.))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Content-Type"),
                )
                .child(div().w(px(28.))),
        );

        for (index, field) in fields.iter().enumerate() {
            let Some(row) = rows.get(index) else {
                continue;
            };
            let is_file = row
                .field_type
                .read(cx)
                .selected_value()
                .is_some_and(|value| *value == MultipartFieldType::File.label());

            let mut value_cell = h_flex().flex_1().gap_1().child(Input::new(&row.value));
            if is_file {
                value_cell = value_cell.child(
                    Button::new(("multipart-file-pick", index))
                        .ghost()
                        .xsmall()
                        .icon(IconName::FolderOpen)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.pick_multipart_file(index, window, cx);
                        })),
                );
            }

            list = list.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .px_2()
                    .child(
                        Checkbox::new((table.enabled_id(), index))
                            .checked(field.enabled)
                            .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                this.toggle_field(table, index, *checked, cx);
                            })),
                    )
                    .child(div().w(px(120.)).child(Input::new(&row.name)))
                    .child(
                        div()
                            .w(px(72.))
                            .child(Select::new(&row.field_type)),
                    )
                    .child(value_cell)
                    .child(div().w(px(140.)).child(Input::new(&row.content_type)))
                    .child(
                        Button::new((table.remove_id(), index))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.remove_field(table, index, window, cx);
                            })),
                    ),
            );
        }

        v_flex()
            .gap_2()
            .size_full()
            .child(list.flex_1().overflow_y_scrollbar())
            .child(
                Button::new(table.add_id())
                    .ghost()
                    .small()
                    .icon(IconName::Plus)
                    .label("+ Add row")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.add_field(table, window, cx);
                    })),
            )
    }
}

pub(crate) fn flush_rows<T>(fields: &mut [T], rows: &[RowInputs], cx: &App)
where
    T: KeyValueRow,
{
    for (field, row) in fields.iter_mut().zip(rows.iter()) {
        field.set_name(row.name.read(cx).value().to_string());
        field.set_value(row.value.read(cx).value().to_string());
    }
}

pub(super) trait KvFieldView {
    fn enabled(&self) -> bool;
}

impl KvFieldView for KeyValueField {
    fn enabled(&self) -> bool {
        self.enabled
    }
}

impl KvFieldView for MultipartField {
    fn enabled(&self) -> bool {
        self.enabled
    }
}

pub(crate) fn flush_multipart_rows(
    fields: &mut [MultipartField],
    rows: &[MultipartRowInputs],
    cx: &App,
) {
    for (field, row) in fields.iter_mut().zip(rows.iter()) {
        field.name = row.name.read(cx).value().to_string();
        field.value = row.value.read(cx).value().to_string();
        field.content_type = row.content_type.read(cx).value().to_string();
        if let Some(label) = row.field_type.read(cx).selected_value() {
            if let Some(field_type) = MultipartFieldType::from_label(label) {
                field.field_type = field_type;
            }
        }
    }
}

pub(crate) fn build_multipart_row_inputs(
    window: &mut Window,
    cx: &mut Context<ApiHelperApp>,
    fields: &[MultipartField],
) -> Vec<MultipartRowInputs> {
    fields
        .iter()
        .map(|field| {
            let name = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Key")
                    .default_value(field.name.clone())
            });
            let value = cx.new(|cx| {
                let placeholder = match field.field_type {
                    MultipartFieldType::Text => "Value",
                    MultipartFieldType::File => "File path",
                };
                InputState::new(window, cx)
                    .placeholder(placeholder)
                    .default_value(field.value.clone())
            });
            let content_type = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Content-Type")
                    .default_value(field.content_type.clone())
            });
            let type_index = if field.field_type == MultipartFieldType::File {
                1
            } else {
                0
            };
            let field_type = cx.new(|cx| {
                SelectState::new(
                    MULTIPART_TYPE_LABELS.to_vec(),
                    Some(IndexPath::default().row(type_index)),
                    window,
                    cx,
                )
            });
            MultipartRowInputs {
                name,
                value,
                content_type,
                field_type,
            }
        })
        .collect()
}

pub(crate) trait KeyValueRow {
    fn set_name(&mut self, name: String);
    fn set_value(&mut self, value: String);
}

impl KeyValueRow for KeyValueField {
    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn set_value(&mut self, value: String) {
        self.value = value;
    }
}

pub(crate) fn build_row_inputs<T: RowField>(
    window: &mut Window,
    cx: &mut Context<ApiHelperApp>,
    fields: &[T],
) -> Vec<RowInputs> {
    fields
        .iter()
        .map(|field| {
            let name = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Key")
                    .default_value(field.name_for_row())
            });
            let value = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Value")
                    .default_value(field.value_for_row())
            });
            RowInputs { name, value }
        })
        .collect()
}

pub(crate) trait RowField: KeyValueRow + Clone {
    fn name_for_row(&self) -> String;
    fn value_for_row(&self) -> String;
}

impl RowField for KeyValueField {
    fn name_for_row(&self) -> String {
        self.name.clone()
    }

    fn value_for_row(&self) -> String {
        self.value.clone()
    }
}
