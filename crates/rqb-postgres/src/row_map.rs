use rqb_core::{AggregateType, ElemType, FieldType, SelectColumn, SelectRepr, TypeFamily};
use serde_json::{Map, Number, Value as JsonValue};
use tokio_postgres::{Row, types::FromSql};

use crate::{Error, Result};

pub fn row_to_json(row: &Row, columns: &[SelectColumn]) -> Result<JsonValue> {
    let mut object = Map::new();
    for column in columns {
        let alias = column.alias();
        let value = match column {
            SelectColumn::Field(field) => field_to_json(row, &alias, field.ty)?,
            SelectColumn::Aggregate { ty, .. } => aggregate_to_json(row, &alias, ty)?,
        };
        object.insert(alias, value);
    }
    Ok(JsonValue::Object(object))
}

fn field_to_json(row: &Row, alias: &str, field_type: FieldType) -> Result<JsonValue> {
    match field_type {
        FieldType::Text
        | FieldType::Citext
        | FieldType::Inet
        | FieldType::Cidr
        | FieldType::Range(_)
        | FieldType::Enum(_) => read_scalar(row, alias, JsonValue::String),
        FieldType::Uuid => uuid_to_json(row, alias),
        FieldType::Timestamp => timestamp_to_json(row, alias),
        FieldType::Timestamptz => timestamptz_to_json(row, alias),
        FieldType::Date => date_to_json(row, alias),
        FieldType::Integer => read_scalar(row, alias, |value: i32| {
            JsonValue::Number(Number::from(value))
        }),
        FieldType::BigInt => read_scalar(row, alias, |value: i64| {
            JsonValue::Number(Number::from(value))
        }),
        FieldType::Float => read_scalar(row, alias, f64_to_json),
        FieldType::Numeric => read_scalar(row, alias, JsonValue::String),
        FieldType::Bool => read_scalar(row, alias, JsonValue::Bool),
        FieldType::Jsonb => read_scalar(row, alias, |value| value),
        FieldType::Bytea => read_scalar(row, alias, bytes_to_json),
        FieldType::Custom(type_spec) => custom_field_to_json(row, alias, *type_spec),
        FieldType::Array(elem_type) => array_to_json(row, alias, elem_type),
    }
}

fn custom_field_to_json(
    row: &Row,
    alias: &str,
    type_spec: rqb_core::TypeSpec,
) -> Result<JsonValue> {
    if type_spec.select_repr == SelectRepr::Text {
        return read_scalar(row, alias, JsonValue::String);
    }

    match type_spec.family {
        TypeFamily::Text
        | TypeFamily::Uuid
        | TypeFamily::Timestamp
        | TypeFamily::Timestamptz
        | TypeFamily::Date
        | TypeFamily::Network
        | TypeFamily::Range => read_scalar(row, alias, JsonValue::String),
        TypeFamily::Numeric => read_scalar(row, alias, JsonValue::String),
        TypeFamily::Bool => read_scalar(row, alias, JsonValue::Bool),
        TypeFamily::Jsonb => read_scalar(row, alias, |value| value),
        TypeFamily::Bytes => read_scalar(row, alias, bytes_to_json),
    }
}

fn aggregate_to_json(row: &Row, alias: &str, ty: &AggregateType) -> Result<JsonValue> {
    match ty {
        AggregateType::Count => read_scalar(row, alias, |value: i64| {
            JsonValue::Number(Number::from(value))
        }),
        AggregateType::Sum | AggregateType::Avg => read_scalar(row, alias, f64_to_json),
        AggregateType::Min(field_type) | AggregateType::Max(field_type) => {
            field_to_json(row, alias, *field_type)
        }
        AggregateType::Json => read_scalar(row, alias, |value| value),
        AggregateType::String => read_scalar(row, alias, JsonValue::String),
    }
}

fn array_to_json(row: &Row, alias: &str, elem_type: ElemType) -> Result<JsonValue> {
    match elem_type {
        ElemType::Text | ElemType::Citext | ElemType::Enum(_) => {
            read_array(row, alias, JsonValue::String)
        }
        ElemType::Uuid => uuid_array_to_json(row, alias),
        ElemType::Timestamp => timestamp_array_to_json(row, alias),
        ElemType::Timestamptz => timestamptz_array_to_json(row, alias),
        ElemType::Date => date_array_to_json(row, alias),
        ElemType::Int => read_array(row, alias, |value: i32| {
            JsonValue::Number(Number::from(value))
        }),
        ElemType::BigInt => read_array(row, alias, |value: i64| {
            JsonValue::Number(Number::from(value))
        }),
        ElemType::Float => read_array(row, alias, f64_to_json),
        ElemType::Numeric => read_array(row, alias, JsonValue::String),
        ElemType::Bool => read_array(row, alias, JsonValue::Bool),
    }
}

fn bytes_to_json(value: Vec<u8>) -> JsonValue {
    JsonValue::Array(
        value
            .into_iter()
            .map(|byte| JsonValue::Number(Number::from(byte)))
            .collect(),
    )
}

fn read_scalar<T, F>(row: &Row, alias: &str, to_json: F) -> Result<JsonValue>
where
    T: for<'a> FromSql<'a>,
    F: Fn(T) -> JsonValue,
{
    row.try_get::<_, Option<T>>(alias)
        .map(|value| value.map_or(JsonValue::Null, to_json))
        .map_err(Error::from)
}

fn read_array<T, F>(row: &Row, alias: &str, to_json: F) -> Result<JsonValue>
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

fn f64_to_json(value: f64) -> JsonValue {
    Number::from_f64(value)
        .map(JsonValue::Number)
        .unwrap_or(JsonValue::Null)
}

#[cfg(feature = "with-uuid")]
fn uuid_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, |value: uuid::Uuid| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-uuid"))]
fn uuid_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, JsonValue::String)
}

#[cfg(feature = "with-uuid")]
fn uuid_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, |value: uuid::Uuid| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-uuid"))]
fn uuid_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn timestamp_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, |value: chrono::NaiveDateTime| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn timestamp_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn timestamp_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, |value: chrono::NaiveDateTime| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn timestamp_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn timestamptz_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, |value: chrono::DateTime<chrono::Utc>| {
        JsonValue::String(value.to_rfc3339())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn timestamptz_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn timestamptz_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, |value: chrono::DateTime<chrono::Utc>| {
        JsonValue::String(value.to_rfc3339())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn timestamptz_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn date_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, |value: chrono::NaiveDate| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn date_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_scalar(row, alias, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn date_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, |value: chrono::NaiveDate| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn date_array_to_json(row: &Row, alias: &str) -> Result<JsonValue> {
    read_array(row, alias, JsonValue::String)
}
