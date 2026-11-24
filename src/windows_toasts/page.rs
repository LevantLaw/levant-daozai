//! Windows 通知页：记录列表、清空与滚动。

use crate::island::banner::app_icon_badge;
use crate::island::App;
use crate::visuals::paint::{Card, H};
use crate::visuals::theme::{self, W};
use eframe::egui::{self, Color32};

impl App {
    /// 应用图标纹理：首次用到时从开始菜单取，失败也缓存，避免每帧重试。
    fn app_icon(&mut self,ctx:&egui::Context,id:&str)->Option<egui::TextureHandle>{
        if let Some(icon)=self.app_icons.get(id){return icon.clone();}
        if self.app_icons.len()>=32{self.app_icons.clear();}
        let icon=crate::os::icons::app_icon(id).map(|image|ctx.load_texture(format!("app-icon:{id}"),image,egui::TextureOptions::LINEAR));
        self.app_icons.insert(id.to_string(),icon.clone());
        icon
    }
    /// Windows 通知页：即时通讯以外的系统通知汇总在这里，每行带应用图标。
    pub(crate) fn page_windows(&mut self,c:&Card) {
        const ROW_H:f32=33.0;
        const ROW_GAP:f32=5.0;
        const LIST_TOP:f32=56.0;
        let count=self.windows_notices.len();
        let header=if count==0 {"暂无通知".to_string()} else {format!("{count} 条通知")};
        c.text(15.0,45.0,&header,10.5,W::Regular,theme::GRAY,H::Left);
        if count==0 {
            c.text(209.5,84.0,"还没有收到 Windows 通知",13.0,W::Bold,theme::WHITE,H::Center);
            c.text(209.5,102.0,"连接系统通知后，即时通讯以外的通知会汇总在这里",10.0,W::Regular,theme::GRAY,H::Center);
            return;
        }
        let (clear,body)=crate::visuals::paint::capsule(c,337.0,33.0,52.0,20.0,"windows-clear",theme::BTN,theme::BTN_HOVER);
        body.text(363.0,47.0,"清空",10.0,W::Semibold,theme::WHITE,H::Center);
        if clear.clicked(){self.windows_notices.clear();self.windows_first=0;return;}
        let max=count.saturating_sub(crate::island::WINDOWS_ROWS);
        self.windows_first=self.windows_first.min(max);
        let first=self.windows_first;
        let rows:Vec<_>=self.windows_notices.iter().skip(first).take(crate::island::WINDOWS_ROWS)
            .map(|notice|(notice.app.clone(),notice.id.clone(),notice.body.clone(),notice.time.clone())).collect();
        let ctx=c.ui.ctx();
        let icons:Vec<_>=rows.iter().map(|(_,id,_,_)|self.app_icon(ctx,id)).collect();
        for (slot,(app,_,body,time)) in rows.iter().enumerate() {
            let top=LIST_TOP+slot as f32*(ROW_H+ROW_GAP);
            c.fill(40.0,top,320.0,ROW_H,10.0,Color32::from_white_alpha(8));
            app_icon_badge(c,52.0,top+ROW_H/2.0,20.0,app,icons[slot].as_ref());
            c.text(68.0,top+14.0,&c.fit(app,11.5,W::Semibold,230.0),11.5,W::Semibold,theme::WHITE,H::Left);
            c.text(352.0,top+14.0,time,9.5,W::Regular,theme::GRAY,H::Right);
            c.text(68.0,top+27.0,&c.fit(body,9.5,W::Regular,280.0),9.5,W::Regular,theme::GRAY,H::Left);
        }
        if max>0 {
            let track_x=367.0;
            let viewport=crate::island::WINDOWS_ROWS as f32*ROW_H+(crate::island::WINDOWS_ROWS-1) as f32*ROW_GAP;
            let thumb_h=((crate::island::WINDOWS_ROWS as f32/count as f32)*viewport).max(18.0);
            let thumb_y=LIST_TOP+first as f32/max as f32*(viewport-thumb_h);
            c.fill(track_x,LIST_TOP,3.0,viewport,1.5,Color32::from_white_alpha(14));
            c.fill(track_x,thumb_y,3.0,thumb_h,1.5,Color32::from_white_alpha(64));
        }
    }
}
