use serde::Serialize;

use crate::dataset::Dataset;
use crate::error::{Error, Result};
use crate::field::Field;
use crate::types::FieldType;
use crate::value::Value;

pub fn fields_from_serializable<T>(dataset: &Dataset, record: &T) -> Result<Vec<(Field, Value)>>
where
    T: Serialize + ?Sized,
{
    let json = serde_json::to_value(record).map_err(|error| Error::SerdeError {
        message: error.to_string(),
    })?;
    let object = json.as_object().ok_or_else(|| Error::ExpectedObject {
        message: format!("expected object, got {}", json_type_name(&json)),
    })?;

    object
        .iter()
        .map(|(key, value)| {
            let field = dataset
                .fields
                .iter()
                .find(|field| field.api_name == key || field.db_name == key)
                .copied()
                .ok_or_else(|| Error::UnknownField {
                    dataset: dataset.api_name.to_string(),
                    field: key.clone(),
                })?;
            Ok((field, json_to_field_value(field, value.clone())))
        })
        .collect()
}

fn json_to_field_value(field: Field, value: serde_json::Value) -> Value {
    if field.ty == FieldType::Bytea
        && let Some(bytes) = json_array_to_bytes(&value)
    {
        return Value::Bytes(bytes);
    }
    json_to_value(value)
}

fn json_array_to_bytes(value: &serde_json::Value) -> Option<Vec<u8>> {
    value.as_array().and_then(|values| {
        values
            .iter()
            .map(|value| value.as_u64().and_then(|value| u8::try_from(value).ok()))
            .collect()
    })
}

fn json_to_value(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(value) => Value::Bool(value),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Value::I64(value)
            } else if let Some(value) = value.as_u64() {
                i64::try_from(value)
                    .map(Value::I64)
                    .unwrap_or_else(|_| Value::String(value.to_string()))
            } else {
                Value::F64(value.as_f64().unwrap_or_default())
            }
        }
        serde_json::Value::String(value) => Value::String(value),
        serde_json::Value::Array(values) => {
            Value::Array(values.into_iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(map) => Value::Json(serde_json::Value::Object(map)),
    }
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dataset, Field, FieldType};
    use pretty_assertions::assert_eq;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Patch {
        email: String,
        total_cents: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        missing: Option<String>,
    }

    #[test]
    fn maps_api_and_db_names_from_serializable() {
        let dataset = Dataset::table("orders").fields([
            Field::new("email", FieldType::Text),
            Field::mapped("totalCents", "total_cents", FieldType::BigInt),
        ]);

        let fields = fields_from_serializable(
            &dataset,
            &Patch {
                email: "ada@example.com".to_owned(),
                total_cents: 42,
                missing: None,
            },
        )
        .unwrap();

        assert_eq!(fields.len(), 2);
        assert!(fields.iter().any(|(field, _)| field.api_name == "email"));
        assert!(
            fields
                .iter()
                .any(|(field, _)| field.api_name == "totalCents")
        );
    }

    #[test]
    fn maps_large_unsigned_numbers_without_precision_loss() {
        #[derive(Serialize)]
        struct Record {
            amount: u64,
        }

        let dataset = Dataset::table("payments").field(Field::new("amount", FieldType::Numeric));
        let fields = fields_from_serializable(&dataset, &Record { amount: u64::MAX }).unwrap();

        assert_eq!(fields[0].1, Value::String(u64::MAX.to_string()));
    }

    #[test]
    fn maps_byte_arrays_for_bytea_fields() {
        #[derive(Serialize)]
        struct Record {
            payload: Vec<u8>,
        }

        let dataset = Dataset::table("events").field(Field::new("payload", FieldType::Bytea));
        let fields = fields_from_serializable(
            &dataset,
            &Record {
                payload: vec![0xde, 0xad, 0xbe, 0xef],
            },
        )
        .unwrap();

        assert_eq!(fields[0].1, Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]));
    }
}
