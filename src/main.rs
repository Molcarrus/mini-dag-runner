use axum::{
    Router,
    routing::{delete, get, post},
};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::state::AppState;

mod errors;
mod models;
mod routes;
mod services;
mod state;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mini_dag_runner=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState::new();

    let serve_dir = ServeDir::new("static").not_found_service(ServeFile::new("static/index.html"));

    let app = Router::new()
        .route("/api/dags", post(routes::submit::submit_dag))
        .route("/api/dags", get(routes::list::list_dags))
        .route("/api/dags/{id}/status", get(routes::status::get_dag_status))
        .route(
            "/api/dags/{id}/stream",
            get(routes::stream::stream_dag_events),
        )
        .route("/api/dags/{id}", delete(routes::delete::delete_dag))
        .route("/api/health", get(health_check))
        .fallback_service(serve_dir)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    tracing::info!(
        "Mini DAG Runner listening on {}",
        listener.local_addr().unwrap()
    );
    tracing::info!("Web UI available at http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}
