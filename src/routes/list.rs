use axum::{Json, extract::State};
use serde::Serialize;

use crate::{models::dag::Dag, state::AppState};

#[derive(Debug, Serialize)]
pub struct ListDagResponse {
    pub total: usize,
    pub dags: Vec<DagSummary>,
}

#[derive(Debug, Serialize)]
pub struct DagSummary {
    pub id: String,
    pub name: Option<String>,
    pub status: crate::models::dag::DagStatus,
    pub task_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<Dag> for DagSummary {
    fn from(value: Dag) -> Self {
        Self {
            id: value.id,
            name: value.name,
            status: value.status,
            task_count: value.tasks.len(),
            created_at: value.created_at,
            finished_at: value.finished_at,
        }
    }
}

pub async fn list_dags(State(state): State<AppState>) -> Json<ListDagResponse> {
    let mut dags = state
        .get_all_dags()
        .into_iter()
        .map(DagSummary::from)
        .collect::<Vec<_>>();

    dags.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Json(ListDagResponse {
        total: dags.len(),
        dags,
    })
}
