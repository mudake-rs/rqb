use crate::error::{Error, Result};
use crate::expr::Operator;
use crate::field::{ResolvedField, TextSearchConfig};
use crate::types::FieldType;
use crate::value::Value;

use super::value_guard::{
    reject_json_for_non_jsonb_field, reject_non_finite_numbers, require_array, require_between,
    require_number, require_scalar, require_scalar_array_json_or_null,
    require_scalar_array_or_json, require_scalar_json_or_null, require_scalar_or_json,
    require_string, require_string_array,
};
use super::value_type::{
    array_elem_type, enum_type_for_array, enum_type_for_field, require_array_values_for_field_type,
    require_enum_array, require_enum_scalar, require_value_for_elem_type,
    validate_value_for_field_type,
};

pub(super) fn validate_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    use Operator::*;

    match operator {
        IsNull | IsNotNull => Ok(()),
        In | NotIn => validate_in_operator(field, operator, value),
        Between | NotBetween => validate_between_operator(field, operator, value),
        ArrayContainsAny | ArrayContainsAll => validate_array_set_operator(field, operator, value),
        ArrayContains | ArrayNotContains => {
            validate_array_membership_operator(field, operator, value)
        }
        ArrayIsEmpty | ArrayIsNotEmpty => validate_array_state_operator(field, operator),
        ArrayElemMatch => validate_array_elem_match_operator(field, operator, value),
        JsonKeyExists => validate_json_key_operator(field, operator, value),
        JsonKeysExistAny | JsonKeysExistAll => {
            validate_json_key_set_operator(field, operator, value)
        }
        Contains | NotContains | StartsWith | EndsWith | NotStartsWith | NotEndsWith => {
            validate_text_match_operator(field, operator, value)
        }
        ContainedBy | Overlaps => validate_containment_operator(field, operator, value),
        Regex | NotRegex => validate_regex_operator(field, operator, value),
        Gt | Gte | Lt | Lte => validate_ordering_operator(field, operator, value),
        Equals | NotEquals => validate_equality_operator(field, operator, value),
        IsDistinctFrom | IsNotDistinctFrom => {
            validate_null_safe_equality_operator(field, operator, value)
        }
        TextSearch => validate_text_search_operator(field, operator, value),
    }
}

fn validate_in_operator(field: &ResolvedField, operator: Operator, value: &Value) -> Result<()> {
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
    Ok(())
}

fn validate_between_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
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
    Ok(())
}

fn validate_array_set_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    require_array(field, operator, value)?;
    if !field.ty.is_array() {
        return unsupported(field, operator);
    }
    if let Some(enum_type) = enum_type_for_array(field) {
        require_enum_array(field, operator, enum_type, value)?;
    } else {
        validate_value_for_field_type(field, operator.as_str(), value)?;
    }
    Ok(())
}

fn validate_array_membership_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    require_scalar(field, operator, value)?;
    if !field.ty.is_array() {
        return unsupported(field, operator);
    }
    if let Some(enum_type) = enum_type_for_array(field) {
        require_enum_scalar(field, operator, enum_type, value)?;
    } else {
        require_value_for_elem_type(field, operator.as_str(), array_elem_type(field), value)?;
    }
    Ok(())
}

fn validate_array_state_operator(field: &ResolvedField, operator: Operator) -> Result<()> {
    if !field.ty.is_array() {
        return unsupported(field, operator);
    }
    Ok(())
}

fn validate_array_elem_match_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    if field.ty.is_array() {
        require_scalar(field, operator, value)?;
        if let Some(enum_type) = enum_type_for_array(field) {
            require_enum_scalar(field, operator, enum_type, value)?;
        } else {
            require_value_for_elem_type(field, operator.as_str(), array_elem_type(field), value)?;
        }
    } else if !field.ty.is_jsonb() {
        return unsupported(field, operator);
    } else {
        reject_non_finite_numbers(field, operator.as_str(), value)?;
    }
    Ok(())
}

fn validate_json_key_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    require_string(field, operator, value)?;
    if !field.ty.is_jsonb() || field.is_json_path() {
        return unsupported(field, operator);
    }
    Ok(())
}

fn validate_json_key_set_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    require_array(field, operator, value)?;
    require_string_array(field, operator, value)?;
    if !field.ty.is_jsonb() || field.is_json_path() {
        return unsupported(field, operator);
    }
    Ok(())
}

fn validate_text_match_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    require_string(field, operator, value)?;
    if matches!(operator, Operator::Contains | Operator::NotContains)
        && (field.ty.is_range() || field.ty.is_network())
    {
        return validate_value_for_field_type(field, operator.as_str(), value);
    }
    if !(field.ty.is_text() || field.ty == FieldType::Uuid || field.is_json_path()) {
        return unsupported(field, operator);
    }
    Ok(())
}

fn validate_containment_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    require_string(field, operator, value)?;
    if !(field.ty.is_range() || field.ty.is_network()) {
        return unsupported(field, operator);
    }
    validate_value_for_field_type(field, operator.as_str(), value)
}

fn validate_regex_operator(field: &ResolvedField, operator: Operator, value: &Value) -> Result<()> {
    require_string(field, operator, value)?;
    if !(field.ty.is_text() || field.is_json_path()) {
        return unsupported(field, operator);
    }
    Ok(())
}

fn validate_ordering_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
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
    Ok(())
}

fn validate_equality_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
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
    Ok(())
}

fn validate_null_safe_equality_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
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
    Ok(())
}

fn validate_text_search_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    require_string(field, operator, value)?;
    if matches!(field.caps.text_search, TextSearchConfig::None) {
        return Err(Error::TextSearchNotConfigured {
            field: field.display_name(),
        });
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
