use sqlx::{Encode, Postgres, Type};

use crate::typed::{BoolExpr, Param};

use super::{ValueExpr, ValueOp};

mod aggregate;
mod array_fn;
mod date;
mod fts;
mod json;
mod math;
mod merge;
mod scalar;
mod text;
mod uuid;

pub use aggregate::{
    __jsonb_agg_object_from_pairs, __jsonb_object_pair, JsonbObjectItem, aggregate, array_agg,
    array_agg_distinct, avg, bool_and, bool_or, count, count_all, count_distinct, every, json_agg,
    jsonb_agg_object, max, min, mode, ordered_set_aggregate, percentile_cont, percentile_disc,
    stddev, stddev_pop, stddev_samp, string_agg, sum, var_pop, var_samp, variance,
};
pub use array_fn::{
    array_append, array_length, array_position, array_positions, array_prepend, array_remove,
    array_replace, array_to_string, cardinality, string_to_array, unnest,
};
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
    abs, ceil, exp, floor, ln, log, mod_, pow, power, random, random_between, round, sqrt, trunc,
};
pub use merge::merge_action;
pub use scalar::{coalesce, greatest, least, nullif};
pub use text::{
    btrim, char_length, concat, concat_ws, left, length, lower, lpad, ltrim, regexp_matches,
    regexp_replace, regexp_split_to_array, replace, right, rpad, rtrim, split_part, substring,
    trim, upper,
};
pub use uuid::{uuid_extract_timestamp, uuid_extract_version, uuidv7};

pub fn param<T>(value: T) -> ValueExpr
where
    T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
{
    ValueExpr::Param(Param::typed(value))
}

pub fn function(
    name: &'static str,
    args: impl IntoIterator<Item = impl Into<ValueExpr>>,
) -> ValueExpr {
    ValueExpr::Function {
        name,
        args: args.into_iter().map(Into::into).collect(),
    }
}

pub fn array(values: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    ValueExpr::Array(values.into_iter().map(Into::into).collect())
}

pub fn row(values: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    ValueExpr::Row(values.into_iter().map(Into::into).collect())
}

pub fn subscript(expr: impl Into<ValueExpr>, index: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Subscript {
        expr: Box::new(expr.into()),
        index: Box::new(index.into()),
    }
}

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

pub fn concat_op(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(left.into()),
        op: ValueOp::Custom("||"),
        right: Box::new(right.into()),
    }
}

pub fn similar_to(expr: impl Into<ValueExpr>, pattern: impl Into<ValueExpr>) -> BoolExpr {
    BoolExpr::SimilarTo {
        expr: expr.into(),
        pattern: pattern.into(),
        negated: false,
    }
}

pub fn not_similar_to(expr: impl Into<ValueExpr>, pattern: impl Into<ValueExpr>) -> BoolExpr {
    BoolExpr::SimilarTo {
        expr: expr.into(),
        pattern: pattern.into(),
        negated: true,
    }
}
