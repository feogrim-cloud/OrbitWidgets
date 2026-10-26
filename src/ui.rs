use crate::{
    config::{Anchor, Config, Layout, NetworkUnit, Theme},
    model::{Metrics, MonitorCommand},
    startup::StartupStatus,
};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::{
    collections::HashMap,
    sync::{atomic::{AtomicBool, Ordering}, mpsc::{self, Receiver, Sender}, Arc},
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct UiSignals {
    pub open_settings: AtomicBool,
    pub click_through: AtomicBool,
    pub exit_requested: AtomicBool,
}

pub struct WidgetApp {
    config: Config,
    rx: Receiver<Metrics>,
    commands: Sender<MonitorCommand>,
    signals: Arc<UiSignals>,
    metrics: Metrics,
    last_sample: Option<Instant>,
    settings_open: bool,
    save_at: Option<Instant>,
    save_error: Option<String>,
    last_size: Vec2,
    last_work_area: [f32; 2],
    last_pixels_per_point: f32,
    need_anchor: bool,
    applied_locked: Option<bool>,
    applied_top: Option<bool>,
    applied_theme: Option<Theme>,
    smoothed: HashMap<String, [f32; 2]>,
    startup_status: Option<StartupStatus>,
    startup_rx: Option<Receiver<StartupOutcome>>,
    startup_error: Option<String>,
    tooltip: TooltipState,
}

struct StartupOutcome { status: Option<StartupStatus>, error: Option<String> }

const TOOLTIP_DELAY: Duration = Duration::from_millis(350);
const TOOLTIP_TEXT_WIDTH: f32 = 300.0;

#[derive(Clone)]
struct TooltipTarget { id: String, text: String, rect: Rect }

#[derive(Default)]
struct TooltipState { target: Option<TooltipTarget>, since: Option<Instant> }

impl TooltipState {
    fn track(&mut self, target: Option<TooltipTarget>, now: Instant) {
        let same_gauge = self.target.as_ref().zip(target.as_ref()).is_some_and(|(old, new)| old.id == new.id);
        if !same_gauge { self.since = target.as_ref().map(|_| now); }
        self.target = target;
    }

    fn remaining_delay(&self, now: Instant) -> Option<Duration> {
        self.since.map(|since| TOOLTIP_DELAY.saturating_sub(now.saturating_duration_since(since)))
    }
}

fn tooltip_text_layout(ctx: &egui::Context, text: &str, color: Color32) -> (Arc<egui::Galley>, Vec2) {
    // Measuring with an explicit wrap width keeps native tooltips independent
    // of the narrow transparent root viewport and its clip rectangle.
    let galley = ctx.layer_painter(egui::LayerId::background()).layout(text.to_owned(), FontId::proportional(13.0), color, TOOLTIP_TEXT_WIDTH);
    let size = Vec2::new((galley.size().x + 24.0).clamp(280.0, TOOLTIP_TEXT_WIDTH + 24.0), galley.size().y.ceil() + 20.0);
    (galley, size)
}

#[derive(Clone)]
struct Gauge {
    id: String,
    label: String,
    main: String,
    detail: String,
    outer: Option<f32>,
    inner: Option<f32>,
    accent: Color32,
    secondary: Color32,
    tooltip: String,
}

#[derive(Debug)]
struct LayoutPlan { columns: usize, rows: usize, diameter: f32, gap: f32, size: Vec2 }

fn layout_plan(count: usize, requested_diameter: f32, requested_gap: f32, orientation: Layout, work_area: [f32; 2]) -> LayoutPlan {
    let count = count.max(1);
    let available = Vec2::new(work_area[0].max(100.0), work_area[1].max(100.0));
    let mut diameter = requested_diameter;
    let mut gap = requested_gap;
    let (columns, rows) = loop {
        let max_columns = (((available.x - 16.0 + gap) / (diameter + gap)).floor() as usize).max(1);
        let max_rows = (((available.y - 16.0 + gap) / (diameter + gap)).floor() as usize).max(1);
        if count <= max_columns * max_rows || diameter <= 48.0 {
            break match orientation {
                Layout::Horizontal => { let columns = count.min(max_columns); (columns, count.div_ceil(columns)) },
                Layout::Vertical => { let rows = count.min(max_rows); (count.div_ceil(rows), rows) },
            };
        }
        // Keep every enabled volume visible when the requested size cannot fit.
        diameter = (diameter - 2.0).max(48.0);
        gap = requested_gap.min(diameter * 0.4);
    };
    let size = Vec2::new(columns as f32 * diameter + columns.saturating_sub(1) as f32 * gap,
        rows as f32 * diameter + rows.saturating_sub(1) as f32 * gap) + Vec2::splat(16.0);
    LayoutPlan { columns, rows, diameter, gap, size }
}

#[derive(Clone, Copy)]
struct Palette {
    fill: Color32,
    text: Color32,
    muted: Color32,
    track: Color32,
    blue: Color32,
    green: Color32,
    purple: Color32,
    orange: Color32,
}

impl Palette {
    fn new(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self {
                fill: Color32::from_rgba_unmultiplied(244, 249, 253, 245),
                text: Color32::from_rgb(8, 48, 77), muted: Color32::from_rgb(67, 99, 123),
                track: Color32::from_rgb(206, 219, 229),
                blue: Color32::from_rgb(0, 128, 156), green: Color32::from_rgb(0, 128, 156),
                purple: Color32::from_rgb(134, 83, 213), orange: Color32::from_rgb(215, 118, 25),
            },
            Theme::Neon => Self {
                fill: Color32::from_rgba_unmultiplied(9, 14, 29, 220),
                text: Color32::from_rgb(233, 255, 255), muted: Color32::from_rgb(139, 164, 183),
                track: Color32::from_rgb(30, 44, 64),
                blue: Color32::from_rgb(40, 227, 255), green: Color32::from_rgb(118, 255, 176),
                purple: Color32::from_rgb(196, 128, 255), orange: Color32::from_rgb(255, 158, 95),
            },
            Theme::Mono => Self {
                fill: Color32::from_rgba_unmultiplied(21, 23, 28, 225),
                text: Color32::from_rgb(244, 244, 245), muted: Color32::from_rgb(158, 163, 173),
                track: Color32::from_rgb(52, 55, 62),
                blue: Color32::from_rgb(235, 236, 239), green: Color32::from_rgb(235, 236, 239),
                purple: Color32::from_rgb(235, 236, 239), orange: Color32::from_rgb(156, 163, 175),
            },
            Theme::Midnight => Self {
                fill: Color32::from_rgba_unmultiplied(19, 26, 41, 232),
                text: Color32::from_rgb(237, 243, 252), muted: Color32::from_rgb(143, 160, 184),
                track: Color32::from_rgb(43, 55, 75),
                blue: Color32::from_rgb(89, 170, 255), green: Color32::from_rgb(91, 215, 170),
                purple: Color32::from_rgb(177, 145, 248), orange: Color32::from_rgb(248, 174, 100),
            },
        }
    }
}

impl WidgetApp {
    pub fn new(config: Config, rx: Receiver<Metrics>, commands: Sender<MonitorCommand>, signals: Arc<UiSignals>) -> Self {
        let _ = commands.send(MonitorCommand::SelectNetwork(config.network_adapter.clone()));
        signals.click_through.store(config.locked, Ordering::Relaxed);
        let mut app = Self {
            config, rx, commands, signals, metrics: Metrics::default(), last_sample: None,
            settings_open: false, save_at: None, save_error: None, last_size: Vec2::ZERO, last_work_area: [0.0; 2], last_pixels_per_point: 0.0,
            need_anchor: true, applied_locked: None, applied_top: None, applied_theme: None,
            smoothed: HashMap::new(),
            startup_status: None, startup_rx: None, startup_error: None,
            tooltip: TooltipState::default(),
        };
        app.request_startup(None);
        app
    }

    fn request_startup(&mut self, desired: Option<StartupStatus>) {
        if self.startup_rx.is_some() { return; }
        self.startup_error = None;
        let (tx, rx) = mpsc::channel();
        self.startup_rx = Some(rx);
        std::thread::spawn(move || {
            let result = match desired {
                Some(status) => crate::startup::configure(status.enabled, status.elevated),
                None => crate::startup::query(),
            };
            let outcome = match result {
                Ok(status) => StartupOutcome { status: Some(status), error: None },
                Err(error) => StartupOutcome { status: crate::startup::query().ok(), error: Some(error) },
            };
            let _ = tx.send(outcome);
        });
    }

    fn startup_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Запуск");
        let busy = self.startup_rx.is_some();
        if let Some(actual) = self.startup_status {
            let mut enabled = actual.enabled;
            let mut elevated = actual.elevated;
            let mut desired = None;
            ui.add_enabled_ui(!busy, |ui| {
                if ui.checkbox(&mut enabled, "Запускать при входе в Windows").changed() {
                    desired = Some(StartupStatus { enabled, elevated: enabled && elevated });
                }
                ui.add_enabled_ui(enabled, |ui| {
                    if ui.checkbox(&mut elevated, "С правами администратора").changed() {
                        desired = Some(StartupStatus { enabled, elevated });
                    }
                });
            });
            if let Some(status) = desired { self.request_startup(Some(status)); }
        }
        if busy { ui.horizontal(|ui| { ui.spinner(); ui.weak("Подождите…"); }); }
        if let Some(error) = &self.startup_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
            if ui.add_enabled(!busy, egui::Button::new("Проверить снова")).clicked() { self.request_startup(None); }
        }
    }

    fn mark_changed(&mut self) {
        // Continuous slider edits and window dragging must not postpone saving indefinitely.
        self.save_at.get_or_insert_with(|| Instant::now() + Duration::from_millis(600));
    }

    fn set_locked(&mut self, locked: bool) {
        self.config.locked = locked;
        self.signals.click_through.store(locked, Ordering::Relaxed);
        if locked { self.settings_open = false; }
        self.mark_changed();
    }

    fn gauges(&self, p: Palette) -> Vec<Gauge> {
        let m = &self.metrics;
        let mut gauges = Vec::new();
        if self.config.cpu_usage || self.config.cpu_temp {
            gauges.push(Gauge {
                id: "cpu".into(), label: "ЦП".into(),
                main: if self.config.cpu_usage { percent(m.cpu_usage) } else { temperature(m.cpu_temp) },
                detail: if self.config.cpu_temp && self.config.cpu_usage { temperature(m.cpu_temp) } else { "процессор".into() },
                outer: if self.config.cpu_usage { normalized(m.cpu_usage, 100.0) } else { None },
                inner: if self.config.cpu_temp { normalized(m.cpu_temp, 100.0) } else { None },
                accent: p.blue, secondary: heat_color(m.cpu_temp, p.orange),
                tooltip: format!("ЦП: {}\nТемпература: {}\nВнешний контур — нагрузка, внутренний — температура (0–100 °C).\n{}",
                    percent(m.cpu_usage), temperature(m.cpu_temp), m.temperature_status),
            });
        }
        if self.config.gpu_usage || self.config.gpu_temp {
            gauges.push(Gauge {
                id: "gpu".into(), label: "ГП".into(),
                main: if self.config.gpu_usage { percent(m.gpu_usage) } else { temperature(m.gpu_temp) },
                detail: if self.config.gpu_temp && self.config.gpu_usage { temperature(m.gpu_temp) } else { "графика".into() },
                outer: if self.config.gpu_usage { normalized(m.gpu_usage, 100.0) } else { None },
                inner: if self.config.gpu_temp { normalized(m.gpu_temp, 100.0) } else { None },
                accent: p.green, secondary: heat_color(m.gpu_temp, p.orange),
                tooltip: format!("ГП: {}\nНагрузка: {}\nТемпература: {}\nВнешний контур — нагрузка, внутренний — температура (0–100 °C).",
                    if m.gpu_name.is_empty() { "ожидание датчика" } else { &m.gpu_name }, percent(m.gpu_usage), temperature(m.gpu_temp)),
            });
        }
        if self.config.ram {
            gauges.push(Gauge {
                id: "ram".into(), label: "ОЗУ".into(), main: if m.ram_total > 0 { format!("{:.1}", gib(m.ram_used)).replace('.', ",") } else { "—".into() },
                detail: if m.ram_total > 0 { format!("из {:.0} ГБ", gib(m.ram_total)) } else { "ожидание".into() },
                outer: ratio(m.ram_used, m.ram_total), inner: None, accent: p.blue, secondary: p.blue,
                tooltip: format!("Оперативная память: {:.2} / {:.2} ГБ\nПравый клик — настройки. Перетаскивание — переместить.", gib(m.ram_used), gib(m.ram_total)),
            });
        }
        if self.config.disks {
            for disk in &m.disks {
                if self.config.hidden_disks.contains(&disk.mount) { continue; }
                let label = disk.mount.trim_end_matches(['\\', '/']).to_owned();
                gauges.push(Gauge {
                    id: format!("disk-{}", disk.mount), label,
                    main: if disk.total > 0 { compact(gib(disk.used)) } else { "—".into() },
                    detail: format!("из {} ГБ", compact(gib(disk.total))),
                    outer: ratio(disk.used, disk.total), inner: None, accent: p.blue, secondary: p.blue,
                    tooltip: format!("Том {}\nЗанято: {:.1} ГБ\nВсего: {:.1} ГБ\nСвободно: {:.1} ГБ",
                        disk.mount, gib(disk.used), gib(disk.total), gib(disk.total.saturating_sub(disk.used))),
                });
            }
        }
        if self.config.network {
            let rx = m.network_rx_bps.max(0.0);
            let tx = m.network_tx_bps.max(0.0);
            let divisor = match self.config.network_unit { NetworkUnit::Megabits => 125_000.0, NetworkUnit::Megabytes => 1_000_000.0 };
            let unit = match self.config.network_unit { NetworkUnit::Megabits => "Мбит/с", NetworkUnit::Megabytes => "МБ/с" };
            let maximum = self.config.network_scale_mbps.max(1.0) * 125_000.0;
            gauges.push(Gauge {
                id: "network".into(), label: format!("СЕТЬ · {unit}"), main: format!("↓ {}", compact(rx / divisor)),
                detail: format!("↑ {}", compact(tx / divisor)),
                outer: Some((rx / maximum).clamp(0.0, 1.0) as f32),
                inner: Some((tx / maximum).clamp(0.0, 1.0) as f32),
                accent: p.blue, secondary: p.purple,
                tooltip: format!("{}\nПриём: {:.2} {}\nОтдача: {:.2} {}\nВнешний контур — приём, внутренний — отдача.\nМаксимум шкалы: {} Мбит/с. Это трафик выбранного адаптера, включая локальную сеть.",
                    m.network_name, rx / divisor, unit, tx / divisor, unit, compact(self.config.network_scale_mbps)),
            });
        }
        if gauges.is_empty() {
            gauges.push(Gauge { id: "empty".into(), label: "ORBIT".into(), main: "···".into(), detail: "правый клик".into(),
                outer: None, inner: None, accent: p.blue, secondary: p.blue, tooltip: "Все виджеты выключены. Правый клик — открыть настройки.".into() });
        }
        if self.last_sample.is_some_and(|sample| sample.elapsed() > Duration::from_secs(4)) {
            for gauge in &mut gauges {
                gauge.main = "—".into(); gauge.detail = "нет обновления".into();
                gauge.outer = None; gauge.inner = None;
                gauge.tooltip.push_str("\nДанные не обновлялись более 4 секунд.");
            }
        }
        gauges
    }

    fn draw_gauge(&mut self, ui: &mut egui::Ui, rect: Rect, gauge: &Gauge, palette: Palette, dt: f32) -> (bool, Option<TooltipTarget>) {
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        let center = rect.center();
        let diameter = rect.width();
        let radius = diameter * 0.5 - 4.0;
        let opacity = self.config.opacity;
        let paint = ui.painter();
        paint.circle_filled(center, radius - 1.0, alpha(palette.fill, opacity));
        let outer_width = (diameter * 0.043).clamp(3.5, 8.0);
        let inner_width = (diameter * 0.027).clamp(2.5, 5.0);
        let inner_radius = radius - outer_width - diameter * 0.032;
        let targets = [gauge.outer.unwrap_or(0.0), gauge.inner.unwrap_or(0.0)];
        let smooth = self.smoothed.entry(gauge.id.clone()).or_insert(targets);
        let interpolation = 1.0 - (-dt.min(0.1) * 24.0).exp();
        let mut animating = false;
        for (value, target) in smooth.iter_mut().zip(targets) {
            *value += (target - *value) * interpolation;
            if (*value - target).abs() < 0.001 { *value = target; } else { animating = true; }
        }
        // Both tracks stay visible for missing sensors, with an explicit dash in the label.
        let outer_enabled = match gauge.id.as_str() { "cpu" => self.config.cpu_usage, "gpu" => self.config.gpu_usage, _ => true };
        let inner_enabled = match gauge.id.as_str() { "cpu" => self.config.cpu_temp, "gpu" => self.config.gpu_temp, "network" => true, _ => false };
        if outer_enabled {
            paint.circle_stroke(center, radius, Stroke::new(outer_width, alpha(palette.track, opacity)));
            if gauge.outer.is_some() { draw_arc(paint, center, radius, smooth[0], outer_width, alpha(gauge.accent, opacity)); }
        }
        if inner_enabled {
            paint.circle_stroke(center, inner_radius, Stroke::new(inner_width, alpha(palette.track, opacity)));
            if gauge.inner.is_some() { draw_arc(paint, center, inner_radius, smooth[1], inner_width, alpha(gauge.secondary, opacity)); }
        }
        let scale = diameter / 128.0;
        paint.text(center + Vec2::new(0.0, -27.0 * scale), Align2::CENTER_CENTER, &gauge.label,
            FontId::proportional((11.0 * scale).max(9.0)), alpha(palette.muted, opacity));
        let main_font = if gauge.main.chars().count() > 6 { 21.0 } else { 28.0 };
        paint.text(center + Vec2::new(0.0, -2.0 * scale), Align2::CENTER_CENTER, &gauge.main,
            FontId::proportional(main_font * scale), alpha(palette.text, opacity));
        paint.text(center + Vec2::new(0.0, 23.0 * scale), Align2::CENTER_CENTER, &gauge.detail,
            FontId::proportional((12.0 * scale).max(9.0)), alpha(if gauge.inner.is_some() { gauge.secondary } else { palette.muted }, opacity));
        let hovered = response.hovered() && !response.dragged()
            && ui.input(|input| !input.pointer.any_down())
            && response.hover_pos().is_some_and(|position| position.distance(center) <= radius);
        let tooltip = hovered.then(|| TooltipTarget { id: gauge.id.clone(), text: gauge.tooltip.clone(), rect });
        // A native settings window can extend beyond the small transparent widget viewport.
        if response.secondary_clicked() { self.settings_open = true; }
        if response.drag_started() && !self.config.locked {
            self.config.anchor = Anchor::Free;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            self.mark_changed();
        }
        if response.drag_stopped() { self.mark_changed(); }
        (animating, tooltip)
    }

    fn native_tooltip(&self, ctx: &egui::Context, palette: Palette) {
        let Some(target) = &self.tooltip.target else { return; };
        let (galley, size) = tooltip_text_layout(ctx, &target.text, palette.text);
        let root_pixels_per_point = ctx.pixels_per_point();
        let [x, y] = crate::platform::tooltip_position(target.rect, [size.x, size.y], root_pixels_per_point);
        let fill = if self.config.theme == Theme::Light { Color32::from_rgb(244, 249, 253) }
            else { Color32::from_rgb(18, 25, 37) };
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("orbit-native-tooltip"),
            egui::ViewportBuilder::default()
                .with_title("Orbit Widgets — подсказка")
                .with_decorations(false).with_has_shadow(false).with_resizable(false)
                // Windows passthrough uses a layered HWND; an alpha-capable GL
                // surface keeps its contents visible. The panel itself is opaque.
                .with_transparent(true).with_taskbar(false).with_active(false).with_visible(false)
                .with_mouse_passthrough(true).with_always_on_top()
                .with_position([x, y]).with_inner_size(size),
            |tooltip_ctx, _class| {
                crate::platform::initialize_tooltip_opacity();
                let child_pixels_per_point = tooltip_ctx.pixels_per_point();
                tooltip_ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(
                    x * root_pixels_per_point / child_pixels_per_point,
                    y * root_pixels_per_point / child_pixels_per_point,
                )));
                tooltip_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                egui::CentralPanel::default().frame(egui::Frame::NONE.fill(fill)).show(tooltip_ctx, |ui| {
                    ui.painter().galley(ui.min_rect().min + Vec2::new(12.0, 10.0), galley.clone(), palette.text);
                });
            });
    }

    fn settings_ui(&mut self, ctx: &egui::Context) {
        let before = serde_json::to_string(&self.config).unwrap_or_default();
        let previous_adapter = self.config.network_adapter.clone();
        let previous_anchor = (self.config.anchor, self.config.monitor_index, self.config.margin);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(4.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.heading("Показатели");
                egui::Grid::new("enabled-metrics").num_columns(2).spacing([20.0, 5.0]).show(ui, |ui| {
                    ui.checkbox(&mut self.config.cpu_usage, "ЦП · нагрузка");
                    ui.checkbox(&mut self.config.cpu_temp, "ЦП · температура"); ui.end_row();
                    ui.checkbox(&mut self.config.gpu_usage, "ГП · нагрузка");
                    ui.checkbox(&mut self.config.gpu_temp, "ГП · температура"); ui.end_row();
                    ui.checkbox(&mut self.config.ram, "Оперативная память");
                    ui.checkbox(&mut self.config.network, "Сеть · приём и отдача"); ui.end_row();
                });
                ui.checkbox(&mut self.config.disks, "Диски по буквам");
                if self.config.disks {
                    ui.horizontal_wrapped(|ui| {
                        for disk in &self.metrics.disks {
                            let mut enabled = !self.config.hidden_disks.contains(&disk.mount);
                            if ui.checkbox(&mut enabled, disk.mount.trim_end_matches(['\\', '/'])).changed() {
                                if enabled { self.config.hidden_disks.retain(|x| x != &disk.mount); }
                                else { self.config.hidden_disks.push(disk.mount.clone()); }
                            }
                        }
                    });
                }
                if !self.metrics.temperature_status.is_empty() {
                    ui.add_space(3.0);
                    ui.label(egui::RichText::new(&self.metrics.temperature_status).small().weak());
                }
                if ui.small_button("Обновить поиск датчиков").clicked() { let _ = self.commands.send(MonitorCommand::RefreshSensors); }
                if self.metrics.cpu_temp.is_none() {
                    if ui.small_button("Перезапустить с правами администратора").clicked() {
                        if crate::platform::relaunch_elevated() { self.signals.exit_requested.store(true, Ordering::Relaxed); }
                    }
                }
                ui.add_space(12.0); ui.separator(); ui.add_space(5.0);
                ui.heading("Внешний вид");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.config.layout, Layout::Horizontal, "Горизонтально");
                    ui.selectable_value(&mut self.config.layout, Layout::Vertical, "Вертикально");
                });
                ui.add(egui::Slider::new(&mut self.config.diameter, 80.0..=220.0).text("Диаметр").suffix(" px"));
                ui.add(egui::Slider::new(&mut self.config.gap, 0.0..=48.0).text("Расстояние").suffix(" px"));
                ui.small("При нехватке места круги переносятся на следующий ряд или столбец. Размер автоматически уменьшается, если весь набор не помещается на экране.");
                let mut opacity = self.config.opacity * 100.0;
                if ui.add(egui::Slider::new(&mut opacity, 20.0..=100.0).text("Непрозрачность").suffix(" %")).changed() { self.config.opacity = opacity / 100.0; }
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.config.theme, Theme::Midnight, "Тёмная");
                    ui.selectable_value(&mut self.config.theme, Theme::Light, "Светлая");
                    ui.selectable_value(&mut self.config.theme, Theme::Neon, "Неон");
                    ui.selectable_value(&mut self.config.theme, Theme::Mono, "Моно");
                });
                ui.add_space(12.0); ui.separator(); ui.add_space(5.0);
                ui.heading("Расположение");
                egui::ComboBox::from_id_salt("anchor").selected_text(self.config.anchor.label()).show_ui(ui, |ui| {
                    for anchor in Anchor::ALL { ui.selectable_value(&mut self.config.anchor, anchor, anchor.label()); }
                });
                let monitors = crate::platform::monitor_names();
                if !monitors.is_empty() {
                    let selected = monitors.get(self.config.monitor_index).cloned().unwrap_or_else(|| "Основной монитор".into());
                    egui::ComboBox::from_id_salt("monitor").selected_text(selected).show_ui(ui, |ui| {
                        for (index, name) in monitors.iter().enumerate() { ui.selectable_value(&mut self.config.monitor_index, index, name); }
                    });
                }
                ui.add(egui::Slider::new(&mut self.config.margin, 0.0..=120.0).text("Отступ от края").suffix(" px"));
                ui.checkbox(&mut self.config.always_on_top, "Поверх других окон");
                ui.small("Перетаскивайте любой круг. Правый клик открывает настройки.");
                ui.add_space(7.0);
                if ui.button("Закрепить виджеты и пропускать клики").clicked() { self.set_locked(true); }
                ui.small("Ctrl + Alt + O — открыть настройки и снова разрешить перемещение.");
                ui.add_space(12.0); ui.separator(); ui.add_space(5.0);
                ui.heading("Сеть");
                let adapter_label = self.config.network_adapter.as_deref().unwrap_or("Автоматически · Ethernet / Wi-Fi");
                egui::ComboBox::from_id_salt("network-adapter").width(ui.available_width().min(390.0)).selected_text(adapter_label).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.network_adapter, None, "Автоматически · Ethernet / Wi-Fi");
                    for name in &self.metrics.network_interfaces { ui.selectable_value(&mut self.config.network_adapter, Some(name.clone()), name); }
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.config.network_unit, NetworkUnit::Megabits, "Мбит/с");
                    ui.selectable_value(&mut self.config.network_unit, NetworkUnit::Megabytes, "МБ/с");
                });
                ui.horizontal(|ui| {
                    ui.label("Максимум шкалы");
                    ui.add(egui::DragValue::new(&mut self.config.network_scale_mbps).range(1.0..=100_000.0).speed(5.0).suffix(" Мбит/с"));
                });
                ui.small("Шкала задаётся вручную; скорость порта не равна скорости тарифа. Автоматический режим считает физические Ethernet / Wi-Fi и не прибавляет VPN повторно.");
                ui.add_space(12.0); ui.separator(); ui.add_space(5.0);
                self.startup_settings(ui);
                ui.add_space(12.0); ui.separator(); ui.add_space(5.0);
                ui.small("Данные: нагрузка, ОЗУ и сеть — каждые 0,5 с; температуры — 1 с; место на дисках — 5 с. Значок «—» означает, что датчик пока недоступен.");
                ui.small("Объёмы памяти и дисков: 1 ГБ = 1024³ байт. Сеть: 1 МБ = 1 000 000 байт.");
                if let Some(error) = &self.save_error { ui.colored_label(Color32::from_rgb(245, 130, 115), format!("Не удалось сохранить настройки: {error}")); }
                ui.add_space(7.0);
                ui.horizontal(|ui| {
                    if ui.button("Вернуть настройки по умолчанию").clicked() { self.config = Config::default(); self.signals.click_through.store(false, Ordering::Relaxed); self.need_anchor = true; }
                    if ui.button("Готово").clicked() { self.settings_open = false; }
                    if ui.button("Выйти").clicked() { self.signals.exit_requested.store(true, Ordering::Relaxed); }
                });
                ui.add_space(12.0);
            });
        });
        if self.config.network_adapter != previous_adapter {
            let _ = self.commands.send(MonitorCommand::SelectNetwork(self.config.network_adapter.clone()));
        }
        if (self.config.anchor, self.config.monitor_index, self.config.margin) != previous_anchor { self.need_anchor = true; }
        if serde_json::to_string(&self.config).unwrap_or_default() != before { self.mark_changed(); }
    }
}

impl eframe::App for WidgetApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let startup_result = self.startup_rx.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(StartupOutcome {
                status: None, error: Some("Не удалось завершить проверку автозапуска. Повторите попытку.".into()),
            }),
        });
        if let Some(outcome) = startup_result {
            self.startup_status = outcome.status;
            self.startup_error = outcome.error;
            self.startup_rx = None;
        }
        if self.signals.exit_requested.load(Ordering::Relaxed) { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        while let Ok(metrics) = self.rx.try_recv() { self.metrics = metrics; self.last_sample = Some(Instant::now()); }
        if self.signals.open_settings.swap(false, Ordering::Relaxed) {
            self.settings_open = true; self.set_locked(false);
        }
        let external_locked = self.signals.click_through.load(Ordering::Relaxed);
        if external_locked != self.config.locked { self.set_locked(external_locked); }
        if self.applied_theme != Some(self.config.theme) {
            let mut visuals = if self.config.theme == Theme::Light { egui::Visuals::light() } else { egui::Visuals::dark() };
            visuals.window_shadow = egui::epaint::Shadow::NONE;
            ctx.set_visuals(visuals); self.applied_theme = Some(self.config.theme);
        }
        let locked = self.config.locked && !self.settings_open;
        if self.applied_locked != Some(locked) {
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(locked));
            self.signals.click_through.store(locked, Ordering::Relaxed); self.applied_locked = Some(locked);
        }
        if self.applied_top != Some(self.config.always_on_top) {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.config.always_on_top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal }));
            self.applied_top = Some(self.config.always_on_top);
        }
        let palette = Palette::new(self.config.theme);
        let gauges = self.gauges(palette);
        let padding = 8.0;
        let work_area = crate::platform::work_area_size(self.config.monitor_index);
        if work_area != self.last_work_area { self.need_anchor = true; self.last_work_area = work_area; }
        let pixels_per_point = ctx.pixels_per_point();
        if pixels_per_point != self.last_pixels_per_point {
            self.need_anchor = true; self.last_pixels_per_point = pixels_per_point;
        }
        let inset = if self.config.anchor == Anchor::Free { 0.0 } else { self.config.margin * 2.0 };
        let plan = layout_plan(gauges.len(), self.config.diameter, self.config.gap, self.config.layout,
            [work_area[0] - inset, work_area[1] - inset]);
        let desired = plan.size;
        if desired != self.last_size {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(desired)); self.last_size = desired; self.need_anchor = true;
        }
        if self.need_anchor {
            let target = if self.config.anchor == Anchor::Free { self.config.position }
                else { crate::platform::anchor_position(self.config.anchor, self.config.monitor_index, [desired.x, desired.y], self.config.margin, pixels_per_point) };
            if let Some([x, y]) = target { ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(x, y))); }
            self.need_anchor = false;
        }
        if self.config.anchor == Anchor::Free {
            if let Some(position) = ctx.input(|input| input.viewport().outer_rect.map(|rect| [rect.min.x, rect.min.y])) {
                if self.config.position != Some(position) { self.config.position = Some(position); self.mark_changed(); }
            }
        }
        let dt = ctx.input(|i| i.stable_dt);
        let mut animating = false;
        let mut hovered_tooltip = None;
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            for (index, gauge) in gauges.iter().enumerate() {
                let (column, row) = match self.config.layout {
                    Layout::Horizontal => (index % plan.columns, index / plan.columns),
                    Layout::Vertical => (index / plan.rows, index % plan.rows),
                };
                let stride = plan.diameter + plan.gap;
                let min = ui.min_rect().min + Vec2::new(padding + column as f32 * stride, padding + row as f32 * stride);
                let rect = Rect::from_min_size(min, Vec2::splat(plan.diameter));
                let (gauge_animating, tooltip) = self.draw_gauge(ui, rect, gauge, palette, dt);
                animating |= gauge_animating;
                if tooltip.is_some() { hovered_tooltip = tooltip; }
            }
        });
        let origin = ctx.input(|input| input.viewport().inner_rect.or(input.viewport().outer_rect).map(|rect| rect.min));
        let hovered_tooltip = if self.settings_open || locked { None } else {
            hovered_tooltip.zip(origin).map(|(mut tooltip, origin)| { tooltip.rect = tooltip.rect.translate(origin.to_vec2()); tooltip })
        };
        let now = Instant::now();
        self.tooltip.track(hovered_tooltip, now);
        if let Some(remaining) = self.tooltip.remaining_delay(now) {
            if remaining.is_zero() { self.native_tooltip(ctx, palette); }
            else { ctx.request_repaint_after(remaining); }
        }
        if self.settings_open {
            let mut closed = false;
            ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("orbit-settings"),
                egui::ViewportBuilder::default().with_title("Orbit Widgets — настройки").with_inner_size([470.0, 750.0])
                    .with_min_inner_size([410.0, 480.0]).with_decorations(true).with_transparent(false).with_always_on_top(),
                |settings_ctx, _class| {
                    closed = settings_ctx.input(|i| i.viewport().close_requested());
                    self.settings_ui(settings_ctx);
                });
            if closed { self.settings_open = false; }
        }
        if self.save_at.is_some_and(|deadline| Instant::now() >= deadline) {
            match self.config.save() {
                Ok(()) => { self.save_error = None; self.save_at = None; }
                Err(error) => {
                    self.save_error = Some(error.to_string());
                    self.save_at = Some(Instant::now() + Duration::from_secs(2));
                }
            }
        }
        ctx.request_repaint_after(if animating { Duration::from_millis(33) } else { Duration::from_millis(250) });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] { [0.0, 0.0, 0.0, 0.0] }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) { let _ = self.config.save(); }
}

fn draw_arc(painter: &egui::Painter, center: Pos2, radius: f32, fraction: f32, width: f32, color: Color32) {
    if fraction <= 0.0001 { return; }
    let sweep = std::f32::consts::TAU * fraction.clamp(0.0, 1.0);
    let segments = ((radius * sweep / 2.0).ceil() as usize).clamp(3, 150);
    let points = (0..=segments).map(|index| {
        let angle = -std::f32::consts::FRAC_PI_2 + sweep * index as f32 / segments as f32;
        center + Vec2::new(angle.cos(), angle.sin()) * radius
    }).collect::<Vec<_>>();
    let start = points[0]; let end = *points.last().unwrap();
    painter.add(egui::Shape::line(points, Stroke::new(width, color)));
    painter.circle_filled(start, width / 2.0, color); painter.circle_filled(end, width / 2.0, color);
}

fn alpha(color: Color32, opacity: f32) -> Color32 { color.gamma_multiply(opacity) }
fn normalized(value: Option<f32>, max: f32) -> Option<f32> { value.filter(|v| v.is_finite()).map(|v| (v / max).clamp(0.0, 1.0)) }
fn ratio(used: u64, total: u64) -> Option<f32> { (total > 0).then_some((used as f64 / total.max(1) as f64).clamp(0.0, 1.0) as f32) }
fn percent(value: Option<f32>) -> String { value.filter(|v| v.is_finite()).map_or_else(|| "—".into(), |v| format!("{:.0}%", v.clamp(0.0, 100.0))) }
fn temperature(value: Option<f32>) -> String { value.filter(|v| v.is_finite()).map_or_else(|| "— °C".into(), |v| format!("{v:.0} °C")) }
fn heat_color(temp: Option<f32>, normal: Color32) -> Color32 { if temp.is_some_and(|t| t >= 85.0) { Color32::from_rgb(255, 102, 111) } else { normal } }
fn gib(bytes: u64) -> f64 { bytes as f64 / 1_073_741_824.0 }
fn compact(value: f64) -> String { (if value >= 100.0 { format!("{value:.0}") } else if value >= 10.0 { format!("{value:.1}") } else { format!("{value:.2}") }).replace('.', ",") }

/// Renderer-only documentation scenes. This wrapper deliberately bypasses the
/// application's constructor, collector, native shell and settings persistence.
#[cfg(feature = "gallery")]
pub mod gallery {
    use super::*;
    use crate::model::DiskMetric;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Scene { HeroHorizontal, ThemeLight, Vertical, Settings }

    impl Scene {
        pub fn parse(name: &str) -> Option<Self> { match name {
            "hero-horizontal" | "horizontal-neon" => Some(Self::HeroHorizontal),
            "theme-light" | "horizontal-light" => Some(Self::ThemeLight),
            "vertical" => Some(Self::Vertical), "settings" => Some(Self::Settings), _ => None,
        }}

        pub fn canvas(self) -> [f32; 2] { match self {
            Self::HeroHorizontal | Self::ThemeLight => [900.0, 200.0],
            Self::Vertical => [420.0, 800.0], Self::Settings => [520.0, 980.0],
        }}
    }

    pub struct GalleryApp { app: WidgetApp, scene: Scene }

    impl GalleryApp {
        pub fn new(scene: Scene) -> Self {
            let mut config = Config::default();
            config.theme = if scene == Scene::ThemeLight { Theme::Light } else { Theme::Neon };
            config.layout = if scene == Scene::Vertical || scene == Scene::Settings { Layout::Vertical } else { Layout::Horizontal };
            config.diameter = if scene == Scene::Vertical { 112.0 } else { 128.0 };
            config.gap = 14.0;
            config.opacity = 1.0;
            config.monitor_index = 1;
            config.anchor = Anchor::MiddleRight;
            config.margin = 10.0;
            let gib = 1_073_741_824_u64;
            let metrics = Metrics {
                cpu_usage: Some(37.0), cpu_temp: Some(62.0),
                gpu_usage: Some(54.0), gpu_temp: Some(66.0),
                gpu_name: "NVIDIA GeForce RTX 3060".into(),
                ram_used: (12.8 * gib as f64) as u64, ram_total: 32 * gib,
                disks: vec![DiskMetric { mount: "C:\\".into(), used: 186 * gib, total: 476 * gib },
                    DiskMetric { mount: "D:\\".into(), used: 742 * gib, total: 1000 * gib }],
                network_rx_bps: 32.4 * 125_000.0, network_tx_bps: 2.1 * 125_000.0,
                network_name: "Ethernet".into(), network_interfaces: vec!["Ethernet".into(), "Wi-Fi".into()],
                temperature_status: "Температура ЦП: MSI Afterburner, общая память".into(),
                sensor_helper_running: false,
            };
            let (_snapshots, rx) = mpsc::channel();
            let (commands, _command_rx) = mpsc::channel();
            let mut app = WidgetApp {
                config, rx, commands, signals: Arc::new(UiSignals::default()), metrics,
                last_sample: Some(Instant::now()), settings_open: false, save_at: None, save_error: None,
                last_size: Vec2::ZERO, last_work_area: [0.0; 2], last_pixels_per_point: 0.0,
                need_anchor: false, applied_locked: None, applied_top: None, applied_theme: None,
                smoothed: HashMap::new(), startup_status: Some(StartupStatus { enabled: false, elevated: false }),
                startup_rx: None, startup_error: None, tooltip: TooltipState::default(),
            };
            for gauge in app.gauges(Palette::new(app.config.theme)) {
                app.smoothed.insert(gauge.id, [gauge.outer.unwrap_or(0.0), gauge.inner.unwrap_or(0.0)]);
            }
            Self { app, scene }
        }
    }

    impl eframe::App for GalleryApp {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            ctx.set_pixels_per_point(1.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.scene.canvas().into()));
            ctx.set_visuals(if self.app.config.theme == Theme::Light { egui::Visuals::light() } else { egui::Visuals::dark() });
            if self.scene == Scene::Settings {
                self.app.settings_ui(ctx);
            } else {
                let palette = Palette::new(self.app.config.theme);
                let gauges = self.app.gauges(palette);
                let canvas = self.scene.canvas();
                let diameter = self.app.config.diameter;
                let gap = self.app.config.gap;
                let length = gauges.len() as f32 * diameter + gauges.len().saturating_sub(1) as f32 * gap;
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                    for (index, gauge) in gauges.iter().enumerate() {
                        let along = index as f32 * (diameter + gap);
                        let min = if self.scene == Scene::Vertical {
                            Pos2::new(canvas[0] - diameter - 28.0, (canvas[1] - length) / 2.0 + along)
                        } else {
                            Pos2::new((canvas[0] - length) / 2.0 + along, (canvas[1] - diameter) / 2.0)
                        };
                        let _ = self.app.draw_gauge(ui, Rect::from_min_size(min, Vec2::splat(diameter)), gauge, palette, 1.0 / 60.0);
                    }
                });
            }
            ctx.request_repaint();
        }

        fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
            if self.scene == Scene::ThemeLight { [0.90, 0.94, 0.97, 1.0] }
            else { [0.025, 0.045, 0.08, 1.0] }
        }

        fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
            // Documentation rendering is non-interactive, including the startup
            // controls. The example also supplies no-I/O platform/service stubs.
            raw.events.clear(); raw.dropped_files.clear(); raw.hovered_files.clear(); raw.focused = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_widget_set_fits_both_orientations_on_small_display() {
        for orientation in [Layout::Horizontal, Layout::Vertical] {
            let plan = layout_plan(30, 220.0, 48.0, orientation, [1280.0, 680.0]);
            assert!(plan.size.x <= 1280.0 && plan.size.y <= 680.0, "{plan:?}");
            assert!(plan.columns * plan.rows >= 30);
            assert!(plan.diameter < 220.0);
        }
    }

    #[test]
    fn nine_widgets_wrap_without_shrinking_when_two_rows_fit() {
        let plan = layout_plan(9, 220.0, 14.0, Layout::Horizontal, [1920.0, 1040.0]);
        assert_eq!(plan.rows, 2);
        assert_eq!(plan.diameter, 220.0);
        assert!(plan.size.x <= 1920.0 && plan.size.y <= 1040.0);
    }

    #[test]
    fn unknown_and_invalid_readings_never_become_fake_zeroes() {
        assert_eq!(percent(None), "—");
        assert_eq!(percent(Some(f32::NAN)), "—");
        assert_eq!(temperature(None), "— °C");
        assert_eq!(ratio(10, 0), None);
        assert_eq!(ratio(300, 100), Some(1.0));
        assert_eq!(gib(32 * 1_073_741_824), 32.0);
    }

    #[test]
    fn live_tooltip_values_do_not_restart_hover_delay_and_leaving_clears_it() {
        let start = Instant::now();
        let target = |text: &str| TooltipTarget { id: "network".into(), text: text.into(), rect: Rect::from_min_size(Pos2::ZERO, Vec2::splat(83.0)) };
        let mut tooltip = TooltipState::default();
        tooltip.track(Some(target("Приём: 1 Мбит/с")), start);
        tooltip.track(Some(target("Приём: 20 Мбит/с")), start + Duration::from_millis(300));
        assert_eq!(tooltip.remaining_delay(start + Duration::from_millis(350)), Some(Duration::ZERO));
        assert_eq!(tooltip.target.as_ref().unwrap().text, "Приём: 20 Мбит/с");
        tooltip.track(None, start + Duration::from_millis(400));
        assert!(tooltip.target.is_none());
        assert!(tooltip.remaining_delay(start + Duration::from_millis(800)).is_none());
    }

    #[test]
    fn network_tooltip_wraps_outside_a_narrow_vertical_widget_viewport() {
        let ctx = egui::Context::default();
        let raw_input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(99.0, 960.0))), ..Default::default() };
        let mut measured = Vec2::ZERO;
        let mut row_count = 0;
        let _ = ctx.run(raw_input, |ctx| {
            let (galley, size) = tooltip_text_layout(ctx,
                "Ethernet\nПриём: 123,45 Мбит/с\nОтдача: 2,10 Мбит/с\nВнешний контур — приём, внутренний — отдача.\nМаксимум шкалы: 100 Мбит/с. Это трафик выбранного адаптера, включая локальную сеть.", Color32::WHITE);
            measured = size; row_count = galley.rows.len();
        });
        assert!(measured.x >= 280.0 && measured.x <= 324.0);
        assert!(measured.x > 99.0);
        assert!(row_count > 5);
        assert!(measured.y > 100.0);
    }
}
