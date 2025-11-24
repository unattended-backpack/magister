use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt::Write;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    // the address at which the hierophant can reach this magister to make drop requests.  This
    // will get passed into the contemplant who will then notify the Hierophant that this is the
    // contemplant's managing Magister
    pub this_magister_addr: String,
    // Passed into Contemplants to tell them which Hierophant to connect to.  Needs to be publically
    // accessible.
    pub hierophant_ip: String,
    // HTTP port the Hierophant (at above ip) is running at.
    pub hierophant_http_port: u16,
    pub vast_query: VastQueryConfig,
    pub vast_api_key: String,
    // how many seconds to wait between each vast api call so we don't get rate limited
    #[serde(default = "vast_api_call_backoff_secs")]
    pub vast_api_call_backoff_secs: u64,
    #[serde(default = "default_task_polling_interval_secs")]
    pub task_polling_interval_secs: u64,
    // How long to wait for verification from the contemplant before dropping this instance.
    // Contemplant verification happens on startup
    #[serde(default = "default_contemplant_verification_timeout_secs")]
    pub contemplant_verification_timeout_secs: u64,
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

fn default_contemplant_verification_timeout_secs() -> u64 {
    180
}

fn vast_api_call_backoff_secs() -> u64 {
    10
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
    // Max cost per hour in USD ex: 0.53
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

impl Config {
    /// Load configuration from .toml file and/or environment variables.
    /// Priority: environment variables > .toml file > defaults
    /// The .toml file is optional if all required fields are provided via environment variables.
    pub fn load(config_path: &str) -> Result<Self> {
        // Try to load from file if it exists
        let mut config = if Path::new(config_path).exists() {
            let contents = std::fs::read_to_string(config_path)
                .context(format!("Failed to read config file: {}", config_path))?;
            toml::from_str::<Config>(&contents)
                .context(format!("Failed to parse config file: {}", config_path))?
        } else {
            // No file exists, create config with defaults (required fields will be empty)
            Config {
                http_port: default_http_port(),
                this_magister_addr: String::new(),
                hierophant_ip: String::new(),
                hierophant_http_port: 0,
                vast_query: VastQueryConfig {
                    allocated_storage: 0,
                    gpu_name: String::new(),
                    reliability: 0.0,
                    min_cuda_version: 0.0,
                    gpu_ram: 0,
                    disk_space: 0,
                    duration: 0.0,
                    cost_per_hour: 0.0,
                },
                vast_api_key: String::new(),
                vast_api_call_backoff_secs: vast_api_call_backoff_secs(),
                task_polling_interval_secs: default_task_polling_interval_secs(),
                contemplant_verification_timeout_secs: default_contemplant_verification_timeout_secs(),
                template_hash: String::new(),
                number_instances: 0,
                bad_hosts: None,
                bad_machines: None,
                good_hosts: None,
                good_machines: None,
            }
        };

        // Override with environment variables if present
        if let Ok(val) = env::var("HTTP_PORT") {
            config.http_port = val.parse().context("HTTP_PORT must be a valid u16")?;
        }
        if let Ok(val) = env::var("THIS_MAGISTER_ADDR") {
            config.this_magister_addr = val;
        }
        if let Ok(val) = env::var("HIEROPHANT_IP") {
            config.hierophant_ip = val;
        }
        if let Ok(val) = env::var("HIEROPHANT_HTTP_PORT") {
            config.hierophant_http_port = val.parse().context("HIEROPHANT_HTTP_PORT must be a valid u16")?;
        }
        if let Ok(val) = env::var("VAST_API_KEY") {
            config.vast_api_key = val;
        }
        if let Ok(val) = env::var("VAST_API_CALL_BACKOFF_SECS") {
            config.vast_api_call_backoff_secs = val.parse().context("VAST_API_CALL_BACKOFF_SECS must be a valid u64")?;
        }
        if let Ok(val) = env::var("TASK_POLLING_INTERVAL_SECS") {
            config.task_polling_interval_secs = val.parse().context("TASK_POLLING_INTERVAL_SECS must be a valid u64")?;
        }
        if let Ok(val) = env::var("CONTEMPLANT_VERIFICATION_TIMEOUT_SECS") {
            config.contemplant_verification_timeout_secs = val.parse().context("CONTEMPLANT_VERIFICATION_TIMEOUT_SECS must be a valid u64")?;
        }
        if let Ok(val) = env::var("TEMPLATE_HASH") {
            config.template_hash = val;
        }
        if let Ok(val) = env::var("NUMBER_INSTANCES") {
            config.number_instances = val.parse().context("NUMBER_INSTANCES must be a valid usize")?;
        }

        // VastQueryConfig overrides
        if let Ok(val) = env::var("VAST_QUERY_ALLOCATED_STORAGE") {
            config.vast_query.allocated_storage = val.parse().context("VAST_QUERY_ALLOCATED_STORAGE must be a valid u16")?;
        }
        if let Ok(val) = env::var("VAST_QUERY_GPU_NAME") {
            config.vast_query.gpu_name = val;
        }
        if let Ok(val) = env::var("VAST_QUERY_RELIABILITY") {
            config.vast_query.reliability = val.parse().context("VAST_QUERY_RELIABILITY must be a valid f64")?;
        }
        if let Ok(val) = env::var("VAST_QUERY_MIN_CUDA_VERSION") {
            config.vast_query.min_cuda_version = val.parse().context("VAST_QUERY_MIN_CUDA_VERSION must be a valid f64")?;
        }
        if let Ok(val) = env::var("VAST_QUERY_GPU_RAM") {
            config.vast_query.gpu_ram = val.parse().context("VAST_QUERY_GPU_RAM must be a valid u64")?;
        }
        if let Ok(val) = env::var("VAST_QUERY_DISK_SPACE") {
            config.vast_query.disk_space = val.parse().context("VAST_QUERY_DISK_SPACE must be a valid u64")?;
        }
        if let Ok(val) = env::var("VAST_QUERY_DURATION") {
            config.vast_query.duration = val.parse().context("VAST_QUERY_DURATION must be a valid f64")?;
        }
        if let Ok(val) = env::var("VAST_QUERY_COST_PER_HOUR") {
            config.vast_query.cost_per_hour = val.parse().context("VAST_QUERY_COST_PER_HOUR must be a valid f64")?;
        }

        // Optional list overrides
        if let Ok(val) = env::var("BAD_HOSTS") {
            let hosts: Result<Vec<u64>, _> = val.split(',').map(|s| s.trim().parse()).collect();
            config.bad_hosts = Some(hosts.context("BAD_HOSTS must be comma-separated u64 values")?);
        }
        if let Ok(val) = env::var("BAD_MACHINES") {
            let machines: Result<Vec<u64>, _> = val.split(',').map(|s| s.trim().parse()).collect();
            config.bad_machines = Some(machines.context("BAD_MACHINES must be comma-separated u64 values")?);
        }
        if let Ok(val) = env::var("GOOD_HOSTS") {
            let hosts: Result<Vec<u64>, _> = val.split(',').map(|s| s.trim().parse()).collect();
            config.good_hosts = Some(hosts.context("GOOD_HOSTS must be comma-separated u64 values")?);
        }
        if let Ok(val) = env::var("GOOD_MACHINES") {
            let machines: Result<Vec<u64>, _> = val.split(',').map(|s| s.trim().parse()).collect();
            config.good_machines = Some(machines.context("GOOD_MACHINES must be comma-separated u64 values")?);
        }

        // Validate required fields
        if config.this_magister_addr.is_empty() {
            anyhow::bail!(
                "this_magister_addr is required. Provide it via config file or THIS_MAGISTER_ADDR environment variable."
            );
        }
        if config.hierophant_ip.is_empty() {
            anyhow::bail!(
                "hierophant_ip is required. Provide it via config file or HIEROPHANT_IP environment variable."
            );
        }
        if config.hierophant_http_port == 0 {
            anyhow::bail!(
                "hierophant_http_port is required. Provide it via config file or HIEROPHANT_HTTP_PORT environment variable."
            );
        }
        if config.vast_api_key.is_empty() {
            anyhow::bail!(
                "vast_api_key is required. Provide it via config file or VAST_API_KEY environment variable."
            );
        }
        if config.template_hash.is_empty() {
            anyhow::bail!(
                "template_hash is required. Provide it via config file or TEMPLATE_HASH environment variable."
            );
        }
        if config.number_instances == 0 {
            anyhow::bail!(
                "number_instances is required. Provide it via config file or NUMBER_INSTANCES environment variable."
            );
        }

        Ok(config)
    }
}
