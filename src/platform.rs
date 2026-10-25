use crate::{config::Anchor, ui::UiSignals};
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, sync::{atomic::Ordering, Arc}, mem::{size_of, zeroed}, ptr::{null, null_mut}};
use windows_sys::Win32::{
    Foundation::*, Graphics::Gdi::*,
    System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
    UI::{WindowsAndMessaging::*, Shell::*, HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI}, Input::KeyboardAndMouse::*},
};

fn wide(s: &str) -> Vec<u16> { OsStr::new(s).encode_wide().chain(Some(0)).collect() }

#[derive(Clone)]
struct Monitor { bounds: RECT, work: RECT, scale: f32, name: String, primary: bool }

unsafe extern "system" fn enumerate_monitor(monitor: HMONITOR, _hdc: HDC, _rect: *mut RECT, data: LPARAM) -> BOOL {
    let list = &mut *(data as *mut Vec<Monitor>);
    let mut info: MONITORINFOEXW = zeroed(); info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(monitor, &mut info as *mut _ as *mut MONITORINFO) != 0 {
        let (mut dx, mut dy) = (96, 96); GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        let end = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
        list.push(Monitor { bounds: info.monitorInfo.rcMonitor, work: info.monitorInfo.rcWork, scale: dx.max(96) as f32 / 96.0,
            name: String::from_utf16_lossy(&info.szDevice[..end]).replace("\\\\.\\", ""), primary: info.monitorInfo.dwFlags & 1 != 0 });
    } 1
}

fn monitors() -> Vec<Monitor> {
    let mut list: Vec<Monitor> = Vec::new();
    unsafe { EnumDisplayMonitors(null_mut(), null(), Some(enumerate_monitor), &mut list as *mut _ as isize); }
    list.sort_by_key(|m| !m.primary); list
}

pub fn monitor_names() -> Vec<String> {
    monitors().iter().enumerate().map(|(i,m)| format!("Экран {} · {}", i + 1, m.name)).collect()
}

pub fn default_monitor_index() -> usize {
    monitors().iter().position(|monitor| !monitor.primary).unwrap_or(0)
}

pub fn work_area_size(monitor_index: usize) -> [f32;2] {
    let list=monitors();
    list.get(monitor_index).or_else(||list.first()).map(|m|[(m.work.right-m.work.left) as f32/m.scale,(m.work.bottom-m.work.top) as f32/m.scale]).unwrap_or([1920.0,1040.0])
}

pub fn anchor_position(anchor: Anchor, monitor_index: usize, size: [f32; 2], margin: f32, current_pixels_per_point: f32) -> Option<[f32; 2]> {
    if anchor == Anchor::Free { return None; }
    let list = monitors(); let m = list.get(monitor_index).or_else(|| list.first())?;
    // Winit converts OuterPosition using the current window DPI, while the
    // target monitor's work area is in physical pixels. Keep those distinct.
    let left=m.work.left as f32; let top=m.work.top as f32;
    let right=m.work.right as f32; let bottom=m.work.bottom as f32;
    let size=[size[0]*m.scale,size[1]*m.scale];let margin=margin*m.scale;
    let x=match anchor { Anchor::TopLeft|Anchor::MiddleLeft|Anchor::BottomLeft=>left+margin,
        Anchor::TopCenter|Anchor::BottomCenter=>(left+right-size[0])/2.0, _=>right-size[0]-margin };
    let y=match anchor { Anchor::TopLeft|Anchor::TopCenter|Anchor::TopRight=>top+margin,
        Anchor::MiddleLeft|Anchor::MiddleRight=>(top+bottom-size[1])/2.0, _=>bottom-size[1]-margin };
    let current_scale=current_pixels_per_point.max(0.5);
    Some([x.max(left)/current_scale,y.max(top)/current_scale])
}

/// Return global coordinates in the root viewport's logical points. The child
/// viewport converts them using its own DPI before applying OuterPosition.
pub fn tooltip_position(anchor: eframe::egui::Rect, size: [f32; 2], current_pixels_per_point: f32) -> [f32; 2] {
    let scale = current_pixels_per_point.max(0.5);
    let center = anchor.center() * scale;
    let list = monitors();
    let nearest = list.iter().min_by(|a, b| {
        let distance = |r: RECT| {
            let x = center.x.clamp(r.left as f32, r.right as f32);
            let y = center.y.clamp(r.top as f32, r.bottom as f32);
            (center.x - x).powi(2) + (center.y - y).powi(2)
        };
        distance(a.bounds).total_cmp(&distance(b.bounds))
    });
    let work = nearest.map(|m| [m.work.left as f32 / scale, m.work.top as f32 / scale,
        m.work.right as f32 / scale, m.work.bottom as f32 / scale])
        .unwrap_or([anchor.left() - size[0] - 20.0, anchor.top() - size[1],
            anchor.right() + size[0] + 20.0, anchor.bottom() + size[1]]);
    place_tooltip([anchor.left(), anchor.top(), anchor.right(), anchor.bottom()], size, work)
}

pub fn initialize_tooltip_opacity() {
    unsafe {
        let hwnd = FindWindowW(null(), wide("Orbit Widgets — подсказка").as_ptr());
        if hwnd.is_null() { return; }
        let mut process_id = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id != windows_sys::Win32::System::Threading::GetCurrentProcessId()
            || GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_LAYERED as isize == 0 { return; }
        let (mut color, mut opacity, mut flags) = (0, 0, 0);
        // Winit adds WS_EX_LAYERED for mouse passthrough. Windows requires an
        // initialized layer opacity before that native window becomes visible.
        if GetLayeredWindowAttributes(hwnd, &mut color, &mut opacity, &mut flags) == 0
            || flags & LWA_ALPHA == 0 || opacity != 255 {
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
        }
    }
}

fn place_tooltip(anchor: [f32; 4], size: [f32; 2], work: [f32; 4]) -> [f32; 2] {
    let margin = 8.0;
    let gap = 10.0;
    let left = work[0] + margin;
    let top = work[1] + margin;
    let right = (work[2] - margin - size[0]).max(left);
    let bottom = (work[3] - margin - size[1]).max(top);
    let beside_right = anchor[2] + gap;
    let beside_left = anchor[0] - gap - size[0];
    let x = if beside_right <= right { beside_right }
        else if beside_left >= left { beside_left }
        else if work[2] - anchor[2] >= anchor[0] - work[0] { beside_right }
        else { beside_left };
    [x.clamp(left, right), ((anchor[1] + anchor[3] - size[1]) / 2.0).clamp(top, bottom)]
}

#[cfg(test)]
mod tooltip_tests {
    use super::place_tooltip;

    fn assert_inside(position: [f32; 2], size: [f32; 2], work: [f32; 4]) {
        assert!(position[0] >= work[0] && position[1] >= work[1]);
        assert!(position[0] + size[0] <= work[2] && position[1] + size[1] <= work[3]);
    }

    #[test]
    fn right_edge_of_left_monitor_keeps_negative_coordinates() {
        let work = [-1280.0, 0.0, 0.0, 984.0];
        let anchor = [-107.0, 178.0, -24.0, 261.0];
        let size = [324.0, 160.0];
        let position = place_tooltip(anchor, size, work);
        assert_inside(position, size, work);
        assert!(position[0] + size[0] < anchor[0]);
    }

    #[test]
    fn left_edge_opens_to_the_right() {
        let work = [0.0, 0.0, 1920.0, 1040.0];
        let anchor = [10.0, 400.0, 93.0, 483.0];
        let size = [324.0, 160.0];
        let position = place_tooltip(anchor, size, work);
        assert_inside(position, size, work);
        assert!(position[0] > anchor[2]);
    }

    #[test]
    fn bottom_and_top_edges_keep_all_text_on_screen() {
        let work = [1920.0, -400.0, 3200.0, 584.0];
        let size = [324.0, 180.0];
        for anchor in [[3090.0, 530.0, 3173.0, 613.0], [3090.0, -420.0, 3173.0, -337.0]] {
            assert_inside(place_tooltip(anchor, size, work), size, work);
        }
    }

    #[test]
    fn mixed_dpi_placement_stays_on_the_selected_monitor() {
        let scale = 1.5;
        let work = [-1920.0 / scale, 0.0, 0.0, 1080.0 / scale];
        let anchor = [-160.0 / scale, 400.0 / scale, -35.0 / scale, 525.0 / scale];
        let size = [324.0, 180.0];
        let position = place_tooltip(anchor, size, work);
        let physical = [position[0] * scale, position[1] * scale];
        assert_inside(physical, [size[0] * scale, size[1] * scale], [-1920.0, 0.0, 0.0, 1080.0]);
        // Applying the child viewport's own DPI must preserve this location.
        let child_scale = 1.0;
        let child_logical = [physical[0] / child_scale, physical[1] / child_scale];
        assert_eq!(child_logical, physical);
    }

    #[test]
    fn oversized_tooltip_does_not_panic() {
        assert_eq!(place_tooltip([0.0, 0.0, 20.0, 20.0], [324.0, 180.0], [0.0, 0.0, 200.0, 100.0]), [8.0, 8.0]);
    }
}

pub fn relaunch_elevated() -> bool {
    let Ok(exe)=std::env::current_exe() else {return false};
    let file:Vec<u16>=exe.as_os_str().encode_wide().chain(Some(0)).collect();
    let result=unsafe {ShellExecuteW(null_mut(),wide("runas").as_ptr(),file.as_ptr(),wide("--settings --elevated-restart").as_ptr(),null(),SW_SHOWNORMAL)};
    result as isize > 32
}

pub fn acquire_instance() -> bool {
    unsafe { let handle=CreateMutexW(null(),0,wide("Local\\OrbitWidgetsInstance").as_ptr());
        if handle.is_null(){false}else if GetLastError()==ERROR_ALREADY_EXISTS {CloseHandle(handle);false}else{true} }
}

pub fn show_existing() {
    unsafe {let hwnd=FindWindowW(wide("OrbitWidgetsShell").as_ptr(),null());
        if !hwnd.is_null() { PostMessageW(hwnd,WM_APP+2,0,0); }}
}

pub fn request_exit_existing() {
    unsafe { let hwnd=FindWindowW(wide("OrbitWidgetsShell").as_ptr(),null());
        if !hwnd.is_null() { PostMessageW(hwnd,EXIT_MESSAGE,0,0); } }
}

const TRAY_MESSAGE:u32=WM_APP+1;
const OPEN_MESSAGE:u32=WM_APP+2;
const EXIT_MESSAGE:u32=WM_APP+3;
const ID_SETTINGS:usize=101;
const ID_LOCK:usize=102;
const ID_EXIT:usize=103;

unsafe extern "system" fn shell_proc(hwnd:HWND,message:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT {
    let raw=GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *const UiSignals;
    if !raw.is_null() {
        let signals=&*raw;
        if message==EXIT_MESSAGE {
            signals.exit_requested.store(true,Ordering::Relaxed);
            DestroyWindow(hwnd);
            return 0;
        }
        if message==OPEN_MESSAGE || (message==WM_HOTKEY && wparam==1) || (message==TRAY_MESSAGE && lparam as u32==WM_LBUTTONDBLCLK) {
            signals.click_through.store(false,Ordering::Relaxed); signals.open_settings.store(true,Ordering::Relaxed); return 0;
        }
        if message==WM_HOTKEY && wparam==2 { signals.click_through.fetch_xor(true,Ordering::Relaxed); return 0; }
        if message==TRAY_MESSAGE && (lparam as u32==WM_RBUTTONUP || lparam as u32==WM_CONTEXTMENU) {
            let menu=CreatePopupMenu();
            AppendMenuW(menu,MF_STRING,ID_SETTINGS,wide("Настройки…  Ctrl+Alt+O").as_ptr());
            let text=if signals.click_through.load(Ordering::Relaxed){"Разблокировать  Ctrl+Alt+L"}else{"Закрепить  Ctrl+Alt+L"};
            AppendMenuW(menu,MF_STRING,ID_LOCK,wide(text).as_ptr());
            AppendMenuW(menu,MF_SEPARATOR,0,null());
            AppendMenuW(menu,MF_STRING,ID_EXIT,wide("Выход").as_ptr());
            let mut point:POINT=zeroed();GetCursorPos(&mut point);SetForegroundWindow(hwnd);
            let picked=TrackPopupMenu(menu,TPM_RETURNCMD|TPM_RIGHTBUTTON,point.x,point.y,0,hwnd,null()) as usize;
            DestroyMenu(menu);PostMessageW(hwnd,WM_NULL,0,0);
            match picked { ID_SETTINGS=>{signals.click_through.store(false,Ordering::Relaxed);signals.open_settings.store(true,Ordering::Relaxed);},
                ID_LOCK=>{signals.click_through.fetch_xor(true,Ordering::Relaxed);},
                ID_EXIT=>{signals.exit_requested.store(true,Ordering::Relaxed);DestroyWindow(hwnd);},_=>{} }
            return 0;
        }
    }
    if message==WM_DESTROY {let mut data:NOTIFYICONDATAW=zeroed();data.cbSize=size_of::<NOTIFYICONDATAW>() as u32;data.hWnd=hwnd;data.uID=1;Shell_NotifyIconW(NIM_DELETE,&data);PostQuitMessage(0);return 0;}
    DefWindowProcW(hwnd,message,wparam,lparam)
}

pub fn start_shell(signals:Arc<UiSignals>) {
    std::thread::spawn(move|| unsafe {
        let instance=GetModuleHandleW(null());let class=wide("OrbitWidgetsShell");
        let wc=WNDCLASSW {lpfnWndProc:Some(shell_proc),hInstance:instance,lpszClassName:class.as_ptr(),..zeroed()};
        if RegisterClassW(&wc)==0 {return;}
        let hwnd=CreateWindowExW(0,class.as_ptr(),wide("Orbit Widgets control").as_ptr(),0,0,0,0,0,null_mut(),null_mut(),instance,null());
        if hwnd.is_null(){return;}
        // A normal shortcut may request settings from an elevated instance.
        // Only our payload-free settings message is allowed through UIPI.
        ChangeWindowMessageFilterEx(hwnd,OPEN_MESSAGE,MSGFLT_ALLOW,null_mut());
        ChangeWindowMessageFilterEx(hwnd,EXIT_MESSAGE,MSGFLT_ALLOW,null_mut());
        SetWindowLongPtrW(hwnd,GWLP_USERDATA,Arc::as_ptr(&signals) as isize);
        RegisterHotKey(hwnd,1,MOD_CONTROL|MOD_ALT|MOD_NOREPEAT,b'O' as u32);
        RegisterHotKey(hwnd,2,MOD_CONTROL|MOD_ALT|MOD_NOREPEAT,b'L' as u32);
        let icon_path=std::env::current_exe().ok().and_then(|p|p.parent().map(|p|p.join("orbit.ico")));
        let icon=icon_path.and_then(|p|{let p:Vec<u16>=p.as_os_str().encode_wide().chain(Some(0)).collect();
            let icon=LoadImageW(null_mut(),p.as_ptr(),IMAGE_ICON,0,0,LR_LOADFROMFILE|LR_DEFAULTSIZE);if icon.is_null(){None}else{Some(icon)}})
            .unwrap_or_else(||LoadIconW(null_mut(),IDI_APPLICATION));
        let mut data:NOTIFYICONDATAW=zeroed();data.cbSize=size_of::<NOTIFYICONDATAW>() as u32;data.hWnd=hwnd;data.uID=1;
        data.uFlags=NIF_MESSAGE|NIF_ICON|NIF_TIP;data.uCallbackMessage=TRAY_MESSAGE;data.hIcon=icon;
        let tip=wide("Orbit Widgets · настройки Ctrl+Alt+O");data.szTip[..tip.len()].copy_from_slice(&tip);
        Shell_NotifyIconW(NIM_ADD,&data);
        let mut message:MSG=zeroed();while GetMessageW(&mut message,null_mut(),0,0)>0 {TranslateMessage(&message);DispatchMessageW(&message);}
        UnregisterHotKey(hwnd,1);UnregisterHotKey(hwnd,2);
    });
}
