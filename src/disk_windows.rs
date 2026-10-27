//! Current throughput of each logical volume, using Windows performance counters.
//! Keep one query in the disk worker. Rate counters need two collections; an
//! unavailable counter is never presented as a real zero or a previous value.
use crate::model::DiskMetric;
use libloading::Library;
use std::{collections::{HashMap, HashSet}, ptr::null, time::{Duration, Instant}};

const REOPEN_DELAY: Duration = Duration::from_secs(30);
const PDH_FORMAT: u32 = 0x200 | 0x1000 | 0x8000; // DOUBLE | NOSCALE | NOCAP100: raw bytes/second.

/// Only drive-letter instances correspond to this application's disk widgets.
/// Do not guess a mapping for physical disks, _Total, volume GUIDs or mounts.
fn drive_instance(mount: &str) -> Option<String> {
    let mount = mount.strip_suffix('\\').unwrap_or(mount);
    let bytes = mount.as_bytes();
    (bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        .then(|| format!("{}:", bytes[0].to_ascii_uppercase() as char))
}

fn valid_rate(value: f64, status: u32) -> Option<f64> {
    (status <= 1 && value.is_finite() && value >= 0.0).then_some(value)
}

fn clear_speeds(disks: &mut [DiskMetric]) {
    for disk in disks { disk.read_bps = None; disk.write_bps = None; }
}

#[derive(Default)]
pub struct WindowsDiskSpeeds {
    query: Option<DiskQuery>,
    last_attempt: Option<Instant>,
}

impl WindowsDiskSpeeds {
    pub fn sample(&mut self, disks: &mut [DiskMetric]) {
        clear_speeds(disks);
        let now = Instant::now();
        if self.query.is_none() && self.last_attempt.map_or(true, |time| now.duration_since(time) >= REOPEN_DELAY) {
            self.last_attempt = Some(now);
            self.query = DiskQuery::open();
        }
        if let Some(query) = self.query.as_mut() {
            query.sample(disks);
            if query.misses >= 8 {
                self.query = None;
                self.last_attempt = Some(now);
            }
        }
    }
}

struct CounterPair {
    read: isize,
    write: isize,
    // A newly added or failed instance must collect a fresh baseline first.
    ready: bool,
}

impl CounterPair {
    fn reading(&mut self, read: Option<f64>, write: Option<f64>) -> (Option<f64>, Option<f64>) {
        let was_ready = self.ready;
        // Even if PDH formats the first sample as zero, keep it unknown.
        // After a missing/invalid volume a fresh baseline is required again.
        self.ready = read.is_some() || write.is_some() || !was_ready;
        if was_ready { (read, write) } else { (None, None) }
    }
}

struct DiskQuery {
    api: PdhApi,
    handle: isize,
    counters: HashMap<String, CounterPair>,
    misses: u8,
}

impl DiskQuery {
    fn open() -> Option<Self> {
        let api = PdhApi::open()?;
        let mut handle = 0;
        if unsafe { (api.open)(null(), 0, &mut handle) } != 0 { return None; }
        Some(Self { api, handle, counters: HashMap::new(), misses: 0 })
    }

    fn add_pair(&self, mount: &str) -> Option<CounterPair> {
        let mut read = 0;
        let mut write = 0;
        let path = |counter| format!("\\LogicalDisk({mount})\\{counter}").encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        if unsafe { (self.api.add)(self.handle, path("Disk Read Bytes/sec").as_ptr(), 0, &mut read) } != 0 { return None; }
        if unsafe { (self.api.add)(self.handle, path("Disk Write Bytes/sec").as_ptr(), 0, &mut write) } != 0 {
            unsafe { (self.api.remove)(read); }
            return None;
        }
        Some(CounterPair { read, write, ready: false })
    }

    fn sample(&mut self, disks: &mut [DiskMetric]) {
        let wanted: HashSet<String> = disks.iter().filter_map(|disk| drive_instance(&disk.mount)).collect();
        // Removing an unplugged volume also discards its baseline. Reusing its
        // letter for another drive cannot reuse our previous formatted reading.
        self.counters.retain(|mount, pair| {
            if wanted.contains(mount) { return true; }
            unsafe { (self.api.remove)(pair.read); (self.api.remove)(pair.write); }
            false
        });
        for mount in &wanted {
            if !self.counters.contains_key(mount) {
                if let Some(pair) = self.add_pair(mount) { self.counters.insert(mount.clone(), pair); }
            }
        }
        if wanted.is_empty() { self.misses = 0; return; }
        if unsafe { (self.api.collect)(self.handle) } != 0 {
            for pair in self.counters.values_mut() { pair.ready = false; }
            self.misses = self.misses.saturating_add(1);
            return;
        }
        let mut any_valid = false;
        for disk in disks {
            let Some(mount) = drive_instance(&disk.mount) else { continue; };
            let Some(pair) = self.counters.get_mut(&mount) else { continue; };
            let read = self.api.rate(pair.read);
            let write = self.api.rate(pair.write);
            (disk.read_bps, disk.write_bps) = pair.reading(read, write);
            any_valid |= disk.read_bps.is_some() || disk.write_bps.is_some();
        }
        self.misses = if any_valid { 0 } else { self.misses.saturating_add(1) };
    }
}

impl Drop for DiskQuery {
    fn drop(&mut self) { unsafe { (self.api.close)(self.handle); } }
}

#[repr(C)]
#[derive(Default)]
struct CounterValue { status: u32, value: f64 }

type OpenQuery = unsafe extern "system" fn(*const u16, usize, *mut isize) -> u32;
type AddCounter = unsafe extern "system" fn(isize, *const u16, usize, *mut isize) -> u32;
type RemoveCounter = unsafe extern "system" fn(isize) -> u32;
type CollectQuery = unsafe extern "system" fn(isize) -> u32;
type FormattedCounter = unsafe extern "system" fn(isize, u32, *mut u32, *mut CounterValue) -> u32;
type CloseQuery = unsafe extern "system" fn(isize) -> u32;

struct PdhApi {
    _library: Library,
    open: OpenQuery,
    add: AddCounter,
    remove: RemoveCounter,
    collect: CollectQuery,
    formatted: FormattedCounter,
    close: CloseQuery,
}

impl PdhApi {
    fn open() -> Option<Self> { unsafe {
        let path = crate::windows_paths::system_directory()?.join("pdh.dll");
        let library = Library::new(path).ok()?;
        let open = *library.get(b"PdhOpenQueryW\0").ok()?;
        let add = *library.get(b"PdhAddEnglishCounterW\0").ok()?;
        let remove = *library.get(b"PdhRemoveCounter\0").ok()?;
        let collect = *library.get(b"PdhCollectQueryData\0").ok()?;
        let formatted = *library.get(b"PdhGetFormattedCounterValue\0").ok()?;
        let close = *library.get(b"PdhCloseQuery\0").ok()?;
        Some(Self { _library: library, open, add, remove, collect, formatted, close })
    } }

    fn rate(&self, counter: isize) -> Option<f64> {
        let mut value = CounterValue::default();
        if unsafe { (self.formatted)(counter, PDH_FORMAT, std::ptr::null_mut(), &mut value) } != 0 { return None; }
        valid_rate(value.value, value.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_instances_map_only_to_exact_drive_letters() {
        assert_eq!(drive_instance("C:"), Some("C:".into()));
        assert_eq!(drive_instance("d:\\"), Some("D:".into()));
        for mount in ["", "_Total", "0 C:", "C:\\folder", "C:\\\\", "C", "CC:", "1:", "\\\\server\\share", "HarddiskVolume1"] {
            assert_eq!(drive_instance(mount), None, "{mount}");
        }
    }

    #[test]
    fn rates_keep_real_idle_and_throughput_without_percent_cap_or_scaling() {
        assert_eq!(valid_rate(0.0, 0), Some(0.0));
        assert_eq!(valid_rate(8_000_000_000.0, 1), Some(8_000_000_000.0));
        for (value, status) in [(f64::NAN, 0), (f64::INFINITY, 0), (-1.0, 0), (0.0, 0x8000_07d1)] {
            assert_eq!(valid_rate(value, status), None);
        }
        assert_eq!(PDH_FORMAT & 0x1000, 0x1000); // NOSCALE, raw B/s rather than provider's default display scale.
    }

    #[test]
    fn missing_sample_removes_previous_read_and_write_values() {
        let mut disks = [DiskMetric { mount: "C:".into(), read_bps: Some(50.0), write_bps: Some(80.0), ..DiskMetric::default() }];
        clear_speeds(&mut disks);
        assert_eq!((disks[0].read_bps, disks[0].write_bps), (None, None));
    }

    #[test]
    fn warmup_and_error_recovery_require_a_baseline_but_keep_real_idle() {
        let mut pair = CounterPair { read: 0, write: 0, ready: false };
        assert_eq!(pair.reading(Some(0.0), Some(0.0)), (None, None));
        assert_eq!(pair.reading(Some(400.0), Some(20.0)), (Some(400.0), Some(20.0)));
        assert_eq!(pair.reading(None, None), (None, None));
        assert_eq!(pair.reading(Some(500.0), Some(30.0)), (None, None));
        assert_eq!(pair.reading(Some(0.0), Some(0.0)), (Some(0.0), Some(0.0)));
        assert_eq!(pair.reading(None, Some(4.0)), (None, Some(4.0)));
    }

    #[test]
    fn native_counter_layout_matches_windows_x64_sdk() {
        assert_eq!(std::mem::size_of::<CounterValue>(), 16);
        assert_eq!(std::mem::offset_of!(CounterValue, value), 8);
    }
}
