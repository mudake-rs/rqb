use serde::{Deserialize, Serialize};

// Untagged serde is order-sensitive: keep integer before float so JSON whole numbers
// do not silently lose precision during request deserialization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
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

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            Self::Bool(_) | Self::I64(_) | Self::F64(_) | Self::String(_)
        )
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Self::I64(_) | Self::F64(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::I64(_) => "i64",
            Self::F64(_) => "f64",
            Self::String(_) => "string",
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
