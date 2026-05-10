use sqlx::error::DatabaseError;
use sqlx::postgres::{PgDatabaseError, PgErrorPosition};
use thiserror::Error;

/// Structured metadata extracted from a Postgres database error.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DbErrorInfo {
    /// Schema name reported by Postgres.
    pub schema: Option<String>,
    /// Table name reported by Postgres.
    pub table: Option<String>,
    /// Column name reported by Postgres.
    pub column: Option<String>,
    /// Data type name reported by Postgres.
    pub datatype: Option<String>,
    /// Constraint name reported by Postgres.
    pub constraint: Option<String>,
    /// Postgres `WHERE` context attached to the error.
    pub where_: Option<String>,
    /// Error position in the original or internally rewritten query.
    pub position: Option<DbErrorPosition>,
}

/// Position metadata reported by Postgres for a database error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DbErrorPosition {
    /// Byte position in the original submitted SQL.
    Original(usize),
    /// Byte position in an internally generated query.
    Internal {
        /// Byte position in `query`.
        position: usize,
        /// Internal query text reported by Postgres.
        query: String,
    },
}

impl DbErrorInfo {
    fn from_sqlx_database_error(db: &(dyn DatabaseError + 'static)) -> Self {
        if let Some(pg) = db.try_downcast_ref::<PgDatabaseError>() {
            return Self::from_sqlx_pg_error(pg);
        }
        Self {
            table: db.table().map(ToOwned::to_owned),
            constraint: db.constraint().map(ToOwned::to_owned),
            ..Self::default()
        }
    }

    fn from_sqlx_pg_error(db: &PgDatabaseError) -> Self {
        Self {
            schema: db.schema().map(ToOwned::to_owned),
            table: db.table().map(ToOwned::to_owned),
            column: db.column().map(ToOwned::to_owned),
            datatype: db.data_type().map(ToOwned::to_owned),
            constraint: db.constraint().map(ToOwned::to_owned),
            where_: db.r#where().map(ToOwned::to_owned),
            position: db.position().map(DbErrorPosition::from),
        }
    }
}

impl From<PgErrorPosition<'_>> for DbErrorPosition {
    fn from(position: PgErrorPosition<'_>) -> Self {
        match position {
            PgErrorPosition::Original(position) => Self::Original(position),
            PgErrorPosition::Internal { position, query } => Self::Internal {
                position,
                query: query.to_owned(),
            },
        }
    }
}

/// Error type returned by rqb builders and sqlx execution helpers.
#[derive(Debug, Error)]
pub enum Error {
    /// A `fetch_one`-style operation did not return a row.
    #[error("query returned no rows")]
    NotFound,

    /// Postgres unique constraint violation (`23505`).
    #[error("unique violation{}", constraint_suffix(.constraint))]
    UniqueViolation {
        /// Constraint name when Postgres reported it.
        constraint: Option<String>,
        /// Postgres detail string.
        detail: Option<String>,
        /// Additional structured database error metadata.
        info: DbErrorInfo,
    },

    /// Postgres foreign key violation (`23503`).
    #[error("foreign key violation{}", constraint_suffix(.constraint))]
    ForeignKeyViolation {
        /// Constraint name when Postgres reported it.
        constraint: Option<String>,
        /// Postgres detail string.
        detail: Option<String>,
        /// Additional structured database error metadata.
        info: DbErrorInfo,
    },

    /// Postgres restrict violation (`23001`).
    #[error("restrict violation{}", constraint_suffix(.constraint))]
    RestrictViolation {
        /// Constraint name when Postgres reported it.
        constraint: Option<String>,
        /// Postgres detail string.
        detail: Option<String>,
        /// Additional structured database error metadata.
        info: DbErrorInfo,
    },

    /// Postgres not-null violation (`23502`).
    #[error("not null violation{}", column_suffix(.column))]
    NotNullViolation {
        /// Column name when Postgres reported it.
        column: Option<String>,
        /// Additional structured database error metadata.
        info: DbErrorInfo,
    },

    /// Postgres check constraint violation (`23514`).
    #[error("check violation{}", constraint_suffix(.constraint))]
    CheckViolation {
        /// Constraint name when Postgres reported it.
        constraint: Option<String>,
        /// Additional structured database error metadata.
        info: DbErrorInfo,
    },

    /// Postgres exclusion constraint violation (`23P01`).
    #[error("exclusion violation{}", constraint_suffix(.constraint))]
    ExclusionViolation {
        /// Constraint name when Postgres reported it.
        constraint: Option<String>,
        /// Postgres detail string.
        detail: Option<String>,
        /// Additional structured database error metadata.
        info: DbErrorInfo,
    },

    /// Postgres serialization failure (`40001`).
    #[error("serialization failure: {message}")]
    SerializationFailure {
        /// Postgres primary error message.
        message: String,
        /// Postgres detail string.
        detail: Option<String>,
        /// Postgres hint string.
        hint: Option<String>,
        /// Additional structured database error metadata.
        info: DbErrorInfo,
    },

    /// Postgres deadlock detected (`40P01`).
    #[error("deadlock detected: {message}")]
    DeadlockDetected {
        /// Postgres primary error message.
        message: String,
        /// Postgres detail string.
        detail: Option<String>,
        /// Postgres hint string.
        hint: Option<String>,
        /// Additional structured database error metadata.
        info: DbErrorInfo,
    },

    /// Postgres query cancellation (`57014`).
    #[error("query canceled: {message}")]
    QueryCanceled {
        /// Postgres primary error message.
        message: String,
        /// Postgres detail string.
        detail: Option<String>,
        /// Postgres hint string.
        hint: Option<String>,
        /// Additional structured database error metadata.
        info: DbErrorInfo,
    },

    /// Postgres insufficient privilege error (`42501`).
    #[error("insufficient privilege: {message}")]
    InsufficientPrivilege {
        /// Postgres primary error message.
        message: String,
        /// Postgres detail string.
        detail: Option<String>,
        /// Postgres hint string.
        hint: Option<String>,
        /// Table name when Postgres reported it.
        table: Option<String>,
        /// Column name when Postgres reported it.
        column: Option<String>,
        /// Additional structured database error metadata.
        info: DbErrorInfo,
    },

    /// A database error that is not mapped to a specialized variant.
    #[error("database error ({code}): {message}")]
    Database {
        /// SQLSTATE error code.
        code: String,
        /// Database primary error message.
        message: String,
        /// Database detail string.
        detail: Option<String>,
        /// Database hint string.
        hint: Option<String>,
        /// Constraint name when available.
        constraint: Option<String>,
        /// Table name when available.
        table: Option<String>,
        /// Column name when available.
        column: Option<String>,
        /// Additional structured database error metadata.
        info: DbErrorInfo,
    },

    /// Connection-level failure.
    #[error("connection error: {0}")]
    Connection(String),

    /// Unclassified sqlx error.
    #[error("sqlx error: {0}")]
    Sqlx(sqlx::Error),

    /// A transaction body failed and rollback also failed.
    #[error("transaction rollback failed after error: {error}; rollback error: {rollback}")]
    TransactionRollbackFailed {
        /// Original transaction body error.
        error: Box<Error>,
        /// Rollback error.
        rollback: Box<Error>,
    },

    /// Failed to encode a bound parameter into Postgres arguments.
    #[error("parameter encode error: {0}")]
    Encode(String),

    /// Raw SQL placeholder count does not match supplied bind values.
    #[error("raw SQL fragment has {placeholders} placeholders but {binds} bind values")]
    RawBindMismatch {
        /// Number of `?` placeholders in the raw SQL fragment.
        placeholders: usize,
        /// Number of bind values supplied for the fragment.
        binds: usize,
    },

    /// A field was used with an operator it does not support.
    #[error("operator `{operator}` is not supported for field `{field}`")]
    InvalidOperator {
        /// Field API name.
        field: String,
        /// Operator name.
        operator: String,
    },

    /// A field without ordering support was used for sorting.
    #[error("field `{field}` is not sortable")]
    InvalidSort {
        /// Field API name.
        field: String,
    },

    /// JSON search referenced a field that is not in the source metadata.
    #[error("unknown search field `{field}`")]
    InvalidSearchField {
        /// Field name from the JSON request.
        field: String,
    },

    /// JSON search referenced a field that is not exposed to JSON requests.
    #[error("search field `{field}` is not exposed to JSON requests")]
    SearchFieldNotExposed {
        /// Field name from the JSON request.
        field: String,
    },

    /// JSON search used an unsupported operator for a field.
    #[error("operator `{operator}` is not supported for search field `{field}`")]
    InvalidSearchOperator {
        /// Field name from the JSON request.
        field: String,
        /// Operator name from the JSON request.
        operator: String,
    },

    /// JSON search value did not match the field's expected JSON shape.
    #[error("invalid JSON value for search field `{field}`; expected {expected}")]
    InvalidSearchValue {
        /// Field name from the JSON request.
        field: String,
        /// Human-readable expected value kind.
        expected: &'static str,
    },

    /// JSON search used an empty `and` or `or` group.
    #[error("empty search logical expression `{logical}`")]
    EmptySearchLogical {
        /// Logical operator name.
        logical: &'static str,
    },

    /// Builder used an empty `AND` or `OR` group.
    #[error("empty logical expression `{logical}`")]
    EmptyLogical {
        /// Logical operator name.
        logical: String,
    },

    /// A row-value comparison used row expressions with different arity.
    #[error("row comparison requires the same arity, got {left} and {right}")]
    InvalidRowShape {
        /// Number of values in the left row expression.
        left: usize,
        /// Number of values in the right row expression.
        right: usize,
    },

    /// A write statement targeted a source kind that cannot be written to.
    #[error("{statement} target must be a table or view source, got {source_kind}")]
    InvalidWriteTarget {
        /// Statement kind.
        statement: &'static str,
        /// Source kind that was used as the write target.
        source_kind: &'static str,
    },

    /// A write statement had no assignments.
    #[error("{statement} statement requires at least one assignment")]
    EmptyAssignments {
        /// Statement kind.
        statement: &'static str,
    },

    /// A write statement had no target columns.
    #[error("{statement} statement requires at least one column")]
    EmptyColumns {
        /// Statement kind.
        statement: &'static str,
    },

    /// Insert builder state was not valid.
    #[error("invalid insert shape: {message}")]
    InvalidInsertShape {
        /// Validation message.
        message: &'static str,
    },

    /// Select builder state was not valid.
    #[error("invalid select shape: {message}")]
    InvalidSelectShape {
        /// Validation message.
        message: &'static str,
    },

    /// Merge builder state was not valid.
    #[error("invalid merge shape: {message}")]
    InvalidMergeShape {
        /// Validation message.
        message: &'static str,
    },

    /// CTE definition or exposed field list was not valid.
    #[error("invalid CTE `{name}`: {message}")]
    InvalidCteShape {
        /// CTE name.
        name: String,
        /// Validation message.
        message: &'static str,
    },

    /// A delete statement was built without a filter.
    #[error("delete without filter is not allowed")]
    DeleteWithoutFilter,

    /// A non-cross join was built without an `ON` condition.
    #[error("{join} requires an ON condition")]
    MissingJoinCondition {
        /// Join kind.
        join: &'static str,
    },
}

fn constraint_suffix(constraint: &Option<String>) -> String {
    constraint
        .as_ref()
        .map(|name| format!(" on constraint \"{name}\""))
        .unwrap_or_default()
}

fn column_suffix(column: &Option<String>) -> String {
    column
        .as_ref()
        .map(|name| format!(" on column \"{name}\""))
        .unwrap_or_default()
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(db) => Self::from_sqlx_database_error(&*db),
            other => Self::Sqlx(other),
        }
    }
}

impl Error {
    fn from_sqlx_database_error(db: &(dyn DatabaseError + 'static)) -> Self {
        let pg = db.try_downcast_ref::<PgDatabaseError>();
        let code = db
            .code()
            .map(|code| code.into_owned())
            .unwrap_or_else(|| "unknown".to_owned());
        let message = db.message().to_owned();
        let detail = pg.and_then(PgDatabaseError::detail).map(ToOwned::to_owned);
        let hint = pg.and_then(PgDatabaseError::hint).map(ToOwned::to_owned);
        let constraint = db.constraint().map(ToOwned::to_owned);
        let table = db.table().map(ToOwned::to_owned);
        let column = pg.and_then(PgDatabaseError::column).map(ToOwned::to_owned);
        let info = DbErrorInfo::from_sqlx_database_error(db);

        match code.as_str() {
            "23505" => Self::UniqueViolation {
                constraint,
                detail,
                info,
            },
            "23503" => Self::ForeignKeyViolation {
                constraint,
                detail,
                info,
            },
            "23001" => Self::RestrictViolation {
                constraint,
                detail,
                info,
            },
            "23502" => Self::NotNullViolation { column, info },
            "23514" => Self::CheckViolation { constraint, info },
            "23P01" => Self::ExclusionViolation {
                constraint,
                detail,
                info,
            },
            "40001" => Self::SerializationFailure {
                message,
                detail,
                hint,
                info,
            },
            "40P01" => Self::DeadlockDetected {
                message,
                detail,
                hint,
                info,
            },
            "57014" => Self::QueryCanceled {
                message,
                detail,
                hint,
                info,
            },
            "42501" => Self::InsufficientPrivilege {
                message,
                detail,
                hint,
                table,
                column,
                info,
            },
            _ => Self::Database {
                code,
                message,
                detail,
                hint,
                constraint,
                table,
                column,
                info,
            },
        }
    }

    /// Returns true for retryable transaction errors and connection failures.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::SerializationFailure { .. } | Self::DeadlockDetected { .. }
        ) || self.is_connection()
    }

    /// Returns true for connection-level failures.
    pub fn is_connection(&self) -> bool {
        matches!(self, Self::Connection(_))
    }

    /// Returns the SQLSTATE code when this error maps to one.
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::UniqueViolation { .. } => Some("23505"),
            Self::ForeignKeyViolation { .. } => Some("23503"),
            Self::RestrictViolation { .. } => Some("23001"),
            Self::NotNullViolation { .. } => Some("23502"),
            Self::CheckViolation { .. } => Some("23514"),
            Self::ExclusionViolation { .. } => Some("23P01"),
            Self::SerializationFailure { .. } => Some("40001"),
            Self::DeadlockDetected { .. } => Some("40P01"),
            Self::InsufficientPrivilege { .. } => Some("42501"),
            Self::QueryCanceled { .. } => Some("57014"),
            Self::Database { code, .. } => Some(code),
            _ => None,
        }
    }

    /// Returns the associated constraint name when available.
    pub fn constraint_name(&self) -> Option<&str> {
        match self {
            Self::UniqueViolation { constraint, .. }
            | Self::ForeignKeyViolation { constraint, .. }
            | Self::RestrictViolation { constraint, .. }
            | Self::CheckViolation { constraint, .. }
            | Self::ExclusionViolation { constraint, .. }
            | Self::Database { constraint, .. } => constraint.as_deref().or_else(|| {
                self.db_error_info()
                    .and_then(|info| info.constraint.as_deref())
            }),
            _ => self
                .db_error_info()
                .and_then(|info| info.constraint.as_deref()),
        }
    }

    /// Returns the associated table name when available.
    pub fn table_name(&self) -> Option<&str> {
        match self {
            Self::InsufficientPrivilege { table, .. } | Self::Database { table, .. } => table
                .as_deref()
                .or_else(|| self.db_error_info().and_then(|info| info.table.as_deref())),
            _ => self.db_error_info().and_then(|info| info.table.as_deref()),
        }
    }

    /// Returns the associated column name when available.
    pub fn column_name(&self) -> Option<&str> {
        match self {
            Self::NotNullViolation { column, .. }
            | Self::InsufficientPrivilege { column, .. }
            | Self::Database { column, .. } => column
                .as_deref()
                .or_else(|| self.db_error_info().and_then(|info| info.column.as_deref())),
            _ => self.db_error_info().and_then(|info| info.column.as_deref()),
        }
    }

    /// Returns the database detail message when available.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::UniqueViolation { detail, .. }
            | Self::ForeignKeyViolation { detail, .. }
            | Self::RestrictViolation { detail, .. }
            | Self::ExclusionViolation { detail, .. }
            | Self::SerializationFailure { detail, .. }
            | Self::DeadlockDetected { detail, .. }
            | Self::InsufficientPrivilege { detail, .. }
            | Self::QueryCanceled { detail, .. }
            | Self::Database { detail, .. } => detail.as_deref(),
            _ => None,
        }
    }

    /// Returns the database hint message when available.
    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::SerializationFailure { hint, .. }
            | Self::DeadlockDetected { hint, .. }
            | Self::InsufficientPrivilege { hint, .. }
            | Self::QueryCanceled { hint, .. }
            | Self::Database { hint, .. } => hint.as_deref(),
            _ => None,
        }
    }

    /// Returns the associated schema name when available.
    pub fn schema_name(&self) -> Option<&str> {
        self.db_error_info().and_then(|info| info.schema.as_deref())
    }

    /// Returns the associated data type name when available.
    pub fn datatype_name(&self) -> Option<&str> {
        self.db_error_info()
            .and_then(|info| info.datatype.as_deref())
    }

    /// Returns the Postgres `WHERE` error context when available.
    pub fn where_context(&self) -> Option<&str> {
        self.db_error_info().and_then(|info| info.where_.as_deref())
    }

    /// Returns the database error position when available.
    pub fn position(&self) -> Option<&DbErrorPosition> {
        self.db_error_info().and_then(|info| info.position.as_ref())
    }

    /// Returns the full structured database error metadata when available.
    pub fn db_error_info(&self) -> Option<&DbErrorInfo> {
        match self {
            Self::UniqueViolation { info, .. }
            | Self::ForeignKeyViolation { info, .. }
            | Self::RestrictViolation { info, .. }
            | Self::NotNullViolation { info, .. }
            | Self::CheckViolation { info, .. }
            | Self::ExclusionViolation { info, .. }
            | Self::SerializationFailure { info, .. }
            | Self::DeadlockDetected { info, .. }
            | Self::QueryCanceled { info, .. }
            | Self::InsufficientPrivilege { info, .. }
            | Self::Database { info, .. } => Some(info),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DbErrorInfo, DbErrorPosition, Error};

    #[test]
    fn structured_error_helpers_expose_constraint_table_column_and_code() {
        let error = Error::Database {
            code: "23505".to_owned(),
            message: "duplicate key value violates unique constraint".to_owned(),
            detail: Some("Key (email)=(ada@example.com) already exists.".to_owned()),
            hint: Some("Use another email.".to_owned()),
            constraint: Some("users_email_key".to_owned()),
            table: Some("users".to_owned()),
            column: Some("email".to_owned()),
            info: DbErrorInfo {
                schema: Some("public".to_owned()),
                datatype: Some("text".to_owned()),
                where_: Some("SQL statement".to_owned()),
                position: Some(DbErrorPosition::Original(42)),
                ..DbErrorInfo::default()
            },
        };

        assert_eq!(error.code(), Some("23505"));
        assert_eq!(error.constraint_name(), Some("users_email_key"));
        assert_eq!(error.table_name(), Some("users"));
        assert_eq!(error.column_name(), Some("email"));
        assert_eq!(
            error.detail(),
            Some("Key (email)=(ada@example.com) already exists.")
        );
        assert_eq!(error.hint(), Some("Use another email."));
        assert_eq!(error.schema_name(), Some("public"));
        assert_eq!(error.datatype_name(), Some("text"));
        assert_eq!(error.where_context(), Some("SQL statement"));
        assert_eq!(error.position(), Some(&DbErrorPosition::Original(42)));
    }

    #[test]
    fn specialized_error_helpers_fall_back_to_db_error_info() {
        let error = Error::UniqueViolation {
            constraint: None,
            detail: None,
            info: DbErrorInfo {
                constraint: Some("users_email_key".to_owned()),
                table: Some("users".to_owned()),
                column: Some("email".to_owned()),
                ..DbErrorInfo::default()
            },
        };

        assert_eq!(error.code(), Some("23505"));
        assert_eq!(error.constraint_name(), Some("users_email_key"));
        assert_eq!(error.table_name(), Some("users"));
        assert_eq!(error.column_name(), Some("email"));
    }

    #[test]
    fn retryable_errors_are_serialization_deadlock_or_connection_only() {
        assert!(
            Error::SerializationFailure {
                message: "could not serialize access".to_owned(),
                detail: None,
                hint: None,
                info: DbErrorInfo::default(),
            }
            .is_retryable()
        );
        assert!(
            Error::DeadlockDetected {
                message: "deadlock detected".to_owned(),
                detail: None,
                hint: None,
                info: DbErrorInfo::default(),
            }
            .is_retryable()
        );
        assert!(Error::Connection("connection closed".to_owned()).is_retryable());
        assert!(
            !Error::QueryCanceled {
                message: "canceling statement due to user request".to_owned(),
                detail: None,
                hint: None,
                info: DbErrorInfo::default(),
            }
            .is_retryable()
        );
    }
}
