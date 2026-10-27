use crate::{disk_windows::WindowsDiskSpeeds, gpu_windows::{same_adapter_name, WindowsGpuUsage}, model::{DiskMetric, Metrics}};
pub use crate::model::MonitorCommand;
use libloading::Library;
use serde::Deserialize;
use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr},
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{mpsc::{self, Receiver, Sender}, Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use sysinfo::{MemoryRefreshKind, System};

const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

/// One collector serves every widget; no process enumeration or polling in UI.
pub fn start_monitor() -> (Receiver<Metrics>, Sender<MonitorCommand>) {
    let (snapshot_tx, snapshot_rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::channel();
    thread::Builder::new().name("orbit-metrics".into()).spawn(move || {
        let mut system = System::new();
        system.refresh_cpu_usage();
        let mut gpu = Nvml::open();
        let mut windows_gpu = gpu.as_ref().filter(|device| device.single_device)
            .and_then(|device| WindowsGpuUsage::open_for_name(&device.name));
        let mut last_windows_attempt = gpu.as_ref().map(|_| Instant::now());
        let mut shared_cpu = SharedCpuTemperature::default();
        let helper_state = Arc::new(Mutex::new(HelperState::default()));
        // Reuse an already-running sensor provider before starting another
        // process. NVML plus a fresh MAHM CPU value cover this desktop fully.
        let mut last_helper_attempt = None;
        let mut helper = if shared_cpu.temperature().is_some() && gpu.is_some() {
            None
        } else {
            last_helper_attempt = Some(Instant::now());
            SensorHelper::launch(helper_state.clone())
        };
        let mut network = NetworkSampler::default();
        let mut selected_network = None;
        let (disk_tx, disk_rx) = mpsc::channel();
        thread::Builder::new().name("orbit-disks".into()).spawn(move || {
            let mut disks = Vec::new();
            let mut capacity_updated = None;
            let mut speeds = WindowsDiskSpeeds::default();
            let mut next_sample = Instant::now();
            loop {
                // Capacity and volume discovery do not need one-second polling.
                // Keep potentially slow removable-volume calls off the main worker.
                if capacity_updated.map_or(true, |time: Instant| time.elapsed() >= Duration::from_secs(5)) {
                    disks = read_disks();
                    capacity_updated = Some(Instant::now());
                }
                thread::sleep(next_sample.saturating_duration_since(Instant::now()));
                speeds.sample(&mut disks);
                next_sample = Instant::now() + Duration::from_secs(1);
                if disk_tx.send((Instant::now(), disks.clone())).is_err() { break; }
            }
        }).ok();
        let mut disks = Vec::new();
        let mut disks_updated = None;
        // Seed cumulative network counters before the first timed sample.
        network.sample(None);
        loop {
            thread::sleep(SAMPLE_INTERVAL);
            while let Ok(command) = command_rx.try_recv() {
                match command {
                    MonitorCommand::SelectNetwork(name) => selected_network = name,
                    MonitorCommand::RefreshSensors => {
                        drop(helper.take());
                        if let Ok(mut state) = helper_state.lock() { *state = HelperState::default(); }
                        last_helper_attempt = Some(Instant::now());
                        helper = SensorHelper::launch(helper_state.clone());
                    }
                }
            }
            system.refresh_cpu_usage();
            system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
            // sysinfo's Windows backend obtains total and available physical
            // memory from one GlobalMemoryStatusEx call. "Available" includes
            // immediately reusable standby cache, not just empty RAM pages.
            let ram_total = system.total_memory();
            let ram_available = valid_ram_available(ram_total, system.available_memory());
            while let Ok((updated, value)) = disk_rx.try_recv() { disks = value; disks_updated = Some(updated); }
            // A stalled or failed disk worker must not freeze a live speed value.
            if disks_updated.is_some_and(|time: Instant| time.elapsed() >= Duration::from_secs(3)) {
                for disk in &mut disks { disk.read_bps = None; disk.write_bps = None; }
            }
            if let Some(child) = helper.as_mut() {
                child.check_exit(&helper_state);
            }
            let net = network.sample(selected_network.as_deref());
            let mut metric = Metrics {
                cpu_usage: finite_percent(system.global_cpu_usage()),
                ram_used: ram_available.map_or(0, |available| ram_total - available),
                ram_total,
                ram_available,
                disks: disks.clone(),
                network_rx_bps: net.rx,
                network_tx_bps: net.tx,
                network_name: net.name,
                network_interfaces: net.interfaces,
                ..Metrics::default()
            };
            if let Some(nvml) = gpu.as_mut() {
                metric.gpu_name = nvml.name.clone();
                metric.gpu_usage = nvml.utilization();
                metric.gpu_usage_nvml = metric.gpu_usage;
                if metric.gpu_usage.is_some() { metric.gpu_usage_source = "NVIDIA NVML · резервный источник".into(); }
                metric.gpu_temp = nvml.temperature();
                if let Some(memory) = nvml.memory() {
                    metric.gpu_memory_used = Some(memory.used);
                    metric.gpu_memory_total = Some(memory.total);
                    metric.gpu_memory_source = format!("NVIDIA NVML v{}", memory.api_version);
                }
            }
            if let Ok(state) = helper_state.lock() {
                let fresh = state.updated.map(|t| t.elapsed() < Duration::from_secs(4)).unwrap_or(false);
                metric.sensor_helper_running = helper.as_ref().map(|h| h.alive).unwrap_or(false);
                metric.temperature_status = state.status.clone();
                if fresh {
                    metric.cpu_temp = valid_temperature(state.data.cpu_temp);
                    let same_gpu = metric.gpu_name.is_empty() || same_adapter_name(&metric.gpu_name, &state.data.gpu_name);
                    if same_gpu && metric.gpu_usage.is_none() {
                        metric.gpu_usage = state.data.gpu_usage.and_then(finite_percent);
                        if metric.gpu_usage.is_some() { metric.gpu_usage_source = "LibreHardwareMonitor · резервный источник".into(); }
                    }
                    if same_gpu && metric.gpu_temp.is_none() { metric.gpu_temp = valid_temperature(state.data.gpu_temp); }
                    if metric.gpu_name.is_empty() { metric.gpu_name = state.data.gpu_name.clone(); }
                } else if state.updated.is_some() && metric.sensor_helper_running {
                    metric.temperature_status = "Датчики временно не отвечают".into();
                }
            }
            // Never take load from an unrelated active GPU. DXGI must provide
            // a unique exact hardware-name match for the temperature adapter.
            // Multi-NVIDIA configurations retain NVML until physical PCI identity
            // can be mapped unambiguously, rather than guessing by GPU number.
            let can_map_gpu = gpu.as_ref().map_or(true, |device| device.single_device);
            if windows_gpu.as_ref().is_some_and(|windows| !windows.matches_name(&metric.gpu_name)) {
                windows_gpu = None;
                last_windows_attempt = None;
            }
            if windows_gpu.is_none() && can_map_gpu && !metric.gpu_name.is_empty()
                && last_windows_attempt.map_or(true, |time: Instant| time.elapsed() >= Duration::from_secs(30)) {
                last_windows_attempt = Some(Instant::now());
                windows_gpu = WindowsGpuUsage::open_for_name(&metric.gpu_name);
            }
            if let Some(windows) = windows_gpu.as_mut() {
                metric.gpu_adapter_luid = windows.luid.label();
                if let Some(usage) = windows.sample() {
                    metric.gpu_usage = Some(usage);
                    metric.gpu_usage_source = "Windows WDDM · наиболее занятый движок".into();
                }
                if windows.needs_reopen() {
                    windows_gpu = None;
                    last_windows_attempt = Some(Instant::now());
                }
            }
            if metric.cpu_temp.is_none() {
                if let Some(temperature) = shared_cpu.temperature() {
                    metric.cpu_temp = Some(temperature);
                    metric.temperature_status = "Температура ЦП: MSI Afterburner, общая память".into();
                }
            }
            let helper_running = helper.as_ref().is_some_and(|h| h.alive);
            let missing_sensor = metric.cpu_temp.is_none() || metric.gpu_usage.is_none() || metric.gpu_temp.is_none();
            if missing_sensor && !helper_running
                && last_helper_attempt.map_or(true, |time: Instant| time.elapsed() >= Duration::from_secs(30)) {
                drop(helper.take());
                last_helper_attempt = Some(Instant::now());
                helper = SensorHelper::launch(helper_state.clone());
            }
            if snapshot_tx.send(metric).is_err() { break; }
        }
        // Drop kills our helper; it cannot linger after the main application exits.
        drop(helper);
    }).expect("Не удалось запустить сборщик показателей");
    (snapshot_rx, command_tx)
}

fn finite_percent(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 100.0))
}

fn valid_ram_available(total: u64, available: u64) -> Option<u64> {
    (total > 0 && available <= total).then_some(available)
}

fn valid_temperature(value: Option<f32>) -> Option<f32> {
    // Some low-level providers return zero when their driver is unavailable.
    value.filter(|t| t.is_finite() && *t > 1.0 && *t < 150.0)
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct SensorData {
    cpu_temp: Option<f32>,
    gpu_usage: Option<f32>,
    gpu_temp: Option<f32>,
    gpu_name: String,
    status: String,
}

struct HelperState {
    data: SensorData,
    updated: Option<Instant>,
    status: String,
}

impl Default for HelperState {
    fn default() -> Self {
        Self { data: SensorData::default(), updated: None, status: "Температура ЦП: модуль датчиков не запущен".into() }
    }
}

struct SensorHelper { child: Child, alive: bool }

impl SensorHelper {
    fn launch(state: Arc<Mutex<HelperState>>) -> Option<Self> {
        let app_dir = std::env::current_exe().ok()?.parent()?.to_owned();
        let candidates = [app_dir.join("sensors").join("OrbitSensors.exe"), app_dir.join("OrbitSensors.exe")];
        let path = match candidates.into_iter().find(|p| p.is_file()) {
            Some(path) => path,
            None => {
                if let Ok(mut s) = state.lock() { s.status = "Модуль температур не найден; остальные показатели работают".into(); }
                return None;
            }
        };
        let mut command = Command::new(&path);
        command.arg("--parent").arg(std::process::id().to_string())
            .current_dir(path.parent().unwrap()).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
        #[cfg(windows)] {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        match command.spawn() {
            Ok(mut child) => {
                if let Some(stdout) = child.stdout.take() {
                    let reader_state = state.clone();
                    thread::Builder::new().name("orbit-sensors".into()).spawn(move || {
                        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                            if line.len() > 16_384 { continue; }
                            if let Ok(data) = serde_json::from_str::<SensorData>(&line) {
                                if let Ok(mut s) = reader_state.lock() {
                                    s.status = data.status.clone();
                                    s.data = data;
                                    s.updated = Some(Instant::now());
                                }
                            }
                        }
                    }).ok();
                }
                if let Ok(mut s) = state.lock() { s.status = "Подключение датчиков…".into(); }
                Some(Self { child, alive: true })
            }
            Err(error) => {
                if let Ok(mut s) = state.lock() { s.status = format!("Не удалось запустить модуль датчиков: {error}"); }
                None
            }
        }
    }

    fn check_exit(&mut self, state: &Arc<Mutex<HelperState>>) {
        if self.alive && matches!(self.child.try_wait(), Ok(Some(_))) {
            self.alive = false;
            if let Ok(mut s) = state.lock() {
                s.updated = None;
                if s.data.status.is_empty() {
                    s.status = "Модуль датчиков завершился. Проверьте установку или права доступа".into();
                }
            }
        }
    }
}

impl Drop for SensorHelper {
    fn drop(&mut self) { let _ = self.child.kill(); let _ = self.child.wait(); }
}

// NVML ships with the NVIDIA driver. Loading the DLL directly avoids launching
// nvidia-smi each sample and leaves non-NVIDIA systems fully functional.
type NvmlHandle = *mut c_void;
type Init = unsafe extern "C" fn() -> u32;
type Shutdown = unsafe extern "C" fn() -> u32;
type GetHandle = unsafe extern "C" fn(u32, *mut NvmlHandle) -> u32;
type GetCount = unsafe extern "C" fn(*mut u32) -> u32;
type GetName = unsafe extern "C" fn(NvmlHandle, *mut c_char, u32) -> u32;
type GetUtil = unsafe extern "C" fn(NvmlHandle, *mut NvmlUtilization) -> u32;
type GetTemp = unsafe extern "C" fn(NvmlHandle, u32, *mut u32) -> u32;
type GetMemory = unsafe extern "C" fn(NvmlHandle, *mut NvmlMemory) -> u32;
type GetMemoryV2 = unsafe extern "C" fn(NvmlHandle, *mut NvmlMemoryV2) -> u32;
#[repr(C)] struct NvmlUtilization { gpu: u32, memory: u32 }
#[repr(C)]
#[derive(Default)]
struct NvmlMemory { total: u64, free: u64, used: u64 }
#[repr(C)]
#[derive(Default)]
struct NvmlMemoryV2 { version: u32, total: u64, reserved: u64, free: u64, used: u64 }
// NVIDIA's NVML_STRUCT_VERSION(Memory, 2) includes both the version and byte size.
const NVML_MEMORY_V2_VERSION: u32 = std::mem::size_of::<NvmlMemoryV2>() as u32 | (2 << 24);

#[derive(Clone, Copy)]
struct GpuMemory { used: u64, total: u64, api_version: u8 }

fn valid_gpu_memory(used: u64, total: u64, api_version: u8) -> Option<GpuMemory> {
    (total > 0 && total != u64::MAX && used <= total).then_some(GpuMemory { used, total, api_version })
}

fn query_nvml_memory(device: NvmlHandle, get_v2: Option<GetMemoryV2>, get_v1: Option<GetMemory>) -> Option<GpuMemory> {
    if let Some(get) = get_v2 {
        let mut value = NvmlMemoryV2 { version: NVML_MEMORY_V2_VERSION, ..NvmlMemoryV2::default() };
        if unsafe { get(device, &mut value) } == 0 {
            // Keep the driver's allocated value unchanged. Reserved-memory
            // accounting differs between API/driver versions; do not add or
            // subtract that field or mix this snapshot with Windows counters.
            if let Some(memory) = valid_gpu_memory(value.used, value.total, 2)
                .filter(|_| value.free <= value.total && value.reserved <= value.total) { return Some(memory); }
        }
    }
    let get = get_v1?;
    let mut value = NvmlMemory::default();
    if unsafe { get(device, &mut value) } != 0 || value.free > value.total { return None; }
    valid_gpu_memory(value.used, value.total, 1)
}

struct Nvml {
    _library: Library,
    shutdown: Shutdown,
    get_util: GetUtil,
    get_temp: GetTemp,
    get_memory: Option<GetMemory>,
    get_memory_v2: Option<GetMemoryV2>,
    device: NvmlHandle,
    name: String,
    single_device: bool,
    last_temperature_read: Option<Instant>,
    cached_temperature: Option<f32>,
    last_memory_read: Option<Instant>,
    cached_memory: Option<GpuMemory>,
}

impl Nvml {
    fn open() -> Option<Self> {
        let windows = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Windows"));
        let programs = std::env::var_os("ProgramW6432").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Program Files"));
        for path in [windows.join("System32").join("nvml.dll"), programs.join("NVIDIA Corporation").join("NVSMI").join("nvml.dll")] {
            if let Some(nvml) = unsafe { Self::load(path) } { return Some(nvml); }
        }
        None
    }

    unsafe fn load(path: PathBuf) -> Option<Self> {
        let library = Library::new(path).ok()?;
        let init: Init = *library.get(b"nvmlInit_v2\0").or_else(|_| library.get(b"nvmlInit\0")).ok()?;
        let shutdown: Shutdown = *library.get(b"nvmlShutdown\0").ok()?;
        let get_handle: GetHandle = *library.get(b"nvmlDeviceGetHandleByIndex_v2\0").or_else(|_| library.get(b"nvmlDeviceGetHandleByIndex\0")).ok()?;
        let get_name: GetName = *library.get(b"nvmlDeviceGetName\0").ok()?;
        let get_util: GetUtil = *library.get(b"nvmlDeviceGetUtilizationRates\0").ok()?;
        let get_temp: GetTemp = *library.get(b"nvmlDeviceGetTemperature\0").ok()?;
        // Memory support is optional and must not disable load or temperature.
        let get_memory = library.get::<GetMemory>(b"nvmlDeviceGetMemoryInfo\0").ok().map(|symbol| *symbol);
        let get_memory_v2 = library.get::<GetMemoryV2>(b"nvmlDeviceGetMemoryInfo_v2\0").ok().map(|symbol| *symbol);
        if init() != 0 { return None; }
        let mut device = std::ptr::null_mut();
        if get_handle(0, &mut device) != 0 { shutdown(); return None; }
        let count_function = library.get::<GetCount>(b"nvmlDeviceGetCount_v2\0")
            .or_else(|_| library.get::<GetCount>(b"nvmlDeviceGetCount\0")).ok();
        let single_device = count_function.is_some_and(|function| {
            let mut count = 0; function(&mut count) == 0 && count == 1
        });
        let mut name_buffer = [0 as c_char; 128];
        let name = if get_name(device, name_buffer.as_mut_ptr(), name_buffer.len() as u32) == 0 {
            CStr::from_ptr(name_buffer.as_ptr()).to_string_lossy().into_owned()
        } else { "NVIDIA GPU".into() };
        Some(Self { _library: library, shutdown, get_util, get_temp, get_memory, get_memory_v2, device, name, single_device,
            last_temperature_read: None, cached_temperature: None, last_memory_read: None, cached_memory: None })
    }

    fn utilization(&mut self) -> Option<f32> {
        let mut value = NvmlUtilization { gpu: 0, memory: 0 };
        (unsafe { (self.get_util)(self.device, &mut value) } == 0).then_some(value.gpu as f32).and_then(finite_percent)
    }
    fn temperature(&mut self) -> Option<f32> {
        if self.last_temperature_read.is_some_and(|t| t.elapsed() < Duration::from_secs(1)) {
            return self.cached_temperature;
        }
        let mut value = 0;
        self.cached_temperature = if unsafe { (self.get_temp)(self.device, 0, &mut value) } == 0 {
            valid_temperature(Some(value as f32))
        } else { None };
        self.last_temperature_read = Some(Instant::now());
        self.cached_temperature
    }

    fn memory(&mut self) -> Option<GpuMemory> {
        if self.last_memory_read.is_some_and(|time| time.elapsed() < Duration::from_secs(1)) {
            return self.cached_memory;
        }
        self.cached_memory = query_nvml_memory(self.device, self.get_memory_v2, self.get_memory);
        self.last_memory_read = Some(Instant::now());
        self.cached_memory
    }
}
impl Drop for Nvml { fn drop(&mut self) { unsafe { (self.shutdown)(); } } }

#[derive(Default)]
struct SharedCpuTemperature {
    last_read: Option<Instant>,
    cached: Option<f32>,
}

impl SharedCpuTemperature {
    fn temperature(&mut self) -> Option<f32> {
        if self.last_read.is_some_and(|t| t.elapsed() < Duration::from_secs(1)) { return self.cached; }
        self.cached = read_mahm_temperature();
        self.last_read = Some(Instant::now());
        self.cached
    }
}

#[derive(Clone, Copy)]
struct MahmHeader { header_size: usize, count: usize, entry_size: usize, total_size: usize }

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?))
}

fn mahm_header(bytes: &[u8], now: u64) -> Option<MahmHeader> {
    if bytes.len() < 32 || u32_at(bytes, 0)? != 0x4D41_484D || u32_at(bytes, 4)? >> 16 != 2 { return None; }
    let header_size = u32_at(bytes, 8)? as usize;
    let count = u32_at(bytes, 12)? as usize;
    let entry_size = u32_at(bytes, 16)? as usize;
    let timestamp = u32_at(bytes, 20)? as u64;
    // Shared memory is untrusted binary input. Never trust its size or a stale
    // leftover value after its producer has stopped updating.
    if !(32..=4096).contains(&header_size) || !(1..=1024).contains(&count)
        || !(1324..=4096).contains(&entry_size) || timestamp > now.saturating_add(5)
        || now.saturating_sub(timestamp) > 10 { return None; }
    let total_size = header_size.checked_add(count.checked_mul(entry_size)?)?;
    if total_size > 8 * 1024 * 1024 { return None; }
    Some(MahmHeader { header_size, count, entry_size, total_size })
}

fn parse_mahm_temperature(bytes: &[u8], now: u64) -> Option<f32> {
    let header = mahm_header(bytes, now)?;
    if bytes.len() < header.total_size { return None; }
    for index in 0..header.count {
        let base = header.header_size.checked_add(index.checked_mul(header.entry_size)?)?;
        // MAHM v2 source 0x80 is CPU temperature. A global CPU reading has no
        // GPU index; this excludes GPU temperatures and individual core rows.
        if u32_at(bytes, base + 1320)? == 0x80 && u32_at(bytes, base + 1316)? == u32::MAX {
            let value = f32::from_bits(u32_at(bytes, base + 1300)?);
            if value.is_finite() && value > 1.0 && value < 130.0 { return Some(value); }
        }
    }
    None
}

#[cfg(windows)]
fn read_mahm_temperature() -> Option<f32> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    for name in ["MAHMSharedMemory", "Local\\MAHMSharedMemory"] {
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let mapping = native::OpenFileMappingW(4, 0, wide.as_ptr()); // FILE_MAP_READ
            if mapping.is_null() { continue; }
            // Read only the fixed header first. A malformed producer cannot
            // trick us into reading beyond the mapped object.
            let header_view = native::MapViewOfFile(mapping, 4, 0, 0, 32);
            if header_view.is_null() { native::CloseHandle(mapping); continue; }
            let mut header_bytes = [0u8; 32];
            std::ptr::copy_nonoverlapping(header_view.cast::<u8>(), header_bytes.as_mut_ptr(), 32);
            native::UnmapViewOfFile(header_view);
            let Some(header) = mahm_header(&header_bytes, now) else { native::CloseHandle(mapping); continue; };
            let view = native::MapViewOfFile(mapping, 4, 0, 0, header.total_size);
            if view.is_null() { native::CloseHandle(mapping); continue; }
            let bytes = std::slice::from_raw_parts(view.cast::<u8>(), header.total_size);
            // Parse header again: the producer may have changed it between maps.
            let reading = parse_mahm_temperature(bytes, now);
            native::UnmapViewOfFile(view);
            native::CloseHandle(mapping);
            if reading.is_some() { return reading; }
        }
    }
    None
}
#[cfg(not(windows))] fn read_mahm_temperature() -> Option<f32> { None }

#[cfg(windows)]
fn read_disks() -> Vec<DiskMetric> {
    let mut result = Vec::new();
    let drives = unsafe { native::GetLogicalDrives() };
    for index in 0..26 {
        if drives & (1 << index) == 0 { continue; }
        let mount = format!("{}:", (b'A' + index as u8) as char);
        let root: Vec<u16> = format!("{mount}\\\0").encode_utf16().collect();
        let kind = unsafe { native::GetDriveTypeW(root.as_ptr()) };
        // Fixed, removable and RAM disks. Optical drives and unavailable network
        // shares must not stall the collector or appear as empty hard disks.
        if ![2, 3, 6].contains(&kind) { continue; }
        let (mut available, mut total, mut free) = (0u64, 0u64, 0u64);
        if unsafe { native::GetDiskFreeSpaceExW(root.as_ptr(), &mut available, &mut total, &mut free) } != 0 && total > 0 {
            result.push(DiskMetric { mount, used: total.saturating_sub(free), total, ..DiskMetric::default() });
        }
    }
    result
}
#[cfg(not(windows))] fn read_disks() -> Vec<DiskMetric> { Vec::new() }

#[derive(Default)]
struct NetworkSampler {
    previous: HashMap<u64, (u64, u64)>,
    updated: Option<Instant>,
}
#[derive(Default)] struct NetworkResult { rx: f64, tx: f64, name: String, interfaces: Vec<String> }

impl NetworkSampler {
    fn sample(&mut self, selected: Option<&str>) -> NetworkResult {
        let now = Instant::now();
        let elapsed = self.updated.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(0.0);
        let adapters = read_adapters();
        let has_physical = adapters.iter().any(|a| a.physical && a.up);
        let fallback = adapters.iter().find(|a| a.up).map(|a| a.id);
        let mut result = NetworkResult::default();
        let mut names = Vec::new();
        let mut counters = HashMap::new();
        for adapter in adapters {
            result.interfaces.push(adapter.name.clone());
            let included = match selected {
                Some(name) => adapter.name == name && adapter.up,
                None if has_physical => adapter.physical && adapter.up,
                None => Some(adapter.id) == fallback,
            };
            if included {
                names.push(adapter.name.clone());
                if let Some((old_rx, old_tx)) = self.previous.get(&adapter.id) {
                    if elapsed > 0.0 {
                        result.rx += adapter.rx.saturating_sub(*old_rx) as f64 / elapsed;
                        result.tx += adapter.tx.saturating_sub(*old_tx) as f64 / elapsed;
                    }
                }
            }
            counters.insert(adapter.id, (adapter.rx, adapter.tx));
        }
        result.interfaces.sort();
        result.interfaces.dedup();
        result.name = if names.is_empty() {
            selected.map(|s| format!("{s} — нет подключения")).unwrap_or_else(|| "Нет подключения".into())
        } else { names.join(" + ") };
        self.previous = counters;
        self.updated = Some(now);
        result
    }
}

struct Adapter { id: u64, name: String, physical: bool, up: bool, rx: u64, tx: u64 }

#[cfg(windows)]
fn read_adapters() -> Vec<Adapter> {
    let mut table: *mut native::MibIfTable2 = std::ptr::null_mut();
    if unsafe { native::GetIfTable2(&mut table) } != 0 || table.is_null() { return Vec::new(); }
    let mut adapters = Vec::new();
    unsafe {
        let count = (*table).num_entries as usize;
        if count <= 4096 {
            let first = std::ptr::addr_of!((*table).table).cast::<native::MibIfRow2>();
            for row in std::slice::from_raw_parts(first, count) {
                if row.interface_type == 24 { continue; } // Loopback
                let len = row.alias.iter().position(|c| *c == 0).unwrap_or(row.alias.len());
                let name = String::from_utf16_lossy(&row.alias[..len]);
                if name.is_empty() { continue; }
                adapters.push(Adapter {
                    id: row.interface_luid,
                    name,
                    physical: row.flags & 1 != 0 && row.flags & 2 == 0 && [6, 71].contains(&row.interface_type),
                    up: row.oper_status == 1,
                    rx: row.in_octets,
                    tx: row.out_octets,
                });
            }
        }
        native::FreeMibTable(table.cast());
    }
    adapters
}
#[cfg(not(windows))] fn read_adapters() -> Vec<Adapter> { Vec::new() }

#[cfg(windows)]
mod native {
    use std::ffi::c_void;
    #[repr(C)] pub struct Guid { _data1: u32, _data2: u16, _data3: u16, _data4: [u8; 8] }
    // Microsoft netioapi.h MIB_IF_ROW2. repr(C) preserves required ULONG64 alignment.
    #[repr(C)]
    pub struct MibIfRow2 {
        pub interface_luid: u64,
        _interface_index: u32,
        _interface_guid: Guid,
        pub alias: [u16; 257],
        _description: [u16; 257],
        _physical_address_length: u32,
        _physical_address: [u8; 32],
        _permanent_physical_address: [u8; 32],
        _mtu: u32,
        pub interface_type: u32,
        _tunnel_type: i32,
        _media_type: i32,
        _physical_medium_type: i32,
        _access_type: i32,
        _direction_type: i32,
        pub flags: u8,
        pub oper_status: i32,
        _admin_status: i32,
        _media_connect_state: i32,
        _network_guid: Guid,
        _connection_type: i32,
        _transmit_link_speed: u64,
        _receive_link_speed: u64,
        pub in_octets: u64,
        _in_ucast_pkts: u64,
        _in_nucast_pkts: u64,
        _in_discards: u64,
        _in_errors: u64,
        _in_unknown_protos: u64,
        _in_ucast_octets: u64,
        _in_multicast_octets: u64,
        _in_broadcast_octets: u64,
        pub out_octets: u64,
        _out_ucast_pkts: u64,
        _out_nucast_pkts: u64,
        _out_discards: u64,
        _out_errors: u64,
        _out_ucast_octets: u64,
        _out_multicast_octets: u64,
        _out_broadcast_octets: u64,
        _out_queue_length: u64,
    }
    #[repr(C)] pub struct MibIfTable2 { pub num_entries: u32, pub table: [MibIfRow2; 1] }
    #[cfg(target_pointer_width = "64")]
    const _: [(); 1352] = [(); std::mem::size_of::<MibIfRow2>()];
    #[link(name = "iphlpapi")]
    extern "system" {
        pub fn GetIfTable2(table: *mut *mut MibIfTable2) -> u32;
        pub fn FreeMibTable(memory: *mut c_void);
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetLogicalDrives() -> u32;
        pub fn GetDriveTypeW(root: *const u16) -> u32;
        pub fn GetDiskFreeSpaceExW(path: *const u16, available: *mut u64, total: *mut u64, free: *mut u64) -> i32;
        pub fn OpenFileMappingW(access: u32, inherit: i32, name: *const u16) -> *mut c_void;
        pub fn MapViewOfFile(mapping: *mut c_void, access: u32, offset_high: u32, offset_low: u32, size: usize) -> *mut c_void;
        pub fn UnmapViewOfFile(address: *const c_void) -> i32;
        pub fn CloseHandle(handle: *mut c_void) -> i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_ram_rejects_missing_or_invalid_total_but_keeps_real_zero() {
        assert_eq!(valid_ram_available(0, 0), None);
        assert_eq!(valid_ram_available(100, 101), None);
        assert_eq!(valid_ram_available(100, 0), Some(0));
        assert_eq!(valid_ram_available(100, 100), Some(100));
    }

    #[test]
    fn nvml_memory_layout_and_version_match_nvidia_x64_header() {
        assert_eq!(std::mem::size_of::<NvmlMemory>(), 24);
        assert_eq!(std::mem::offset_of!(NvmlMemory, used), 16);
        assert_eq!(std::mem::size_of::<NvmlMemoryV2>(), 40);
        assert_eq!(std::mem::offset_of!(NvmlMemoryV2, total), 8);
        assert_eq!(std::mem::offset_of!(NvmlMemoryV2, reserved), 16);
        assert_eq!(std::mem::offset_of!(NvmlMemoryV2, free), 24);
        assert_eq!(std::mem::offset_of!(NvmlMemoryV2, used), 32);
        assert_eq!(NVML_MEMORY_V2_VERSION, 0x0200_0028);
    }

    #[test]
    fn gpu_memory_rejects_invalid_capacity_and_keeps_real_zero_usage() {
        assert!(valid_gpu_memory(0, 0, 2).is_none());
        assert!(valid_gpu_memory(101, 100, 2).is_none());
        assert!(valid_gpu_memory(u64::MAX, u64::MAX, 2).is_none());
        assert_eq!(valid_gpu_memory(0, 12 * 1_073_741_824, 2).unwrap().used, 0);
        assert!(query_nvml_memory(std::ptr::null_mut(), None, None).is_none());
    }

    #[test]
    fn nvml_memory_prefers_v2_and_falls_back_to_v1_without_inventing_values() {
        unsafe extern "C" fn v2(_device: NvmlHandle, output: *mut NvmlMemoryV2) -> u32 {
            if (*output).version != 0x0200_0028 { return 25; }
            *output = NvmlMemoryV2 { version: (*output).version, total: 12_000, reserved: 500, free: 9_000, used: 2_500 };
            0
        }
        unsafe extern "C" fn unsupported(_device: NvmlHandle, _output: *mut NvmlMemoryV2) -> u32 { 3 }
        unsafe extern "C" fn v1(_device: NvmlHandle, output: *mut NvmlMemory) -> u32 {
            *output = NvmlMemory { total: 12_000, free: 9_000, used: 3_000 };
            0
        }
        let memory = query_nvml_memory(std::ptr::null_mut(), Some(v2), Some(v1)).unwrap();
        assert_eq!((memory.used, memory.total, memory.api_version), (2_500, 12_000, 2));
        let fallback = query_nvml_memory(std::ptr::null_mut(), Some(unsupported), Some(v1)).unwrap();
        assert_eq!((fallback.used, fallback.total, fallback.api_version), (3_000, 12_000, 1));
        assert!(query_nvml_memory(std::ptr::null_mut(), Some(unsupported), None).is_none());
    }

    #[test]
    fn missing_pawnio_zero_from_real_helper_payload_is_not_a_temperature() {
        // LHM 0.9.6 returned this sentinel on the test PC without PawnIO.
        let payload = r#"{"cpu_temp":0,"gpu_temp":49,"gpu_usage":87,"gpu_name":"NVIDIA GeForce RTX 3060","status":""}"#;
        let sample: SensorData = serde_json::from_str(payload).unwrap();
        assert_eq!(valid_temperature(sample.cpu_temp), None);
        assert_eq!(valid_temperature(sample.gpu_temp), Some(49.0));
        let unavailable: SensorData = serde_json::from_str(r#"{"cpu_temp":null,"gpu_temp":null}"#).unwrap();
        assert_eq!(valid_temperature(unavailable.cpu_temp), None);
        assert_eq!(valid_temperature(unavailable.gpu_temp), None);
    }

    #[cfg(windows)]
    #[test]
    fn network_ffi_layout_matches_windows_sdk_x64_abi() {
        // These offsets come from netioapi.h; wrong layout corrupts byte counters.
        assert_eq!(std::mem::size_of::<native::MibIfRow2>(), 1352);
        assert_eq!(std::mem::offset_of!(native::MibIfRow2, in_octets), 1208);
        assert_eq!(std::mem::offset_of!(native::MibIfRow2, out_octets), 1280);
        assert_eq!(std::mem::offset_of!(native::MibIfTable2, table), 8);
    }

    fn shared_memory_fixture(timestamp: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; 32 + 1324 * 2];
        for (offset, value) in [(0, 0x4D41_484Du32), (4, 0x20000), (8, 32), (12, 2), (16, 1324), (20, timestamp)] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        // GPU temperature goes first so the selector must distinguish sources.
        bytes[32 + 1300..32 + 1304].copy_from_slice(&49.0f32.to_le_bytes());
        let cpu = 32 + 1324;
        bytes[cpu + 1300..cpu + 1304].copy_from_slice(&65.125f32.to_le_bytes());
        bytes[cpu + 1316..cpu + 1320].copy_from_slice(&u32::MAX.to_le_bytes());
        bytes[cpu + 1320..cpu + 1324].copy_from_slice(&0x80u32.to_le_bytes());
        bytes
    }

    #[test]
    fn shared_memory_selects_global_cpu_and_rejects_stale_truncated_or_malformed_input() {
        let now = 1_700_000_000u32;
        let mut bytes = shared_memory_fixture(now);
        assert_eq!(parse_mahm_temperature(&bytes, now as u64), Some(65.125));
        assert_eq!(parse_mahm_temperature(&bytes, now as u64 + 11), None);
        assert_eq!(parse_mahm_temperature(&bytes[..100], now as u64), None);
        bytes[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(parse_mahm_temperature(&bytes, now as u64), None);
        bytes = shared_memory_fixture(now);
        let cpu_data = 32 + 1324 + 1300;
        bytes[cpu_data..cpu_data + 4].copy_from_slice(&f32::NAN.to_le_bytes());
        assert_eq!(parse_mahm_temperature(&bytes, now as u64), None);
    }
}
