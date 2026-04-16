use crate::typed::ValueExpr;

use super::function;

/// Builds `abs(expr)`.
pub fn abs(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("abs", [expr])
}

/// Builds `ceil(expr)`.
pub fn ceil(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("ceil", [expr])
}

/// Builds `floor(expr)`.
pub fn floor(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("floor", [expr])
}

/// Builds `round(...)`.
pub fn round(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("round", args)
}

/// Builds `trunc(...)`.
pub fn trunc(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("trunc", args)
}

/// Builds `power(left, right)`.
pub fn power(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    function("power", [left.into(), right.into()])
}

/// Alias for `power(left, right)`.
pub fn pow(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    power(left, right)
}

/// Builds `sqrt(expr)`.
pub fn sqrt(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("sqrt", [expr])
}

/// Builds `exp(expr)`.
pub fn exp(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("exp", [expr])
}

/// Builds `ln(expr)`.
pub fn ln(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("ln", [expr])
}

/// Builds `log(...)`.
pub fn log(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("log", args)
}

/// Builds `mod(left, right)`.
pub fn mod_(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    function("mod", [left.into(), right.into()])
}

/// Builds `random()`.
pub fn random() -> ValueExpr {
    function("random", Vec::<ValueExpr>::new())
}

/// Builds `random(min, max)` for Postgres versions that support bounds.
pub fn random_between(min: impl Into<ValueExpr>, max: impl Into<ValueExpr>) -> ValueExpr {
    function("random", [min.into(), max.into()])
}
