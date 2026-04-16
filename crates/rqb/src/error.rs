use sqlx::error::DatabaseError;
use sqlx::postgres::{PgDatabaseError, PgErrorPosition};
use thiserror::Error;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DbErrorInfo {
    pub schema: Option<String>,
    pub table: Option<String>,
    pub column: Option<String>,
    pub datatype: Option<String>,
    pub constraint: Option<String>,
    pub where_: Option<String>,
    pub position: Option<DbErrorPosition>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DbErrorPosition {
    Original(usize),
    Internal { position: usize, query: String },
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

#[derive(Debug, Error)]
pub enum Error {
    #[error("query returned no rows")]
    NotFound,

    #[error("unique violation{}", constraint_suffix(.constraint))]
    UniqueViolation {
        constraint: Option<String>,
        detail: Option<String>,
        info: DbErrorInfo,
    },

    #[error("foreign key violation{}", constraint_suffix(.constraint))]
    ForeignKeyViolation {
        constraint: Option<String>,
        detail: Option<String>,
        info: DbErrorInfo,
    },

    #[error("restrict violation{}", constraint_suffix(.constraint))]
    RestrictViolation {
        constraint: Option<String>,
        detail: Option<String>,
        info: DbErrorInfo,
    },

    #[error("not null violation{}", column_suffix(.column))]
    NotNullViolation {
        column: Option<String>,
        info: DbErrorInfo,
    },

    #[error("check violation{}", constraint_suffix(.constraint))]
    CheckViolation {
        constraint: Option<String>,
        info: DbErrorInfo,
    },

    #[error("exclusion violation{}", constraint_suffix(.constraint))]
    ExclusionViolation {
        constraint: Option<String>,
        detail: Option<String>,
        info: DbErrorInfo,
    },

    #[error("serialization failure: {message}")]
    SerializationFailure {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        info: DbErrorInfo,
    },

    #[error("deadlock detected: {message}")]
    DeadlockDetected {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        info: DbErrorInfo,
    },

    #[error("query canceled: {message}")]
    QueryCanceled {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        info: DbErrorInfo,
    },

    #[error("insufficient privilege: {message}")]
    InsufficientPrivilege {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        table: Option<String>,
        column: Option<String>,
        info: DbErrorInfo,
    },

    #[error("database error ({code}): {message}")]
    Database {
        code: String,
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        constraint: Option<String>,
        table: Option<String>,
        column: Option<String>,
        info: DbErrorInfo,
    },

    #[error("connection error: {0}")]
    Connection(String),

    #[error("sqlx error: {0}")]
    Sqlx(sqlx::Error),

    #[error("transaction rollback failed after error: {error}; rollback error: {rollback}")]
    TransactionRollbackFailed {
        error: Box<Error>,
        rollback: Box<Error>,
    },

    #[error("parameter encode error: {0}")]
    Encode(String),

    #[error("raw SQL fragment has {placeholders} placeholders but {binds} bind values")]
    RawBindMismatch { placeholders: usize, binds: usize },

    #[error("operator `{operator}` is not supported for typed field `{field}`")]
    InvalidTypedOperator { field: String, operator: String },

    #[error("typed field `{field}` is not sortable")]
    InvalidTypedSort { field: String },

    #[error("unknown search field `{field}`")]
    InvalidSearchField { field: String },

    #[error("search field `{field}` is not exposed to JSON requests")]
    SearchFieldNotExposed { field: String },

    #[error("operator `{operator}` is not supported for search field `{field}`")]
    InvalidSearchOperator { field: String, operator: String },

    #[error("invalid JSON value for search field `{field}`; expected {expected}")]
    InvalidSearchValue {
        field: String,
        expected: &'static str,
    },

    #[error("empty search logical expression `{logical}`")]
    EmptySearchLogical { logical: &'static str },

    #[error("empty typed logical expression `{logical}`")]
    EmptyTypedLogical { logical: String },

    #[error("{statement} target must be a table or view source, got {source_kind}")]
    InvalidTypedWriteTarget {
        statement: &'static str,
        source_kind: &'static str,
    },

    #[error("{statement} statement requires at least one assignment")]
    EmptyTypedAssignments { statement: &'static str },

    #[error("{statement} statement requires at least one column")]
    EmptyTypedColumns { statement: &'static str },

    #[error("invalid insert shape: {message}")]
    InvalidInsertShape { message: &'static str },

    #[error("invalid select shape: {message}")]
    InvalidSelectShape { message: &'static str },

    #[error("invalid merge shape: {message}")]
    InvalidMergeShape { message: &'static str },

    #[error("invalid CTE `{name}`: {message}")]
    InvalidCteShape { name: String, message: &'static str },

    #[error("typed delete without filter is not allowed")]
    TypedDeleteWithoutFilter,

    #[error("{join} requires an ON condition")]
    MissingJoinCondition { join: &'static str },
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

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::SerializationFailure { .. } | Self::DeadlockDetected { .. }
        ) || self.is_connection()
    }

    pub fn is_connection(&self) -> bool {
        matches!(self, Self::Connection(_))
    }

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

    pub fn table_name(&self) -> Option<&str> {
        match self {
            Self::InsufficientPrivilege { table, .. } | Self::Database { table, .. } => table
                .as_deref()
                .or_else(|| self.db_error_info().and_then(|info| info.table.as_deref())),
            _ => self.db_error_info().and_then(|info| info.table.as_deref()),
        }
    }

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

    pub fn schema_name(&self) -> Option<&str> {
        self.db_error_info().and_then(|info| info.schema.as_deref())
    }

    pub fn datatype_name(&self) -> Option<&str> {
        self.db_error_info()
            .and_then(|info| info.datatype.as_deref())
    }

    pub fn where_context(&self) -> Option<&str> {
        self.db_error_info().and_then(|info| info.where_.as_deref())
    }

    pub fn position(&self) -> Option<&DbErrorPosition> {
        self.db_error_info().and_then(|info| info.position.as_ref())
    }

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
