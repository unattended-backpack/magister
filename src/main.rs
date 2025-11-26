mod config;
mod http_handler;
mod instance_controller;
mod types;
mod vast;

use anyhow::{Context, Result, anyhow};
pub use config::Config;
use log::{error, info};
use std::{net::SocketAddr, sync::Arc};
use tokio::time::Instant;
use types::MagisterState;
use vast::VastClient;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let config = Config::load("magister.toml").context("load configuration")?;

    // validate query.  Exit on query error or 0 (or less than desired instances) results returned
    match validate_query(config.clone()).await {
        Ok(_) => {
            info!("Query validated");
        }
        Err(e) => {
            error!("Error validating query: {e}");
            error!("Couldn't validate query. Shutting down.");
            return Ok(());
        }
    }

    let state = Arc::new(
        MagisterState::new(config.clone())
            .await
            .context("Create MagisterState")?,
    );

    // Create the axum router with all routes
    let app = http_handler::create_router(state);

    let http_addr: SocketAddr = ([0, 0, 0, 0], config.http_port).into();

    // Create a broadcast channel for shutdown signal
    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);
    let shutdown_tx_clone = shutdown_tx.clone();

    // Spawn a task to listen for ctrl+c and broadcast shutdown
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C signal handler");
        info!("Received shutdown signal, stopping server...");
        let _ = shutdown_tx_clone.send(());
    });

    // Create shutdown signal handler for HTTP server
    let mut http_shutdown_rx = shutdown_tx.subscribe();
    let http_shutdown_signal = async move {
        let _ = http_shutdown_rx.recv().await;
    };

    // Run the HTTP server with graceful shutdown
    let http_server = tokio::spawn(async move {
        info!("HTTP server starting on {http_addr}");
        axum::serve(
            tokio::net::TcpListener::bind(http_addr)
                .await
                .context("bind http server to {http_addr}")
                .unwrap(),
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(http_shutdown_signal)
        .await
        .context("Axum serve on {http_addr}")
        .unwrap();
    });

    info!("Magister started. Press Ctrl+C to stop.");

    http_server.await?;
    info!("HTTP server shutdown complete");

    Ok(())
}

async fn validate_query(config: Config) -> Result<()> {
    info!("Validating query...");
    let vast_client = VastClient::new(config.clone());
    let start = Instant::now();
    let offers = vast_client
        .find_offers(0)
        .await
        .context("Call find_offers")?;

    if offers.is_empty() {
        Err(anyhow!(
            "query returned 0 offers. Query might be incorrectly constructed or too strict"
        ))
    } else if offers.len() < config.number_instances {
        Err(anyhow!(
            "Query returned {} instance offers but this Magister is configured to run {} instances. Loosen the restrictions on the query to return more results.",
            offers.len(),
            config.number_instances
        ))
    } else {
        info!(
            "Validation query returned {} offers in {:.2} seconds",
            offers.len(),
            start.elapsed().as_secs_f32()
        );
        Ok(())
    }
}
