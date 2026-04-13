use rqb_core::{FieldType, Operator, ResolvedField, Value};

use crate::helpers::value_to_json;

use super::super::Renderer;

impl Renderer {
    pub(super) fn render_null_check(&mut self, field: &ResolvedField, operator: Operator) {
        if field.is_json_path() {
            self.render_text_target(field);
        } else {
            self.render_column_name(field);
        }
        match operator {
            Operator::IsNull => self.sql.push_str(" IS NULL"),
            Operator::IsNotNull => self.sql.push_str(" IS NOT NULL"),
            _ => unreachable!("operator category validated by Operator::category"),
        }
    }

    pub(super) fn render_equality_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        match operator {
            Operator::Equals => self.render_binary(field, "=", value),
            Operator::NotEquals => self.render_binary(field, "<>", value),
            _ => unreachable!("operator category validated by Operator::category"),
        }
    }

    pub(super) fn render_null_safe_equality_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        match operator {
            Operator::IsDistinctFrom => {
                self.render_null_safe_binary(field, "IS DISTINCT FROM", value)
            }
            Operator::IsNotDistinctFrom => {
                self.render_null_safe_binary(field, "IS NOT DISTINCT FROM", value)
            }
            _ => unreachable!("operator category validated by Operator::category"),
        }
    }

    pub(super) fn render_ordering_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        match operator {
            Operator::Gt => self.render_binary(field, ">", value),
            Operator::Gte => self.render_binary(field, ">=", value),
            Operator::Lt => self.render_binary(field, "<", value),
            Operator::Lte => self.render_binary(field, "<=", value),
            _ => unreachable!("operator category validated by Operator::category"),
        }
    }

    pub(super) fn render_inclusion_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        match operator {
            Operator::In => self.render_in(field, value),
            Operator::NotIn => self.render_not_in(field, value),
            _ => unreachable!("operator category validated by Operator::category"),
        }
    }

    pub(super) fn render_between_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        match operator {
            Operator::Between => self.render_between(field, value),
            Operator::NotBetween => self.render_not_between(field, value),
            _ => unreachable!("operator category validated by Operator::category"),
        }
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

    fn render_in(&mut self, field: &ResolvedField, value: &Value) {
        let Value::Array(values) = value else {
            unreachable!("validated by rqb-core");
        };
        if values.is_empty() {
            self.sql.push_str("FALSE");
            return;
        }

        if field.is_json_path() {
            self.render_json_target(field);
            self.sql.push_str(" = ANY(");
            self.push_jsonb_array_param(value);
            self.sql.push(')');
            return;
        }

        self.render_column_name(field);
        self.sql.push_str(" = ANY(");
        if field.ty.is_jsonb() {
            self.push_jsonb_array_param(value);
        } else {
            self.push_scalar_array_param(value, field.ty);
        }
        self.sql.push(')');
    }

    fn render_not_in(&mut self, field: &ResolvedField, value: &Value) {
        let Value::Array(values) = value else {
            unreachable!("validated by rqb-core");
        };
        if values.is_empty() {
            self.sql.push_str("TRUE");
            return;
        }

        self.sql.push_str("NOT (");
        self.render_in(field, value);
        self.sql.push(')');
    }

    fn render_between(&mut self, field: &ResolvedField, value: &Value) {
        self.render_between_op(field, value, "BETWEEN")
    }

    fn render_not_between(&mut self, field: &ResolvedField, value: &Value) {
        self.render_between_op(field, value, "NOT BETWEEN")
    }

    fn render_between_op(&mut self, field: &ResolvedField, value: &Value, op: &str) {
        let Value::Array(values) = value else {
            unreachable!("validated by rqb-core");
        };
        if field.is_json_path() {
            self.sql.push('(');
            self.render_text_target(field);
            self.sql.push_str(")::numeric ");
            self.sql.push_str(op);
            self.sql.push(' ');
            self.push_typed_param(&values[0], FieldType::Numeric);
            self.sql.push_str(" AND ");
            self.push_typed_param(&values[1], FieldType::Numeric);
            return;
        }

        self.render_column_name(field);
        self.sql.push(' ');
        self.sql.push_str(op);
        self.sql.push(' ');
        self.push_typed_param(&values[0], field.ty);
        self.sql.push_str(" AND ");
        self.push_typed_param(&values[1], field.ty);
    }
}
