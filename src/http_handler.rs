use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
};
use log::{error, info};
use std::sync::Arc;

use crate::types::{MagisterState, SummaryResponse, VastInstance};

pub fn create_router(state: Arc<MagisterState>) -> Router {
    Router::new()
        .route("/hello", get(hello_world))
        .route("/drop/:id", delete(drop))
        .route("/instances", get(instances))
        .route("/summary", get(summary))
        .route("/verify/:id", get(verify))
        .with_state(state)
}

async fn hello_world() -> impl IntoResponse {
    format!("Hello world!")
}

async fn verify(
    State(state): State<Arc<MagisterState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let offer_id: u64 = match id.parse() {
        Ok(id) => id,
        Err(e) => {
            error!("Error parsing {id} as u64 in drop request: {e}");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    match state.instance_controller_client.verify(offer_id).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Error verifying instance: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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

async fn summary(
    State(state): State<Arc<MagisterState>>,
) -> Result<axum::Json<SummaryResponse>, StatusCode> {
    let mut instances = match state.instance_controller_client.instances().await {
        Ok(instances) => instances,
        Err(e) => {
            error!("Error getting instances: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // only keep instances that we aren't about to drop
    instances.retain(|instance| !instance.should_drop);

    let total_dph = instances
        .iter()
        .map(|instance| instance.offer.dph_total)
        .sum();

    let num_instances = instances.len();

    let instance_overview = instances
        .into_iter()
        .map(|instance| instance.into())
        .collect();

    let summary = SummaryResponse {
        total_cost_per_hour: total_dph,
        num_instances,
        instance_overview,
    };

    Ok(axum::Json(summary))
}

async fn drop(
    State(state): State<Arc<MagisterState>>,
    Path(id): Path<String>,
    body: Option<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let offer_id: u64 = match id.parse() {
        Ok(id) => id,
        Err(e) => {
            error!("Error parsing {id} as u64 in drop request: {e}");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    match body {
        Some(reason) => {
            info!("Received request to drop instance of offer {offer_id} with reason: {reason}");
        }
        None => {
            info!("Received request to drop instance of offer {offer_id}");
        }
    }

    match state.instance_controller_client.drop(offer_id).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Error getting instances: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
