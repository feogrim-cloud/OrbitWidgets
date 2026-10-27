//! Export actual Orbit UI scenes with fixed demonstration values.
//! cargo run --release --features gallery --example gallery -- 
//!   --scene hero-horizontal --output assets/screenshots/hero-horizontal.png
//! The example never loads/saves a profile, starts collectors, queries startup,
//! registers a task, installs a shell hook, or interacts with an existing instance.

#![allow(dead_code)]

#[path = "../src/config.rs"] mod config;
#[path = "../src/model.rs"] mod model;
#[path = "../src/ui.rs"] mod ui;

// The real UI references these bridges. Rendering uses deterministic values and
// deliberately cannot access native shell or startup configuration operations.
mod startup {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct StartupStatus { pub enabled: bool, pub elevated: bool }
    pub fn query() -> Result<StartupStatus, String> { Err("Недоступно в режиме иллюстраций".into()) }
    pub fn configure(_enabled: bool, _elevated: bool) -> Result<StartupStatus, String> { Err("Недоступно в режиме иллюстраций".into()) }
}

mod platform {
    pub fn prepare_taskbar_windows() {}
    pub fn monitor_names() -> Vec<String> { vec!["Экран 1 · DISPLAY1".into(), "Экран 2 · DISPLAY2".into()] }
    pub fn work_area_size(_index: usize) -> [f32; 2] { [1920.0, 1040.0] }
    pub fn anchor_position(_anchor: crate::config::Anchor, _index: usize, _size: [f32; 2], _margin: f32, _ppp: f32) -> Option<[f32; 2]> { None }
    pub fn tooltip_position(_anchor: eframe::egui::Rect, _size: [f32; 2], _ppp: f32) -> [f32; 2] { [0.0; 2] }
    pub fn initialize_tooltip_opacity() {}
    pub fn relaunch_elevated() -> bool { false }
}

fn usage() -> ! {
    eprintln!("gallery --scene hero-horizontal|theme-light|vertical|settings|gpu-memory|disk-speed --output FILE.png");
    std::process::exit(2);
}

fn main() -> eframe::Result<()> {
    let mut scene = None;
    let mut output = None;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--scene" => scene = args.next().as_deref().and_then(ui::gallery::Scene::parse),
            "--output" => output = args.next().map(std::path::PathBuf::from),
            _ => usage(),
        }
    }
    let (Some(scene), Some(output)) = (scene, output) else { usage() };
    if output.extension().and_then(|extension| extension.to_str()) != Some("png") { usage(); }
    if let Some(parent) = output.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).expect("create screenshot output directory");
    }
    // This variable belongs only to this short-lived rendering process. eframe's
    // screenshot feature exports the settled second GL frame and exits.
    std::env::set_var("EFRAME_SCREENSHOT_TO", &output);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Orbit Widgets — gallery renderer")
            .with_inner_size(scene.canvas()).with_position([80.0, 20.0])
            .with_decorations(false).with_resizable(false)
            .with_taskbar(false).with_active(false),
        renderer: eframe::Renderer::Glow, vsync: true, multisampling: 4,
        ..Default::default()
    };
    eframe::run_native("Orbit Gallery", options, Box::new(move |cc| {
        // Match the application's Windows font without redistributing font data.
        let font_path = std::env::var_os("WINDIR").map(std::path::PathBuf::from)
            .unwrap_or_else(|| "C:\\Windows".into()).join("Fonts").join("segoeui.ttf");
        if let Ok(data) = std::fs::read(font_path) {
            let mut fonts = eframe::egui::FontDefinitions::default();
            fonts.font_data.insert("windows-ui".into(), std::sync::Arc::new(eframe::egui::FontData::from_owned(data)));
            fonts.families.entry(eframe::egui::FontFamily::Proportional).or_default().insert(0, "windows-ui".into());
            cc.egui_ctx.set_fonts(fonts);
        }
        Ok(Box::new(ui::gallery::GalleryApp::new(scene)))
    }))
}
