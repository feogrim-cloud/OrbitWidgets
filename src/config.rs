use serde::{Deserialize, Serialize};
use std::{fs, io::{self, Write}, path::{Path, PathBuf}};

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
    pub gpu_memory: bool,
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
        cpu_usage: true, cpu_temp: true, gpu_usage: true, gpu_temp: true, gpu_memory: true,
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
    pub fn has_saved_profile() -> bool {
        let path = config_path();
        read_valid_config(&path).or_else(|| read_valid_config(&backup_path(&path))).is_some()
    }

    pub fn load() -> Self {
        Self::load_from(&config_path())
    }

    fn load_from(path: &Path) -> Self {
        let mut config = read_valid_config(path)
            .or_else(|| read_valid_config(&backup_path(path)))
            .unwrap_or_default();
        config.diameter = finite_clamp(config.diameter, 80.0, 220.0, 128.0);
        config.gap = finite_clamp(config.gap, 0.0, 48.0, 14.0);
        config.opacity = finite_clamp(config.opacity, 0.2, 1.0, 0.94);
        config.margin = finite_clamp(config.margin, 0.0, 120.0, 24.0);
        if !config.network_scale_mbps.is_finite() { config.network_scale_mbps = 100.0; }
        config.network_scale_mbps = config.network_scale_mbps.clamp(1.0, 100_000.0);
        config
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to(&config_path())
    }

    fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
        let data = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let backup = backup_path(path);
        let previous = match fs::read(path) {
            Ok(bytes) => serde_json::from_slice::<Self>(&bytes).ok().map(|_| bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        // Never replace the last good backup with an unreadable primary file.
        // Preparing the backup first also leaves a usable profile if replacement fails.
        if let Some(previous) = previous {
            atomic_write(&backup, &previous)?;
        } else if read_valid_config(&backup).is_none() {
            atomic_write(&backup, &data)?;
        }
        atomic_write(path, &data)
    }
}

fn read_valid_config(path: &Path) -> Option<Config> {
    fs::read(path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

fn backup_path(path: &Path) -> PathBuf { append_suffix(path, ".bak") }

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    name.into()
}

fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let temp_path = append_suffix(path, ".tmp");
    let result = (|| {
        let mut temp = fs::File::create(&temp_path)?;
        temp.write_all(data)?;
        temp.sync_all()?;
        drop(temp);
        atomic_replace(&temp_path, path)
    })();
    if result.is_err() { let _ = fs::remove_file(&temp_path); }
    result
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
    // Both files are in the same directory, so replacement cannot fall back to
    // a cross-volume copy that would truncate the previous profile in place.
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) } == 0 {
        Err(io::Error::last_os_error())
    } else { Ok(()) }
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)?;
    if let Some(parent) = destination.parent() { fs::File::open(parent)?.sync_all()?; }
    Ok(())
}

fn finite_clamp(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() { value.clamp(min, max) } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!("orbit-config-test-{}-{}-{}", std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn profile(&self) -> PathBuf { self.0.join("settings.json") }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    fn neon_config() -> Config {
        Config { theme: Theme::Neon, diameter: 83.0, gap: 6.0, opacity: 0.74,
            monitor_index: 1, margin: 10.0, ..Config::default() }
    }

    fn assert_neon(config: Config) {
        assert_eq!(config.theme, Theme::Neon);
        assert_eq!(config.diameter, 83.0);
        assert_eq!(config.gap, 6.0);
        assert_eq!(config.opacity, 0.74);
        assert_eq!(config.monitor_index, 1);
        assert_eq!(config.margin, 10.0);
    }

    #[test]
    fn settings_survive_replacement_and_backup_keeps_previous_profile() {
        let directory = TestDirectory::new();
        let path = directory.profile();
        let original = neon_config();
        original.save_to(&path).unwrap();
        let previous_bytes = fs::read(&path).unwrap();
        let mut updated = original.clone();
        updated.diameter = 96.0;
        updated.save_to(&path).unwrap();
        assert_eq!(Config::load_from(&path).diameter, 96.0);
        assert_eq!(fs::read(backup_path(&path)).unwrap(), previous_bytes);
        assert!(!append_suffix(&path, ".tmp").exists());
    }

    #[test]
    fn corrupt_or_missing_primary_recovers_valid_backup() {
        let directory = TestDirectory::new();
        let path = directory.profile();
        neon_config().save_to(&path).unwrap();
        fs::write(&path, br#"{"theme":"Neon","diameter":"#).unwrap();
        assert_neon(Config::load_from(&path));
        fs::remove_file(&path).unwrap();
        assert_neon(Config::load_from(&path));
    }

    #[test]
    fn interrupted_temp_write_does_not_change_active_profile() {
        let directory = TestDirectory::new();
        let path = directory.profile();
        neon_config().save_to(&path).unwrap();
        let original = fs::read(&path).unwrap();
        fs::write(append_suffix(&path, ".tmp"), br#"{"theme":"#).unwrap();
        assert_neon(Config::load_from(&path));
        assert_eq!(fs::read(&path).unwrap(), original);
    }

    #[test]
    fn saving_recovered_profile_preserves_valid_backup() {
        let directory = TestDirectory::new();
        let path = directory.profile();
        neon_config().save_to(&path).unwrap();
        let backup_bytes = fs::read(backup_path(&path)).unwrap();
        fs::write(&path, b"damaged profile").unwrap();
        let recovered = Config::load_from(&path);
        recovered.save_to(&path).unwrap();
        assert_neon(Config::load_from(&path));
        assert_eq!(fs::read(backup_path(&path)).unwrap(), backup_bytes);
    }

    #[test]
    fn failed_replacement_leaves_previous_profile_readable() {
        let directory = TestDirectory::new();
        let path = directory.profile();
        neon_config().save_to(&path).unwrap();
        let original = fs::read(&path).unwrap();
        // A directory at the staging path reproduces a denied/interrupted write
        // without touching permissions or any real user profile.
        fs::create_dir(append_suffix(&path, ".tmp")).unwrap();
        let mut changed = neon_config();
        changed.theme = Theme::Light;
        assert!(changed.save_to(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_neon(Config::load_from(&path));
    }

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
