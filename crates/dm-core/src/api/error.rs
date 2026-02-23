use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: String,
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub error: String,
    pub details: String,
}

impl ApiError {
    pub fn new(status: StatusCode, error: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            status,
            error: error.into(),
            details: details.into(),
        }
    }

    pub fn bad_request(error: impl Into<String>, details: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, error, details)
    }

    pub fn not_found(error: impl Into<String>, details: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, error, details)
    }

    pub fn internal(error: impl Into<String>, details: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, error, details)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.error,
                details: self.details,
            }),
        )
            .into_response()
    }
}
