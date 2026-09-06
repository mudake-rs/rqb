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
                rqb::Error::UniqueViolation(_) | rqb::Error::ExclusionViolation(_) => {
                    (StatusCode::CONFLICT, "conflicting record".to_owned())
                }
                rqb::Error::ForeignKeyViolation(_) | rqb::Error::RestrictViolation(_) => (
                    StatusCode::BAD_REQUEST,
                    "invalid record reference".to_owned(),
                ),
                rqb::Error::NotNullViolation(_) | rqb::Error::CheckViolation(_) => {
                    (StatusCode::BAD_REQUEST, "invalid record values".to_owned())
                }
                rqb::Error::SerializationFailure(_) | rqb::Error::DeadlockDetected(_) => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "retryable database error".to_owned(),
                ),
                rqb::Error::LockNotAvailable(_) => (
                    StatusCode::CONFLICT,
                    "database resource is locked".to_owned(),
                ),
                rqb::Error::QueryCanceled(_) => (
                    StatusCode::GATEWAY_TIMEOUT,
                    "database query canceled".to_owned(),
                ),
                rqb::Error::InsufficientPrivilege(_) => (
                    StatusCode::FORBIDDEN,
                    "insufficient database privilege".to_owned(),
                ),
                rqb::Error::InvalidSearchField { field } => (
                    StatusCode::BAD_REQUEST,
                    format!("unknown search field `{field}`"),
                ),
                rqb::Error::SearchFieldNotExposed { field } => (
                    StatusCode::BAD_REQUEST,
                    format!("search field `{field}` is not exposed"),
                ),
                rqb::Error::InvalidSearchOperator(err) => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "operator `{}` is not allowed for search field `{}`",
                        err.operator, err.field
                    ),
                ),
                rqb::Error::InvalidSearchValue(err) => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "invalid value for search field `{}`; expected {}",
                        err.field, err.expected
                    ),
                ),
                rqb::Error::InvalidSort { field } => (
                    StatusCode::BAD_REQUEST,
                    format!("field `{field}` is not sortable"),
                ),
                rqb::Error::EmptySearchLogical { logical } => (
                    StatusCode::BAD_REQUEST,
                    format!("{logical} group must contain at least one filter"),
                ),
                error => {
                    eprintln!("database operation failed: {error:?}");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "internal server error".to_owned(),
                    )
                }
            },
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn internal_database_details_do_not_reach_http_clients() {
        let error = rqb::Error::Database(Box::new(rqb::DatabaseFailure::new(
            "XX000",
            "secret internal SQL",
        )));
        let response = ApiError::from(error).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value, serde_json::json!({"error":"internal server error"}));
    }
}
