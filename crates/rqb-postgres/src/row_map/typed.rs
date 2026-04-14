#![deny(clippy::wildcard_enum_match_arm)]

use rqb_core::{AggregateType, ElemType, FieldType, SelectColumn, TypeFamily};
use serde_json::{Map, Number, Value as JsonValue};
use tokio_postgres::Row;

use crate::Result;

use super::values::{
    bytes_to_json, f64_to_json, raw_date_array_to_json, raw_date_to_json,
    raw_timestamp_array_to_json, raw_timestamp_to_json, raw_timestamptz_array_to_json,
    raw_timestamptz_to_json, raw_uuid_array_to_json, raw_uuid_to_json, read_array_idx,
    read_scalar_idx,
};

pub fn row_to_json(row: &Row, columns: &[SelectColumn]) -> Result<JsonValue> {
    let mut object = Map::with_capacity(columns.len());
    for (index, column) in columns.iter().enumerate() {
        let alias = column.alias();
        let value = column_to_json(row, index, column)?;
        object.insert(alias, value);
    }
    Ok(JsonValue::Object(object))
}

fn column_to_json(row: &Row, index: usize, column: &SelectColumn) -> Result<JsonValue> {
    match column {
        SelectColumn::Field(field) => field_to_json(row, index, field.ty),
        SelectColumn::Aggregate { ty, .. } => aggregate_to_json(row, index, ty),
        SelectColumn::Expression { ty, .. } => field_to_json(row, index, *ty),
    }
}

fn field_to_json(row: &Row, index: usize, field_type: FieldType) -> Result<JsonValue> {
    match field_type {
        FieldType::Text
        | FieldType::Citext
        | FieldType::Time
        | FieldType::Timetz
        | FieldType::Interval
        | FieldType::Inet
        | FieldType::Cidr
        | FieldType::Range(_)
        | FieldType::Enum(_) => read_scalar_idx(row, index, JsonValue::String),
        FieldType::Uuid => raw_uuid_to_json(row, index),
        FieldType::Timestamp => raw_timestamp_to_json(row, index),
        FieldType::Timestamptz => raw_timestamptz_to_json(row, index),
        FieldType::Date => raw_date_to_json(row, index),
        FieldType::Integer => read_scalar_idx(row, index, |value: i32| {
            JsonValue::Number(Number::from(value))
        }),
        FieldType::BigInt => read_scalar_idx(row, index, |value: i64| {
            JsonValue::Number(Number::from(value))
        }),
        FieldType::Float => read_scalar_idx(row, index, f64_to_json),
        FieldType::Numeric => read_scalar_idx(row, index, JsonValue::String),
        FieldType::Bool => read_scalar_idx(row, index, JsonValue::Bool),
        FieldType::Jsonb => read_scalar_idx(row, index, |value| value),
        FieldType::Bytea => read_scalar_idx(row, index, bytes_to_json),
        FieldType::Custom(type_spec) => custom_field_to_json(row, index, *type_spec),
        FieldType::Array(elem_type) => array_to_json(row, index, elem_type),
    }
}

fn custom_field_to_json(
    row: &Row,
    index: usize,
    type_spec: rqb_core::TypeSpec,
) -> Result<JsonValue> {
    if type_spec.selects_as_text() {
        return read_scalar_idx(row, index, JsonValue::String);
    }

    match type_spec.family {
        TypeFamily::Text
        | TypeFamily::Uuid
        | TypeFamily::Timestamp
        | TypeFamily::Timestamptz
        | TypeFamily::Date
        | TypeFamily::Time
        | TypeFamily::Timetz
        | TypeFamily::Interval
        | TypeFamily::Network
        | TypeFamily::Range
        | TypeFamily::Numeric => read_scalar_idx(row, index, JsonValue::String),
        TypeFamily::Bool => read_scalar_idx(row, index, JsonValue::Bool),
        TypeFamily::Jsonb => read_scalar_idx(row, index, |value| value),
        TypeFamily::Bytes => read_scalar_idx(row, index, bytes_to_json),
    }
}

fn aggregate_to_json(row: &Row, index: usize, ty: &AggregateType) -> Result<JsonValue> {
    match ty {
        AggregateType::Count => read_scalar_idx(row, index, |value: i64| {
            JsonValue::Number(Number::from(value))
        }),
        AggregateType::Sum(field_type)
        | AggregateType::Avg(field_type)
        | AggregateType::Min(field_type)
        | AggregateType::Max(field_type) => field_to_json(row, index, *field_type),
        AggregateType::Json => read_scalar_idx(row, index, |value| value),
        AggregateType::String => read_scalar_idx(row, index, JsonValue::String),
    }
}

fn array_to_json(row: &Row, index: usize, elem_type: ElemType) -> Result<JsonValue> {
    match elem_type {
        ElemType::Text
        | ElemType::Citext
        | ElemType::Time
        | ElemType::Timetz
        | ElemType::Interval
        | ElemType::Enum(_) => read_array_idx(row, index, JsonValue::String),
        ElemType::Uuid => raw_uuid_array_to_json(row, index),
        ElemType::Timestamp => raw_timestamp_array_to_json(row, index),
        ElemType::Timestamptz => raw_timestamptz_array_to_json(row, index),
        ElemType::Date => raw_date_array_to_json(row, index),
        ElemType::Int => read_array_idx(row, index, |value: i32| {
            JsonValue::Number(Number::from(value))
        }),
        ElemType::BigInt => read_array_idx(row, index, |value: i64| {
            JsonValue::Number(Number::from(value))
        }),
        ElemType::Float => read_array_idx(row, index, f64_to_json),
        ElemType::Numeric => read_array_idx(row, index, JsonValue::String),
        ElemType::Bool => read_array_idx(row, index, JsonValue::Bool),
        ElemType::Custom(type_spec) => custom_array_to_json(row, index, *type_spec),
    }
}

fn custom_array_to_json(
    row: &Row,
    index: usize,
    type_spec: rqb_core::TypeSpec,
) -> Result<JsonValue> {
    if type_spec.selects_as_text() {
        return read_array_idx(row, index, JsonValue::String);
    }

    match type_spec.family {
        TypeFamily::Text
        | TypeFamily::Uuid
        | TypeFamily::Timestamp
        | TypeFamily::Timestamptz
        | TypeFamily::Date
        | TypeFamily::Time
        | TypeFamily::Timetz
        | TypeFamily::Interval
        | TypeFamily::Network
        | TypeFamily::Range
        | TypeFamily::Numeric => read_array_idx(row, index, JsonValue::String),
        TypeFamily::Bool => read_array_idx(row, index, JsonValue::Bool),
        TypeFamily::Jsonb => read_array_idx(row, index, |value| value),
        TypeFamily::Bytes => read_array_idx(row, index, bytes_to_json),
    }
}
