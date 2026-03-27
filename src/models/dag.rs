use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Succeeded,
    Failed { error: String },
    Skipped,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Succeeded | TaskStatus::Failed { .. } | TaskStatus::Skipped
        )
    }

    pub fn is_successful(&self) -> bool {
        matches!(self, TaskStatus::Succeeded)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DagStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    PartialFail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub command: String,
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub definition: TaskDefinition,
    pub status: TaskStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub output: Option<String>,
    pub level: Option<usize>,
}

impl TaskState {
    pub fn new(definition: TaskDefinition) -> Self {
        Self {
            definition,
            status: TaskStatus::Pending,
            started_at: None,
            finished_at: None,
            duration_ms: None,
            output: None,
            level: None,
        }
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self, status: TaskStatus, output: Option<String>) {
        let finished_at = Utc::now();
        self.finished_at = Some(finished_at);
        self.status = status;
        self.output = output;

        if let Some(started) = self.started_at {
            self.duration_ms = Some((finished_at - started).num_milliseconds() as u64);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dag {
    pub id: String,
    pub name: Option<String>,
    pub status: DagStatus,
    pub tasks: HashMap<String, TaskState>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub execution_levels: Vec<Vec<String>>,
}

impl Dag {
    pub fn new(id: String, name: Option<String>, tasks: Vec<TaskDefinition>) -> Self {
        let task_states = tasks
            .into_iter()
            .map(|def| {
                let id = def.id.clone();
                (id, TaskState::new(def))
            })
            .collect();

        Self {
            id,
            name,
            status: DagStatus::Pending,
            tasks: task_states,
            created_at: Utc::now(),
            finished_at: None,
            execution_levels: Vec::new(),
        }
    }

    pub fn update_status(&mut self) {
        let all_terminal = self.tasks.values().all(|t| t.status.is_terminal());

        if !all_terminal {
            self.status = DagStatus::Running;
            return;
        }

        let has_failed = self
            .tasks
            .values()
            .any(|t| matches!(t.status, TaskStatus::Failed { .. }));
        let has_skipped = self
            .tasks
            .values()
            .any(|t| matches!(t.status, TaskStatus::Skipped));
        let all_succeeded = self.tasks.values().all(|t| t.status.is_successful());

        if all_succeeded {
            self.status = DagStatus::Succeeded;
        } else if has_failed || has_skipped {
            if self
                .tasks
                .values()
                .any(|t| matches!(t.status, TaskStatus::Succeeded))
            {
                self.status = DagStatus::PartialFail;
            } else {
                self.status = DagStatus::Failed;
            }
        }

        self.finished_at = Some(Utc::now());
    }

    pub fn get_task(&self, task_id: &str) -> Option<&TaskState> {
        self.tasks.get(task_id)
    }

    pub fn get_task_mut(&mut self, task_id: &str) -> Option<&mut TaskState> {
        self.tasks.get_mut(task_id)
    }
}

#[derive(Debug, Deserialize)]
pub struct SubmitDagRequest {
    pub name: Option<String>,
    pub tasks: Vec<TaskDefinition>,
}

#[derive(Debug, Serialize)]
pub struct SubmitDagResponse {
    pub dag_id: String,
    pub status: DagStatus,
    pub task_count: usize,
    pub execution_levels: Vec<Vec<String>>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct DagStatusResponse {
    pub dag: Dag,
    pub progress: ProgressInfo,
}

#[derive(Debug, Serialize)]
pub struct ProgressInfo {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub percent_complete: f32,
}

impl ProgressInfo {
    pub fn from_dag(dag: &Dag) -> Self {
        let total = dag.tasks.len();
        let mut pending = 0;
        let mut running = 0;
        let mut succeeded = 0;
        let mut failed = 0;
        let mut skipped = 0;

        for task in dag.tasks.values() {
            match &task.status {
                TaskStatus::Pending => pending += 1,
                TaskStatus::Running => running += 1,
                TaskStatus::Succeeded => succeeded += 1,
                TaskStatus::Failed { .. } => failed += 1,
                TaskStatus::Skipped => skipped += 1,
            }
        }

        let completed = succeeded + failed + skipped;
        let percent_complete = if total > 0 {
            (completed as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        Self {
            total,
            pending,
            running,
            succeeded,
            failed,
            skipped,
            percent_complete,
        }
    }
}
