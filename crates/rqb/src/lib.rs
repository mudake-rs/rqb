//! Public facade for rqb.
//!
//! Use `rqb::prelude::*` in application code. It exports the query builders,
//! field/dataset metadata types, Postgres execution traits, and pool helpers
//! enabled by feature flags.
//!
//! The usual flow is:
//!
//! 1. Define or generate `Field` constants and `Dataset` functions.
//! 2. Build trusted query shape in Rust with `select`, `insert`, `update`, or `delete`.
//! 3. Optionally merge a JSON `SearchRequest` with `.request(request)`.
//! 4. Render with `build_pg()` or execute with `fetch_*`/`execute`.
//!
//! See the repository `README.md`, `docs/guide.md`, `docs/recipes.md`, and
//! `docs/ergonomics.md` for end-to-end examples.

pub use rqb_core::{
    Aggregate, AggregateType, Capabilities, CaseBranch, CaseBuilder, CaseThenBuilder,
    ColumnOperator, ColumnPredicate, ConflictAction, ConflictClause, ConflictTarget, Cte, CteBody,
    Dataset, DbEnum, DeleteBuilder, DeleteQuery, ElemType, EnumType, Error as CoreError,
    ExistsPredicate, Expr, Field, FieldRef, FieldType, FunctionBuilder, InsertBuilder,
    InsertConflictBuilder, InsertQuery, IntoFieldRefs, IntoSqlExpr, Join, JoinKind, JsonPathPolicy,
    LockMode, LockWait, LogicalExpr, LogicalOp, NullsOrder, Operator, Predicate, RawQuery, RawSql,
    Relation, Result as CoreResult, ReturningMode, RowLock, SearchRequest, SelectBuilder,
    SelectColumn, SelectItem, SelectQuery, SelectRepr, Sort, SortDir, Source, SqlExpr,
    SubqueryOperator, SubqueryPredicate, TextSearchConfig, TypeFamily, TypeSpec, UpdateBuilder,
    UpdateQuery, Value, ValueRepr, WriteAssignment, WriteValue, all, any, array_agg, avg,
    case_when, cast, coalesce, count, count_distinct, count_field, cte, delete, excluded, exists,
    field, func, insert, json_agg, json_agg_nullable, max, max_agg, min, min_agg, not, not_exists,
    raw, raw_expr, raw_query, select, set, set_col, set_default, set_expr, set_raw, string_agg,
    sum, update,
};
pub use rqb_postgres as postgres;
#[cfg(feature = "pool")]
pub use rqb_postgres::{
    BeginBuilder, Db, IsolationLevel, Savepoint, Tx, TxFuture, connect, connect_with_tls,
};
pub use rqb_postgres::{
    BuildPostgres, BuildRowsPostgres, BuiltQuery, BuiltSelect, DebugSelectSql, DebugSql, Postgres,
};
pub use rqb_postgres::{Error, Result};
#[cfg(feature = "runtime-tokio-postgres")]
pub use rqb_postgres::{
    ExecutePostgres, ExecuteRawPostgres, ExecuteWritePostgres, Page, PgExecutor, PgParams,
    ResultExt, StatementCache,
};
pub use serde;

pub mod prelude {
    pub use rqb_core::prelude::*;
    pub use rqb_postgres::{
        BuildPostgres, BuildRowsPostgres, BuiltQuery, BuiltSelect, DebugSelectSql, DebugSql,
        Postgres,
    };

    #[cfg(feature = "runtime-tokio-postgres")]
    pub use rqb_postgres::{
        ExecutePostgres, ExecuteRawPostgres, ExecuteWritePostgres, Page, PgExecutor, PgParams,
        ResultExt, StatementCache,
    };

    #[cfg(feature = "pool")]
    pub use rqb_postgres::{
        BeginBuilder, Db, IsolationLevel, Savepoint, Tx, TxFuture, connect, connect_with_tls, txn,
    };
}

#[cfg(test)]
mod tests {
    use super::{CoreError, Error};

    #[test]
    fn facade_error_alias_points_at_postgres_error() {
        let error = Error::Core(CoreError::UnknownField {
            dataset: "orders".to_owned(),
            field: "missing".to_owned(),
        });

        assert!(matches!(error, Error::Core(CoreError::UnknownField { .. })));
    }
}
