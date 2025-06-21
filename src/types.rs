use crate::instance_controller::InstanceControllerClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::config::Config;

pub const VAST_BASE_URL: &str = "https://console.vast.ai/api/v0";
pub const VAST_OFFERS_ENDPOINT: &str = "/bundles";
pub const VAST_CREATE_INSTANCE_ENDPOINT: &str = "/asks";
pub const VAST_DELETE_INSTANCE_ENDPOINT: &str = "/instances";

#[derive(Clone)]
pub struct MagisterState {
    pub instance_controller_client: InstanceControllerClient,
}

impl MagisterState {
    pub async fn new(config: Config) -> Result<Self> {
        let instance_controller_client = InstanceControllerClient::new(config.clone()).await?;
        Ok(Self {
            instance_controller_client,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VastCreateInstanceResponse {
    pub success: bool,
    pub new_contract: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct VastInstance {
    pub offer: Offer,
    pub instance_id: u64,
    pub should_drop: bool,
}

impl VastInstance {
    pub fn new(instance_id: u64, offer: Offer) -> Self {
        let should_drop = false;
        Self {
            instance_id,
            offer,
            should_drop,
        }
    }
}

impl fmt::Display for VastInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "instance_id {}: {} in {} machine_id {} host_id {} ${:.2}/hour",
            self.instance_id,
            self.offer.gpu_name,
            self.offer.geolocation,
            self.offer.machine_id,
            self.offer.host_id,
            self.offer.dph_total
        )
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VastOfferResponse {
    pub offers: Vec<Offer>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Offer {
    pub id: u64,
    #[serde(skip_serializing)]
    pub ask_contract_id: u64,
    #[serde(skip_serializing)]
    pub bundle_id: u64,
    #[serde(skip_serializing)]
    pub bundled_results: Option<u64>,
    #[serde(skip_serializing)]
    pub bw_nvlink: f64,
    pub compute_cap: u32,
    pub cpu_arch: String,
    pub cpu_cores: Option<u32>,
    pub cpu_cores_effective: f64,
    pub cpu_ghz: Option<f64>,
    pub cpu_name: Option<String>,
    pub cpu_ram: u64,
    #[serde(skip_serializing)]
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
    #[serde(skip_serializing)]
    pub external: Option<serde_json::Value>,
    #[serde(skip_serializing)]
    pub flops_per_dphtotal: f64,
    pub geolocation: String,
    #[serde(skip_serializing)]
    pub geolocode: u64,
    pub gpu_arch: String,
    #[serde(skip_serializing)]
    pub gpu_display_active: bool,
    #[serde(skip_serializing)]
    pub gpu_frac: f64,
    #[serde(skip_serializing)]
    pub gpu_ids: Vec<u64>,
    #[serde(skip_serializing)]
    pub gpu_lanes: u32,
    #[serde(skip_serializing)]
    pub gpu_mem_bw: f64,
    pub gpu_name: String,
    pub gpu_ram: u64,
    pub gpu_total_ram: u64,
    pub gpu_max_power: f64,
    pub gpu_max_temp: f64,
    #[serde(skip_serializing)]
    pub has_avx: u32,
    pub host_id: u64,
    pub hosting_type: u32,
    pub hostname: Option<String>,
    pub inet_down: f64,
    pub inet_down_cost: f64,
    pub inet_up: f64,
    pub inet_up_cost: f64,
    #[serde(skip_serializing)]
    pub is_bid: bool,
    pub logo: String,
    pub machine_id: u64,
    #[serde(skip_serializing)]
    pub min_bid: f64,
    #[serde(skip_serializing)]
    pub mobo_name: Option<String>,
    pub num_gpus: u32,
    pub os_version: String,
    pub pci_gen: f64,
    pub pcie_bw: f64,
    pub public_ipaddr: String,
    pub reliability: f64,
    #[serde(skip_serializing)]
    pub reliability_mult: f64,
    #[serde(skip_serializing)]
    pub rentable: bool,
    #[serde(skip_serializing)]
    pub rented: bool,
    pub score: f64,
    pub start_date: Option<f64>,
    pub static_ip: bool,
    pub storage_cost: f64,
    pub storage_total_cost: f64,
    #[serde(skip_serializing)]
    pub total_flops: f64,
    #[serde(skip_serializing)]
    pub verification: String,
    #[serde(skip_serializing)]
    pub vericode: u32,
    #[serde(skip_serializing)]
    pub vram_costperhour: f64,
    pub webpage: Option<String>,
    #[serde(skip_serializing)]
    pub vms_enabled: bool,
    #[serde(skip_serializing)]
    pub expected_reliability: f64,
    #[serde(skip_serializing)]
    pub is_vm_deverified: bool,
    #[serde(skip_serializing)]
    pub resource_type: String,
    #[serde(skip_serializing)]
    pub cluster_id: Option<serde_json::Value>,
    #[serde(skip_serializing)]
    pub avail_vol_ask_id: Option<u64>,
    #[serde(skip_serializing)]
    pub avail_vol_dph: Option<f64>,
    #[serde(skip_serializing)]
    pub avail_vol_size: Option<f64>,
    #[serde(skip_serializing)]
    pub rn: u32,
    #[serde(skip_serializing)]
    pub dph_total_adj: f64,
    pub reliability2: f64,
    #[serde(skip_serializing)]
    pub discount_rate: Option<f64>,
    #[serde(skip_serializing)]
    pub discounted_hourly: f64,
    #[serde(skip_serializing)]
    pub discounted_dph_total: f64,
    #[serde(skip_serializing)]
    pub search: CostBreakdown,
    #[serde(skip_serializing)]
    pub instance: CostBreakdown,
    #[serde(skip_serializing)]
    pub time_remaining: String,
    #[serde(skip_serializing)]
    pub time_remaining_isbid: String,
    #[serde(skip_serializing)]
    pub internet_up_cost_per_tb: f64,
    #[serde(skip_serializing)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SummaryResponse {
    pub total_cost_per_hour: f64,
    pub num_instances: usize,
    pub instance_overview: Vec<InstanceOverview>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceOverview {
    instance_id: u64,
    gpu: String,
    location: String,
    machine_id: u64,
    host_id: u64,
    cost_per_hour: f64,
}

impl From<Offer> for InstanceOverview {
    fn from(offer: Offer) -> Self {
        InstanceOverview {
            instance_id: offer.id,
            gpu: offer.gpu_name,
            location: offer.geolocation,
            machine_id: offer.machine_id,
            host_id: offer.host_id,
            cost_per_hour: offer.dph_total,
        }
    }
}
