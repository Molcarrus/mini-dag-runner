use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    errors::{AppError, AppResult},
    models::dag::{DagStatusResponse, ProgressInfo},
    state::AppState,
};

pub async fn get_dag_status(
    State(state): State<AppState>,
    Path(dag_id): Path<String>,
) -> AppResult<Json<DagStatusResponse>> {
    let dag = state
        .get_dag(&dag_id)
        .ok_or_else(|| AppError::DagNotFound(dag_id.clone()))?;

    let progress = ProgressInfo::from_dag(&dag);

    Ok(Json(DagStatusResponse { dag, progress }))
}
