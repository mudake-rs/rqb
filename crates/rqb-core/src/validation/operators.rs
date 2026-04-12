use crate::error::{Error, Result};
use crate::expr::{ColumnOperator, Operator};
use crate::field::{ElemType, EnumType, FieldType, ResolvedField, TextSearchConfig};
use crate::value::Value;

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
            } else if !(field.ty.is_numeric()
                || field.ty.is_temporal()
                || field.ty == FieldType::Text)
            {
                return unsupported(field, operator);
            }
        }
        ArrayContainsAny | ArrayContainsAll => {
            require_array(field, operator, value)?;
            if !field.ty.is_array() {
                return unsupported(field, operator);
            }
            if let Some(enum_type) = enum_type_for_array(field) {
                require_enum_array(field, operator, enum_type, value)?;
            }
        }
        ArrayContains | ArrayNotContains => {
            require_scalar(field, operator, value)?;
            if !field.ty.is_array() {
                return unsupported(field, operator);
            }
            if let Some(enum_type) = enum_type_for_array(field) {
                require_enum_scalar(field, operator, enum_type, value)?;
            }
        }
        ArrayIsEmpty | ArrayIsNotEmpty => {
            if !field.ty.is_array() {
                return unsupported(field, operator);
            }
        }
        ArrayElemMatch => {
            if !(field.ty.is_array() || field.ty.is_jsonb()) {
                return unsupported(field, operator);
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
            if !(field.ty == FieldType::Text || field.ty == FieldType::Uuid || field.is_json_path())
            {
                return unsupported(field, operator);
            }
        }
        Regex | NotRegex => {
            require_string(field, operator, value)?;
            if !(field.ty == FieldType::Text || field.is_json_path()) {
                return unsupported(field, operator);
            }
        }
        Gt | Gte | Lt | Lte => {
            require_scalar(field, operator, value)?;
            if field.is_json_path() {
                require_number(field, operator, value)?;
            } else if let Some(enum_type) = enum_type_for_field(field) {
                require_enum_scalar(field, operator, enum_type, value)?;
            } else if !(field.ty.is_numeric()
                || field.ty.is_temporal()
                || field.ty == FieldType::Text)
            {
                return unsupported(field, operator);
            }
        }
        Equals | NotEquals | IsDistinctFrom | IsNotDistinctFrom => {
            require_scalar_or_json(field, operator, value)?;
            if let Some(enum_type) = enum_type_for_field(field) {
                require_enum_scalar(field, operator, enum_type, value)?;
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

pub(super) fn enum_type_for_field(field: &ResolvedField) -> Option<EnumType> {
    match field.ty {
        FieldType::Enum(enum_type) => Some(enum_type),
        _ => None,
    }
}

pub(super) fn enum_type_for_array(field: &ResolvedField) -> Option<EnumType> {
    match field.ty {
        FieldType::Array(ElemType::Enum(enum_type)) => Some(enum_type),
        _ => None,
    }
}

pub(super) fn require_enum_scalar(
    field: &ResolvedField,
    operator: Operator,
    enum_type: EnumType,
    value: &Value,
) -> Result<()> {
    let Value::String(value) = value else {
        return Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: format!("expected enum string, got {}", value.type_name()),
        });
    };
    if enum_type.contains(value) {
        return Ok(());
    }
    Err(Error::InvalidEnumValue {
        field: field.display_name(),
        value: value.clone(),
        allowed: enum_type.allowed_values(),
    })
}

pub(super) fn require_enum_array(
    field: &ResolvedField,
    operator: Operator,
    enum_type: EnumType,
    value: &Value,
) -> Result<()> {
    let Value::Array(values) = value else {
        unreachable!("array shape validated by require_array");
    };
    for value in values {
        require_enum_scalar(field, operator, enum_type, value)?;
    }
    Ok(())
}

pub(super) fn validate_column_operator(
    left: &ResolvedField,
    operator: ColumnOperator,
    right: &ResolvedField,
) -> Result<()> {
    let compatible = if left.ty == right.ty {
        true
    } else {
        left.ty.is_numeric() && right.ty.is_numeric()
    };

    if compatible {
        return Ok(());
    }

    Err(Error::IncompatibleColumnTypes {
        left: left.display_name(),
        left_type: left.ty.as_str().to_owned(),
        right: right.display_name(),
        right_type: right.ty.as_str().to_owned(),
        operator: operator.as_str().to_owned(),
    })
}

fn unsupported<T>(field: &ResolvedField, operator: Operator) -> Result<T> {
    Err(Error::UnsupportedOperator {
        field: field.display_name(),
        field_type: field.ty.as_str().to_owned(),
        operator: operator.as_str().to_owned(),
    })
}

fn require_array(field: &ResolvedField, operator: Operator, value: &Value) -> Result<()> {
    if value.is_array() {
        Ok(())
    } else {
        Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: format!("expected array, got {}", value.type_name()),
        })
    }
}

fn require_string_array(field: &ResolvedField, operator: Operator, value: &Value) -> Result<()> {
    let Value::Array(values) = value else {
        unreachable!("array shape validated by require_array");
    };
    if values.iter().all(|value| matches!(value, Value::String(_))) {
        return Ok(());
    }
    Err(Error::InvalidValue {
        field: field.display_name(),
        operator: operator.as_str().to_owned(),
        message: "expected array of strings".to_owned(),
    })
}

fn require_between(field: &ResolvedField, operator: Operator, value: &Value) -> Result<()> {
    match value {
        Value::Array(values) if values.len() == 2 => {
            if field.is_json_path() && (!values[0].is_number() || !values[1].is_number()) {
                return Err(Error::InvalidValue {
                    field: field.display_name(),
                    operator: operator.as_str().to_owned(),
                    message: "JSONB path range comparisons require numeric bounds".to_owned(),
                });
            }
            Ok(())
        }
        Value::Array(values) => Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: format!("expected exactly 2 values, got {}", values.len()),
        }),
        other => Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: format!("expected array, got {}", other.type_name()),
        }),
    }
}

fn require_string(field: &ResolvedField, operator: Operator, value: &Value) -> Result<()> {
    if matches!(value, Value::String(_)) {
        Ok(())
    } else {
        Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: format!("expected string, got {}", value.type_name()),
        })
    }
}

fn require_scalar(field: &ResolvedField, operator: Operator, value: &Value) -> Result<()> {
    if value.is_scalar() {
        Ok(())
    } else {
        Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: format!("expected scalar, got {}", value.type_name()),
        })
    }
}

fn require_scalar_or_json(field: &ResolvedField, operator: Operator, value: &Value) -> Result<()> {
    if value.is_scalar() || matches!(value, Value::Json(_)) {
        Ok(())
    } else {
        Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: format!("expected scalar or JSON, got {}", value.type_name()),
        })
    }
}

fn require_number(field: &ResolvedField, operator: Operator, value: &Value) -> Result<()> {
    if value.is_number() {
        Ok(())
    } else {
        Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: format!("expected number, got {}", value.type_name()),
        })
    }
}

pub(super) fn count_raw_placeholders(sql: &str) -> usize {
    let mut count = 0;
    let mut chars = sql.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '?' {
            if chars.peek() == Some(&'?') {
                chars.next();
            } else {
                count += 1;
            }
        }
    }
    count
}
