use serde::Serialize;

use crate::models::dag::{DagStatus, TaskStatus};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum DagEvent {
    DagStarted {
        dag_id: String,
        total_tasls: usize,
        execution_plan: Vec<Vec<String>>,
    },
    TaskStarted {
        dag_id: String,
        task_id: String,
        level: usize,
        timestamp: i64,
    },
    TaskCompleted {
        dag_id: String,
        task_id: String,
        status: TaskStatus,
        duration_ms: u64,
        output: Option<String>,
        timestamp: i64,
    },
    LevelCompleted {
        dag_id: String,
        level: usize,
        tasks_in_level: Vec<String>,
        timestamp: i64,
    },
    DagCompleted {
        dag_id: String,
        status: DagStatus,
        total_duration_ms: u64,
        timestamp: i64,
    },
    Error {
        dag_id: String,
        message: String,
        timestamp: i64,
    },
}

impl DagEvent {
    pub fn to_see_event(&self) -> String {
        let event_name = match self {
            DagEvent::DagStarted { .. } => "dag_started",
            DagEvent::TaskStarted { .. } => "task_started",
            DagEvent::TaskCompleted { .. } => "task_completed",
            DagEvent::LevelCompleted { .. } => "level_completed",
            DagEvent::DagCompleted { .. } => "dag_completed",
            DagEvent::Error { .. } => "error",
        };

        let data = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());

        format!("event: {}\ndata: {}\n\n", event_name, data)
    }
}
