use gpui::*;

use crate::domain::{FormField, KeyValueField, MultipartField, Variable};

use super::ui::FieldTable;
use super::LoomApp;

impl LoomApp {
    pub(super) fn toggle_field(
        &mut self,
        table: FieldTable,
        index: usize,
        enabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let sync_query = matches!(table, FieldTable::QueryParams);
        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        match table {
            FieldTable::QueryParams => {
                if let Some(field) = tab.query_params.get_mut(index) {
                    field.enabled = enabled;
                }
            }
            FieldTable::RequestHeaders => {
                if let Some(field) = tab.headers.get_mut(index) {
                    field.enabled = enabled;
                }
            }
            FieldTable::FormFields => {
                if let Some(field) = tab.form_fields.get_mut(index) {
                    field.enabled = enabled;
                }
            }
            FieldTable::MultipartFields => {
                if let Some(field) = tab.multipart_fields.get_mut(index) {
                    field.enabled = enabled;
                }
            }
        }

        if sync_query {
            self.sync_url_from_params(window, cx);
        }
        self.sync_active_tab_to_collection(cx);
        cx.notify();
    }

    pub(super) fn remove_field(
        &mut self,
        table: FieldTable,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.flush_field_inputs(cx);

        if let Some(tab) = self.active_tab_mut() {
            match table {
                FieldTable::QueryParams => {
                    if tab.query_params.len() > 1 {
                        tab.query_params.remove(index);
                    } else {
                        tab.query_params[0] = KeyValueField::empty();
                    }
                }
                FieldTable::RequestHeaders => {
                    if tab.headers.len() > 1 {
                        tab.headers.remove(index);
                    } else {
                        tab.headers[0] = KeyValueField::empty();
                    }
                }
                FieldTable::FormFields => {
                    if tab.form_fields.len() > 1 {
                        tab.form_fields.remove(index);
                    } else {
                        tab.form_fields[0] = FormField::empty();
                    }
                }
                FieldTable::MultipartFields => {
                    if tab.multipart_fields.len() > 1 {
                        tab.multipart_fields.remove(index);
                    } else {
                        tab.multipart_fields[0] = MultipartField::empty();
                    }
                }
            }
        }

        self.reload_field_inputs(window, cx);
        if matches!(table, FieldTable::QueryParams) {
            self.sync_url_from_params(window, cx);
        }
        cx.notify();
    }

    pub(super) fn add_field(
        &mut self,
        table: FieldTable,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.flush_field_inputs(cx);

        if let Some(tab) = self.active_tab_mut() {
            match table {
                FieldTable::QueryParams => tab.query_params.push(KeyValueField::empty()),
                FieldTable::RequestHeaders => tab.headers.push(KeyValueField::empty()),
                FieldTable::FormFields => tab.form_fields.push(FormField::empty()),
                FieldTable::MultipartFields => tab.multipart_fields.push(MultipartField::empty()),
            }
        }

        self.reload_field_inputs(window, cx);
        if matches!(table, FieldTable::QueryParams) {
            self.sync_url_from_params(window, cx);
        }
        cx.notify();
    }

    pub(super) fn add_request_variable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush_field_inputs(cx);

        if let Some(tab) = self.active_tab_mut() {
            tab.variables.push(Variable::empty());
        }

        self.reload_field_inputs(window, cx);
        self.sync_active_tab_to_collection(cx);
        cx.notify();
    }

    pub(super) fn remove_request_variable(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.flush_field_inputs(cx);

        if let Some(tab) = self.active_tab_mut() {
            if tab.variables.len() > 1 {
                tab.variables.remove(index);
            } else {
                tab.variables[0] = Variable::empty();
            }
        }

        self.reload_field_inputs(window, cx);
        self.sync_active_tab_to_collection(cx);
        cx.notify();
    }
}
