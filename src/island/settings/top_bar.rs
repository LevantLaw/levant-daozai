//! 顶部任务栏设置：不处理、下移避开，或保持在任务栏上方。
use super::*;
use crate::model::TopBarMode;

impl App {
    pub(crate) fn top_bar_settings(&mut self, ui:&mut egui::Ui) {
        let reserved=crate::os::display_layout::selected(&self.displays,&self.settings.display_id).map_or(0,|d|{
            let b=crate::os::display_layout::placement_bounds(d,crate::model::Dock::Top,true);
            (b.top-d.bounds.top).max(0)
        });
        let detected=reserved>0;
        let mut mode=self.settings.top_bar_mode();
        let mut changed=false;
        settings_control_row_right(ui,"显示方式",|ui|{
            changed=settings_segmented(ui,&mut mode,&[
                (TopBarMode::Off,"不处理"),
                (TopBarMode::Avoid,"下移"),
                (TopBarMode::OnTop,"岛仔在上"),
            ],84.0);
        });
        if changed {self.settings.set_top_bar_mode(mode);}
        ui.horizontal(|ui|{
            settings_chip(ui,if detected{"已检测到"}else{"未检测到"},detected);
            let hint=match (detected,mode) {
                (false,_)=>"未检测到任务栏或其他系统栏，三档当前效果相同。".to_owned(),
                (true,TopBarMode::Off)=>"岛仔停在屏幕顶部，可能被任务栏或其他系统栏挡住。".to_owned(),
                (true,TopBarMode::Avoid)=>format!("岛仔下移到任务栏或其他系统栏下方（{reserved} 像素）。"),
                (true,TopBarMode::OnTop)=>"任务栏或其他系统栏挡住岛仔时，自动抬回最上层。".to_owned(),
            };
            settings_hint(ui,&hint);
        });
    }
}
