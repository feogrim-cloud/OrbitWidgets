use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout { #[default] Horizontal, Vertical }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme { Midnight, #[default] Light, Neon, Mono }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkUnit { #[default] Megabits, Megabytes }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Anchor {
    #[default] Free,
    TopLeft, TopCenter, TopRight,
    MiddleLeft, MiddleRight,
    BottomLeft, BottomCenter, BottomRight,
}

impl Anchor {
    pub const ALL: [Self; 9] = [Self::Free, Self::TopLeft, Self::TopCenter, Self::TopRight,
        Self::MiddleLeft, Self::MiddleRight, Self::BottomLeft, Self::BottomCenter, Self::BottomRight];
    pub fn label(self) -> &'static str { match self {
        Self::Free => "Свободное размещение", Self::TopLeft => "Сверху слева",
        Self::TopCenter => "Сверху по центру", Self::TopRight => "Сверху справа",
        Self::MiddleLeft => "Слева по центру", Self::MiddleRight => "Справа по центру",
        Self::BottomLeft => "Снизу слева", Self::BottomCenter => "Снизу по центру",
        Self::BottomRight => "Снизу справа",
    }}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub cpu_usage: bool,
    pub cpu_temp: bool,
    pub gpu_usage: bool,
    pub gpu_temp: bool,
    pub ram: bool,
    pub disks: bool,
    pub hidden_disks: Vec<String>,
    pub network: bool,
    pub network_adapter: Option<String>,
    pub network_unit: NetworkUnit,
    pub network_scale_mbps: f64,
    pub layout: Layout,
    pub diameter: f32,
    pub gap: f32,
    pub opacity: f32,
    pub theme: Theme,
    pub anchor: Anchor,
    pub monitor_index: usize,
    pub margin: f32,
    pub always_on_top: bool,
    pub locked: bool,
    pub position: Option<[f32; 2]>,
}

impl Default for Config {
    fn default() -> Self { Self {
        cpu_usage: true, cpu_temp: true, gpu_usage: true, gpu_temp: true,
        ram: true, disks: true, hidden_disks: Vec::new(), network: true,
        network_adapter: None, network_unit: NetworkUnit::Megabits,
        network_scale_mbps: 100.0, layout: Layout::Vertical, diameter: 128.0,
        gap: 14.0, opacity: 0.94, theme: Theme::Light,
        anchor: Anchor::MiddleRight, monitor_index: 0, margin: 24.0,
        always_on_top: false, locked: false, position: None,
    }}
}

pub fn config_path() -> PathBuf {
    #[cfg(feature = "startup-qa")]
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() { return directory.join("profile").join("settings.json"); }
    }
    if let Some(directory) = std::env::var_os("ORBIT_CONFIG_DIR") {
        return PathBuf::from(directory).join("settings.json");
    }
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Orbit Widgets").join("settings.json")
}

impl Config {
    pub fn load() -> Self {
        let mut config: Self = fs::read(config_path()).ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
        config.diameter = finite_clamp(config.diameter, 80.0, 220.0, 128.0);
        config.gap = finite_clamp(config.gap, 0.0, 48.0, 14.0);
        config.opacity = finite_clamp(config.opacity, 0.2, 1.0, 0.94);
        config.margin = finite_clamp(config.margin, 0.0, 120.0, 24.0);
        if !config.network_scale_mbps.is_finite() { config.network_scale_mbps = 100.0; }
        config.network_scale_mbps = config.network_scale_mbps.clamp(1.0, 100_000.0);
        config
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
        let data = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, data)?;
        // Windows cannot rename a file over an existing destination.
        fs::copy(&temp_path, &path)?;
        fs::remove_file(temp_path)?;
        Ok(())
    }
}

fn finite_clamp(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() { value.clamp(min, max) } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_profile_choices_survive_new_defaults() {
        let config: Config = serde_json::from_str(r#"{
            "diameter":83,"theme":"Neon","layout":"Horizontal",
            "anchor":"Free","monitor_index":0,"opacity":0.73
        }"#).unwrap();
        assert_eq!(config.diameter, 83.0);
        assert_eq!(config.theme, Theme::Neon);
        assert_eq!(config.layout, Layout::Horizontal);
        assert_eq!(config.anchor, Anchor::Free);
        assert_eq!(config.monitor_index, 0);
        assert_eq!(config.opacity, 0.73);
    }
}
