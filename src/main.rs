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
use types::{
    MagisterState, Template, TemplateResponse, VAST_BASE_URL, VAST_TEMPLATE_INFORMATION_ENDPOINT,
};
use vast::VastClient;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");

    env_logger::init();

    let config = tokio::fs::read_to_string("magister.toml")
        .await
        .context("read magister.toml file")?;

    let config: Config = toml::de::from_str(&config).context("parse config")?;

    let new_instance_args = match get_template_information(config.clone()).await {
        Ok(template) => {
            todo!()
        }
        Err(e) => {
            error!("Error getting information on template {e}");
        }
    };

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

async fn get_template_information(config: Config) -> Result<Template> {
    let template_hash = config.template_hash;
    info!("Getting information on template {template_hash}");
    let url = format!(
        r#"{}{}/?select_filters={{"hash_id":{{"eq":"{}"}}}}"#,
        VAST_BASE_URL, VAST_TEMPLATE_INFORMATION_ENDPOINT, template_hash
    );

    let response = reqwest::Client::new()
        .get(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", config.vast_api_key))
        .send()
        .await?;

    if response.status().is_success() {
        let template_response: TemplateResponse = response.json().await?;
        if template_response.templates.len() == 0 {
            Err(anyhow!(
                "No templates returned for template hash {template_hash}"
            ))
        } else if template_response.templates.len() > 1 {
            Err(anyhow!(
                "More than 1 templates returned for template hash {template_hash}"
            ))
        } else {
            Ok(template_response.templates[0].clone())
        }
    } else {
        let status = response.status();
        let error_text = response.text().await?;
        Err(anyhow!(
            "API request for {url} failed with status {status}: {error_text}"
        ))
    }
}

async fn validate_query(config: Config, new_instance_args: String) -> Result<()> {
    info!("Validating query...");
    let vast_client = VastClient::new(config.clone(), new_instance_args);
    let start = Instant::now();
    let offers = vast_client.find_offers().await?;

    if offers.len() == 0 {
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
