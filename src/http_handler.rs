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

use crate::{
    types::{
        MagisterState, Offer, VAST_BASE_URL, VAST_OFFERS_ENDPOINT, VastInstance, VastOfferResponse,
    },
    vast::VastClient,
};

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
    let query = state.config.vast_query.to_query_string();
    let url = format!("{}{}/?q={}", VAST_BASE_URL, VAST_OFFERS_ENDPOINT, query);
    info!("url:\n{url}");

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header(
            "Authorization",
            format!("Bearer {}", state.config.vast_api_key),
        )
        //.json(&query)
        .send()
        .await
        .unwrap();

    if response.status().is_success() {
        let response_text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                panic!("Error reading response body: {e}");
            }
        };
        let vast_response: VastOfferResponse = match serde_json::from_str(&response_text) {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("Raw response body: {}", response_text);
                eprintln!("Deserialization error: {}", e);
                panic!("Error deserializing response as json: {e}");
            }
        };
        // let vast_response: VastOfferResponse = match response.json().await {
        //     Ok(resp) => resp,
        //     Err(e) => {
        //         panic!("Error deserializing response as json: {e}");
        //     }
        // };
        info!("Found {} offers", vast_response.offers.len());
        let offers = VastClient::filter_out(state.config.clone(), vast_response.offers);
        for offer in &offers {
            info!(
                "{}: {} {:.2}",
                offer.geolocation, offer.gpu_name, offer.dph_total
            );
        }
        let vast_response = VastOfferResponse { offers };
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
