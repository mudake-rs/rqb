use crate::error::{Error, Result};
use crate::expr::Operator;
use crate::field::ResolvedField;
use crate::value::Value;

pub(super) fn reject_non_finite_numbers(
    field: &ResolvedField,
    operator: &str,
    value: &Value,
) -> Result<()> {
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

pub(super) fn reject_json_for_non_jsonb_field(
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

pub(super) fn require_array(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
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

pub(super) fn require_string_array(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
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

pub(super) fn require_between(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
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

pub(super) fn require_string(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
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

pub(super) fn require_scalar(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
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

pub(super) fn require_scalar_or_json(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
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

pub(super) fn require_scalar_array_or_json(
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

pub(super) fn require_scalar_json_or_null(
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

pub(super) fn require_scalar_array_json_or_null(
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

pub(super) fn require_number(
    field: &ResolvedField,
    operator: Operator,
    value: &Value,
) -> Result<()> {
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
