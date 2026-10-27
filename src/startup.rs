//! Per-user logon registration. No task is changed while reading settings.
use std::{
    collections::HashMap,
    ffi::{c_void, OsStr, OsString},
    fs,
    mem::{size_of, zeroed},
    os::windows::{ffi::OsStrExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Output},
    ptr::null_mut,
};
#[cfg(feature = "startup-qa")]
use std::io::Write;
use xml::reader::{EventReader, XmlEvent};
use windows::{core::BSTR, Win32::{Foundation::RPC_E_CHANGED_MODE, System::{Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED}, TaskScheduler::{ITaskService, TaskScheduler, TASK_CREATE, TASK_UPDATE, TASK_IGNORE_REGISTRATION_TRIGGERS, TASK_LOGON_INTERACTIVE_TOKEN}, Variant::VARIANT}}};

#[path = "startup_security.rs"]
mod security;

const SOURCE: &str = "OrbitWidgets.Startup.v1";
const TASK_NS: &str = "http://schemas.microsoft.com/windows/2004/02/mit/task";
const TEST_PREFIX: &str = "OrbitWidgets.Startup.Test.";
const APPLY_FLAG: &str = "--startup-apply";
const CONFIGURE_FLAG: &str = "--startup-configure";

/// CLI exit codes: 0 verified success, 2 invalid arguments, 3 UAC cancelled,
/// 4 operation failed, 5 registration did not match the requested result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct StartupStatus { pub enabled: bool, pub elevated: bool }

struct Failure { code: i32, message: String }
type Result<T> = std::result::Result<T, Failure>;
impl Failure {
    fn operation(message: impl Into<String>) -> Self { Self { code: 4, message: message.into() } }
    fn verify() -> Self { Self { code: 5, message: "Не удалось подтвердить сохранение автозапуска в Планировщике заданий".into() } }
}

#[derive(Clone)]
struct Context { executable: PathBuf, sid: String, task_name: String }

impl Context {
    fn current() -> Result<Self> {
        let sid = current_sid()?;
        #[cfg(feature = "startup-qa")]
        let task_name = "OrbitWidgets.Startup.Test.GUIQA-v110".to_owned();
        #[cfg(not(feature = "startup-qa"))]
        let task_name = match std::env::var("ORBIT_STARTUP_TASK_NAME") {
            Ok(value) if valid_test_task(&value) => value,
            Ok(_) => return Err(Failure::operation("Тестовое имя автозапуска должно начинаться с OrbitWidgets.Startup.Test.")),
            Err(std::env::VarError::NotPresent) => format!("OrbitWidgets.Logon.{sid}"),
            Err(_) => return Err(Failure::operation("Некорректное тестовое имя автозапуска")),
        };
        Ok(Self { executable: std::env::current_exe().map_err(|e| Failure::operation(e.to_string()))?, sid, task_name })
    }
}

pub fn query() -> std::result::Result<StartupStatus, String> {
    Context::current().and_then(|context| query_task(&context)).map(|state| state.unwrap_or_default()).map_err(|e| e.message)
}

pub fn configure(enabled: bool, elevated: bool) -> std::result::Result<StartupStatus, String> {
    configure_inner(enabled, elevated).map_err(|e| e.message)
}

fn configure_inner(enabled: bool, elevated: bool) -> Result<StartupStatus> {
    let context = Context::current()?;
    if enabled && elevated { validate_elevated_location(&context)?; }
    let previous = query_task(&context)?;
    if !enabled && previous.is_none() { return Ok(StartupStatus::default()); }
    // Raising privileges changes only the task, never the running widget window.
    if !is_elevated()? && ((enabled && elevated) || previous.is_some_and(|s| s.elevated)) {
        apply_elevated(&context, enabled, elevated)?;
    } else if let Err(error) = apply_local(&context, enabled, elevated) {
        // Local least-privilege registration normally needs no UAC. Scheduler
        // policy or a previously elevated registration can still require it.
        if is_elevated()? { return Err(error); }
        apply_elevated(&context, enabled, elevated)?;
    }
    verify_requested(&context, enabled, elevated)
}

/// Call before the single-instance guard. Installer examples:
/// `--startup-configure enable normal`, `enable elevated`, or `disable`.
pub fn handle_cli(args: &[String]) -> Option<i32> {
    if let Some(index) = args.iter().position(|arg| arg == "--startup-query") {
        if args.len() != index + 2 { return Some(2); }
        return Some(match query() {
            Ok(status) => match serde_json::to_vec_pretty(&status).ok().and_then(|bytes| fs::write(&args[index + 1], bytes).ok()) { Some(()) => 0, None => 4 },
            Err(_) => 4,
        });
    }
    if let Some(index) = args.iter().position(|arg| arg == CONFIGURE_FLAG) {
        let parsed = match args.get(index + 1).map(String::as_str) {
            Some("disable") if args.len() == index + 2 => Some((false, false)),
            Some("enable") if args.len() == index + 3 => match args[index + 2].as_str() {
                "normal" => Some((true, false)), "elevated" => Some((true, true)), _ => None,
            },
            _ => None,
        };
        return Some(match parsed { Some((enabled, elevated)) => exit_code(configure_inner(enabled, elevated)), None => 2 });
    }
    let index = args.iter().position(|arg| arg == APPLY_FLAG)?;
    if args.len() != index + 5 { return Some(2); }
    let enabled = match parse_bit(&args[index + 1]) { Some(v) => v, None => return Some(2) };
    let elevated = match parse_bit(&args[index + 2]) { Some(v) => v, None => return Some(2) };
    let sid = &args[index + 3];
    let task_name = &args[index + 4];
    if !valid_sid(sid) || !(task_name == &format!("OrbitWidgets.Logon.{sid}") || valid_test_task(task_name)) { return Some(2); }
    let executable = match std::env::current_exe() { Ok(path) => path, Err(_) => return Some(4) };
    Some(exit_code(apply_local(&Context { executable, sid: sid.clone(), task_name: task_name.clone() }, enabled, elevated)))
}

fn exit_code(result: Result<StartupStatus>) -> i32 {
    result.map_or_else(|e| { eprintln!("{}", e.message); e.code }, |_| 0)
}
fn parse_bit(value: &str) -> Option<bool> { match value { "0" => Some(false), "1" => Some(true), _ => None } }
fn valid_sid(value: &str) -> bool {
    value.starts_with("S-1-") && value.len() <= 184 && value.split('-').skip(1).all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}
fn valid_test_task(value: &str) -> bool {
    value.starts_with(TEST_PREFIX) && value.len() > TEST_PREFIX.len() && value.len() <= 150
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn query_task(context: &Context) -> Result<Option<StartupStatus>> {
    let output = schtasks(&[OsString::from("/Query"), OsString::from("/TN"), context.task_name.clone().into(), OsString::from("/XML")])?;
    if output.status.success() {
        let xml = decode_output(&output.stdout);
        return owned_status(&xml, context).map(Some);
    }
    // Do not call every Scheduler error a missing task. A successful full
    // listing distinguishes absence from permission/service failures without
    // relying on the Windows display language of an error message.
    let listing = schtasks(&["/Query".into(), "/FO".into(), "CSV".into(), "/NH".into()])?;
    if listing.status.success() {
        // CSV layouts vary (/V and Windows versions can prepend HostName).
        // Match an entire field, never assume that TaskName is column zero.
        let present = decode_output(&listing.stdout).lines().filter_map(csv_fields).flatten()
            .any(|name| name.trim_start_matches('\\').eq_ignore_ascii_case(&context.task_name));
        if !present { return Ok(None); }
    }
    Err(Failure::operation(format!("Не удалось прочитать автозапуск: {}", output_error(&output))))
}

fn verify_requested(context: &Context, enabled: bool, elevated: bool) -> Result<StartupStatus> {
    match query_task(context)? {
        Some(status) if enabled && status.enabled && status.elevated == elevated => Ok(status),
        None if !enabled => Ok(StartupStatus::default()),
        _ => Err(Failure::verify()),
    }
}

fn apply_local(context: &Context, enabled: bool, elevated: bool) -> Result<StartupStatus> {
    if enabled && elevated { validate_elevated_location(context)?; }
    let previous = query_task(context)?; // Verifies marker, SID and executable before any overwrite/delete.
    if enabled {
        if !context.executable.is_file() { return Err(Failure::operation("Файл Orbit Widgets не найден")); }
        register_in_memory(context, elevated, previous.is_some())?;
    } else if previous.is_some() {
        let output = schtasks(&["/Delete".into(), "/TN".into(), context.task_name.clone().into(), "/F".into()])?;
        if !output.status.success() { return Err(Failure::operation(format!("Не удалось отключить автозапуск: {}", output_error(&output)))); }
    }
    verify_requested(context, enabled, elevated)
}

fn create_xml(context: &Context, elevated: bool) -> String {
    let sid = escape_xml(&context.sid);
    let command = escape_xml(&context.executable.to_string_lossy());
    let directory = escape_xml(&context.executable.parent().unwrap_or(Path::new("")).to_string_lossy());
    let level = if elevated { "HighestAvailable" } else { "LeastPrivilege" };
    format!(r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="{TASK_NS}">
 <RegistrationInfo><Author>{sid}</Author><Description>Запуск Orbit Widgets при входе текущего пользователя</Description><Source>{SOURCE}</Source></RegistrationInfo>
 <Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{sid}</UserId><Delay>PT10S</Delay></LogonTrigger></Triggers>
 <Principals><Principal id="Author"><UserId>{sid}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>{level}</RunLevel></Principal></Principals>
 <Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><AllowHardTerminate>false</AllowHardTerminate><StartWhenAvailable>true</StartWhenAvailable><RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable><IdleSettings><StopOnIdleEnd>false</StopOnIdleEnd><RestartOnIdle>false</RestartOnIdle></IdleSettings><AllowStartOnDemand>true</AllowStartOnDemand><Enabled>true</Enabled><Hidden>false</Hidden><RunOnlyIfIdle>false</RunOnlyIfIdle><WakeToRun>false</WakeToRun><ExecutionTimeLimit>PT0S</ExecutionTimeLimit><Priority>7</Priority></Settings>
 <Actions Context="Author"><Exec><Command>{command}</Command><Arguments>--autostart</Arguments><WorkingDirectory>{directory}</WorkingDirectory></Exec></Actions>
</Task>"#)
}

fn escape_xml(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() { match c { '&' => out.push_str("&amp;"), '<' => out.push_str("&lt;"), '>' => out.push_str("&gt;"), '"' => out.push_str("&quot;"), '\'' => out.push_str("&apos;"), c => out.push(c) } }
    out
}

fn owned_status(xml: &str, context: &Context) -> Result<StartupStatus> {
    owned_status_with_resolver(xml, context, &resolve_account_sid)
}

fn owned_status_with_resolver(
    xml: &str,
    context: &Context,
    resolve: &impl Fn(&str) -> Option<String>,
) -> Result<StartupStatus> {
    let foreign = || Failure::operation("Задание автозапуска принадлежит другой программе или установке Orbit Widgets; оно не изменено");
    if xml.len() > 131_072 || xml.to_ascii_uppercase().contains("<!DOCTYPE") { return Err(foreign()); }
    let mut stack = Vec::<String>::new();
    let mut values = HashMap::<String, String>::new();
    let (mut principals, mut actions, mut triggers) = (0, 0, 0);
    let (mut principal_id, mut action_context) = (String::new(), String::new());
    // The string is decoded already; an exported UTF-16 declaration otherwise
    // conflicts with the UTF-8 bytes passed to EventReader::from_str.
    let parsed_xml = xml.replacen("encoding=\"UTF-16\"", "encoding=\"UTF-8\"", 1).replacen("encoding=\"utf-16\"", "encoding=\"UTF-8\"", 1);
    for event in EventReader::from_str(&parsed_xml) {
        match event.map_err(|_| foreign())? {
            XmlEvent::StartElement { name, attributes, .. } => {
                if name.namespace.as_deref() != Some(TASK_NS) { return Err(foreign()); }
                stack.push(name.local_name.clone());
                let path = stack.join("/");
                if stack.len() == 1 && name.local_name != "Task" { return Err(foreign()); }
                if path == "Task/Principals/Principal" {
                    principals += 1;
                    principal_id = attributes.iter().find(|a| a.name.local_name == "id").map(|a| a.value.clone()).unwrap_or_default();
                }
                if stack.len() == 3 && stack[1] == "Actions" { actions += 1; if name.local_name != "Exec" { return Err(foreign()); } }
                if stack.len() == 3 && stack[1] == "Triggers" { triggers += 1; if name.local_name != "LogonTrigger" { return Err(foreign()); } }
                if path == "Task/Actions" { action_context = attributes.iter().find(|a| a.name.local_name == "Context").map(|a| a.value.clone()).unwrap_or_default(); }
            }
            XmlEvent::Characters(text) | XmlEvent::CData(text) => { values.entry(stack.join("/")).or_default().push_str(&text); }
            XmlEvent::EndElement { .. } => { stack.pop(); }
            _ => {}
        }
    }
    let value = |key: &str| values.get(key).map(|s| s.trim()).unwrap_or("");
    let run_level = value("Task/Principals/Principal/RunLevel");
    if principals != 1 || actions != 1 || triggers != 1 || principal_id.is_empty() || principal_id != action_context
        || value("Task/RegistrationInfo/Source") != SOURCE
        || !same_user(value("Task/Principals/Principal/UserId"), &context.sid, resolve)
        || !same_user(value("Task/Triggers/LogonTrigger/UserId"), &context.sid, resolve)
        || value("Task/Principals/Principal/LogonType") != "InteractiveToken"
        // Task Scheduler omits the default LeastPrivilege element on export.
        || !matches!(run_level, "" | "LeastPrivilege" | "HighestAvailable")
        || !same_path(value("Task/Actions/Exec/Command"), &context.executable)
        || !same_path(value("Task/Actions/Exec/WorkingDirectory"), context.executable.parent().unwrap_or(Path::new("")))
        || value("Task/Actions/Exec/Arguments") != "--autostart" { return Err(foreign()); }
    let enabled = |key: &str| matches!(value(key), "" | "true" | "1");
    Ok(StartupStatus { enabled: enabled("Task/Settings/Enabled") && enabled("Task/Triggers/LogonTrigger/Enabled"), elevated: run_level == "HighestAvailable" })
}

fn same_user(value: &str, expected_sid: &str, resolve: &impl Fn(&str) -> Option<String>) -> bool {
    if valid_sid(value) { return value.eq_ignore_ascii_case(expected_sid); }
    // A matching display name is never sufficient: resolve the account through
    // Windows and compare its SID to the signed-in user's SID.
    resolve(value).is_some_and(|sid| valid_sid(&sid) && sid.eq_ignore_ascii_case(expected_sid))
}

fn resolve_account_sid(account: &str) -> Option<String> {
    if account.is_empty() || account.len() > 1024 || account.contains('\0') { return None; }
    if valid_sid(account) { return Some(account.to_owned()); }
    let name = wide(OsStr::new(account));
    let (mut sid_size, mut domain_size, mut sid_use) = (0u32, 0u32, 0u32);
    unsafe {
        native::LookupAccountNameW(std::ptr::null(), name.as_ptr(), null_mut(), &mut sid_size, null_mut(), &mut domain_size, &mut sid_use);
    }
    if sid_size == 0 || sid_size > 4096 || domain_size > 32768 { return None; }
    let mut sid = vec![0u8; sid_size as usize];
    let mut domain = vec![0u16; domain_size.max(1) as usize];
    if unsafe {
        native::LookupAccountNameW(std::ptr::null(), name.as_ptr(), sid.as_mut_ptr().cast(), &mut sid_size, domain.as_mut_ptr(), &mut domain_size, &mut sid_use)
    } == 0 { return None; }
    sid_to_string(sid.as_ptr().cast())
}

fn sid_to_string(sid: *const c_void) -> Option<String> {
    let mut text = null_mut();
    if unsafe { native::ConvertSidToStringSidW(sid, &mut text) } == 0 || text.is_null() { return None; }
    let mut len = 0;
    unsafe { while len < 256 && *text.add(len) != 0 { len += 1; } }
    let value = unsafe { String::from_utf16_lossy(std::slice::from_raw_parts(text, len)) };
    unsafe { native::LocalFree(text.cast()); }
    valid_sid(&value).then_some(value)
}

fn same_path(value: &str, expected: &Path) -> bool {
    if value.is_empty() || !Path::new(value).is_absolute() { return false; }
    let normal = |value: &str| value.trim_start_matches("\\\\?\\").trim_end_matches(['\\', '/']).replace('/', "\\").to_lowercase();
    normal(value) == normal(&expected.to_string_lossy())
}

// Registration receives an owned BSTR directly. No temporary path or shared
// writable file exists between validation and the privileged Scheduler service.
struct RegistrationRequest { name: BSTR, xml: BSTR, user: VARIANT, flags: i32 }
impl RegistrationRequest {
    fn new(context: &Context, elevated: bool, update: bool) -> Self {
        Self { name: BSTR::from(context.task_name.as_str()), xml: BSTR::from(create_xml(context, elevated)),
            user: VARIANT::from(context.sid.as_str()),
            flags: (if update { TASK_UPDATE.0 } else { TASK_CREATE.0 }) | TASK_IGNORE_REGISTRATION_TRIGGERS.0 }
    }
}

struct ComApartment(bool);
impl ComApartment {
    fn initialize() -> windows::core::Result<Self> {
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if result == RPC_E_CHANGED_MODE { return Ok(Self(false)); }
        result.ok()?;
        Ok(Self(true))
    }
}
impl Drop for ComApartment { fn drop(&mut self) { if self.0 { unsafe { CoUninitialize(); } } } }

fn register_in_memory(context: &Context, elevated: bool, update: bool) -> Result<()> {
    let register = || -> windows::core::Result<()> {
        let _apartment = ComApartment::initialize()?;
        let service: ITaskService = unsafe { CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)? };
        let empty = VARIANT::default();
        unsafe { service.Connect(&empty, &empty, &empty, &empty)?; }
        let folder = unsafe { service.GetFolder(&BSTR::from("\\"))? };
        let request = RegistrationRequest::new(context, elevated, update);
        // CREATE refuses a new name collision. UPDATE refuses to recreate a
        // previously owned task that disappeared after the ownership check.
        let _registered = unsafe { folder.RegisterTask(&request.name, &request.xml, request.flags,
            &request.user, &empty, TASK_LOGON_INTERACTIVE_TOKEN, &empty)? };
        Ok(())
    };
    register().map_err(|error| Failure::operation(format!("Не удалось настроить автозапуск (Windows 0x{:08X})", error.code().0 as u32)))
}

fn schtasks(args: &[OsString]) -> Result<Output> {
    let system_directory = crate::windows_paths::system_directory().ok_or_else(|| Failure::operation("Не найдена системная папка Windows"))?;
    let result = Command::new(system_directory.join("schtasks.exe")).args(args).creation_flags(0x08000000)
        .output().map_err(|error| Failure::operation(format!("Планировщик заданий недоступен: {error}")));
    #[cfg(feature = "startup-qa")]
    if let Ok(output) = &result {
        if let Some(directory) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_owned)) {
            if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(directory.join("startup-debug.log")) {
                let _ = writeln!(file,"{:?}: {} | {}",args,output.status,output_error(output));
            }
            if output.status.success() && args.iter().any(|arg| arg == "/XML") {
                let _ = fs::write(directory.join("startup-task.xml"),decode_output(&output.stdout));
            }
        }
    }
    result
}
fn output_error(output: &Output) -> String {
    let raw = if output.stderr.is_empty() { &output.stdout } else { &output.stderr };
    decode_output(raw).trim().chars().take(700).collect()
}
fn decode_output(bytes: &[u8]) -> String {
    let utf16 = bytes.starts_with(&[0xFF, 0xFE]) || (bytes.len() >= 4 && bytes[1] == 0 && bytes[3] == 0);
    if utf16 {
        let start = if bytes.starts_with(&[0xFF, 0xFE]) { 2 } else { 0 };
        return String::from_utf16_lossy(&bytes[start..].chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect::<Vec<_>>());
    }
    if let Ok(text) = std::str::from_utf8(bytes) { return text.trim_start_matches('\u{FEFF}').to_owned(); }
    let needed = unsafe { native::MultiByteToWideChar(1, 0, bytes.as_ptr(), bytes.len().min(i32::MAX as usize) as i32, null_mut(), 0) };
    if needed <= 0 { return String::from_utf8_lossy(bytes).into_owned(); }
    let mut wide = vec![0u16; needed as usize];
    unsafe { native::MultiByteToWideChar(1, 0, bytes.as_ptr(), bytes.len() as i32, wide.as_mut_ptr(), needed); }
    String::from_utf16_lossy(&wide)
}
fn csv_fields(line: &str) -> Option<Vec<String>> {
    let mut result = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.trim().chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => { chars.next(); field.push('"'); }
            '"' => { quoted = !quoted; }
            ',' if !quoted => { result.push(std::mem::take(&mut field)); }
            _ => field.push(c),
        }
    }
    if quoted { return None; }
    result.push(field);
    Some(result)
}

fn wide(value: &OsStr) -> Vec<u16> { value.encode_wide().chain(Some(0)).collect() }
fn quote_argument(value: &str) -> String {
    let mut out = String::from("\"");
    let mut slashes = 0;
    for c in value.chars() {
        if c == '\\' { slashes += 1; continue; }
        if c == '"' { out.extend(std::iter::repeat_n('\\', slashes * 2 + 1)); out.push('"'); }
        else { out.extend(std::iter::repeat_n('\\', slashes)); out.push(c); }
        slashes = 0;
    }
    out.extend(std::iter::repeat_n('\\', slashes * 2)); out.push('"'); out
}

fn apply_elevated(context: &Context, enabled: bool, elevated: bool) -> Result<()> {
    // Never elevate the executable from a writable portable location, even
    // when UAC is required only to disable an old elevated registration.
    validate_elevated_location(context)?;
    let arguments = [APPLY_FLAG.to_owned(), u8::from(enabled).to_string(), u8::from(elevated).to_string(), context.sid.clone(), context.task_name.clone()]
        .iter().map(|arg| quote_argument(arg)).collect::<Vec<_>>().join(" ");
    let file = wide(context.executable.as_os_str());
    let parameters = wide(OsStr::new(&arguments));
    let verb = wide(OsStr::new("runas"));
    let directory = wide(context.executable.parent().unwrap_or(Path::new("")).as_os_str());
    let mut info: native::ShellExecuteInfo = unsafe { zeroed() };
    info.size = size_of::<native::ShellExecuteInfo>() as u32;
    info.mask = 0x40 | 0x400; // SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI
    info.verb = verb.as_ptr(); info.file = file.as_ptr(); info.parameters = parameters.as_ptr(); info.directory = directory.as_ptr();
    info.show = 0;
    unsafe {
        if native::ShellExecuteExW(&mut info) == 0 {
            let code = native::GetLastError();
            return Err(if code == 1223 { Failure { code: 3, message: "Изменение автозапуска отменено в запросе Windows".into() } }
                else { Failure::operation(format!("Не удалось запросить права администратора (Windows {code})")) });
        }
        if info.process.is_null() { return Err(Failure::operation("Windows не вернула процесс настройки автозапуска")); }
        let wait = native::WaitForSingleObject(info.process, 180_000);
        let mut code = 4;
        let got_code = native::GetExitCodeProcess(info.process, &mut code) != 0;
        native::CloseHandle(info.process);
        if wait != 0 || !got_code { return Err(Failure::operation("Настройка автозапуска ещё не подтверждена. Проверьте её состояние")); }
        match code { 0 => Ok(()), 3 => Err(Failure { code: 3, message: "Изменение автозапуска отменено".into() }), 5 => Err(Failure::verify()),
            _ => Err(Failure::operation(format!("Windows не сохранила автозапуск (код {code})"))) }
    }
}

fn validate_elevated_location(context: &Context) -> Result<()> {
    native_safety_preflight(&context.executable).map_err(Failure::operation)
}

pub(crate) fn native_safety_preflight(executable: &Path) -> std::result::Result<(), String> {
    security::validate_installation(executable)
}

fn process_token() -> Result<*mut c_void> {
    let mut token = null_mut();
    if unsafe { native::OpenProcessToken(native::GetCurrentProcess(), 8, &mut token) } == 0 { return Err(Failure::operation("Не удалось определить текущего пользователя Windows")); }
    Ok(token)
}
fn is_elevated() -> Result<bool> {
    let token = process_token()?;
    let (mut elevation, mut length) = (0u32, 0u32);
    let success = unsafe { native::GetTokenInformation(token, 20, (&mut elevation as *mut u32).cast(), 4, &mut length) };
    unsafe { native::CloseHandle(token); }
    if success == 0 { Err(Failure::operation("Не удалось проверить права текущего процесса")) } else { Ok(elevation != 0) }
}
fn current_sid() -> Result<String> {
    let token = process_token()?;
    let mut length = 0;
    unsafe { native::GetTokenInformation(token, 1, null_mut(), 0, &mut length); }
    if length == 0 || length > 65_536 { unsafe { native::CloseHandle(token); } return Err(Failure::operation("Не удалось прочитать идентификатор пользователя")); }
    let mut buffer = vec![0u8; length as usize];
    let success = unsafe { native::GetTokenInformation(token, 1, buffer.as_mut_ptr().cast(), length, &mut length) };
    unsafe { native::CloseHandle(token); }
    if success == 0 || buffer.len() < size_of::<*mut c_void>() { return Err(Failure::operation("Не удалось прочитать идентификатор пользователя")); }
    let sid_pointer = unsafe { std::ptr::read_unaligned(buffer.as_ptr().cast::<*mut c_void>()) };
    sid_to_string(sid_pointer).ok_or_else(|| Failure::operation("Не удалось преобразовать идентификатор пользователя"))
}

mod native {
    use std::ffi::c_void;
    #[repr(C)] pub struct ShellExecuteInfo {
        pub size: u32, pub mask: u32, pub window: *mut c_void, pub verb: *const u16, pub file: *const u16,
        pub parameters: *const u16, pub directory: *const u16, pub show: i32, pub instance: *mut c_void,
        pub id_list: *mut c_void, pub class: *const u16, pub class_key: *mut c_void, pub hot_key: u32,
        pub icon_or_monitor: *mut c_void, pub process: *mut c_void,
    }
    #[link(name = "shell32")] extern "system" { pub fn ShellExecuteExW(info: *mut ShellExecuteInfo) -> i32; }
    #[link(name = "advapi32")] extern "system" {
        pub fn OpenProcessToken(process: *mut c_void, access: u32, token: *mut *mut c_void) -> i32;
        pub fn GetTokenInformation(token: *mut c_void, class: i32, data: *mut c_void, size: u32, length: *mut u32) -> i32;
        pub fn ConvertSidToStringSidW(sid: *const c_void, string: *mut *mut u16) -> i32;
        pub fn LookupAccountNameW(system: *const u16, account: *const u16, sid: *mut c_void, sid_size: *mut u32, domain: *mut u16, domain_size: *mut u32, sid_use: *mut u32) -> i32;
    }
    #[link(name = "kernel32")] extern "system" {
        pub fn GetCurrentProcess() -> *mut c_void;
        pub fn CloseHandle(handle: *mut c_void) -> i32;
        pub fn LocalFree(memory: *mut c_void) -> *mut c_void;
        pub fn GetLastError() -> u32;
        pub fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
        pub fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
        pub fn MultiByteToWideChar(code_page: u32, flags: u32, input: *const u8, input_len: i32, output: *mut u16, output_len: i32) -> i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Context { Context { executable: PathBuf::from("C:\\Программы\\Orbit & Widgets\\orbit-widgets.exe"), sid: "S-1-5-21-100-200-300-1001".into(), task_name: "OrbitWidgets.Startup.Test.Unit".into() } }
    #[test]
    fn registration_xml_is_owned_in_memory_with_collision_safe_flags() {
        let context = fixture();
        let fresh = RegistrationRequest::new(&context, true, false);
        assert_eq!(fresh.name.to_string(), context.task_name);
        assert_eq!(fresh.xml.to_string(), create_xml(&context, true));
        assert_eq!(fresh.user.vt(), windows::Win32::System::Variant::VT_BSTR);
        assert_eq!(fresh.flags, TASK_CREATE.0 | TASK_IGNORE_REGISTRATION_TRIGGERS.0);
        let changed = RegistrationRequest::new(&context, false, true);
        assert_eq!(changed.flags, TASK_UPDATE.0 | TASK_IGNORE_REGISTRATION_TRIGGERS.0);
        assert!(changed.xml.to_string().contains("<RunLevel>LeastPrivilege</RunLevel>"));
    }

    #[test]
    #[ignore = "Creates, reads and removes only its unique normal-privilege test task in a native Windows session"]
    fn native_com_registration_round_trip_without_temporary_xml() {
        let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let context = Context { executable: std::env::current_exe().unwrap(), sid: current_sid().map_err(|e| e.message).unwrap(),
            task_name: format!("{TEST_PREFIX}Security118.{}.{nonce}", std::process::id()) };
        struct RemoveTask(Context);
        impl Drop for RemoveTask {
            fn drop(&mut self) { let _ = apply_local(&self.0, false, false); }
        }
        assert!(query_task(&context).map_err(|e| e.message).unwrap().is_none());
        let _cleanup = RemoveTask(context.clone());
        assert_eq!(apply_local(&context, true, false).map_err(|e| e.message).unwrap(), StartupStatus { enabled: true, elevated: false });
        assert_eq!(query_task(&context).map_err(|e| e.message).unwrap(), Some(StartupStatus { enabled: true, elevated: false }));
        // Exercise the UPDATE-only route without ever running the action.
        assert_eq!(apply_local(&context, true, false).map_err(|e| e.message).unwrap(), StartupStatus { enabled: true, elevated: false });
        assert_eq!(apply_local(&context, false, false).map_err(|e| e.message).unwrap(), StartupStatus::default());
        assert!(query_task(&context).map_err(|e| e.message).unwrap().is_none());
    }
    #[test]
    fn task_xml_round_trips_unicode_escaped_paths_and_both_privilege_levels() {
        let context = fixture();
        assert_eq!(escape_xml("a<&\"'b"), "a&lt;&amp;&quot;&apos;b");
        for elevated in [false, true] {
            let xml = create_xml(&context, elevated);
            assert!(xml.contains("<Delay>PT10S</Delay>"));
            assert!(xml.contains("<LogonType>InteractiveToken</LogonType>"));
            assert!(!xml.contains("<Password>"));
            assert_eq!(owned_status(&xml, &context).map_err(|e| e.message).unwrap(), StartupStatus { enabled: true, elevated });
        }
    }
    #[test]
    fn ownership_rejects_other_binary_sid_source_and_additional_actions() {
        let context = fixture();
        let xml = create_xml(&context, false);
        for foreign in [xml.replace(SOURCE, "OtherProgram"), xml.replace("--autostart", "--other"), xml.replace("1001", "1002"), xml.replace("</Actions>", "<Exec><Command>other.exe</Command></Exec></Actions>")] {
            assert!(owned_status(&foreign, &context).is_err());
        }
        let disabled = xml.replace("<Settings><MultipleInstancesPolicy>", "<Settings><Enabled>false</Enabled><MultipleInstancesPolicy>").replacen("<Enabled>true</Enabled><Hidden>", "<Hidden>", 1);
        assert!(!owned_status(&disabled, &context).map_err(|e| e.message).unwrap().enabled);
    }
    #[test]
    fn test_names_and_cli_reject_unrelated_tasks_and_bad_phases() {
        assert!(valid_test_task("OrbitWidgets.Startup.Test.RoundTrip-1"));
        assert!(!valid_test_task("Karing Elevated Startup"));
        assert!(!valid_test_task("OrbitWidgets.Startup.Test.x\\other"));
        assert_eq!(handle_cli(&["app".into(), CONFIGURE_FLAG.into(), "enable".into(), "wrong".into()]), Some(2));
        assert_eq!(handle_cli(&["app".into(), APPLY_FLAG.into(), "1".into()]), Some(2));
        assert_eq!(handle_cli(&["app".into(), "--settings".into()]), None);
        assert_eq!(quote_argument("path with space\\"), "\"path with space\\\\\"");
        assert_eq!(csv_fields("\"\\OrbitWidgets.Startup.Test.X\",\"next\""), Some(vec!["\\OrbitWidgets.Startup.Test.X".into(), "next".into()]));
    }
    #[test]
    fn task_listing_accepts_hostname_before_taskname_and_escaped_csv_fields() {
        let row = "\"DESKTOP-PC\",\"\\OrbitWidgets.Startup.Test.RoundTrip\",\"N/A\",\"Ready\"";
        let fields = csv_fields(row).unwrap();
        assert_eq!(fields[0], "DESKTOP-PC");
        assert_eq!(fields[1], "\\OrbitWidgets.Startup.Test.RoundTrip");
        assert!(fields.iter().any(|field| field.trim_start_matches('\\') == "OrbitWidgets.Startup.Test.RoundTrip"));
        assert_eq!(csv_fields("\"a, b\",\"say \"\"yes\"\"\""), Some(vec!["a, b".into(), "say \"yes\"".into()]));
        assert_eq!(csv_fields("\"unterminated"), None);
    }
    #[test]
    fn windows_export_default_runlevel_and_account_trigger_preserve_sid_ownership() {
        let context = fixture();
        let xml = create_xml(&context, false)
            .replace("<RunLevel>LeastPrivilege</RunLevel>", "")
            .replace("<Enabled>true</Enabled>", "")
            .replace(&format!("<LogonTrigger><UserId>{}</UserId>", context.sid), "<LogonTrigger><UserId>DESKTOP\\Kirill</UserId>");
        let resolver = |account: &str| (account == "DESKTOP\\Kirill").then(|| context.sid.clone());
        assert_eq!(owned_status_with_resolver(&xml, &context, &resolver).map_err(|e| e.message).unwrap(), StartupStatus { enabled: true, elevated: false });
        let other_user = xml.replace("DESKTOP\\Kirill", "DESKTOP\\SomeoneElse");
        assert!(owned_status_with_resolver(&other_user, &context, &resolver).is_err());
        let wrong_sid = |_account: &str| Some("S-1-5-21-100-200-300-9999".to_owned());
        assert!(owned_status_with_resolver(&xml, &context, &wrong_sid).is_err());
        assert!(owned_status_with_resolver(&xml.replace(SOURCE, "OtherProgram"), &context, &resolver).is_err());
    }
}
