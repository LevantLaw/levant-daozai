//! 专注设置：番茄钟与休息时长。

use crate::island::settings::{setting_number, settings_card, settings_hint};
use crate::island::App;
use eframe::egui;

impl App {
    pub(crate) fn focus_settings(&mut self, ui:&mut egui::Ui) {
        let on=self.settings_feature_header(ui,crate::island::FOCUS_PAGE,"番茄钟与休息计时");
        ui.add_enabled_ui(on,|ui|{
            settings_card(ui,|ui|{
                setting_number(ui,"专注",&mut self.settings.focus_minutes,1,180," 分钟",1.0);
                ui.separator();
                setting_number(ui,"休息",&mut self.settings.rest_minutes,1,60," 分钟",1.0);
                settings_hint(ui,"调整后从下一阶段开始生效。");
            });
        });
    }
}
