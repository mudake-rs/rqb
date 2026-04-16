use crate::typed::ValueExpr;

use super::function;

/// Builds `uuidv7()`.
pub fn uuidv7() -> ValueExpr {
    function("uuidv7", Vec::<ValueExpr>::new())
}

/// Builds `uuid_extract_timestamp(uuid)`.
pub fn uuid_extract_timestamp(uuid: impl Into<ValueExpr>) -> ValueExpr {
    function("uuid_extract_timestamp", [uuid])
}

/// Builds `uuid_extract_version(uuid)`.
pub fn uuid_extract_version(uuid: impl Into<ValueExpr>) -> ValueExpr {
    function("uuid_extract_version", [uuid])
}
