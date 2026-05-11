use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Conflict(String),
    Db(rqb::Error),
}

impl From<rqb::Error> for ApiError {
    fn from(error: rqb::Error) -> Self {
        Self::Db(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::Conflict(message) => (StatusCode::CONFLICT, message),
            Self::Db(error) => match error {
                rqb::Error::NotFound => (StatusCode::NOT_FOUND, "not found".to_owned()),
                rqb::Error::UniqueViolation(err) => (
                    StatusCode::CONFLICT,
                    format!(
                        "unique violation{}",
                        suffix("constraint", err.constraint.as_deref())
                    ),
                ),
                rqb::Error::ExclusionViolation(err) => (
                    StatusCode::CONFLICT,
                    format!(
                        "exclusion violation{}",
                        suffix("constraint", err.constraint.as_deref())
                    ),
                ),
                rqb::Error::ForeignKeyViolation(err) | rqb::Error::RestrictViolation(err) => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "constraint violation{}",
                        suffix("constraint", err.constraint.as_deref())
                    ),
                ),
                rqb::Error::NotNullViolation(err) => (
                    StatusCode::BAD_REQUEST,
                    format!("not null violation{}", suffix("column", err.column.as_deref())),
                ),
                rqb::Error::CheckViolation(err) => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "check violation{}",
                        suffix("constraint", err.constraint.as_deref())
                    ),
                ),
                rqb::Error::SerializationFailure(_) | rqb::Error::DeadlockDetected(_) => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "retryable database error".to_owned(),
                ),
                rqb::Error::QueryCanceled(_) => (
                    StatusCode::GATEWAY_TIMEOUT,
                    "database query canceled".to_owned(),
                ),
                rqb::Error::InsufficientPrivilege(_) => (
                    StatusCode::FORBIDDEN,
                    "insufficient database privilege".to_owned(),
                ),
                error => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
            },
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

fn suffix(label: &str, value: Option<&str>) -> String {
    value
        .map(|value| format!(" on {label} `{value}`"))
        .unwrap_or_default()
}
