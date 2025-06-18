use crate::instance_controller::InstanceControllerClient;
use serde::{Deserialize, Serialize};

use crate::{config::Config, vast::VastClient};

pub const VAST_BASE_URL: &str = "https://cloud.vast.ai/api/v0";
pub const VAST_OFFERS_ENDPOINT: &str = "/bundles";

#[derive(Clone)]
pub struct MagisterState {
    pub config: Config,
    pub instance_controller_client: InstanceControllerClient,
}

impl MagisterState {
    pub async fn new(config: Config) -> Self {
        let instance_controller_client = InstanceControllerClient::new(config.clone()).await;
        Self {
            config,
            instance_controller_client,
        }
    }
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VastOfferResponse {
    pub offers: Vec<Offer>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VastInstance {
    mins_alive: u64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Offer {
    pub id: u64,
    pub ask_contract_id: u64,
    pub bundle_id: u64,
    pub bundled_results: Option<u64>,
    pub bw_nvlink: f64,
    pub compute_cap: u32,
    pub cpu_arch: String,
    pub cpu_cores: u32,
    pub cpu_cores_effective: f64,
    pub cpu_ghz: f64,
    pub cpu_name: String,
    pub cpu_ram: u64,
    pub credit_discount_max: f64,
    pub cuda_max_good: f64,
    pub direct_port_count: u32,
    pub disk_bw: f64,
    pub disk_name: String,
    pub disk_space: f64,
    pub dlperf: f64,
    pub dlperf_per_dphtotal: f64,
    pub dph_base: f64,
    pub dph_total: f64,
    pub driver_version: String,
    pub driver_vers: u64,
    pub duration: f64,
    pub end_date: f64,
    pub external: Option<serde_json::Value>,
    pub flops_per_dphtotal: f64,
    pub geolocation: String,
    pub geolocode: u64,
    pub gpu_arch: String,
    pub gpu_display_active: bool,
    pub gpu_frac: f64,
    pub gpu_ids: Vec<u64>,
    pub gpu_lanes: u32,
    pub gpu_mem_bw: f64,
    pub gpu_name: String,
    pub gpu_ram: u64,
    pub gpu_total_ram: u64,
    pub gpu_max_power: f64,
    pub gpu_max_temp: f64,
    pub has_avx: u32,
    pub host_id: u64,
    pub hosting_type: u32,
    pub hostname: Option<String>,
    pub inet_down: f64,
    pub inet_down_cost: f64,
    pub inet_up: f64,
    pub inet_up_cost: f64,
    pub is_bid: bool,
    pub logo: String,
    pub machine_id: u64,
    pub min_bid: f64,
    pub mobo_name: Option<String>,
    pub num_gpus: u32,
    pub os_version: String,
    pub pci_gen: f64,
    pub pcie_bw: f64,
    pub public_ipaddr: String,
    pub reliability: f64,
    pub reliability_mult: f64,
    pub rentable: bool,
    pub rented: bool,
    pub score: f64,
    pub start_date: Option<f64>,
    pub static_ip: bool,
    pub storage_cost: f64,
    pub storage_total_cost: f64,
    pub total_flops: f64,
    pub verification: String,
    pub vericode: u32,
    pub vram_costperhour: f64,
    pub webpage: Option<String>,
    pub vms_enabled: bool,
    pub expected_reliability: f64,
    pub is_vm_deverified: bool,
    pub resource_type: String,
    pub cluster_id: Option<serde_json::Value>,
    pub avail_vol_ask_id: Option<u64>,
    pub avail_vol_dph: Option<f64>,
    pub avail_vol_size: Option<f64>,
    pub rn: u32,
    pub dph_total_adj: f64,
    pub reliability2: f64,
    pub discount_rate: Option<f64>,
    pub discounted_hourly: f64,
    pub discounted_dph_total: f64,
    pub search: CostBreakdown,
    pub instance: CostBreakdown,
    pub time_remaining: String,
    pub time_remaining_isbid: String,
    pub internet_up_cost_per_tb: f64,
    pub internet_down_cost_per_tb: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CostBreakdown {
    #[serde(rename = "gpuCostPerHour")]
    pub gpu_cost_per_hour: f64,
    #[serde(rename = "diskHour")]
    pub disk_hour: f64,
    #[serde(rename = "totalHour")]
    pub total_hour: f64,
    #[serde(rename = "discountTotalHour")]
    pub discount_total_hour: f64,
    #[serde(rename = "discountedTotalPerHour")]
    pub discounted_total_per_hour: f64,
}
