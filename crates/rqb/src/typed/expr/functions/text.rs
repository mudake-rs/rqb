use crate::typed::ValueExpr;

use super::function;

pub fn length(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("length", [expr])
}

pub fn char_length(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("char_length", [expr])
}

pub fn lower(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("lower", [expr])
}

pub fn upper(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("upper", [expr])
}

pub fn substring(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("substring", args)
}

pub fn replace(
    text: impl Into<ValueExpr>,
    from: impl Into<ValueExpr>,
    to: impl Into<ValueExpr>,
) -> ValueExpr {
    function("replace", [text.into(), from.into(), to.into()])
}

pub fn regexp_replace(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("regexp_replace", args)
}

pub fn regexp_matches(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("regexp_matches", args)
}

pub fn regexp_split_to_array(
    text: impl Into<ValueExpr>,
    pattern: impl Into<ValueExpr>,
) -> ValueExpr {
    function("regexp_split_to_array", [text.into(), pattern.into()])
}

pub fn split_part(
    text: impl Into<ValueExpr>,
    delimiter: impl Into<ValueExpr>,
    field: impl Into<ValueExpr>,
) -> ValueExpr {
    function("split_part", [text.into(), delimiter.into(), field.into()])
}

pub fn trim(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("trim", [expr])
}

pub fn ltrim(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("ltrim", [expr])
}

pub fn rtrim(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("rtrim", [expr])
}

pub fn btrim(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("btrim", [expr])
}

pub fn left(expr: impl Into<ValueExpr>, count: impl Into<ValueExpr>) -> ValueExpr {
    function("left", [expr.into(), count.into()])
}

pub fn right(expr: impl Into<ValueExpr>, count: impl Into<ValueExpr>) -> ValueExpr {
    function("right", [expr.into(), count.into()])
}

pub fn lpad(
    expr: impl Into<ValueExpr>,
    length: impl Into<ValueExpr>,
    fill: impl Into<ValueExpr>,
) -> ValueExpr {
    function("lpad", [expr.into(), length.into(), fill.into()])
}

pub fn rpad(
    expr: impl Into<ValueExpr>,
    length: impl Into<ValueExpr>,
    fill: impl Into<ValueExpr>,
) -> ValueExpr {
    function("rpad", [expr.into(), length.into(), fill.into()])
}

pub fn concat(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("concat", args)
}

pub fn concat_ws(
    separator: impl Into<ValueExpr>,
    args: impl IntoIterator<Item = impl Into<ValueExpr>>,
) -> ValueExpr {
    let mut values = vec![separator.into()];
    values.extend(args.into_iter().map(Into::into));
    function("concat_ws", values)
}
