use rqb_core::{AggregateType, ElemType, FieldType, SelectColumn, SelectRepr, TypeFamily};
use serde_json::{Map, Number, Value as JsonValue};
use tokio_postgres::{
    Row,
    types::{FromSql, Type},
};

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

pub fn raw_row_to_json(row: &Row) -> Result<JsonValue> {
    let mut object = Map::new();
    for (idx, column) in row.columns().iter().enumerate() {
        object.insert(
            column.name().to_owned(),
            raw_column_to_json(row, idx, column.type_())?,
        );
    }
    Ok(JsonValue::Object(object))
}

fn raw_column_to_json(row: &Row, idx: usize, ty: &Type) -> Result<JsonValue> {
    match *ty {
        Type::BOOL => read_scalar_idx(row, idx, JsonValue::Bool),
        Type::INT2 => read_scalar_idx(row, idx, |value: i16| {
            JsonValue::Number(Number::from(value))
        }),
        Type::INT4 => read_scalar_idx(row, idx, |value: i32| {
            JsonValue::Number(Number::from(value))
        }),
        Type::INT8 => read_scalar_idx(row, idx, |value: i64| {
            JsonValue::Number(Number::from(value))
        }),
        Type::FLOAT4 => read_scalar_idx(row, idx, |value: f32| f64_to_json(value.into())),
        Type::FLOAT8 => read_scalar_idx(row, idx, f64_to_json),
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME => {
            read_scalar_idx(row, idx, JsonValue::String)
        }
        Type::JSON | Type::JSONB => read_scalar_idx(row, idx, |value| value),
        Type::BYTEA => read_scalar_idx(row, idx, bytes_to_json),
        Type::BOOL_ARRAY => read_array_idx(row, idx, JsonValue::Bool),
        Type::INT2_ARRAY => read_array_idx(row, idx, |value: i16| {
            JsonValue::Number(Number::from(value))
        }),
        Type::INT4_ARRAY => read_array_idx(row, idx, |value: i32| {
            JsonValue::Number(Number::from(value))
        }),
        Type::INT8_ARRAY => read_array_idx(row, idx, |value: i64| {
            JsonValue::Number(Number::from(value))
        }),
        Type::FLOAT4_ARRAY => read_array_idx(row, idx, |value: f32| f64_to_json(value.into())),
        Type::FLOAT8_ARRAY => read_array_idx(row, idx, f64_to_json),
        Type::TEXT_ARRAY | Type::VARCHAR_ARRAY | Type::BPCHAR_ARRAY | Type::NAME_ARRAY => {
            read_array_idx(row, idx, JsonValue::String)
        }
        Type::JSON_ARRAY | Type::JSONB_ARRAY => read_array_idx(row, idx, |value| value),
        Type::BYTEA_ARRAY => read_array_idx(row, idx, bytes_to_json),
        Type::UUID => raw_uuid_to_json(row, idx),
        Type::UUID_ARRAY => raw_uuid_array_to_json(row, idx),
        Type::TIMESTAMP => raw_timestamp_to_json(row, idx),
        Type::TIMESTAMP_ARRAY => raw_timestamp_array_to_json(row, idx),
        Type::TIMESTAMPTZ => raw_timestamptz_to_json(row, idx),
        Type::TIMESTAMPTZ_ARRAY => raw_timestamptz_array_to_json(row, idx),
        Type::DATE => raw_date_to_json(row, idx),
        Type::DATE_ARRAY => raw_date_array_to_json(row, idx),
        _ => Err(Error::Deserialize(format!(
            "raw query column `{}` has unsupported Postgres type `{}`; cast it to a supported type",
            row.columns()[idx].name(),
            ty.name()
        ))),
    }
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
        ElemType::Custom(type_spec) => custom_array_to_json(row, alias, *type_spec),
    }
}

fn custom_array_to_json(
    row: &Row,
    alias: &str,
    type_spec: rqb_core::TypeSpec,
) -> Result<JsonValue> {
    if type_spec.select_repr == SelectRepr::Text {
        return read_array(row, alias, JsonValue::String);
    }

    match type_spec.family {
        TypeFamily::Text
        | TypeFamily::Uuid
        | TypeFamily::Timestamp
        | TypeFamily::Timestamptz
        | TypeFamily::Date
        | TypeFamily::Network
        | TypeFamily::Range => read_array(row, alias, JsonValue::String),
        TypeFamily::Numeric => read_array(row, alias, JsonValue::String),
        TypeFamily::Bool => read_array(row, alias, JsonValue::Bool),
        TypeFamily::Jsonb => read_array(row, alias, |value| value),
        TypeFamily::Bytes => read_array(row, alias, bytes_to_json),
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

fn read_scalar_idx<T, F>(row: &Row, idx: usize, to_json: F) -> Result<JsonValue>
where
    T: for<'a> FromSql<'a>,
    F: Fn(T) -> JsonValue,
{
    row.try_get::<_, Option<T>>(idx)
        .map(|value| value.map_or(JsonValue::Null, to_json))
        .map_err(Error::from)
}

fn read_array_idx<T, F>(row: &Row, idx: usize, to_json: F) -> Result<JsonValue>
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

#[cfg(feature = "with-uuid")]
fn raw_uuid_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, |value: uuid::Uuid| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-uuid"))]
fn raw_uuid_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-uuid")]
fn raw_uuid_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, |value: uuid::Uuid| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-uuid"))]
fn raw_uuid_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn raw_timestamp_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, |value: chrono::NaiveDateTime| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn raw_timestamp_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn raw_timestamp_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, |value: chrono::NaiveDateTime| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn raw_timestamp_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn raw_timestamptz_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, |value: chrono::DateTime<chrono::Utc>| {
        JsonValue::String(value.to_rfc3339())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn raw_timestamptz_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn raw_timestamptz_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, |value: chrono::DateTime<chrono::Utc>| {
        JsonValue::String(value.to_rfc3339())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn raw_timestamptz_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn raw_date_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, |value: chrono::NaiveDate| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn raw_date_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_scalar_idx(row, idx, JsonValue::String)
}

#[cfg(feature = "with-chrono")]
fn raw_date_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, |value: chrono::NaiveDate| {
        JsonValue::String(value.to_string())
    })
}

#[cfg(not(feature = "with-chrono"))]
fn raw_date_array_to_json(row: &Row, idx: usize) -> Result<JsonValue> {
    read_array_idx(row, idx, JsonValue::String)
}
