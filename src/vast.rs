use crate::{
    config::Config,
    types::{
        Offer, VAST_BASE_URL, VAST_CREATE_INSTANCE_ENDPOINT, VAST_DELETE_INSTANCE_ENDPOINT,
        VAST_OFFERS_ENDPOINT, VastCreateInstanceResponse, VastInstance, VastOfferResponse,
    },
};
use anyhow::{Context, Result, anyhow};
use log::{info, warn};

pub struct VastClient {
    config: Config,
    client: reqwest::Client,
}

impl VastClient {
    pub fn new(config: Config) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    pub async fn create_instances(&self, count: usize) -> Result<Vec<(u64, VastInstance)>> {
        if count == 0 {
            return Ok(Vec::new());
        }
        info!("Requesting {count} new instances");

        let mut instances = Vec::new();
        let offers = self.find_offers().await?;
        if offers.len() == 0 {
            warn!("Query returned 0 offers.");
        } else if offers.len() > count {
            warn!(
                "Query only returned {} offers but {} instances were requested.  Try restarting with a less strict query parameters to return more results",
                offers.len(),
                count
            );
        }
        for offer in offers {
            let offer_id = offer.id;

            match self.request_new_instance(offer_id).await {
                Ok(instance_id) => {
                    let new_instance = VastInstance::new(instance_id, offer);
                    info!("Accepted offer {offer_id} for {new_instance}");
                    instances.push((instance_id, new_instance));
                }
                Err(e) => {
                    warn!(
                        "Unable to request offer {offer_id} of a {} in {} with machine_id {} and host_id {}.\nError: {e}",
                        offer.gpu_name, offer.geolocation, offer.machine_id, offer.host_id
                    );
                }
            };

            if instances.len() == count {
                break;
            }
        }

        Ok(instances)
    }

    pub async fn drop_instance(&self, instance_id: u64) -> Result<()> {
        self.request_destroy_instance(instance_id).await
    }

    pub async fn find_offers(&self) -> Result<Vec<Offer>> {
        let offers = self
            .request_offers()
            .await
            .context("Call to request offers")?;
        let filtered_offers = filter_offers(self.config.clone(), offers);
        Ok(filtered_offers)
    }

    // TODO: retry logic
    async fn request_destroy_instance(&self, instance_id: u64) -> Result<()> {
        let url = format!(
            "{}{}/{}/",
            VAST_BASE_URL, VAST_DELETE_INSTANCE_ENDPOINT, instance_id
        );

        let response = self
            .client
            .delete(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.config.vast_api_key),
            )
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await?;
            Err(anyhow!(
                "API request for {url} failed with status {status}: {error_text}"
            ))
        }
    }

    // TODO: retry logic
    async fn request_offers(&self) -> Result<Vec<Offer>> {
        let query = self.config.vast_query.to_query_string();
        let url = format!("{}{}/?q={}", VAST_BASE_URL, VAST_OFFERS_ENDPOINT, query);

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.config.vast_api_key),
            )
            .send()
            .await?;

        if response.status().is_success() {
            let vast_response: VastOfferResponse = response.json().await?;
            info!("Found {} offers", vast_response.offers.len());
            Ok(vast_response.offers)
        } else {
            let status = response.status();
            let error_text = response.text().await?;
            Err(anyhow!(
                "API request for {url} failed with status {status}: {error_text}"
            ))
        }
    }

    // returns instance_id of the offer on a success
    // TODO: retry logic
    async fn request_new_instance(&self, offer_id: u64) -> Result<u64> {
        let url = format!(
            "{}{}/{}/",
            VAST_BASE_URL, VAST_CREATE_INSTANCE_ENDPOINT, offer_id
        );
        let body = format!(
            r#"{{"template_hash_id": "{}", "label": "magister"}}"#,
            self.config.template_hash_id
        );
        info!("new instance query:\n{url} body: {body}");
        let response = self
            .client
            .put(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.config.vast_api_key),
            )
            .json(&body)
            .send()
            .await?;
        if response.status().is_success() {
            let resp: VastCreateInstanceResponse = response.json().await?;
            Ok(resp.new_contract)
        } else {
            let status = response.status();
            let error_text = response.text().await?;
            Err(anyhow!(
                "API request for {url} with body {body} failed with status {status}: {error_text}"
            ))
        }
    }
}

fn filter_offers(config: Config, offers: Vec<Offer>) -> Vec<Offer> {
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
