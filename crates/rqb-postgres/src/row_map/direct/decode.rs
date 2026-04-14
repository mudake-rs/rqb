#![deny(clippy::wildcard_enum_match_arm)]

use rqb_core::{AggregateType, ElemType, FieldType, SelectColumn, TypeFamily};
use serde::de::Error as _;
use serde_json::Error as JsonError;
use tokio_postgres::{
    Row,
    types::{FromSql, Type},
};

use super::DeResult;
use super::value::{DecodedArray, DecodedValue};

pub(super) fn column_to_decoded(
    row: &Row,
    index: usize,
    column: &SelectColumn,
) -> DeResult<DecodedValue> {
    match column {
        SelectColumn::Field(field) => field_to_decoded(row, index, field.ty),
        SelectColumn::Aggregate { ty, .. } => aggregate_to_decoded(row, index, ty),
        SelectColumn::Expression { ty, .. } => field_to_decoded(row, index, *ty),
    }
}

pub(super) fn raw_column_to_decoded(row: &Row, index: usize, ty: &Type) -> DeResult<DecodedValue> {
    match *ty {
        Type::BOOL => read_decoded_scalar(row, index, DecodedValue::Bool),
        Type::INT2 => read_decoded_scalar(row, index, |value: i16| DecodedValue::I32(value.into())),
        Type::INT4 => read_decoded_scalar(row, index, DecodedValue::I32),
        Type::INT8 => read_decoded_scalar(row, index, DecodedValue::I64),
        Type::FLOAT4 => {
            read_decoded_scalar(row, index, |value: f32| DecodedValue::F64(value.into()))
        }
        Type::FLOAT8 => read_decoded_scalar(row, index, DecodedValue::F64),
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME => {
            read_decoded_scalar(row, index, DecodedValue::String)
        }
        Type::JSON | Type::JSONB => read_decoded_scalar(row, index, DecodedValue::Json),
        Type::BYTEA => read_decoded_scalar(row, index, DecodedValue::Bytes),
        Type::BOOL_ARRAY => read_decoded_array(row, index, DecodedArray::Bool),
        Type::INT2_ARRAY => read_decoded_array(row, index, |values: Vec<i16>| {
            DecodedArray::I32(values.into_iter().map(Into::into).collect())
        }),
        Type::INT4_ARRAY => read_decoded_array(row, index, DecodedArray::I32),
        Type::INT8_ARRAY => read_decoded_array(row, index, DecodedArray::I64),
        Type::FLOAT4_ARRAY => read_decoded_array(row, index, |values: Vec<f32>| {
            DecodedArray::F64(values.into_iter().map(Into::into).collect())
        }),
        Type::FLOAT8_ARRAY => read_decoded_array(row, index, DecodedArray::F64),
        Type::TEXT_ARRAY | Type::VARCHAR_ARRAY | Type::BPCHAR_ARRAY | Type::NAME_ARRAY => {
            read_decoded_array(row, index, DecodedArray::String)
        }
        Type::JSON_ARRAY | Type::JSONB_ARRAY => read_decoded_array(row, index, DecodedArray::Json),
        Type::BYTEA_ARRAY => read_decoded_array(row, index, DecodedArray::Bytes),
        Type::UUID => uuid_to_decoded(row, index),
        Type::UUID_ARRAY => uuid_array_to_decoded(row, index),
        Type::TIMESTAMP => timestamp_to_decoded(row, index),
        Type::TIMESTAMP_ARRAY => timestamp_array_to_decoded(row, index),
        Type::TIMESTAMPTZ => timestamptz_to_decoded(row, index),
        Type::TIMESTAMPTZ_ARRAY => timestamptz_array_to_decoded(row, index),
        Type::DATE => date_to_decoded(row, index),
        Type::DATE_ARRAY => date_array_to_decoded(row, index),
        _ => Err(JsonError::custom(format!(
            "raw query column `{}` has unsupported Postgres type `{}`; cast it to a supported type",
            row.columns()[index].name(),
            ty.name()
        ))),
    }
}

fn field_to_decoded(row: &Row, index: usize, field_type: FieldType) -> DeResult<DecodedValue> {
    match field_type {
        FieldType::Text
        | FieldType::Citext
        | FieldType::Time
        | FieldType::Timetz
        | FieldType::Interval
        | FieldType::Inet
        | FieldType::Cidr
        | FieldType::Range(_)
        | FieldType::Enum(_) => read_decoded_scalar(row, index, DecodedValue::String),
        FieldType::Uuid => uuid_to_decoded(row, index),
        FieldType::Timestamp => timestamp_to_decoded(row, index),
        FieldType::Timestamptz => timestamptz_to_decoded(row, index),
        FieldType::Date => date_to_decoded(row, index),
        FieldType::Integer => read_decoded_scalar(row, index, DecodedValue::I32),
        FieldType::BigInt => read_decoded_scalar(row, index, DecodedValue::I64),
        FieldType::Float => read_decoded_scalar(row, index, DecodedValue::F64),
        FieldType::Numeric => read_decoded_scalar(row, index, DecodedValue::String),
        FieldType::Bool => read_decoded_scalar(row, index, DecodedValue::Bool),
        FieldType::Jsonb => read_decoded_scalar(row, index, DecodedValue::Json),
        FieldType::Bytea => read_decoded_scalar(row, index, DecodedValue::Bytes),
        FieldType::Custom(type_spec) => custom_field_to_decoded(row, index, *type_spec),
        FieldType::Array(elem_type) => array_to_decoded(row, index, elem_type),
    }
}

fn custom_field_to_decoded(
    row: &Row,
    index: usize,
    type_spec: rqb_core::TypeSpec,
) -> DeResult<DecodedValue> {
    if type_spec.selects_as_text() {
        return read_decoded_scalar(row, index, DecodedValue::String);
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
        | TypeFamily::Numeric => read_decoded_scalar(row, index, DecodedValue::String),
        TypeFamily::Bool => read_decoded_scalar(row, index, DecodedValue::Bool),
        TypeFamily::Jsonb => read_decoded_scalar(row, index, DecodedValue::Json),
        TypeFamily::Bytes => read_decoded_scalar(row, index, DecodedValue::Bytes),
    }
}

fn aggregate_to_decoded(row: &Row, index: usize, ty: &AggregateType) -> DeResult<DecodedValue> {
    match ty {
        AggregateType::Count => read_decoded_scalar(row, index, DecodedValue::I64),
        AggregateType::Sum(field_type)
        | AggregateType::Avg(field_type)
        | AggregateType::Min(field_type)
        | AggregateType::Max(field_type) => field_to_decoded(row, index, *field_type),
        AggregateType::Json => read_decoded_scalar(row, index, DecodedValue::Json),
        AggregateType::String => read_decoded_scalar(row, index, DecodedValue::String),
    }
}

fn array_to_decoded(row: &Row, index: usize, elem_type: ElemType) -> DeResult<DecodedValue> {
    match elem_type {
        ElemType::Text
        | ElemType::Citext
        | ElemType::Time
        | ElemType::Timetz
        | ElemType::Interval
        | ElemType::Enum(_) => read_decoded_array(row, index, DecodedArray::String),
        ElemType::Uuid => uuid_array_to_decoded(row, index),
        ElemType::Timestamp => timestamp_array_to_decoded(row, index),
        ElemType::Timestamptz => timestamptz_array_to_decoded(row, index),
        ElemType::Date => date_array_to_decoded(row, index),
        ElemType::Int => read_decoded_array(row, index, DecodedArray::I32),
        ElemType::BigInt => read_decoded_array(row, index, DecodedArray::I64),
        ElemType::Float => read_decoded_array(row, index, DecodedArray::F64),
        ElemType::Numeric => read_decoded_array(row, index, DecodedArray::String),
        ElemType::Bool => read_decoded_array(row, index, DecodedArray::Bool),
        ElemType::Custom(type_spec) => custom_array_to_decoded(row, index, *type_spec),
    }
}

fn custom_array_to_decoded(
    row: &Row,
    index: usize,
    type_spec: rqb_core::TypeSpec,
) -> DeResult<DecodedValue> {
    if type_spec.selects_as_text() {
        return read_decoded_array(row, index, DecodedArray::String);
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
        | TypeFamily::Numeric => read_decoded_array(row, index, DecodedArray::String),
        TypeFamily::Bool => read_decoded_array(row, index, DecodedArray::Bool),
        TypeFamily::Jsonb => read_decoded_array(row, index, DecodedArray::Json),
        TypeFamily::Bytes => read_decoded_array(row, index, DecodedArray::Bytes),
    }
}

fn read_decoded_scalar<T, F>(row: &Row, index: usize, map: F) -> DeResult<DecodedValue>
where
    T: for<'a> FromSql<'a>,
    F: FnOnce(T) -> DecodedValue,
{
    row.try_get::<_, Option<T>>(index)
        .map(|value| value.map_or(DecodedValue::Null, map))
        .map_err(to_json_error)
}

fn read_decoded_array<T, F>(row: &Row, index: usize, map: F) -> DeResult<DecodedValue>
where
    T: for<'a> FromSql<'a>,
    F: FnOnce(Vec<T>) -> DecodedArray,
{
    row.try_get::<_, Option<Vec<T>>>(index)
        .map(|value| {
            value.map_or(DecodedValue::Null, |values| {
                DecodedValue::Array(map(values))
            })
        })
        .map_err(to_json_error)
}

fn to_json_error(error: tokio_postgres::Error) -> JsonError {
    JsonError::custom(error.to_string())
}

fn uuid_to_decoded(row: &Row, index: usize) -> DeResult<DecodedValue> {
    read_decoded_scalar(row, index, |value: uuid::Uuid| {
        DecodedValue::String(value.to_string())
    })
}

fn uuid_array_to_decoded(row: &Row, index: usize) -> DeResult<DecodedValue> {
    read_decoded_array(row, index, |values: Vec<uuid::Uuid>| {
        DecodedArray::String(values.into_iter().map(|value| value.to_string()).collect())
    })
}

fn timestamp_to_decoded(row: &Row, index: usize) -> DeResult<DecodedValue> {
    read_decoded_scalar(row, index, |value: chrono::NaiveDateTime| {
        DecodedValue::String(value.to_string())
    })
}

fn timestamp_array_to_decoded(row: &Row, index: usize) -> DeResult<DecodedValue> {
    read_decoded_array(row, index, |values: Vec<chrono::NaiveDateTime>| {
        DecodedArray::String(values.into_iter().map(|value| value.to_string()).collect())
    })
}

fn timestamptz_to_decoded(row: &Row, index: usize) -> DeResult<DecodedValue> {
    read_decoded_scalar(row, index, |value: chrono::DateTime<chrono::Utc>| {
        DecodedValue::String(value.to_rfc3339())
    })
}

fn timestamptz_array_to_decoded(row: &Row, index: usize) -> DeResult<DecodedValue> {
    read_decoded_array(row, index, |values: Vec<chrono::DateTime<chrono::Utc>>| {
        DecodedArray::String(values.into_iter().map(|value| value.to_rfc3339()).collect())
    })
}

fn date_to_decoded(row: &Row, index: usize) -> DeResult<DecodedValue> {
    read_decoded_scalar(row, index, |value: chrono::NaiveDate| {
        DecodedValue::String(value.to_string())
    })
}

fn date_array_to_decoded(row: &Row, index: usize) -> DeResult<DecodedValue> {
    read_decoded_array(row, index, |values: Vec<chrono::NaiveDate>| {
        DecodedArray::String(values.into_iter().map(|value| value.to_string()).collect())
    })
}
