use crate::config::AppSettings;
use crate::transliteration::default_mappings;
use eframe::egui::{self, Color32, RichText};

pub struct SettingsWindow {
    pub settings: AppSettings,
    new_pattern: String,
    new_replacement: String,
    saved_notice: bool,
}

impl SettingsWindow {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            settings,
            new_pattern: String::new(),
            new_replacement: String::new(),
            saved_notice: false,
        }
    }

    /// Draw the settings content. The caller owns the window itself (a native
    /// viewport), so this never creates a nested `egui::Window`.
    pub fn show_contents(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("General").size(17.0).strong());
        ui.add_space(4.0);
        ui.checkbox(&mut self.settings.launch_on_login, "Launch on system login");
        ui.checkbox(
            &mut self.settings.start_minimized,
            "Start minimized to tray",
        );
        ui.checkbox(&mut self.settings.show_tray_icon, "Show system tray icon");
        ui.checkbox(&mut self.settings.dark_theme, "Use dark theme");

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);

        ui.heading(RichText::new("Keyboard & Shortcuts").size(17.0).strong());
        ui.add_space(4.0);
        egui::Grid::new("shortcut_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Global activation shortcut");
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut self.settings.global_shortcut)
                        .desired_width(180.0),
                );
                ui.end_row();

                ui.label("Default input mode");
                egui::ComboBox::from_id_salt("default_mode")
                    .selected_text(if self.settings.default_input_mode == "RU" {
                        "Russian (RU)"
                    } else {
                        "English (EN)"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.settings.default_input_mode,
                            "EN".to_string(),
                            "English (EN)",
                        );
                        ui.selectable_value(
                            &mut self.settings.default_input_mode,
                            "RU".to_string(),
                            "Russian (RU)",
                        );
                    });
                ui.end_row();
            });

        ui.add_space(6.0);
        ui.label(
            RichText::new("Press Meta + R anywhere to toggle between English and Russian.")
                .size(11.0)
                .color(Color32::GRAY),
        );
        ui.label(
            RichText::new(
                "The global shortcut is fixed to Meta + R in this release. \
                 Some options take effect after the app restarts or Russian mode is re-activated.",
            )
            .size(11.0)
            .color(Color32::GRAY),
        );

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);

        ui.heading(
            RichText::new("Transliteration & Suggestions")
                .size(17.0)
                .strong(),
        );
        ui.add_space(4.0);
        ui.checkbox(
            &mut self.settings.enable_suggestions,
            "Enable word suggestions & predictive text",
        );
        ui.checkbox(
            &mut self.settings.auto_capitalization,
            "Match capitalisation of what you type",
        );

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);

        ui.heading(
            RichText::new("Custom Transliteration Mappings")
                .size(17.0)
                .strong(),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Pattern");
            ui.add(
                egui::TextEdit::singleline(&mut self.new_pattern)
                    .desired_width(90.0)
                    .hint_text("shch"),
            );
            ui.label("»");
            ui.add(
                egui::TextEdit::singleline(&mut self.new_replacement)
                    .desired_width(90.0)
                    .hint_text("щ"),
            );
            if ui.button("Add").clicked()
                && !self.new_pattern.trim().is_empty()
                && !self.new_replacement.trim().is_empty()
            {
                self.settings
                    .custom_mappings
                    .push(crate::transliteration::MappingRule {
                        pattern: self.new_pattern.trim().to_string(),
                        replacement: self.new_replacement.trim().to_string(),
                    });
                self.new_pattern.clear();
                self.new_replacement.clear();
                self.saved_notice = false;
            }
        });

        ui.add_space(6.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("custom_mappings")
                .max_height(150.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let mut to_remove = None;
                    for (i, rule) in self.settings.custom_mappings.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&rule.pattern).monospace());
                            ui.label("»");
                            ui.label(RichText::new(&rule.replacement).strong());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("×").clicked() {
                                        to_remove = Some(i);
                                    }
                                },
                            );
                        });
                    }
                    if let Some(i) = to_remove {
                        self.settings.custom_mappings.remove(i);
                        self.saved_notice = false;
                    }
                    if self.settings.custom_mappings.is_empty() {
                        ui.label(
                            RichText::new("No custom mappings added yet.")
                                .italics()
                                .color(Color32::GRAY),
                        );
                    }
                });
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(RichText::new("Save settings").strong()).clicked() {
                match self.settings.save() {
                    Ok(()) => self.saved_notice = true,
                    Err(e) => {
                        self.saved_notice = false;
                        log::warn!("failed to save settings: {e}");
                    }
                }
            }
            if ui.button("Reset mappings").clicked() {
                self.settings.custom_mappings = default_mappings();
                self.saved_notice = false;
            }
            if self.saved_notice {
                ui.label(RichText::new("✓ Saved").color(Color32::GREEN));
            }
        });

        ui.add_space(10.0);
        ui.label(
            RichText::new(format!(
                "Settings file: {}",
                AppSettings::config_path().display()
            ))
            .size(10.0)
            .color(Color32::GRAY),
        );
    }
}
