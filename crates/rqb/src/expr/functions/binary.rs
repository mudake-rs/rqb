use crate::ValueExpr;

use super::function;

/// Builds `crc32(expr)`.
pub fn crc32(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("crc32", [expr])
}

/// Builds `crc32c(expr)`.
pub fn crc32c(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("crc32c", [expr])
}
