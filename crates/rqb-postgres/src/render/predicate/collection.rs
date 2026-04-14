use rqb_core::{
    FieldType, ResolvedField, ValidatedArraySetOperator, ValidatedContainmentOperator,
    ValidatedContainmentTarget, Value,
};

use crate::helpers::value_to_json_array;

use super::super::Renderer;

impl Renderer {
    pub(super) fn render_array_set_predicate(
        &mut self,
        field: &ResolvedField,
        op: ValidatedArraySetOperator,
        value: &Value,
    ) {
        self.render_column_name(field);
        match op {
            ValidatedArraySetOperator::OverlapsAny => self.sql.push_str(" && "),
            ValidatedArraySetOperator::ContainsAll => self.sql.push_str(" @> "),
        }
        self.push_typed_param(value, field.ty);
    }

    pub(super) fn render_array_membership_predicate(
        &mut self,
        field: &ResolvedField,
        value: &Value,
        negated: bool,
    ) {
        self.render_array_contains(field, value, negated);
    }

    pub(super) fn render_array_state_predicate(&mut self, field: &ResolvedField, empty: bool) {
        self.sql.push_str("cardinality(");
        self.render_column_name(field);
        if empty {
            self.sql.push_str(") = 0");
        } else {
            self.sql.push_str(") > 0");
        }
    }

    pub(super) fn render_array_elem_match(&mut self, field: &ResolvedField, value: &Value) {
        if field.ty.is_array() && !field.is_json_path() {
            self.render_column_name(field);
            self.sql.push_str(" @> ");
            self.push_typed_param(&Value::Array(vec![value.clone()]), field.ty);
        } else {
            if field.is_json_path() {
                self.render_json_target(field);
            } else {
                self.render_column_name(field);
            }
            self.sql.push_str(" @> ");
            self.push_typed_param(&value_to_json_array(value), FieldType::Jsonb);
        }
    }

    pub(super) fn render_json_key(&mut self, field: &ResolvedField, key: &str) {
        self.render_column_name(field);
        self.sql.push_str(" ? ");
        self.push_owned_param(Value::String(key.to_owned()));
    }

    pub(super) fn render_json_key_set(
        &mut self,
        field: &ResolvedField,
        keys: &[String],
        all: bool,
    ) {
        self.render_column_name(field);
        if all {
            self.sql.push_str(" ?& ");
        } else {
            self.sql.push_str(" ?| ");
        }
        self.push_string_array_param(keys);
    }

    pub(super) fn render_containment_predicate(
        &mut self,
        field: &ResolvedField,
        op: ValidatedContainmentOperator,
        target: ValidatedContainmentTarget,
        value: &Value,
        negated: bool,
    ) {
        if negated {
            self.sql.push_str("NOT (");
        }
        self.render_column_name(field);
        self.sql.push(' ');
        self.sql.push_str(containment_operator_sql(op, target));
        self.sql.push(' ');
        self.push_typed_param(value, field.ty);
        if negated {
            self.sql.push(')');
        }
    }

    fn render_array_contains(&mut self, field: &ResolvedField, value: &Value, negate: bool) {
        if negate {
            self.sql.push_str("NOT (");
        }
        self.push_typed_param(value, field.ty.array_element_type());
        self.sql.push_str(" = ANY(");
        self.render_column_name(field);
        self.sql.push(')');
        if negate {
            self.sql.push(')');
        }
    }
}

fn containment_operator_sql(
    op: ValidatedContainmentOperator,
    target: ValidatedContainmentTarget,
) -> &'static str {
    match (op, target) {
        (ValidatedContainmentOperator::Contains, ValidatedContainmentTarget::Network) => ">>=",
        (ValidatedContainmentOperator::Contains, ValidatedContainmentTarget::Range) => "@>",
        (ValidatedContainmentOperator::ContainedBy, ValidatedContainmentTarget::Network) => "<<=",
        (ValidatedContainmentOperator::ContainedBy, ValidatedContainmentTarget::Range) => "<@",
        (ValidatedContainmentOperator::Overlaps, _) => "&&",
    }
}
