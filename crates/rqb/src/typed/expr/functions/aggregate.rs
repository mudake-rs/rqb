use crate::typed::{OrderItem, ValueExpr};

pub fn aggregate(
    name: &'static str,
    args: impl IntoIterator<Item = impl Into<ValueExpr>>,
    distinct: bool,
) -> ValueExpr {
    ValueExpr::Aggregate {
        name,
        args: args.into_iter().map(Into::into).collect(),
        distinct,
        order_by: Vec::new(),
        filter: None,
    }
}

pub fn ordered_set_aggregate(
    name: &'static str,
    args: impl IntoIterator<Item = impl Into<ValueExpr>>,
    within_group: impl IntoIterator<Item = OrderItem>,
) -> ValueExpr {
    ValueExpr::OrderedSetAggregate {
        name,
        args: args.into_iter().map(Into::into).collect(),
        within_group: within_group.into_iter().collect(),
        filter: None,
    }
}

pub fn count(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("count", [expr], false)
}

pub fn count_all() -> ValueExpr {
    aggregate("count", Vec::<ValueExpr>::new(), false)
}

pub fn count_distinct(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("count", [expr], true)
}

pub fn sum(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("sum", [expr], false)
}

pub fn avg(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("avg", [expr], false)
}

pub fn min(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("min", [expr], false)
}

pub fn max(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("max", [expr], false)
}

pub fn array_agg(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("array_agg", [expr], false)
}

pub fn array_agg_distinct(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("array_agg", [expr], true)
}

pub fn json_agg(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("json_agg", [expr], false)
}

pub fn string_agg(expr: impl Into<ValueExpr>, delimiter: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("string_agg", [expr.into(), delimiter.into()], false)
}

pub fn bool_and(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("bool_and", [expr], false)
}

pub fn bool_or(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("bool_or", [expr], false)
}

pub fn every(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("every", [expr], false)
}

pub fn stddev(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("stddev", [expr], false)
}

pub fn stddev_pop(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("stddev_pop", [expr], false)
}

pub fn stddev_samp(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("stddev_samp", [expr], false)
}

pub fn variance(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("variance", [expr], false)
}

pub fn var_pop(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("var_pop", [expr], false)
}

pub fn var_samp(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("var_samp", [expr], false)
}

pub fn percentile_cont(
    fraction: impl Into<ValueExpr>,
    order_by: impl Into<ValueExpr>,
) -> ValueExpr {
    ordered_set_aggregate("percentile_cont", [fraction], [OrderItem::asc(order_by)])
}

pub fn percentile_disc(
    fraction: impl Into<ValueExpr>,
    order_by: impl Into<ValueExpr>,
) -> ValueExpr {
    ordered_set_aggregate("percentile_disc", [fraction], [OrderItem::asc(order_by)])
}

pub fn mode(order_by: impl Into<ValueExpr>) -> ValueExpr {
    ordered_set_aggregate("mode", Vec::<ValueExpr>::new(), [OrderItem::asc(order_by)])
}
