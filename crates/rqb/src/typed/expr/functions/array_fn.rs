use crate::typed::ValueExpr;

use super::function;

/// Builds `cardinality(expr)`.
pub fn cardinality(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("cardinality", [expr])
}

/// Builds `array_length(expr, dimension)`.
pub fn array_length(expr: impl Into<ValueExpr>, dimension: impl Into<ValueExpr>) -> ValueExpr {
    function("array_length", [expr.into(), dimension.into()])
}

/// Builds `array_dims(expr)`.
pub fn array_dims(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("array_dims", [expr])
}

/// Builds `array_lower(expr, dimension)`.
pub fn array_lower(expr: impl Into<ValueExpr>, dimension: impl Into<ValueExpr>) -> ValueExpr {
    function("array_lower", [expr.into(), dimension.into()])
}

/// Builds `array_upper(expr, dimension)`.
pub fn array_upper(expr: impl Into<ValueExpr>, dimension: impl Into<ValueExpr>) -> ValueExpr {
    function("array_upper", [expr.into(), dimension.into()])
}

/// Builds `array_ndims(expr)`.
pub fn array_ndims(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("array_ndims", [expr])
}

/// Builds `unnest(expr)`.
pub fn unnest(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("unnest", [expr])
}

/// Builds `array_cat(left, right)`.
pub fn array_cat(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    function("array_cat", [left.into(), right.into()])
}

/// Builds `array_fill(value, dimensions)`.
pub fn array_fill(value: impl Into<ValueExpr>, dimensions: impl Into<ValueExpr>) -> ValueExpr {
    function("array_fill", [value.into(), dimensions.into()])
}

/// Builds `array_fill(value, dimensions, lower_bounds)`.
pub fn array_fill_with_lower_bounds(
    value: impl Into<ValueExpr>,
    dimensions: impl Into<ValueExpr>,
    lower_bounds: impl Into<ValueExpr>,
) -> ValueExpr {
    function(
        "array_fill",
        [value.into(), dimensions.into(), lower_bounds.into()],
    )
}

/// Builds `array_append(expr, value)`.
pub fn array_append(expr: impl Into<ValueExpr>, value: impl Into<ValueExpr>) -> ValueExpr {
    function("array_append", [expr.into(), value.into()])
}

/// Builds `array_prepend(value, expr)`.
pub fn array_prepend(value: impl Into<ValueExpr>, expr: impl Into<ValueExpr>) -> ValueExpr {
    function("array_prepend", [value.into(), expr.into()])
}

/// Builds `array_position(expr, value)`.
pub fn array_position(expr: impl Into<ValueExpr>, value: impl Into<ValueExpr>) -> ValueExpr {
    function("array_position", [expr.into(), value.into()])
}

/// Builds `array_positions(expr, value)`.
pub fn array_positions(expr: impl Into<ValueExpr>, value: impl Into<ValueExpr>) -> ValueExpr {
    function("array_positions", [expr.into(), value.into()])
}

/// Builds `array_remove(expr, value)`.
pub fn array_remove(expr: impl Into<ValueExpr>, value: impl Into<ValueExpr>) -> ValueExpr {
    function("array_remove", [expr.into(), value.into()])
}

/// Builds `array_replace(expr, from, to)`.
pub fn array_replace(
    expr: impl Into<ValueExpr>,
    from: impl Into<ValueExpr>,
    to: impl Into<ValueExpr>,
) -> ValueExpr {
    function("array_replace", [expr.into(), from.into(), to.into()])
}

/// Builds `array_reverse(expr)`.
pub fn array_reverse(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("array_reverse", [expr])
}

/// Builds `array_sample(expr, count)`.
pub fn array_sample(expr: impl Into<ValueExpr>, count: impl Into<ValueExpr>) -> ValueExpr {
    function("array_sample", [expr.into(), count.into()])
}

/// Builds `array_shuffle(expr)`.
pub fn array_shuffle(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("array_shuffle", [expr])
}

/// Builds `array_sort(expr)`.
pub fn array_sort(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("array_sort", [expr])
}

/// Builds `array_sort(expr, true, false)`.
pub fn array_sort_desc(expr: impl Into<ValueExpr>) -> ValueExpr {
    array_sort_with(expr, true, false)
}

/// Builds `array_sort(expr, descending, nulls_first)`.
pub fn array_sort_with(
    expr: impl Into<ValueExpr>,
    descending: bool,
    nulls_first: bool,
) -> ValueExpr {
    function(
        "array_sort",
        [expr.into(), descending.into(), nulls_first.into()],
    )
}

/// Builds `array_to_string(expr, delimiter)`.
pub fn array_to_string(expr: impl Into<ValueExpr>, delimiter: impl Into<ValueExpr>) -> ValueExpr {
    function("array_to_string", [expr.into(), delimiter.into()])
}

/// Builds `string_to_array(expr, delimiter)`.
pub fn string_to_array(expr: impl Into<ValueExpr>, delimiter: impl Into<ValueExpr>) -> ValueExpr {
    function("string_to_array", [expr.into(), delimiter.into()])
}
