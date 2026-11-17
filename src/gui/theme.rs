use eframe::egui::{Color32, Stroke};

pub struct ThemeColors {
    pub bg: Color32,
    pub panel_bg: Color32,
    pub accent_active: Color32,
    pub accent_inactive: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub border: Stroke,
}

impl ThemeColors {
    pub fn dark() -> Self {
        Self {
            bg: Color32::from_rgb(26, 27, 38),
            panel_bg: Color32::from_rgb(36, 40, 59),
            accent_active: Color32::from_rgb(122, 162, 247), // Russian Blue / Cyan
            accent_inactive: Color32::from_rgb(65, 72, 104),
            text_primary: Color32::from_rgb(192, 202, 245),
            text_muted: Color32::from_rgb(86, 95, 137),
            border: Stroke::new(1.0_f32, Color32::from_rgb(41, 46, 66)),
        }
    }

    pub fn light() -> Self {
        Self {
            bg: Color32::from_rgb(245, 246, 250),
            panel_bg: Color32::from_rgb(255, 255, 255),
            accent_active: Color32::from_rgb(67, 97, 238),
            accent_inactive: Color32::from_rgb(220, 224, 230),
            text_primary: Color32::from_rgb(33, 37, 41),
            text_muted: Color32::from_rgb(108, 117, 125),
            border: Stroke::new(1.0_f32, Color32::from_rgb(206, 212, 218)),
        }
    }
}
