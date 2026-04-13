use std::borrow::Cow;
use std::fmt;

use serde::Serialize;

use crate::value::Value;

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
pub struct EnumType {
    pub schema: Option<&'static str>,
    pub name: &'static str,
    pub variants: &'static [&'static str],
}

impl EnumType {
    pub const fn new(
        schema: Option<&'static str>,
        name: &'static str,
        variants: &'static [&'static str],
    ) -> Self {
        Self {
            schema,
            name,
            variants,
        }
    }

    pub fn contains(self, value: &str) -> bool {
        self.variants.contains(&value)
    }

    pub fn display_name(self) -> String {
        match self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.to_owned(),
        }
    }

    pub fn allowed_values(self) -> String {
        self.variants.join(", ")
    }
}

pub trait DbEnum: Copy {
    const TYPE: EnumType;

    fn as_db_str(self) -> &'static str;
}

impl<T> From<T> for Value
where
    T: DbEnum,
{
    fn from(value: T) -> Self {
        Self::String(value.as_db_str().to_owned())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TypeFamily {
    Text,
    Numeric,
    Bool,
    Uuid,
    Timestamp,
    Timestamptz,
    Date,
    Jsonb,
    Bytes,
    Network,
    Range,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ValueRepr {
    Native,
    String,
    DecimalString,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectRepr {
    Native,
    Text,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeSpec {
    pub schema: Option<&'static str>,
    pub name: &'static str,
    pub family: TypeFamily,
    pub value_repr: ValueRepr,
    pub select_repr: SelectRepr,
}

impl TypeSpec {
    pub const fn domain(schema: Option<&'static str>, name: &'static str) -> Self {
        Self {
            schema,
            name,
            family: TypeFamily::Text,
            value_repr: ValueRepr::String,
            select_repr: SelectRepr::Text,
        }
    }

    pub const fn base(mut self, family: TypeFamily) -> Self {
        self.family = family;
        self
    }

    pub const fn value_repr(mut self, value_repr: ValueRepr) -> Self {
        self.value_repr = value_repr;
        self
    }

    pub const fn select_repr(mut self, select_repr: SelectRepr) -> Self {
        self.select_repr = select_repr;
        self
    }

    pub fn display_name(self) -> String {
        match self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.to_owned(),
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
