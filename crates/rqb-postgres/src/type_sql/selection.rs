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
        FieldType::Numeric => Some("::text"),
        FieldType::Array(elem_type) => postgres_array_selection_cast(elem_type),
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
        | FieldType::Custom(_) => None,
    }
}

fn postgres_array_selection_cast(elem_type: ElemType) -> Option<&'static str> {
    match elem_type {
        ElemType::Citext
        | ElemType::Numeric
        | ElemType::Time
        | ElemType::Timetz
        | ElemType::Interval
        | ElemType::Enum(_) => Some("::text[]"),
        ElemType::Custom(type_spec) if type_spec.selects_as_text() => Some("::text[]"),
        ElemType::Custom(_) => None,
        ElemType::Text
        | ElemType::Int
        | ElemType::BigInt
        | ElemType::Float
        | ElemType::Bool
        | ElemType::Uuid
        | ElemType::Date
        | ElemType::Timestamp
        | ElemType::Timestamptz => None,
    }
}
