use serde::Serialize;

use crate::error::{Error, Result};
use crate::{Field, Value};

#[doc(hidden)]
pub type __RqbWriteRecordResult<T> = Result<T>;

/// Converts a write DTO into field/value assignments for `insert(...).value`,
/// `insert(...).values`, and `update(...).set_from`.
///
/// Application code normally derives this trait with `#[derive(rqb::WriteRecord)]`.
pub trait WriteRecord {
    fn write_fields(&self) -> __RqbWriteRecordResult<Vec<(Field, Value)>>;
}

#[doc(hidden)]
pub fn __rqb_json_write_value<T>(value: &T) -> Result<Value>
where
    T: Serialize + ?Sized,
{
    serde_json::to_value(value)
        .map(Value::Json)
        .map_err(|error| Error::SerdeError {
            message: error.to_string(),
        })
}
