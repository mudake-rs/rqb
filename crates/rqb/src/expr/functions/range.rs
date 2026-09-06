use crate::{BoolExpr, ValueExpr};

use super::function;

/// Builds `lower(range)`.
pub fn range_lower(range: impl Into<ValueExpr>) -> ValueExpr {
    function("lower", [range])
}

/// Builds `upper(range)`.
pub fn range_upper(range: impl Into<ValueExpr>) -> ValueExpr {
    function("upper", [range])
}

/// Returns the smallest range containing a multirange via `range_merge(multirange)`.
/// For the two-range overload, use `function("range_merge", [left, right])`.
pub fn range_merge(multirange: impl Into<ValueExpr>) -> ValueExpr {
    function("range_merge", [multirange])
}

/// Builds `isempty(range)`.
pub fn isempty(range: impl Into<ValueExpr>) -> BoolExpr {
    function("isempty", [range]).is_true()
}

/// Builds `lower_inc(range)`.
pub fn lower_inc(range: impl Into<ValueExpr>) -> BoolExpr {
    function("lower_inc", [range]).is_true()
}

/// Builds `upper_inc(range)`.
pub fn upper_inc(range: impl Into<ValueExpr>) -> BoolExpr {
    function("upper_inc", [range]).is_true()
}

/// Builds `lower_inf(range)`.
pub fn lower_inf(range: impl Into<ValueExpr>) -> BoolExpr {
    function("lower_inf", [range]).is_true()
}

/// Builds `upper_inf(range)`.
pub fn upper_inf(range: impl Into<ValueExpr>) -> BoolExpr {
    function("upper_inf", [range]).is_true()
}
