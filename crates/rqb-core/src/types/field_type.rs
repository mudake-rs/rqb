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
    #[inline]
    pub fn is_jsonb(self) -> bool {
        matches!(self, Self::Jsonb)
            || matches!(self, Self::Custom(type_spec) if type_spec.family == TypeFamily::Jsonb)
    }

    #[inline]
    pub fn is_array(self) -> bool {
        matches!(self, Self::Array(_))
    }

    #[inline]
    pub fn is_range(self) -> bool {
        matches!(self, Self::Range(_))
            || matches!(self, Self::Custom(type_spec) if type_spec.family == TypeFamily::Range)
    }

    #[inline]
    pub fn is_network(self) -> bool {
        matches!(self, Self::Inet | Self::Cidr)
            || matches!(self, Self::Custom(type_spec) if type_spec.family == TypeFamily::Network)
    }

    #[inline]
    pub fn is_numeric(self) -> bool {
        matches!(
            self,
            Self::Integer | Self::BigInt | Self::Float | Self::Numeric
        ) || matches!(self, Self::Custom(type_spec) if type_spec.family == TypeFamily::Numeric)
    }

    #[inline]
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

    #[inline]
    pub fn is_text(self) -> bool {
        matches!(self, Self::Text | Self::Citext)
            || matches!(self, Self::Custom(type_spec) if type_spec.family == TypeFamily::Text)
    }

    pub fn array_type_for_scalar(self) -> Option<Self> {
        match self {
            Self::Text => Some(Self::Array(ElemType::Text)),
            Self::Citext => Some(Self::Array(ElemType::Citext)),
            Self::Integer => Some(Self::Array(ElemType::Int)),
            Self::BigInt => Some(Self::Array(ElemType::BigInt)),
            Self::Float => Some(Self::Array(ElemType::Float)),
            Self::Numeric => Some(Self::Array(ElemType::Numeric)),
            Self::Bool => Some(Self::Array(ElemType::Bool)),
            Self::Uuid => Some(Self::Array(ElemType::Uuid)),
            Self::Timestamp => Some(Self::Array(ElemType::Timestamp)),
            Self::Timestamptz => Some(Self::Array(ElemType::Timestamptz)),
            Self::Date => Some(Self::Array(ElemType::Date)),
            Self::Enum(enum_type) => Some(Self::Array(ElemType::Enum(enum_type))),
            Self::Custom(type_spec) => Some(Self::Array(ElemType::Custom(type_spec))),
            Self::Jsonb
            | Self::Bytea
            | Self::Inet
            | Self::Cidr
            | Self::Range(_)
            | Self::Array(_) => None,
        }
    }

    pub fn array_element_type(self) -> Self {
        match self {
            Self::Array(ElemType::Text) => Self::Text,
            Self::Array(ElemType::Citext) => Self::Citext,
            Self::Array(ElemType::Int) => Self::Integer,
            Self::Array(ElemType::BigInt) => Self::BigInt,
            Self::Array(ElemType::Float) => Self::Float,
            Self::Array(ElemType::Numeric) => Self::Numeric,
            Self::Array(ElemType::Bool) => Self::Bool,
            Self::Array(ElemType::Uuid) => Self::Uuid,
            Self::Array(ElemType::Timestamp) => Self::Timestamp,
            Self::Array(ElemType::Timestamptz) => Self::Timestamptz,
            Self::Array(ElemType::Date) => Self::Date,
            Self::Array(ElemType::Enum(enum_type)) => Self::Enum(enum_type),
            Self::Array(ElemType::Custom(type_spec)) => Self::Custom(type_spec),
            other => other,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    const MONEY: TypeSpec = TypeSpec::domain(Some("public"), "money_256")
        .base(TypeFamily::Numeric)
        .value_repr(super::super::ValueRepr::DecimalString);

    #[test]
    fn scalar_field_types_report_matching_array_types() {
        assert_eq!(
            FieldType::Text.array_type_for_scalar(),
            Some(FieldType::Array(ElemType::Text))
        );
        assert_eq!(
            FieldType::Enum(crate::EnumType::new(Some("public"), "status", &["active"]))
                .array_type_for_scalar(),
            Some(FieldType::Array(ElemType::Enum(crate::EnumType::new(
                Some("public"),
                "status",
                &["active"]
            ))))
        );
        assert_eq!(
            FieldType::Custom(&MONEY).array_type_for_scalar(),
            Some(FieldType::Array(ElemType::Custom(&MONEY)))
        );
        assert_eq!(FieldType::Jsonb.array_type_for_scalar(), None);
        assert_eq!(
            FieldType::Array(ElemType::Text).array_type_for_scalar(),
            None
        );
    }

    #[test]
    fn array_field_types_report_scalar_element_types() {
        assert_eq!(
            FieldType::Array(ElemType::BigInt).array_element_type(),
            FieldType::BigInt
        );
        assert_eq!(
            FieldType::Array(ElemType::Custom(&MONEY)).array_element_type(),
            FieldType::Custom(&MONEY)
        );
        assert_eq!(FieldType::Jsonb.array_element_type(), FieldType::Jsonb);
    }
}
