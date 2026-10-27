//! Values use bytes and bytes/second internally. Unknown sensors stay `None`.

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct DiskMetric {
    pub mount: String,
    pub used: u64,
    pub total: u64,
    /// Current logical-volume throughput. Missing/warmup counters stay unknown.
    pub read_bps: Option<f64>,
    pub write_bps: Option<f64>,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Metrics {
    pub cpu_usage: Option<f32>,
    pub cpu_temp: Option<f32>,
    pub gpu_usage: Option<f32>,
    pub gpu_usage_source: String,
    /// Unmodified NVIDIA sample for diagnostics; the displayed load prefers WDDM.
    pub gpu_usage_nvml: Option<f32>,
    pub gpu_adapter_luid: String,
    pub gpu_temp: Option<f32>,
    /// Dedicated framebuffer bytes reported by the same NVIDIA device as temperature.
    pub gpu_memory_used: Option<u64>,
    pub gpu_memory_total: Option<u64>,
    pub gpu_memory_source: String,
    pub gpu_name: String,
    pub ram_used: u64,
    pub ram_total: u64,
    /// Immediately reusable physical bytes, including standby cache on Windows.
    pub ram_available: Option<u64>,
    pub disks: Vec<DiskMetric>,
    pub network_rx_bps: f64,
    pub network_tx_bps: f64,
    pub network_name: String,
    pub network_interfaces: Vec<String>,
    pub temperature_status: String,
    pub sensor_helper_running: bool,
}

#[derive(Clone, Debug)]
pub enum MonitorCommand {
    /// `None` sums physical connected interfaces, avoiding VPN duplication.
    SelectNetwork(Option<String>),
    RefreshSensors,
}
