use eframe::egui::{self, Color32, RichText, Sense, Stroke, Vec2};

pub struct VirtualKeyboard;

const KEY_GAP: f32 = 4.0;

impl VirtualKeyboard {
    pub fn show(ui: &mut egui::Ui, is_russian: bool) {
        ui.heading(
            RichText::new("Russian Keyboard Phonetic Layout")
                .size(18.0)
                .strong(),
        );
        ui.label(
            RichText::new(
                "Type with the English QWERTY keys to get the matching Cyrillic letters.",
            )
            .color(Color32::GRAY),
        );
        ui.add_space(10.0);

        // Scale the keys so the whole board always fits the window width.
        let columns = 13.0;
        let avail = ui.available_width().max(320.0);
        let key_w = ((avail - KEY_GAP * (columns - 1.0) - 8.0) / columns).clamp(24.0, 48.0);
        let key_h = (key_w * 1.1).clamp(28.0, 46.0);

        let row1 = [
            ("`", "~", "ё", "Ё"),
            ("1", "!", "1", "!"),
            ("2", "@", "2", "@"),
            ("3", "#", "3", "#"),
            ("4", "$", "4", "$"),
            ("5", "%", "5", "%"),
            ("6", "^", "6", "^"),
            ("7", "&", "7", "&"),
            ("8", "*", "8", "*"),
            ("9", "(", "9", "("),
            ("0", ")", "0", ")"),
            ("-", "_", "-", "_"),
            ("=", "+", "=", "+"),
        ];

        let row2 = [
            ("q", "Q", "я", "Я"),
            ("w", "W", "в", "В"),
            ("e", "E", "е", "Е"),
            ("r", "R", "р", "Р"),
            ("t", "T", "т", "Т"),
            ("y", "Y", "ы", "Ы"),
            ("u", "U", "у", "У"),
            ("i", "I", "и", "И"),
            ("o", "O", "о", "О"),
            ("p", "P", "п", "П"),
            ("[", "{", "х", "Х"),
            ("]", "}", "ъ", "Ъ"),
        ];

        let row3 = [
            ("a", "A", "а", "А"),
            ("s", "S", "с", "С"),
            ("d", "D", "д", "Д"),
            ("f", "F", "ф", "Ф"),
            ("g", "G", "г", "Г"),
            ("h", "H", "х", "Х"),
            ("j", "J", "й", "Й"),
            ("k", "K", "к", "К"),
            ("l", "L", "л", "Л"),
            (";", ":", "ж", "Ж"),
            ("'", "\"", "ь", "Э"),
        ];

        let row4 = [
            ("z", "Z", "з", "З"),
            ("x", "X", "х", "Х"),
            ("c", "C", "ц", "Ц"),
            ("v", "V", "в", "В"),
            ("b", "B", "б", "Б"),
            ("n", "N", "н", "Н"),
            ("m", "M", "м", "М"),
            (",", "<", "б", "<"),
            (".", ">", "ю", ">"),
        ];

        for row in [&row1[..], &row2[..], &row3[..], &row4[..]] {
            Self::draw_row(ui, row, is_russian, key_w, key_h);
        }

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);

        ui.heading(
            RichText::new("Multi-character combinations")
                .size(15.0)
                .strong(),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "These sequences are resolved while you type (press space to commit the word).",
            )
            .size(11.0)
            .color(Color32::GRAY),
        );
        ui.add_space(6.0);

        egui::Grid::new("multi_combos")
            .num_columns(4)
            .spacing([18.0, 8.0])
            .show(ui, |ui| {
                let combos = [
                    ("shch", "щ"),
                    ("sch", "щ"),
                    ("sh", "ш"),
                    ("zh", "ж"),
                    ("kh", "х"),
                    ("ts", "ц"),
                    ("ch", "ч"),
                    ("yo", "ё"),
                    ("yu", "ю"),
                    ("ya", "я"),
                    ("ye", "э"),
                ];

                for (i, (eng, rus)) in combos.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(*eng)
                                .monospace()
                                .strong()
                                .color(Color32::from_rgb(122, 162, 247)),
                        );
                        ui.label(RichText::new("»").color(Color32::GRAY));
                        ui.label(
                            RichText::new(*rus)
                                .strong()
                                .size(15.0)
                                .color(Color32::from_rgb(158, 206, 106)),
                        );
                    });
                    if (i + 1) % 4 == 0 {
                        ui.end_row();
                    }
                }
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);
        ui.heading(RichText::new("Examples").size(15.0).strong());
        ui.add_space(4.0);
        for (eng, rus) in [
            ("privet", "привет"),
            ("khorosho", "хорошо"),
            ("zhizn", "жизнь"),
            ("lyubov", "любовь"),
            ("shchuka", "щука"),
        ] {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(eng)
                        .monospace()
                        .color(Color32::from_rgb(140, 150, 170)),
                );
                ui.label(RichText::new("»").color(Color32::GRAY));
                ui.label(RichText::new(rus).strong());
            });
        }
    }

    fn draw_row(
        ui: &mut egui::Ui,
        keys: &[(&str, &str, &str, &str)],
        is_russian: bool,
        key_w: f32,
        key_h: f32,
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = KEY_GAP;
            for &(en_l, _en_u, ru_l, _ru_u) in keys {
                let (rect, _response) =
                    ui.allocate_exact_size(Vec2::new(key_w, key_h), Sense::hover());

                let fill = if is_russian {
                    Color32::from_rgb(45, 52, 71)
                } else {
                    Color32::from_rgb(35, 38, 50)
                };

                ui.painter().rect(
                    rect,
                    4.0,
                    fill,
                    Stroke::new(1.0_f32, Color32::from_rgb(70, 75, 95)),
                    egui::StrokeKind::Inside,
                );

                // English key in the top-left corner.
                ui.painter().text(
                    rect.min + Vec2::new(4.0, 3.0),
                    egui::Align2::LEFT_TOP,
                    en_l,
                    egui::FontId::proportional((key_w * 0.3).max(9.0)),
                    Color32::from_rgb(140, 150, 170),
                );

                // Cyrillic key in the bottom-right corner.
                ui.painter().text(
                    rect.max - Vec2::new(5.0, 3.0),
                    egui::Align2::RIGHT_BOTTOM,
                    ru_l,
                    egui::FontId::proportional((key_w * 0.38).max(11.0)),
                    if is_russian {
                        Color32::from_rgb(122, 162, 247)
                    } else {
                        Color32::WHITE
                    },
                );
            }
        });
    }
}
