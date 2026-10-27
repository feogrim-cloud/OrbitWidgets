//! Conservative policy for code that will run with administrator rights.
//! Reading normal/portable startup does not invoke this policy or change tasks.
use std::{ffi::c_void, fs, mem::size_of, os::windows::{ffi::OsStrExt, fs::MetadataExt}, path::Path, ptr::{null_mut, read_unaligned}};

const INSTALL_REQUIRED: &str = "Для повышенного автозапуска установите Orbit Widgets в защищённую папку Program Files. Обычный автозапуск доступен и переносной версии.";
const MUTATING_ACCESS: u32 = 0x0000_0002 | 0x0000_0004 | 0x0000_0010 | 0x0000_0040 | 0x0000_0100
    | 0x0001_0000 | 0x0004_0000 | 0x0008_0000 | 0x4000_0000 | 0x1000_0000;
// A parent can replace an existing child via DELETE_CHILD or change its own
// security. Creating an unrelated sibling alone does not replace Program Files.
const PARENT_MUTATING_ACCESS: u32 = 0x0000_0040 | 0x0001_0000 | 0x0004_0000 | 0x0008_0000 | 0x1000_0000;
const MAX_TREE_OBJECTS: usize = 4096;
const MAX_TREE_DEPTH: usize = 16;

pub(super) fn validate_installation(executable: &Path) -> Result<(), String> {
    let program_files = crate::windows_paths::program_files_directory().ok_or_else(|| INSTALL_REQUIRED.to_owned())?;
    // Check the original chain too: canonicalization must not hide a junction.
    reject_reparse_chain(executable)?;
    reject_reparse_chain(&program_files)?;
    let root = fs::canonicalize(&program_files).map_err(|_| INSTALL_REQUIRED.to_owned())?;
    let executable = fs::canonicalize(executable).map_err(|_| INSTALL_REQUIRED.to_owned())?;
    let directory = executable.parent().ok_or_else(|| INSTALL_REQUIRED.to_owned())?;
    if !executable.is_file() || same_path(directory, &root) || !directory.ancestors().any(|parent| same_path(parent, &root)) {
        return Err(INSTALL_REQUIRED.into());
    }
    // Every ancestor that could replace the protected installation is covered.
    for ancestor in root.ancestors() {
        let program_root = same_path(ancestor, &root);
        // Inherit-only creation templates on the volume root do not affect the
        // existing protected Program Files child; its actual ACL is checked.
        validate_security(ancestor, program_root, if program_root { MUTATING_ACCESS } else { PARENT_MUTATING_ACCESS })?;
    }
    let mut ancestor = directory.parent();
    while let Some(parent) = ancestor {
        if same_path(parent, &root) { break; }
        validate_security(parent, true, MUTATING_ACCESS)?;
        ancestor = parent.parent();
    }
    // Covers the main executable, helper, all adjacent dependency DLLs, and
    // any subdirectory from which a future dependency could be resolved.
    let mut pending = vec![(directory.to_owned(), 0usize)];
    let mut checked = 0;
    while let Some((path, depth)) = pending.pop() {
        checked += 1;
        if checked > MAX_TREE_OBJECTS || depth > MAX_TREE_DEPTH { return Err(INSTALL_REQUIRED.into()); }
        let metadata = fs::symlink_metadata(&path).map_err(|_| INSTALL_REQUIRED.to_owned())?;
        if metadata.file_attributes() & 0x400 != 0 { return Err(INSTALL_REQUIRED.into()); }
        validate_security(&path, metadata.is_dir(), MUTATING_ACCESS)?;
        if metadata.is_dir() {
            for entry in fs::read_dir(&path).map_err(|_| INSTALL_REQUIRED.to_owned())? {
                pending.push((entry.map_err(|_| INSTALL_REQUIRED.to_owned())?.path(), depth + 1));
                if pending.len() + checked > MAX_TREE_OBJECTS { return Err(INSTALL_REQUIRED.into()); }
            }
        } else if !metadata.is_file() { return Err(INSTALL_REQUIRED.into()); }
    }
    Ok(())
}

fn reject_reparse_chain(path: &Path) -> Result<(), String> {
    if !path.is_absolute() { return Err(INSTALL_REQUIRED.into()); }
    for ancestor in path.ancestors() {
        let attributes = fs::symlink_metadata(ancestor).map_err(|_| INSTALL_REQUIRED.to_owned())?.file_attributes();
        if attributes & 0x400 != 0 { return Err(INSTALL_REQUIRED.into()); }
    }
    Ok(())
}

fn same_path(left: &Path, right: &Path) -> bool {
    let normal = |path: &Path| path.to_string_lossy().trim_start_matches("\\\\?\\").trim_end_matches(['\\', '/']).replace('/', "\\").to_lowercase();
    normal(left) == normal(right)
}

fn trusted_owner(sid: &str) -> bool {
    matches!(sid, "S-1-5-18" | "S-1-5-32-544"
        | "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464")
}

fn allowed_ace_is_safe(sid: &str, mask: u32, flags: u8, directory: bool, mutating: u32) -> bool {
    if mask & mutating == 0 { return true; }
    if flags & 0x08 != 0 {
        // INHERIT_ONLY does not apply to this object. On directories retain
        // only the usual Creator Owner template: new files are created by the
        // trusted installer/admin, not by an ordinary writable child folder.
        if !directory || sid == "S-1-3-0" { return true; }
    }
    trusted_owner(sid)
}

#[repr(C)]
struct Acl { revision: u8, reserved: u8, size: u16, count: u16, reserved2: u16 }
#[repr(C)]
#[derive(Clone, Copy)]
struct AceHeader { kind: u8, flags: u8, size: u16 }

fn validate_security(path: &Path, directory: bool, mutating: u32) -> Result<(), String> {
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let (mut owner, mut dacl, mut descriptor) = (null_mut(), null_mut(), null_mut());
    let status = unsafe { native::GetNamedSecurityInfoW(path.as_ptr(), 1, 1 | 4, &mut owner, null_mut(), &mut dacl, null_mut(), &mut descriptor) };
    if status != 0 || descriptor.is_null() {
        if !descriptor.is_null() { unsafe { native::LocalFree(descriptor); } }
        return Err(INSTALL_REQUIRED.into());
    }
    let result = unsafe { validate_acl(owner, dacl, directory, mutating) };
    unsafe { native::LocalFree(descriptor); }
    result
}

unsafe fn validate_acl(owner: *const c_void, dacl: *const c_void, directory: bool, mutating: u32) -> Result<(), String> {
    let rejected = || INSTALL_REQUIRED.to_owned();
    if !sid_string(owner).is_some_and(|sid| trusted_owner(&sid)) || dacl.is_null() { return Err(rejected()); }
    let acl = &*dacl.cast::<Acl>();
    if usize::from(acl.size) < size_of::<Acl>() { return Err(rejected()); }
    for index in 0..u32::from(acl.count) {
        let mut ace = null_mut();
        if native::GetAce(dacl, index, &mut ace) == 0 || ace.is_null() { return Err(rejected()); }
        let header = read_unaligned(ace.cast::<AceHeader>());
        let start = dacl as usize;
        let address = ace as usize;
        if address < start || address.checked_add(usize::from(header.size)).is_none_or(|end| end > start + usize::from(acl.size)) { return Err(rejected()); }
        match header.kind {
            0 => { // ACCESS_ALLOWED_ACE: Header, ACCESS_MASK, SID.
                if usize::from(header.size) < 8 + 8 { return Err(rejected()); }
                let mask = read_unaligned(ace.cast::<u8>().add(4).cast::<u32>());
                let sid = ace.cast::<u8>().add(8).cast::<c_void>();
                if native::IsValidSid(sid) == 0 || native::GetLengthSid(sid) as usize > usize::from(header.size) - 8 { return Err(rejected()); }
                let sid = sid_string(sid).ok_or_else(rejected)?;
                if !allowed_ace_is_safe(&sid, mask, header.flags, directory, mutating) { return Err(rejected()); }
            }
            1 => {} // Deny ACEs cannot grant a modification that is not allowed above.
            // Object/callback grants need a richer access check. Refuse these
            // unusual ACLs rather than guessing that they cannot grant writes.
            _ => return Err(rejected()),
        }
    }
    Ok(())
}

unsafe fn sid_string(sid: *const c_void) -> Option<String> {
    if sid.is_null() || native::IsValidSid(sid) == 0 { return None; }
    let mut value = null_mut();
    if native::ConvertSidToStringSidW(sid, &mut value) == 0 || value.is_null() { return None; }
    let mut count = 0;
    while count < 256 && *value.add(count) != 0 { count += 1; }
    let result = (count < 256).then(|| String::from_utf16_lossy(std::slice::from_raw_parts(value, count)));
    native::LocalFree(value.cast());
    result
}

mod native {
    use std::ffi::c_void;
    #[link(name = "advapi32")]
    extern "system" {
        pub fn GetNamedSecurityInfoW(path: *const u16, kind: i32, info: u32, owner: *mut *mut c_void, group: *mut *mut c_void, dacl: *mut *mut c_void, sacl: *mut *mut c_void, descriptor: *mut *mut c_void) -> u32;
        pub fn GetAce(acl: *const c_void, index: u32, ace: *mut *mut c_void) -> i32;
        pub fn IsValidSid(sid: *const c_void) -> i32;
        pub fn GetLengthSid(sid: *const c_void) -> u32;
        pub fn ConvertSidToStringSidW(sid: *const c_void, value: *mut *mut u16) -> i32;
        #[cfg(test)]
        pub fn ConvertStringSecurityDescriptorToSecurityDescriptorW(value: *const u16, revision: u32, descriptor: *mut *mut c_void, length: *mut u32) -> i32;
        #[cfg(test)]
        pub fn GetSecurityDescriptorOwner(descriptor: *const c_void, owner: *mut *mut c_void, defaulted: *mut i32) -> i32;
        #[cfg(test)]
        pub fn GetSecurityDescriptorDacl(descriptor: *const c_void, present: *mut i32, dacl: *mut *mut c_void, defaulted: *mut i32) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" { pub fn LocalFree(memory: *mut c_void) -> *mut c_void; }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    fn descriptor_is_safe(sddl: &str, directory: bool) -> bool {
        let input: Vec<u16> = OsStr::new(sddl).encode_wide().chain(Some(0)).collect();
        let mut descriptor = null_mut();
        unsafe {
            assert_ne!(native::ConvertStringSecurityDescriptorToSecurityDescriptorW(input.as_ptr(), 1, &mut descriptor, null_mut()), 0);
            let (mut owner, mut dacl) = (null_mut(), null_mut());
            let (mut present, mut defaulted) = (0, 0);
            assert_ne!(native::GetSecurityDescriptorOwner(descriptor, &mut owner, &mut defaulted), 0);
            assert_ne!(native::GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted), 0);
            let result = present != 0 && validate_acl(owner, dacl, directory, MUTATING_ACCESS).is_ok();
            native::LocalFree(descriptor);
            result
        }
    }
    #[test]
    fn protected_acl_allows_only_trusted_writers_and_owner() {
        assert!(descriptor_is_safe("O:BAD:(A;;FA;;;SY)(A;;FA;;;BA)(A;;GRGX;;;BU)", true));
        assert!(!descriptor_is_safe("O:BAD:(A;;FA;;;SY)(A;;FA;;;BA)(A;;GW;;;BU)", true));
        assert!(!descriptor_is_safe("O:BAD:(A;;FA;;;SY)(A;;WD;;;AU)", false));
        assert!(!descriptor_is_safe("O:BUD:(A;;FA;;;BA)(A;;GRGX;;;BU)", false));
        assert!(!descriptor_is_safe("O:BAD:NO_ACCESS_CONTROL", false));
        assert!(!descriptor_is_safe("O:BAD:(A;OICIIO;FA;;;BU)(A;;FA;;;BA)", true));
        assert!(descriptor_is_safe("O:BAD:(A;OICIIO;FA;;;CO)(A;;FA;;;BA)(A;;GRGX;;;BU)", true));
    }
    #[test]
    fn writable_portable_location_is_rejected_for_elevated_startup() {
        assert!(validate_installation(&std::env::current_exe().unwrap()).is_err());
    }
    #[test]
    fn volume_parent_sibling_creation_does_not_grant_replacement_access() {
        assert!(allowed_ace_is_safe("S-1-5-11", 0x0000_0004, 0, false, PARENT_MUTATING_ACCESS));
        assert!(allowed_ace_is_safe("S-1-5-11", 0xE001_0000, 0x0B, false, PARENT_MUTATING_ACCESS));
        assert!(!allowed_ace_is_safe("S-1-5-11", 0x0000_0040, 0, false, PARENT_MUTATING_ACCESS));
        assert!(!allowed_ace_is_safe("S-1-5-11", 0x0004_0000, 0, false, PARENT_MUTATING_ACCESS));
    }
    #[test]
    #[ignore = "Requires a real protected Orbit Widgets installation; root runs this in the native Windows session"]
    fn native_installed_tree_is_protected() {
        let root = crate::windows_paths::program_files_directory().unwrap();
        let path = std::env::var_os("ORBIT_TEST_PROTECTED_INSTALL").map(std::path::PathBuf::from)
            .unwrap_or_else(|| root.join("OrbitWidgets").join("OrbitWidgets.exe"));
        validate_installation(&path).unwrap();
    }
}
