use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;
use validator::ValidationErrors;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("validation error: {0}")]
    Validation(#[from] ValidationErrors),
    #[error("internal server error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
}

impl From<rqb::postgres::Error> for AppError {
    fn from(error: rqb::postgres::Error) -> Self {
        if error.is_not_found() {
            Self::NotFound
        } else if error.is_unique_violation() {
            Self::Conflict(
                error
                    .constraint_name()
                    .unwrap_or("unique constraint")
                    .to_owned(),
            )
        } else if error.is_foreign_key_violation() {
            Self::Conflict(
                error
                    .constraint_name()
                    .unwrap_or("foreign key constraint")
                    .to_owned(),
            )
        } else if error.is_core() {
            Self::BadRequest(error.to_string())
        } else {
            Self::Internal(error.to_string())
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::BadRequest(_) | Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let message = match self {
            Self::Internal(_) => "internal server error".to_owned(),
            _ => self.to_string(),
        };
        HttpResponse::build(self.status_code()).json(ErrorBody { message })
    }
}
