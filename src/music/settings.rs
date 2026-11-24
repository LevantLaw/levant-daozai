//! 音乐设置页：歌词显示与时间偏移。

use crate::island::settings::{setting_number, setting_row, settings_card, settings_hint, settings_label};
use crate::island::App;
use eframe::egui;

impl App {
    pub(crate) fn music_settings(&mut self, ui: &mut egui::Ui) {
        let on = self.settings_feature_header(ui, crate::island::MUSIC_PAGE, "播放控制与同步歌词");
        ui.add_enabled_ui(on, |ui| {
            settings_label(ui, "歌词");
            settings_card(ui, |ui| {
                setting_row(ui, "岛上歌词", "收起时显示当前歌词；无同步歌词时显示封面和歌名", &mut self.settings.island_lyrics);
                ui.separator();
                setting_row(ui, "展开时显示歌词", "在播放器右侧同步滚动", &mut self.settings.lyrics_enabled);
                self.show_lyrics = self.settings.lyrics_enabled;
                ui.separator();
                let mut advance = -self.settings.lyric_delay_ms as f64 / 1000.0;
                if setting_number(ui, "时间偏移", &mut advance, -5.0, 5.0, " 秒", 0.1) {
                    self.settings.lyric_delay_ms = (-advance * 1000.0).round() as i32;
                }
                settings_hint(ui, "歌词和歌声对不上时调整：正数提前，负数延后；默认提前 0.35 秒。");
            });
        });
    }
}
