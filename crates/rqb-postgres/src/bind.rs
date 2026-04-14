use rqb_core::{ElemType, FieldType, TypeFamily, Value};

/// A concrete Postgres bind value chosen after rqb validation.
///
/// `rqb_core::Value` intentionally keeps integers backend-neutral as `i64`.
/// The Postgres renderer lowers those values into concrete wire types here,
/// for example `FieldType::Integer` becomes `BindParam::Int4`.
#[derive(Clone, Debug, PartialEq)]
pub enum BindParam {
    Null(BindType),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindType {
    Text,
    Bool,
    Int4,
    Int8,
    Float8,
    Bytes,
    Json,
    TextArray,
    BoolArray,
    Int4Array,
    Int8Array,
    Float8Array,
    BytesArray,
    JsonArray,
}

impl BindParam {
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null(BindType::Text),
            Value::Bool(value) => Self::Bool(*value),
            Value::I64(value) => Self::Int8(*value),
            Value::F64(value) => Self::Float8(*value),
            Value::String(value) => Self::Text(value.clone()),
            Value::Bytes(value) => Self::Bytes(value.clone()),
            Value::Json(value) => Self::Json(value.clone()),
            Value::Array(values) => array_from_values(values),
        }
    }

    pub fn from_typed_value(value: &Value, field_type: FieldType) -> Self {
        if value.is_null() {
            return Self::Null(null_bind_type(field_type));
        }

        match field_type {
            FieldType::Integer => Self::Int4(expect_int4(value)),
            FieldType::BigInt => Self::Int8(expect_i64(value)),
            FieldType::Float => Self::Float8(expect_float8(value)),
            FieldType::Bool => Self::Bool(expect_bool(value)),
            FieldType::Bytea => Self::Bytes(expect_bytes(value).to_vec()),
            FieldType::Jsonb => Self::Json(value_to_json(value)),
            FieldType::Array(elem_type) => typed_array_from_value(value, elem_type),
            FieldType::Text
            | FieldType::Citext
            | FieldType::Uuid
            | FieldType::Timestamp
            | FieldType::Timestamptz
            | FieldType::Date
            | FieldType::Inet
            | FieldType::Cidr
            | FieldType::Enum(_)
            | FieldType::Range(_) => Self::Text(expect_string(value).to_owned()),
            FieldType::Numeric => {
                unreachable!("numeric values are lowered through the text::numeric render path")
            }
            FieldType::Custom(type_spec) => match type_spec.value_repr {
                rqb_core::ValueRepr::String | rqb_core::ValueRepr::DecimalString => {
                    Self::Text(expect_string_like(value))
                }
                rqb_core::ValueRepr::Native => match type_spec.family {
                    TypeFamily::Text
                    | TypeFamily::Uuid
                    | TypeFamily::Timestamp
                    | TypeFamily::Timestamptz
                    | TypeFamily::Date
                    | TypeFamily::Network
                    | TypeFamily::Range => Self::Text(expect_string(value).to_owned()),
                    TypeFamily::Bool => Self::Bool(expect_bool(value)),
                    TypeFamily::Numeric => Self::from_value(value),
                    TypeFamily::Jsonb => Self::Json(value_to_json(value)),
                    TypeFamily::Bytes => Self::Bytes(expect_bytes(value).to_vec()),
                },
            },
        }
    }
}

impl From<&str> for BindParam {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for BindParam {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<bool> for BindParam {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for BindParam {
    fn from(value: i64) -> Self {
        Self::Int8(value)
    }
}

impl From<serde_json::Value> for BindParam {
    fn from(value: serde_json::Value) -> Self {
        Self::Json(value)
    }
}

impl From<Value> for BindParam {
    fn from(value: Value) -> Self {
        Self::from_value(&value)
    }
}

fn null_bind_type(field_type: FieldType) -> BindType {
    match field_type {
        FieldType::Bool => BindType::Bool,
        FieldType::Integer => BindType::Int4,
        FieldType::BigInt => BindType::Int8,
        FieldType::Float => BindType::Float8,
        FieldType::Jsonb => BindType::Json,
        FieldType::Bytea => BindType::Bytes,
        FieldType::Array(elem_type) => null_array_bind_type(elem_type),
        FieldType::Numeric
        | FieldType::Text
        | FieldType::Citext
        | FieldType::Uuid
        | FieldType::Timestamp
        | FieldType::Timestamptz
        | FieldType::Date
        | FieldType::Inet
        | FieldType::Cidr
        | FieldType::Enum(_)
        | FieldType::Custom(_)
        | FieldType::Range(_) => BindType::Text,
    }
}

fn null_array_bind_type(elem_type: ElemType) -> BindType {
    match elem_type {
        ElemType::Bool => BindType::BoolArray,
        ElemType::Int => BindType::Int4Array,
        ElemType::BigInt => BindType::Int8Array,
        ElemType::Float => BindType::Float8Array,
        ElemType::Numeric
        | ElemType::Text
        | ElemType::Citext
        | ElemType::Uuid
        | ElemType::Timestamp
        | ElemType::Timestamptz
        | ElemType::Date
        | ElemType::Enum(_)
        | ElemType::Custom(_) => BindType::TextArray,
    }
}

fn typed_array_from_value(value: &Value, elem_type: ElemType) -> BindParam {
    let Value::Array(values) = value else {
        unreachable!("array value shape is validated before rendering");
    };

    match elem_type {
        ElemType::Bool => BindParam::BoolArray(values.iter().map(expect_bool).collect()),
        ElemType::Int => BindParam::Int4Array(values.iter().map(expect_int4).collect()),
        ElemType::BigInt => BindParam::Int8Array(values.iter().map(expect_i64).collect()),
        ElemType::Float => BindParam::Float8Array(values.iter().map(expect_float8).collect()),
        ElemType::Numeric
        | ElemType::Text
        | ElemType::Citext
        | ElemType::Uuid
        | ElemType::Timestamp
        | ElemType::Timestamptz
        | ElemType::Date
        | ElemType::Enum(_)
        | ElemType::Custom(_) => {
            BindParam::TextArray(values.iter().map(expect_string_like).collect())
        }
    }
}

fn array_from_values(values: &[Value]) -> BindParam {
    if let Some(values) = try_extract(values, |value| match value {
        Value::String(value) => Some(value.clone()),
        _ => None,
    }) {
        return BindParam::TextArray(values);
    }
    if let Some(values) = try_extract(values, |value| match value {
        Value::I64(value) => Some(*value),
        _ => None,
    }) {
        return BindParam::Int8Array(values);
    }
    if let Some(values) = try_extract(values, |value| match value {
        Value::F64(value) => Some(*value),
        _ => None,
    }) {
        return BindParam::Float8Array(values);
    }
    if let Some(values) = try_extract(values, |value| match value {
        Value::Bool(value) => Some(*value),
        _ => None,
    }) {
        return BindParam::BoolArray(values);
    }
    if let Some(values) = try_extract(values, |value| match value {
        Value::Bytes(value) => Some(value.clone()),
        _ => None,
    }) {
        return BindParam::BytesArray(values);
    }
    BindParam::JsonArray(values.iter().map(value_to_json).collect())
}

fn expect_string(value: &Value) -> &str {
    let Value::String(value) = value else {
        unreachable!("string value shape is validated before rendering");
    };
    value
}

fn expect_string_like(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::I64(value) => value.to_string(),
        Value::F64(value) => value.to_string(),
        _ => unreachable!("string-like value shape is validated before rendering"),
    }
}

fn expect_bool(value: &Value) -> bool {
    let Value::Bool(value) = value else {
        unreachable!("bool value shape is validated before rendering");
    };
    *value
}

fn expect_i64(value: &Value) -> i64 {
    let Value::I64(value) = value else {
        unreachable!("integer value shape is validated before rendering");
    };
    *value
}

fn expect_int4(value: &Value) -> i32 {
    let value = expect_i64(value);
    i32::try_from(value).expect("int4 value range is validated before rendering")
}

fn expect_float8(value: &Value) -> f64 {
    match value {
        Value::I64(value) => *value as f64,
        Value::F64(value) => *value,
        _ => unreachable!("number value shape is validated before rendering"),
    }
}

fn expect_bytes(value: &Value) -> &[u8] {
    let Value::Bytes(value) = value else {
        unreachable!("bytes value shape is validated before rendering");
    };
    value
}

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(value) => serde_json::Value::Bool(*value),
        Value::I64(value) => serde_json::Value::Number((*value).into()),
        Value::F64(value) => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::String(value) => serde_json::Value::String(value.clone()),
        Value::Bytes(value) => serde_json::Value::Array(
            value
                .iter()
                .map(|byte| serde_json::Value::Number((*byte).into()))
                .collect(),
        ),
        Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(value_to_json).collect())
        }
        Value::Json(value) => value.clone(),
    }
}

fn try_extract<T>(values: &[Value], f: impl Fn(&Value) -> Option<T>) -> Option<Vec<T>> {
    values.iter().map(f).collect()
}
