use crate::typed::ValueExpr;

use super::function;

/// Builds `length(expr)`.
pub fn length(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("length", [expr])
}

/// Builds `char_length(expr)`.
pub fn char_length(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("char_length", [expr])
}

/// Builds `lower(expr)`.
pub fn lower(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("lower", [expr])
}

/// Builds `upper(expr)`.
pub fn upper(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("upper", [expr])
}

/// Builds `substring(...)`.
pub fn substring(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("substring", args)
}

/// Builds `replace(text, from, to)`.
pub fn replace(
    text: impl Into<ValueExpr>,
    from: impl Into<ValueExpr>,
    to: impl Into<ValueExpr>,
) -> ValueExpr {
    function("replace", [text.into(), from.into(), to.into()])
}

/// Builds `regexp_replace(...)`.
pub fn regexp_replace(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("regexp_replace", args)
}

/// Builds `regexp_matches(...)`.
pub fn regexp_matches(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("regexp_matches", args)
}

/// Builds `regexp_split_to_array(text, pattern)`.
pub fn regexp_split_to_array(
    text: impl Into<ValueExpr>,
    pattern: impl Into<ValueExpr>,
) -> ValueExpr {
    function("regexp_split_to_array", [text.into(), pattern.into()])
}

/// Builds `split_part(text, delimiter, field)`.
pub fn split_part(
    text: impl Into<ValueExpr>,
    delimiter: impl Into<ValueExpr>,
    field: impl Into<ValueExpr>,
) -> ValueExpr {
    function("split_part", [text.into(), delimiter.into(), field.into()])
}

/// Builds `trim(expr)`.
pub fn trim(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("trim", [expr])
}

/// Builds `ltrim(expr)`.
pub fn ltrim(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("ltrim", [expr])
}

/// Builds `rtrim(expr)`.
pub fn rtrim(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("rtrim", [expr])
}

/// Builds `btrim(expr)`.
pub fn btrim(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("btrim", [expr])
}

/// Builds `left(expr, count)`.
pub fn left(expr: impl Into<ValueExpr>, count: impl Into<ValueExpr>) -> ValueExpr {
    function("left", [expr.into(), count.into()])
}

/// Builds `right(expr, count)`.
pub fn right(expr: impl Into<ValueExpr>, count: impl Into<ValueExpr>) -> ValueExpr {
    function("right", [expr.into(), count.into()])
}

/// Builds `lpad(expr, length, fill)`.
pub fn lpad(
    expr: impl Into<ValueExpr>,
    length: impl Into<ValueExpr>,
    fill: impl Into<ValueExpr>,
) -> ValueExpr {
    function("lpad", [expr.into(), length.into(), fill.into()])
}

/// Builds `rpad(expr, length, fill)`.
pub fn rpad(
    expr: impl Into<ValueExpr>,
    length: impl Into<ValueExpr>,
    fill: impl Into<ValueExpr>,
) -> ValueExpr {
    function("rpad", [expr.into(), length.into(), fill.into()])
}

/// Builds `concat(...)`.
pub fn concat(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("concat", args)
}

/// Builds `concat_ws(separator, ...)`.
pub fn concat_ws(
    separator: impl Into<ValueExpr>,
    args: impl IntoIterator<Item = impl Into<ValueExpr>>,
) -> ValueExpr {
    let mut values = vec![separator.into()];
    values.extend(args.into_iter().map(Into::into));
    function("concat_ws", values)
}
