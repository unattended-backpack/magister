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
    // Configuration for Contemplants spawned by this Magister
    #[serde(default)]
    pub contemplant: ContemplantConfig,
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
pub struct ContemplantConfig {
    /// Prover type: "cpu" or "cuda" (default: "cpu")
    #[serde(default = "default_prover_type")]
    pub prover_type: String,
    /// Human-readable name for Contemplants (default: generated from names.txt)
    #[serde(default)]
    pub contemplant_name: Option<String>,
    /// Port for HTTP health check server (default: 9011)
    #[serde(default = "default_contemplant_http_port")]
    pub http_port: u16,
    /// Moongate CUDA prover endpoint (default: none)
    #[serde(default)]
    pub moongate_endpoint: Option<String>,
    /// Heartbeat interval in seconds (default: 30)
    #[serde(default = "default_heartbeat_interval_seconds")]
    pub heartbeat_interval_seconds: u64,
    /// Maximum number of finished proofs stored in memory (default: 2)
    #[serde(default = "default_max_proofs_stored")]
    pub max_proofs_stored: usize,
    /// Path to log file for progress tracking (default: "./moongate.log")
    #[serde(default = "default_moongate_log_path")]
    pub moongate_log_path: String,
    /// Log polling interval in milliseconds (default: 2000)
    #[serde(default = "default_watcher_polling_interval_ms")]
    pub watcher_polling_interval_ms: u64,
    /// SSH public keys for debugging access (default: none)
    /// Format: newline-separated SSH public keys
    #[serde(default)]
    pub ssh_authorized_keys: Option<String>,
}

fn default_prover_type() -> String {
    "cpu".to_string()
}

fn default_contemplant_http_port() -> u16 {
    9011
}

fn default_heartbeat_interval_seconds() -> u64 {
    30
}

fn default_max_proofs_stored() -> usize {
    2
}

fn default_moongate_log_path() -> String {
    "./moongate.log".to_string()
}

fn default_watcher_polling_interval_ms() -> u64 {
    2000
}

impl Default for ContemplantConfig {
    fn default() -> Self {
        Self {
            prover_type: default_prover_type(),
            contemplant_name: None,
            http_port: default_contemplant_http_port(),
            moongate_endpoint: None,
            heartbeat_interval_seconds: default_heartbeat_interval_seconds(),
            max_proofs_stored: default_max_proofs_stored(),
            moongate_log_path: default_moongate_log_path(),
            watcher_polling_interval_ms: default_watcher_polling_interval_ms(),
            ssh_authorized_keys: None,
        }
    }
}

impl ContemplantConfig {
    /// Generate environment variable exports for the onstart command.
    /// These will be passed to Contemplants spawned on Vast.ai.
    pub fn to_env_exports(&self) -> String {
        let mut exports = Vec::new();

        // Always export prover type
        exports.push(format!("export PROVER_TYPE=\\\"{}\\\"", self.prover_type));

        // Optional exports
        if let Some(ref name) = self.contemplant_name {
            exports.push(format!("export CONTEMPLANT_NAME=\\\"{}\\\"", name));
        }

        // Always export http_port
        exports.push(format!("export HTTP_PORT=\\\"{}\\\"", self.http_port));

        if let Some(ref endpoint) = self.moongate_endpoint {
            exports.push(format!("export MOONGATE_ENDPOINT=\\\"{}\\\"", endpoint));
        }

        exports.push(format!("export HEARTBEAT_INTERVAL_SECONDS=\\\"{}\\\"", self.heartbeat_interval_seconds));
        exports.push(format!("export MAX_PROOFS_STORED=\\\"{}\\\"", self.max_proofs_stored));
        exports.push(format!("export MOONGATE_LOG_PATH=\\\"{}\\\"", self.moongate_log_path));
        exports.push(format!("export WATCHER_POLLING_INTERVAL_MS=\\\"{}\\\"", self.watcher_polling_interval_ms));

        if let Some(ref keys) = self.ssh_authorized_keys {
            // SSH keys can contain newlines, so we need to escape them for the shell
            let escaped_keys = keys.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
            exports.push(format!("export SSH_AUTHORIZED_KEYS=\\\"{}\\\"", escaped_keys));
        }

        exports.join("; ")
    }
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
                contemplant: ContemplantConfig::default(),
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

        // ContemplantConfig overrides
        if let Ok(val) = env::var("CONTEMPLANT_PROVER_TYPE") {
            config.contemplant.prover_type = val;
        }
        if let Ok(val) = env::var("CONTEMPLANT_NAME") {
            config.contemplant.contemplant_name = Some(val);
        }
        if let Ok(val) = env::var("CONTEMPLANT_HTTP_PORT") {
            config.contemplant.http_port = val.parse().context("CONTEMPLANT_HTTP_PORT must be a valid u16")?;
        }
        if let Ok(val) = env::var("CONTEMPLANT_MOONGATE_ENDPOINT") {
            config.contemplant.moongate_endpoint = Some(val);
        }
        if let Ok(val) = env::var("CONTEMPLANT_HEARTBEAT_INTERVAL_SECONDS") {
            config.contemplant.heartbeat_interval_seconds = val.parse().context("CONTEMPLANT_HEARTBEAT_INTERVAL_SECONDS must be a valid u64")?;
        }
        if let Ok(val) = env::var("CONTEMPLANT_MAX_PROOFS_STORED") {
            config.contemplant.max_proofs_stored = val.parse().context("CONTEMPLANT_MAX_PROOFS_STORED must be a valid usize")?;
        }
        if let Ok(val) = env::var("CONTEMPLANT_MOONGATE_LOG_PATH") {
            config.contemplant.moongate_log_path = val;
        }
        if let Ok(val) = env::var("CONTEMPLANT_WATCHER_POLLING_INTERVAL_MS") {
            config.contemplant.watcher_polling_interval_ms = val.parse().context("CONTEMPLANT_WATCHER_POLLING_INTERVAL_MS must be a valid u64")?;
        }
        if let Ok(val) = env::var("CONTEMPLANT_SSH_AUTHORIZED_KEYS") {
            config.contemplant.ssh_authorized_keys = Some(val);
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
