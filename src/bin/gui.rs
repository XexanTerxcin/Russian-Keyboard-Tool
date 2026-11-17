use eframe::egui::{self, Color32, RichText, Vec2};
use ksni::TrayMethods;
use log::{info, warn};
use russian_keyboard_tool::config::AppSettings;
use russian_keyboard_tool::gui::{SettingsWindow, TrayIcon, VirtualKeyboard};
use russian_keyboard_tool::input_method::{
    get_ibus_engine, EngineSupervisor, ShortcutListener, ENGINE_NAME,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const ACCENT: Color32 = Color32::from_rgb(122, 162, 247);
const ACTIVE_GREEN: Color32 = Color32::from_rgb(158, 206, 106);
const IDLE_GREY: Color32 = Color32::from_rgb(140, 150, 170);

const PILL_SIZE: [f32; 2] = [560.0, 78.0];

struct RussianKeyboardApp {
    is_russian: Arc<AtomicBool>,
    settings_window: SettingsWindow,
    show_settings: bool,
    show_keyboard: bool,
    show_about: bool,
    supervisor: EngineSupervisor,
    rt: tokio::runtime::Runtime,
    tray_handle: Option<ksni::Handle<TrayIcon>>,
    tray_mode: bool,
    shortcut: Option<ShortcutListener>,
    toggle_requested: Arc<AtomicBool>,
    show_window_requested: Arc<AtomicBool>,
    show_keyboard_requested: Arc<AtomicBool>,
    show_settings_requested: Arc<AtomicBool>,
    quit_requested: Arc<AtomicBool>,
    status: Option<(String, bool, Instant)>,
    startup_done: bool,
    last_toggle: Instant,
    last_supervise: Instant,
}

impl RussianKeyboardApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = AppSettings::load();
        let is_russian = Arc::new(AtomicBool::new(settings.default_input_mode == "RU"));

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to create the tokio runtime");

        let supervisor = EngineSupervisor::new(rt.handle().clone());

        // System tray
        let (tray, toggle_req, show_win_req, show_kb_req, show_settings_req, quit_req) =
            TrayIcon::new(is_russian.clone());

        let tray_handle = if settings.show_tray_icon {
            match rt.block_on(async { tray.spawn().await }) {
                Ok(handle) => {
                    info!("system tray icon registered");
                    Some(handle)
                }
                Err(e) => {
                    warn!("could not create the tray icon: {e}");
                    None
                }
            }
        } else {
            None
        };

        // Global shortcut (Meta + R). It only raises a flag; the UI thread performs
        // the actual engine switch so everything stays on one thread.
        let ctx_for_shortcut = cc.egui_ctx.clone();
        let toggle_for_shortcut = toggle_req.clone();
        let shortcut = ShortcutListener::start(move || {
            toggle_for_shortcut.store(true, Ordering::Relaxed);
            ctx_for_shortcut.request_repaint();
        });

        // Visual style
        let visuals = if settings.dark_theme {
            let mut v = egui::Visuals::dark();
            v.window_corner_radius = egui::CornerRadius::same(8);
            v
        } else {
            egui::Visuals::light()
        };
        cc.egui_ctx.set_visuals(visuals);

        Self {
            is_russian,
            settings_window: SettingsWindow::new(settings.clone()),
            show_settings: false,
            show_keyboard: false,
            show_about: false,
            supervisor,
            rt,
            tray_handle,
            tray_mode: false,
            shortcut: Some(shortcut),
            toggle_requested: toggle_req,
            show_window_requested: show_win_req,
            show_keyboard_requested: show_kb_req,
            show_settings_requested: show_settings_req,
            quit_requested: quit_req,
            status: None,
            startup_done: false,
            last_toggle: Instant::now() - Duration::from_secs(60),
            last_supervise: Instant::now(),
        }
    }

    fn set_status(&mut self, message: impl Into<String>, ok: bool) {
        self.status = Some((message.into(), ok, Instant::now()));
    }

    fn toggle_mode(&mut self) {
        let next = !self.is_russian.load(Ordering::Relaxed);
        info!("mode toggle -> {}", if next { "RU" } else { "EN" });
        self.is_russian.store(next, Ordering::Relaxed);
        self.apply_mode(next);
    }

    fn apply_mode(&mut self, russian: bool) {
        self.last_toggle = Instant::now();

        match self.supervisor.apply(russian) {
            Ok(()) => {
                if russian {
                    self.set_status("Russian mode is active — type phonetically", true);
                } else {
                    self.set_status("English layout active", true);
                }
            }
            Err(e) => {
                self.set_status(e, false);
                // Keep the visual state honest if the switch failed.
                self.is_russian.store(!russian, Ordering::Relaxed);
            }
        }

        self.sync_tray();
    }

    fn sync_tray(&mut self) {
        let mode = self.is_russian.load(Ordering::Relaxed);
        if mode == self.tray_mode {
            return;
        }
        self.tray_mode = mode;

        if let Some(handle) = self.tray_handle.as_ref() {
            self.rt.block_on(async {
                handle
                    .update(|tray: &mut TrayIcon| tray.set_mode(mode))
                    .await;
            });
        }
    }

    /// Once per start-up: make sure the engine process exists so it can register
    /// itself with `ibus-daemon`, then apply the configured default mode.
    fn startup(&mut self) {
        if self.startup_done {
            return;
        }
        self.startup_done = true;

        self.supervisor.kill_stale_engines();

        if !self.supervisor.ensure_running() {
            self.set_status(
                "Could not start russian-keyboard-engine — is it installed?",
                false,
            );
            return;
        }

        let want_russian = self.is_russian.load(Ordering::Relaxed);
        if want_russian {
            self.apply_mode(true);
        } else {
            self.sync_tray();
        }
    }

    /// Every few seconds: revive a dead engine and mirror external engine changes.
    fn supervise(&mut self) {
        if self.last_supervise.elapsed() < Duration::from_secs(3) {
            return;
        }
        self.last_supervise = Instant::now();

        // Mirror engine changes made elsewhere (IBus panel, Super+Space) first, so a
        // manual switch is never fought by the supervisor below.
        if self.last_toggle.elapsed() >= Duration::from_secs(4) {
            if let Some(current) = get_ibus_engine() {
                let actually_ru = current == ENGINE_NAME;
                if actually_ru != self.is_russian.load(Ordering::Relaxed) {
                    self.is_russian.store(actually_ru, Ordering::Relaxed);
                    self.sync_tray();
                }
            }
        }

        let is_ru = self.is_russian.load(Ordering::Relaxed);
        self.supervisor.revive_if_needed(is_ru);
    }

    fn quit(&mut self, ctx: &egui::Context) {
        if self.is_russian.load(Ordering::Relaxed) {
            let _ = self.supervisor.apply(false);
        }
        if let Some(shortcut) = self.shortcut.take() {
            shortcut.stop();
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn status_color(&self) -> Color32 {
        match &self.status {
            Some((_, true, _)) => ACTIVE_GREEN,
            Some((_, false, _)) => Color32::from_rgb(247, 118, 142),
            None => IDLE_GREY,
        }
    }

    fn pill(&mut self, ctx: &egui::Context) {
        let is_ru = self.is_russian.load(Ordering::Relaxed);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Russian").strong().size(16.0).color(ACCENT));
                    ui.add_space(4.0);

                    let (label, color) = if is_ru {
                        ("RU", ACTIVE_GREEN)
                    } else {
                        ("EN", IDLE_GREY)
                    };
                    let toggle = egui::Button::new(RichText::new(label).strong().color(color))
                        .min_size(Vec2::new(52.0, 26.0));
                    if ui
                        .add(toggle)
                        .on_hover_text("Toggle Russian transliteration (Meta + R)")
                        .clicked()
                    {
                        self.toggle_mode();
                    }

                    ui.separator();

                    if ui
                        .button("⌨ Layout")
                        .on_hover_text("Show the Russian phonetic key map")
                        .clicked()
                    {
                        self.show_keyboard = !self.show_keyboard;
                    }
                    if ui
                        .button("⚙ Settings")
                        .on_hover_text("Open the settings window")
                        .clicked()
                    {
                        self.show_settings = !self.show_settings;
                    }
                    if ui
                        .button("ℹ")
                        .on_hover_text("About and diagnostics")
                        .clicked()
                    {
                        self.show_about = !self.show_about;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("×")
                            .on_hover_text("Quit Russian Keyboard Tool")
                            .clicked()
                        {
                            self.quit(ctx);
                        }
                        if ui
                            .button("—")
                            .on_hover_text("Minimise to the task bar")
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }

                        let color = self.status_color();
                        let hover = self
                            .status
                            .as_ref()
                            .map(|(msg, _, _)| msg.clone())
                            .unwrap_or_else(|| "No messages".to_string());
                        ui.label(RichText::new("•").color(color))
                            .on_hover_text(hover);
                    });
                });

                ui.add_space(4.0);

                let (text, color) = match &self.status {
                    Some((msg, ok, at)) => {
                        let color = if *ok {
                            ACTIVE_GREEN
                        } else {
                            Color32::from_rgb(247, 118, 142)
                        };
                        let text = if at.elapsed() > Duration::from_secs(10) {
                            default_hint(is_ru).to_string()
                        } else {
                            msg.clone()
                        };
                        (text, color)
                    }
                    None => (default_hint(is_ru).to_string(), IDLE_GREY),
                };

                ui.label(RichText::new(text).size(11.5).color(color));
            });
    }

    /// A real, resizable OS window — not an `egui::Window` squeezed into the pill.
    fn child_viewport(
        ctx: &egui::Context,
        key: &'static str,
        title: &str,
        size: [f32; 2],
        min_size: [f32; 2],
        open: &mut bool,
        mut add_contents: impl FnMut(&mut egui::Ui),
    ) {
        let viewport_id = egui::ViewportId::from_hash_of(key);
        let builder = egui::ViewportBuilder::default()
            .with_title(title.to_owned())
            .with_inner_size(size)
            .with_min_inner_size(min_size)
            .with_resizable(true);

        let mut close_requested = false;

        ctx.show_viewport_immediate(viewport_id, builder, |ctx, _class| {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(egui::Margin::same(12)),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(key)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            add_contents(ui);
                        });
                });

            if ctx.input(|i| i.viewport().close_requested()) {
                close_requested = true;
            }
        });

        if close_requested {
            *open = false;
        }
    }

    fn windows(&mut self, ctx: &egui::Context) {
        if self.show_settings {
            let mut open = true;
            let settings_window = &mut self.settings_window;
            Self::child_viewport(
                ctx,
                "keyboard-settings",
                "Settings — Russian Keyboard Tool",
                [640.0, 720.0],
                [460.0, 320.0],
                &mut open,
                |ui| settings_window.show_contents(ui),
            );
            self.show_settings = open;
        }

        if self.show_keyboard {
            let mut open = true;
            let is_ru = self.is_russian.load(Ordering::Relaxed);
            Self::child_viewport(
                ctx,
                "keyboard-layout",
                "Virtual Keyboard — Russian Keyboard Tool",
                [820.0, 620.0],
                [420.0, 300.0],
                &mut open,
                |ui| VirtualKeyboard::show(ui, is_ru),
            );
            self.show_keyboard = open;
        }

        if self.show_about {
            let mut open = true;
            let is_ru = self.is_russian.load(Ordering::Relaxed);
            let engine_running = self.supervisor.is_running();
            let current_engine = get_ibus_engine().unwrap_or_else(|| "none set".to_string());
            let engine_binary = EngineSupervisor::engine_binary()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "not found".to_string());
            let tray = if self.tray_handle.is_some() {
                "registered"
            } else {
                "unavailable"
            };

            Self::child_viewport(
                ctx,
                "keyboard-about",
                "About — Russian Keyboard Tool",
                [520.0, 440.0],
                [400.0, 300.0],
                &mut open,
                |ui| {
                    ui.heading(RichText::new("Russian Keyboard Tool").size(20.0).strong());
                    ui.label(RichText::new("Version 1.0.0 · MIT License").color(IDLE_GREY));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("by").color(IDLE_GREY));
                        ui.hyperlink_to("Xexan Terxcin", "https://github.com/XexanTerxcin");
                    });
                    ui.add_space(10.0);
                    ui.label(
                        "A system-wide Russian phonetic transliteration keyboard for Linux, \
                         inspired by the workflow of OpenBangla Keyboard.",
                    );
                    ui.add_space(12.0);
                    ui.label(RichText::new("How to use").strong());
                    ui.label("1. Press Meta + R anywhere to switch between English and Russian.");
                    ui.label("2. Type the word the way it sounds: privet » привет.");
                    ui.label("3. Press space to commit the word (suggestions appear above it).");

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.label(RichText::new("Diagnostics").strong());

                    egui::Grid::new("diagnostics")
                        .num_columns(2)
                        .spacing([14.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Mode");
                            ui.label(if is_ru {
                                "Russian (RU)"
                            } else {
                                "English (EN)"
                            });
                            ui.end_row();

                            ui.label("IBus global engine");
                            ui.label(current_engine.as_str());
                            ui.end_row();

                            ui.label("Engine process");
                            ui.label(if engine_running {
                                "running"
                            } else {
                                "not started"
                            });
                            ui.end_row();

                            ui.label("Engine binary");
                            ui.label(engine_binary.as_str());
                            ui.end_row();

                            ui.label("Tray icon");
                            ui.label(tray);
                            ui.end_row();

                            ui.label("Settings file");
                            ui.label(AppSettings::config_path().display().to_string());
                            ui.end_row();
                        });
                },
            );
            self.show_about = open;
        }
    }
}

fn default_hint(is_russian: bool) -> &'static str {
    if is_russian {
        "Russian mode: type phonetically — space commits the word."
    } else {
        "English mode: press Meta + R to start typing Russian."
    }
}

impl eframe::App for RussianKeyboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keeps the pill responsive to the tray/shortcut threads.
        ctx.request_repaint_after(Duration::from_millis(200));

        self.startup();

        if self.quit_requested.swap(false, Ordering::Relaxed) {
            self.quit(ctx);
            return;
        }

        if self.show_window_requested.swap(false, Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if self.show_keyboard_requested.swap(false, Ordering::Relaxed) {
            self.show_keyboard = true;
        }
        if self.show_settings_requested.swap(false, Ordering::Relaxed) {
            self.show_settings = true;
        }
        if self.toggle_requested.swap(false, Ordering::Relaxed) {
            self.toggle_mode();
        }

        self.supervise();

        self.pill(ctx);
        self.windows(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(shortcut) = self.shortcut.take() {
            shortcut.stop();
        }
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init();
    info!("Starting Russian Keyboard Tool GUI");

    let settings = AppSettings::load();
    let start_hidden = settings.start_minimized && settings.show_tray_icon;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Russian Keyboard Tool")
            .with_inner_size(PILL_SIZE)
            .with_min_inner_size(PILL_SIZE)
            .with_resizable(false)
            .with_visible(!start_hidden)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "Russian Keyboard Tool",
        options,
        Box::new(|cc| Ok(Box::new(RussianKeyboardApp::new(cc)))),
    )
}
