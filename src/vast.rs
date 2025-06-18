use crate::{
    config::Config,
    types::{Offer, VAST_BASE_URL, VastInstance},
};
use anyhow::{Result, anyhow};
use log::info;

pub struct VastClient {
    config: Config,
    client: reqwest::Client,
    base_url: String,
}

impl VastClient {
    pub fn new(config: Config) -> Self {
        let client = reqwest::Client::new();
        let base_url = VAST_BASE_URL.to_string();
        Self {
            config,
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
        let query = self.config.vast_query.to_query_string();

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

    pub fn filter_out(config: Config, offers: Vec<Offer>) -> Vec<Offer> {
        info!("Num offers before filter: {}", offers.len());

        let bad_hosts = config.bad_hosts;
        let bad_machines = config.bad_machines;

        let offers: Vec<Offer> = offers
            .into_iter()
            .filter(|offer| {
                let host_ok = bad_hosts
                    .as_ref()
                    .map_or(true, |bad_list| !bad_list.contains(&offer.host_id));

                let machine_ok = bad_machines
                    .as_ref()
                    .map_or(true, |bad_list| !bad_list.contains(&offer.machine_id));

                host_ok && machine_ok
            })
            .collect();

        info!("Num offers after filter: {}", offers.len());

        offers
    }
}
