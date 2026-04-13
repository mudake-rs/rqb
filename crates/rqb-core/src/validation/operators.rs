use crate::error::{Error, Result};
use crate::expr::{Operator, OperatorCategory};
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
use super::{
    ValidatedArraySetOperator, ValidatedBinaryOperator, ValidatedContainmentOperator,
    ValidatedContainmentTarget, ValidatedLikePattern, ValidatedNullSafeBinaryOperator,
    ValidatedPredicate,
};

pub(super) fn validate_predicate(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<ValidatedPredicate> {
    match operator.category() {
        OperatorCategory::NullCheck => Ok(ValidatedPredicate::NullCheck {
            field: field.clone(),
            negated: operator == Operator::IsNotNull,
        }),
        OperatorCategory::Inclusion => {
            validate_in_operator(field, operator, value)?;
            Ok(ValidatedPredicate::In {
                field: field.clone(),
                values: array_values(value),
                negated: operator == Operator::NotIn,
            })
        }
        OperatorCategory::Between => {
            validate_between_operator(field, operator, value)?;
            let (lower, upper) = between_values(value);
            Ok(ValidatedPredicate::Between {
                field: field.clone(),
                lower,
                upper,
                negated: operator == Operator::NotBetween,
            })
        }
        OperatorCategory::ArraySet => {
            validate_array_set_operator(field, operator, value)?;
            Ok(ValidatedPredicate::ArraySet {
                field: field.clone(),
                op: array_set_operator(operator),
                value: value.clone(),
            })
        }
        OperatorCategory::ArrayMembership => {
            validate_array_membership_operator(field, operator, value)?;
            Ok(ValidatedPredicate::ArrayMembership {
                field: field.clone(),
                value: value.clone(),
                negated: operator == Operator::ArrayNotContains,
            })
        }
        OperatorCategory::ArrayState => {
            validate_array_state_operator(field, operator)?;
            Ok(ValidatedPredicate::ArrayState {
                field: field.clone(),
                empty: operator == Operator::ArrayIsEmpty,
            })
        }
        OperatorCategory::ArrayElementMatch => {
            validate_array_elem_match_operator(field, operator, value)?;
            Ok(ValidatedPredicate::ArrayElemMatch {
                field: field.clone(),
                value: value.clone(),
            })
        }
        OperatorCategory::JsonKey => {
            validate_json_key_operator(field, operator, value)?;
            Ok(ValidatedPredicate::JsonKey {
                field: field.clone(),
                key: string_value(value),
            })
        }
        OperatorCategory::JsonKeySet => {
            validate_json_key_set_operator(field, operator, value)?;
            Ok(ValidatedPredicate::JsonKeySet {
                field: field.clone(),
                keys: string_values(value),
                all: operator == Operator::JsonKeysExistAll,
            })
        }
        OperatorCategory::Contains => {
            validate_contains_operator(field, operator, value)?;
            if field.ty.is_range() || field.ty.is_network() {
                Ok(ValidatedPredicate::Containment {
                    field: field.clone(),
                    op: ValidatedContainmentOperator::Contains,
                    target: containment_target(field),
                    value: value.clone(),
                    negated: operator == Operator::NotContains,
                })
            } else {
                Ok(ValidatedPredicate::Like {
                    field: field.clone(),
                    pattern: ValidatedLikePattern::Contains,
                    value: string_value(value),
                    negated: operator == Operator::NotContains,
                })
            }
        }
        OperatorCategory::TextAffix => {
            validate_text_affix_operator(field, operator, value)?;
            Ok(ValidatedPredicate::Like {
                field: field.clone(),
                pattern: like_pattern(operator),
                value: string_value(value),
                negated: matches!(operator, Operator::NotStartsWith | Operator::NotEndsWith),
            })
        }
        OperatorCategory::Containment => {
            validate_containment_operator(field, operator, value)?;
            Ok(ValidatedPredicate::Containment {
                field: field.clone(),
                op: containment_operator(operator),
                target: containment_target(field),
                value: value.clone(),
                negated: false,
            })
        }
        OperatorCategory::Regex => {
            validate_regex_operator(field, operator, value)?;
            Ok(ValidatedPredicate::Regex {
                field: field.clone(),
                value: string_value(value),
                negated: operator == Operator::NotRegex,
            })
        }
        OperatorCategory::Ordering => {
            validate_ordering_operator(field, operator, value)?;
            Ok(ValidatedPredicate::Binary {
                field: field.clone(),
                op: binary_operator(operator),
                value: value.clone(),
            })
        }
        OperatorCategory::Equality => {
            validate_equality_operator(field, operator, value)?;
            Ok(ValidatedPredicate::Binary {
                field: field.clone(),
                op: binary_operator(operator),
                value: value.clone(),
            })
        }
        OperatorCategory::NullSafeEquality => {
            validate_null_safe_equality_operator(field, operator, value)?;
            Ok(ValidatedPredicate::NullSafeBinary {
                field: field.clone(),
                op: null_safe_binary_operator(operator),
                value: value.clone(),
            })
        }
        OperatorCategory::TextSearch => {
            validate_text_search_operator(field, operator, value)?;
            Ok(ValidatedPredicate::TextSearch {
                field: field.clone(),
                value: string_value(value),
            })
        }
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

fn validate_contains_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    require_string(field, operator, value)?;
    if field.ty.is_range() || field.ty.is_network() {
        return validate_value_for_field_type(field, operator.as_str(), value);
    }
    if !(field.ty.is_text() || field.ty == FieldType::Uuid || field.is_json_path()) {
        return unsupported(field, operator);
    }
    Ok(())
}

fn validate_text_affix_operator(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    require_string(field, operator, value)?;
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

fn binary_operator(operator: Operator) -> ValidatedBinaryOperator {
    match operator {
        Operator::Equals => ValidatedBinaryOperator::Eq,
        Operator::NotEquals => ValidatedBinaryOperator::NotEq,
        Operator::Lt => ValidatedBinaryOperator::Lt,
        Operator::Lte => ValidatedBinaryOperator::Lte,
        Operator::Gt => ValidatedBinaryOperator::Gt,
        Operator::Gte => ValidatedBinaryOperator::Gte,
        _ => unreachable!("operator category validated by Operator::category"),
    }
}

fn null_safe_binary_operator(operator: Operator) -> ValidatedNullSafeBinaryOperator {
    match operator {
        Operator::IsDistinctFrom => ValidatedNullSafeBinaryOperator::DistinctFrom,
        Operator::IsNotDistinctFrom => ValidatedNullSafeBinaryOperator::NotDistinctFrom,
        _ => unreachable!("operator category validated by Operator::category"),
    }
}

fn like_pattern(operator: Operator) -> ValidatedLikePattern {
    match operator {
        Operator::StartsWith | Operator::NotStartsWith => ValidatedLikePattern::StartsWith,
        Operator::EndsWith | Operator::NotEndsWith => ValidatedLikePattern::EndsWith,
        _ => unreachable!("operator category validated by Operator::category"),
    }
}

fn array_set_operator(operator: Operator) -> ValidatedArraySetOperator {
    match operator {
        Operator::ArrayContainsAny => ValidatedArraySetOperator::OverlapsAny,
        Operator::ArrayContainsAll => ValidatedArraySetOperator::ContainsAll,
        _ => unreachable!("operator category validated by Operator::category"),
    }
}

fn containment_operator(operator: Operator) -> ValidatedContainmentOperator {
    match operator {
        Operator::ContainedBy => ValidatedContainmentOperator::ContainedBy,
        Operator::Overlaps => ValidatedContainmentOperator::Overlaps,
        _ => unreachable!("operator category validated by Operator::category"),
    }
}

fn containment_target(field: &ResolvedField) -> ValidatedContainmentTarget {
    if field.ty.is_network() {
        ValidatedContainmentTarget::Network
    } else {
        ValidatedContainmentTarget::Range
    }
}

fn string_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        _ => unreachable!("validated by value shape checks"),
    }
}

fn string_values(value: &Value) -> Vec<String> {
    match value {
        Value::Array(values) => values.iter().map(string_value).collect(),
        _ => unreachable!("validated by value shape checks"),
    }
}

fn array_values(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(values) => values.clone(),
        _ => unreachable!("validated by value shape checks"),
    }
}

fn between_values(value: &Value) -> (Value, Value) {
    match value {
        Value::Array(values) => (values[0].clone(), values[1].clone()),
        _ => unreachable!("validated by value shape checks"),
    }
}

pub(super) fn count_raw_placeholders(sql: &str) -> usize {
    crate::raw::count_placeholders(sql)
}
