use axum::{
    Router,
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{any, delete, get, post, put},
};
use log::info;
use std::sync::Arc;

use crate::types::{MagisterState, Offer, VAST_BASE_URL, VastInstance, VastOfferResponse};

pub fn create_router(state: Arc<MagisterState>) -> Router {
    Router::new()
        .route("/hello", get(hello_world))
        .route("/deallocate/:id", delete(deallocate))
        .route("/instances", get(instances))
        .route("/find_offers", get(find_offers))
        .with_state(state)
}

async fn hello_world(State(state): State<Arc<MagisterState>>) -> impl IntoResponse {
    format!("Hello world!")
}

async fn find_offers(
    State(state): State<Arc<MagisterState>>,
) -> Result<axum::Json<VastOfferResponse>, StatusCode> {
    let url = format!("{}/search/asks/", VAST_BASE_URL);
    let query = state.config.vast_config.query.clone();
    info!("Query: {query}");

    let client = reqwest::Client::new();
    let response = client
        .put(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&query)
        .send()
        .await
        .unwrap();

    if response.status().is_success() {
        let vast_response = response.json().await.unwrap();
        Ok(axum::Json(vast_response))
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap();
        panic!("API request failed with status {}: {}", status, error_text)
    }
}

async fn instances(
    State(state): State<Arc<MagisterState>>,
) -> Result<axum::Json<Vec<(String, VastInstance)>>, StatusCode> {
    Ok(axum::Json(vec![]))
}

async fn deallocate(
    State(state): State<Arc<MagisterState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
}
