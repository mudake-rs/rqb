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
pub use typed::Changeset;
pub use typed::{
    Assignment, BoolExpr, BoolOp, BuiltQuery, ConflictAction, ConflictClause, ConflictTarget, Cte,
    Delete, Field, FieldRef, Insert, InsertConflictBuilder, Insertable, IntoFieldRef, Join,
    JoinKind, JsonKind, LockMode, LockWait, Meta, OffsetWindowFunctionBuilder, OpSet,
    OrderDirection, OrderItem, Param, Params, RawStmt, RowLock, SearchFilter, SearchOperator,
    SearchPredicate, SearchRequest, SearchSort, Select, SelectItem, SetOperator, SetQuery,
    SortDirection, Source, Stmt, Update, ValueExpr, ValueOp, WindowFunction, WindowFunctionBuilder,
    WindowSpec, aggregate, array_agg, array_agg_distinct, avg, count, count_all, count_distinct,
    cte, cte_source, delete_from, dense_rank, except, except_all, insert, intersect, intersect_all,
    json_agg, lag, lead, max, min, partition_by, rank, raw, raw_source, row_number, select,
    string_agg, subquery, sum, table, union, union_all, update, view, window,
};
pub use uuid;

pub type Result<T> = std::result::Result<T, Error>;

pub mod prelude {
    pub use crate::{
        Assignment, BoolExpr, BoolOp, BuiltQuery, Changeset, ConflictAction, ConflictClause,
        ConflictTarget, Cte, DbErrorInfo, DbErrorPosition, Delete, Error, Field, FieldRef, Insert,
        InsertConflictBuilder, Insertable, IntoFieldRef, Join, JoinKind, JsonKind, LockMode,
        LockWait, Meta, OffsetWindowFunctionBuilder, OpSet, OrderDirection, OrderItem, Param,
        Params, PgConnection, PgExecutor, PgPool, RawStmt, Result, RowLock, SearchFilter,
        SearchOperator, SearchPredicate, SearchRequest, SearchSort, Select, SelectItem,
        SetOperator, SetQuery, SortDirection, Source, Stmt, Update, ValueExpr, ValueOp,
        WindowFunction, WindowFunctionBuilder, WindowSpec, aggregate, array_agg,
        array_agg_distinct, avg, count, count_all, count_distinct, cte, cte_source, delete_from,
        dense_rank, except, except_all, insert, intersect, intersect_all, json_agg, lag, lead, max,
        min, partition_by, rank, raw, raw_source, row_number, schema, select, string_agg, subquery,
        sum, table, tx, union, union_all, update, view, window,
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
