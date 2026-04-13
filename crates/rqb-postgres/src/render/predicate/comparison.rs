use rqb_core::{
    FieldType, ResolvedField, ValidatedBinaryOperator, ValidatedNullSafeBinaryOperator, Value,
};

use crate::helpers::value_to_json;

use super::super::Renderer;

impl Renderer {
    pub(super) fn render_null_check(&mut self, field: &ResolvedField, negated: bool) {
        if field.is_json_path() {
            self.render_text_target(field);
        } else {
            self.render_column_name(field);
        }
        if negated {
            self.sql.push_str(" IS NOT NULL");
        } else {
            self.sql.push_str(" IS NULL");
        }
    }

    pub(super) fn render_binary_predicate(
        &mut self,
        field: &ResolvedField,
        op: ValidatedBinaryOperator,
        value: &Value,
    ) {
        self.render_binary(field, binary_operator_sql(op), value);
    }

    pub(super) fn render_null_safe_binary_predicate(
        &mut self,
        field: &ResolvedField,
        op: ValidatedNullSafeBinaryOperator,
        value: &Value,
    ) {
        self.render_null_safe_binary(field, null_safe_binary_operator_sql(op), value);
    }

    pub(super) fn render_inclusion_predicate(
        &mut self,
        field: &ResolvedField,
        values: &[Value],
        negated: bool,
    ) {
        if negated {
            self.render_not_in(field, values);
        } else {
            self.render_in(field, values);
        }
    }

    pub(super) fn render_between_predicate(
        &mut self,
        field: &ResolvedField,
        lower: &Value,
        upper: &Value,
        negated: bool,
    ) {
        let op = if negated { "NOT BETWEEN" } else { "BETWEEN" };
        self.render_between_op(field, lower, upper, op);
    }

    fn render_binary(&mut self, field: &ResolvedField, op: &str, value: &Value) {
        if field.is_json_path() {
            match op {
                "=" | "<>" => {
                    self.render_json_target(field);
                    self.sql.push(' ');
                    self.sql.push_str(op);
                    self.sql.push(' ');
                    self.push_typed_param(&value_to_json(value), FieldType::Jsonb);
                }
                _ => {
                    self.sql.push('(');
                    self.render_text_target(field);
                    self.sql.push_str(")::numeric ");
                    self.sql.push_str(op);
                    self.sql.push(' ');
                    self.push_typed_param(value, FieldType::Numeric);
                }
            }
            return;
        }

        self.render_column_name(field);
        self.sql.push(' ');
        self.sql.push_str(op);
        self.sql.push(' ');
        if field.ty.is_jsonb() {
            self.push_typed_param(&value_to_json(value), FieldType::Jsonb);
        } else {
            self.push_typed_param(value, field.ty);
        }
    }

    fn render_null_safe_binary(&mut self, field: &ResolvedField, op: &str, value: &Value) {
        if field.is_json_path() {
            if value.is_null() {
                self.render_text_target(field);
                self.sql.push(' ');
                self.sql.push_str(op);
                self.sql.push(' ');
                self.push_param(value);
                return;
            }

            self.render_json_target(field);
            self.sql.push(' ');
            self.sql.push_str(op);
            self.sql.push(' ');
            self.push_typed_param(&value_to_json(value), FieldType::Jsonb);
            return;
        }

        self.render_column_name(field);
        self.sql.push(' ');
        self.sql.push_str(op);
        self.sql.push(' ');
        if field.ty.is_jsonb() && !value.is_null() {
            self.push_typed_param(&value_to_json(value), FieldType::Jsonb);
        } else {
            self.push_typed_param(value, field.ty);
        }
    }

    fn render_in(&mut self, field: &ResolvedField, values: &[Value]) {
        if values.is_empty() {
            self.sql.push_str("FALSE");
            return;
        }
        if field.is_json_path() {
            self.render_json_target(field);
            self.sql.push_str(" = ANY(");
            self.push_jsonb_array_values_param(values);
            self.sql.push(')');
            return;
        }

        self.render_column_name(field);
        self.sql.push_str(" = ANY(");
        if field.ty.is_jsonb() {
            self.push_jsonb_array_values_param(values);
        } else {
            self.push_scalar_array_values_param(values, field.ty);
        }
        self.sql.push(')');
    }

    fn render_not_in(&mut self, field: &ResolvedField, values: &[Value]) {
        if values.is_empty() {
            self.sql.push_str("TRUE");
            return;
        }

        self.sql.push_str("NOT (");
        self.render_in(field, values);
        self.sql.push(')');
    }

    fn render_between_op(&mut self, field: &ResolvedField, lower: &Value, upper: &Value, op: &str) {
        if field.is_json_path() {
            self.sql.push('(');
            self.render_text_target(field);
            self.sql.push_str(")::numeric ");
            self.sql.push_str(op);
            self.sql.push(' ');
            self.push_typed_param(lower, FieldType::Numeric);
            self.sql.push_str(" AND ");
            self.push_typed_param(upper, FieldType::Numeric);
            return;
        }

        self.render_column_name(field);
        self.sql.push(' ');
        self.sql.push_str(op);
        self.sql.push(' ');
        self.push_typed_param(lower, field.ty);
        self.sql.push_str(" AND ");
        self.push_typed_param(upper, field.ty);
    }
}

fn binary_operator_sql(op: ValidatedBinaryOperator) -> &'static str {
    match op {
        ValidatedBinaryOperator::Eq => "=",
        ValidatedBinaryOperator::NotEq => "<>",
        ValidatedBinaryOperator::Lt => "<",
        ValidatedBinaryOperator::Lte => "<=",
        ValidatedBinaryOperator::Gt => ">",
        ValidatedBinaryOperator::Gte => ">=",
    }
}

fn null_safe_binary_operator_sql(op: ValidatedNullSafeBinaryOperator) -> &'static str {
    match op {
        ValidatedNullSafeBinaryOperator::DistinctFrom => "IS DISTINCT FROM",
        ValidatedNullSafeBinaryOperator::NotDistinctFrom => "IS NOT DISTINCT FROM",
    }
}
