mod config;
mod http_handler;
mod instance_controller;
mod types;
mod vast;

use anyhow::{Context, Result, anyhow};
pub use config::Config;
use config::VastQueryConfig;
use log::{error, info};
use std::{net::SocketAddr, sync::Arc};
use types::{MagisterState, VAST_BASE_URL, VAST_OFFERS_ENDPOINT, VastOfferResponse};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");

    env_logger::init();

    let config = tokio::fs::read_to_string("magister.toml")
        .await
        .context("read magister.toml file")?;

    let config: Config = toml::de::from_str(&config).context("parse config")?;

    // match validate_query(config.clone()).await {
    //     Ok(_) => {
    //         info!("Query validated");
    //     }
    //     Err(e) => {
    //         error!("Error validating query: {e}");
    //         error!("Couldn't execute query. Shutting down.");
    //         return Ok(());
    //     }
    // }

    let state = Arc::new(MagisterState::new(config.clone()).await);

    // Create the axum router with all routes
    let app = http_handler::create_router(state);

    let http_addr: SocketAddr = ([0, 0, 0, 0], config.http_port).into();
    // Run the HTTP server in a separate task
    let http_server = tokio::spawn(async move {
        info!("HTTP server starting on {http_addr}");
        axum::serve(
            tokio::net::TcpListener::bind(http_addr)
                .await
                .context("bind http server to {http_addr}")
                .unwrap(),
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .context("Axum serve on {http_addr}")
        .unwrap();
    });

    http_server.await?;

    Ok(())
}

// async fn validate_query(config: Config) -> Result<()> {
//     let query = config.vast_query.to_json_query_string();
//     info!("Validating query...\n{query}");
//
//     let url = format!("{}{}", VAST_BASE_URL, VAST_OFFERS_ENDPOINT);
//
//     let client = reqwest::Client::new();
//     let response = client
//         .get(&url)
//         .header("Accept", "application/json")
//         .header("Content-Type", "application/json")
//         .header("Authorization", format!("Bearer {}", config.vast_api_key))
//         .json(&query)
//         .send()
//         .await?;
//
//     if response.status().is_success() {
//         let vast_response: VastOfferResponse = response.json().await?;
//         let num_offers = vast_response.offers.len();
//         if num_offers > 0 {
//             Ok(())
//         } else {
//             Err(anyhow!(
//                 "Reponse returned 0 vast offers.  Change the query to return more"
//             ))
//         }
//     } else {
//         let status = response.status();
//         let error_text = response.text().await.unwrap();
//         Err(anyhow!(
//             "Error in response: status = {status}, {error_text}"
//         ))
//     }
// }
