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
    Aggregate, AggregateType, Capabilities, ColumnOperator, ColumnPredicate, ConflictAction,
    ConflictClause, ConflictTarget, Cte, CteBody, Dataset, DbEnum, DeleteBuilder, DeleteQuery,
    ElemType, EnumType, Error, ExistsPredicate, Expr, Field, FieldRef, FieldType, InsertBuilder,
    InsertConflictBuilder, InsertQuery, IntoFieldRefs, Join, JoinKind, JsonPathPolicy, LockMode,
    LockWait, LogicalExpr, LogicalOp, NullsOrder, Operator, Predicate, RawQuery, RawSql, Relation,
    Result, ReturningMode, RowLock, SearchRequest, SelectBuilder, SelectColumn, SelectQuery,
    SelectRepr, Sort, SortDir, Source, SubqueryOperator, SubqueryPredicate, TextSearchConfig,
    TypeFamily, TypeSpec, UpdateBuilder, UpdateQuery, Value, ValueRepr, WriteAssignment,
    WriteValue, all, any, array_agg, avg, count, count_distinct, count_field, cte, delete, exists,
    field, insert, max, max_agg, min, min_agg, not, not_exists, raw, raw_query, select, string_agg,
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
