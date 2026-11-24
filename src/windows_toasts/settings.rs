//! Windows 通知设置：本次运行的记录条数与清空。

use crate::island::settings::{settings_action_row, settings_card, settings_danger_button};
use crate::island::settings_theme::color as settings_color;
use crate::island::App;
use eframe::egui::{self, Color32};

impl App {
    pub(crate) fn windows_settings(&mut self, ui:&mut egui::Ui) {
        let on=self.settings_feature_header(ui,crate::island::WINDOWS_PAGE,"即时通讯以外的系统通知汇总，横幅结束后保留铃铛");
        ui.add_space(4.0);
        ui.add_enabled_ui(on,|ui|{
            settings_card(ui,|ui|{
                let ctx=&ui.ctx().clone();
                ui.label(egui::RichText::new(format!("本次运行已记录 {} 条通知",self.windows_notices.len())).size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                settings_action_row(ui,|ui|{
                    if ui.add(settings_danger_button("清空记录")).clicked(){self.windows_notices.clear();self.windows_first=0;}
                });
            });
        });
    }
}
