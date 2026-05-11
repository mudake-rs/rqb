use crate::ValueExpr;

use super::function;

/// Builds `gen_random_uuid()`.
pub fn gen_random_uuid() -> ValueExpr {
    function("gen_random_uuid", Vec::<ValueExpr>::new())
}

/// Builds `uuidv4()`.
pub fn uuidv4() -> ValueExpr {
    function("uuidv4", Vec::<ValueExpr>::new())
}

/// Builds `uuidv7()`.
pub fn uuidv7() -> ValueExpr {
    function("uuidv7", Vec::<ValueExpr>::new())
}

/// Builds `uuidv7(shift)`.
pub fn uuidv7_shift(shift: impl Into<ValueExpr>) -> ValueExpr {
    function("uuidv7", [shift])
}

/// Builds `uuid_extract_timestamp(uuid)`.
pub fn uuid_extract_timestamp(uuid: impl Into<ValueExpr>) -> ValueExpr {
    function("uuid_extract_timestamp", [uuid])
}

/// Builds `uuid_extract_version(uuid)`.
pub fn uuid_extract_version(uuid: impl Into<ValueExpr>) -> ValueExpr {
    function("uuid_extract_version", [uuid])
}
