use rqb_core::Error as CoreError;
use thiserror::Error;

#[cfg(feature = "runtime-tokio-postgres")]
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

#[cfg(feature = "runtime-tokio-postgres")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DbErrorPosition {
    Original(u32),
    Internal { position: u32, query: String },
}

#[cfg(feature = "runtime-tokio-postgres")]
impl DbErrorInfo {
    fn from_db_error(db: &tokio_postgres::error::DbError) -> Self {
        Self {
            schema: db.schema().map(ToOwned::to_owned),
            table: db.table().map(ToOwned::to_owned),
            column: db.column().map(ToOwned::to_owned),
            datatype: db.datatype().map(ToOwned::to_owned),
            constraint: db.constraint().map(ToOwned::to_owned),
            where_: db.where_().map(ToOwned::to_owned),
            position: db.position().map(DbErrorPosition::from),
        }
    }
}

#[cfg(feature = "runtime-tokio-postgres")]
impl From<&tokio_postgres::error::ErrorPosition> for DbErrorPosition {
    fn from(position: &tokio_postgres::error::ErrorPosition) -> Self {
        match position {
            tokio_postgres::error::ErrorPosition::Original(position) => Self::Original(*position),
            tokio_postgres::error::ErrorPosition::Internal { position, query } => Self::Internal {
                position: *position,
                query: query.clone(),
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] CoreError),

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("query returned no rows")]
    NotFound,

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("unique violation{}", constraint_suffix(.constraint))]
    UniqueViolation {
        constraint: Option<String>,
        detail: Option<String>,
        info: DbErrorInfo,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("foreign key violation{}", constraint_suffix(.constraint))]
    ForeignKeyViolation {
        constraint: Option<String>,
        detail: Option<String>,
        info: DbErrorInfo,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("restrict violation{}", constraint_suffix(.constraint))]
    RestrictViolation {
        constraint: Option<String>,
        detail: Option<String>,
        info: DbErrorInfo,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("not null violation{}", column_suffix(.column))]
    NotNullViolation {
        column: Option<String>,
        info: DbErrorInfo,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("check violation{}", constraint_suffix(.constraint))]
    CheckViolation {
        constraint: Option<String>,
        info: DbErrorInfo,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("exclusion violation{}", constraint_suffix(.constraint))]
    ExclusionViolation {
        constraint: Option<String>,
        detail: Option<String>,
        info: DbErrorInfo,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("serialization failure: {message}")]
    SerializationFailure {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        info: DbErrorInfo,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("deadlock detected: {message}")]
    DeadlockDetected {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        info: DbErrorInfo,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("query canceled: {message}")]
    QueryCanceled {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        info: DbErrorInfo,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("insufficient privilege: {message}")]
    InsufficientPrivilege {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        table: Option<String>,
        column: Option<String>,
        info: DbErrorInfo,
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
        info: DbErrorInfo,
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
        let hint = db.hint().map(ToOwned::to_owned);
        let column = db.column().map(ToOwned::to_owned);
        let table = db.table().map(ToOwned::to_owned);
        let message = db.message().to_owned();
        let info = DbErrorInfo::from_db_error(db);

        if *code == SqlState::UNIQUE_VIOLATION {
            return Self::UniqueViolation {
                constraint,
                detail,
                info,
            };
        }
        if *code == SqlState::FOREIGN_KEY_VIOLATION {
            return Self::ForeignKeyViolation {
                constraint,
                detail,
                info,
            };
        }
        if *code == SqlState::RESTRICT_VIOLATION {
            return Self::RestrictViolation {
                constraint,
                detail,
                info,
            };
        }
        if *code == SqlState::NOT_NULL_VIOLATION {
            return Self::NotNullViolation { column, info };
        }
        if *code == SqlState::CHECK_VIOLATION {
            return Self::CheckViolation { constraint, info };
        }
        if *code == SqlState::EXCLUSION_VIOLATION {
            return Self::ExclusionViolation {
                constraint,
                detail,
                info,
            };
        }
        if *code == SqlState::T_R_SERIALIZATION_FAILURE {
            return Self::SerializationFailure {
                message,
                detail,
                hint,
                info,
            };
        }
        if *code == SqlState::T_R_DEADLOCK_DETECTED {
            return Self::DeadlockDetected {
                message,
                detail,
                hint,
                info,
            };
        }
        if *code == SqlState::QUERY_CANCELED {
            return Self::QueryCanceled {
                message,
                detail,
                hint,
                info,
            };
        }
        if *code == SqlState::INSUFFICIENT_PRIVILEGE {
            return Self::InsufficientPrivilege {
                message,
                detail,
                hint,
                table,
                column,
                info,
            };
        }

        Self::Database {
            code: code.code().to_owned(),
            message,
            detail,
            hint,
            constraint,
            table,
            column,
            info,
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
    #[cfg(feature = "runtime-tokio-postgres")]
    pub fn as_core(&self) -> Option<&CoreError> {
        match self {
            Self::Core(error) => Some(error),
            _ => None,
        }
    }

    #[cfg(not(feature = "runtime-tokio-postgres"))]
    pub fn as_core(&self) -> Option<&CoreError> {
        let Self::Core(error) = self;
        Some(error)
    }
}

#[cfg(feature = "runtime-tokio-postgres")]
impl Error {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::SerializationFailure { .. } | Self::DeadlockDetected { .. }
        ) || self.is_connection()
    }

    pub fn is_connection(&self) -> bool {
        match self {
            Self::Connection(_) => true,
            #[cfg(feature = "pool")]
            Self::Pool(_) => true,
            _ => false,
        }
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
