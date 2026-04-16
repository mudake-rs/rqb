//! sqlx-first Postgres query builder for Rust services.
//!
//! Application code describes server-owned query shape with typed fields,
//! renders parameterized SQL, and executes through any
//! `sqlx::Executor<Database = Postgres>`.

#![allow(clippy::result_large_err)]

mod error;
mod tx;
pub mod typed;

extern crate self as rqb;

pub use chrono;
pub use error::{DbErrorInfo, DbErrorPosition, Error};
pub use rqb_macros::{Changeset, Insertable, schema};
pub use serde;
pub use serde_json;
pub use sqlx;
pub use sqlx::{PgConnection, PgExecutor, PgPool};
pub use typed::{
    Assignment, BoolExpr, BoolOp, BooleanTest, BuiltQuery, Changeset, ConflictAction,
    ConflictClause, ConflictTarget, Cte, CteMaterialization, Delete, FetchClause, Field, FieldRef,
    FrameBound, FrameExclude, GroupByItem, Insert, InsertConflictBuilder, Insertable, IntoFieldRef,
    Join, JoinKind, JsonKind, LockMode, LockWait, Merge, MergeAction, MergeWhen, Meta,
    NullsPosition, OffsetWindowFunctionBuilder, OpSet, OrderDirection, OrderItem, Param, Params,
    RawStmt, RowLock, SearchFilter, SearchOperator, SearchPredicate, SearchRequest, SearchSort,
    Select, SelectItem, SetOperator, SetQuery, SortDirection, Source, Stmt, Update, ValueExpr,
    ValueOp, WindowFrame, WindowFrameKind, WindowFunction, WindowFunctionBuilder, WindowSpec, cte,
    cte_source, delete_from, except, except_all, function_source, insert, intersect, intersect_all,
    merge_into, raw, raw_source, select, subquery, table, union, union_all, update, view,
};
pub use uuid;

pub type Result<T> = std::result::Result<T, Error>;

/// SQL expression helpers that are useful on demand but too broad for the prelude.
pub mod dsl {
    pub use crate::typed::{
        abs, age, aggregate, array, array_agg, array_agg_distinct, array_append, array_length,
        array_position, array_positions, array_prepend, array_remove, array_replace,
        array_to_string, avg, bool_and, bool_or, btrim, cardinality, ceil, char_length, coalesce,
        concat, concat_op, concat_ws, count, count_all, count_distinct, cume_dist, current_date,
        current_row, current_timestamp, date_trunc, dense_rank, every, exp, extract, first_value,
        floor, following, function, greatest, groups, json, json_agg, json_exists, json_get,
        json_get_text, json_path, json_path_text, json_query, json_scalar, json_serialize,
        json_value, jsonb_array_elements, jsonb_build_array, jsonb_build_object, jsonb_delete,
        jsonb_each, jsonb_insert, jsonb_object, jsonb_path_exists, jsonb_path_query, jsonb_set,
        jsonb_strip_nulls, jsonb_typeof, lag, last_value, lead, least, left, length, ln, log,
        lower, lpad, ltrim, make_date, make_time, make_timestamp, make_timestamptz, max,
        merge_action, min, mod_, mode, not_similar_to, now, nth_value, ntile, nullif,
        ordered_set_aggregate, param, partition_by, percent_rank, percentile_cont, percentile_disc,
        plainto_tsquery, pow, power, preceding, random, random_between, range, rank,
        regexp_matches, regexp_replace, regexp_split_to_array, replace, right, round, row,
        row_number, rows, rpad, rtrim, similar_to, slice, split_part, sqrt, stddev, stddev_pop,
        stddev_samp, string_agg, string_to_array, subscript, substring, sum, timezone, to_json,
        to_jsonb, to_tsquery, to_tsvector, to_tsvector_config, trim, trunc, ts_match, ts_rank,
        ts_rank_cd, unbounded_following, unbounded_preceding, unnest, upper,
        uuid_extract_timestamp, uuid_extract_version, uuidv7, var_pop, var_samp, variance,
        websearch_to_tsquery, window,
    };
}

pub mod prelude {
    pub use crate::{
        Assignment, BoolExpr, BoolOp, BooleanTest, BuiltQuery, Changeset, ConflictAction,
        ConflictClause, ConflictTarget, Cte, CteMaterialization, DbErrorInfo, DbErrorPosition,
        Delete, Error, FetchClause, Field, FieldRef, FrameBound, FrameExclude, GroupByItem, Insert,
        InsertConflictBuilder, Insertable, IntoFieldRef, Join, JoinKind, JsonKind, LockMode,
        LockWait, Merge, MergeAction, MergeWhen, Meta, NullsPosition, OffsetWindowFunctionBuilder,
        OpSet, OrderDirection, OrderItem, Param, Params, PgConnection, PgExecutor, PgPool, RawStmt,
        Result, RowLock, SearchFilter, SearchOperator, SearchPredicate, SearchRequest, SearchSort,
        Select, SelectItem, SetOperator, SetQuery, SortDirection, Source, Stmt, Update, ValueExpr,
        ValueOp, WindowFrame, WindowFrameKind, WindowFunction, WindowFunctionBuilder, WindowSpec,
        cte, cte_source, delete_from, except, except_all, function_source, insert, intersect,
        intersect_all, merge_into, raw, raw_source, schema, select, subquery, table, tx, union,
        union_all, update, view,
    };
}

#[cfg(test)]
mod tests {
    use super::{Error, Field, Meta, OpSet};

    #[test]
    fn facade_exports_typed_field_and_error() {
        static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::equality());
        const ID: Field<i32> = Field::new(&ID_META);

        let error = ID.gt(10).validate().unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidTypedOperator { field, operator }
                if field == "id" && operator == "gt"
        ));
    }

    #[test]
    fn tx_macro_type_checks_with_pool_reference_and_value_return() {
        fn assert_type_checks(pool: sqlx::PgPool) {
            let future = crate::tx!(&pool, |conn| {
                let _: &mut sqlx::PgConnection = conn;
                Ok::<_, Error>(123_i32)
            });
            drop(future);
        }

        let _ = assert_type_checks as fn(sqlx::PgPool);
    }

    #[test]
    fn tx_macro_type_checks_with_owned_pool_expression() {
        fn assert_type_checks(pool: sqlx::PgPool) {
            let future = crate::tx!(pool.clone(), |conn| {
                let _: &mut sqlx::PgConnection = conn;
                Ok::<_, Error>(())
            });
            drop(future);
        }

        let _ = assert_type_checks as fn(sqlx::PgPool);
    }
}
