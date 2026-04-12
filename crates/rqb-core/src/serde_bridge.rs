use serde::Serialize;

use crate::dataset::Dataset;
use crate::error::{Error, Result};
use crate::field::Field;
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
                    dataset: dataset.api_name.clone(),
                    field: key.clone(),
                })?;
            Ok((field, json_to_value(value.clone())))
        })
        .collect()
}

fn json_to_value(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(value) => Value::Bool(value),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Value::I64(value)
            } else if let Some(value) = value.as_u64() {
                i64::try_from(value).map_or_else(
                    |_| {
                        #[allow(clippy::cast_precision_loss)]
                        {
                            Value::F64(value as f64)
                        }
                    },
                    Value::I64,
                )
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
}
