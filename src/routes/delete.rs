use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;

use crate::{
    errors::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Serialize)]
pub struct DeleteDagResponse {
    pub message: String,
}

pub async fn delete_dag(
    State(state): State<AppState>,
    Path(dag_id): Path<String>,
) -> AppResult<(StatusCode, Json<DeleteDagResponse>)> {
    if state.get_dag(&dag_id).is_none() {
        return Err(AppError::DagNotFound(dag_id));
    }

    state.remove_dag(&dag_id);

    Ok((
        StatusCode::OK,
        Json(DeleteDagResponse {
            message: format!("DAG '{}' deleted successfully", dag_id),
        }),
    ))
}
