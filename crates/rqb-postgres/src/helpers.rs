use rqb_core::{ColumnOperator, ElemType, EnumType, FieldType, ValidatedSelect, Value};

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

fn quote_enum_type(enum_type: EnumType) -> String {
    match enum_type.schema {
        Some(schema) => format!("{}.{}", quote_ident(schema), quote_ident(enum_type.name)),
        None => quote_ident(enum_type.name),
    }
}

pub(crate) fn needs_count_subquery(validated: &ValidatedSelect) -> bool {
    validated.query.distinct
        || !validated.distinct_on.is_empty()
        || !validated.group_by.is_empty()
        || !validated.aggregates.is_empty()
        || validated.query.having.is_some()
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
        FieldType::Timestamp => Some("::text::timestamptz"),
        FieldType::Date => Some("::text::date"),
        FieldType::Jsonb => Some("::jsonb"),
        FieldType::Array(ElemType::Text) => Some("::text[]"),
        FieldType::Array(ElemType::Int) => Some("::bigint[]::int[]"),
        FieldType::Array(ElemType::BigInt) => Some("::bigint[]"),
        FieldType::Array(ElemType::Uuid) => Some("::text[]::uuid[]"),
        FieldType::Array(ElemType::Float) => Some("::double precision[]"),
        FieldType::Array(ElemType::Numeric) => Some("::numeric[]"),
        FieldType::Array(ElemType::Bool) => Some("::boolean[]"),
        FieldType::Array(ElemType::Timestamp) => Some("::text[]::timestamptz[]"),
        FieldType::Array(ElemType::Date) => Some("::text[]::date[]"),
        FieldType::Integer => Some("::bigint::int"),
        FieldType::BigInt => Some("::bigint"),
        FieldType::Float => Some("::double precision"),
        FieldType::Numeric => Some("::numeric"),
        FieldType::Text
        | FieldType::Bool
        | FieldType::Enum(_)
        | FieldType::Array(ElemType::Enum(_)) => None,
    }
}

pub(crate) fn postgres_cast_sql(field_type: FieldType) -> Option<String> {
    match field_type {
        FieldType::Enum(enum_type) => Some(format!("::text::{}", quote_enum_type(enum_type))),
        FieldType::Array(ElemType::Enum(enum_type)) => {
            Some(format!("::text[]::{}[]", quote_enum_type(enum_type)))
        }
        other => postgres_cast(other).map(ToOwned::to_owned),
    }
}

pub(crate) fn postgres_selection_cast(field_type: FieldType) -> Option<&'static str> {
    match field_type {
        FieldType::Uuid => uuid_selection_cast(),
        FieldType::Timestamp | FieldType::Date => chrono_selection_cast(),
        FieldType::Enum(_) => Some("::text"),
        FieldType::Array(ElemType::Enum(_)) => Some("::text[]"),
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

pub(crate) fn array_element_field_type(field_type: FieldType) -> FieldType {
    match field_type {
        FieldType::Array(ElemType::Text) => FieldType::Text,
        FieldType::Array(ElemType::Int) => FieldType::Integer,
        FieldType::Array(ElemType::BigInt) => FieldType::BigInt,
        FieldType::Array(ElemType::Float) => FieldType::Float,
        FieldType::Array(ElemType::Numeric) => FieldType::Numeric,
        FieldType::Array(ElemType::Bool) => FieldType::Bool,
        FieldType::Array(ElemType::Uuid) => FieldType::Uuid,
        FieldType::Array(ElemType::Timestamp) => FieldType::Timestamp,
        FieldType::Array(ElemType::Date) => FieldType::Date,
        FieldType::Array(ElemType::Enum(enum_type)) => FieldType::Enum(enum_type),
        other => other,
    }
}

pub(crate) fn value_to_json(value: &Value) -> Value {
    match value {
        Value::Null => Value::Json(serde_json::Value::Null),
        Value::Bool(value) => Value::Json(serde_json::Value::Bool(*value)),
        Value::I64(value) => Value::Json(serde_json::json!(value)),
        Value::F64(value) => Value::Json(serde_json::json!(value)),
        Value::String(value) => Value::Json(serde_json::Value::String(value.clone())),
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

pub(crate) fn renumber_postgres_placeholders(sql: &str, offset: usize) -> String {
    let mut output = String::with_capacity(sql.len());
    let chars = sql.as_bytes();
    let mut idx = 0;
    while idx < chars.len() {
        if chars[idx] != b'$' {
            output.push(chars[idx] as char);
            idx += 1;
            continue;
        }

        let start = idx + 1;
        let mut end = start;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
        if end == start {
            output.push('$');
            idx += 1;
            continue;
        }
        let number = sql[start..end].parse::<usize>().unwrap_or(0);
        output.push('$');
        output.push_str(&(number + offset).to_string());
        idx = end;
    }
    output
}
