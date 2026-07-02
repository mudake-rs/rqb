//! SQL-first Postgres query builder for Rust services.
//!
//! rqb is not an ORM. Application code owns SQL shape, rqb validates the typed
//! AST before rendering, and values are passed to Postgres as sqlx bind
//! arguments.
//! It is not a complete compile-time SQL type system: typed fields and bind
//! values make common service queries hard to mix up, but operator legality,
//! statement shape, and raw bind counts are runtime validation checks that fail
//! at `.build()?`.
//!
//! # Basic shape
//!
//! Start with schema metadata. In applications this usually comes from
//! `rqb-cli` and the [`schema!`] macro; small examples can write the macro
//! directly.
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
//! `select(table())` projects known metadata fields, not `SELECT *`. Joined
//! fields and computed expressions stay explicit.
//!
//! # Imports
//!
//! [`prelude`] contains the builders, core AST types, schema support, execution
//! traits, and structured errors that normal service modules use. Broad SQL
//! helper names such as `lower`, `left`, `array`, `row`, and `replace` are
//! available at the crate root for qualified calls, but are intentionally kept
//! out of [`prelude`]. The [`Result`] alias is also kept qualified as
//! `rqb::Result` so application entry points can keep using their normal error
//! type. Import SQL helpers from [`dsl`] when a module wants short names.
//!
//! ```rust,ignore
//! use rqb::prelude::*;
//! use rqb::dsl::{count_all, date_trunc, sum};
//! ```
//!
//! # Execution
//!
//! Statements build parameterized Postgres SQL and execute through sqlx
//! executors:
//!
//! ```rust,ignore
//! #[derive(sqlx::FromRow)]
//! struct UserRow {
//!     id: uuid::Uuid,
//!     email: String,
//! }
//!
//! let users = rqb::select(users::table())
//!     .columns((users::ID, users::EMAIL))
//!     .filter(users::ACTIVE.eq(true))
//!     .fetch_all_as::<UserRow>(&pool)
//!     .await?;
//! ```
//!
//! Service functions can accept `&PgPool` by default. Reuse query-shape helper
//! functions when the same statement should run inside a transaction. Use
//! `impl PgExecutor<'_>` for small helpers that should execute directly from
//! both pool-backed code and transaction code. Pool-owned streaming helpers are
//! available for HTTP response streams where the returned stream must own the
//! built query.
//!
//! # JSON search boundary
//!
//! [`SearchRequest`] lets client JSON control filters, sort, limit, and offset
//! only over fields exposed with [`Meta::json`]. It cannot define tables, joins,
//! raw SQL, CTEs, writes, subqueries, or projections. Use
//! [`Select::apply_search`] to preserve tenant, permission, and other
//! server-owned filters. When a request should own the whole search clause,
//! start from a fresh `select(...)` and call [`Select::apply_search`] there.
//! LIKE and regex-style search operators require text-pattern field capability
//! and reject very long client patterns before SQL is rendered; still set
//! statement timeouts at HTTP or job boundaries for public search endpoints.
//!
//! # Writes and raw SQL
//!
//! Writes use field assignments or `#[derive(Insertable)]` /
//! `#[derive(Changeset)]`; there is no serde JSON write bridge. Raw SQL is still
//! server-owned and stays parameterized through [`raw`], [`raw_expr`],
//! [`raw_predicate`], and [`raw_source`]. Raw placeholders are SQL-aware, but
//! any raw fragment disables persistent prepared-statement caching for the whole
//! built query.

mod advisory;
mod built;
mod error;
mod execute;
mod expr;
mod ident;
mod meta;
mod param;
mod raw;
mod render;
mod request;
mod source;
mod stmt;
mod summary;
mod tx;

extern crate self as rqb;

pub use advisory::AdvisoryLockKey;
pub use built::BuiltQuery;
pub use error::{
    ColumnError, ConstraintError, CteShapeError, DatabaseFailure, DbErrorInfo, DbErrorPosition,
    Error, OperatorError, PgFailure, SearchValueError, WriteTargetError,
};
pub use execute::ScalarValue;
pub use expr::{
    __jsonb_agg_object_from_pairs, __jsonb_object_pair, BoolExpr, BoolOp, BooleanTest, CaseBuilder,
    DatePart, Field, FieldRef, FrameBound, FrameExclude, IntoFieldRef, IntoRowValues,
    JsonbObjectItem, OffsetWindowFunctionBuilder, ValueExpr, ValueOp, WindowFrame, WindowFrameKind,
    WindowFunction, WindowFunctionBuilder, WindowSpec, abs, age, aggregate, and, any_value, array,
    array_agg, array_agg_distinct, array_append, array_cat, array_dims, array_fill,
    array_fill_with_lower_bounds, array_length, array_lower, array_ndims, array_position,
    array_positions, array_prepend, array_remove, array_replace, array_reverse, array_sample,
    array_shuffle, array_sort, array_sort_desc, array_sort_with, array_to_json, array_to_string,
    array_upper, ascii, avg, avg_distinct, bit_and, bit_or, bit_xor, bool_and, bool_or, btrim,
    cardinality, case, casefold, cbrt, ceil, char_length, chr, coalesce, concat, concat_op,
    concat_ws, count, count_all, count_distinct, crc32, crc32c, cume_dist, current_database,
    current_date, current_row, current_schema, current_timestamp, current_user, date_bin,
    date_trunc, date_trunc_part, decode, degrees, dense_rank, div, encode, every, exists, exp,
    extract, factorial, false_, first_value, floor, following, format, function, gamma, gcd,
    gen_random_uuid, greatest, grouping, groups, initcap, isempty, isfinite, json, json_agg,
    json_agg_strict, json_array_length, json_build_array, json_build_object, json_exists, json_get,
    json_get_text, json_object, json_object_agg, json_object_agg_strict, json_object_agg_unique,
    json_object_agg_unique_strict, json_path, json_path_text, json_query, json_scalar,
    json_serialize, json_typeof, json_value, jsonb_agg, jsonb_agg_object, jsonb_agg_strict,
    jsonb_array_elements, jsonb_array_length, jsonb_build_array, jsonb_build_object, jsonb_delete,
    jsonb_each, jsonb_insert, jsonb_object, jsonb_object_agg, jsonb_object_agg_strict,
    jsonb_object_agg_unique, jsonb_object_agg_unique_strict, jsonb_path_exists, jsonb_path_query,
    jsonb_pretty, jsonb_set, jsonb_strip_nulls, jsonb_typeof, lag, last_value, lcm, lead, least,
    left, length, lgamma, literal, ln, log, lower, lower_inc, lower_inf, lpad, ltrim, make_date,
    make_time, make_timestamp, make_timestamptz, max, md5, merge_action, min, mod_, mode,
    multirange_merge, normalize, normalize_form, not, not_similar_to, now, nth_value, ntile, null,
    nullif, octet_length, or, ordered_set_aggregate, param, partition_by, percent_rank,
    percentile_cont, percentile_disc, phraseto_tsquery, phraseto_tsquery_config, pi,
    plainto_tsquery, pow, power, preceding, radians, random, random_between, range, range_agg,
    range_intersect_agg, range_lower, range_merge, range_upper, rank, raw_expr, raw_predicate,
    regexp_matches, regexp_replace, regexp_split_to_array, repeat, replace, reverse, right, round,
    row, row_number, row_to_json, rows, rpad, rtrim, scalar_subquery, session_user, sign,
    similar_to, slice, split_part, sqrt, starts_with, stddev, stddev_pop, stddev_samp, string_agg,
    string_to_array, strpos, subscript, substring, sum, sum_distinct, timezone, to_char, to_date,
    to_json, to_jsonb, to_number, to_timestamp, to_tsquery, to_tsvector, to_tsvector_config,
    translate, trim, trim_array, true_, trunc, ts_headline, ts_match, ts_rank, ts_rank_cd,
    unbounded_following, unbounded_preceding, unicode_assigned, unnest, upper, upper_inc,
    upper_inf, uuid_extract_timestamp, uuid_extract_version, uuidv4, uuidv7, uuidv7_shift, var_pop,
    var_samp, variance, version, websearch_to_tsquery, width_bucket, window,
};
pub use meta::{JsonKind, Meta, OpSet};
pub use param::{BindValue, Param, Params};
pub use request::{
    SearchFilter, SearchOperator, SearchPredicate, SearchRequest, SearchSort, SortDirection,
};
pub use rqb_macros::{Changeset, Insertable, schema};
pub use source::{
    Cte, FunctionSource, IntoFieldMetas, Source, cte, cte_ref, function_source,
    generate_series_source, generate_series_step_source, generate_subscripts_source,
    json_array_elements_source, json_each_source, json_object_keys_source,
    jsonb_array_elements_source, jsonb_each_source, jsonb_object_keys_source, raw_source,
    regexp_split_to_table_source, subquery, table, unnest_source, values_source, view,
};
pub use sqlx::{PgConnection, PgExecutor, PgPool};
pub use stmt::{
    Assignment, Changeset, ColumnConflictBuilder, ColumnList, ConflictFields,
    ConstraintConflictBuilder, Delete, Insert, Insertable, IntoColumn, IntoColumns, LockMode,
    MatchedMergeBuilder, Merge, NotMatchedBySourceMergeBuilder, NotMatchedMergeBuilder, OrderItem,
    RawStmt, Select, SetQuery, Stmt, Update, delete_from, except, except_all, insert, intersect,
    intersect_all, merge_into, raw, select, union, union_all, update,
};
pub(crate) use stmt::{
    AssignmentValue, ConflictAction, ConflictClause, ConflictTarget, GroupByItem, InsertBody,
    MergeAction, MergeWhen, OrderDirection, RowLimit, RowLock, SelectItem,
};

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
    (@build $name:literal, $pg:expr, $ty:ty, text) => {{
        static __RQB_FIELD_META: ::std::sync::OnceLock<$crate::Meta> =
            ::std::sync::OnceLock::new();
        $crate::Field::<$ty>::new(__RQB_FIELD_META.get_or_init(|| {
            $crate::Meta::col($name, $pg).ops($crate::OpSet::text())
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
        $crate::__jsonb_agg_object_from_pairs([
            $($crate::__jsonb_object_pair($item)),+
        ])
    }};
}

/// rqb result type.
pub type Result<T> = std::result::Result<T, Error>;

/// SQL expression and source helpers that are useful on demand but too broad
/// for the prelude.
///
/// Prefer importing only the helpers a query module needs:
///
/// ```rust,ignore
/// use rqb::dsl::{count_all, date_trunc, sum};
/// use rqb::prelude::*;
/// ```
pub mod dsl {
    pub use crate::advisory::{
        advisory_xact_lock, advisory_xact_lock_named, try_advisory_xact_lock,
        try_advisory_xact_lock_named,
    };
    pub use crate::{
        AdvisoryLockKey, DatePart, abs, age, aggregate, and, any_value, array, array_agg,
        array_agg_distinct, array_append, array_cat, array_dims, array_fill,
        array_fill_with_lower_bounds, array_length, array_lower, array_ndims, array_position,
        array_positions, array_prepend, array_remove, array_replace, array_reverse, array_sample,
        array_shuffle, array_sort, array_sort_desc, array_sort_with, array_to_json,
        array_to_string, array_upper, ascii, avg, avg_distinct, bit_and, bit_or, bit_xor, bool_and,
        bool_or, btrim, cardinality, case, casefold, cbrt, ceil, char_length, chr, coalesce,
        concat, concat_op, concat_ws, count, count_all, count_distinct, crc32, crc32c, cume_dist,
        current_database, current_date, current_row, current_schema, current_timestamp,
        current_user, date_bin, date_trunc, date_trunc_part, decode, degrees, dense_rank, div,
        encode, every, exists, exp, extract, factorial, false_, first_value, floor, following,
        format, function, gamma, gcd, gen_random_uuid, generate_series_source,
        generate_series_step_source, generate_subscripts_source, greatest, grouping, groups,
        initcap, isempty, isfinite, json, json_agg, json_agg_strict, json_array_elements_source,
        json_array_length, json_build_array, json_build_object, json_each_source, json_exists,
        json_get, json_get_text, json_object, json_object_agg, json_object_agg_strict,
        json_object_agg_unique, json_object_agg_unique_strict, json_object_keys_source, json_path,
        json_path_text, json_query, json_scalar, json_serialize, json_typeof, json_value,
        jsonb_agg, jsonb_agg_object, jsonb_agg_strict, jsonb_array_elements,
        jsonb_array_elements_source, jsonb_array_length, jsonb_build_array, jsonb_build_object,
        jsonb_delete, jsonb_each, jsonb_each_source, jsonb_insert, jsonb_object, jsonb_object_agg,
        jsonb_object_agg_strict, jsonb_object_agg_unique, jsonb_object_agg_unique_strict,
        jsonb_object_keys_source, jsonb_path_exists, jsonb_path_query, jsonb_pretty, jsonb_set,
        jsonb_strip_nulls, jsonb_typeof, lag, last_value, lcm, lead, least, left, length, lgamma,
        literal, ln, log, lower, lower_inc, lower_inf, lpad, ltrim, make_date, make_time,
        make_timestamp, make_timestamptz, max, md5, merge_action, min, mod_, mode,
        multirange_merge, normalize, normalize_form, not, not_similar_to, now, nth_value, ntile,
        null, nullif, octet_length, or, ordered_set_aggregate, param, partition_by, percent_rank,
        percentile_cont, percentile_disc, phraseto_tsquery, phraseto_tsquery_config, pi,
        plainto_tsquery, pow, power, preceding, radians, random, random_between, range, range_agg,
        range_intersect_agg, range_lower, range_merge, range_upper, rank, raw_expr, raw_predicate,
        regexp_matches, regexp_replace, regexp_split_to_array, regexp_split_to_table_source,
        repeat, replace, reverse, right, round, row, row_number, row_to_json, rows, rpad, rtrim,
        scalar_subquery, session_user, sign, similar_to, slice, split_part, sqrt, starts_with,
        stddev, stddev_pop, stddev_samp, string_agg, string_to_array, strpos, subscript, substring,
        sum, sum_distinct, timezone, to_char, to_date, to_json, to_jsonb, to_number, to_timestamp,
        to_tsquery, to_tsvector, to_tsvector_config, translate, trim, trim_array, true_, trunc,
        ts_headline, ts_match, ts_rank, ts_rank_cd, unbounded_following, unbounded_preceding,
        unicode_assigned, unnest, unnest_source, upper, upper_inc, upper_inf,
        uuid_extract_timestamp, uuid_extract_version, uuidv4, uuidv7, uuidv7_shift, values_source,
        var_pop, var_samp, variance, version, websearch_to_tsquery, width_bucket, window,
    };
}

/// Common imports for application query code.
///
/// The prelude intentionally excludes broad SQL function names such as `lower`
/// or `replace`; import those from [`dsl`] only where needed. It also excludes
/// [`Result`] so `use rqb::prelude::*` does not hijack application-level
/// `Result` signatures.
pub mod prelude {
    pub use crate::{
        Assignment, BindValue, BoolExpr, BuiltQuery, Changeset, Cte, Delete, Error, Field,
        FieldRef, Insert, Insertable, JsonKind, Merge, Meta, OpSet, Param, Params, PgConnection,
        PgExecutor, PgPool, RawStmt, SearchFilter, SearchOperator, SearchPredicate, SearchRequest,
        SearchSort, Select, SetQuery, SortDirection, Source, Stmt, Update, ValueExpr, and, cte,
        cte_ref, delete_from, except, except_all, field, insert, intersect, intersect_all,
        jsonb_agg_object, merge_into, not, or, raw, raw_expr, raw_predicate, raw_source, schema,
        select, subquery, table, tx, union, union_all, update, values_source, view,
    };
}

#[cfg(test)]
mod tests {
    use super::{Error, Field, Meta, OpSet, Result};

    #[derive(Clone)]
    struct PgU256;

    #[derive(Clone)]
    struct Vector;

    #[test]
    fn facade_exports_typed_field_and_error() {
        static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::equality());
        const ID: Field<i32> = Field::new(&ID_META);

        let error = ID.gt(10).validate().unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidOperator(err)
                if err.field == "id" && err.operator == "gt"
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
    fn tx_macro_type_checks_with_expression_body() {
        fn use_connection(_: &mut sqlx::PgConnection) -> Result<i32> {
            Ok(123)
        }

        fn assert_type_checks(pool: sqlx::PgPool) {
            let future = crate::tx!(&pool, |conn| use_connection(conn));
            drop(future);
        }

        let _ = assert_type_checks as fn(sqlx::PgPool);
    }

    #[test]
    fn dsl_helpers_are_available_outside_the_core_prelude() {
        static EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::text());
        static SPEND_META: Meta = Meta::new("spend", "spend", "int8").ops(OpSet::ordered());
        static FIELDS: [&Meta; 2] = [&EMAIL_META, &SPEND_META];
        const EMAIL: Field<String> = Field::new(&EMAIL_META);
        const SPEND: Field<i64> = Field::new(&SPEND_META);

        let built = crate::select(crate::table("public.users", &FIELDS))
            .expr_as(crate::dsl::lower(EMAIL), "lower_email")
            .expr_as(crate::dsl::sum_distinct(SPEND), "distinct_spend")
            .filter(crate::dsl::and([EMAIL.ilike("%@example.com")]))
            .build()
            .unwrap();

        assert_eq!(
            built.sql,
            "SELECT lower(\"email\") AS \"lower_email\", sum(DISTINCT \"spend\") AS \"distinct_spend\" FROM \"public\".\"users\" WHERE (\"email\" ILIKE $1)"
        );
    }

    #[test]
    fn computed_field_macro_creates_static_metadata_once_per_site() {
        let item_count = crate::field!("item_count": int8 => i64, ordered);
        let label = crate::field!("label": text => String, text);

        assert_eq!(item_count.meta.api, "item_count");
        assert_eq!(item_count.meta.pg, "int8");
        assert!(item_count.meta.ops.ordering);
        assert!(label.meta.ops.pattern);

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
            value: crate::AssignmentValue::Expr(crate::ValueExpr::from(1_i32)),
        };

        assert!(matches!(source, crate::Source::Table { .. }));
        assert_eq!(assignment.field.db, "id");
    }

    #[test]
    fn schema_macro_accepts_metadata_overrides_and_constraints() {
        crate::schema! {
            table public.wallets {
                id: uuid = uuid::Uuid,
                #[rqb(ops = ordered, json = text)]
                balance: "bitcoin.uint256" = crate::tests::PgU256,
                #[rqb(ops = none, json = none)]
                embedding: vector = crate::tests::Vector,
                #[rqb(ops = equality, json = text)]
                tags: "text[]" = Vec<String>,
                constraints {
                    WALLETS_PKEY: "wallets_pkey",
                    WALLETS_BALANCE_KEY: "wallets_balance_key",
                }
            }
        }

        assert!(wallets::BALANCE_META.ops.ordering);
        assert_eq!(wallets::BALANCE_META.json, Some(crate::JsonKind::Text));
        assert_eq!(wallets::EMBEDDING_META.ops, OpSet::none());
        assert_eq!(wallets::EMBEDDING_META.json, None);
        assert_eq!(wallets::TAGS_META.json, None);
        assert_eq!(wallets::constraints::WALLETS_PKEY, "wallets_pkey");
        assert_eq!(
            wallets::constraints::WALLETS_BALANCE_KEY,
            "wallets_balance_key"
        );
    }
}
