use sqlx::error::DatabaseError;
use sqlx::postgres::{PgDatabaseError, PgErrorPosition};
use thiserror::Error;

/// Structured metadata extracted from a Postgres database error.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
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
    /// One-based character position in the original submitted SQL.
    Original(usize),
    /// One-based character position in an internally generated query.
    Internal {
        /// One-based character position in `query`.
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

/// Payload for Postgres constraint-class errors.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConstraintError {
    /// Postgres detail string.
    pub detail: Option<String>,
    /// Additional structured database error metadata.
    pub info: DbErrorInfo,
}

/// Payload for Postgres failures without a dedicated constraint or column shape.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PgFailure {
    /// Postgres primary error message.
    pub message: String,
    /// Postgres detail string.
    pub detail: Option<String>,
    /// Postgres hint string.
    pub hint: Option<String>,
    /// Additional structured database error metadata.
    pub info: DbErrorInfo,
}

/// Payload for mapped or unmapped Postgres database failures.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DatabaseFailure {
    /// SQLSTATE error code.
    pub code: String,
    /// Database primary error message.
    pub message: String,
    /// Database detail string.
    pub detail: Option<String>,
    /// Database hint string.
    pub hint: Option<String>,
    /// Additional structured database error metadata.
    pub info: DbErrorInfo,
}

/// Payload for field/operator validation failures.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct OperatorError {
    /// Field API name.
    pub field: String,
    /// Operator name.
    pub operator: String,
}

/// Payload for JSON search value validation failures.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SearchValueError {
    /// Field name from the JSON request.
    pub field: String,
    /// Human-readable expected value kind.
    pub expected: &'static str,
}

/// Payload for write-target validation failures.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct WriteTargetError {
    /// Statement kind.
    pub statement: &'static str,
    /// Source kind that was used as the write target.
    pub source_kind: &'static str,
}

/// Payload for CTE shape validation failures.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CteShapeError {
    /// CTE name.
    pub name: String,
    /// Validation message.
    pub message: &'static str,
}

impl ConstraintError {
    /// Creates a constraint error payload for tests or adapters.
    pub fn new(detail: Option<String>, info: DbErrorInfo) -> Self {
        Self { detail, info }
    }
}

impl PgFailure {
    /// Creates a generic Postgres failure payload for tests or adapters.
    pub fn new(
        message: impl Into<String>,
        detail: Option<String>,
        hint: Option<String>,
        info: DbErrorInfo,
    ) -> Self {
        Self {
            message: message.into(),
            detail,
            hint,
            info,
        }
    }
}

impl DatabaseFailure {
    /// Creates a database failure payload for tests or adapters.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: None,
            hint: None,
            info: DbErrorInfo::default(),
        }
    }

    /// Sets the Postgres detail string.
    pub fn detail(mut self, detail: Option<String>) -> Self {
        self.detail = detail;
        self
    }

    /// Sets the Postgres hint string.
    pub fn hint(mut self, hint: Option<String>) -> Self {
        self.hint = hint;
        self
    }

    /// Sets the reported constraint name.
    pub fn constraint(mut self, constraint: Option<String>) -> Self {
        self.info.constraint = constraint;
        self
    }

    /// Sets the reported table name.
    pub fn table(mut self, table: Option<String>) -> Self {
        self.info.table = table;
        self
    }

    /// Sets the reported column name.
    pub fn column(mut self, column: Option<String>) -> Self {
        self.info.column = column;
        self
    }

    /// Sets structured database error metadata.
    pub fn info(mut self, info: DbErrorInfo) -> Self {
        self.info = info;
        self
    }
}

impl OperatorError {
    /// Creates a field/operator validation payload.
    pub fn new(field: impl Into<String>, operator: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            operator: operator.into(),
        }
    }
}

impl SearchValueError {
    /// Creates a JSON search value validation payload.
    pub fn new(field: impl Into<String>, expected: &'static str) -> Self {
        Self {
            field: field.into(),
            expected,
        }
    }
}

impl WriteTargetError {
    /// Creates a write-target validation payload.
    pub fn new(statement: &'static str, source_kind: &'static str) -> Self {
        Self {
            statement,
            source_kind,
        }
    }
}

impl CteShapeError {
    /// Creates a CTE shape validation payload.
    pub fn new(name: impl Into<String>, message: &'static str) -> Self {
        Self {
            name: name.into(),
            message,
        }
    }
}

/// Error type returned by rqb builders and sqlx execution helpers.
///
/// Builder/search validation errors are returned before SQL is rendered.
/// Execution errors preserve sqlx/Postgres detail and map common SQLSTATE
/// classes to structured variants such as [`Error::UniqueViolation`] and
/// [`Error::NotNullViolation`]. HTTP adapters typically map search/validation
/// errors to 400, `NotFound` to 404, constraint conflicts to 409, and other
/// database failures to 500 unless the application has a narrower policy.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A `fetch_one`-style operation did not return a row.
    #[error("query returned no rows")]
    NotFound,

    /// Postgres unique constraint violation (`23505`).
    #[error("unique violation{}", constraint_suffix(&.0.info.constraint))]
    UniqueViolation(Box<ConstraintError>),

    /// Postgres foreign key violation (`23503`).
    #[error("foreign key violation{}", constraint_suffix(&.0.info.constraint))]
    ForeignKeyViolation(Box<ConstraintError>),

    /// Postgres restrict violation (`23001`).
    #[error("restrict violation{}", constraint_suffix(&.0.info.constraint))]
    RestrictViolation(Box<ConstraintError>),

    /// Postgres not-null violation (`23502`).
    #[error("not null violation{}", column_suffix(&.0.column))]
    NotNullViolation(Box<DbErrorInfo>),

    /// Postgres check constraint violation (`23514`).
    #[error("check violation{}", constraint_suffix(&.0.info.constraint))]
    CheckViolation(Box<ConstraintError>),

    /// Postgres exclusion constraint violation (`23P01`).
    #[error("exclusion violation{}", constraint_suffix(&.0.info.constraint))]
    ExclusionViolation(Box<ConstraintError>),

    /// Postgres serialization failure (`40001`).
    #[error("serialization failure: {}", .0.message)]
    SerializationFailure(Box<PgFailure>),

    /// Postgres deadlock detected (`40P01`).
    #[error("deadlock detected: {}", .0.message)]
    DeadlockDetected(Box<PgFailure>),

    /// Postgres lock not available (`55P03`).
    #[error("lock not available: {}", .0.message)]
    LockNotAvailable(Box<PgFailure>),

    /// Postgres query cancellation (`57014`).
    #[error("query canceled: {}", .0.message)]
    QueryCanceled(Box<PgFailure>),

    /// Postgres insufficient privilege error (`42501`).
    #[error("insufficient privilege: {}", .0.message)]
    InsufficientPrivilege(Box<DatabaseFailure>),

    /// A database error that is not mapped to a specialized variant.
    #[error("database error ({}): {}", .0.code, .0.message)]
    Database(Box<DatabaseFailure>),

    /// Connection-level failure.
    #[error("connection error: {0}")]
    Connection(#[source] Box<sqlx::Error>),

    /// Unclassified sqlx error.
    #[error("sqlx error: {0}")]
    Sqlx(Box<sqlx::Error>),

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
    #[error("operator `{}` is not supported for field `{}`", .0.operator, .0.field)]
    InvalidOperator(Box<OperatorError>),

    /// An aggregate-local modifier was applied to a non-aggregate expression.
    #[error("aggregate modifier `{modifier}` can only be applied to aggregate expressions")]
    InvalidAggregateModifier {
        /// Modifier method name.
        modifier: &'static str,
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
    #[error("operator `{}` is not supported for search field `{}`", .0.operator, .0.field)]
    InvalidSearchOperator(Box<OperatorError>),

    /// JSON search value did not match the field's expected JSON shape.
    #[error("invalid JSON value for search field `{}`; expected {}", .0.field, .0.expected)]
    InvalidSearchValue(Box<SearchValueError>),

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
        logical: &'static str,
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
    #[error("{} target must be a table or view source, got {}", .0.statement, .0.source_kind)]
    InvalidWriteTarget(Box<WriteTargetError>),

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
    #[error("invalid CTE `{}`: {}", .0.name, .0.message)]
    InvalidCteShape(Box<CteShapeError>),

    /// A delete statement was built without a filter.
    #[error("delete without filter is not allowed")]
    DeleteWithoutFilter,
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
            error @ (sqlx::Error::Io(_)
            | sqlx::Error::Tls(_)
            | sqlx::Error::PoolTimedOut
            | sqlx::Error::PoolClosed
            | sqlx::Error::WorkerCrashed) => Self::Connection(Box::new(error)),
            other => Self::Sqlx(Box::new(other)),
        }
    }
}

impl Error {
    pub(crate) fn invalid_operator(field: impl Into<String>, operator: impl Into<String>) -> Self {
        Self::InvalidOperator(Box::new(OperatorError::new(field, operator)))
    }

    pub(crate) fn invalid_search_operator(
        field: impl Into<String>,
        operator: impl Into<String>,
    ) -> Self {
        Self::InvalidSearchOperator(Box::new(OperatorError::new(field, operator)))
    }

    pub(crate) fn invalid_search_value(field: impl Into<String>, expected: &'static str) -> Self {
        Self::InvalidSearchValue(Box::new(SearchValueError::new(field, expected)))
    }

    pub(crate) fn invalid_write_target(statement: &'static str, source_kind: &'static str) -> Self {
        Self::InvalidWriteTarget(Box::new(WriteTargetError::new(statement, source_kind)))
    }

    pub(crate) fn invalid_cte_shape(name: impl Into<String>, message: &'static str) -> Self {
        Self::InvalidCteShape(Box::new(CteShapeError::new(name, message)))
    }

    fn from_sqlx_database_error(db: &(dyn DatabaseError + 'static)) -> Self {
        let pg = db.try_downcast_ref::<PgDatabaseError>();
        let code = db
            .code()
            .map(|code| code.into_owned())
            .unwrap_or_else(|| "unknown".to_owned());
        let message = db.message().to_owned();
        let detail = pg.and_then(PgDatabaseError::detail).map(ToOwned::to_owned);
        let hint = pg.and_then(PgDatabaseError::hint).map(ToOwned::to_owned);
        let info = DbErrorInfo::from_sqlx_database_error(db);

        match code.as_str() {
            "23505" => Self::UniqueViolation(Box::new(ConstraintError::new(detail, info))),
            "23503" => Self::ForeignKeyViolation(Box::new(ConstraintError::new(detail, info))),
            "23001" => Self::RestrictViolation(Box::new(ConstraintError::new(detail, info))),
            "23502" => Self::NotNullViolation(Box::new(info)),
            "23514" => Self::CheckViolation(Box::new(ConstraintError::new(detail, info))),
            "23P01" => Self::ExclusionViolation(Box::new(ConstraintError::new(detail, info))),
            "40001" => {
                Self::SerializationFailure(Box::new(PgFailure::new(message, detail, hint, info)))
            }
            "40P01" => {
                Self::DeadlockDetected(Box::new(PgFailure::new(message, detail, hint, info)))
            }
            "55P03" => {
                Self::LockNotAvailable(Box::new(PgFailure::new(message, detail, hint, info)))
            }
            "57014" => Self::QueryCanceled(Box::new(PgFailure::new(message, detail, hint, info))),
            "42501" => Self::InsufficientPrivilege(Box::new(
                DatabaseFailure::new(code, message)
                    .detail(detail)
                    .hint(hint)
                    .info(info),
            )),
            _ => Self::Database(Box::new(
                DatabaseFailure::new(code, message)
                    .detail(detail)
                    .hint(hint)
                    .info(info),
            )),
        }
    }

    /// Returns true for serialization failures and deadlocks.
    /// Retry the whole transaction, including its decisions, with bounded backoff.
    /// Connection failures are excluded: a lost commit response can have an unknown outcome.
    pub fn is_retryable(&self) -> bool {
        matches!(self.code(), Some("40001" | "40P01"))
    }

    /// Returns true for connection-level failures.
    /// Classification only; it does not establish whether replaying an operation is safe.
    pub fn is_connection(&self) -> bool {
        matches!(self, Self::Connection(_))
    }

    /// Returns the SQLSTATE code when this error maps to one.
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::UniqueViolation(_) => Some("23505"),
            Self::ForeignKeyViolation(_) => Some("23503"),
            Self::RestrictViolation(_) => Some("23001"),
            Self::NotNullViolation(_) => Some("23502"),
            Self::CheckViolation(_) => Some("23514"),
            Self::ExclusionViolation(_) => Some("23P01"),
            Self::SerializationFailure(_) => Some("40001"),
            Self::DeadlockDetected(_) => Some("40P01"),
            Self::LockNotAvailable(_) => Some("55P03"),
            Self::InsufficientPrivilege(_) => Some("42501"),
            Self::QueryCanceled(_) => Some("57014"),
            Self::Database(err) => Some(&err.code),
            _ => None,
        }
    }

    /// Returns the associated constraint name when available.
    pub fn constraint_name(&self) -> Option<&str> {
        self.db_error_info()
            .and_then(|info| info.constraint.as_deref())
    }

    /// Returns the associated column name when available.
    pub fn column_name(&self) -> Option<&str> {
        self.db_error_info().and_then(|info| info.column.as_deref())
    }

    /// Returns the database detail message when available.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::UniqueViolation(err)
            | Self::ForeignKeyViolation(err)
            | Self::RestrictViolation(err)
            | Self::CheckViolation(err)
            | Self::ExclusionViolation(err) => err.detail.as_deref(),
            Self::SerializationFailure(err)
            | Self::DeadlockDetected(err)
            | Self::LockNotAvailable(err)
            | Self::QueryCanceled(err) => err.detail.as_deref(),
            Self::InsufficientPrivilege(err) | Self::Database(err) => err.detail.as_deref(),
            _ => None,
        }
    }

    /// Returns the database hint message when available.
    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::SerializationFailure(err)
            | Self::DeadlockDetected(err)
            | Self::LockNotAvailable(err)
            | Self::QueryCanceled(err) => err.hint.as_deref(),
            Self::InsufficientPrivilege(err) | Self::Database(err) => err.hint.as_deref(),
            _ => None,
        }
    }

    fn db_error_info(&self) -> Option<&DbErrorInfo> {
        match self {
            Self::UniqueViolation(err)
            | Self::ForeignKeyViolation(err)
            | Self::RestrictViolation(err)
            | Self::CheckViolation(err)
            | Self::ExclusionViolation(err) => Some(&err.info),
            Self::NotNullViolation(info) => Some(info),
            Self::SerializationFailure(err)
            | Self::DeadlockDetected(err)
            | Self::LockNotAvailable(err)
            | Self::QueryCanceled(err) => Some(&err.info),
            Self::InsufficientPrivilege(err) | Self::Database(err) => Some(&err.info),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ConstraintError, DatabaseFailure, DbErrorInfo, DbErrorPosition, Error, PgFailure};

    #[test]
    fn error_size_stays_small_for_application_results() {
        assert!(std::mem::size_of::<Error>() <= 32);
    }

    #[test]
    fn structured_error_helpers_expose_common_api_boundary_fields() {
        let error = Error::Database(Box::new(DatabaseFailure {
            code: "23505".to_owned(),
            message: "duplicate key value violates unique constraint".to_owned(),
            detail: Some("Key (email)=(ada@example.com) already exists.".to_owned()),
            hint: Some("Use another email.".to_owned()),
            info: DbErrorInfo {
                constraint: Some("users_email_key".to_owned()),
                table: Some("users".to_owned()),
                column: Some("email".to_owned()),
                schema: Some("public".to_owned()),
                datatype: Some("text".to_owned()),
                where_: Some("SQL statement".to_owned()),
                position: Some(DbErrorPosition::Original(42)),
                ..DbErrorInfo::default()
            },
        }));

        assert_eq!(error.code(), Some("23505"));
        assert_eq!(error.constraint_name(), Some("users_email_key"));
        assert_eq!(error.column_name(), Some("email"));
        assert_eq!(
            error.detail(),
            Some("Key (email)=(ada@example.com) already exists.")
        );
        assert_eq!(error.hint(), Some("Use another email."));

        let Error::Database(err) = error else {
            panic!("expected database error");
        };
        assert_eq!(err.info.table.as_deref(), Some("users"));
        assert_eq!(err.info.schema.as_deref(), Some("public"));
        assert_eq!(err.info.datatype.as_deref(), Some("text"));
        assert_eq!(err.info.where_.as_deref(), Some("SQL statement"));
        assert_eq!(err.info.position, Some(DbErrorPosition::Original(42)));
    }

    #[test]
    fn specialized_error_helpers_share_db_error_info() {
        let error = Error::UniqueViolation(Box::new(ConstraintError {
            detail: None,
            info: DbErrorInfo {
                constraint: Some("users_email_key".to_owned()),
                table: Some("users".to_owned()),
                column: Some("email".to_owned()),
                ..DbErrorInfo::default()
            },
        }));

        assert_eq!(error.code(), Some("23505"));
        assert_eq!(error.constraint_name(), Some("users_email_key"));
        assert_eq!(error.column_name(), Some("email"));
    }

    #[test]
    fn retryable_errors_exclude_unknown_connection_outcomes() {
        assert!(
            Error::SerializationFailure(Box::new(PgFailure {
                message: "could not serialize access".to_owned(),
                detail: None,
                hint: None,
                info: DbErrorInfo::default(),
            }))
            .is_retryable()
        );
        assert!(
            Error::DeadlockDetected(Box::new(PgFailure {
                message: "deadlock detected".to_owned(),
                detail: None,
                hint: None,
                info: DbErrorInfo::default(),
            }))
            .is_retryable()
        );
        assert!(!Error::from(sqlx::Error::PoolClosed).is_retryable());
        for code in ["57P01", "57P02", "57P03"] {
            assert!(
                !Error::Database(Box::new(DatabaseFailure::new(
                    code,
                    "transient server failure"
                )))
                .is_retryable(),
                "{code} must not imply safe transaction replay"
            );
        }
        assert!(
            !Error::QueryCanceled(Box::new(PgFailure {
                message: "canceling statement due to user request".to_owned(),
                detail: None,
                hint: None,
                info: DbErrorInfo::default(),
            }))
            .is_retryable()
        );
        assert!(
            !Error::NotNullViolation(Box::new(DbErrorInfo {
                column: Some("email".to_owned()),
                ..DbErrorInfo::default()
            }))
            .is_retryable()
        );
    }

    #[test]
    fn lock_not_available_exposes_sqlstate_without_global_retry_policy() {
        let error = Error::LockNotAvailable(Box::new(PgFailure {
            message: "could not obtain lock on row in relation".to_owned(),
            detail: Some("row is already locked".to_owned()),
            hint: Some("retry later".to_owned()),
            info: DbErrorInfo::default(),
        }));

        assert_eq!(error.code(), Some("55P03"));
        assert_eq!(error.detail(), Some("row is already locked"));
        assert_eq!(error.hint(), Some("retry later"));
        assert!(!error.is_retryable());
    }

    #[test]
    fn connection_like_sqlx_errors_map_to_connection_variant() {
        let pool_timed_out = Error::from(sqlx::Error::PoolTimedOut);
        assert!(matches!(pool_timed_out, Error::Connection(_)));
        assert!(pool_timed_out.is_connection());
        assert!(!pool_timed_out.is_retryable());

        let io = Error::from(sqlx::Error::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "connection reset",
        )));
        assert!(matches!(io, Error::Connection(_)));
        assert!(io.is_connection());
        assert!(!io.is_retryable());
        assert!(!Error::NotFound.is_connection());
        let Error::Connection(source) = io else {
            unreachable!()
        };
        assert!(matches!(*source, sqlx::Error::Io(_)));
    }
}
