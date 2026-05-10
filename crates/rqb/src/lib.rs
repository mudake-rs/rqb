//! sqlx-first Postgres query builder for Rust services.
//!
//! rqb is not an ORM. Application code owns SQL shape, rqb validates the typed
//! AST before rendering, and values are passed to Postgres as sqlx bind
//! arguments.
//!
//! Start with generated schema modules:
//!
//! ```rust,ignore
//! rqb::schema! {
//!     table public.users {
//!         id: uuid = uuid::Uuid,
//!         email: text = String,
//!         active: bool = bool,
//!     }
//! }
//!
//! let u = users::alias("u");
//! let query = rqb::select(&u)
//!     .column(u.id())
//!     .column(u.email())
//!     .filter(u.active().eq(true))
//!     .build()?;
//! ```
//!
//! Complex expression helpers live in [`dsl`]. Core builder types and generated
//! schema support are exported from [`prelude`].

#![allow(clippy::result_large_err)]

mod error;
mod tx;
/// Lower-level typed query building modules.
///
/// Most application code should import [`prelude`] and selected helpers from
/// [`dsl`]. This module remains public for users who want explicit paths to the
/// AST, source, metadata, parameter, and request types.
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
    Assignment, BindValue, BoolExpr, BoolOp, BooleanTest, BuiltQuery, CaseBuilder, Changeset,
    ColumnConflictBuilder, ConflictAction, ConflictClause, ConflictFields, ConflictTarget,
    ConstraintConflictBuilder, Cte, CteMaterialization, Delete, FetchClause, Field, FieldRef,
    FrameBound, FrameExclude, GroupByItem, Insert, Insertable, IntoFieldMetas, IntoFieldRef,
    IntoSelectItems, Join, JoinKind, JsonKind, LockMode, LockWait, MatchedMergeBuilder, Merge,
    MergeAction, MergeWhen, Meta, NotMatchedBySourceMergeBuilder, NotMatchedMergeBuilder,
    NullsPosition, OffsetWindowFunctionBuilder, OpSet, OrderDirection, OrderItem, Param, Params,
    RawStmt, RowLock, ScalarValue, SearchFilter, SearchOperator, SearchPredicate, SearchRequest,
    SearchSort, Select, SelectItem, SetOperator, SetQuery, SortDirection, Source, Stmt, Update,
    ValueExpr, ValueOp, WindowFrame, WindowFrameKind, WindowFunction, WindowFunctionBuilder,
    WindowSpec, case, cte, cte_ref, delete_from, except, except_all, false_, function_source,
    insert, intersect, intersect_all, merge_into, raw, raw_source, select, subquery, table, true_,
    union, union_all, update, view,
};
pub use uuid;

/// Creates a metadata-backed computed field for CTEs, subqueries, and projections.
///
/// Generated schema should stay the default for real table columns. This macro
/// is for derived columns such as `count(*) AS item_count`, where rqb still
/// needs field metadata for later joins or outer projections.
///
/// The metadata is initialized once per macro expansion site with `OnceLock`,
/// so it can be used inside functions without leaking on repeated calls.
///
/// ```rust,ignore
/// let item_count = rqb::field!("item_count": int8 => i64, ordered);
/// let raw_score = rqb::field!("score": "numeric" => rust_decimal::Decimal, equality);
/// ```
#[macro_export]
macro_rules! field {
    ($name:literal : $pg:ident => $ty:ty) => {
        $crate::field!(@build $name, stringify!($pg), $ty, none)
    };
    ($name:literal : $pg:literal => $ty:ty) => {
        $crate::field!(@build $name, $pg, $ty, none)
    };
    ($name:literal : $pg:ident => $ty:ty, $ops:ident) => {
        $crate::field!(@build $name, stringify!($pg), $ty, $ops)
    };
    ($name:literal : $pg:literal => $ty:ty, $ops:ident) => {
        $crate::field!(@build $name, $pg, $ty, $ops)
    };
    (@build $name:literal, $pg:expr, $ty:ty, none) => {{
        static __RQB_FIELD_META: ::std::sync::OnceLock<$crate::Meta> =
            ::std::sync::OnceLock::new();
        $crate::Field::<$ty>::new(__RQB_FIELD_META.get_or_init(|| {
            $crate::Meta::col($name, $pg).ops($crate::OpSet::none())
        }))
    }};
    (@build $name:literal, $pg:expr, $ty:ty, equality) => {{
        static __RQB_FIELD_META: ::std::sync::OnceLock<$crate::Meta> =
            ::std::sync::OnceLock::new();
        $crate::Field::<$ty>::new(__RQB_FIELD_META.get_or_init(|| {
            $crate::Meta::col($name, $pg).ops($crate::OpSet::equality())
        }))
    }};
    (@build $name:literal, $pg:expr, $ty:ty, ordered) => {{
        static __RQB_FIELD_META: ::std::sync::OnceLock<$crate::Meta> =
            ::std::sync::OnceLock::new();
        $crate::Field::<$ty>::new(__RQB_FIELD_META.get_or_init(|| {
            $crate::Meta::col($name, $pg).ops($crate::OpSet::ordered())
        }))
    }};
}

/// Builds `jsonb_agg(jsonb_build_object(...))` from metadata-backed fields.
///
/// Field and field-ref arguments use their metadata database name as the JSON
/// object key. Computed expressions can be passed as `("key", expr)` pairs.
#[macro_export]
macro_rules! jsonb_agg_object {
    ($($item:expr),+ $(,)?) => {{
        $crate::typed::__jsonb_agg_object_from_pairs([
            $($crate::typed::__jsonb_object_pair($item)),+
        ])
    }};
}

/// rqb result type.
pub type Result<T> = std::result::Result<T, Error>;

/// SQL expression helpers that are useful on demand but too broad for the prelude.
pub mod dsl {
    /// Boolean predicate helper functions.
    pub mod bools {
        pub use crate::typed::{and, exists, false_, not, or, true_};
    }

    /// Aggregate helper functions.
    pub mod agg {
        pub use crate::typed::{
            aggregate, any_value, array_agg, array_agg_distinct, avg, bit_and, bit_or, bit_xor,
            bool_and, bool_or, count, count_all, count_distinct, every, grouping, json_agg,
            json_agg_strict, json_object_agg, json_object_agg_strict, json_object_agg_unique,
            json_object_agg_unique_strict, jsonb_agg, jsonb_agg_object, jsonb_agg_strict,
            jsonb_object_agg, jsonb_object_agg_strict, jsonb_object_agg_unique,
            jsonb_object_agg_unique_strict, max, min, mode, ordered_set_aggregate, percentile_cont,
            percentile_disc, range_agg, range_intersect_agg, stddev, stddev_pop, stddev_samp,
            string_agg, sum, var_pop, var_samp, variance,
        };
    }

    /// Array helper functions.
    pub mod arrays {
        pub use crate::typed::{
            array, array_append, array_cat, array_dims, array_fill, array_fill_with_lower_bounds,
            array_length, array_lower, array_ndims, array_position, array_positions, array_prepend,
            array_remove, array_replace, array_reverse, array_sample, array_shuffle, array_sort,
            array_sort_desc, array_sort_with, array_to_string, array_upper, cardinality,
            string_to_array, unnest,
        };
    }

    /// Binary string helper functions.
    pub mod binary {
        pub use crate::typed::{crc32, crc32c};
    }

    /// Date and time helper functions.
    pub mod date {
        pub use crate::typed::{
            age, current_date, current_timestamp, date_trunc, extract, make_date, make_time,
            make_timestamp, make_timestamptz, now, timezone,
        };
    }

    /// Full-text search helper functions.
    pub mod fts {
        pub use crate::typed::{
            plainto_tsquery, to_tsquery, to_tsvector, to_tsvector_config, ts_match, ts_rank,
            ts_rank_cd, websearch_to_tsquery,
        };
    }

    /// JSON and JSONB helper functions.
    pub mod json {
        pub use crate::typed::{
            json, json_exists, json_get, json_get_text, json_path, json_path_text, json_query,
            json_scalar, json_serialize, json_value, jsonb_agg_object, jsonb_array_elements,
            jsonb_build_array, jsonb_build_object, jsonb_delete, jsonb_each, jsonb_insert,
            jsonb_object, jsonb_path_exists, jsonb_path_query, jsonb_set, jsonb_strip_nulls,
            jsonb_typeof, to_json, to_jsonb,
        };
    }

    /// Math helper functions.
    pub mod math {
        pub use crate::typed::{
            abs, cbrt, ceil, degrees, div, exp, factorial, floor, gamma, gcd, lcm, lgamma, ln, log,
            mod_, pi, pow, power, radians, random, random_between, round, sign, sqrt, trunc,
        };
    }

    /// Scalar helper functions.
    pub mod scalar {
        pub use crate::typed::{case, coalesce, greatest, least, nullif, scalar_subquery};
    }

    /// Text helper functions.
    pub mod text {
        pub use crate::typed::{
            btrim, casefold, char_length, concat, concat_op, concat_ws, left, length, lower, lpad,
            ltrim, md5, normalize, normalize_form, not_similar_to, regexp_matches, regexp_replace,
            regexp_split_to_array, replace, reverse, right, rpad, rtrim, similar_to, split_part,
            strpos, substring, text_starts_with, trim, unicode_assigned, upper,
        };
    }

    /// UUID helper functions.
    pub mod uuid {
        pub use crate::typed::{
            gen_random_uuid, uuid_extract_timestamp, uuid_extract_version, uuidv4, uuidv7,
            uuidv7_shift,
        };
    }

    /// Window helper functions and frame constructors.
    pub mod window {
        pub use crate::typed::{
            cume_dist, current_row, dense_rank, first_value, following, groups, lag, last_value,
            lead, nth_value, ntile, partition_by, percent_rank, preceding, range, rank, row_number,
            rows, unbounded_following, unbounded_preceding, window,
        };
    }

    pub use crate::typed::{
        abs, age, aggregate, and, any_value, array, array_agg, array_agg_distinct, array_append,
        array_cat, array_dims, array_fill, array_fill_with_lower_bounds, array_length, array_lower,
        array_ndims, array_position, array_positions, array_prepend, array_remove, array_replace,
        array_reverse, array_sample, array_shuffle, array_sort, array_sort_desc, array_sort_with,
        array_to_string, array_upper, avg, bit_and, bit_or, bit_xor, bool_and, bool_or, btrim,
        cardinality, case, casefold, cbrt, ceil, char_length, coalesce, concat, concat_op,
        concat_ws, count, count_all, count_distinct, crc32, crc32c, cume_dist, current_date,
        current_row, current_timestamp, date_trunc, degrees, dense_rank, div, every, exists, exp,
        extract, factorial, false_, first_value, floor, following, function, gamma, gcd,
        gen_random_uuid, greatest, grouping, groups, json, json_agg, json_agg_strict, json_exists,
        json_get, json_get_text, json_object_agg, json_object_agg_strict, json_object_agg_unique,
        json_object_agg_unique_strict, json_path, json_path_text, json_query, json_scalar,
        json_serialize, json_value, jsonb_agg, jsonb_agg_object, jsonb_agg_strict,
        jsonb_array_elements, jsonb_build_array, jsonb_build_object, jsonb_delete, jsonb_each,
        jsonb_insert, jsonb_object, jsonb_object_agg, jsonb_object_agg_strict,
        jsonb_object_agg_unique, jsonb_object_agg_unique_strict, jsonb_path_exists,
        jsonb_path_query, jsonb_set, jsonb_strip_nulls, jsonb_typeof, lag, last_value, lcm, lead,
        least, left, length, lgamma, ln, log, lower, lpad, ltrim, make_date, make_time,
        make_timestamp, make_timestamptz, max, md5, merge_action, min, mod_, mode, normalize,
        normalize_form, not, not_similar_to, now, nth_value, ntile, nullif, or,
        ordered_set_aggregate, param, partition_by, percent_rank, percentile_cont, percentile_disc,
        pi, plainto_tsquery, pow, power, preceding, radians, random, random_between, range,
        range_agg, range_intersect_agg, rank, regexp_matches, regexp_replace,
        regexp_split_to_array, replace, reverse, right, round, row, row_number, rows, rpad, rtrim,
        scalar_subquery, sign, similar_to, slice, split_part, sqrt, stddev, stddev_pop,
        stddev_samp, string_agg, string_to_array, strpos, subscript, substring, sum,
        text_starts_with, timezone, to_json, to_jsonb, to_tsquery, to_tsvector, to_tsvector_config,
        trim, true_, trunc, ts_match, ts_rank, ts_rank_cd, unbounded_following,
        unbounded_preceding, unicode_assigned, unnest, upper, uuid_extract_timestamp,
        uuid_extract_version, uuidv4, uuidv7, uuidv7_shift, var_pop, var_samp, variance,
        websearch_to_tsquery, window,
    };
}

/// Common imports for application query code.
///
/// The prelude intentionally excludes broad SQL function names such as `lower`
/// or `replace`; import those from [`dsl`] only where needed.
pub mod prelude {
    pub use crate::{
        Assignment, BindValue, BoolExpr, BoolOp, BooleanTest, BuiltQuery, CaseBuilder, Changeset,
        ColumnConflictBuilder, ConflictAction, ConflictClause, ConflictFields, ConflictTarget,
        ConstraintConflictBuilder, Cte, CteMaterialization, DbErrorInfo, DbErrorPosition, Delete,
        Error, FetchClause, Field, FieldRef, FrameBound, FrameExclude, GroupByItem, Insert,
        Insertable, IntoFieldMetas, IntoFieldRef, IntoSelectItems, Join, JoinKind, JsonKind,
        LockMode, LockWait, MatchedMergeBuilder, Merge, MergeAction, MergeWhen, Meta,
        NotMatchedBySourceMergeBuilder, NotMatchedMergeBuilder, NullsPosition,
        OffsetWindowFunctionBuilder, OpSet, OrderDirection, OrderItem, Param, Params, PgConnection,
        PgExecutor, PgPool, RawStmt, Result, RowLock, ScalarValue, SearchFilter, SearchOperator,
        SearchPredicate, SearchRequest, SearchSort, Select, SelectItem, SetOperator, SetQuery,
        SortDirection, Source, Stmt, Update, ValueExpr, ValueOp, WindowFrame, WindowFrameKind,
        WindowFunction, WindowFunctionBuilder, WindowSpec, cte, cte_ref, delete_from, except,
        except_all, field, function_source, insert, intersect, intersect_all, jsonb_agg_object,
        merge_into, raw, raw_source, schema, select, subquery, table, tx, union, union_all, update,
        view,
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
            Error::InvalidOperator { field, operator }
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

    #[test]
    fn dsl_helpers_are_available_outside_the_core_prelude() {
        static EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::ordered());
        static FIELDS: [&Meta; 1] = [&EMAIL_META];
        const EMAIL: Field<String> = Field::new(&EMAIL_META);

        let built = crate::select(crate::table("public.users", &FIELDS))
            .item(crate::dsl::lower(EMAIL).alias("lower_email"))
            .filter(crate::dsl::and([EMAIL.ilike("%@example.com")]))
            .build()
            .unwrap();

        assert_eq!(
            built.sql,
            "SELECT lower(\"email\") AS \"lower_email\" FROM \"public\".\"users\" WHERE (\"email\" ILIKE $1)"
        );
    }

    #[test]
    fn computed_field_macro_creates_static_metadata_once_per_site() {
        let item_count = crate::field!("item_count": int8 => i64, ordered);

        assert_eq!(item_count.meta.api, "item_count");
        assert_eq!(item_count.meta.pg, "int8");
        assert!(item_count.meta.ops.ordering);

        let source = crate::raw_source(
            "SELECT ?::int8 AS item_count",
            "counts",
            vec![crate::Param::typed(1_i64)],
            item_count,
        );
        let built = crate::select(source)
            .column(item_count.at("counts"))
            .build()
            .unwrap();

        assert_eq!(
            built.sql,
            "SELECT \"counts\".\"item_count\" AS \"counts_item_count\" FROM (SELECT $1::int8 AS item_count) AS \"counts\" (\"item_count\")"
        );
    }

    #[test]
    fn facade_exports_schema_macro_runtime_paths() {
        let source = crate::table("public.users", &[]);
        let assignment = crate::Assignment {
            field: Meta::col("id", "int4"),
            value: crate::ValueExpr::from(1_i32),
        };

        assert!(matches!(source, crate::Source::Table { .. }));
        assert_eq!(assignment.field.db, "id");
    }
}
