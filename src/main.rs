#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod metrics;
mod model;
mod platform;
mod startup;
mod ui;

use std::{sync::{atomic::Ordering, Arc}, time::{Duration, Instant}};
use eframe::egui;

fn app_icon() -> egui::IconData {
    let side = 64_u32;
    let mut rgba = vec![0; (side * side * 4) as usize];
    for y in 0..side { for x in 0..side {
        let dx = x as f32 - 31.5; let dy = y as f32 - 31.5;
        let r = (dx * dx + dy * dy).sqrt();
        let angle = (dy.atan2(dx) + std::f32::consts::FRAC_PI_2).rem_euclid(std::f32::consts::TAU);
        let color = if r < 29.5 { [20, 33, 44, 255] } else { [0, 0, 0, 0] };
        let color = if (25.0..29.0).contains(&r) { if angle < 4.6 { [76, 216, 221, 255] } else { [51, 70, 87, 255] } } else { color };
        let color = if (18.0..21.0).contains(&r) { if angle < 3.8 { [237, 181, 103, 255] } else { [51, 70, 87, 255] } } else { color };
        let i = ((y * side + x) * 4) as usize; rgba[i..i+4].copy_from_slice(&color);
    }}
    egui::IconData { rgba, width: side, height: side }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = startup::handle_cli(&args) { std::process::exit(code); }
    if args.iter().any(|arg| arg == "--quit") { platform::request_exit_existing(); return Ok(()); }
    if let Some(index) = args.iter().position(|a| a == "--diagnostics") {
        let path = args.get(index + 1).cloned().unwrap_or_else(|| "diagnostics.json".into());
        let (rx, _commands) = metrics::start_monitor();
        let until = Instant::now() + Duration::from_secs(8);
        let mut latest = model::Metrics::default();
        while Instant::now() < until {
            if let Ok(snapshot) = rx.recv_timeout(Duration::from_millis(600)) { latest = snapshot; }
        }
        if let Ok(json) = serde_json::to_vec_pretty(&latest) { let _ = std::fs::write(path, json); }
        return Ok(());
    }
    let mut primary = platform::acquire_instance();
    if !primary && args.iter().any(|a| a == "--elevated-restart") {
        for _ in 0..50 { std::thread::sleep(Duration::from_millis(100)); if platform::acquire_instance() { primary = true; break; } }
    }
    if !primary {
        if !args.iter().any(|arg| arg == "--autostart") { platform::show_existing(); }
        return Ok(());
    }
    let first_launch = !config::config_path().exists();
    let mut config = config::Config::load();
    if first_launch { config.monitor_index = platform::default_monitor_index(); }
    let (rx, commands) = metrics::start_monitor();
    let signals = Arc::new(ui::UiSignals::default());
    signals.click_through.store(config.locked, Ordering::Relaxed);
    if args.iter().any(|a| a == "--settings") { signals.open_settings.store(true, Ordering::Relaxed); }
    platform::start_shell(signals.clone());
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Orbit Widgets")
        .with_active(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_has_shadow(false)
        .with_resizable(false)
        .with_inner_size([720.0, 156.0])
        .with_taskbar(false)
        .with_window_level(if config.always_on_top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal })
        .with_icon(Arc::new(app_icon()));
    if let Some(position) = config.position { viewport = viewport.with_position(position); }
    let native_options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        vsync: true,
        multisampling: 4,
        ..Default::default()
    };
    eframe::run_native("Orbit Widgets", native_options, Box::new(move |cc| {
        // Use the installed Windows UI font for Cyrillic and traffic arrows.
        // The font is read locally and is not redistributed with the app.
        let font_path = std::env::var_os("WINDIR").map(std::path::PathBuf::from)
            .unwrap_or_else(|| "C:\\Windows".into()).join("Fonts").join("segoeui.ttf");
        if let Ok(data) = std::fs::read(font_path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert("windows-ui".into(), Arc::new(egui::FontData::from_owned(data)));
            fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0,"windows-ui".into());
            cc.egui_ctx.set_fonts(fonts);
        }
        Ok(Box::new(ui::WidgetApp::new(config, rx, commands, signals)))
    }))
}
