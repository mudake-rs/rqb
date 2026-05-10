use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Conflict(String),
    Db(Box<rqb::Error>),
}

impl From<rqb::Error> for ApiError {
    fn from(error: rqb::Error) -> Self {
        Self::Db(Box::new(error))
    }
}

impl From<Box<rqb::Error>> for ApiError {
    fn from(error: Box<rqb::Error>) -> Self {
        Self::Db(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::Conflict(message) => (StatusCode::CONFLICT, message),
            Self::Db(error) => match *error {
                rqb::Error::NotFound => (StatusCode::NOT_FOUND, "not found".to_owned()),
                rqb::Error::UniqueViolation { constraint, .. } => (
                    StatusCode::CONFLICT,
                    format!("unique violation{}", suffix("constraint", constraint.as_deref())),
                ),
                rqb::Error::ForeignKeyViolation { constraint, .. } => (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "foreign key violation{}",
                        suffix("constraint", constraint.as_deref())
                    ),
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
