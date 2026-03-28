use axum::{Json, extract::State, http::StatusCode};
use tracing::info;
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    models::dag::{Dag, SubmitDagRequest, SubmitDagResponse},
    services::{executor::execute_dag, scheduler::ExecutionPlan},
    state::AppState,
};

pub async fn submit_dag(
    State(state): State<AppState>,
    Json(payload): Json<SubmitDagRequest>,
) -> AppResult<(StatusCode, Json<SubmitDagResponse>)> {
    info!("Recieved DAG submissions: {:?}", payload.name);

    if payload.tasks.is_empty() {
        return Err(AppError::ValidationError(
            "DAG must contain atleast one task".into(),
        ));
    }

    let plan = ExecutionPlan::build(&payload.tasks)?;

    let dag_id = Uuid::new_v4().to_string();

    let mut dag = Dag::new(dag_id.clone(), payload.name.clone(), payload.tasks);
    dag.execution_levels = plan.levels.clone();

    for (level_idx, level_tasks) in plan.levels.iter().enumerate() {
        for task_id in level_tasks {
            if let Some(task) = dag.get_task_mut(task_id) {
                task.level = Some(level_idx);
            }
        }
    }

    state.upsert_dag(dag.clone());

    state.create_event_channel(&dag_id);

    let state_clone = state.clone();
    let dag_id_clone = dag_id.clone();
    tokio::spawn(async move {
        execute_dag(state_clone, dag_id_clone, plan).await;
    });

    info!("DAG {} submitted successfully", dag_id);

    Ok((
        StatusCode::CREATED,
        Json(SubmitDagResponse {
            dag_id,
            status: crate::models::dag::DagStatus::Pending,
            task_count: dag.tasks.len(),
            execution_levels: dag.execution_levels.clone(),
            message: "DAG submitted and execution started".into(),
        }),
    ))
}
