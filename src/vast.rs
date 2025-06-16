use crate::{
    config::VastConfig,
    types::{Offer, VAST_BASE_URL, VastInstance},
};
use anyhow::{Result, anyhow};
use log::info;

pub struct VastClient {
    template_id: String,
    config: VastConfig,
    client: reqwest::Client,
    base_url: String,
}

impl VastClient {
    pub fn new(template_id: String, vast_config: VastConfig) -> Self {
        let client = reqwest::Client::new();
        let base_url = VAST_BASE_URL.to_string();
        Self {
            template_id,
            config: vast_config,
            client,
            base_url,
        }
    }

    pub async fn new_instance(&self) -> Result<(String, VastInstance)> {
        todo!()
    }

    pub async fn drop_instance(&self, instance_id: &str) -> Result<()> {
        todo!()
    }

    async fn find_offers(&self) -> Result<Vec<Offer>> {
        let url = format!("{}/search/asks/", self.base_url);
        let query = self.config.query.clone();
        info!("Query: {query}");

        let response = self
            .client
            .put(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&query)
            .send()
            .await?;

        if response.status().is_success() {
            let vast_response = response.json().await?;
            Ok(vast_response)
        } else {
            let status = response.status();
            let error_text = response.text().await?;
            Err(anyhow!(
                "API request failed with status {}: {}",
                status,
                error_text
            ))
        }
    }
}
