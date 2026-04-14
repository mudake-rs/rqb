use rqb_core::{ElemType, FieldType, Value};

use crate::helpers::value_to_json;
use crate::type_sql::{write_postgres_array_cast_for_scalar, write_postgres_cast};
use crate::{BindParam, BindType};

use super::Renderer;

impl Renderer {
    pub(super) fn push_param(&mut self, value: &Value) {
        self.push_bind_param(BindParam::from_value(value));
    }

    pub(super) fn push_owned_param(&mut self, value: Value) {
        self.push_bind_param(BindParam::from_value(&value));
    }

    pub(super) fn push_text_param(&mut self, value: &str) {
        self.push_bind_param(BindParam::Text(value.to_owned()));
    }

    pub(super) fn push_owned_text_param(&mut self, value: String) {
        self.push_bind_param(BindParam::Text(value));
    }

    pub(super) fn push_bind_param(&mut self, value: BindParam) {
        self.params.push(value);
        self.sql.push('$');
        let mut buffer = itoa::Buffer::new();
        self.sql.push_str(buffer.format(self.params.len()));
    }

    pub(super) fn push_typed_param(&mut self, value: &Value, field_type: FieldType) {
        if let Value::Array(values) = value
            && values.is_empty()
            && field_type.is_array()
        {
            self.sql.push_str("ARRAY[]");
            write_postgres_cast(&mut self.sql, field_type);
            return;
        }

        match field_type {
            FieldType::Numeric => {
                self.push_numeric_param(value);
                return;
            }
            FieldType::Array(ElemType::Numeric) => {
                self.push_numeric_array_param(value);
                return;
            }
            FieldType::Array(ElemType::Custom(type_spec))
                if type_spec.value_is_decimal_string() =>
            {
                self.push_decimal_string_array_param(value, field_type);
                return;
            }
            FieldType::Custom(type_spec) if type_spec.value_is_decimal_string() => {
                self.push_decimal_string_param(value, field_type);
                return;
            }
            _ => {}
        }

        self.push_bind_param(BindParam::from_typed_value(value, field_type));
        write_postgres_cast(&mut self.sql, field_type);
    }

    pub(super) fn push_jsonb_array_values_param(&mut self, values: &[Value]) {
        self.push_bind_param(BindParam::JsonArray(
            values
                .iter()
                .map(|value| match value_to_json(value) {
                    Value::Json(json) => json,
                    _ => unreachable!(),
                })
                .collect(),
        ));
        write_postgres_array_cast_for_scalar(&mut self.sql, FieldType::Jsonb);
    }

    pub(super) fn push_scalar_array_values_param(
        &mut self,
        values: &[Value],
        field_type: FieldType,
    ) {
        if let Some(array_type) = field_type.array_type_for_scalar() {
            self.push_typed_array_values_param(values, array_type);
            return;
        }

        self.push_bind_param(BindParam::from_value(&Value::Array(values.to_vec())));
        if !write_postgres_array_cast_for_scalar(&mut self.sql, field_type) {
            self.cacheable = false;
        }
    }

    pub(super) fn push_string_array_param(&mut self, values: &[String]) {
        self.push_bind_param(BindParam::TextArray(values.to_vec()));
        write_postgres_cast(&mut self.sql, FieldType::Array(ElemType::Text));
    }

    fn push_typed_array_values_param(&mut self, values: &[Value], field_type: FieldType) {
        if values.is_empty() && field_type.is_array() {
            self.sql.push_str("ARRAY[]");
            write_postgres_cast(&mut self.sql, field_type);
            return;
        }

        match field_type {
            FieldType::Array(ElemType::Numeric) => {
                self.push_owned_param(Value::Array(
                    values.iter().map(numeric_text_value).collect(),
                ));
                self.sql.push_str("::text[]::numeric[]");
            }
            FieldType::Array(ElemType::Custom(type_spec))
                if type_spec.value_is_decimal_string() =>
            {
                self.push_owned_param(Value::Array(
                    values.iter().map(numeric_text_value).collect(),
                ));
                write_postgres_cast(&mut self.sql, field_type);
            }
            _ => {
                self.push_bind_param(BindParam::from_typed_array_values(values, field_type));
                write_postgres_cast(&mut self.sql, field_type);
            }
        }
    }

    fn push_numeric_param(&mut self, value: &Value) {
        self.push_bind_param(numeric_text_bind(value));
        self.sql.push_str("::text::numeric");
    }

    fn push_decimal_string_param(&mut self, value: &Value, field_type: FieldType) {
        self.push_bind_param(numeric_text_bind(value));
        write_postgres_cast(&mut self.sql, field_type);
    }

    fn push_numeric_array_param(&mut self, value: &Value) {
        let value = match value {
            Value::Array(values) => Value::Array(values.iter().map(numeric_text_value).collect()),
            other => numeric_text_value(other),
        };
        self.push_owned_param(value);
        self.sql.push_str("::text[]::numeric[]");
    }

    fn push_decimal_string_array_param(&mut self, value: &Value, field_type: FieldType) {
        let value = match value {
            Value::Array(values) => Value::Array(values.iter().map(numeric_text_value).collect()),
            other => numeric_text_value(other),
        };
        self.push_owned_param(value);
        write_postgres_cast(&mut self.sql, field_type);
    }
}

fn numeric_text_bind(value: &Value) -> BindParam {
    match numeric_text_value(value) {
        Value::Null => BindParam::Null(BindType::Text),
        Value::String(value) => BindParam::Text(value),
        _ => unreachable!("numeric value shape is validated before rendering"),
    }
}

fn numeric_text_value(value: &Value) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::I64(value) => Value::String(value.to_string()),
        Value::F64(value) => Value::String(numeric_f64_text(*value)),
        Value::String(value) => Value::String(value.clone()),
        other => other.clone(),
    }
}

fn numeric_f64_text(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_owned()
    } else if value == f64::INFINITY {
        "Infinity".to_owned()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_owned()
    } else {
        value.to_string()
    }
}
