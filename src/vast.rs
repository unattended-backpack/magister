use std::time::Duration;

use crate::{
    config::Config,
    types::{
        Offer, VAST_BASE_URL, VAST_CREATE_INSTANCE_ENDPOINT, VAST_DELETE_INSTANCE_ENDPOINT,
        VAST_OFFERS_ENDPOINT, VastCreateInstanceResponse, VastInstance, VastOfferResponse,
    },
};
use anyhow::{Context, Result, anyhow};
use log::{debug, info, warn};

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
        } else if offers.len() < count {
            warn!(
                "Query only returned {} offers but {} instances were requested.  Try restarting with a less strict query parameters to return more results",
                offers.len(),
                count
            );
        } else {
            info!("Query returned {} offers", offers.len());
        }

        for offer in offers {
            tokio::time::sleep(Duration::from_secs(self.config.vast_api_call_delay_secs)).await;
            let offer_id = offer.id;

            match self.request_new_instance(offer_id).await {
                Ok(instance_id) => {
                    let new_instance = VastInstance::new(instance_id, offer);
                    info!("Accepted offer {offer_id} for {new_instance}");
                    instances.push((instance_id, new_instance));
                }
                Err(e) => {
                    warn!(
                        "Unable to request offer {offer_id} of a {} in {} with machine_id {} and host_id {} for ${:.2}/hour.\nError: {e}",
                        offer.gpu_name,
                        offer.geolocation,
                        offer.machine_id,
                        offer.host_id,
                        offer.dph_total
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
        // TODO: look for good machines
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
            debug!("Found {} offers", vast_response.offers.len());
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
        // unfortunately these all have to be passed in as null
        let body = format!(
            r#"{{
            "template_id": null,
            "template_hash_id": "{}",
            "client_id": null,
            "image": null,
            "env": null,
            "args_str": null,
            "onstart": null,
            "runtype": null,
            "image_login": null,
            "use_jupyter_lab": false,
            "jupyter_dir": null,
            "python_utf8": null,
            "lang_utf8": null,
            "label": "magister",
            "disk": {}
        }}"#,
            self.config.template_hash, self.config.vast_query.disk_space
        );

        let response = self
            .client
            .put(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.config.vast_api_key),
            )
            .body(body.clone())
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
    let count_before_filter = offers.len();

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

    let count_after_filter = offers.len();
    debug!(
        "Filtered out {} offers",
        count_before_filter - count_after_filter
    );

    offers
}
