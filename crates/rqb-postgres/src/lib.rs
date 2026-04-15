//! sqlx-first Postgres query builder runtime for rqb.
//!
//! This crate owns the typed Postgres AST, SQL rendering, typed bind
//! parameters, and execution through `sqlx::Executor`.

#![allow(clippy::result_large_err)]

mod error;
pub mod typed;

pub use error::{DbErrorInfo, DbErrorPosition, Error};
pub use typed::{
    Assignment, BoolExpr, BoolOp, BuiltQuery, Delete, ErasedParam, Field, Insert, JsonKind, Meta,
    OpSet, OrderDirection, OrderItem, Param, Params, RawStmt, SearchFilter, SearchOperator,
    SearchPredicate, SearchRequest, SearchSort, Select, SelectItem, SortDirection, Source, Stmt,
    Update, ValueExpr, ValueOp, delete_from, insert, raw, select, table, update, view,
};
pub type Result<T> = std::result::Result<T, Error>;
