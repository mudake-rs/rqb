use serde_json::{Map, Number, Value as JsonValue};
use tokio_postgres::{Row, types::Type};

use crate::{Error, Result};

use super::{
    bytes_to_json, f64_to_json, raw_date_array_to_json, raw_date_to_json,
    raw_timestamp_array_to_json, raw_timestamp_to_json, raw_timestamptz_array_to_json,
    raw_timestamptz_to_json, raw_uuid_array_to_json, raw_uuid_to_json, read_array_idx,
    read_scalar_idx,
};

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
