use rqb_core::{AggregateType, ElemType, FieldType, SelectColumn, SelectRepr, TypeFamily};
use serde_json::{Map, Number, Value as JsonValue};
use tokio_postgres::Row;

use crate::Result;

use super::{
    bytes_to_json, date_array_to_json, date_to_json, f64_to_json, read_array, read_scalar,
    timestamp_array_to_json, timestamp_to_json, timestamptz_array_to_json, timestamptz_to_json,
    uuid_array_to_json, uuid_to_json,
};

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
