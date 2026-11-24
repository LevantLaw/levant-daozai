//! 待办设置：功能开关。

use crate::island::App;
use eframe::egui;

impl App {
    pub(crate) fn todo_settings(&mut self, ui:&mut egui::Ui) {
        let on=self.settings_feature_header(ui,crate::island::TODO_PAGE,"任务清单与到期提醒");
        let _=on;
    }
}
