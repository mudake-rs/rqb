use crate::error::{Error, Result};
use crate::expr::Operator;
use crate::field::{ResolvedField, TextSearchConfig};
use crate::types::FieldType;
use crate::value::Value;

use super::value_check::{
    array_elem_type, enum_type_for_array, enum_type_for_field, reject_json_for_non_jsonb_field,
    reject_non_finite_numbers, require_array, require_array_values_for_field_type, require_between,
    require_enum_array, require_enum_scalar, require_number, require_scalar,
    require_scalar_array_json_or_null, require_scalar_array_or_json, require_scalar_json_or_null,
    require_scalar_or_json, require_string, require_string_array, require_value_for_elem_type,
    validate_value_for_field_type,
};

pub(super) fn validate_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    use Operator::*;

    match operator {
        IsNull | IsNotNull => return Ok(()),
        In | NotIn => {
            require_array(field, operator, value)?;
            if let Some(enum_type) = enum_type_for_field(field) {
                require_enum_array(field, operator, enum_type, value)?;
            } else if field.ty.is_array() {
                return unsupported(field, operator);
            } else if field.is_json_path() {
                reject_non_finite_numbers(field, operator.as_str(), value)?;
            } else {
                require_array_values_for_field_type(field, operator.as_str(), value)?;
            }
        }
        Between | NotBetween => {
            require_between(field, operator, value)?;
            if let Some(enum_type) = enum_type_for_field(field) {
                require_enum_array(field, operator, enum_type, value)?;
            } else if field.is_json_path() {
                let Value::Array(values) = value else {
                    unreachable!("array shape validated by require_between");
                };
                require_number(field, operator, &values[0])?;
                require_number(field, operator, &values[1])?;
            } else if !(field.ty.is_numeric() || field.ty.is_temporal() || field.ty.is_text()) {
                return unsupported(field, operator);
            } else {
                require_array_values_for_field_type(field, operator.as_str(), value)?;
            }
        }
        ArrayContainsAny | ArrayContainsAll => {
            require_array(field, operator, value)?;
            if !field.ty.is_array() {
                return unsupported(field, operator);
            }
            if let Some(enum_type) = enum_type_for_array(field) {
                require_enum_array(field, operator, enum_type, value)?;
            } else {
                validate_value_for_field_type(field, operator.as_str(), value)?;
            }
        }
        ArrayContains | ArrayNotContains => {
            require_scalar(field, operator, value)?;
            if !field.ty.is_array() {
                return unsupported(field, operator);
            }
            if let Some(enum_type) = enum_type_for_array(field) {
                require_enum_scalar(field, operator, enum_type, value)?;
            } else {
                require_value_for_elem_type(
                    field,
                    operator.as_str(),
                    array_elem_type(field),
                    value,
                )?;
            }
        }
        ArrayIsEmpty | ArrayIsNotEmpty => {
            if !field.ty.is_array() {
                return unsupported(field, operator);
            }
        }
        ArrayElemMatch => {
            if field.ty.is_array() {
                require_scalar(field, operator, value)?;
                if let Some(enum_type) = enum_type_for_array(field) {
                    require_enum_scalar(field, operator, enum_type, value)?;
                } else {
                    require_value_for_elem_type(
                        field,
                        operator.as_str(),
                        array_elem_type(field),
                        value,
                    )?;
                }
            } else if !field.ty.is_jsonb() {
                return unsupported(field, operator);
            } else {
                reject_non_finite_numbers(field, operator.as_str(), value)?;
            }
        }
        JsonKeyExists => {
            require_string(field, operator, value)?;
            if !field.ty.is_jsonb() || field.is_json_path() {
                return unsupported(field, operator);
            }
        }
        JsonKeysExistAny | JsonKeysExistAll => {
            require_array(field, operator, value)?;
            require_string_array(field, operator, value)?;
            if !field.ty.is_jsonb() || field.is_json_path() {
                return unsupported(field, operator);
            }
        }
        Contains | NotContains | StartsWith | EndsWith | NotStartsWith | NotEndsWith => {
            require_string(field, operator, value)?;
            if matches!(operator, Contains | NotContains)
                && (field.ty.is_range() || field.ty.is_network())
            {
                return validate_value_for_field_type(field, operator.as_str(), value);
            }
            if !(field.ty.is_text() || field.ty == FieldType::Uuid || field.is_json_path()) {
                return unsupported(field, operator);
            }
        }
        ContainedBy | Overlaps => {
            require_string(field, operator, value)?;
            if !(field.ty.is_range() || field.ty.is_network()) {
                return unsupported(field, operator);
            }
            validate_value_for_field_type(field, operator.as_str(), value)?;
        }
        Regex | NotRegex => {
            require_string(field, operator, value)?;
            if !(field.ty.is_text() || field.is_json_path()) {
                return unsupported(field, operator);
            }
        }
        Gt | Gte | Lt | Lte => {
            require_scalar(field, operator, value)?;
            if field.is_json_path() {
                require_number(field, operator, value)?;
            } else if let Some(enum_type) = enum_type_for_field(field) {
                require_enum_scalar(field, operator, enum_type, value)?;
            } else if !(field.ty.is_numeric() || field.ty.is_temporal() || field.ty.is_text()) {
                return unsupported(field, operator);
            } else {
                validate_value_for_field_type(field, operator.as_str(), value)?;
            }
        }
        Equals | NotEquals => {
            if field.ty.is_jsonb() {
                require_scalar_array_or_json(field, operator, value)?;
            } else {
                require_scalar_or_json(field, operator, value)?;
                reject_json_for_non_jsonb_field(field, operator, value)?;
            }
            if let Some(enum_type) = enum_type_for_field(field) {
                require_enum_scalar(field, operator, enum_type, value)?;
            } else {
                validate_value_for_field_type(field, operator.as_str(), value)?;
            }
        }
        IsDistinctFrom | IsNotDistinctFrom => {
            if field.ty.is_jsonb() {
                require_scalar_array_json_or_null(field, operator, value)?;
            } else {
                require_scalar_json_or_null(field, operator, value)?;
                reject_json_for_non_jsonb_field(field, operator, value)?;
            }
            if !value.is_null()
                && let Some(enum_type) = enum_type_for_field(field)
            {
                require_enum_scalar(field, operator, enum_type, value)?;
            } else if !value.is_null() {
                validate_value_for_field_type(field, operator.as_str(), value)?;
            }
        }
        TextSearch => {
            require_string(field, operator, value)?;
            if matches!(field.caps.text_search, TextSearchConfig::None) {
                return Err(Error::TextSearchNotConfigured {
                    field: field.display_name(),
                });
            }
        }
    }

    Ok(())
}

fn unsupported<T>(field: &ResolvedField, operator: Operator) -> Result<T> {
    Err(Error::UnsupportedOperator {
        field: field.display_name(),
        field_type: field.ty.display_name().into_owned(),
        operator: operator.as_str().to_owned(),
    })
}

pub(super) fn count_raw_placeholders(sql: &str) -> usize {
    crate::raw::count_placeholders(sql)
}
