//! Values use bytes and bytes/second internally. Unknown sensors stay `None`.

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct DiskMetric {
    pub mount: String,
    pub used: u64,
    pub total: u64,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Metrics {
    pub cpu_usage: Option<f32>,
    pub cpu_temp: Option<f32>,
    pub gpu_usage: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub gpu_name: String,
    pub ram_used: u64,
    pub ram_total: u64,
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
