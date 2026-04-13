use rqb_core::{ColumnOperator, FieldType, ResolvedField};

use crate::Result;
use crate::helpers::column_operator_sql;

use super::super::Renderer;

impl Renderer {
    pub(in crate::render) fn render_column_predicate(
        &mut self,
        left: &ResolvedField,
        operator: ColumnOperator,
        right: &ResolvedField,
    ) -> Result<()> {
        self.render_column_compare_target(left, operator);
        self.sql.push(' ');
        self.sql.push_str(column_operator_sql(operator));
        self.sql.push(' ');
        self.render_column_compare_target(right, operator);
        Ok(())
    }

    pub(in crate::render::predicate) fn render_text_target(&mut self, field: &ResolvedField) {
        if field.is_json_path() {
            self.render_column_name(field);
            self.sql.push_str(" #>> ");
            self.render_json_path(&field.json_path);
        } else {
            self.render_column_name(field);
            if field.ty != FieldType::Text {
                self.sql.push_str("::text");
            }
        }
    }

    pub(in crate::render::predicate) fn render_json_target(&mut self, field: &ResolvedField) {
        if field.is_json_path() {
            self.render_column_name(field);
            self.sql.push_str(" #> ");
            self.render_json_path(&field.json_path);
        } else {
            self.render_column_name(field);
        }
    }

    fn render_column_compare_target(&mut self, field: &ResolvedField, operator: ColumnOperator) {
        if field.is_json_path() {
            if matches!(operator, ColumnOperator::Equals | ColumnOperator::NotEquals) {
                self.render_json_target(field);
            } else {
                self.render_text_target(field);
            }
        } else {
            self.render_column_name(field);
        }
    }
}
