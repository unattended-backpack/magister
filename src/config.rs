use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    pub vast_config: VastConfig,
    // Id of the template that magister will be making instances of.
    // Find the id at the Vast.ai web console
    pub template_id: String,
    // how many instances of the template this Magister will make sure are allocated
    pub number_instances: usize,
    pub bad_hosts: Option<Vec<u64>>,
    pub bad_machines: Option<Vec<u64>>,
    pub good_hosts: Option<Vec<u64>>,
    pub good_machines: Option<Vec<u64>>,
}

fn default_http_port() -> u16 {
    8555
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VastConfig {
    pub query: String,
}
