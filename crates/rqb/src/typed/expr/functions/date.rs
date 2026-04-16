use crate::typed::ValueExpr;

use super::function;

pub fn now() -> ValueExpr {
    function("now", Vec::<ValueExpr>::new())
}

pub fn current_timestamp() -> ValueExpr {
    ValueExpr::Keyword("CURRENT_TIMESTAMP")
}

pub fn current_date() -> ValueExpr {
    ValueExpr::Keyword("CURRENT_DATE")
}

pub fn date_trunc(part: impl Into<ValueExpr>, expr: impl Into<ValueExpr>) -> ValueExpr {
    function("date_trunc", [part.into(), expr.into()])
}

pub fn extract(field: &'static str, expr: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Extract {
        field,
        expr: Box::new(expr.into()),
    }
}

pub fn age(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("age", args)
}

pub fn make_date(
    year: impl Into<ValueExpr>,
    month: impl Into<ValueExpr>,
    day: impl Into<ValueExpr>,
) -> ValueExpr {
    function("make_date", [year.into(), month.into(), day.into()])
}

pub fn make_time(
    hour: impl Into<ValueExpr>,
    min: impl Into<ValueExpr>,
    sec: impl Into<ValueExpr>,
) -> ValueExpr {
    function("make_time", [hour.into(), min.into(), sec.into()])
}

pub fn make_timestamp(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("make_timestamp", args)
}

pub fn make_timestamptz(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("make_timestamptz", args)
}

pub fn timezone(zone: impl Into<ValueExpr>, expr: impl Into<ValueExpr>) -> ValueExpr {
    function("timezone", [zone.into(), expr.into()])
}
