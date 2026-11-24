//! 置物架设置：条目数与本机保存说明。

use crate::island::settings::settings_card;
use crate::island::settings_theme::color as settings_color;
use crate::island::App;
use eframe::egui::{self, Color32};

impl App {
    pub(crate) fn shelf_settings(&mut self, ui:&mut egui::Ui) {
        let on=self.settings_feature_header(ui,crate::island::SHELF_PAGE,"临时收纳文件位置");
        ui.add_enabled_ui(on,|ui|{
            settings_card(ui,|ui|{
                ui.label(egui::RichText::new(format!("当前 {} 项 · 移除条目不会删除原文件",self.shelf.items.len())).size(12.0).color(settings_color(ui.ctx(),Color32::from_gray(180),Color32::from_gray(75))));
            });
        });
    }
}
