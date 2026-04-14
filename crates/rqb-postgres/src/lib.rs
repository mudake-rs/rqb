//! Postgres renderer and optional runtime execution for rqb query models.
//!
//! Without runtime features this crate can render `SelectQuery`, `InsertQuery`,
//! `UpdateQuery`, and `DeleteQuery` into parameterized Postgres SQL. With
//! `runtime-tokio-postgres` and `pool`, it also provides execution traits,
//! row-to-serde mapping, pooled `Db`, transactions, and savepoints.

#![allow(clippy::result_large_err)]

mod bind;
mod build;
mod built;
mod error;
#[cfg(feature = "runtime-tokio-postgres")]
mod executor;
mod helpers;
#[cfg(feature = "runtime-tokio-postgres")]
mod params;
#[cfg(feature = "pool")]
mod pool;
mod render;
#[cfg(feature = "runtime-tokio-postgres")]
mod result_ext;
#[cfg(feature = "runtime-tokio-postgres")]
mod row_map;
#[cfg(test)]
mod tests;
mod type_sql;

pub use bind::{BindParam, BindType};
pub use build::{BuildPostgres, BuildRowsPostgres, Postgres};
pub use built::{BuiltQuery, BuiltSelect, DebugSelectSql, DebugSql};
pub use error::Error;
#[cfg(feature = "runtime-tokio-postgres")]
pub use error::{DbErrorInfo, DbErrorPosition};
#[cfg(feature = "runtime-tokio-postgres")]
pub use executor::{
    ExecutePostgres, ExecuteRawPostgres, ExecuteWritePostgres, Page, PgExecutor, StatementCache,
};
#[cfg(feature = "runtime-tokio-postgres")]
pub use params::PgParams;
#[cfg(feature = "pool")]
pub use pool::{
    BeginBuilder, Db, IsolationLevel, Savepoint, Tx, TxFuture, connect, connect_with_tls,
};
#[cfg(feature = "runtime-tokio-postgres")]
pub use result_ext::ResultExt;
#[cfg(feature = "runtime-tokio-postgres")]
pub use row_map::{raw_row_to_json, row_to_json};

pub type Result<T> = std::result::Result<T, Error>;
