use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    pub bad_hosts: Option<Vec<u64>>,
    pub bad_machines: Option<Vec<u64>>,
    pub good_hosts: Option<Vec<u64>>,
    pub good_machines: Option<Vec<u64>>,
}

fn default_http_port() -> u16 {
    8555
}
