use serde_json::{Number, Value as JsonValue};
use tokio_postgres::{Row, types::FromSql};

use crate::{Error, Result};

pub(super) fn bytes_to_json(value: Vec<u8>) -> JsonValue {
    JsonValue::Array(
        value
            .into_iter()
            .map(|byte| JsonValue::Number(Number::from(byte)))
            .collect(),
    )
}

pub(super) fn read_scalar<T, F>(row: &Row, alias: &str, to_json: F) -> Result<JsonValue>
where
    T: for<'a> FromSql<'a>,
    F: Fn(T) -> JsonValue,
{
    row.try_get::<_, Option<T>>(alias)
        .map(|value| value.map_or(JsonValue::Null, to_json))
        .map_err(Error::from)
}

pub(super) fn read_array<T, F>(row: &Row, alias: &str, to_json: F) -> Result<JsonValue>
where
    T: for<'a> FromSql<'a>,
    F: Fn(T) -> JsonValue,
{
    row.try_get::<_, Option<Vec<T>>>(alias)
        .map(|value| {
            value.map_or(JsonValue::Null, |values| {
                JsonValue::Array(values.into_iter().map(to_json).collect())
            })
        })
        .map_err(Error::from)
}

pub(super) fn read_scalar_idx<T, F>(row: &Row, idx: usize, to_json: F) -> Result<JsonValue>
where
    T: for<'a> FromSql<'a>,
    F: Fn(T) -> JsonValue,
{
    row.try_get::<_, Option<T>>(idx)
        .map(|value| value.map_or(JsonValue::Null, to_json))
        .map_err(Error::from)
}

pub(super) fn read_array_idx<T, F>(row: &Row, idx: usize, to_json: F) -> Result<JsonValue>
where
    T: for<'a> FromSql<'a>,
    F: Fn(T) -> JsonValue,
{
    row.try_get::<_, Option<Vec<T>>>(idx)
        .map(|value| {
            value.map_or(JsonValue::Null, |values| {
                JsonValue::Array(values.into_iter().map(to_json).collect())
            })
        })
        .map_err(Error::from)
}

pub(super) fn f64_to_json(value: f64) -> JsonValue {
    Number::from_f64(value)
        .map(JsonValue::Number)
        .unwrap_or(JsonValue::Null)
}

#[cfg(feature = "with-uuid")]
pub(super) fn uuid_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, |value: uuid::Uuid| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-uuid"))]
pub(super) fn uuid_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, JsonValue::String)
}

#[cfg(feature = "with-uuid")]
pub(super) fn uuid_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, |value: uuid::Uuid| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-uuid"))]
pub(super) fn uuid_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn timestamp_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, |value: chrono::NaiveDateTime| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn timestamp_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn timestamp_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, |value: chrono::NaiveDateTime| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn timestamp_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn timestamptz_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, |value: chrono::DateTime<chrono::Utc>| {
        JsonValue::String(value.to_rfc3339())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn timestamptz_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn timestamptz_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, |value: chrono::DateTime<chrono::Utc>| {
        JsonValue::String(value.to_rfc3339())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn timestamptz_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn date_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, |value: chrono::NaiveDate| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn date_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn date_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, |value: chrono::NaiveDate| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn date_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, JsonValue::String)
}

#[cfg(feature = "with-uuid")]
pub(super) fn raw_uuid_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, |value: uuid::Uuid| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-uuid"))]
pub(super) fn raw_uuid_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-uuid")]
pub(super) fn raw_uuid_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, |value: uuid::Uuid| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-uuid"))]
pub(super) fn raw_uuid_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn raw_timestamp_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, |value: chrono::NaiveDateTime| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn raw_timestamp_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn raw_timestamp_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, |value: chrono::NaiveDateTime| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn raw_timestamp_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn raw_timestamptz_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, |value: chrono::DateTime<chrono::Utc>| {
        JsonValue::String(value.to_rfc3339())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn raw_timestamptz_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn raw_timestamptz_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, |value: chrono::DateTime<chrono::Utc>| {
        JsonValue::String(value.to_rfc3339())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn raw_timestamptz_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn raw_date_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, |value: chrono::NaiveDate| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn raw_date_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
pub(super) fn raw_date_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, |value: chrono::NaiveDate| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
pub(super) fn raw_date_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, JsonValue::String)
}
