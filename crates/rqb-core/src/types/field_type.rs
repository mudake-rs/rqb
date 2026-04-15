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
    Time,
    Timetz,
    Interval,
    Enum(EnumType),
    Custom(&'static TypeSpec),
}

impl ElemType {
    pub fn field_type(self) -> FieldType {
        match self {
            Self::Text => FieldType::Text,
            Self::Citext => FieldType::Citext,
            Self::Int => FieldType::Integer,
            Self::BigInt => FieldType::BigInt,
            Self::Float => FieldType::Float,
            Self::Numeric => FieldType::Numeric,
            Self::Bool => FieldType::Bool,
            Self::Uuid => FieldType::Uuid,
            Self::Timestamp => FieldType::Timestamp,
            Self::Timestamptz => FieldType::Timestamptz,
            Self::Date => FieldType::Date,
            Self::Time => FieldType::Time,
            Self::Timetz => FieldType::Timetz,
            Self::Interval => FieldType::Interval,
            Self::Enum(enum_type) => FieldType::Enum(enum_type),
            Self::Custom(type_spec) => FieldType::Custom(type_spec),
        }
    }

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
            Self::Time => "time",
            Self::Timetz => "timetz",
            Self::Interval => "interval",
            Self::Enum(enum_type) => enum_type.name,
            Self::Custom(type_spec) => type_spec.name,
        }
    }

    pub fn display_name(self) -> Cow<'static, str> {
        match self {
            Self::Enum(enum_type) => Cow::Owned(enum_type.display_name()),
            Self::Custom(type_spec) => Cow::Owned(type_spec.display_name()),
            other => Cow::Borrowed(other.as_str()),
        }
    }

    fn array_display_name(self) -> Cow<'static, str> {
        match self {
            Self::Enum(enum_type) => Cow::Owned(format!("{}[]", enum_type.display_name())),
            Self::Custom(type_spec) => Cow::Owned(format!("{}[]", type_spec.display_name())),
            other => Cow::Borrowed(other.array_type_name()),
        }
    }

    fn array_type_name(self) -> &'static str {
        match self {
            Self::Text => "text[]",
            Self::Citext => "citext[]",
            Self::Int => "int[]",
            Self::BigInt => "bigint[]",
            Self::Float => "float[]",
            Self::Numeric => "numeric[]",
            Self::Bool => "bool[]",
            Self::Uuid => "uuid[]",
            Self::Timestamp => "timestamp[]",
            Self::Timestamptz => "timestamptz[]",
            Self::Date => "date[]",
            Self::Time => "time[]",
            Self::Timetz => "timetz[]",
            Self::Interval => "interval[]",
            Self::Enum(_) => "enum[]",
            Self::Custom(_) => "custom[]",
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
    Time,
    Timetz,
    Interval,
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
        matches!(
            self,
            Self::Timestamp
                | Self::Timestamptz
                | Self::Date
                | Self::Time
                | Self::Timetz
                | Self::Interval
        ) || matches!(
            self,
            Self::Custom(type_spec)
                if matches!(
                    type_spec.family,
                    TypeFamily::Timestamp
                        | TypeFamily::Timestamptz
                        | TypeFamily::Date
                        | TypeFamily::Time
                        | TypeFamily::Timetz
                        | TypeFamily::Interval
                )
        )
    }

    #[inline]
    pub fn is_text(self) -> bool {
        matches!(self, Self::Text | Self::Citext)
            || matches!(self, Self::Custom(type_spec) if type_spec.family == TypeFamily::Text)
    }

    pub fn array_type_for_scalar(self) -> Option<Self> {
        self.elem_type_for_scalar().map(Self::Array)
    }

    fn elem_type_for_scalar(self) -> Option<ElemType> {
        match self {
            Self::Text => Some(ElemType::Text),
            Self::Citext => Some(ElemType::Citext),
            Self::Integer => Some(ElemType::Int),
            Self::BigInt => Some(ElemType::BigInt),
            Self::Float => Some(ElemType::Float),
            Self::Numeric => Some(ElemType::Numeric),
            Self::Bool => Some(ElemType::Bool),
            Self::Uuid => Some(ElemType::Uuid),
            Self::Timestamp => Some(ElemType::Timestamp),
            Self::Timestamptz => Some(ElemType::Timestamptz),
            Self::Date => Some(ElemType::Date),
            Self::Time => Some(ElemType::Time),
            Self::Timetz => Some(ElemType::Timetz),
            Self::Interval => Some(ElemType::Interval),
            Self::Enum(enum_type) => Some(ElemType::Enum(enum_type)),
            Self::Custom(type_spec) => Some(ElemType::Custom(type_spec)),
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
            Self::Array(elem_type) => elem_type.field_type(),
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
            Self::Time => "time",
            Self::Timetz => "timetz",
            Self::Interval => "interval",
            Self::Jsonb => "jsonb",
            Self::Bytea => "bytea",
            Self::Inet => "inet",
            Self::Cidr => "cidr",
            Self::Enum(enum_type) => enum_type.name,
            Self::Custom(type_spec) => type_spec.name,
            Self::Range(elem) => range_type_name(elem),
            Self::Array(elem) => elem.array_type_name(),
        }
    }

    pub fn display_name(self) -> Cow<'static, str> {
        match self {
            Self::Enum(enum_type) => Cow::Owned(enum_type.display_name()),
            Self::Custom(type_spec) => Cow::Owned(type_spec.display_name()),
            Self::Array(elem_type) => elem_type.array_display_name(),
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
            FieldType::Time.array_type_for_scalar(),
            Some(FieldType::Array(ElemType::Time))
        );
        assert_eq!(
            FieldType::Timetz.array_type_for_scalar(),
            Some(FieldType::Array(ElemType::Timetz))
        );
        assert_eq!(
            FieldType::Interval.array_type_for_scalar(),
            Some(FieldType::Array(ElemType::Interval))
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
            FieldType::Array(ElemType::Interval).array_element_type(),
            FieldType::Interval
        );
        assert_eq!(
            FieldType::Array(ElemType::Custom(&MONEY)).array_element_type(),
            FieldType::Custom(&MONEY)
        );
        assert_eq!(FieldType::Jsonb.array_element_type(), FieldType::Jsonb);
    }

    #[test]
    fn array_field_types_display_as_arrays() {
        assert_eq!(FieldType::Array(ElemType::Text).display_name(), "text[]");
        assert_eq!(ElemType::Text.display_name(), "text");
        assert_eq!(
            FieldType::Array(ElemType::Custom(&MONEY)).display_name(),
            "public.money_256[]"
        );
        assert_eq!(ElemType::Custom(&MONEY).display_name(), "public.money_256");
    }
}
