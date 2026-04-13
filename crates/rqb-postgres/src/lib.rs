//! Postgres renderer and optional runtime execution for rqb query models.
//!
//! Without runtime features this crate can render `SelectQuery`, `InsertQuery`,
//! `UpdateQuery`, and `DeleteQuery` into parameterized Postgres SQL. With
//! `runtime-tokio-postgres` and `pool`, it also provides execution traits,
//! row-to-serde mapping, pooled `Db`, transactions, and savepoints.

#![allow(clippy::result_large_err)]

use std::fmt;

use rqb_core::{
    DeleteBuilder, DeleteQuery, Error as CoreError, InsertBuilder, InsertQuery, SelectBuilder,
    SelectColumn, SelectQuery, UpdateBuilder, UpdateQuery, ValidatedDelete, ValidatedInsert,
    ValidatedSelect, ValidatedUpdate, Value,
};
use thiserror::Error;

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

use render::Renderer;

#[cfg(feature = "runtime-tokio-postgres")]
pub use executor::{ExecutePostgres, ExecuteWritePostgres, Page, PgExecutor};
#[cfg(feature = "runtime-tokio-postgres")]
pub use params::PgParams;
#[cfg(feature = "pool")]
pub use pool::{
    BeginBuilder, Db, IsolationLevel, Savepoint, Tx, TxFuture, connect, connect_with_tls,
};
#[cfg(feature = "runtime-tokio-postgres")]
pub use result_ext::ResultExt;
#[cfg(feature = "runtime-tokio-postgres")]
pub use row_map::row_to_json;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] CoreError),

    #[error("raw SQL fragment has too few bind values")]
    TooFewRawBinds,

    #[error("raw SQL fragment has unused bind values")]
    UnusedRawBinds,

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("query returned no rows")]
    NotFound,

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("unique violation{}", constraint_suffix(.constraint))]
    UniqueViolation {
        constraint: Option<String>,
        detail: Option<String>,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("foreign key violation{}", constraint_suffix(.constraint))]
    ForeignKeyViolation {
        constraint: Option<String>,
        detail: Option<String>,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("not null violation{}", column_suffix(.column))]
    NotNullViolation { column: Option<String> },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("check violation{}", constraint_suffix(.constraint))]
    CheckViolation { constraint: Option<String> },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("exclusion violation{}", constraint_suffix(.constraint))]
    ExclusionViolation {
        constraint: Option<String>,
        detail: Option<String>,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("database error ({code}): {message}")]
    Database {
        code: String,
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        constraint: Option<String>,
        table: Option<String>,
        column: Option<String>,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("connection error: {0}")]
    Connection(String),

    #[cfg(feature = "pool")]
    #[error("pool error: {0}")]
    Pool(String),

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("deserialization error: {0}")]
    Deserialize(String),
}

#[cfg(feature = "runtime-tokio-postgres")]
fn constraint_suffix(constraint: &Option<String>) -> String {
    constraint
        .as_ref()
        .map(|name| format!(" on constraint \"{name}\""))
        .unwrap_or_default()
}

#[cfg(feature = "runtime-tokio-postgres")]
fn column_suffix(column: &Option<String>) -> String {
    column
        .as_ref()
        .map(|name| format!(" on column \"{name}\""))
        .unwrap_or_default()
}

#[cfg(feature = "runtime-tokio-postgres")]
impl From<tokio_postgres::Error> for Error {
    fn from(error: tokio_postgres::Error) -> Self {
        use tokio_postgres::error::SqlState;

        let Some(db) = error.as_db_error() else {
            return Self::Connection(error.to_string());
        };

        let code = db.code();
        let constraint = db.constraint().map(ToOwned::to_owned);
        let detail = db.detail().map(ToOwned::to_owned);
        let column = db.column().map(ToOwned::to_owned);

        if *code == SqlState::UNIQUE_VIOLATION {
            return Self::UniqueViolation { constraint, detail };
        }
        if *code == SqlState::FOREIGN_KEY_VIOLATION {
            return Self::ForeignKeyViolation { constraint, detail };
        }
        if *code == SqlState::NOT_NULL_VIOLATION {
            return Self::NotNullViolation { column };
        }
        if *code == SqlState::CHECK_VIOLATION {
            return Self::CheckViolation { constraint };
        }
        if *code == SqlState::EXCLUSION_VIOLATION {
            return Self::ExclusionViolation { constraint, detail };
        }

        Self::Database {
            code: code.code().to_owned(),
            message: db.message().to_owned(),
            detail,
            hint: db.hint().map(ToOwned::to_owned),
            constraint,
            table: db.table().map(ToOwned::to_owned),
            column,
        }
    }
}

#[cfg(feature = "runtime-tokio-postgres")]
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Deserialize(error.to_string())
    }
}

impl Error {
    pub fn as_core(&self) -> Option<&CoreError> {
        match self {
            Self::Core(error) => Some(error),
            _ => None,
        }
    }

    pub fn is_core(&self) -> bool {
        self.as_core().is_some()
    }
}

#[cfg(feature = "runtime-tokio-postgres")]
impl Error {
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound)
    }

    pub fn is_unique_violation(&self) -> bool {
        matches!(self, Self::UniqueViolation { .. })
    }

    pub fn is_foreign_key_violation(&self) -> bool {
        matches!(self, Self::ForeignKeyViolation { .. })
    }

    pub fn is_not_null_violation(&self) -> bool {
        matches!(self, Self::NotNullViolation { .. })
    }

    pub fn is_check_violation(&self) -> bool {
        matches!(self, Self::CheckViolation { .. })
    }

    pub fn is_constraint(&self, name: &str) -> bool {
        self.constraint_name() == Some(name)
    }

    pub fn is_connection(&self) -> bool {
        matches!(self, Self::Connection(_))
    }

    pub fn constraint_name(&self) -> Option<&str> {
        match self {
            Self::UniqueViolation { constraint, .. }
            | Self::ForeignKeyViolation { constraint, .. }
            | Self::CheckViolation { constraint }
            | Self::ExclusionViolation { constraint, .. }
            | Self::Database { constraint, .. } => constraint.as_deref(),
            _ => None,
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::UniqueViolation { detail, .. }
            | Self::ForeignKeyViolation { detail, .. }
            | Self::ExclusionViolation { detail, .. }
            | Self::Database { detail, .. } => detail.as_deref(),
            _ => None,
        }
    }
}

/// A single rendered Postgres statement with collected bind parameters.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct BuiltQuery {
    pub sql: String,
    pub params: Vec<Value>,
    pub columns: Vec<SelectColumn>,
    pub cacheable: bool,
}

impl BuiltQuery {
    pub fn debug_sql(&self) -> DebugSql<'_> {
        DebugSql { query: self }
    }
}

#[must_use]
pub struct DebugSql<'a> {
    query: &'a BuiltQuery,
}

impl fmt::Display for DebugSql<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.query.sql)?;
        write!(f, "-- params: {:?}", self.query.params)
    }
}

#[cfg(feature = "runtime-tokio-postgres")]
impl BuiltQuery {
    pub fn params(&self) -> PgParams {
        PgParams::from_values(&self.params)
    }
}

/// Rendered SELECT statements for page-style execution.
///
/// rqb renders both the rows query and matching count query for a select,
/// because the same builder powers list endpoints and JSON search APIs.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct BuiltSelect {
    pub rows: BuiltQuery,
    pub count: BuiltQuery,
}

impl BuiltSelect {
    pub fn debug_sql(&self) -> DebugSelectSql<'_> {
        DebugSelectSql { select: self }
    }
}

#[must_use]
pub struct DebugSelectSql<'a> {
    select: &'a BuiltSelect,
}

impl fmt::Display for DebugSelectSql<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "-- rows")?;
        writeln!(f, "{}", self.select.rows.debug_sql())?;
        writeln!(f)?;
        writeln!(f, "-- count")?;
        write!(f, "{}", self.select.count.debug_sql())
    }
}

pub struct Postgres;

impl Postgres {
    pub fn build(query: SelectQuery) -> Result<BuiltSelect> {
        let (built, _, _) = Self::build_page(query)?;
        Ok(built)
    }

    pub(crate) fn build_page(query: SelectQuery) -> Result<(BuiltSelect, u32, u64)> {
        let validated = ValidatedSelect::new(query)?;
        let limit = validated.limit;
        let offset = validated.offset;
        let built = BuiltSelect {
            rows: Renderer::new().render_rows(&validated)?,
            count: Renderer::new().render_count(&validated)?,
        };
        Ok((built, limit, offset))
    }

    pub fn build_rows(query: SelectQuery) -> Result<BuiltQuery> {
        let validated = ValidatedSelect::new(query)?;
        Renderer::new().render_rows(&validated)
    }

    pub fn build_insert(query: InsertQuery) -> Result<BuiltQuery> {
        let validated = ValidatedInsert::new(query)?;
        Renderer::new().render_insert(&validated)
    }

    pub fn build_update(query: UpdateQuery) -> Result<BuiltQuery> {
        let validated = ValidatedUpdate::new(query)?;
        Renderer::new().render_update(&validated)
    }

    pub fn build_delete(query: DeleteQuery) -> Result<BuiltQuery> {
        let validated = ValidatedDelete::new(query)?;
        Renderer::new().render_delete(&validated)
    }
}

pub trait BuildPostgres {
    type Output;

    fn build_pg(self) -> Result<Self::Output>;
}

impl BuildPostgres for SelectQuery {
    type Output = BuiltSelect;

    fn build_pg(self) -> Result<Self::Output> {
        Postgres::build(self)
    }
}

pub trait BuildRowsPostgres {
    fn build_rows_pg(self) -> Result<BuiltQuery>;
}

impl BuildRowsPostgres for SelectQuery {
    fn build_rows_pg(self) -> Result<BuiltQuery> {
        Postgres::build_rows(self)
    }
}

impl BuildPostgres for SelectBuilder {
    type Output = BuiltSelect;

    fn build_pg(self) -> Result<Self::Output> {
        self.build().build_pg()
    }
}

impl BuildRowsPostgres for SelectBuilder {
    fn build_rows_pg(self) -> Result<BuiltQuery> {
        self.build().build_rows_pg()
    }
}

impl BuildPostgres for InsertQuery {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        Postgres::build_insert(self)
    }
}

impl BuildPostgres for InsertBuilder {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        self.build()?.build_pg()
    }
}

impl BuildPostgres for UpdateQuery {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        Postgres::build_update(self)
    }
}

impl BuildPostgres for UpdateBuilder {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        self.build()?.build_pg()
    }
}

impl BuildPostgres for DeleteQuery {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        Postgres::build_delete(self)
    }
}

impl BuildPostgres for DeleteBuilder {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        self.build().build_pg()
    }
}
