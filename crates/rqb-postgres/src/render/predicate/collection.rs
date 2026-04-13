use rqb_core::{ElemType, FieldType, Operator, ResolvedField, Value};

use crate::helpers::value_to_json_array;
use crate::type_sql::array_element_field_type;

use super::super::Renderer;

impl Renderer {
    pub(super) fn render_array_set_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        self.render_column_name(field);
        match operator {
            Operator::ArrayContainsAny => self.sql.push_str(" && "),
            Operator::ArrayContainsAll => self.sql.push_str(" @> "),
            _ => unreachable!("operator category validated by Operator::category"),
        }
        self.push_typed_param(value, field.ty);
    }

    pub(super) fn render_array_membership_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        match operator {
            Operator::ArrayContains => self.render_array_contains(field, value, false),
            Operator::ArrayNotContains => self.render_array_contains(field, value, true),
            _ => unreachable!("operator category validated by Operator::category"),
        }
    }

    pub(super) fn render_array_state_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
    ) {
        self.sql.push_str("cardinality(");
        self.render_column_name(field);
        match operator {
            Operator::ArrayIsEmpty => self.sql.push_str(") = 0"),
            Operator::ArrayIsNotEmpty => self.sql.push_str(") > 0"),
            _ => unreachable!("operator category validated by Operator::category"),
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

    pub(super) fn render_json_key(&mut self, field: &ResolvedField, value: &Value) {
        self.render_column_name(field);
        self.sql.push_str(" ? ");
        self.push_param(value);
    }

    pub(super) fn render_json_key_set(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        self.render_column_name(field);
        match operator {
            Operator::JsonKeysExistAny => self.sql.push_str(" ?| "),
            Operator::JsonKeysExistAll => self.sql.push_str(" ?& "),
            _ => unreachable!("operator category validated by Operator::category"),
        }
        self.push_typed_param(value, FieldType::Array(ElemType::Text));
    }

    pub(super) fn render_containment_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        match operator {
            Operator::ContainedBy => self.render_contained_by(field, value),
            Operator::Overlaps => self.render_overlaps(field, value),
            _ => unreachable!("operator category validated by Operator::category"),
        }
    }

    fn render_array_contains(&mut self, field: &ResolvedField, value: &Value, negate: bool) {
        if negate {
            self.sql.push_str("NOT (");
        }
        self.push_typed_param(value, array_element_field_type(field.ty));
        self.sql.push_str(" = ANY(");
        self.render_column_name(field);
        self.sql.push(')');
        if negate {
            self.sql.push(')');
        }
    }

    fn render_contained_by(&mut self, field: &ResolvedField, value: &Value) {
        self.render_column_name(field);
        if field.ty.is_network() {
            self.sql.push_str(" <<= ");
        } else {
            self.sql.push_str(" <@ ");
        }
        self.push_typed_param(value, field.ty);
    }

    fn render_overlaps(&mut self, field: &ResolvedField, value: &Value) {
        self.render_column_name(field);
        self.sql.push_str(" && ");
        self.push_typed_param(value, field.ty);
    }
}
