use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{any, get, post, put},
};
use std::sync::Arc;

use crate::types::MagisterState;

pub fn create_router(state: Arc<MagisterState>) -> Router {
    Router::new()
        .route("/hello", get(hello_world))
        .with_state(state)
}

async fn hello_world(State(state): State<Arc<MagisterState>>) -> impl IntoResponse {
    format!("Hello world!")
}
