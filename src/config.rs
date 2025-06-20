use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    pub vast_query: VastQueryConfig,
    pub vast_api_key: String,
    // how many seconds to wait between each vast api call so we don't get rate limited
    // TODO: have a backoff
    #[serde(default = "default_vast_api_cal_delay_secs")]
    pub vast_api_call_delay_secs: u64,
    #[serde(default = "default_task_polling_interval_secs")]
    pub task_polling_interval_secs: u64,
    // Id of the template that magister will be making instances of.
    // Find the id at the Vast.ai web console
    pub template_hash: String,
    // how many instances of the template this Magister will make sure are allocated
    pub number_instances: usize,
    // Won't use a machine if its in bad_hosts OR bad_machines
    pub bad_hosts: Option<Vec<u64>>,
    pub bad_machines: Option<Vec<u64>>,
    // Will prioritize a machine if its in good_hosts OR good_machines
    pub good_hosts: Option<Vec<u64>>,
    pub good_machines: Option<Vec<u64>>,
}

fn default_vast_api_cal_delay_secs() -> u64 {
    2
}

fn default_task_polling_interval_secs() -> u64 {
    30
}

fn default_http_port() -> u16 {
    8555
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VastQueryConfig {
    // in gb.  ex: 16
    pub allocated_storage: u16,
    // ex: "RTX 4090"
    pub gpu_name: String,
    // percent 0-1 ex: 0.98
    pub reliability: f64,
    // ex: 12.8
    pub min_cuda_version: f64,
    // in gb ex: 21
    pub gpu_ram: u64,
    // in gb ex: 16
    pub disk_space: u64,
    // ex: 192679
    pub duration: f64,
    // In USD ex: 0.53
    pub cost_per_hour: f64,
}

impl VastQueryConfig {
    pub fn to_query_string(&self) -> String {
        let mut query = String::new();

        write!(query, r#"{{"#).unwrap();
        write!(query, r#""disk_space":{{"gte": {}}},"#, self.disk_space).unwrap();
        write!(query, r#""reliability2":{{"gte":{}}},"#, self.reliability).unwrap();
        write!(query, r#""duration":{{"gte":{}}},"#, self.duration).unwrap();
        write!(query, r#""verified":{{"eq":true}}, "#).unwrap();
        write!(query, r#""dph_total":{{"lte":{}}},"#, self.cost_per_hour).unwrap();
        write!(query, r#""gpu_ram":{{"gte":{}000}},"#, self.gpu_ram).unwrap();
        write!(query, r#""sort_option": {{"0":["score", "desc"]}},"#).unwrap();
        write!(query, r#""rentable":{{"eq":true}}, "#).unwrap();
        write!(
            query,
            r#""cuda_max_good":{{"gte":"{}"}},"#,
            self.min_cuda_version
        )
        .unwrap();
        write!(query, r#""gpu_name":{{"in":["{}"]}},"#, self.gpu_name).unwrap();
        write!(query, r#""allocated_storage":{},"#, self.allocated_storage).unwrap();
        write!(query, r#""order": [["score", "desc"]],"#).unwrap();
        write!(query, r#""type":"ask""#).unwrap();
        write!(query, "}}").unwrap();

        query
    }
}
