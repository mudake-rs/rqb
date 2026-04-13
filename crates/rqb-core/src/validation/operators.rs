use crate::error::{Error, Result};
use crate::expr::{ColumnOperator, Operator};
use crate::field::{
    ElemType, EnumType, FieldType, ResolvedField, TextSearchConfig, TypeFamily, TypeSpec, ValueRepr,
};
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
        left_type: left.ty.display_name().into_owned(),
        right: right.display_name(),
        right_type: right.ty.display_name().into_owned(),
        operator: operator.as_str().to_owned(),
    })
}

pub(super) fn validate_value_for_field_type(
    field: &ResolvedField,
    operator: &str,
    value: &Value,
) -> Result<()> {
    if field.is_json_path() {
        reject_non_finite_numbers(field, operator, value)?;
        return Ok(());
    }
    if value.is_null() {
        return Ok(());
    }
    require_value_for_field_type(field, operator, field.ty, value)
}

fn require_array_values_for_field_type(
    field: &ResolvedField,
    operator: &str,
    value: &Value,
) -> Result<()> {
    let Value::Array(values) = value else {
        unreachable!("array shape validated by require_array");
    };
    for value in values {
        validate_value_for_field_type(field, operator, value)?;
    }
    Ok(())
}

fn require_value_for_field_type(
    field: &ResolvedField,
    operator: &str,
    field_type: FieldType,
    value: &Value,
) -> Result<()> {
    match field_type {
        FieldType::Text => require_value_shape(field, operator, value, "string", |value| {
            matches!(value, Value::String(_))
        }),
        FieldType::Citext => require_value_shape(field, operator, value, "string", |value| {
            matches!(value, Value::String(_))
        }),
        FieldType::Integer | FieldType::BigInt => {
            require_value_shape(field, operator, value, "integer", |value| {
                matches!(value, Value::I64(_))
            })
        }
        FieldType::Float => require_value_shape(field, operator, value, "number", Value::is_number),
        FieldType::Numeric => require_value_shape(
            field,
            operator,
            value,
            "number or numeric string",
            |value| value.is_number() || matches!(value, Value::String(_)),
        ),
        FieldType::Bool => require_value_shape(field, operator, value, "bool", |value| {
            matches!(value, Value::Bool(_))
        }),
        FieldType::Uuid => require_value_shape(field, operator, value, "UUID string", |value| {
            matches!(value, Value::String(_))
        }),
        FieldType::Timestamp => {
            require_value_shape(field, operator, value, "timestamp string", |value| {
                matches!(value, Value::String(_))
            })
        }
        FieldType::Timestamptz => {
            require_value_shape(field, operator, value, "timestamptz string", |value| {
                matches!(value, Value::String(_))
            })
        }
        FieldType::Date => require_value_shape(field, operator, value, "date string", |value| {
            matches!(value, Value::String(_))
        }),
        FieldType::Jsonb => {
            if value.is_scalar() || matches!(value, Value::Array(_) | Value::Json(_)) {
                reject_non_finite_numbers(field, operator, value)?;
                Ok(())
            } else {
                Err(Error::InvalidValue {
                    field: field.display_name(),
                    operator: operator.to_owned(),
                    message: format!("expected scalar, array, or JSON, got {}", value.type_name()),
                })
            }
        }
        FieldType::Bytea => require_value_shape(field, operator, value, "bytes", |value| {
            matches!(value, Value::Bytes(_))
        }),
        FieldType::Inet | FieldType::Cidr => {
            require_value_shape(field, operator, value, "network string", |value| {
                matches!(value, Value::String(_))
            })
        }
        FieldType::Enum(enum_type) => {
            require_enum_scalar_by_name(field, operator, enum_type, value)
        }
        FieldType::Custom(type_spec) => {
            require_value_for_type_spec(field, operator, *type_spec, value)
        }
        FieldType::Range(elem_type) => require_range_value(field, operator, elem_type, value),
        FieldType::Array(elem_type) => {
            let Value::Array(values) = value else {
                return Err(Error::InvalidValue {
                    field: field.display_name(),
                    operator: operator.to_owned(),
                    message: format!("expected array, got {}", value.type_name()),
                });
            };
            for value in values {
                require_value_for_elem_type(field, operator, elem_type, value)?;
            }
            Ok(())
        }
    }
}

fn require_value_for_type_spec(
    field: &ResolvedField,
    operator: &str,
    type_spec: TypeSpec,
    value: &Value,
) -> Result<()> {
    match type_spec.value_repr {
        ValueRepr::DecimalString => require_value_shape(
            field,
            operator,
            value,
            "integer or decimal string",
            |value| match value {
                Value::I64(_) => true,
                Value::String(value) => looks_like_decimal(value),
                _ => false,
            },
        ),
        ValueRepr::String => require_value_shape(field, operator, value, "string", |value| {
            matches!(value, Value::String(_))
        }),
        ValueRepr::Native => match type_spec.family {
            TypeFamily::Text => require_value_shape(field, operator, value, "string", |value| {
                matches!(value, Value::String(_))
            }),
            TypeFamily::Numeric => require_value_shape(
                field,
                operator,
                value,
                "number or numeric string",
                |value| value.is_number() || matches!(value, Value::String(_)),
            ),
            TypeFamily::Bool => require_value_shape(field, operator, value, "bool", |value| {
                matches!(value, Value::Bool(_))
            }),
            TypeFamily::Uuid => {
                require_value_shape(field, operator, value, "UUID string", |value| {
                    matches!(value, Value::String(_))
                })
            }
            TypeFamily::Timestamp => {
                require_value_shape(field, operator, value, "timestamp string", |value| {
                    matches!(value, Value::String(_))
                })
            }
            TypeFamily::Timestamptz => {
                require_value_shape(field, operator, value, "timestamptz string", |value| {
                    matches!(value, Value::String(_))
                })
            }
            TypeFamily::Date => {
                require_value_shape(field, operator, value, "date string", |value| {
                    matches!(value, Value::String(_))
                })
            }
            TypeFamily::Jsonb => {
                if value.is_scalar() || matches!(value, Value::Array(_) | Value::Json(_)) {
                    reject_non_finite_numbers(field, operator, value)?;
                    Ok(())
                } else {
                    Err(Error::InvalidValue {
                        field: field.display_name(),
                        operator: operator.to_owned(),
                        message: format!(
                            "expected scalar, array, or JSON, got {}",
                            value.type_name()
                        ),
                    })
                }
            }
            TypeFamily::Bytes => require_value_shape(field, operator, value, "bytes", |value| {
                matches!(value, Value::Bytes(_))
            }),
            TypeFamily::Network | TypeFamily::Range => {
                require_value_shape(field, operator, value, "string", |value| {
                    matches!(value, Value::String(_))
                })
            }
        },
    }
}

fn require_range_value(
    field: &ResolvedField,
    operator: &str,
    elem_type: ElemType,
    value: &Value,
) -> Result<()> {
    if !is_supported_range_elem(elem_type) {
        return Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.to_owned(),
            message: format!("unsupported range element type {}", elem_type.as_str()),
        });
    }
    require_value_shape(field, operator, value, "range literal string", |value| {
        matches!(value, Value::String(_))
    })
}

fn is_supported_range_elem(elem_type: ElemType) -> bool {
    matches!(
        elem_type,
        ElemType::Int
            | ElemType::BigInt
            | ElemType::Numeric
            | ElemType::Timestamp
            | ElemType::Timestamptz
            | ElemType::Date
    )
}

fn looks_like_decimal(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut idx = 0;
    if bytes.is_empty() {
        return false;
    }
    if matches!(bytes[idx], b'+' | b'-') {
        idx += 1;
    }

    let before_dot = consume_digits(bytes, &mut idx);
    let mut after_dot = 0;
    if idx < bytes.len() && bytes[idx] == b'.' {
        idx += 1;
        after_dot = consume_digits(bytes, &mut idx);
    }
    if before_dot + after_dot == 0 {
        return false;
    }

    if idx < bytes.len() && matches!(bytes[idx], b'e' | b'E') {
        idx += 1;
        if idx < bytes.len() && matches!(bytes[idx], b'+' | b'-') {
            idx += 1;
        }
        if consume_digits(bytes, &mut idx) == 0 {
            return false;
        }
    }

    idx == bytes.len()
}

fn consume_digits(bytes: &[u8], idx: &mut usize) -> usize {
    let start = *idx;
    while *idx < bytes.len() && bytes[*idx].is_ascii_digit() {
        *idx += 1;
    }
    *idx - start
}

fn require_value_for_elem_type(
    field: &ResolvedField,
    operator: &str,
    elem_type: ElemType,
    value: &Value,
) -> Result<()> {
    match elem_type {
        ElemType::Text => require_value_shape(field, operator, value, "string", |value| {
            matches!(value, Value::String(_))
        }),
        ElemType::Citext => require_value_shape(field, operator, value, "string", |value| {
            matches!(value, Value::String(_))
        }),
        ElemType::Int | ElemType::BigInt => {
            require_value_shape(field, operator, value, "integer", |value| {
                matches!(value, Value::I64(_))
            })
        }
        ElemType::Float => require_value_shape(field, operator, value, "number", Value::is_number),
        ElemType::Numeric => require_value_shape(
            field,
            operator,
            value,
            "number or numeric string",
            |value| value.is_number() || matches!(value, Value::String(_)),
        ),
        ElemType::Bool => require_value_shape(field, operator, value, "bool", |value| {
            matches!(value, Value::Bool(_))
        }),
        ElemType::Uuid => require_value_shape(field, operator, value, "UUID string", |value| {
            matches!(value, Value::String(_))
        }),
        ElemType::Timestamp => {
            require_value_shape(field, operator, value, "timestamp string", |value| {
                matches!(value, Value::String(_))
            })
        }
        ElemType::Timestamptz => {
            require_value_shape(field, operator, value, "timestamptz string", |value| {
                matches!(value, Value::String(_))
            })
        }
        ElemType::Date => require_value_shape(field, operator, value, "date string", |value| {
            matches!(value, Value::String(_))
        }),
        ElemType::Enum(enum_type) => require_enum_scalar_by_name(field, operator, enum_type, value),
        ElemType::Custom(type_spec) => {
            require_value_for_type_spec(field, operator, *type_spec, value)
        }
    }
}

fn reject_non_finite_numbers(field: &ResolvedField, operator: &str, value: &Value) -> Result<()> {
    match value {
        Value::F64(value) if !value.is_finite() => Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.to_owned(),
            message: "non-finite numbers are not supported".to_owned(),
        }),
        Value::Array(values) => {
            for value in values {
                reject_non_finite_numbers(field, operator, value)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn require_value_shape(
    field: &ResolvedField,
    operator: &str,
    value: &Value,
    expected: &str,
    matches: impl FnOnce(&Value) -> bool,
) -> Result<()> {
    if matches(value) {
        return Ok(());
    }
    Err(Error::InvalidValue {
        field: field.display_name(),
        operator: operator.to_owned(),
        message: format!("expected {expected}, got {}", value.type_name()),
    })
}

fn require_enum_scalar_by_name(
    field: &ResolvedField,
    operator: &str,
    enum_type: EnumType,
    value: &Value,
) -> Result<()> {
    let Value::String(value) = value else {
        return Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.to_owned(),
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

fn array_elem_type(field: &ResolvedField) -> ElemType {
    let FieldType::Array(elem_type) = field.ty else {
        unreachable!("array field validated before element type lookup");
    };
    elem_type
}

fn unsupported<T>(field: &ResolvedField, operator: Operator) -> Result<T> {
    Err(Error::UnsupportedOperator {
        field: field.display_name(),
        field_type: field.ty.display_name().into_owned(),
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

fn require_scalar_array_or_json(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    if value.is_scalar() || matches!(value, Value::Array(_) | Value::Json(_)) {
        Ok(())
    } else {
        Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: format!("expected scalar, array, or JSON, got {}", value.type_name()),
        })
    }
}

fn require_scalar_json_or_null(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    if value.is_scalar() || value.is_null() || matches!(value, Value::Json(_)) {
        Ok(())
    } else {
        Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: format!("expected scalar, JSON, or null, got {}", value.type_name()),
        })
    }
}

fn require_scalar_array_json_or_null(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    if value.is_scalar() || value.is_null() || matches!(value, Value::Array(_) | Value::Json(_)) {
        Ok(())
    } else {
        Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: format!(
                "expected scalar, array, JSON, or null, got {}",
                value.type_name()
            ),
        })
    }
}

fn reject_json_for_non_jsonb_field(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
    if matches!(value, Value::Json(_)) && !field.ty.is_jsonb() {
        return Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.as_str().to_owned(),
            message: "JSON values require a JSONB field".to_owned(),
        });
    }
    Ok(())
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
    crate::raw::count_placeholders(sql)
}
