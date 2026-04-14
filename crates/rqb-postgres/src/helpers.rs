use rqb_core::{ColumnOperator, ValidatedSelect, Value};

#[cfg(feature = "pool")]
pub(crate) fn quote_ident(ident: &str) -> String {
    let mut output = String::with_capacity(ident.len() + 2);
    write_quoted_ident(&mut output, ident);
    output
}

pub(crate) fn write_quoted_ident(output: &mut String, ident: &str) {
    output.push('"');
    if !ident.contains('"') {
        output.push_str(ident);
    } else {
        for ch in ident.chars() {
            if ch == '"' {
                output.push('"');
            }
            output.push(ch);
        }
    }
    output.push('"');
}

pub(crate) fn write_quoted_literal(output: &mut String, value: &str) {
    output.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            output.push('\'');
        }
        output.push(ch);
    }
    output.push('\'');
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

pub(crate) fn write_escaped_like(output: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '\\' | '%' | '_' => {
                output.push('\\');
                output.push(ch);
            }
            _ => output.push(ch),
        }
    }
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
