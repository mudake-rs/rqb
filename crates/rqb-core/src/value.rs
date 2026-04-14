use serde::{Deserialize, Deserializer, Serialize};

// Custom serde keeps whole numbers exact when possible. Large unsigned JSON
// numbers become strings instead of silently passing through f64.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Json(serde_json::Value),
}

macro_rules! impl_value_from {
    ($variant:ident: $($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for Value {
                fn from(value: $ty) -> Self {
                    Self::$variant(value.into())
                }
            }
        )*
    };
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_json::Value::deserialize(deserializer).map(Self::from_json_value)
    }
}

impl Value {
    fn from_json_value(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(value),
            serde_json::Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    Self::I64(value)
                } else if let Some(value) = value.as_u64() {
                    i64::try_from(value)
                        .map(Self::I64)
                        .unwrap_or_else(|_| Self::String(value.to_string()))
                } else if let Some(value) = value.as_f64() {
                    Self::F64(value)
                } else {
                    Self::String(value.to_string())
                }
            }
            serde_json::Value::String(value) => Self::String(value),
            serde_json::Value::Array(values) => {
                Self::Array(values.into_iter().map(Self::from_json_value).collect())
            }
            serde_json::Value::Object(map) => Self::Json(serde_json::Value::Object(map)),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            Self::Bool(_) | Self::I64(_) | Self::F64(_) | Self::String(_) | Self::Bytes(_)
        )
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Self::I64(_) | Self::F64(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    pub fn bytes(value: impl Into<Vec<u8>>) -> Self {
        Self::Bytes(value.into())
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::I64(_) => "i64",
            Self::F64(_) => "f64",
            Self::String(_) => "string",
            Self::Bytes(_) => "bytes",
            Self::Array(_) => "array",
            Self::Json(_) => "json",
        }
    }
}

impl From<()> for Value {
    fn from((): ()) -> Self {
        Self::Null
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl_value_from!(I64: i8, i16, i32, i64, u8, u16, u32);
impl_value_from!(F64: f32, f64);

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        i64::try_from(value)
            .map(Self::I64)
            .unwrap_or_else(|_| Self::String(value.to_string()))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&[u8]> for Value {
    fn from(value: &[u8]) -> Self {
        Self::Bytes(value.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for Value {
    fn from(value: &[u8; N]) -> Self {
        Self::Bytes(value.to_vec())
    }
}

#[cfg(feature = "with-uuid")]
impl From<uuid::Uuid> for Value {
    fn from(value: uuid::Uuid) -> Self {
        Self::String(value.to_string())
    }
}

#[cfg(feature = "with-uuid")]
impl From<&uuid::Uuid> for Value {
    fn from(value: &uuid::Uuid) -> Self {
        Self::String(value.to_string())
    }
}

#[cfg(feature = "with-chrono")]
impl From<chrono::NaiveDate> for Value {
    fn from(value: chrono::NaiveDate) -> Self {
        Self::String(value.to_string())
    }
}

#[cfg(feature = "with-chrono")]
impl From<&chrono::NaiveDate> for Value {
    fn from(value: &chrono::NaiveDate) -> Self {
        Self::String(value.to_string())
    }
}

#[cfg(feature = "with-chrono")]
impl From<chrono::NaiveDateTime> for Value {
    fn from(value: chrono::NaiveDateTime) -> Self {
        Self::String(value.to_string())
    }
}

#[cfg(feature = "with-chrono")]
impl From<&chrono::NaiveDateTime> for Value {
    fn from(value: &chrono::NaiveDateTime) -> Self {
        Self::String(value.to_string())
    }
}

#[cfg(feature = "with-chrono")]
impl<Tz> From<chrono::DateTime<Tz>> for Value
where
    Tz: chrono::TimeZone,
    Tz::Offset: std::fmt::Display,
{
    fn from(value: chrono::DateTime<Tz>) -> Self {
        Self::String(value.to_rfc3339())
    }
}

#[cfg(feature = "with-chrono")]
impl<Tz> From<&chrono::DateTime<Tz>> for Value
where
    Tz: chrono::TimeZone,
    Tz::Offset: std::fmt::Display,
{
    fn from(value: &chrono::DateTime<Tz>) -> Self {
        Self::String(value.to_rfc3339())
    }
}

impl<T> From<Vec<T>> for Value
where
    T: Into<Value>,
{
    fn from(values: Vec<T>) -> Self {
        Self::Array(values.into_iter().map(Into::into).collect())
    }
}

impl<T, const N: usize> From<[T; N]> for Value
where
    T: Into<Value>,
{
    fn from(values: [T; N]) -> Self {
        Self::Array(values.into_iter().map(Into::into).collect())
    }
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Self {
        Self::Json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn serde_keeps_whole_numbers_as_i64_before_f64() {
        let value =
            serde_json::from_value::<Value>(serde_json::json!(9_007_199_254_740_993_i64)).unwrap();

        assert_eq!(value, Value::I64(9_007_199_254_740_993));
    }

    #[test]
    fn serde_keeps_large_unsigned_numbers_lossless_as_strings() {
        let value = serde_json::from_value::<Value>(serde_json::json!(u64::MAX)).unwrap();

        assert_eq!(value, Value::String(u64::MAX.to_string()));
    }

    #[test]
    fn u64_conversion_preserves_large_values_without_f64() {
        assert_eq!(Value::from(42_u64), Value::I64(42));
        assert_eq!(Value::from(u64::MAX), Value::String(u64::MAX.to_string()));
    }

    #[test]
    fn serde_maps_arrays_recursively_and_objects_to_json() {
        let value =
            serde_json::from_value::<Value>(serde_json::json!([1, "x", { "nested": true }]))
                .unwrap();

        assert_eq!(
            value,
            Value::Array(vec![
                Value::I64(1),
                Value::String("x".to_owned()),
                Value::Json(serde_json::json!({ "nested": true })),
            ])
        );
    }

    #[test]
    fn value_type_helpers_describe_runtime_shape() {
        assert!(Value::Null.is_null());
        assert!(Value::Bool(true).is_scalar());
        assert!(Value::Bytes(vec![1, 2, 3]).is_scalar());
        assert!(Value::I64(1).is_number());
        assert!(Value::Array(vec![]).is_array());
        assert_eq!(Value::bytes([1, 2, 3]), Value::Bytes(vec![1, 2, 3]));
        assert_eq!(Value::Json(serde_json::json!({})).type_name(), "json");
    }

    #[cfg(feature = "with-uuid")]
    #[test]
    fn uuid_values_convert_to_strings() {
        let id = uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap();

        assert_eq!(Value::from(id), Value::String(id.to_string()));
        assert_eq!(Value::from(&id), Value::String(id.to_string()));
    }

    #[cfg(feature = "with-chrono")]
    #[test]
    fn chrono_values_convert_to_wire_strings() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 4, 12).unwrap();
        let local_timestamp = date.and_hms_opt(10, 30, 0).unwrap();
        let timestamp = chrono::DateTime::parse_from_rfc3339("2026-04-12T10:30:00Z").unwrap();

        assert_eq!(Value::from(date), Value::String("2026-04-12".to_owned()));
        assert_eq!(
            Value::from(local_timestamp),
            Value::String("2026-04-12 10:30:00".to_owned())
        );
        assert_eq!(
            Value::from(timestamp),
            Value::String("2026-04-12T10:30:00+00:00".to_owned())
        );
    }
}
