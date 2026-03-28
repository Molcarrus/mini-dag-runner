use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

pub enum AppError {
    DagNotFound(String),
    ValidationError(String),
    SchedulerError(String),
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status_code, message) = match self {
            AppError::DagNotFound(id) => (StatusCode::NOT_FOUND, format!("Dag '{}' not found", id)),
            AppError::ValidationError(msg) => (StatusCode::BAD_GATEWAY, msg),
            AppError::SchedulerError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status_code, Json(json!({ "error": message }))).into_response()
    }
}

impl From<crate::services::scheduler::SchedulerError> for AppError {
    fn from(value: crate::services::scheduler::SchedulerError) -> Self {
        AppError::SchedulerError(value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
