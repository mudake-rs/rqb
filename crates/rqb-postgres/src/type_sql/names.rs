use rqb_core::{EnumType, TypeSpec};

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
