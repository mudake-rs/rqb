use crate::{Field, FieldRef, OrderItem, SelectItem, ValueExpr};

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

macro_rules! aggregate_fn {
    ($(#[$meta:meta])* $fn:ident => $name:literal) => {
        $(#[$meta])*
        pub fn $fn(expr: impl Into<ValueExpr>) -> ValueExpr {
            aggregate($name, [expr], false)
        }
    };

    ($(#[$meta:meta])* $fn:ident => distinct $name:literal) => {
        $(#[$meta])*
        pub fn $fn(expr: impl Into<ValueExpr>) -> ValueExpr {
            aggregate($name, [expr], true)
        }
    };
}

aggregate_fn!(/// Builds `count(expr)`.
    count => "count");

/// Builds `count(*)`.
pub fn count_all() -> ValueExpr {
    aggregate("count", Vec::<ValueExpr>::new(), false)
}

aggregate_fn!(/// Builds `count(DISTINCT expr)`.
    count_distinct => distinct "count");
aggregate_fn!(/// Builds `sum(expr)`.
    sum => "sum");
aggregate_fn!(/// Builds `avg(expr)`.
    avg => "avg");
aggregate_fn!(/// Builds `min(expr)`.
    min => "min");
aggregate_fn!(/// Builds `max(expr)`.
    max => "max");
aggregate_fn!(/// Builds `array_agg(expr)`.
    array_agg => "array_agg");
aggregate_fn!(/// Builds `array_agg(DISTINCT expr)`.
    array_agg_distinct => distinct "array_agg");
aggregate_fn!(/// Builds `json_agg(expr)`.
    json_agg => "json_agg");
aggregate_fn!(/// Builds `json_agg_strict(expr)`.
    json_agg_strict => "json_agg_strict");
aggregate_fn!(/// Builds `jsonb_agg(expr)`.
    jsonb_agg => "jsonb_agg");
aggregate_fn!(/// Builds `jsonb_agg_strict(expr)`.
    jsonb_agg_strict => "jsonb_agg_strict");

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

/// Builds `json_object_agg(key, value)`.
pub fn json_object_agg(key: impl Into<ValueExpr>, value: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("json_object_agg", [key.into(), value.into()], false)
}

/// Builds `jsonb_object_agg(key, value)`.
pub fn jsonb_object_agg(key: impl Into<ValueExpr>, value: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("jsonb_object_agg", [key.into(), value.into()], false)
}

/// Builds `json_object_agg_strict(key, value)`.
pub fn json_object_agg_strict(key: impl Into<ValueExpr>, value: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("json_object_agg_strict", [key.into(), value.into()], false)
}

/// Builds `jsonb_object_agg_strict(key, value)`.
pub fn jsonb_object_agg_strict(
    key: impl Into<ValueExpr>,
    value: impl Into<ValueExpr>,
) -> ValueExpr {
    aggregate("jsonb_object_agg_strict", [key.into(), value.into()], false)
}

/// Builds `json_object_agg_unique(key, value)`.
pub fn json_object_agg_unique(key: impl Into<ValueExpr>, value: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("json_object_agg_unique", [key.into(), value.into()], false)
}

/// Builds `jsonb_object_agg_unique(key, value)`.
pub fn jsonb_object_agg_unique(
    key: impl Into<ValueExpr>,
    value: impl Into<ValueExpr>,
) -> ValueExpr {
    aggregate("jsonb_object_agg_unique", [key.into(), value.into()], false)
}

/// Builds `json_object_agg_unique_strict(key, value)`.
pub fn json_object_agg_unique_strict(
    key: impl Into<ValueExpr>,
    value: impl Into<ValueExpr>,
) -> ValueExpr {
    aggregate(
        "json_object_agg_unique_strict",
        [key.into(), value.into()],
        false,
    )
}

/// Builds `jsonb_object_agg_unique_strict(key, value)`.
pub fn jsonb_object_agg_unique_strict(
    key: impl Into<ValueExpr>,
    value: impl Into<ValueExpr>,
) -> ValueExpr {
    aggregate(
        "jsonb_object_agg_unique_strict",
        [key.into(), value.into()],
        false,
    )
}

aggregate_fn!(/// Builds `bool_and(expr)`.
    bool_and => "bool_and");
aggregate_fn!(/// Builds `bool_or(expr)`.
    bool_or => "bool_or");
aggregate_fn!(/// Builds `every(expr)`.
    every => "every");
aggregate_fn!(/// Builds `any_value(expr)`.
    any_value => "any_value");
aggregate_fn!(/// Builds `bit_and(expr)`.
    bit_and => "bit_and");
aggregate_fn!(/// Builds `bit_or(expr)`.
    bit_or => "bit_or");
aggregate_fn!(/// Builds `bit_xor(expr)`.
    bit_xor => "bit_xor");
aggregate_fn!(/// Builds `range_agg(expr)`.
    range_agg => "range_agg");
aggregate_fn!(/// Builds `range_intersect_agg(expr)`.
    range_intersect_agg => "range_intersect_agg");
aggregate_fn!(/// Builds `stddev(expr)`.
    stddev => "stddev");
aggregate_fn!(/// Builds `stddev_pop(expr)`.
    stddev_pop => "stddev_pop");
aggregate_fn!(/// Builds `stddev_samp(expr)`.
    stddev_samp => "stddev_samp");
aggregate_fn!(/// Builds `variance(expr)`.
    variance => "variance");
aggregate_fn!(/// Builds `var_pop(expr)`.
    var_pop => "var_pop");
aggregate_fn!(/// Builds `var_samp(expr)`.
    var_samp => "var_samp");

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

/// Builds `GROUPING(...)` for grouped reports using rollups, cubes, or grouping sets.
pub fn grouping(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    ValueExpr::Function {
        name: "GROUPING",
        args: args.into_iter().map(Into::into).collect(),
    }
}
