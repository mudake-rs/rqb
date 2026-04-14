use rqb_core::{ElemType, FieldType};

pub(crate) fn postgres_selection_cast(field_type: FieldType) -> Option<&'static str> {
    match field_type {
        FieldType::Uuid => uuid_selection_cast(),
        FieldType::Timestamp => timestamp_selection_cast(),
        FieldType::Timestamptz => timestamptz_selection_cast(),
        FieldType::Date => chrono_selection_cast(),
        FieldType::Citext | FieldType::Inet | FieldType::Cidr | FieldType::Range(_) => {
            Some("::text")
        }
        FieldType::Enum(_) => Some("::text"),
        FieldType::Array(ElemType::Citext) => Some("::text[]"),
        FieldType::Array(ElemType::Enum(_)) => Some("::text[]"),
        FieldType::Array(ElemType::Custom(type_spec)) if type_spec.selects_as_text() => {
            Some("::text[]")
        }
        FieldType::Numeric => Some("::text"),
        FieldType::Array(ElemType::Numeric) => Some("::text[]"),
        FieldType::Array(ElemType::Timestamp) => timestamp_array_selection_cast(),
        FieldType::Array(ElemType::Timestamptz) => timestamptz_array_selection_cast(),
        FieldType::Array(ElemType::Custom(_)) => None,
        FieldType::Custom(type_spec) if type_spec.selects_as_text() => Some("::text"),
        FieldType::Text
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
        | FieldType::Custom(_) => None,
    }
}

#[cfg(feature = "with-uuid")]
fn uuid_selection_cast() -> Option<&'static str> {
    None
}

#[cfg(not(feature = "with-uuid"))]
fn uuid_selection_cast() -> Option<&'static str> {
    Some("::text")
}

#[cfg(feature = "with-chrono")]
fn chrono_selection_cast() -> Option<&'static str> {
    None
}

#[cfg(not(feature = "with-chrono"))]
fn chrono_selection_cast() -> Option<&'static str> {
    Some("::text")
}

#[cfg(feature = "with-chrono")]
fn timestamp_selection_cast() -> Option<&'static str> {
    None
}

#[cfg(not(feature = "with-chrono"))]
fn timestamp_selection_cast() -> Option<&'static str> {
    Some("::text")
}

#[cfg(feature = "with-chrono")]
fn timestamptz_selection_cast() -> Option<&'static str> {
    None
}

#[cfg(not(feature = "with-chrono"))]
fn timestamptz_selection_cast() -> Option<&'static str> {
    Some("::text")
}

#[cfg(feature = "with-chrono")]
fn timestamp_array_selection_cast() -> Option<&'static str> {
    None
}

#[cfg(not(feature = "with-chrono"))]
fn timestamp_array_selection_cast() -> Option<&'static str> {
    Some("::text[]")
}

#[cfg(feature = "with-chrono")]
fn timestamptz_array_selection_cast() -> Option<&'static str> {
    None
}

#[cfg(not(feature = "with-chrono"))]
fn timestamptz_array_selection_cast() -> Option<&'static str> {
    Some("::text[]")
}
