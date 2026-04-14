use rqb_core::{ElemType, FieldType};

pub(crate) fn postgres_selection_cast(field_type: FieldType) -> Option<&'static str> {
    match field_type {
        FieldType::Citext
        | FieldType::Inet
        | FieldType::Cidr
        | FieldType::Time
        | FieldType::Timetz
        | FieldType::Interval
        | FieldType::Range(_) => Some("::text"),
        FieldType::Enum(_) => Some("::text"),
        FieldType::Array(ElemType::Citext) => Some("::text[]"),
        FieldType::Array(ElemType::Time) => Some("::text[]"),
        FieldType::Array(ElemType::Timetz) => Some("::text[]"),
        FieldType::Array(ElemType::Interval) => Some("::text[]"),
        FieldType::Array(ElemType::Enum(_)) => Some("::text[]"),
        FieldType::Array(ElemType::Custom(type_spec)) if type_spec.selects_as_text() => {
            Some("::text[]")
        }
        FieldType::Numeric => Some("::text"),
        FieldType::Array(ElemType::Numeric) => Some("::text[]"),
        FieldType::Array(ElemType::Custom(_)) => None,
        FieldType::Custom(type_spec) if type_spec.selects_as_text() => Some("::text"),
        FieldType::Text
        | FieldType::Uuid
        | FieldType::Timestamp
        | FieldType::Timestamptz
        | FieldType::Date
        | FieldType::Integer
        | FieldType::BigInt
        | FieldType::Float
        | FieldType::Bool
        | FieldType::Jsonb
        | FieldType::Bytea
        | FieldType::Array(ElemType::Text)
        | FieldType::Array(ElemType::Int)
        | FieldType::Array(ElemType::BigInt)
        | FieldType::Array(ElemType::Float)
        | FieldType::Array(ElemType::Bool)
        | FieldType::Array(ElemType::Uuid)
        | FieldType::Array(ElemType::Date)
        | FieldType::Array(ElemType::Timestamp)
        | FieldType::Array(ElemType::Timestamptz)
        | FieldType::Custom(_) => None,
    }
}
