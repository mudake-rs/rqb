use crate::error::{Error, Result};
use crate::expr::{ColumnOperator, Operator};
use crate::field::ResolvedField;
use crate::types::{ElemType, EnumType, FieldType, TypeFamily, TypeSpec, ValueRepr};
use crate::value::Value;

use super::sql_expr::compatible_type;
use super::value_guard::reject_non_finite_numbers;

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
    let compatible = left.ty == right.ty || compatible_type(left.ty, right.ty).is_some();

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
        require_value_for_field_type(field, operator, field.ty, value)?;
    }
    Ok(())
}

fn require_value_for_field_type(
    field: &ResolvedField,
    operator: &str,
    field_type: FieldType,
    value: &Value,
) -> Result<()> {
    reject_non_finite_numbers(field, operator, value)?;

    match field_type {
        FieldType::Text | FieldType::Citext => {
            require_string_value(field, operator, value, "string")
        }
        FieldType::Integer => require_int4_value(field, operator, value),
        FieldType::BigInt => require_i64_value(field, operator, value),
        FieldType::Float => require_value_shape(field, operator, value, "number", Value::is_number),
        FieldType::Numeric => require_exact_numeric_value(field, operator, value),
        FieldType::Bool => require_bool_value(field, operator, value),
        FieldType::Uuid => require_string_value(field, operator, value, "UUID string"),
        FieldType::Timestamp => require_string_value(field, operator, value, "timestamp string"),
        FieldType::Timestamptz => {
            require_string_value(field, operator, value, "timestamptz string")
        }
        FieldType::Date => require_string_value(field, operator, value, "date string"),
        FieldType::Time => require_string_value(field, operator, value, "time string"),
        FieldType::Timetz => require_string_value(field, operator, value, "timetz string"),
        FieldType::Interval => require_string_value(field, operator, value, "interval string"),
        FieldType::Jsonb => require_jsonb_value(field, operator, value),
        FieldType::Bytea => require_bytes_value(field, operator, value),
        FieldType::Inet | FieldType::Cidr => {
            require_string_value(field, operator, value, "network string")
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
                require_value_for_field_type(field, operator, elem_type.field_type(), value)?;
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
        ValueRepr::DecimalString => require_decimal_string_value(field, operator, value),
        ValueRepr::String => require_string_value(field, operator, value, "string"),
        ValueRepr::Native => {
            require_value_for_type_family(field, operator, type_spec.family, value)
        }
    }
}

fn require_value_for_type_family(
    field: &ResolvedField,
    operator: &str,
    family: TypeFamily,
    value: &Value,
) -> Result<()> {
    match family {
        TypeFamily::Text => require_string_value(field, operator, value, "string"),
        TypeFamily::Numeric => require_exact_numeric_value(field, operator, value),
        TypeFamily::Bool => require_bool_value(field, operator, value),
        TypeFamily::Uuid => require_string_value(field, operator, value, "UUID string"),
        TypeFamily::Timestamp => require_string_value(field, operator, value, "timestamp string"),
        TypeFamily::Timestamptz => {
            require_string_value(field, operator, value, "timestamptz string")
        }
        TypeFamily::Date => require_string_value(field, operator, value, "date string"),
        TypeFamily::Time => require_string_value(field, operator, value, "time string"),
        TypeFamily::Timetz => require_string_value(field, operator, value, "timetz string"),
        TypeFamily::Interval => require_string_value(field, operator, value, "interval string"),
        TypeFamily::Jsonb => require_jsonb_value(field, operator, value),
        TypeFamily::Bytes => require_bytes_value(field, operator, value),
        TypeFamily::Network | TypeFamily::Range => {
            require_string_value(field, operator, value, "string")
        }
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
    require_string_value(field, operator, value, "range literal string")
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

fn is_exact_numeric_value(value: &Value) -> bool {
    is_decimal_string_value(value)
}

fn is_decimal_string_value(value: &Value) -> bool {
    match value {
        Value::I64(_) => true,
        Value::String(value) => looks_like_decimal(value),
        _ => false,
    }
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
    require_value_for_field_type(field, operator, elem_type.field_type(), value)
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

fn require_string_value(
    field: &ResolvedField,
    operator: &str,
    value: &Value,
    expected: &str,
) -> Result<()> {
    require_value_shape(field, operator, value, expected, |value| {
        matches!(value, Value::String(_))
    })
}

fn require_bool_value(field: &ResolvedField, operator: &str, value: &Value) -> Result<()> {
    require_value_shape(field, operator, value, "bool", |value| {
        matches!(value, Value::Bool(_))
    })
}

fn require_i64_value(field: &ResolvedField, operator: &str, value: &Value) -> Result<()> {
    require_value_shape(field, operator, value, "integer", |value| {
        matches!(value, Value::I64(_))
    })
}

fn require_bytes_value(field: &ResolvedField, operator: &str, value: &Value) -> Result<()> {
    require_value_shape(field, operator, value, "bytes", |value| {
        matches!(value, Value::Bytes(_))
    })
}

fn require_exact_numeric_value(field: &ResolvedField, operator: &str, value: &Value) -> Result<()> {
    require_value_shape(
        field,
        operator,
        value,
        "integer or numeric string",
        is_exact_numeric_value,
    )
}

fn require_decimal_string_value(
    field: &ResolvedField,
    operator: &str,
    value: &Value,
) -> Result<()> {
    require_value_shape(
        field,
        operator,
        value,
        "integer or decimal string",
        is_decimal_string_value,
    )
}

fn require_jsonb_value(field: &ResolvedField, operator: &str, value: &Value) -> Result<()> {
    if value.is_scalar() || matches!(value, Value::Array(_) | Value::Json(_)) {
        return Ok(());
    }
    Err(Error::InvalidValue {
        field: field.display_name(),
        operator: operator.to_owned(),
        message: format!("expected scalar, array, or JSON, got {}", value.type_name()),
    })
}

fn require_int4_value(field: &ResolvedField, operator: &str, value: &Value) -> Result<()> {
    let Value::I64(value) = value else {
        return Err(Error::InvalidValue {
            field: field.display_name(),
            operator: operator.to_owned(),
            message: format!("expected integer, got {}", value.type_name()),
        });
    };
    if i32::try_from(*value).is_ok() {
        return Ok(());
    }
    Err(Error::InvalidValue {
        field: field.display_name(),
        operator: operator.to_owned(),
        message: format!("integer value `{value}` is outside the PostgreSQL integer range"),
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
