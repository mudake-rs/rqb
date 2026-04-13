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
