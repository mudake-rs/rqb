use crate::typed::{BindValue, BoolExpr, CaseBuilder, Param};

use super::{ValueExpr, ValueOp};

mod aggregate;
mod array_fn;
mod binary;
mod date;
mod fts;
mod json;
mod math;
mod merge;
mod scalar;
mod text;
mod uuid;

pub use aggregate::{
    __jsonb_agg_object_from_pairs, __jsonb_object_pair, JsonbObjectItem, aggregate, any_value,
    array_agg, array_agg_distinct, avg, bit_and, bit_or, bit_xor, bool_and, bool_or, count,
    count_all, count_distinct, every, grouping, json_agg, json_agg_strict, json_object_agg,
    json_object_agg_strict, json_object_agg_unique, json_object_agg_unique_strict, jsonb_agg,
    jsonb_agg_object, jsonb_agg_strict, jsonb_object_agg, jsonb_object_agg_strict,
    jsonb_object_agg_unique, jsonb_object_agg_unique_strict, max, min, mode, ordered_set_aggregate,
    percentile_cont, percentile_disc, range_agg, range_intersect_agg, stddev, stddev_pop,
    stddev_samp, string_agg, sum, var_pop, var_samp, variance,
};
pub use array_fn::{
    array_append, array_cat, array_dims, array_fill, array_fill_with_lower_bounds, array_length,
    array_lower, array_ndims, array_position, array_positions, array_prepend, array_remove,
    array_replace, array_reverse, array_sample, array_shuffle, array_sort, array_sort_desc,
    array_sort_with, array_to_string, array_upper, cardinality, string_to_array, unnest,
};
pub use binary::{crc32, crc32c};
pub use date::{
    age, current_date, current_timestamp, date_trunc, extract, make_date, make_time,
    make_timestamp, make_timestamptz, now, timezone,
};
pub use fts::{
    plainto_tsquery, to_tsquery, to_tsvector, to_tsvector_config, ts_match, ts_rank, ts_rank_cd,
    websearch_to_tsquery,
};
pub use json::{
    json, json_exists, json_get, json_get_text, json_path, json_path_text, json_query, json_scalar,
    json_serialize, json_value, jsonb_array_elements, jsonb_build_array, jsonb_build_object,
    jsonb_delete, jsonb_each, jsonb_insert, jsonb_object, jsonb_path_exists, jsonb_path_query,
    jsonb_set, jsonb_strip_nulls, jsonb_typeof, to_json, to_jsonb,
};
pub use math::{
    abs, cbrt, ceil, degrees, div, exp, factorial, floor, gamma, gcd, lcm, lgamma, ln, log, mod_,
    pi, pow, power, radians, random, random_between, round, sign, sqrt, trunc,
};
pub use merge::merge_action;
pub use scalar::{coalesce, greatest, least, nullif};
pub use text::{
    btrim, casefold, char_length, concat, concat_ws, left, length, lower, lpad, ltrim, md5,
    normalize, normalize_form, regexp_matches, regexp_replace, regexp_split_to_array, replace,
    reverse, right, rpad, rtrim, split_part, strpos, substring, text_starts_with, trim,
    unicode_assigned, upper,
};
pub use uuid::{
    gen_random_uuid, uuid_extract_timestamp, uuid_extract_version, uuidv4, uuidv7, uuidv7_shift,
};

/// Wraps an owned Rust value as a typed bind parameter expression.
pub fn param<T>(value: T) -> ValueExpr
where
    T: BindValue,
{
    ValueExpr::Param(Param::typed(value))
}

/// Builds a scalar subquery value expression.
pub fn scalar_subquery(stmt: impl Into<crate::typed::Stmt>) -> ValueExpr {
    ValueExpr::Subquery(Box::new(stmt.into()))
}

/// Starts a SQL `CASE` expression builder.
pub fn case() -> CaseBuilder {
    CaseBuilder::new()
}

/// Builds a generic SQL function call.
pub fn function(
    name: &'static str,
    args: impl IntoIterator<Item = impl Into<ValueExpr>>,
) -> ValueExpr {
    ValueExpr::Function {
        name,
        args: args.into_iter().map(Into::into).collect(),
    }
}

/// Builds an SQL array expression from value expressions.
pub fn array(values: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    ValueExpr::Array(values.into_iter().map(Into::into).collect())
}

/// Builds an SQL row expression from value expressions.
pub fn row(values: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    ValueExpr::Row(values.into_iter().map(Into::into).collect())
}

/// Builds an array or JSON subscript expression.
pub fn subscript(expr: impl Into<ValueExpr>, index: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Subscript {
        expr: Box::new(expr.into()),
        index: Box::new(index.into()),
    }
}

/// Builds an array or JSON slice expression.
pub fn slice(
    expr: impl Into<ValueExpr>,
    start: Option<impl Into<ValueExpr>>,
    end: Option<impl Into<ValueExpr>>,
) -> ValueExpr {
    ValueExpr::Slice {
        expr: Box::new(expr.into()),
        start: start.map(|expr| Box::new(expr.into())),
        end: end.map(|expr| Box::new(expr.into())),
    }
}

/// Builds a Postgres `||` concatenation expression.
pub fn concat_op(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(left.into()),
        op: ValueOp::Custom("||"),
        right: Box::new(right.into()),
    }
}

/// Builds a `SIMILAR TO` predicate.
pub fn similar_to(expr: impl Into<ValueExpr>, pattern: impl Into<ValueExpr>) -> BoolExpr {
    BoolExpr::SimilarTo {
        expr: expr.into(),
        pattern: pattern.into(),
        negated: false,
    }
}

/// Builds a negated `SIMILAR TO` predicate.
pub fn not_similar_to(expr: impl Into<ValueExpr>, pattern: impl Into<ValueExpr>) -> BoolExpr {
    BoolExpr::SimilarTo {
        expr: expr.into(),
        pattern: pattern.into(),
        negated: true,
    }
}
