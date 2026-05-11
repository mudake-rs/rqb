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

/// Builds `range_merge(range_or_multirange)`.
pub fn range_merge(range: impl Into<ValueExpr>) -> ValueExpr {
    function("range_merge", [range])
}

/// Builds `multirange_merge(multirange)`.
pub fn multirange_merge(multirange: impl Into<ValueExpr>) -> ValueExpr {
    function("multirange_merge", [multirange])
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
