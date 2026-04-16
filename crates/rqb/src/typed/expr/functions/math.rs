use crate::typed::ValueExpr;

use super::function;

pub fn abs(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("abs", [expr])
}

pub fn ceil(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("ceil", [expr])
}

pub fn floor(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("floor", [expr])
}

pub fn round(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("round", args)
}

pub fn trunc(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("trunc", args)
}

pub fn power(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    function("power", [left.into(), right.into()])
}

pub fn pow(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    power(left, right)
}

pub fn sqrt(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("sqrt", [expr])
}

pub fn exp(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("exp", [expr])
}

pub fn ln(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("ln", [expr])
}

pub fn log(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("log", args)
}

pub fn mod_(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    function("mod", [left.into(), right.into()])
}

pub fn random() -> ValueExpr {
    function("random", Vec::<ValueExpr>::new())
}

pub fn random_between(min: impl Into<ValueExpr>, max: impl Into<ValueExpr>) -> ValueExpr {
    function("random", [min.into(), max.into()])
}
