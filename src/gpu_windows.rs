//! Windows GPU-engine busy time, matched to a unique physical adapter by DXGI.
//! PDH is sampled once by the metrics worker; no per-process API polling is used.
use libloading::Library;
use std::{collections::HashMap, ffi::c_void, mem::{size_of, zeroed}, path::PathBuf, ptr::{null, null_mut}};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AdapterLuid { pub low: u32, pub high: u32 }

impl AdapterLuid {
    pub fn label(self) -> String { format!("luid_0x{:08x}_0x{:08x}", self.high, self.low) }
}

#[derive(Clone, Debug)]
struct Adapter { name: String, luid: AdapterLuid, software: bool }

pub fn same_adapter_name(left: &str, right: &str) -> bool {
    !left.trim().is_empty() && left.trim().eq_ignore_ascii_case(right.trim())
}

fn unique_adapter(adapters: &[Adapter], name: &str) -> Option<AdapterLuid> {
    let mut matches = adapters.iter().filter(|a| !a.software && same_adapter_name(&a.name, name));
    let matched = matches.next()?;
    matches.next().is_none().then_some(matched.luid)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct EngineKey { luid: AdapterLuid, physical: u32, engine: u32 }

fn hex_part(value: &str) -> Option<u32> {
    let digits = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X"))?;
    if digits.is_empty() || digits.len() > 8 { return None; }
    u32::from_str_radix(digits, 16).ok()
}

fn parse_instance(name: &str) -> Option<(u32, EngineKey)> {
    let (pid, rest) = name.strip_prefix("pid_")?.split_once("_luid_")?;
    let pid = pid.parse().ok()?;
    let (high, rest) = rest.split_once('_')?;
    let (low, rest) = rest.split_once("_phys_")?;
    let (physical, rest) = rest.split_once("_eng_")?;
    let (engine, kind) = rest.split_once("_engtype_")?;
    if kind.is_empty() { return None; }
    Some((pid, EngineKey { luid: AdapterLuid { low: hex_part(low)?, high: hex_part(high)? },
        physical: physical.parse().ok()?, engine: engine.parse().ok()? }))
}

fn busiest_engine<'a>(target: AdapterLuid, samples: impl IntoIterator<Item = (&'a str, f64, u32)>) -> Option<f32> {
    // Each GPU Engine instance already represents one process on one engine.
    // Deduplicate repeated PDH instance names before summing processes. Engines
    // (including physical nodes) run in parallel and must never be added together.
    let mut processes: HashMap<(EngineKey, u32), f64> = HashMap::new();
    for (name, value, status) in samples {
        if status > 1 || !value.is_finite() || value < 0.0 { continue; }
        let Some((pid, key)) = parse_instance(name) else { continue; };
        if key.luid != target { continue; }
        processes.entry((key, pid)).and_modify(|old| *old = old.max(value)).or_insert(value);
    }
    let mut engines: HashMap<EngineKey, f64> = HashMap::new();
    for ((engine, _pid), value) in processes { *engines.entry(engine).or_default() += value; }
    engines.values().copied().reduce(f64::max).map(|value| value.clamp(0.0, 100.0) as f32)
}

pub struct WindowsGpuUsage {
    api: PdhApi,
    query: isize,
    counter: isize,
    adapter_name: String,
    pub luid: AdapterLuid,
    ready: bool,
    misses: u8,
}

impl WindowsGpuUsage {
    pub fn open_for_name(name: &str) -> Option<Self> {
        let luid = unique_adapter(&enumerate_adapters()?, name)?;
        let api = PdhApi::open()?;
        let mut query = 0;
        if unsafe { (api.open)(null(), 0, &mut query) } != 0 { return None; }
        let path: Vec<u16> = "\\GPU Engine(*)\\Utilization Percentage".encode_utf16().chain(Some(0)).collect();
        let mut counter = 0;
        if unsafe { (api.add)(query, path.as_ptr(), 0, &mut counter) } != 0 {
            unsafe { (api.close)(query); }
            return None;
        }
        // Rate counters need two collections; no artificial zero during warmup.
        let ready = unsafe { (api.collect)(query) } == 0;
        Some(Self { api, query, counter, adapter_name: name.into(), luid, ready, misses: 0 })
    }

    pub fn sample(&mut self) -> Option<f32> {
        let result = self.read_sample();
        self.misses = if result.is_some() { 0 } else { self.misses.saturating_add(1) };
        result
    }

    pub fn needs_reopen(&self) -> bool { self.misses >= 8 }
    pub fn matches_name(&self, name: &str) -> bool { same_adapter_name(&self.adapter_name, name) }

    fn read_sample(&mut self) -> Option<f32> {
        if unsafe { (self.api.collect)(self.query) } != 0 { self.ready = false; return None; }
        if !self.ready { self.ready = true; return None; }
        // A process may appear between the sizing and data calls. Restart the
        // sizing query on MORE_DATA: its nonzero-input size is not trustworthy.
        for _ in 0..4 {
            let (mut bytes, mut count) = (0, 0);
            if unsafe { (self.api.array)(self.counter, PDH_FORMAT, &mut bytes, &mut count, null_mut()) } != PDH_MORE_DATA
                || bytes == 0 || bytes > 16 * 1024 * 1024 { return None; }
            // u64 storage gives the native x64 structures their required alignment.
            let mut storage = vec![0_u64; (bytes as usize).div_ceil(size_of::<u64>())];
            let capacity = storage.len() * size_of::<u64>();
            let status = unsafe { (self.api.array)(self.counter, PDH_FORMAT, &mut bytes, &mut count, storage.as_mut_ptr().cast()) };
            if status == PDH_MORE_DATA { continue; }
            if status != 0 || count as usize > capacity / size_of::<CounterItem>() { return None; }
            let mut samples = Vec::with_capacity(count as usize);
            for index in 0..count as usize {
                let item = unsafe { &*storage.as_ptr().cast::<CounterItem>().add(index) };
                if let Some(name) = name_in_buffer(item.name, &storage) {
                    samples.push((name, item.formatted.value, item.formatted.status));
                }
            }
            return busiest_engine(self.luid, samples.iter().map(|(name, value, status)| (name.as_str(), *value, *status)));
        }
        None
    }
}

impl Drop for WindowsGpuUsage { fn drop(&mut self) { unsafe { (self.api.close)(self.query); } } }

fn name_in_buffer(pointer: *const u16, storage: &[u64]) -> Option<String> {
    let start = storage.as_ptr() as usize;
    let end = start.checked_add(std::mem::size_of_val(storage))?;
    let address = pointer as usize;
    if address < start || address >= end || address % 2 != 0 { return None; }
    let remaining = (end - address) / size_of::<u16>();
    let units = unsafe { std::slice::from_raw_parts(pointer, remaining) };
    let length = units.iter().position(|&unit| unit == 0)?;
    String::from_utf16(&units[..length]).ok()
}

#[repr(C)]
struct CounterValue { status: u32, value: f64 }
#[repr(C)]
struct CounterItem { name: *const u16, formatted: CounterValue }
const PDH_FORMAT: u32 = 0x200 | 0x8000; // DOUBLE | NOCAP100; cap after per-engine aggregation.
const PDH_MORE_DATA: u32 = 0x8000_07d2;

type OpenQuery = unsafe extern "system" fn(*const u16, usize, *mut isize) -> u32;
type AddCounter = unsafe extern "system" fn(isize, *const u16, usize, *mut isize) -> u32;
type CollectQuery = unsafe extern "system" fn(isize) -> u32;
type CounterArray = unsafe extern "system" fn(isize, u32, *mut u32, *mut u32, *mut CounterItem) -> u32;
type CloseQuery = unsafe extern "system" fn(isize) -> u32;

fn system_dll(name: &str) -> Option<PathBuf> {
    Some(crate::windows_paths::system_directory()?.join(name))
}

struct PdhApi { _library: Library, open: OpenQuery, add: AddCounter, collect: CollectQuery, array: CounterArray, close: CloseQuery }
impl PdhApi {
    fn open() -> Option<Self> { unsafe {
        let library = Library::new(system_dll("pdh.dll")?).ok()?;
        let open = *library.get(b"PdhOpenQueryW\0").ok()?;
        let add = *library.get(b"PdhAddEnglishCounterW\0").ok()?;
        let collect = *library.get(b"PdhCollectQueryData\0").ok()?;
        let array = *library.get(b"PdhGetFormattedCounterArrayW\0").ok()?;
        let close = *library.get(b"PdhCloseQuery\0").ok()?;
        Some(Self { _library: library, open, add, collect, array, close })
    } }
}

#[repr(C)]
struct AdapterDescription {
    description: [u16; 128], vendor: u32, device: u32, subsystem: u32, revision: u32,
    dedicated_video: usize, dedicated_system: usize, shared_system: usize,
    low: u32, high: i32, flags: u32,
}

struct ComObject(*mut c_void);
impl ComObject {
    unsafe fn method(&self, index: usize) -> *const c_void { *(*(self.0 as *const *const *const c_void)).add(index) }
}
impl Drop for ComObject {
    fn drop(&mut self) { unsafe {
        let release: unsafe extern "system" fn(*mut c_void) -> u32 = std::mem::transmute(self.method(2));
        release(self.0);
    } }
}

fn enumerate_adapters() -> Option<Vec<Adapter>> { unsafe {
    // IDXGIFactory1::EnumAdapters1 and IDXGIAdapter1::GetDesc1. The method
    // positions follow the SDK inheritance layout (IUnknown + IDXGIObject).
    let iid = windows_sys::core::GUID { data1: 0x770aae78, data2: 0xf26f, data3: 0x4dba,
        data4: [0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87] };
    let library = Library::new(system_dll("dxgi.dll")?).ok()?;
    let create: unsafe extern "system" fn(*const windows_sys::core::GUID, *mut *mut c_void) -> i32 = *library.get(b"CreateDXGIFactory1\0").ok()?;
    let mut raw = null_mut();
    if create(&iid, &mut raw) < 0 || raw.is_null() { return None; }
    let factory = ComObject(raw);
    let enumerate: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32 = std::mem::transmute(factory.method(12));
    let mut adapters = Vec::new();
    for index in 0..64 {
        let mut raw = null_mut();
        let status = enumerate(factory.0, index, &mut raw);
        if status as u32 == 0x887a_0002 { return Some(adapters); } // DXGI_ERROR_NOT_FOUND.
        if status < 0 || raw.is_null() { return None; }
        let adapter = ComObject(raw);
        let describe: unsafe extern "system" fn(*mut c_void, *mut AdapterDescription) -> i32 = std::mem::transmute(adapter.method(10));
        let mut description: AdapterDescription = zeroed();
        if describe(adapter.0, &mut description) < 0 { return None; }
        let end = description.description.iter().position(|&unit| unit == 0).unwrap_or(128);
        adapters.push(Adapter { name: String::from_utf16_lossy(&description.description[..end]),
            luid: AdapterLuid { low: description.low, high: description.high as u32 }, software: description.flags & 2 != 0 });
    }
    None
} }

#[cfg(test)]
mod tests {
    use super::*;

    fn luid() -> AdapterLuid { AdapterLuid { high: 0, low: 0x111fa } }
    fn instance(pid: u32, physical: u32, engine: u32, kind: &str) -> String {
        format!("pid_{pid}_luid_0x00000000_0x000111FA_phys_{physical}_eng_{engine}_engtype_{kind}")
    }

    #[test]
    fn sums_processes_on_one_engine_then_takes_busiest_not_sum_of_engines() {
        let names = [instance(10, 0, 0, "3D"), instance(20, 0, 0, "3D"), instance(10, 0, 1, "Copy"), instance(10, 1, 0, "3D")];
        assert_eq!(busiest_engine(luid(), [(names[0].as_str(), 12.0, 0), (&names[1], 18.0, 1), (&names[2], 23.0, 0), (&names[3], 25.0, 0)]), Some(30.0));
    }

    #[test]
    fn rejects_other_adapters_invalid_status_and_missing_data_without_fake_zero() {
        let valid = instance(1, 0, 0, "3D");
        let other = valid.replace("000111FA", "00012592");
        assert_eq!(busiest_engine(luid(), [(&other[..], 99.0, 0), (&valid, 75.0, 0x8000_07d1), (&valid, f64::NAN, 0), (&valid, -1.0, 0)]), None);
        assert_eq!(busiest_engine(luid(), [(&valid[..], 0.0, 0)]), Some(0.0));
    }

    #[test]
    fn clamps_after_aggregation_and_deduplicates_process_instances() {
        let first = instance(1, 0, 0, "3D"); let second = instance(2, 0, 0, "3D");
        assert_eq!(busiest_engine(luid(), [(&first[..], 65.0, 0), (&first, 65.0, 1), (&second, 40.0, 0)]), Some(100.0));
    }

    #[test]
    fn malformed_instances_do_not_match_and_luid_word_order_is_explicit() {
        for name in ["_Total", "pid_1_luid_0x0_0x1_phys_x_eng_0_engtype_3D", "pid_1_luid_0x100000000_0x1_phys_0_eng_0_engtype_3D", "pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_"] {
            assert!(parse_instance(name).is_none(), "{name}");
        }
        assert_eq!(parse_instance("pid_2_luid_0x89ABCDEF_0x12345678_phys_1_eng_3_engtype_Video Decode").unwrap().1.luid,
            AdapterLuid { high: 0x89ab_cdef, low: 0x1234_5678 });
        assert_eq!(luid().label(), "luid_0x00000000_0x000111fa");
    }

    #[test]
    fn adapter_mapping_requires_a_unique_exact_hardware_name() {
        let candidate = Adapter { name: "NVIDIA GeForce RTX 3060".into(), luid: luid(), software: false };
        assert_eq!(unique_adapter(&[candidate.clone()], "nvidia geforce rtx 3060"), Some(luid()));
        assert_eq!(unique_adapter(&[candidate.clone()], "NVIDIA GeForce RTX 3080"), None);
        assert_eq!(unique_adapter(&[candidate.clone(), candidate.clone()], &candidate.name), None);
        let software = Adapter { software: true, ..candidate.clone() };
        assert_eq!(unique_adapter(&[software], &candidate.name), None);
    }

    #[test]
    fn native_structures_match_windows_x64_sdk_layout() {
        assert_eq!(size_of::<CounterValue>(), 16);
        assert_eq!(std::mem::offset_of!(CounterValue, value), 8);
        assert_eq!(size_of::<CounterItem>(), 24);
        assert_eq!(std::mem::offset_of!(CounterItem, formatted), 8);
        assert_eq!(size_of::<AdapterDescription>(), 312);
        assert_eq!(std::mem::offset_of!(AdapterDescription, low), 296);
    }
}
