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

impl From<rqb::Error> for AppError {
    fn from(error: rqb::Error) -> Self {
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

#[cfg(test)]
mod tests {
    use actix_web::{ResponseError, body::to_bytes, http::StatusCode};

    use super::*;

    #[test]
    fn app_error_maps_postgres_errors_to_http_boundary_errors() {
        let not_found = AppError::from(rqb::Error::NotFound);
        assert!(matches!(not_found, AppError::NotFound));
        assert_eq!(not_found.status_code(), StatusCode::NOT_FOUND);

        let unique = AppError::from(rqb::Error::UniqueViolation {
            constraint: Some("users_email_key".to_owned()),
            detail: None,
        });
        assert!(matches!(unique, AppError::Conflict(ref name) if name == "users_email_key"));
        assert_eq!(unique.status_code(), StatusCode::CONFLICT);

        let core = AppError::from(rqb::Error::Core(rqb::CoreError::UnknownField {
            dataset: "orders".to_owned(),
            field: "missing".to_owned(),
        }));
        assert!(matches!(core, AppError::BadRequest(_)));
        assert_eq!(core.status_code(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn internal_error_response_hides_internal_details() {
        let response = AppError::Internal("database password leaked".to_owned()).error_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body()).await.unwrap();
        assert_eq!(body, r#"{"message":"internal server error"}"#);
    }
}
