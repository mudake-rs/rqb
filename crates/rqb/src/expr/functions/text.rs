use crate::{BoolExpr, ValueExpr};

use super::function;

/// Builds `length(expr)`.
pub fn length(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("length", [expr])
}

/// Builds `char_length(expr)`.
pub fn char_length(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("char_length", [expr])
}

/// Builds `octet_length(expr)`.
pub fn octet_length(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("octet_length", [expr])
}

/// Builds `lower(expr)`.
pub fn lower(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("lower", [expr])
}

/// Builds `upper(expr)`.
pub fn upper(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("upper", [expr])
}

/// Builds `initcap(expr)`.
pub fn initcap(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("initcap", [expr])
}

/// Builds `casefold(expr)`.
pub fn casefold(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("casefold", [expr])
}

/// Builds `normalize(expr)`.
pub fn normalize(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("normalize", [expr])
}

/// Builds `normalize(expr, form)` where form is a PostgreSQL normalization keyword.
pub fn normalize_form(expr: impl Into<ValueExpr>, form: &'static str) -> ValueExpr {
    function("normalize", [expr.into(), ValueExpr::Keyword(form)])
}

/// Builds `unicode_assigned(expr)`.
pub fn unicode_assigned(expr: impl Into<ValueExpr>) -> BoolExpr {
    function("unicode_assigned", [expr]).is_true()
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

/// Builds `strpos(text, substring)`.
pub fn strpos(text: impl Into<ValueExpr>, substring: impl Into<ValueExpr>) -> ValueExpr {
    function("strpos", [text.into(), substring.into()])
}

/// Builds `format(format, ...)`.
pub fn format(
    format: impl Into<ValueExpr>,
    args: impl IntoIterator<Item = impl Into<ValueExpr>>,
) -> ValueExpr {
    let mut values = vec![format.into()];
    values.extend(args.into_iter().map(Into::into));
    function("format", values)
}

/// Builds `translate(text, from, to)`.
pub fn translate(
    text: impl Into<ValueExpr>,
    from: impl Into<ValueExpr>,
    to: impl Into<ValueExpr>,
) -> ValueExpr {
    function("translate", [text.into(), from.into(), to.into()])
}

/// Builds `repeat(text, count)`.
pub fn repeat(text: impl Into<ValueExpr>, count: impl Into<ValueExpr>) -> ValueExpr {
    function("repeat", [text.into(), count.into()])
}

/// Builds `ascii(expr)`.
pub fn ascii(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("ascii", [expr])
}

/// Builds `chr(code)`.
pub fn chr(code: impl Into<ValueExpr>) -> ValueExpr {
    function("chr", [code])
}

/// Builds `encode(bytea, format)`.
pub fn encode(bytea: impl Into<ValueExpr>, format: impl Into<ValueExpr>) -> ValueExpr {
    function("encode", [bytea.into(), format.into()])
}

/// Builds `decode(text, format)`.
pub fn decode(text: impl Into<ValueExpr>, format: impl Into<ValueExpr>) -> ValueExpr {
    function("decode", [text.into(), format.into()])
}

/// Builds `starts_with(text, prefix)`.
pub fn text_starts_with(text: impl Into<ValueExpr>, prefix: impl Into<ValueExpr>) -> BoolExpr {
    function("starts_with", [text.into(), prefix.into()]).is_true()
}

/// Alias for [`text_starts_with`].
pub fn starts_with(text: impl Into<ValueExpr>, prefix: impl Into<ValueExpr>) -> BoolExpr {
    text_starts_with(text, prefix)
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

/// Builds `reverse(expr)`.
pub fn reverse(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("reverse", [expr])
}

/// Builds `md5(expr)`.
pub fn md5(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("md5", [expr])
}
