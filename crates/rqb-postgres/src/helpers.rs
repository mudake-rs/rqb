use rqb_core::{
    ColumnOperator, ElemType, EnumType, FieldType, SelectRepr, TypeSpec, ValidatedSelect, Value,
    ValueRepr,
};

#[cfg(feature = "pool")]
pub(crate) fn quote_ident(ident: &str) -> String {
    let mut output = String::with_capacity(ident.len() + 2);
    write_quoted_ident(&mut output, ident);
    output
}

pub(crate) fn write_quoted_ident(output: &mut String, ident: &str) {
    output.push('"');
    for ch in ident.chars() {
        if ch == '"' {
            output.push('"');
        }
        output.push(ch);
    }
    output.push('"');
}

pub(crate) fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn write_enum_type(output: &mut String, enum_type: EnumType) {
    if let Some(schema) = enum_type.schema {
        write_quoted_ident(output, schema);
        output.push('.');
    }
    write_quoted_ident(output, enum_type.name);
}

fn write_type_spec(output: &mut String, type_spec: TypeSpec) {
    if let Some(schema) = type_spec.schema {
        write_quoted_ident(output, schema);
        output.push('.');
    }
    write_quoted_ident(output, type_spec.name);
}

pub(crate) fn needs_count_subquery(validated: &ValidatedSelect) -> bool {
    validated.distinct
        || !validated.distinct_on.is_empty()
        || !validated.group_by.is_empty()
        || !validated.aggregates.is_empty()
        || validated.having.is_some()
}

pub(crate) fn column_operator_sql(operator: ColumnOperator) -> &'static str {
    match operator {
        ColumnOperator::Equals => "=",
        ColumnOperator::NotEquals => "<>",
        ColumnOperator::Lt => "<",
        ColumnOperator::Lte => "<=",
        ColumnOperator::Gt => ">",
        ColumnOperator::Gte => ">=",
    }
}

pub(crate) fn write_quoted_qualified(output: &mut String, name: &str) {
    for (idx, part) in name.split('.').enumerate() {
        if idx > 0 {
            output.push('.');
        }
        write_quoted_ident(output, part);
    }
}

pub(crate) fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub(crate) fn postgres_cast(field_type: FieldType) -> Option<&'static str> {
    match field_type {
        FieldType::Uuid => Some("::text::uuid"),
        FieldType::Timestamp => Some("::text::timestamp"),
        FieldType::Timestamptz => Some("::text::timestamptz"),
        FieldType::Date => Some("::text::date"),
        FieldType::Jsonb => Some("::jsonb"),
        FieldType::Bytea => Some("::bytea"),
        FieldType::Citext => Some("::text::citext"),
        FieldType::Inet => Some("::text::inet"),
        FieldType::Cidr => Some("::text::cidr"),
        FieldType::Array(ElemType::Text) => Some("::text[]"),
        FieldType::Array(ElemType::Citext) => Some("::text[]::citext[]"),
        FieldType::Array(ElemType::Int) => Some("::bigint[]::int[]"),
        FieldType::Array(ElemType::BigInt) => Some("::bigint[]"),
        FieldType::Array(ElemType::Uuid) => Some("::text[]::uuid[]"),
        FieldType::Array(ElemType::Float) => Some("::double precision[]"),
        FieldType::Array(ElemType::Numeric) => Some("::numeric[]"),
        FieldType::Array(ElemType::Bool) => Some("::boolean[]"),
        FieldType::Array(ElemType::Timestamp) => Some("::text[]::timestamp[]"),
        FieldType::Array(ElemType::Timestamptz) => Some("::text[]::timestamptz[]"),
        FieldType::Array(ElemType::Date) => Some("::text[]::date[]"),
        FieldType::Integer => Some("::bigint::int"),
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
            output.push_str(match type_spec.value_repr {
                ValueRepr::String | ValueRepr::DecimalString => "::text::",
                ValueRepr::Native => "::",
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
        FieldType::Integer => output.push_str("::bigint[]::int[]"),
        FieldType::BigInt => output.push_str("::bigint[]"),
        FieldType::Float => output.push_str("::double precision[]"),
        FieldType::Numeric => output.push_str("::text[]::numeric[]"),
        FieldType::Bool => output.push_str("::boolean[]"),
        FieldType::Uuid => output.push_str("::text[]::uuid[]"),
        FieldType::Timestamp => output.push_str("::text[]::timestamp[]"),
        FieldType::Timestamptz => output.push_str("::text[]::timestamptz[]"),
        FieldType::Date => output.push_str("::text[]::date[]"),
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

pub(crate) fn array_field_type_for_scalar(field_type: FieldType) -> Option<FieldType> {
    match field_type {
        FieldType::Text => Some(FieldType::Array(ElemType::Text)),
        FieldType::Citext => Some(FieldType::Array(ElemType::Citext)),
        FieldType::Integer => Some(FieldType::Array(ElemType::Int)),
        FieldType::BigInt => Some(FieldType::Array(ElemType::BigInt)),
        FieldType::Float => Some(FieldType::Array(ElemType::Float)),
        FieldType::Numeric => Some(FieldType::Array(ElemType::Numeric)),
        FieldType::Bool => Some(FieldType::Array(ElemType::Bool)),
        FieldType::Uuid => Some(FieldType::Array(ElemType::Uuid)),
        FieldType::Timestamp => Some(FieldType::Array(ElemType::Timestamp)),
        FieldType::Timestamptz => Some(FieldType::Array(ElemType::Timestamptz)),
        FieldType::Date => Some(FieldType::Array(ElemType::Date)),
        FieldType::Enum(enum_type) => Some(FieldType::Array(ElemType::Enum(enum_type))),
        FieldType::Custom(type_spec) => Some(FieldType::Array(ElemType::Custom(type_spec))),
        FieldType::Jsonb
        | FieldType::Bytea
        | FieldType::Inet
        | FieldType::Cidr
        | FieldType::Range(_)
        | FieldType::Array(_) => None,
    }
}

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
        FieldType::Array(ElemType::Custom(type_spec))
            if type_spec.select_repr == SelectRepr::Text =>
        {
            Some("::text[]")
        }
        FieldType::Numeric => Some("::text"),
        FieldType::Array(ElemType::Numeric) => Some("::text[]"),
        FieldType::Array(ElemType::Timestamp) => timestamp_array_selection_cast(),
        FieldType::Array(ElemType::Timestamptz) => timestamptz_array_selection_cast(),
        FieldType::Custom(type_spec) if type_spec.select_repr == SelectRepr::Text => Some("::text"),
        _ => None,
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

pub(crate) fn array_element_field_type(field_type: FieldType) -> FieldType {
    match field_type {
        FieldType::Array(ElemType::Text) => FieldType::Text,
        FieldType::Array(ElemType::Citext) => FieldType::Citext,
        FieldType::Array(ElemType::Int) => FieldType::Integer,
        FieldType::Array(ElemType::BigInt) => FieldType::BigInt,
        FieldType::Array(ElemType::Float) => FieldType::Float,
        FieldType::Array(ElemType::Numeric) => FieldType::Numeric,
        FieldType::Array(ElemType::Bool) => FieldType::Bool,
        FieldType::Array(ElemType::Uuid) => FieldType::Uuid,
        FieldType::Array(ElemType::Timestamp) => FieldType::Timestamp,
        FieldType::Array(ElemType::Timestamptz) => FieldType::Timestamptz,
        FieldType::Array(ElemType::Date) => FieldType::Date,
        FieldType::Array(ElemType::Enum(enum_type)) => FieldType::Enum(enum_type),
        FieldType::Array(ElemType::Custom(type_spec)) => FieldType::Custom(type_spec),
        other => other,
    }
}

fn write_type_spec_array_cast(output: &mut String, type_spec: TypeSpec) {
    output.push_str(match type_spec.value_repr {
        ValueRepr::String | ValueRepr::DecimalString => "::text[]::",
        ValueRepr::Native => "::",
    });
    write_type_spec(output, type_spec);
    output.push_str("[]");
}

pub(crate) fn value_to_json(value: &Value) -> Value {
    match value {
        Value::Null => Value::Json(serde_json::Value::Null),
        Value::Bool(value) => Value::Json(serde_json::Value::Bool(*value)),
        Value::I64(value) => Value::Json(serde_json::json!(value)),
        Value::F64(value) => Value::Json(serde_json::json!(value)),
        Value::String(value) => Value::Json(serde_json::Value::String(value.clone())),
        Value::Bytes(value) => Value::Json(serde_json::Value::Array(
            value
                .iter()
                .map(|byte| serde_json::Value::Number((*byte).into()))
                .collect(),
        )),
        Value::Array(values) => Value::Json(serde_json::Value::Array(
            values
                .iter()
                .map(|value| match value_to_json(value) {
                    Value::Json(json) => json,
                    _ => unreachable!(),
                })
                .collect(),
        )),
        Value::Json(value) => Value::Json(value.clone()),
    }
}

pub(crate) fn value_to_json_array(value: &Value) -> Value {
    match value {
        Value::Json(json) if json.is_array() => Value::Json(json.clone()),
        Value::Array(_) => value_to_json(value),
        other => Value::Json(serde_json::Value::Array(vec![match value_to_json(other) {
            Value::Json(json) => json,
            _ => unreachable!(),
        }])),
    }
}
