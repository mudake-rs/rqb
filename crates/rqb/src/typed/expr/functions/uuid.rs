use crate::typed::ValueExpr;

use super::function;

pub fn uuidv7() -> ValueExpr {
    function("uuidv7", Vec::<ValueExpr>::new())
}

pub fn uuid_extract_timestamp(uuid: impl Into<ValueExpr>) -> ValueExpr {
    function("uuid_extract_timestamp", [uuid])
}

pub fn uuid_extract_version(uuid: impl Into<ValueExpr>) -> ValueExpr {
    function("uuid_extract_version", [uuid])
}
