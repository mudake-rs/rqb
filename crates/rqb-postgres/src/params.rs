use rqb_core::Value;
use tokio_postgres::types::ToSql;

use crate::{BindParam, BindType};

pub struct PgParams {
    inner: Vec<PgParam>,
}

impl PgParams {
    pub fn from_binds(values: &[BindParam]) -> Self {
        Self {
            inner: values.iter().map(convert_bind).collect(),
        }
    }

    pub fn from_values(values: &[Value]) -> Self {
        Self {
            inner: values
                .iter()
                .map(BindParam::from_value)
                .map(|value| convert_bind(&value))
                .collect(),
        }
    }

    pub fn as_refs(&self) -> Vec<&(dyn ToSql + Sync)> {
        self.inner.iter().map(PgParam::as_ref).collect()
    }
}

enum PgParam {
    NullText(Option<String>),
    NullBool(Option<bool>),
    NullInt4(Option<i32>),
    NullInt8(Option<i64>),
    NullFloat8(Option<f64>),
    NullBytes(Option<Vec<u8>>),
    NullJson(Option<serde_json::Value>),
    NullTextArray(Option<Vec<String>>),
    NullBoolArray(Option<Vec<bool>>),
    NullInt4Array(Option<Vec<i32>>),
    NullInt8Array(Option<Vec<i64>>),
    NullFloat8Array(Option<Vec<f64>>),
    NullBytesArray(Option<Vec<Vec<u8>>>),
    NullJsonArray(Option<Vec<serde_json::Value>>),
    Bool(bool),
    Int4(i32),
    Int8(i64),
    Float8(f64),
    Text(String),
    Bytes(Vec<u8>),
    Json(serde_json::Value),
    BoolArray(Vec<bool>),
    Int4Array(Vec<i32>),
    Int8Array(Vec<i64>),
    Float8Array(Vec<f64>),
    TextArray(Vec<String>),
    BytesArray(Vec<Vec<u8>>),
    JsonArray(Vec<serde_json::Value>),
}

impl PgParam {
    fn as_ref(&self) -> &(dyn ToSql + Sync) {
        match self {
            Self::NullText(value) => value,
            Self::NullBool(value) => value,
            Self::NullInt4(value) => value,
            Self::NullInt8(value) => value,
            Self::NullFloat8(value) => value,
            Self::NullBytes(value) => value,
            Self::NullJson(value) => value,
            Self::NullTextArray(value) => value,
            Self::NullBoolArray(value) => value,
            Self::NullInt4Array(value) => value,
            Self::NullInt8Array(value) => value,
            Self::NullFloat8Array(value) => value,
            Self::NullBytesArray(value) => value,
            Self::NullJsonArray(value) => value,
            Self::Bool(value) => value,
            Self::Int4(value) => value,
            Self::Int8(value) => value,
            Self::Float8(value) => value,
            Self::Text(value) => value,
            Self::Bytes(value) => value,
            Self::Json(value) => value,
            Self::BoolArray(value) => value,
            Self::Int4Array(value) => value,
            Self::Int8Array(value) => value,
            Self::Float8Array(value) => value,
            Self::TextArray(value) => value,
            Self::BytesArray(value) => value,
            Self::JsonArray(value) => value,
        }
    }
}

fn convert_bind(value: &BindParam) -> PgParam {
    match value {
        BindParam::Null(ty) => null_param(*ty),
        BindParam::Bool(value) => PgParam::Bool(*value),
        BindParam::Int4(value) => PgParam::Int4(*value),
        BindParam::Int8(value) => PgParam::Int8(*value),
        BindParam::Float8(value) => PgParam::Float8(*value),
        BindParam::Text(value) => PgParam::Text(value.clone()),
        BindParam::Bytes(value) => PgParam::Bytes(value.clone()),
        BindParam::Json(value) => PgParam::Json(value.clone()),
        BindParam::BoolArray(value) => PgParam::BoolArray(value.clone()),
        BindParam::Int4Array(value) => PgParam::Int4Array(value.clone()),
        BindParam::Int8Array(value) => PgParam::Int8Array(value.clone()),
        BindParam::Float8Array(value) => PgParam::Float8Array(value.clone()),
        BindParam::TextArray(value) => PgParam::TextArray(value.clone()),
        BindParam::BytesArray(value) => PgParam::BytesArray(value.clone()),
        BindParam::JsonArray(value) => PgParam::JsonArray(value.clone()),
    }
}

fn null_param(ty: BindType) -> PgParam {
    match ty {
        BindType::Text => PgParam::NullText(None),
        BindType::Bool => PgParam::NullBool(None),
        BindType::Int4 => PgParam::NullInt4(None),
        BindType::Int8 => PgParam::NullInt8(None),
        BindType::Float8 => PgParam::NullFloat8(None),
        BindType::Bytes => PgParam::NullBytes(None),
        BindType::Json => PgParam::NullJson(None),
        BindType::TextArray => PgParam::NullTextArray(None),
        BindType::BoolArray => PgParam::NullBoolArray(None),
        BindType::Int4Array => PgParam::NullInt4Array(None),
        BindType::Int8Array => PgParam::NullInt8Array(None),
        BindType::Float8Array => PgParam::NullFloat8Array(None),
        BindType::BytesArray => PgParam::NullBytesArray(None),
        BindType::JsonArray => PgParam::NullJsonArray(None),
    }
}
