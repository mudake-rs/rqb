use crate::typed::{Field, FieldRef, OrderItem, SelectItem, ValueExpr};

/// Builds a generic aggregate function call.
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

/// Builds an ordered-set aggregate with a `WITHIN GROUP` ordering.
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

/// Builds `count(expr)`.
pub fn count(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("count", [expr], false)
}

/// Builds `count(*)`.
pub fn count_all() -> ValueExpr {
    aggregate("count", Vec::<ValueExpr>::new(), false)
}

/// Builds `count(DISTINCT expr)`.
pub fn count_distinct(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("count", [expr], true)
}

/// Builds `sum(expr)`.
pub fn sum(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("sum", [expr], false)
}

/// Builds `avg(expr)`.
pub fn avg(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("avg", [expr], false)
}

/// Builds `min(expr)`.
pub fn min(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("min", [expr], false)
}

/// Builds `max(expr)`.
pub fn max(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("max", [expr], false)
}

/// Builds `array_agg(expr)`.
pub fn array_agg(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("array_agg", [expr], false)
}

/// Builds `array_agg(DISTINCT expr)`.
pub fn array_agg_distinct(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("array_agg", [expr], true)
}

/// Builds `json_agg(expr)`.
pub fn json_agg(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("json_agg", [expr], false)
}

/// Builds `jsonb_agg(jsonb_build_object(...))` from selected fields or aliased expressions.
pub fn jsonb_agg_object(items: impl IntoIterator<Item = impl Into<SelectItem>>) -> ValueExpr {
    __jsonb_agg_object_from_pairs(items.into_iter().map(select_item_jsonb_object_pair))
}

#[doc(hidden)]
pub fn __jsonb_agg_object_from_pairs(
    items: impl IntoIterator<Item = (String, ValueExpr)>,
) -> ValueExpr {
    aggregate(
        "jsonb_agg",
        [ValueExpr::Function {
            name: "jsonb_build_object",
            args: items
                .into_iter()
                .flat_map(|(key, expr)| [ValueExpr::from(key), expr])
                .collect(),
        }],
        false,
    )
}

#[doc(hidden)]
pub fn __jsonb_object_pair(item: impl JsonbObjectItem) -> (String, ValueExpr) {
    item.into_jsonb_object_pair()
}

#[doc(hidden)]
pub trait JsonbObjectItem {
    fn into_jsonb_object_pair(self) -> (String, ValueExpr);
}

impl<T> JsonbObjectItem for Field<T> {
    fn into_jsonb_object_pair(self) -> (String, ValueExpr) {
        (self.meta.api.to_owned(), self.expr())
    }
}

impl<T> JsonbObjectItem for FieldRef<T> {
    fn into_jsonb_object_pair(self) -> (String, ValueExpr) {
        (self.meta.api.to_owned(), self.expr())
    }
}

impl JsonbObjectItem for SelectItem {
    fn into_jsonb_object_pair(self) -> (String, ValueExpr) {
        select_item_jsonb_object_pair(self)
    }
}

impl<V> JsonbObjectItem for (&str, V)
where
    V: Into<ValueExpr>,
{
    fn into_jsonb_object_pair(self) -> (String, ValueExpr) {
        (self.0.to_owned(), self.1.into())
    }
}

impl<V> JsonbObjectItem for (String, V)
where
    V: Into<ValueExpr>,
{
    fn into_jsonb_object_pair(self) -> (String, ValueExpr) {
        (self.0, self.1.into())
    }
}

fn select_item_jsonb_object_pair(item: impl Into<SelectItem>) -> (String, ValueExpr) {
    let SelectItem { expr, alias } = item.into();
    let key = alias
        .or_else(|| expr.field_meta().map(|meta| meta.api.to_owned()))
        .unwrap_or_else(|| "value".to_owned());

    (key, expr)
}

/// Builds `string_agg(expr, delimiter)`.
pub fn string_agg(expr: impl Into<ValueExpr>, delimiter: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("string_agg", [expr.into(), delimiter.into()], false)
}

/// Builds `bool_and(expr)`.
pub fn bool_and(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("bool_and", [expr], false)
}

/// Builds `bool_or(expr)`.
pub fn bool_or(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("bool_or", [expr], false)
}

/// Builds `every(expr)`.
pub fn every(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("every", [expr], false)
}

/// Builds `stddev(expr)`.
pub fn stddev(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("stddev", [expr], false)
}

/// Builds `stddev_pop(expr)`.
pub fn stddev_pop(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("stddev_pop", [expr], false)
}

/// Builds `stddev_samp(expr)`.
pub fn stddev_samp(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("stddev_samp", [expr], false)
}

/// Builds `variance(expr)`.
pub fn variance(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("variance", [expr], false)
}

/// Builds `var_pop(expr)`.
pub fn var_pop(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("var_pop", [expr], false)
}

/// Builds `var_samp(expr)`.
pub fn var_samp(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("var_samp", [expr], false)
}

/// Builds `percentile_cont(fraction) WITHIN GROUP (ORDER BY order_by ASC)`.
pub fn percentile_cont(
    fraction: impl Into<ValueExpr>,
    order_by: impl Into<ValueExpr>,
) -> ValueExpr {
    ordered_set_aggregate("percentile_cont", [fraction], [OrderItem::asc(order_by)])
}

/// Builds `percentile_disc(fraction) WITHIN GROUP (ORDER BY order_by ASC)`.
pub fn percentile_disc(
    fraction: impl Into<ValueExpr>,
    order_by: impl Into<ValueExpr>,
) -> ValueExpr {
    ordered_set_aggregate("percentile_disc", [fraction], [OrderItem::asc(order_by)])
}

/// Builds `mode() WITHIN GROUP (ORDER BY order_by ASC)`.
pub fn mode(order_by: impl Into<ValueExpr>) -> ValueExpr {
    ordered_set_aggregate("mode", Vec::<ValueExpr>::new(), [OrderItem::asc(order_by)])
}
