use rqb_core::Error as CoreError;
use thiserror::Error;

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
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("foreign key violation{}", constraint_suffix(.constraint))]
    ForeignKeyViolation {
        constraint: Option<String>,
        detail: Option<String>,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("restrict violation{}", constraint_suffix(.constraint))]
    RestrictViolation {
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
    #[error("serialization failure: {message}")]
    SerializationFailure {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("deadlock detected: {message}")]
    DeadlockDetected {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("query canceled: {message}")]
    QueryCanceled {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
    },

    #[cfg(feature = "runtime-tokio-postgres")]
    #[error("insufficient privilege: {message}")]
    InsufficientPrivilege {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        table: Option<String>,
        column: Option<String>,
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
        let hint = db.hint().map(ToOwned::to_owned);
        let column = db.column().map(ToOwned::to_owned);
        let table = db.table().map(ToOwned::to_owned);
        let message = db.message().to_owned();

        if *code == SqlState::UNIQUE_VIOLATION {
            return Self::UniqueViolation { constraint, detail };
        }
        if *code == SqlState::FOREIGN_KEY_VIOLATION {
            return Self::ForeignKeyViolation { constraint, detail };
        }
        if *code == SqlState::RESTRICT_VIOLATION {
            return Self::RestrictViolation { constraint, detail };
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
        if *code == SqlState::T_R_SERIALIZATION_FAILURE {
            return Self::SerializationFailure {
                message,
                detail,
                hint,
            };
        }
        if *code == SqlState::T_R_DEADLOCK_DETECTED {
            return Self::DeadlockDetected {
                message,
                detail,
                hint,
            };
        }
        if *code == SqlState::QUERY_CANCELED {
            return Self::QueryCanceled {
                message,
                detail,
                hint,
            };
        }
        if *code == SqlState::INSUFFICIENT_PRIVILEGE {
            return Self::InsufficientPrivilege {
                message,
                detail,
                hint,
                table,
                column,
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

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::SerializationFailure { .. } | Self::DeadlockDetected { .. }
        ) || self.is_connection()
    }

    pub fn is_constraint(&self, name: &str) -> bool {
        self.constraint_name() == Some(name)
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
            | Self::CheckViolation { constraint }
            | Self::ExclusionViolation { constraint, .. }
            | Self::Database { constraint, .. } => constraint.as_deref(),
            _ => None,
        }
    }

    pub fn table_name(&self) -> Option<&str> {
        match self {
            Self::InsufficientPrivilege { table, .. } | Self::Database { table, .. } => {
                table.as_deref()
            }
            _ => None,
        }
    }

    pub fn column_name(&self) -> Option<&str> {
        match self {
            Self::NotNullViolation { column }
            | Self::InsufficientPrivilege { column, .. }
            | Self::Database { column, .. } => column.as_deref(),
            _ => None,
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
}
