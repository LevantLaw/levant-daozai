//! 健康设置：喝水与久坐开关、提醒时段，以及岛内健康页的设置子页。

use crate::health::HealthReminder;
use crate::island::extras::{island_number_field, setting_button};
use crate::island::settings::{
    number_stepper, setting_number, setting_row, settings_card, settings_control_row, settings_hint,
    time_stepper, TIME_RANGE_CLOCK_W, TIME_RANGE_HOURS_W,
};
use crate::island::{App, Cmd};
use crate::visuals::paint::{self, Card, H};
use crate::visuals::theme::{self, W};
use eframe::egui::{self, Color32};

impl App {
    pub(crate) fn health_settings(&mut self, ui:&mut egui::Ui) {
        let on=self.settings_feature_header(ui,crate::island::WATER_PAGE,"喝水与久坐提醒");
        ui.add_enabled_ui(on,|ui|{
            settings_card(ui,|ui|{
                // 喝水：开关一行，数量与间隔紧随其后。
                let old_remind=self.water.remind;
                setting_row(ui,"喝水提醒","确认或喝水后重新计时；达标后停止提醒",&mut self.water.remind);
                if self.water.remind!=old_remind{self.water.save();self.confirm_health_reminder(HealthReminder::Water);}
                ui.add_enabled_ui(self.water.remind,|ui|{
                    setting_number(ui,"提醒间隔",&mut self.settings.water_minutes,5,180," 分钟",1.0);
                    setting_number(ui,"每次记录",&mut self.settings.serving_ml,50,1000," ml",25.0);
                    if setting_number(ui,"每日目标",&mut self.water.goal_ml,500,6000," ml",50.0){self.water.save();}
                });
                ui.separator();
                let old_sit=self.settings.sit_remind;
                setting_row(ui,"久坐提醒","确认提醒或点「起身了」后重新计时",&mut self.settings.sit_remind);
                if self.settings.sit_remind!=old_sit {self.confirm_health_reminder(HealthReminder::Sit);}
                ui.add_enabled_ui(self.settings.sit_remind,|ui|{
                    setting_number(ui,"久坐时长",&mut self.settings.sit_minutes,15,180," 分钟",1.0);
                });
                ui.separator();
                // 提醒时段与午休对喝水和久坐都生效。
                // 喝水和久坐提醒都关掉时，时段和午休不起作用，一起置灰。
                let any_reminder=self.water.remind||self.settings.sit_remind;
                ui.add_enabled_ui(any_reminder,|ui|{
                    settings_control_row(ui,"提醒时段",80.0,|ui|{
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                            // 区域宽度正好等于两个加减器加「至」，靠右放置后与上面单个加减器的右边缘对齐。
                            ui.allocate_ui_with_layout(egui::vec2(TIME_RANGE_HOURS_W,30.0),egui::Layout::left_to_right(egui::Align::Center),|ui|{
                                number_stepper(ui,&mut self.settings.water_start,0,23," 时",1,48.0);
                                ui.label("至");
                                number_stepper(ui,&mut self.settings.water_end,0,24," 时",1,48.0);
                            });
                        }).response
                    });
                    setting_row(ui,"午休不提醒","午休时间内喝水和久坐都不弹提醒",&mut self.settings.lunch_rest);
                    if self.settings.lunch_rest {
                        settings_control_row(ui,"午休时间",80.0,|ui|{
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                                ui.allocate_ui_with_layout(egui::vec2(TIME_RANGE_CLOCK_W,30.0),egui::Layout::left_to_right(egui::Align::Center),|ui|{
                                    time_stepper(ui,&mut self.settings.lunch_start,"lunch-start");
                                    ui.label("至");
                                    time_stepper(ui,&mut self.settings.lunch_end,"lunch-end");
                                });
                            }).response
                        });
                    }
                    settings_hint(ui,"提醒时段和午休对喝水、久坐都生效；专注计时进行中不打扰，结束后再提醒。");
                });
            });
        });
    }

    /// 岛内健康页的「设置」子页：喝水三行、久坐一行，最后一行进入提醒时段二级页。
    pub(crate) fn health_quick_settings(&mut self, c:&Card) {
        // 与健康页同一套切换（标题旁的水滴 / 椅子）；四行一列，标签靠左、加减器靠右对齐。
        // 喝水三行、久坐一行加两句说明；最后一行「提醒时段」两边共用，位置不变。
        let sv=self.sit_view_t.pos.clamp(0.0,1.0);
        let row_y=|i:usize|38.0+i as f32*22.0;
        for i in 1..4{c.fill(44.0,row_y(i)-2.0,326.0,1.0,0.5,Color32::from_rgb(28,28,31));}
        if sv<0.99{
            let wc=c.faded(1.0-sv);let wc=if self.sit_mode{wc.ghost()}else{wc};
            for (i,label) in ["每日目标","每次记录","提醒间隔"].iter().enumerate(){
                wc.text(44.0,row_y(i)+13.0,label,10.5,W::Regular,theme::GRAY,H::Left);
                let goal=self.water.goal_ml;let mut liters=goal as f32/1000.0;
                let value=match i {
                    0=>egui::DragValue::new(&mut liters).range(0.5..=6.0).speed(0.25).suffix(" L").min_decimals(1).max_decimals(2),
                    1=>egui::DragValue::new(&mut self.settings.serving_ml).range(50..=1000).speed(50.0).suffix(" ml"),
                    _=>egui::DragValue::new(&mut self.settings.water_minutes).range(5..=180).speed(5.0).suffix(" 分钟"),
                };
                let (d,field)=stepper(&wc,370.0,row_y(i),value,&format!("water-{i}"));
                if field.gained_focus(){self.pending.push(Cmd::Focus);}self.settings_editing|=field.has_focus();
                match i {
                    0=>{self.water.goal_ml=((liters*1000.0).round() as i32+d*250).clamp(500,6000) as u32;if self.water.goal_ml!=goal{self.water.save();}}
                    1=>self.settings.serving_ml=(self.settings.serving_ml as i32+d*50).clamp(50,1000) as u32,
                    _=>self.settings.water_minutes=(self.settings.water_minutes as i32+d*5).clamp(5,180) as u32,
                }
            }
        }
        if sv>0.01{
            let sc=c.faded(sv);let sc=if self.sit_mode{sc}else{sc.ghost()};
            sc.text(44.0,row_y(0)+13.0,"坐满多久提醒",10.5,W::Regular,theme::GRAY,H::Left);
            let value=egui::DragValue::new(&mut self.settings.sit_minutes).range(15..=180).speed(5.0).suffix(" 分钟");
            let (d,field)=stepper(&sc,370.0,row_y(0),value,"sit");
            if field.gained_focus(){self.pending.push(Cmd::Focus);}self.settings_editing|=field.has_focus();
            if d!=0{self.settings.sit_minutes=(self.settings.sit_minutes as i32+d*5).clamp(15,180) as u32;}
            sc.text(44.0,row_y(1)+13.0,"专注计时进行中不打扰，专注结束后再提醒",10.5,W::Regular,theme::GRAY,H::Left);
            sc.text(44.0,row_y(2)+13.0,"提醒后常驻，点击确认或起身后重新计时",10.5,W::Regular,theme::GRAY,H::Left);
        }
        // 最后一行两边共用：提醒时段 + 午休的摘要，点进二级页修改。
        let y=row_y(3);
        let r=c.hit(40.0,y-2.0,334.0,22.0,"health-time").on_hover_text("设置提醒时段和午休免打扰");
        let (hv,_)=c.hover_t(&r);
        if hv>0.0{c.fill(40.0,y-2.0,334.0,22.0,8.0,Color32::from_white_alpha((hv*14.0) as u8));}
        c.text(44.0,y+13.0,"提醒时段",10.5,W::Regular,theme::GRAY,H::Left);
        c.text(356.0,y+13.0,&self.health_time_summary(),10.5,W::Semibold,theme::WHITE,H::Right);
        c.text(366.0,y+13.5,"›",14.0,W::Bold,theme::GRAY,H::Center);
        if r.clicked(){self.health_time=true;}
    }

    fn health_time_summary(&self)->String{
        let s=&self.settings;
        let hours=format!("{:02}:00–{:02}:00",s.water_start,s.water_end);
        if s.lunch_rest{format!("{hours} · 午休 {}–{}",hhmm(s.lunch_start),hhmm(s.lunch_end))}else{hours}
    }
    /// 「提醒时段」二级页：提醒时段起止、午休开关、午休起止（15 分钟一档），喝水和久坐共用。
    pub(crate) fn health_time_page(&mut self,c:&Card){
        let row_y=|i:usize|38.0+i as f32*22.0;
        for i in 1..4{c.fill(44.0,row_y(i)-2.0,326.0,1.0,0.5,Color32::from_rgb(28,28,31));}
        let y=row_y(0);
        c.text(44.0,y+13.0,"提醒时段",10.5,W::Regular,theme::GRAY,H::Left);
        c.text(250.0,y+13.0,"至",10.5,W::Regular,theme::GRAY,H::Center);
        for i in 0..2{
            let hour=if i==0{&mut self.settings.water_start}else{&mut self.settings.water_end};
            let value=egui::DragValue::new(hour).range(0..=if i==0{23}else{24})
                .custom_formatter(|v,_|format!("{:02}:00",v as u32))
                .custom_parser(|t|{let t=t.trim();t.strip_suffix(":00").or_else(||t.strip_suffix("：00")).unwrap_or(t).parse::<u32>().ok().map(|v|v as f64)});
            let (d,field)=stepper(c,if i==0{238.0}else{370.0},y,value,&format!("hour-{i}"));
            if field.gained_focus(){self.pending.push(Cmd::Focus);}self.settings_editing|=field.has_focus();
            if d!=0{let hour=if i==0{&mut self.settings.water_start}else{&mut self.settings.water_end};*hour=(*hour as i32+d).rem_euclid(24) as u32;}
        }
        let y=row_y(1);
        c.text(44.0,y+13.0,"午休不提醒",10.5,W::Regular,theme::GRAY,H::Left);
        if island_switch(c,336.0,y-0.5,self.settings.lunch_rest,"lunch-rest"){self.settings.lunch_rest=!self.settings.lunch_rest;}
        let y=row_y(2);
        let on=self.settings.lunch_rest;
        let lc=c.faded(if on{1.0}else{0.35});let lc=if on{lc}else{lc.ghost()};
        lc.text(44.0,y+13.0,"午休时间",10.5,W::Regular,theme::GRAY,H::Left);
        lc.text(250.0,y+13.0,"至",10.5,W::Regular,theme::GRAY,H::Center);
        for i in 0..2{
            let minute=if i==0{&mut self.settings.lunch_start}else{&mut self.settings.lunch_end};
            let value=egui::DragValue::new(minute).range(0..=1439).speed(1.0).custom_formatter(|v,_|hhmm(v as u32))
                .custom_parser(|t|{let (h,m)=t.trim().split_once([':','：'])?;Some((h.parse::<u32>().ok()?.min(23)*60+m.parse::<u32>().ok()?.min(59)) as f64)});
            let (d,field)=stepper(&lc,if i==0{238.0}else{370.0},y,value,&format!("lunch-{i}"));
            if field.gained_focus(){self.pending.push(Cmd::Focus);}self.settings_editing|=field.has_focus();
            if d!=0{let minute=if i==0{&mut self.settings.lunch_start}else{&mut self.settings.lunch_end};*minute=(*minute as i32+d*15).rem_euclid(1440) as u32;}
        }
        c.text(44.0,row_y(3)+13.0,"喝水和久坐提醒都只在提醒时段内、午休以外弹出",10.5,W::Regular,theme::GRAY,H::Left);
    }
}

fn hhmm(m:u32)->String{format!("{:02}:{:02}",m/60%24,m%60)}
/// 岛内小开关（与健康页右上开关同款）。
fn island_switch(c:&Card,x:f32,y:f32,on:bool,id:&str)->bool{
    let r=c.hit(x-2.0,y-2.0,38.0,24.0,id);
    let (hv,_)=c.hover_t(&r);
    let track=if on{theme::SIT_GREEN}else{Color32::from_rgb(58,58,62)};
    c.fill(x,y,34.0,20.0,10.0,paint::mix(track,theme::WHITE,hv*0.08));
    c.circle(if on{x+24.0}else{x+10.0},y+10.0,8.0,theme::WHITE);
    r.clicked()
}
/// 加减器：[−] 可输入数值 [+]，右缘对齐到 right。
fn stepper(c:&Card,right:f32,y:f32,value:egui::DragValue<'_>,id:&str)->(i32,egui::Response) {
    let (w,vw)=(25.0,58.0);
    let x=right-w*2.0-vw;
    let field=island_number_field(c,c.rect(x+w,y,vw,19.0),value,("health-number",id));
    let delta=if setting_button(c,x,y,w,"−",false,&format!("{id}-minus")){-1}
        else if setting_button(c,right-w,y,w,"+",false,&format!("{id}-plus")){1}else{0};
    (delta,field)
}
