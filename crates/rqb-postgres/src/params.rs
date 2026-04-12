use rqb_core::Value;
use tokio_postgres::types::ToSql;

pub struct PgParams {
    inner: Vec<PgParam>,
}

impl PgParams {
    pub fn from_values(values: &[Value]) -> Self {
        Self {
            inner: values.iter().map(convert_param).collect(),
        }
    }

    pub fn as_refs(&self) -> Vec<&(dyn ToSql + Sync)> {
        self.inner.iter().map(PgParam::as_ref).collect()
    }
}

enum PgParam {
    Null(Option<String>),
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Json(serde_json::Value),
    BoolVec(Vec<bool>),
    I64Vec(Vec<i64>),
    F64Vec(Vec<f64>),
    StringVec(Vec<String>),
    JsonVec(Vec<serde_json::Value>),
}

impl PgParam {
    fn as_ref(&self) -> &(dyn ToSql + Sync) {
        match self {
            Self::Null(value) => value,
            Self::Bool(value) => value,
            Self::I64(value) => value,
            Self::F64(value) => value,
            Self::String(value) => value,
            Self::Json(value) => value,
            Self::BoolVec(value) => value,
            Self::I64Vec(value) => value,
            Self::F64Vec(value) => value,
            Self::StringVec(value) => value,
            Self::JsonVec(value) => value,
        }
    }
}

fn convert_param(value: &Value) -> PgParam {
    match value {
        Value::Null => PgParam::Null(None),
        Value::Bool(value) => PgParam::Bool(*value),
        Value::I64(value) => PgParam::I64(*value),
        Value::F64(value) => PgParam::F64(*value),
        Value::String(value) => PgParam::String(value.clone()),
        Value::Json(value) => PgParam::Json(value.clone()),
        Value::Array(values) => convert_array_param(values),
    }
}

fn convert_array_param(values: &[Value]) -> PgParam {
    if let Some(values) = try_extract(values, |value| match value {
        Value::String(value) => Some(value.clone()),
        _ => None,
    }) {
        return PgParam::StringVec(values);
    }
    if let Some(values) = try_extract(values, |value| match value {
        Value::I64(value) => Some(*value),
        _ => None,
    }) {
        return PgParam::I64Vec(values);
    }
    if let Some(values) = try_extract(values, |value| match value {
        Value::F64(value) => Some(*value),
        _ => None,
    }) {
        return PgParam::F64Vec(values);
    }
    if let Some(values) = try_extract(values, |value| match value {
        Value::Bool(value) => Some(*value),
        _ => None,
    }) {
        return PgParam::BoolVec(values);
    }
    PgParam::JsonVec(
        values
            .iter()
            .map(|value| match value {
                Value::Json(value) => value.clone(),
                other => param_value_to_json(other),
            })
            .collect(),
    )
}

fn param_value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(value) => serde_json::Value::Bool(*value),
        Value::I64(value) => serde_json::Value::Number((*value).into()),
        Value::F64(value) => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::String(value) => serde_json::Value::String(value.clone()),
        Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(param_value_to_json).collect())
        }
        Value::Json(value) => value.clone(),
    }
}

fn try_extract<T>(values: &[Value], f: impl Fn(&Value) -> Option<T>) -> Option<Vec<T>> {
    values.iter().map(f).collect()
}
