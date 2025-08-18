use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use axum::extract::rejection::JsonRejection;
use anyhow;
use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Internal(anyhow::Error),
    ValidationError(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Internal(err) => write!(f, "Internal error: {}", err),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_response) = match self {
            AppError::Internal(err) => {
                tracing::error!("Internal error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse {
                        error: "Internal Server Error".to_string(),
                        message: "An unexpected error occurred".to_string(),
                    },
                )
            }
            AppError::ValidationError(msg) => {
                (
                    StatusCode::BAD_REQUEST,
                    ErrorResponse {
                        error: "Validation Error".to_string(),
                        message: msg,
                    },
                )
            }
        };
        (status, Json(error_response)).into_response()
    }
}
impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        AppError::Internal(error)
    }
}
impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        let message = format!("Failed to deserialize the JSON body into the target type: {}", rejection.body_text());
        AppError::ValidationError(message)
    }
}