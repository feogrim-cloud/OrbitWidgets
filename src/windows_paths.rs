//! Resolve operating-system locations through Windows, not inherited environment
//! variables. These locations are also used when the process is elevated.
use std::{ffi::{c_void, OsString}, os::windows::ffi::OsStringExt, path::PathBuf, ptr::null_mut, sync::OnceLock};

#[repr(C)]
struct Guid { data1: u32, data2: u16, data3: u16, data4: [u8; 8] }

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
    fn GetWindowsDirectoryW(buffer: *mut u16, size: u32) -> u32;
}
#[link(name = "shell32")]
unsafe extern "system" {
    fn SHGetKnownFolderPath(folder: *const Guid, flags: u32, token: *mut c_void, path: *mut *mut u16) -> i32;
}
#[link(name = "ole32")]
unsafe extern "system" { fn CoTaskMemFree(value: *mut c_void); }

fn directory(get: unsafe extern "system" fn(*mut u16, u32) -> u32) -> Option<PathBuf> {
    let mut buffer = vec![0_u16; 32_768];
    let length = unsafe { get(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if length == 0 || length >= buffer.len() { return None; }
    let path = PathBuf::from(OsString::from_wide(&buffer[..length]));
    path.is_absolute().then_some(path)
}

pub fn system_directory() -> Option<PathBuf> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| directory(GetSystemDirectoryW)).clone()
}

pub fn windows_directory() -> Option<PathBuf> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| directory(GetWindowsDirectoryW)).clone()
}

pub fn program_files_directory() -> Option<PathBuf> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| unsafe {
        // FOLDERID_ProgramFiles resolves the native x64 directory for this x64 app.
        let id = Guid { data1: 0x905e63b6, data2: 0xc1bf, data3: 0x494e,
            data4: [0xb2, 0x9c, 0x65, 0xb7, 0x32, 0xd3, 0xd2, 0x1a] };
        let mut value = null_mut();
        let status = SHGetKnownFolderPath(&id, 0, null_mut(), &mut value);
        if status < 0 || value.is_null() {
            if !value.is_null() { CoTaskMemFree(value.cast()); }
            return None;
        }
        let mut length = 0;
        while length < 32_768 && *value.add(length) != 0 { length += 1; }
        let path = if length < 32_768 { Some(PathBuf::from(OsString::from_wide(std::slice::from_raw_parts(value, length)))) } else { None };
        CoTaskMemFree(value.cast());
        path.filter(|path| path.is_absolute())
    }).clone()
}

/// Preserve the selected Windows UI font while rejecting oversized input. The
/// application does not accept downloaded or user-selected font files.
pub fn ui_font() -> Option<Vec<u8>> {
    use std::io::Read;
    const MAX_BYTES: u64 = 16 * 1024 * 1024;
    let file = std::fs::File::open(windows_directory()?.join("Fonts").join("segoeui.ttf")).ok()?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes).ok()?;
    (!bytes.is_empty() && bytes.len() as u64 <= MAX_BYTES).then_some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_paths_are_absolute_and_point_at_system_components() {
        assert!(system_directory().unwrap().join("kernel32.dll").is_file());
        assert!(windows_directory().unwrap().is_dir());
        assert!(program_files_directory().unwrap().is_dir());
    }
}
