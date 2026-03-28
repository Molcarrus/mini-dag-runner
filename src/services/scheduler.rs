use std::collections::{HashMap, HashSet};

use petgraph::{
    algo::toposort,
    graph::{DiGraph, NodeIndex},
};
use thiserror::Error;

use crate::models::dag::TaskDefinition;

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Duplicate task ID: {0}")]
    DuplicateTaskId(String),
    #[error("Task '{0}' depends on unknown task '{1}'")]
    MissingDependency(String, String),
    #[error("Cycle detected in DAG")]
    CycleDetected,
    #[error("DAG has no tasks")]
    EmptyDag,
}

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub levels: Vec<Vec<String>>,
}

impl ExecutionPlan {
    pub fn build(tasks: &[TaskDefinition]) -> Result<Self, SchedulerError> {
        if tasks.is_empty() {
            return Err(SchedulerError::EmptyDag);
        }

        let mut seen_ids = HashSet::new();
        for task in tasks {
            if !seen_ids.insert(&task.id) {
                return Err(SchedulerError::DuplicateTaskId(task.id.clone()));
            }
        }

        for task in tasks {
            for dep in &task.depends_on {
                if !seen_ids.contains(dep) {
                    return Err(SchedulerError::MissingDependency(
                        task.id.clone(),
                        dep.clone(),
                    ));
                }
            }
        }

        let mut graph = DiGraph::<String, ()>::new();
        let mut node_indices = HashMap::new();

        for task in tasks {
            let idx = graph.add_node(task.id.clone());
            node_indices.insert(task.id.clone(), idx);
        }

        for task in tasks {
            let dependent_idx = node_indices[&task.id];
            for dep in &task.depends_on {
                let dependency_idx = node_indices[dep];
                graph.add_edge(dependency_idx, dependent_idx, ());
            }
        }

        let topo_order = toposort(&graph, None).map_err(|_| SchedulerError::CycleDetected)?;

        let depths = compute_node_depths(&graph, &node_indices, tasks);

        let max_depth = *depths.values().max().unwrap_or(&0);
        let mut levels = vec![Vec::new(); max_depth + 1];

        for (task_id, depth) in depths {
            levels[depth].push(task_id);
        }

        for level in &mut levels {
            level.sort();
        }

        Ok(Self { levels })
    }
}

fn compute_node_depths(
    graph: &DiGraph<String, ()>,
    node_indices: &HashMap<String, NodeIndex>,
    tasks: &[TaskDefinition],
) -> HashMap<String, usize> {
    let mut depths = HashMap::new();
    let mut processing_order = Vec::new();

    let topo = toposort(graph, None).unwrap();

    for node_idx in topo {
        let task_id = &graph[node_idx];
        processing_order.push(task_id.clone());
    }

    for task_id in processing_order {
        let task = tasks.iter().find(|t| t.id == task_id).unwrap();

        if task.depends_on.is_empty() {
            depths.insert(task_id, 0);
        } else {
            let max_dep_depth = task
                .depends_on
                .iter()
                .map(|dep| depths.get(dep).unwrap_or(&0))
                .max()
                .unwrap_or(&0);
            depths.insert(task_id, max_dep_depth + 1);
        }
    }

    depths
}
