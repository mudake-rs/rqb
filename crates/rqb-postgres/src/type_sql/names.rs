use rqb_core::{ElemType, EnumType, FieldType, TypeSpec};

use crate::helpers::write_quoted_ident;

pub(super) fn write_enum_type(output: &mut String, enum_type: EnumType) {
    if let Some(schema) = enum_type.schema {
        write_quoted_ident(output, schema);
        output.push('.');
    }
    write_quoted_ident(output, enum_type.name);
}

pub(super) fn write_type_spec(output: &mut String, type_spec: TypeSpec) {
    if let Some(schema) = type_spec.schema {
        write_quoted_ident(output, schema);
        output.push('.');
    }
    write_quoted_ident(output, type_spec.name);
}

pub(super) fn write_type_spec_array_cast(output: &mut String, type_spec: TypeSpec) {
    output.push_str(if type_spec.value_is_string_backed() {
        "::text[]::"
    } else {
        "::"
    });
    write_type_spec(output, type_spec);
    output.push_str("[]");
}

pub(crate) fn write_postgres_type_name(output: &mut String, field_type: FieldType) {
    match field_type {
        FieldType::Text => output.push_str("text"),
        FieldType::Citext => output.push_str("citext"),
        FieldType::Integer => output.push_str("integer"),
        FieldType::BigInt => output.push_str("bigint"),
        FieldType::Float => output.push_str("double precision"),
        FieldType::Numeric => output.push_str("numeric"),
        FieldType::Bool => output.push_str("boolean"),
        FieldType::Uuid => output.push_str("uuid"),
        FieldType::Timestamp => output.push_str("timestamp"),
        FieldType::Timestamptz => output.push_str("timestamptz"),
        FieldType::Date => output.push_str("date"),
        FieldType::Jsonb => output.push_str("jsonb"),
        FieldType::Bytea => output.push_str("bytea"),
        FieldType::Inet => output.push_str("inet"),
        FieldType::Cidr => output.push_str("cidr"),
        FieldType::Enum(enum_type) => write_enum_type(output, enum_type),
        FieldType::Custom(type_spec) => write_type_spec(output, *type_spec),
        FieldType::Range(elem_type) => output.push_str(FieldType::Range(elem_type).as_str()),
        FieldType::Array(elem_type) => write_postgres_array_type_name(output, elem_type),
    }
}

fn write_postgres_array_type_name(output: &mut String, elem_type: ElemType) {
    match elem_type {
        ElemType::Enum(enum_type) => write_enum_type(output, enum_type),
        ElemType::Custom(type_spec) => write_type_spec(output, *type_spec),
        other => output.push_str(other.as_str()),
    }
    output.push_str("[]");
}
