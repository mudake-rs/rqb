use std::borrow::Cow;

use rqb_core::SelectColumn;
use serde::de::{DeserializeSeed, Error as _, IntoDeserializer, MapAccess, Visitor};
use serde::{Deserializer, forward_to_deserialize_any};
use serde_json::Error as JsonError;
use tokio_postgres::Row;

use crate::Result;

use self::decode::column_to_decoded;

mod decode;
mod value;

type DeResult<T> = std::result::Result<T, JsonError>;

pub fn row_to_deserialized<T>(
    row: &Row,
    columns: &[SelectColumn],
    aliases: &[Cow<'_, str>],
) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(RowDeserializer {
        row,
        columns,
        aliases,
    })
    .map_err(crate::Error::from)
}

pub fn column_aliases(columns: &[SelectColumn]) -> Vec<Cow<'_, str>> {
    columns.iter().map(column_alias).collect()
}

fn column_alias(column: &SelectColumn) -> Cow<'_, str> {
    match column {
        SelectColumn::Field(field) => {
            if let Some(alias) = &field.alias {
                Cow::Borrowed(alias.as_str())
            } else if let Some(qualifier) = &field.explicit_qualifier {
                Cow::Owned(format!("{qualifier}_{}", field.api_name))
            } else {
                Cow::Borrowed(field.api_name.as_ref())
            }
        }
        SelectColumn::Aggregate { alias, .. } | SelectColumn::Expression { alias, .. } => {
            Cow::Borrowed(alias.as_str())
        }
    }
}

struct RowDeserializer<'a> {
    row: &'a Row,
    columns: &'a [SelectColumn],
    aliases: &'a [Cow<'a, str>],
}

impl<'de> Deserializer<'de> for RowDeserializer<'_> {
    type Error = JsonError;

    fn deserialize_any<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(RowMapAccess {
            row: self.row,
            columns: self.columns,
            aliases: self.aliases,
            index: 0,
        })
    }

    fn deserialize_map<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct enum identifier
        ignored_any
    }
}

struct RowMapAccess<'a> {
    row: &'a Row,
    columns: &'a [SelectColumn],
    aliases: &'a [Cow<'a, str>],
    index: usize,
}

impl<'de> MapAccess<'de> for RowMapAccess<'_> {
    type Error = JsonError;

    fn next_key_seed<K>(&mut self, seed: K) -> DeResult<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        let Some(alias) = self.aliases.get(self.index) else {
            return Ok(None);
        };
        seed.deserialize(alias.as_ref().into_deserializer())
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> DeResult<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let index = self.index;
        self.index += 1;
        let column = self
            .columns
            .get(index)
            .ok_or_else(|| JsonError::custom("missing rqb column metadata"))?;
        let value = column_to_decoded(self.row, index, column)?;
        seed.deserialize(value)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.aliases.len().saturating_sub(self.index))
    }
}
