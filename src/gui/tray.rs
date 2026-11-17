use ksni::menu::*;
use ksni::{Icon, MenuItem, ToolTip, Tray};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

type Flag = Arc<AtomicBool>;

const TRAY_ICON_PNG: &[u8] = include_bytes!("../../resources/icons/icon-4.png");

fn tray_icons() -> Vec<Icon> {
    static ICONS: std::sync::OnceLock<Vec<Icon>> = std::sync::OnceLock::new();

    ICONS
        .get_or_init(|| {
            let Ok(image) =
                image::load_from_memory_with_format(TRAY_ICON_PNG, image::ImageFormat::Png)
            else {
                return Vec::new();
            };

            let rgba = image.to_rgba8();
            let width = rgba.width() as i32;
            let height = rgba.height() as i32;

            // StatusNotifierItem wants ARGB32 in network byte order.
            let mut data = Vec::with_capacity((width * height * 4) as usize);
            for pixel in rgba.pixels() {
                data.push(pixel[3]);
                data.push(pixel[0]);
                data.push(pixel[1]);
                data.push(pixel[2]);
            }

            vec![Icon {
                width,
                height,
                data,
            }]
        })
        .clone()
}

pub struct TrayIcon {
    pub is_russian: Arc<AtomicBool>,
    pub toggle_requested: Arc<AtomicBool>,
    pub show_window_requested: Arc<AtomicBool>,
    pub show_keyboard_requested: Arc<AtomicBool>,
    pub show_settings_requested: Arc<AtomicBool>,
    pub quit_requested: Arc<AtomicBool>,
}

impl TrayIcon {
    pub fn new(is_russian: Flag) -> (Self, Flag, Flag, Flag, Flag, Flag) {
        let toggle = Arc::new(AtomicBool::new(false));
        let show_win = Arc::new(AtomicBool::new(false));
        let show_kb = Arc::new(AtomicBool::new(false));
        let show_settings = Arc::new(AtomicBool::new(false));
        let quit = Arc::new(AtomicBool::new(false));

        let tray = Self {
            is_russian,
            toggle_requested: toggle.clone(),
            show_window_requested: show_win.clone(),
            show_keyboard_requested: show_kb.clone(),
            show_settings_requested: show_settings.clone(),
            quit_requested: quit.clone(),
        };

        (tray, toggle, show_win, show_kb, show_settings, quit)
    }

    /// Keep the tray in sync after the mode changed from the GUI or the shortcut.
    pub fn set_mode(&mut self, russian: bool) {
        self.is_russian.store(russian, Ordering::Relaxed);
    }
}

impl Tray for TrayIcon {
    fn id(&self) -> String {
        "russian-keyboard-tool".to_string()
    }

    fn title(&self) -> String {
        let mode = if self.is_russian.load(Ordering::Relaxed) {
            "RU"
        } else {
            "EN"
        };
        format!("Russian Keyboard [{mode}]")
    }

    fn tool_tip(&self) -> ToolTip {
        let mode = if self.is_russian.load(Ordering::Relaxed) {
            "Russian (Phonetic)"
        } else {
            "English (QWERTY)"
        };

        ToolTip {
            title: "Russian Keyboard Tool".to_string(),
            description: format!("Active mode: {mode}\nShortcut: Meta + R"),
            icon_name: "russian-keyboard-tool".to_string(),
            icon_pixmap: tray_icons(),
        }
    }

    fn icon_name(&self) -> String {
        "russian-keyboard-tool".to_string()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        tray_icons()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let is_ru = self.is_russian.load(Ordering::Relaxed);
        let mode_label = if is_ru {
            "Switch to English [EN]"
        } else {
            "Switch to Russian [RU]"
        };

        vec![
            StandardItem {
                label: mode_label.to_string(),
                activate: Box::new(|this: &mut Self| {
                    this.toggle_requested.store(true, Ordering::Relaxed);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Show TopBar".to_string(),
                activate: Box::new(|this: &mut Self| {
                    this.show_window_requested.store(true, Ordering::Relaxed);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Virtual Keyboard".to_string(),
                activate: Box::new(|this: &mut Self| {
                    this.show_keyboard_requested.store(true, Ordering::Relaxed);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Settings…".to_string(),
                activate: Box::new(|this: &mut Self| {
                    this.show_settings_requested.store(true, Ordering::Relaxed);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".to_string(),
                activate: Box::new(|this: &mut Self| {
                    this.quit_requested.store(true, Ordering::Relaxed);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
