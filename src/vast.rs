use std::time::Duration;

use crate::{
    config::Config,
    types::{
        Offer, VAST_BASE_URL, VAST_CREATE_INSTANCE_ENDPOINT, VAST_DELETE_INSTANCE_ENDPOINT,
        VAST_OFFERS_ENDPOINT, VastCreateInstanceResponse, VastInstance, VastOfferResponse,
    },
};
use anyhow::{Context, Result, anyhow};
use axum::http::StatusCode;
use log::{debug, error, info, warn};

pub struct VastClient {
    config: Config,
    client: reqwest::Client,
}

impl VastClient {
    pub fn new(config: Config) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    pub async fn create_initial_instances(&self, count: usize) -> Result<Vec<(u64, VastInstance)>> {
        let offers = self.find_offers().await?;

        if offers.len() < count {
            let err = format!(
                "Only found {} offers but {} instances were requested. Restart with a less restrictive query.",
                offers.len(),
                count
            );
            error!("{err}");
            return Err(anyhow!(err));
        }

        let mut new_instances = Vec::new();
        let mut i = 0;
        let backoff = self.config.vast_api_call_backoff_secs;
        let mut current_sleep_duration = backoff;
        let mut last_run_rate_limited = false;
        while new_instances.len() != count {
            let offer = match offers.get(i) {
                Some(o) => o,
                None => {
                    return Err(anyhow!(
                        "Ran out of offers.  Try a less restrictive query or try again later."
                    ));
                }
            };
            let offer_id = offer.id;

            match self.request_new_instance(offer_id).await {
                Ok(Some(instance_id)) => {
                    last_run_rate_limited = false;
                    let new_instance = VastInstance::new(instance_id, offer.clone());
                    info!("Accepted offer {offer_id} for {new_instance}");
                    new_instances.push((instance_id, new_instance));
                }
                Ok(None) => {
                    if last_run_rate_limited {
                        current_sleep_duration += backoff;
                    } else {
                        current_sleep_duration = backoff;
                    }
                    last_run_rate_limited = true;
                    warn!(
                        "Reached vast rate limit.  Sleeping for {} seconds then trying again",
                        current_sleep_duration
                    );
                    tokio::time::sleep(Duration::from_secs(current_sleep_duration)).await;
                    // loop without incrementing i to attempt this machine again
                    continue;
                }
                Err(e) => {
                    last_run_rate_limited = false;
                    warn!(
                        "Unable to request offer {offer_id} of a {} in {} with machine_id {} and host_id {} for ${:.2}/hour.\nError: {e}",
                        offer.gpu_name,
                        offer.geolocation,
                        offer.machine_id,
                        offer.host_id,
                        offer.dph_total
                    );
                }
            }

            i += 1;
        }

        Ok(new_instances)
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
        info!("found {} offers", filtered_offers.len());
        Ok(filtered_offers)
    }

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
            .await
            .context("Reqwest call to get vast offers")?;

        if response.status().is_success() {
            let vast_response: VastOfferResponse = match response.json().await {
                Ok(x) => x,
                Err(e) => {
                    let err =
                        format!("Error parsing vast response from offer request as json: {e}");
                    error!("{err}");
                    return Err(anyhow!(err));
                }
            };
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
    // if Ok(None), then we are making too many requests and need to wait
    pub async fn request_new_instance(&self, offer_id: u64) -> Result<Option<u64>> {
        let url = format!(
            "{}{}/{}/",
            VAST_BASE_URL, VAST_CREATE_INSTANCE_ENDPOINT, offer_id
        );

        // remove a trailing / if it exists on the address
        let this_magister_addr = self
            .config
            .this_magister_addr
            .strip_suffix('/')
            .unwrap_or(&self.config.this_magister_addr);

        // this onstart overrides the onstart from the template.  We have to pass in
        // MAGISTER_DROP_ENDPOINT here instead of the the `extra_env` field because the `extra_env` field
        // doesn't properly combine envs if the template already has an ENV.
        let onstart = format!(
            r#""export MAGISTER_DROP_ENDPOINT=\"{}:{}/drop/{}\" chmod +x /entrypoint.sh;bash /entrypoint.sh""#,
            this_magister_addr, self.config.http_port, offer_id
        );
        debug!("onstart command: \n{onstart}");

        // unfortunately these all have to be passed in as null
        let body = format!(
            r#"{{
            "template_id": null,
            "template_hash_id": "{}",
            "client_id": null,
            "image": null,
            "extra_env": null,
            "args_str": null,
            "onstart": {onstart},
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

        debug!("New instance request body:\n{body}");

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
            Ok(Some(resp.new_contract))
        } else if response.status() == StatusCode::TOO_MANY_REQUESTS {
            Ok(None)
        } else {
            let status = response.status();
            let error_text = response.text().await?;
            Err(anyhow!(
                "API request for {url} failed with status {status}: {error_text}"
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
