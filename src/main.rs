mod config;
mod http_handler;
mod instance_controller;
mod types;
mod vast;

use anyhow::{Context, Result};
pub use config::Config;
use log::info;
use std::{net::SocketAddr, sync::Arc};
pub use types::MagisterState;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");

    env_logger::init();

    let config = tokio::fs::read_to_string("magister.toml")
        .await
        .context("read magister.toml file")?;

    let config: Config = toml::de::from_str(&config).context("parse config")?;

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
