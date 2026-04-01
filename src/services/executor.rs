use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use tokio::task::JoinSet;
use tracing::{error, info};

use crate::{
    models::{
        dag::{DagStatus, TaskDefinition, TaskStatus},
        events::DagEvent,
    },
    services::scheduler::ExecutionPlan,
    state::AppState,
};

pub async fn execute_dag(state: AppState, dag_id: String, plan: ExecutionPlan) {
    info!("Starting execution of DAG: {}", dag_id);

    let event_sender = match state.get_event_sender(&dag_id) {
        Some(sender) => sender,
        None => {
            error!("No event sender found for Dag: {}", dag_id);
            return;
        }
    };

    {
        if let Some(mut dag) = state.dags.get_mut(&dag_id) {
            dag.status = DagStatus::Running;
        }
    }

    let _ = event_sender.send(DagEvent::DagStarted {
        dag_id: dag_id.clone(),
        total_tasls: plan.levels.iter().map(|level| level.len()).sum::<usize>(),
        execution_plan: plan.levels.clone(),
    });

    let start_time = Utc::now();

    for (level_idx, level_tasks) in plan.levels.iter().enumerate() {
        info!("Executing level {}: {:?}", level_idx, level_tasks);

        let dependencies_satisfied = check_dependencies(&state, &dag_id, level_tasks);

        let mut join_set = JoinSet::new();
        let mut skipped_tasks = Vec::new();

        for task_id in level_tasks {
            if !dependencies_satisfied.get(task_id).unwrap_or(&false) {
                info!("Skipping task {} due to failed dependenices", task_id);
                skipped_tasks.push(task_id.clone());

                if let Some(mut dag) = state.dags.get_mut(&dag_id) {
                    if let Some(task) = dag.get_task_mut(task_id) {
                        task.complete(crate::models::dag::TaskStatus::Skipped, None);
                    }
                }

                let _ = event_sender.send(DagEvent::TaskCompleted {
                    dag_id: dag_id.clone(),
                    task_id: task_id.clone(),
                    status: crate::models::dag::TaskStatus::Skipped,
                    duration_ms: 0,
                    output: None,
                    timestamp: Utc::now().timestamp_millis(),
                });

                continue;
            }

            let task_def = {
                let dag = state.dags.get(&dag_id).unwrap();
                dag.get_task(task_id).unwrap().definition.clone()
            };

            {
                if let Some(mut dag) = state.dags.get_mut(&dag_id) {
                    if let Some(task) = dag.get_task_mut(task_id) {
                        task.start();
                        task.level = Some(level_idx);
                    }
                }
            }

            let _ = event_sender.send(DagEvent::TaskStarted {
                dag_id: dag_id.clone(),
                task_id: task_id.clone(),
                level: level_idx,
                timestamp: Utc::now().timestamp_millis(),
            });

            let dag_id_clone = dag_id.clone();
            let task_id_clone = task_id.clone();
            let state_clone = state.clone();
            let event_sender_clone = event_sender.clone();

            join_set.spawn(async move {
                let result = execute_task(&task_def).await;

                let (status, output) = match result {
                    Ok(out) => (TaskStatus::Succeeded, Some(out)),
                    Err(err) => (TaskStatus::Failed { error: err.clone() }, Some(err)),
                };

                let duration_ms = {
                    if let Some(mut dag) = state_clone.dags.get_mut(&dag_id_clone) {
                        if let Some(task) = dag.get_task_mut(&task_id_clone) {
                            task.complete(status.clone(), output.clone());
                            task.duration_ms.unwrap_or(0)
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                };

                let _ = event_sender_clone.send(DagEvent::TaskCompleted {
                    dag_id: dag_id_clone.clone(),
                    task_id: task_id_clone.clone(),
                    status: status.clone(),
                    duration_ms,
                    output,
                    timestamp: Utc::now().timestamp_millis(),
                });

                (task_id_clone, status)
            });
        }

        let mut level_results = Vec::new();
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((task_id, status)) => {
                    level_results.push((task_id, status));
                }
                Err(e) => {
                    error!("Task panicked: {:?}", e);
                }
            }
        }

        let _ = event_sender.send(DagEvent::LevelCompleted {
            dag_id: dag_id.clone(),
            level: level_idx,
            tasks_in_level: level_tasks.clone(),
            timestamp: Utc::now().timestamp_millis(),
        });

        info!(
            "Level {} completed. Results: {:?}",
            level_idx, level_results
        );

        let final_status = {
            if let Some(mut dag) = state.dags.get_mut(&dag_id) {
                dag.update_status();
                dag.status.clone()
            } else {
                DagStatus::Failed
            }
        };

        let toal_duration = (Utc::now() - start_time).num_milliseconds() as u64;

        let _ = event_sender.send(DagEvent::DagCompleted {
            dag_id: dag_id.clone(),
            status: final_status.clone(),
            total_duration_ms: toal_duration,
            timestamp: Utc::now().timestamp_millis(),
        });

        info!(
            "DAG {} completed with status: {:?} in {}ms",
            dag_id, final_status, toal_duration
        );
    }
}

fn check_dependencies(
    state: &AppState,
    dag_id: &str,
    level_tasks: &[String],
) -> HashMap<String, bool> {
    let mut satisfied = HashMap::new();

    let dag = match state.dags.get(dag_id) {
        Some(dag) => dag,
        None => return satisfied,
    };

    for task_id in level_tasks {
        let task = match dag.get_task(task_id) {
            Some(t) => t,
            None => continue,
        };

        let all_deps_succeeded = task.definition.depends_on.iter().all(|dep_id| {
            dag.get_task(dep_id)
                .map(|dep| dep.status.is_successful())
                .unwrap_or(false)
        });

        satisfied.insert(task_id.clone(), all_deps_succeeded);
    }

    satisfied
}

async fn execute_task(task: &TaskDefinition) -> Result<String, String> {
    info!("Executing task: {} with command: {}", task.id, task.command);

    match task.command.as_str() {
        "send_email" => {
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok("Email sent successfully".into())
        }
        "build" => {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Ok("Build completed".into())
        }
        "test" => {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok("All tests passed".into())
        }
        "deploy" => {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            Ok("Deployed to production".into())
        }
        "fail" => {
            tokio::time::sleep(Duration::from_millis(300)).await;
            Err("Intentional failure for testing".into())
        }
        "shell" => {
            tokio::time::sleep(Duration::from_millis(800)).await;
            Ok("Shell command executed".into())
        }
        other => {
            let delay = (task.id.len() * 100) % 2000 + 500;
            tokio::time::sleep(Duration::from_millis(delay as u64)).await;
            Ok(format!("Executed '{}' successfully", other))
        }
    }
}
