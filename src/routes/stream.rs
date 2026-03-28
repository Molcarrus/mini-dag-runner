use std::convert::Infallible;

use axum::{
    extract::{Path, State},
    response::{
        IntoResponse, Sse,
        sse::{Event, KeepAlive},
    },
};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use tracing::{info, warn};

use crate::{errors::AppError, models::events::DagEvent, state::AppState};

pub async fn stream_dag_events(
    State(state): State<AppState>,
    Path(dag_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    info!("SSE client connected to DAG: {}", dag_id);

    if state.get_dag(&dag_id).is_none() {
        return Err(AppError::DagNotFound(dag_id));
    }

    let reciever = state
        .subsrcibe_to_events(&dag_id)
        .ok_or_else(|| AppError::DagNotFound(dag_id.clone()))?;

    let stream = BroadcastStream::new(reciever);

    let event_stream = stream.filter_map(move |result| match result {
        Ok(event) => {
            let is_final = matches!(
                event,
                DagEvent::DagCompleted { .. } | DagEvent::Error { .. }
            );

            let sse_event = match serde_json::to_string(&event) {
                Ok(json) => {
                    let event_name = match &event {
                        DagEvent::DagStarted { .. } => "dag_started",
                        DagEvent::TaskStarted { .. } => "task_started",
                        DagEvent::TaskCompleted { .. } => "task_completed",
                        DagEvent::LevelCompleted { .. } => "level_completed",
                        DagEvent::DagCompleted { .. } => "dag_completed",
                        DagEvent::Error { .. } => "error",
                    };

                    Some(Ok::<_, Infallible>(
                        Event::default().event(event_name).data(json),
                    ))
                }
                Err(e) => {
                    warn!("Failed tp serialize event: {}", e);
                    None
                }
            };

            sse_event
        }
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
            warn!("SSE client lagged by {} messages", n);
            Some(Ok(Event::default()
                .event("error")
                .data(format!("Client lagged by {} messages", n))))
        }
    });

    Ok(Sse::new(event_stream).keep_alive(KeepAlive::default()))
}
