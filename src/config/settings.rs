use crate::transliteration::MappingRule;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub launch_on_login: bool,
    pub start_minimized: bool,
    pub show_tray_icon: bool,
    pub default_input_mode: String,
    pub global_shortcut: String,
    pub enable_suggestions: bool,
    pub auto_capitalization: bool,
    pub dark_theme: bool,
    pub custom_mappings: Vec<MappingRule>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            launch_on_login: false,
            start_minimized: false,
            show_tray_icon: true,
            default_input_mode: "EN".to_string(),
            global_shortcut: "Meta+R".to_string(),
            enable_suggestions: true,
            auto_capitalization: true,
            dark_theme: true,
            custom_mappings: Vec::new(),
        }
    }
}

impl AppSettings {
    pub fn config_path() -> PathBuf {
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".config")
            });
        base.join("russian-keyboard-tool").join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&data) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, data)?;
        self.sync_autostart()?;
        Ok(())
    }

    /// Create or remove `~/.config/autostart/russian-keyboard-tool.desktop` so the
    /// "Launch on system login" checkbox actually does something.
    fn sync_autostart(&self) -> std::io::Result<()> {
        let config_home = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".config")
            });
        let autostart_dir = config_home.join("autostart");
        let entry = autostart_dir.join("russian-keyboard-tool.desktop");

        if !self.launch_on_login {
            if entry.exists() {
                fs::remove_file(entry)?;
            }
            return Ok(());
        }

        let binary = PathBuf::from("/usr/bin/russian-keyboard-tool");
        let binary = if binary.is_file() {
            binary
        } else {
            std::env::current_exe()?
        };

        let contents = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Russian Keyboard Tool\n\
             Comment=Russian phonetic transliteration keyboard\n\
             Exec={}\n\
             Icon=russian-keyboard-tool\n\
             Terminal=false\n\
             X-GNOME-Autostart-enabled=true\n",
            binary.display()
        );

        fs::create_dir_all(autostart_dir)?;
        fs::write(entry, contents)
    }
}
