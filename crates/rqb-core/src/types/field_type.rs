use std::borrow::Cow;
use std::fmt;

use serde::Serialize;

use super::{EnumType, TypeFamily, TypeSpec};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ElemType {
    Text,
    Citext,
    Int,
    BigInt,
    Float,
    Numeric,
    Bool,
    Uuid,
    Timestamp,
    Timestamptz,
    Date,
    Enum(EnumType),
    Custom(&'static TypeSpec),
}

impl ElemType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Citext => "citext",
            Self::Int => "int",
            Self::BigInt => "bigint",
            Self::Float => "float",
            Self::Numeric => "numeric",
            Self::Bool => "bool",
            Self::Uuid => "uuid",
            Self::Timestamp => "timestamp",
            Self::Timestamptz => "timestamptz",
            Self::Date => "date",
            Self::Enum(enum_type) => enum_type.name,
            Self::Custom(type_spec) => type_spec.name,
        }
    }

    pub fn display_name(self) -> Cow<'static, str> {
        match self {
            Self::Enum(enum_type) => Cow::Owned(format!("{}[]", enum_type.display_name())),
            Self::Custom(type_spec) => Cow::Owned(format!("{}[]", type_spec.display_name())),
            other => Cow::Borrowed(other.as_str()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldType {
    Text,
    Citext,
    Integer,
    BigInt,
    Float,
    Numeric,
    Bool,
    Uuid,
    Timestamp,
    Timestamptz,
    Date,
    Jsonb,
    Bytea,
    Inet,
    Cidr,
    Enum(EnumType),
    Custom(&'static TypeSpec),
    Range(ElemType),
    Array(ElemType),
}

impl FieldType {
    pub fn is_jsonb(self) -> bool {
        matches!(self, Self::Jsonb)
            || matches!(self, Self::Custom(type_spec) if type_spec.family == TypeFamily::Jsonb)
    }

    pub fn is_array(self) -> bool {
        matches!(self, Self::Array(_))
    }

    pub fn is_range(self) -> bool {
        matches!(self, Self::Range(_))
            || matches!(self, Self::Custom(type_spec) if type_spec.family == TypeFamily::Range)
    }

    pub fn is_network(self) -> bool {
        matches!(self, Self::Inet | Self::Cidr)
            || matches!(self, Self::Custom(type_spec) if type_spec.family == TypeFamily::Network)
    }

    pub fn is_numeric(self) -> bool {
        matches!(
            self,
            Self::Integer | Self::BigInt | Self::Float | Self::Numeric
        ) || matches!(self, Self::Custom(type_spec) if type_spec.family == TypeFamily::Numeric)
    }

    pub fn is_temporal(self) -> bool {
        matches!(self, Self::Timestamp | Self::Timestamptz | Self::Date)
            || matches!(
                self,
                Self::Custom(type_spec)
                    if matches!(
                        type_spec.family,
                        TypeFamily::Timestamp | TypeFamily::Timestamptz | TypeFamily::Date
                    )
            )
    }

    pub fn is_text(self) -> bool {
        matches!(self, Self::Text | Self::Citext)
            || matches!(self, Self::Custom(type_spec) if type_spec.family == TypeFamily::Text)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Citext => "citext",
            Self::Integer => "integer",
            Self::BigInt => "bigint",
            Self::Float => "float",
            Self::Numeric => "numeric",
            Self::Bool => "bool",
            Self::Uuid => "uuid",
            Self::Timestamp => "timestamp",
            Self::Timestamptz => "timestamptz",
            Self::Date => "date",
            Self::Jsonb => "jsonb",
            Self::Bytea => "bytea",
            Self::Inet => "inet",
            Self::Cidr => "cidr",
            Self::Enum(enum_type) => enum_type.name,
            Self::Custom(type_spec) => type_spec.name,
            Self::Range(elem) => range_type_name(elem),
            Self::Array(elem) => match elem {
                ElemType::Text => "text[]",
                ElemType::Citext => "citext[]",
                ElemType::Int => "int[]",
                ElemType::BigInt => "bigint[]",
                ElemType::Float => "float[]",
                ElemType::Numeric => "numeric[]",
                ElemType::Bool => "bool[]",
                ElemType::Uuid => "uuid[]",
                ElemType::Timestamp => "timestamp[]",
                ElemType::Timestamptz => "timestamptz[]",
                ElemType::Date => "date[]",
                ElemType::Enum(_) => "enum[]",
                ElemType::Custom(_) => "custom[]",
            },
        }
    }

    pub fn display_name(self) -> Cow<'static, str> {
        match self {
            Self::Enum(enum_type) => Cow::Owned(enum_type.display_name()),
            Self::Custom(type_spec) => Cow::Owned(type_spec.display_name()),
            Self::Array(elem_type) => elem_type.display_name(),
            other => Cow::Borrowed(other.as_str()),
        }
    }

    pub fn enum_type(self) -> Option<EnumType> {
        match self {
            Self::Enum(enum_type) => Some(enum_type),
            Self::Array(ElemType::Enum(enum_type)) => Some(enum_type),
            _ => None,
        }
    }
}

pub fn range_type_name(elem: ElemType) -> &'static str {
    match elem {
        ElemType::Int => "int4range",
        ElemType::BigInt => "int8range",
        ElemType::Numeric => "numrange",
        ElemType::Timestamp => "tsrange",
        ElemType::Timestamptz => "tstzrange",
        ElemType::Date => "daterange",
        _ => "range",
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}
