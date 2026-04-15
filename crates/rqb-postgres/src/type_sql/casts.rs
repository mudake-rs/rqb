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
        FieldType::Array(elem_type) => postgres_builtin_array_cast(elem_type, false),
        FieldType::Integer => Some("::int"),
        FieldType::BigInt => Some("::bigint"),
        FieldType::Float => Some("::double precision"),
        FieldType::Numeric => Some("::numeric"),
        FieldType::Text
        | FieldType::Bool
        | FieldType::Custom(_)
        | FieldType::Enum(_)
        | FieldType::Range(_) => None,
    }
}

fn postgres_builtin_array_cast(
    elem_type: ElemType,
    numeric_text_cast: bool,
) -> Option<&'static str> {
    match elem_type {
        ElemType::Text => Some("::text[]"),
        ElemType::Citext => Some("::text[]::citext[]"),
        ElemType::Int => Some("::int[]"),
        ElemType::BigInt => Some("::bigint[]"),
        ElemType::Uuid => Some("::text[]::uuid[]"),
        ElemType::Float => Some("::double precision[]"),
        ElemType::Numeric if numeric_text_cast => Some("::text[]::numeric[]"),
        ElemType::Numeric => Some("::numeric[]"),
        ElemType::Bool => Some("::boolean[]"),
        ElemType::Timestamp => Some("::text[]::timestamp[]"),
        ElemType::Timestamptz => Some("::text[]::timestamptz[]"),
        ElemType::Date => Some("::text[]::date[]"),
        ElemType::Time => Some("::text[]::time[]"),
        ElemType::Timetz => Some("::text[]::timetz[]"),
        ElemType::Interval => Some("::text[]::interval[]"),
        ElemType::Enum(_) | ElemType::Custom(_) => None,
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
        other => {
            let Some(FieldType::Array(elem_type)) = other.array_type_for_scalar() else {
                return false;
            };
            let Some(cast) = postgres_builtin_array_cast(elem_type, true) else {
                return false;
            };
            output.push_str(cast);
        }
    }
    true
}
