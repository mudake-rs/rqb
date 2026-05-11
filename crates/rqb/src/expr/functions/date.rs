use crate::{BoolExpr, ValueExpr};

use super::function;

/// Builds `now()`.
pub fn now() -> ValueExpr {
    function("now", Vec::<ValueExpr>::new())
}

/// Builds the `CURRENT_TIMESTAMP` SQL keyword.
pub fn current_timestamp() -> ValueExpr {
    ValueExpr::Keyword("CURRENT_TIMESTAMP")
}

/// Builds the `CURRENT_DATE` SQL keyword.
pub fn current_date() -> ValueExpr {
    ValueExpr::Keyword("CURRENT_DATE")
}

/// Builds `date_trunc(part, expr)`.
pub fn date_trunc(part: impl Into<ValueExpr>, expr: impl Into<ValueExpr>) -> ValueExpr {
    function("date_trunc", [part.into(), expr.into()])
}

/// Builds `date_bin(stride, source, origin)`.
pub fn date_bin(
    stride: impl Into<ValueExpr>,
    source: impl Into<ValueExpr>,
    origin: impl Into<ValueExpr>,
) -> ValueExpr {
    function("date_bin", [stride.into(), source.into(), origin.into()])
}

/// Builds `to_char(value, format)`.
pub fn to_char(value: impl Into<ValueExpr>, format: impl Into<ValueExpr>) -> ValueExpr {
    function("to_char", [value.into(), format.into()])
}

/// Builds `to_date(text, format)`.
pub fn to_date(text: impl Into<ValueExpr>, format: impl Into<ValueExpr>) -> ValueExpr {
    function("to_date", [text.into(), format.into()])
}

/// Builds `to_timestamp(text, format)`.
pub fn to_timestamp(text: impl Into<ValueExpr>, format: impl Into<ValueExpr>) -> ValueExpr {
    function("to_timestamp", [text.into(), format.into()])
}

/// Builds `to_number(text, format)`.
pub fn to_number(text: impl Into<ValueExpr>, format: impl Into<ValueExpr>) -> ValueExpr {
    function("to_number", [text.into(), format.into()])
}

/// Builds `isfinite(value)`.
pub fn isfinite(value: impl Into<ValueExpr>) -> BoolExpr {
    function("isfinite", [value]).is_true()
}

/// Builds `EXTRACT(field FROM expr)`.
pub fn extract(field: &'static str, expr: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Extract {
        field,
        expr: Box::new(expr.into()),
    }
}

/// Builds `age(...)` with one or two arguments.
pub fn age(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("age", args)
}

/// Builds `make_date(year, month, day)`.
pub fn make_date(
    year: impl Into<ValueExpr>,
    month: impl Into<ValueExpr>,
    day: impl Into<ValueExpr>,
) -> ValueExpr {
    function("make_date", [year.into(), month.into(), day.into()])
}

/// Builds `make_time(hour, min, sec)`.
pub fn make_time(
    hour: impl Into<ValueExpr>,
    min: impl Into<ValueExpr>,
    sec: impl Into<ValueExpr>,
) -> ValueExpr {
    function("make_time", [hour.into(), min.into(), sec.into()])
}

/// Builds `make_timestamp(...)`.
pub fn make_timestamp(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("make_timestamp", args)
}

/// Builds `make_timestamptz(...)`.
pub fn make_timestamptz(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("make_timestamptz", args)
}

/// Builds `timezone(zone, expr)`.
pub fn timezone(zone: impl Into<ValueExpr>, expr: impl Into<ValueExpr>) -> ValueExpr {
    function("timezone", [zone.into(), expr.into()])
}
