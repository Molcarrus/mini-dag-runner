use crate::{models::dag::Dag, state::AppState};

impl AppState {
    pub fn update_dag<F>(&self, dag_id: &str, updater: F) -> bool
    where
        F: FnOnce(&mut Dag),
    {
        if let Some(mut entry) = self.dags.get_mut(dag_id) {
            updater(&mut entry);
            true
        } else {
            false
        }
    }

    pub fn get_all_dags(&self) -> Vec<Dag> {
        self.dags
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn remove_dag(&self, dag_id: &str) -> bool {
        let dag_removed = self.dags.remove(dag_id).is_some();
        let channel_removed = self.event_channels.remove(dag_id).is_some();

        dag_removed || channel_removed
    }
}
