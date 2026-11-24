//! 剪贴板设置：记录开关与本机保存说明。

use crate::island::settings::{setting_row, settings_card, settings_hint};
use crate::island::App;
use eframe::egui;

impl App {
    pub(crate) fn clipboard_settings(&mut self, ui:&mut egui::Ui) {
        let on=self.settings_feature_header(ui,crate::island::CLIPBOARD_PAGE,"记录最近复制的文字，点一下再次复制");
        ui.add_enabled_ui(on,|ui|{
            settings_card(ui,|ui|{
                setting_row(ui,"记录剪贴板","在剪贴板页查看，点一下再次复制",&mut self.settings.clipboard_history);
                settings_hint(ui,&format!("本机保存最近 {} 条；密码管理器标记为敏感的内容不会记录。",crate::clipboard::MAX_RECENT));
                if ui.button("打开剪贴板").clicked(){self.settings_open=false;self.show_page(crate::island::CLIPBOARD_PAGE);}
            });
        });
    }
}
