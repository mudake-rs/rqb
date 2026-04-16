use crate::typed::ValueExpr;

use super::function;

/// Builds `coalesce(...)`.
pub fn coalesce(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("coalesce", args)
}

/// Builds `nullif(left, right)`.
pub fn nullif(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    function("nullif", [left.into(), right.into()])
}

/// Builds `greatest(...)`.
pub fn greatest(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("greatest", args)
}

/// Builds `least(...)`.
pub fn least(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("least", args)
}
