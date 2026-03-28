use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::broadcast;

use crate::models::{dag::Dag, events::DagEvent};

pub type EventSender = broadcast::Sender<DagEvent>;

#[derive(Clone)]
pub struct AppState {
    pub dags: Arc<DashMap<String, Dag>>,
    pub event_channels: Arc<DashMap<String, EventSender>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            dags: Arc::new(DashMap::new()),
            event_channels: Arc::new(DashMap::new()),
        }
    }

    pub fn create_event_channel(&self, dag_id: &str) -> EventSender {
        let (tx, _rx) = broadcast::channel(256);
        self.event_channels.insert(dag_id.to_string(), tx.clone());

        tx
    }

    pub fn get_event_sender(&self, dag_id: &str) -> Option<EventSender> {
        self.event_channels.get(dag_id).map(|entry| entry.clone())
    }

    pub fn subsrcibe_to_events(&self, dag_id: &str) -> Option<broadcast::Receiver<DagEvent>> {
        self.event_channels
            .get(dag_id)
            .map(|sender| sender.subscribe())
    }

    pub fn get_dag(&self, dag_id: &str) -> Option<Dag> {
        self.dags.get(dag_id).map(|entry| entry.clone())
    }

    pub fn upsert_dag(&self, dag: Dag) {
        self.dags.insert(dag.id.clone(), dag);
    }

    pub fn list_dag_ids(&self) -> Vec<String> {
        self.dags.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
