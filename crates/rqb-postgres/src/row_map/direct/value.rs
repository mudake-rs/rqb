use serde::de::{DeserializeSeed, IntoDeserializer, SeqAccess, Visitor};
use serde::{Deserializer, forward_to_deserialize_any};
use serde_json::Error as JsonError;
use serde_json::Value as JsonValue;

use super::DeResult;

pub(super) enum DecodedValue {
    Null,
    Bool(bool),
    I32(i32),
    I64(i64),
    F64(f64),
    String(String),
    Json(JsonValue),
    Bytes(Vec<u8>),
    Array(DecodedArray),
}

pub(super) enum DecodedArray {
    Bool(Vec<bool>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F64(Vec<f64>),
    String(Vec<String>),
    Json(Vec<JsonValue>),
    Bytes(Vec<Vec<u8>>),
}

trait IntoDecodedValue {
    fn into_decoded_value(self) -> DecodedValue;
}

impl IntoDecodedValue for bool {
    fn into_decoded_value(self) -> DecodedValue {
        DecodedValue::Bool(self)
    }
}

impl IntoDecodedValue for u8 {
    fn into_decoded_value(self) -> DecodedValue {
        DecodedValue::I64(self.into())
    }
}

impl IntoDecodedValue for i32 {
    fn into_decoded_value(self) -> DecodedValue {
        DecodedValue::I32(self)
    }
}

impl IntoDecodedValue for i64 {
    fn into_decoded_value(self) -> DecodedValue {
        DecodedValue::I64(self)
    }
}

impl IntoDecodedValue for f64 {
    fn into_decoded_value(self) -> DecodedValue {
        DecodedValue::F64(self)
    }
}

impl IntoDecodedValue for String {
    fn into_decoded_value(self) -> DecodedValue {
        DecodedValue::String(self)
    }
}

impl IntoDecodedValue for JsonValue {
    fn into_decoded_value(self) -> DecodedValue {
        DecodedValue::Json(self)
    }
}

impl IntoDecodedValue for Vec<u8> {
    fn into_decoded_value(self) -> DecodedValue {
        DecodedValue::Bytes(self)
    }
}

impl<'de> Deserializer<'de> for DecodedValue {
    type Error = JsonError;

    fn deserialize_any<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Null => visitor.visit_unit(),
            Self::Bool(value) => visitor.visit_bool(value),
            Self::I32(value) => visitor.visit_i32(value),
            Self::I64(value) => visitor.visit_i64(value),
            Self::F64(value) => visitor.visit_f64(value),
            Self::String(value) => visitor.visit_string(value),
            Self::Json(value) => value.deserialize_any(visitor),
            Self::Bytes(value) => visit_byte_seq(value, visitor),
            Self::Array(values) => values.deserialize_seq(visitor),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Null => visitor.visit_none(),
            Self::Json(JsonValue::Null) => visitor.visit_none(),
            other => visitor.visit_some(other),
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::String(value) => value
                .into_deserializer()
                .deserialize_enum(name, variants, visitor),
            Self::Json(value) => value.deserialize_enum(name, variants, visitor),
            other => other.deserialize_any(visitor),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Bytes(value) => visit_byte_seq(value, visitor),
            Self::Array(values) => values.deserialize_seq(visitor),
            Self::Json(value) => value.deserialize_seq(visitor),
            other => other.deserialize_any(visitor),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Bytes(value) => visitor.visit_byte_buf(value),
            Self::Json(value) => value.deserialize_bytes(visitor),
            other => other.deserialize_any(visitor),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
        unit unit_struct map struct identifier ignored_any
    }
}

impl DecodedArray {
    fn deserialize_seq<'de, V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Bool(values) => visit_owned_seq(values, visitor),
            Self::I32(values) => visit_owned_seq(values, visitor),
            Self::I64(values) => visit_owned_seq(values, visitor),
            Self::F64(values) => visit_owned_seq(values, visitor),
            Self::String(values) => visit_owned_seq(values, visitor),
            Self::Json(values) => visit_owned_seq(values, visitor),
            Self::Bytes(values) => visit_owned_seq(values, visitor),
        }
    }
}

struct OwnedSeq<I> {
    iter: I,
}

impl<'de, I> SeqAccess<'de> for OwnedSeq<I>
where
    I: Iterator + ExactSizeIterator,
    I::Item: IntoDecodedValue,
{
    type Error = JsonError;

    fn next_element_seed<T>(&mut self, seed: T) -> DeResult<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        self.iter
            .next()
            .map(|value| seed.deserialize(value.into_decoded_value()))
            .transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

fn visit_owned_seq<'de, T, V>(values: Vec<T>, visitor: V) -> DeResult<V::Value>
where
    T: IntoDecodedValue,
    V: Visitor<'de>,
{
    visitor.visit_seq(OwnedSeq {
        iter: values.into_iter(),
    })
}

fn visit_byte_seq<'de, V>(values: Vec<u8>, visitor: V) -> DeResult<V::Value>
where
    V: Visitor<'de>,
{
    visit_owned_seq(values, visitor)
}
