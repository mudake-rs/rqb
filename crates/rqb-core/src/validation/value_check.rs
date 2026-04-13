use crate::error::{Error, Result};
use crate::expr::{ColumnOperator, Operator};
use crate::field::ResolvedField;
use crate::types::{ElemType, EnumType, FieldType, TypeFamily, TypeSpec, ValueRepr};
use crate::value::Value;

use super::value_shape::reject_non_finite_numbers;

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

pub(super) fn require_array_values_for_field_type(
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

pub(super) fn require_value_for_elem_type(
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

pub(super) fn array_elem_type(field: &ResolvedField) -> ElemType {
    let FieldType::Array(elem_type) = field.ty else {
        unreachable!("array field validated before element type lookup");
    };
    elem_type
}
