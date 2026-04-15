use serde_json::Value as JsonValue;

use crate::typed::{JsonKind, Param};
use crate::{Error, Result};

pub(super) fn json_array<'a>(
    field: &str,
    value: &'a JsonValue,
    expected: &'static str,
) -> Result<&'a Vec<JsonValue>> {
    value.as_array().ok_or_else(|| Error::InvalidSearchValue {
        field: field.to_owned(),
        expected,
    })
}

pub(super) fn json_param(field: &str, kind: JsonKind, value: &JsonValue) -> Result<Param> {
    match kind {
        JsonKind::Text => value
            .as_str()
            .map(|value| Param::typed(value.to_owned()))
            .ok_or_else(|| invalid_value(field, "string")),
        JsonKind::Bool => value
            .as_bool()
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "boolean")),
        JsonKind::Integer => value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "32-bit integer")),
        JsonKind::BigInt => value
            .as_i64()
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "64-bit integer")),
        JsonKind::Float => value
            .as_f64()
            .filter(|value| value.is_finite())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "finite number")),
        JsonKind::NumericString => value
            .as_str()
            .and_then(|value| value.parse::<sqlx::types::BigDecimal>().ok())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "decimal string")),
        JsonKind::Uuid => value
            .as_str()
            .and_then(|value| value.parse::<uuid::Uuid>().ok())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "UUID string")),
        JsonKind::Date => value
            .as_str()
            .and_then(|value| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "date string")),
        JsonKind::Time => value
            .as_str()
            .and_then(|value| chrono::NaiveTime::parse_from_str(value, "%H:%M:%S%.f").ok())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "time string")),
        JsonKind::Timestamp => value
            .as_str()
            .and_then(parse_naive_datetime)
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "timestamp string")),
        JsonKind::Timestamptz => value
            .as_str()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| Param::typed(value.with_timezone(&chrono::Utc)))
            .ok_or_else(|| invalid_value(field, "RFC3339 timestamp string")),
        JsonKind::Jsonb => Ok(Param::typed(value.clone())),
    }
}

fn parse_naive_datetime(value: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f"))
        .ok()
}

fn invalid_value(field: &str, expected: &'static str) -> Error {
    Error::InvalidSearchValue {
        field: field.to_owned(),
        expected,
    }
}
