use crate::typed::ValueExpr;

use super::function;

pub fn coalesce(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("coalesce", args)
}

pub fn nullif(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    function("nullif", [left.into(), right.into()])
}

pub fn greatest(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("greatest", args)
}

pub fn least(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("least", args)
}
