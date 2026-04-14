use rqb_core::{ElemType, FieldType};

use super::names::{write_enum_type, write_type_spec, write_type_spec_array_cast};

fn postgres_cast(field_type: FieldType) -> Option<&'static str> {
    match field_type {
        FieldType::Uuid => Some("::text::uuid"),
        FieldType::Timestamp => Some("::text::timestamp"),
        FieldType::Timestamptz => Some("::text::timestamptz"),
        FieldType::Date => Some("::text::date"),
        FieldType::Time => Some("::text::time"),
        FieldType::Timetz => Some("::text::timetz"),
        FieldType::Interval => Some("::text::interval"),
        FieldType::Jsonb => Some("::jsonb"),
        FieldType::Bytea => Some("::bytea"),
        FieldType::Citext => Some("::text::citext"),
        FieldType::Inet => Some("::text::inet"),
        FieldType::Cidr => Some("::text::cidr"),
        FieldType::Array(ElemType::Text) => Some("::text[]"),
        FieldType::Array(ElemType::Citext) => Some("::text[]::citext[]"),
        FieldType::Array(ElemType::Int) => Some("::int[]"),
        FieldType::Array(ElemType::BigInt) => Some("::bigint[]"),
        FieldType::Array(ElemType::Uuid) => Some("::text[]::uuid[]"),
        FieldType::Array(ElemType::Float) => Some("::double precision[]"),
        FieldType::Array(ElemType::Numeric) => Some("::numeric[]"),
        FieldType::Array(ElemType::Bool) => Some("::boolean[]"),
        FieldType::Array(ElemType::Timestamp) => Some("::text[]::timestamp[]"),
        FieldType::Array(ElemType::Timestamptz) => Some("::text[]::timestamptz[]"),
        FieldType::Array(ElemType::Date) => Some("::text[]::date[]"),
        FieldType::Array(ElemType::Time) => Some("::text[]::time[]"),
        FieldType::Array(ElemType::Timetz) => Some("::text[]::timetz[]"),
        FieldType::Array(ElemType::Interval) => Some("::text[]::interval[]"),
        FieldType::Integer => Some("::int"),
        FieldType::BigInt => Some("::bigint"),
        FieldType::Float => Some("::double precision"),
        FieldType::Numeric => Some("::numeric"),
        FieldType::Text
        | FieldType::Bool
        | FieldType::Custom(_)
        | FieldType::Enum(_)
        | FieldType::Range(_)
        | FieldType::Array(ElemType::Enum(_) | ElemType::Custom(_)) => None,
    }
}

pub(crate) fn write_postgres_cast(output: &mut String, field_type: FieldType) -> bool {
    match field_type {
        FieldType::Enum(enum_type) => {
            output.push_str("::text::");
            write_enum_type(output, enum_type);
        }
        FieldType::Array(ElemType::Enum(enum_type)) => {
            output.push_str("::text[]::");
            write_enum_type(output, enum_type);
            output.push_str("[]");
        }
        FieldType::Array(ElemType::Custom(type_spec)) => {
            write_type_spec_array_cast(output, *type_spec);
        }
        FieldType::Range(elem_type) => {
            output.push_str("::text::");
            output.push_str(FieldType::Range(elem_type).as_str());
        }
        FieldType::Custom(type_spec) => {
            output.push_str(if type_spec.value_is_string_backed() {
                "::text::"
            } else {
                "::"
            });
            write_type_spec(output, *type_spec);
        }
        other => {
            let Some(cast) = postgres_cast(other) else {
                return false;
            };
            output.push_str(cast);
        }
    }
    true
}

pub(crate) fn write_postgres_array_cast_for_scalar(
    output: &mut String,
    field_type: FieldType,
) -> bool {
    match field_type {
        FieldType::Text => output.push_str("::text[]"),
        FieldType::Citext => output.push_str("::text[]::citext[]"),
        FieldType::Integer => output.push_str("::int[]"),
        FieldType::BigInt => output.push_str("::bigint[]"),
        FieldType::Float => output.push_str("::double precision[]"),
        FieldType::Numeric => output.push_str("::text[]::numeric[]"),
        FieldType::Bool => output.push_str("::boolean[]"),
        FieldType::Uuid => output.push_str("::text[]::uuid[]"),
        FieldType::Timestamp => output.push_str("::text[]::timestamp[]"),
        FieldType::Timestamptz => output.push_str("::text[]::timestamptz[]"),
        FieldType::Date => output.push_str("::text[]::date[]"),
        FieldType::Time => output.push_str("::text[]::time[]"),
        FieldType::Timetz => output.push_str("::text[]::timetz[]"),
        FieldType::Interval => output.push_str("::text[]::interval[]"),
        FieldType::Jsonb => output.push_str("::jsonb[]"),
        FieldType::Bytea => output.push_str("::bytea[]"),
        FieldType::Inet => output.push_str("::text[]::inet[]"),
        FieldType::Cidr => output.push_str("::text[]::cidr[]"),
        FieldType::Enum(enum_type) => {
            output.push_str("::text[]::");
            write_enum_type(output, enum_type);
            output.push_str("[]");
        }
        FieldType::Custom(type_spec) => write_type_spec_array_cast(output, *type_spec),
        FieldType::Range(elem_type) => {
            output.push_str("::text[]::");
            output.push_str(FieldType::Range(elem_type).as_str());
            output.push_str("[]");
        }
        FieldType::Array(_) => return false,
    }
    true
}
