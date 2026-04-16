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

/// Builds `unnest(expr)`.
pub fn unnest(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("unnest", [expr])
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

/// Builds `array_to_string(expr, delimiter)`.
pub fn array_to_string(expr: impl Into<ValueExpr>, delimiter: impl Into<ValueExpr>) -> ValueExpr {
    function("array_to_string", [expr.into(), delimiter.into()])
}

/// Builds `string_to_array(expr, delimiter)`.
pub fn string_to_array(expr: impl Into<ValueExpr>, delimiter: impl Into<ValueExpr>) -> ValueExpr {
    function("string_to_array", [expr.into(), delimiter.into()])
}
