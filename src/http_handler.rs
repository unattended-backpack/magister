use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
};
use log::{error, info};
use std::sync::Arc;

use crate::types::{MagisterState, VastInstance};

pub fn create_router(state: Arc<MagisterState>) -> Router {
    Router::new()
        .route("/hello", get(hello_world))
        .route("/drop/:id", delete(drop))
        .route("/instances", get(instances))
        .with_state(state)
}

async fn hello_world() -> impl IntoResponse {
    format!("Hello world!")
}

async fn instances(
    State(state): State<Arc<MagisterState>>,
) -> Result<axum::Json<Vec<VastInstance>>, StatusCode> {
    match state.instance_controller_client.instances().await {
        Ok(instances) => Ok(axum::Json(instances)),
        Err(e) => {
            error!("Error getting instances: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn drop(
    State(state): State<Arc<MagisterState>>,
    Path(id): Path<String>,
    body: Option<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let instance_id: u64 = match id.parse() {
        Ok(id) => id,
        Err(e) => {
            error!("Error parsing {id} as u64 in drop request: {e}");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    match body {
        Some(reason) => {
            info!("Received request to drop {instance_id} with reason: {reason}");
        }
        None => {
            info!("Received request to drop {instance_id}");
        }
    }

    match state.instance_controller_client.drop(instance_id).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Error getting instances: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
