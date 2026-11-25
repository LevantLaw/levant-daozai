//! 岛仔表情、岛内设置页与公共控件。
use crate::island::{App, Cmd, Phase};

pub(crate) use crate::island::settings::{
    setting_row, setting_row_nested, settings_action_row, settings_apply_style,
    settings_card, settings_control_row, settings_control_row_right, settings_danger_button,
    settings_group_fold, settings_group_fold_switch, settings_hint, settings_label, settings_modal_frame, settings_modal_title,
    settings_primary_button, settings_segmented,
};
#[cfg(test)]
pub(crate) use crate::island::settings::{
    island_size_slider, number_stepper, setting_number, settings_feature_rows,
    settings_tab_group_for_tests, settings_tab_module, SETTINGS_GROUPS, SETTINGS_SECTIONS,
    SETTINGS_TAB_APPEARANCE, SETTINGS_TAB_BEHAVIOR, SETTINGS_TAB_BODY, SETTINGS_TAB_CLIPBOARD,
    SETTINGS_TAB_PAGES, SETTINGS_TAB_REMINDERS, SETTINGS_TAB_WINDOWS,
};
use crate::visuals::paint::{Card, H};
use crate::visuals::theme::{self, W};
use eframe::egui::{self, Color32, Rect};


pub(crate) use crate::island::pet::PetState;

impl App {
    pub(crate) fn pet_state(&self) -> PetState {
        if let Some((state, _)) = self.pet.reaction {
            state
        } else if self.pet.waking_until > self.t {
            PetState::Waking
        } else if self.pomo.running && self.pomo.phase == Phase::Focus {
            PetState::Focused
        } else if self.pet.worried {
            PetState::Worried
        } else if self.media.playing && self.media.is_music() {
            if self.notice.is_some() || self.hud_notice.is_some() {PetState::Awake}else{PetState::Music}
        } else if self.pet.hovered {
            PetState::Curious
        } else if self.idle_for() >= 180.0 {
            PetState::Sleeping
        } else if self.idle_for() >= self.opts.pet_idle {
            PetState::Dozing
        } else {
            PetState::Awake
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn island_size_drag_commits_only_on_release(){
        let ctx=egui::Context::default();let mut value=100;
        let pos=egui::pos2(30.0,30.0);let end=egui::pos2(110.0,30.0);
        let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
        let frames=[vec![],vec![egui::Event::PointerMoved(pos)],vec![button(pos,true)],vec![egui::Event::PointerMoved(end)],vec![button(end,false)]];
        for (i,events) in frames.into_iter().enumerate(){
            let _=ctx.run(egui::RawInput{events,screen_rect:Some(Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(320.0,80.0))),..Default::default()},|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(20)).show(ctx,|ui|{ui.spacing_mut().slider_width=180.0;island_size_slider(ui,&mut value);});
            });
            if i<4{assert_eq!(value,100);}else{assert_ne!(value,100);}
        }
    }
    #[test]
    fn number_stepper_clicks_follow_steps_and_limits() {
        for (initial,step,x,expected) in [(5u32,1,127.0,6),(5,1,13.0,4),(0,1,13.0,0),(10,1,127.0,10),(9,3,127.0,10),(1,3,13.0,0)] {
            let ctx=egui::Context::default();
            let mut value=initial;
            let pos=egui::pos2(x+20.0,34.0);
            let events=[vec![],vec![egui::Event::PointerMoved(pos)],vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}],vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]];
            for events in events {
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(200.0,80.0))),events,..Default::default()};
                let _=ctx.run(raw,|ctx|{
                    egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(20)).show(ctx,|ui|{
                        ui.horizontal(|ui|{number_stepper(ui,&mut value,0,10,"",step,76.0);});
                    });
                });
            }
            assert_eq!(value,expected,"初值 {initial}，步长 {step}，点击位置 {x}");
        }
    }

    #[test]
    fn number_stepper_fits_with_settings_button_padding() {
        let ctx=egui::Context::default();
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(340.0,240.0))),..Default::default()};
        let _=ctx.run(raw,|ctx|{
            egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{
                ui.spacing_mut().button_padding=egui::vec2(10.0,6.0);
                for mut value in [0u32,25,180] {
                    let row=ui.horizontal(|ui|{number_stepper(ui,&mut value,0,180," 分钟",1,76.0);});
                    assert!(row.response.rect.width()<=140.1,"数值控件超出预留宽度：{}",row.response.rect.width());
                }
                for mut value in [25u32,5,15] {setting_number(ui,"计时时长",&mut value,1,180," 分钟",1.0);}
                assert!(ui.min_rect().right()<=340.1,"连续数值行不应撑宽卡片");
            });
        });
    }

    #[test]
    fn settings_groups_keep_every_page_reachable() {
        use crate::island::extras::settings_tab_group_for_tests as group;
        let names:Vec<_>=SETTINGS_GROUPS.iter().map(|group|group.0).collect();
        assert_eq!(names,["通用","岛体","行为","提醒","音乐","专注","健康","待办","置物架","剪贴板","邮箱","日程","AI 额度","AI 会话","天气","Windows 通知","关于"]);
        let sections:Vec<_>=SETTINGS_SECTIONS.iter().map(|section|section.0).collect();
        assert_eq!(sections,["应用","功能","其他"]);
        // 分组必须连续覆盖所有侧栏项，侧栏才能按切片直接渲染。
        let mut covered=0;
        for (_,from,to) in SETTINGS_SECTIONS {assert_eq!(from,covered);covered=to;}
        assert_eq!(covered,SETTINGS_GROUPS.len());
        assert_eq!(group(0),"通用");
        assert_eq!(group(2),"专注");assert_eq!(group(3),"健康");
        assert_eq!(group(4),"待办");assert_eq!(group(5),"置物架");
        assert_eq!(group(6),"提醒");assert_eq!(group(7),"邮箱");assert_eq!(group(8),"日程");
        assert_eq!(group(9),"关于");assert_eq!(group(15),"AI 会话");
        assert_eq!(group(16),"岛体");assert_eq!(group(SETTINGS_TAB_CLIPBOARD),"剪贴板");
        assert_eq!(group(SETTINGS_TAB_WINDOWS),"Windows 通知");
        assert_eq!(group(SETTINGS_TAB_APPEARANCE),"岛体","外观并入岛体");
        assert_eq!(group(SETTINGS_TAB_PAGES),"岛体","功能页面并入岛体");
        assert_eq!(group(SETTINGS_TAB_BEHAVIOR),"行为");
        let body=names.iter().position(|name|*name=="岛体").unwrap();
        assert_eq!((names[body-1],names[body+1]),("通用","行为"));
        let reminders=names.iter().position(|name|*name=="提醒").unwrap();
        assert_eq!((names[reminders-1],names[reminders+1]),("行为","音乐"));
        let feature:Vec<_>=settings_feature_rows([0,1,2,3,11,4,5,6,7,8,9,10]).into_iter().map(|row|row.0).collect();
        assert_eq!(feature,["音乐","专注","健康","置物架","剪贴板","待办","邮箱","AI 额度","日程","天气","AI 会话","Windows 通知"]);
        let reordered:Vec<_>=settings_feature_rows([1,0,2,3,11,4,5,6,7,8,9,10]).into_iter().map(|row|row.0).collect();
        assert_eq!(&reordered[..2],["专注","音乐"],"功能组应跟随岛上的页面顺序");
    }

    #[test]
    fn settings_body_icon_renders() {
        let image=egui_extras::image::load_svg_bytes(include_bytes!("../../assets/settings-icons/system/body.svg")).unwrap();
        assert!(image.pixels.iter().any(|pixel|pixel.a()>0));
    }

    #[test]
    fn device_icons_render_as_visible_svg() {
        let icons: &[&[u8]] = &[
            include_bytes!("../../assets/device-icons/headphones-round-bold.svg"),
            include_bytes!("../../assets/device-icons/speaker-bold.svg"),
            include_bytes!("../../assets/device-icons/keyboard-bold.svg"),
            include_bytes!("../../assets/device-icons/mouse-bold.svg"),
            include_bytes!("../../assets/device-icons/gamepad-bold.svg"),
            include_bytes!("../../assets/device-icons/smartphone-bold.svg"),
            include_bytes!("../../assets/device-icons/bluetooth-linear.svg"),
        ];
        for bytes in icons {
            let image=egui_extras::image::load_svg_bytes(bytes).unwrap();
            assert!(image.pixels.iter().any(|p|p.a()>0));
        }
    }


}

pub(crate) fn setting_button(c:&Card,x:f32,y:f32,w:f32,text:&str,on:bool,id:&str)->bool {
    let (r,b)=crate::visuals::paint::capsule(c,x,y,w,19.0,id,if on{Color32::from_rgb(39,94,150)}else{theme::BTN},theme::BTN_HOVER);
    b.text(x+w/2.0,y+13.3,text,10.5,W::Semibold,theme::WHITE,H::Center);
    r.clicked()
}
pub(crate) fn island_number_field(c:&Card,rect:Rect,value:egui::DragValue<'_>,id:impl std::hash::Hash)->egui::Response {
    let mut ui=egui::Ui::new(c.ui.ctx().clone(),egui::Id::new(id),egui::UiBuilder::new().layer_id(c.ui.layer_id()).max_rect(rect).style(c.ui.style().clone()));
    ui.set_clip_rect(c.painter.clip_rect());ui.set_opacity(c.painter.opacity());if !c.live{ui.disable();}
    ui.spacing_mut().interact_size=rect.size();ui.spacing_mut().button_padding=egui::vec2(2.0,1.0);
    ui.style_mut().drag_value_text_style=egui::TextStyle::Body;
    ui.style_mut().text_styles.insert(egui::TextStyle::Body,c.font(11.0,W::Bold));
    ui.style_mut().explanation_tooltips=false;
    ui.visuals_mut().widgets.inactive.weak_bg_fill=Color32::TRANSPARENT;
    crate::quota::ui::bar_tooltip(ui.put(rect,value.update_while_editing(false)),"点击输入数值".into())
}
impl App {
    pub(crate) fn page_settings(&mut self,c:&Card) {
        let help=match self.settings_category{0=>"外观与系统",1=>"调整喝水、久坐与提醒时段",2=>"点击分钟数可直接输入，也可用加减微调",_=>"通知"};
        if self.settings_category!=1{c.text(44.0,44.0,help,10.5,W::Regular,theme::GRAY,H::Left);}
        let before=format!("{:?}",self.settings);
        self.settings_editing=false;
        match self.settings_category {
            0=>{
                let system_hud=self.settings.hud_volume||self.settings.hud_power||self.settings.hud_network||self.settings.hud_keyboard;
                let choices=[("吉祥物",self.settings.pet),("系统状态",system_hud),("全屏隐藏",self.settings.hide_fullscreen)];
                for (i,(name,on)) in choices.iter().enumerate(){let x=42.0+(i%2) as f32*169.0;let y=57.0+(i/2) as f32*27.0;let text=format!("{}  {}",name,if *on{"开"}else{"关"});if setting_button(c,x,y,158.0,&text,*on,&format!("general-{i}")){match i{0=>self.set_pet(!self.settings.pet),1=>{self.settings.hud_volume=!system_hud;self.settings.hud_power=!system_hud;self.settings.hud_network=!system_hud;self.settings.hud_keyboard=!system_hud;},_=>self.settings.hide_fullscreen=!self.settings.hide_fullscreen}}}
                c.text(42.0,116.0,"岛仔收起时常驻",10.5,W::Regular,theme::GRAY,H::Left);
            }
            1 if self.health_time=>self.health_time_page(c),
            1=>self.health_quick_settings(c),
            2=>{
                for (i,name) in ["专注","休息"].iter().enumerate(){
                    let y=53.0+i as f32*23.0;
                    c.text(48.0,y+13.0,name,10.5,W::Regular,theme::GRAY,H::Left);
                    let value=if i==0{&mut self.settings.focus_minutes}else{&mut self.settings.rest_minutes};
                    let max=if i==0{180}else{60};let rect=c.rect(168.0,y,76.0,19.0);
                    let field=island_number_field(c,rect,egui::DragValue::new(value).range(1..=max).suffix(" 分钟"),("focus-duration",i));
                    if field.gained_focus(){self.pending.push(Cmd::Focus);}
                    self.settings_editing|=field.has_focus();
                    if setting_button(c,131.0,y,27.0,"−",false,&format!("focus-minus-{i}")){*value=value.saturating_sub(1).max(1);}
                    if setting_button(c,255.0,y,27.0,"+",false,&format!("focus-plus-{i}")){*value=(*value+1).min(max);}
                }
                c.text(315.0,93.0,"下次生效",10.5,W::Regular,theme::GRAY,H::Center);
            }
            _=>{
                if setting_button(c,43.0,56.0,152.0,if self.settings.notifications{"提醒 · 开"}else{"提醒 · 关"},self.settings.notifications,"set-notice"){self.settings.notifications=!self.settings.notifications;}
                if setting_button(c,212.0,56.0,153.0,if self.settings.system_notify && self.settings.sound{"声音 · 开"}else{"声音 · 关"},self.settings.system_notify && self.settings.sound,"set-sound"){
                    self.settings.sound=!(self.settings.system_notify && self.settings.sound);
                    if self.settings.sound {self.settings.system_notify=true;}
                }
                if setting_button(c,43.0,84.0,152.0,if self.settings.read_notifications{"断开系统通知"}else{"连接系统通知"},self.settings.read_notifications,"set-listener"){self.pending.push(if self.settings.read_notifications{Cmd::DisconnectNotifications}else{Cmd::RequestNotifications});}
                let status=c.fit(&self.listener_status,10.5,W::Regular,170.0);c.text(210.0,97.0,&status,10.5,W::Regular,theme::GRAY,H::Left);
            }
        }
        if before!=format!("{:?}",self.settings){self.settings.save();}
    }
}

