use crate::{app::bar_layout::*,app::extras::{setting_row_nested,settings_group_fold},render,render::paint::{self,Card,H},app::settings_theme::color as settings_color,render::theme::W};
use eframe::egui::{self,Color32,RichText,Rect,Pos2,Vec2};

/// 预览编辑器自己的统一间距；不跟随岛上的 `GAP`，以便在有限宽度里放下全部组件。
const PREVIEW_GAP:f32=6.0;

pub(crate) fn preview_layout(order:&[usize],quotas:usize,music:bool,focus:bool,weather:bool,battery:bool)->Layout {
    let mut layout=Layout::new_with_gap(PREVIEW_GAP,battery,quotas,music,focus,false,true,weather,true);
    layout.reorder(order);layout
}

impl crate::app::App {
    /// 组件当前是否属于事件容器。
    pub(crate) fn resident_tray_member(&self,slot:usize)->bool {
        match slot {
            WEATHER=>self.settings.tray_weather,QUOTA=>self.settings.tray_quota,FOCUS=>self.settings.tray_focus,
            MAIL=>self.settings.tray_mail,HEALTH=>self.settings.tray_health,TODO=>self.settings.tray_todo,
            SHELF=>self.settings.tray_shelf,CALENDAR=>self.settings.tray_calendar,MUSIC=>self.settings.tray_music,
            BATTERY=>self.settings.tray_battery,_=>false,
        }
    }
    /// 「常驻显示 / 事件显示」两组：按固定顺序展示组件开关；归属在岛体预览里拖动调整。
    pub(crate) fn resident_groups_settings(&mut self,ui:&mut egui::Ui) {
        let persistent:Vec<usize>=GROUP_ORDER.iter().copied().filter(|slot|!self.resident_tray_member(*slot)).collect();
        let event:Vec<usize>=GROUP_ORDER.iter().copied().filter(|slot|self.resident_tray_member(*slot)).collect();
        self.resident_group(ui,"persistent","常驻显示",&persistent,false);
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(2.0);
        self.resident_group(ui,"event","事件显示",&event,true);
    }
    fn resident_group(&mut self,ui:&mut egui::Ui,id:&str,title:&str,slots:&[usize],empty_hint:bool) {
        settings_group_fold(ui,id,title,|ui|{
            for (index,slot) in slots.iter().copied().enumerate() {
                if index>0 {ui.separator();}
                ui.push_id(("resident-group",slot),|ui|ui.horizontal(|ui|{
                    ui.add_space(12.0);
                    ui.vertical(|ui|{
                        ui.set_width(ui.available_width());
                        self.resident_group_row(ui,slot);
                    });
                }));
            }
            if empty_hint && slots.is_empty() {
                let ctx=ui.ctx().clone();
                ui.horizontal(|ui|{
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("暂无事件组件；在岛体预览中把组件拖入右侧事件区即可加入。").size(11.0).color(settings_color(&ctx,Color32::from_gray(150),Color32::from_gray(95))));
                });
            }
        });
    }
    fn resident_group_row(&mut self,ui:&mut egui::Ui,slot:usize) {
        match slot {
            AGENT=>{
                let before=self.agent.config.display.bar_indicator;
                setting_row_nested(ui,"AI 会话状态灯","运行与等待的状态灯，无活跃会话时显示暗色占位",&mut self.agent.config.display.bar_indicator);
                if before!=self.agent.config.display.bar_indicator {self.agent.config.save();}
            }
            PET=>{
                let mut pet=self.settings.pet;
                setting_row_nested(ui,"岛仔吉祥物","收起时常驻，播放音乐时一起唱歌",&mut pet);
                if pet!=self.settings.pet {self.set_pet(pet);}
            }
            BATTERY=>setting_row_nested(ui,"电量","无电池设备自动隐藏",&mut self.settings.show_battery),
            CLOCK=>setting_row_nested(ui,"时间","关闭后仍可在展开顶栏查看",&mut self.settings.show_clock),
            DND=>setting_row_nested(ui,"勿扰模式","点击月亮自动打开控制中心切换系统勿扰；开启时常驻",&mut self.settings.show_dnd),
            MUSIC=>setting_row_nested(ui,"音乐","播放时显示动态音柱",&mut self.settings.show_music),
            FOCUS=>setting_row_nested(ui,"专注","计时进行中显示进度环",&mut self.settings.show_focus),
            QUOTA=>{
                let before=self.quota.config.compact;
                setting_row_nested(ui,"AI 额度","常驻账号；WorkBuddy 积分在前，DeepSeek 余额在最后",&mut self.quota.config.compact);
                if before!=self.quota.config.compact {self.quota.config.save();}
            }
            CALENDAR=>setting_row_nested(ui,"日程","会议提醒时短暂显示，点击打开日程页",&mut self.settings.show_calendar),
            WEATHER=>setting_row_nested(ui,"天气","天气图标和温度，点击打开天气页",&mut self.settings.show_weather),
            MAIL=>setting_row_nested(ui,"邮箱","未读邮件时显示信封和数量，点击打开邮箱页",&mut self.settings.show_mail),
            HEALTH=>setting_row_nested(ui,"健康","喝水 / 久坐提醒未确认时显示图标，点击打开健康页",&mut self.settings.show_health),
            TODO=>setting_row_nested(ui,"待办","到期未完成时显示清单和数量，点击打开待办页",&mut self.settings.show_todo),
            SHELF=>setting_row_nested(ui,"置物架","拖入文件后短暂显示，点击打开置物架页",&mut self.settings.show_shelf),
            _=>{}
        }
    }
    pub(crate) fn resident_order_settings(&mut self,ui:&mut egui::Ui){
        let default=if self.settings.show_battery {DEFAULT_ORDER.to_vec()}else{vec![QUOTA,PET,CLOCK,WEATHER,BATTERY,MUSIC,FOCUS,AGENT]};
        let mut order=if valid_order(&self.settings.resident_order){self.settings.resident_order.clone()}else{default.clone()};
        ui.horizontal(|ui|{
            ui.label(RichText::new("岛体预览").color(ui.visuals().text_color()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                if ui.small_button("恢复默认").clicked(){
                    self.settings.resident_order.clear();
                    self.settings.tray_order.clear();
                    order=default;
                    self.settings.tray_weather=true;self.settings.tray_quota=true;self.settings.tray_focus=true;
                    self.settings.tray_mail=false;self.settings.tray_health=false;self.settings.tray_shelf=false;
                    self.settings.tray_todo=false;self.settings.tray_music=false;self.settings.tray_battery=false;
                    self.settings.tray_calendar=false;
                    self.settings.show_mail=true;self.settings.show_health=true;self.settings.show_todo=true;
                    self.settings.show_shelf=true;self.settings.show_calendar=true;self.settings.show_dnd=true;
                }
            });
        });
        ui.label(RichText::new("按住图标左右拖动可排序；拖入或拖出右侧事件区，切换常驻与事件显示。").size(11.0).color(ui.visuals().weak_text_color()));
        ui.add_space(12.0);
        let tray_weather=self.settings.tray_weather;
        let tray_quota=self.settings.tray_quota;
        let tray_focus=self.settings.tray_focus;
        let tray_mail=self.settings.tray_mail;
        let tray_health=self.settings.tray_health;
        let tray_todo=self.settings.tray_todo;
        let tray_shelf=self.settings.tray_shelf;
        let tray_calendar=self.settings.tray_calendar;
        let tray_music=self.settings.tray_music;
        let tray_battery=self.settings.tray_battery;
        let indices=self.quota_pinned();let count=indices.len();
        let weather_snapshot=self.weather_pill().cloned();
        let weather_extra=self.weather_pill_extra(ui.ctx()).unwrap_or(0.0).max(0.0);
        let weather_width=WEATHER_WIDTH+weather_extra;
        let quota_width=if count>0{self.quota_tray_width(ui.ctx())}else{27.0};
        let mut layout=preview_layout(&order,if tray_quota{0}else{count.max(1)},!tray_music,!tray_focus,!tray_weather,!tray_battery);
        for (position,extra) in self.quota_balance_layout(ui.ctx()){layout.balance_tail(position,extra);}
        if self.settings.clock_period!=0 { layout.widen_clock(40.0); }
        if self.settings.clock_seconds { layout.widen_clock(24.0); }
        if let Some(extra)=self.weather_pill_extra(ui.ctx()){layout.widen(WEATHER,extra);}
        // 事件容器编辑器：按保存的顺序并排，空容器也保留可拖回的框。
        let tray_order=if valid_tray_order(&self.settings.tray_order){self.settings.tray_order.clone()}else{TRAY_ORDER.to_vec()};
        let tray_on=|slot:usize|match slot{
            WEATHER=>tray_weather,QUOTA=>tray_quota,FOCUS=>tray_focus,
            MAIL=>tray_mail,HEALTH=>tray_health,TODO=>tray_todo,SHELF=>tray_shelf,CALENDAR=>tray_calendar,
            MUSIC=>tray_music,BATTERY=>tray_battery,_=>false,
        };
        let members:Vec<(usize,bool,f32)>=tray_order.iter().copied()
            .map(|slot|{
                let width=match slot{
                    WEATHER=>weather_width,QUOTA=>quota_width,FOCUS=>crate::app::focus_bar::WIDTH,
                    MAIL|HEALTH|TODO|SHELF|CALENDAR=>crate::app::page_badges::badge_width(slot),
                    MUSIC|BATTERY=>slot_width(slot),_=>0.0,
                };
                (slot,tray_on(slot),width)
            }).collect();
        let natural:Vec<f32>=members.iter().filter(|(_,on,_)|*on).map(|(_,_,width)|*width).collect();
        let editor_width=if natural.is_empty(){44.0}else{natural.iter().sum::<f32>()+PREVIEW_GAP*(natural.len()-1) as f32+16.0};
        let badges=self.preview_badges();
        layout.add_extras(&badges.iter().map(|badge|(badge.slot,badge.width)).collect::<Vec<_>>());
        layout.add_tray(editor_width);
        let (stage,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),100.0),egui::Sense::hover());
        ui.painter().rect_filled(stage,10.0,Color32::from_rgb(19,25,36));
        // 预览按预览区宽度整体缩放，保证所有已开启组件都能完整显示；间距在预览内统一。
        let scale=((stage.width()-28.0)/layout.width).min(1.0);
        let origin=Pos2::new(stage.center().x-layout.width*scale/2.0,stage.top());
        // 预览跟随当前的皮肤与形状。
        let pill=self.settings.island_shape==crate::model::IslandShape::Pill;
        let origin=if pill{origin+Vec2::new(0.0,4.0)}else{origin};
        let body=Rect::from_min_size(origin,Vec2::new(layout.width*scale,30.0*scale));
        let fill=self.preview_fill();
        if pill {ui.painter().rect_filled(body,15.0*scale,fill);}
        else {crate::render::shape::fill(ui.painter(),body,8.0*scale,10.0*scale,12.0*scale,fill,1.0/ui.ctx().pixels_per_point());}
        if self.settings.skin.is_glass() {
            let rim=egui::Stroke::new(1.0_f32,Color32::from_white_alpha(60));
            if pill {ui.painter().rect_stroke(body,15.0*scale,rim,egui::StrokeKind::Inside);}
            else {
                let edge=Rect::from_min_max(body.min+Vec2::new(0.5,0.0),body.max-Vec2::new(0.5,0.5));
                ui.painter().add(egui::Shape::line(crate::render::shape::outline(edge,8.0*scale,10.0*scale,12.0*scale-0.5),rim));
            }
        }
        let content_from=crate::render::skin_paint::shape_count(ui.ctx(),ui.layer_id());
        let mut card=Card::new(ui,origin,stage);card.s=scale;
        let cx=layout.width/2.0;
        let pointer=ui.ctx().pointer_interact_pos();
        let dragged=egui::DragAndDrop::payload::<usize>(ui.ctx());
        let mut released=None;
        let mut row_bounds:Vec<(usize,Rect)>=Vec::new();
        let label=|slot|match slot{CLOCK=>"时间",BATTERY=>"电量",WEATHER=>"天气",QUOTA=>"AI 额度",MUSIC=>"音乐",FOCUS=>"专注",AGENT=>"AI 会话",MAIL=>"邮箱",HEALTH=>"健康",TODO=>"待办",SHELF=>"置物架",CALENDAR=>"日程",_=>"吉祥物"};
        // 事件容器的框：拖入拖出都按它判断。
        let tray_width=layout.widths[TRAY];
        let tray_x=cx+layout.x[TRAY].unwrap();
        let tray_rect=card.rect(tray_x-tray_width/2.0,1.0,tray_width,28.0);
        ui.painter().rect_filled(tray_rect,8.0*scale,Color32::from_white_alpha(13));
        ui.painter().rect_stroke(tray_rect,8.0*scale,egui::Stroke::new(1.0,Color32::from_white_alpha(30)),egui::StrokeKind::Inside);
        let mut member_bounds:Vec<(usize,Rect)>=Vec::new();
        let mut member_x=tray_x-tray_width/2.0+8.0;
        for (slot,on,width) in members {
            if !on {continue;}
            let x=member_x+width/2.0;
            let rect=card.rect(member_x-2.0,1.0,width+4.0,28.0);
            member_bounds.push((slot,rect));
            let response=ui.interact(rect,ui.id().with(("resident-icon",slot)),egui::Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::Grab);
            response.dnd_set_drag_payload(slot);
            if response.drag_stopped(){released=Some(slot);}
            if dragged.as_deref()==Some(&slot){ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);}
            else if response.hovered(){ui.painter().rect_filled(rect,6.0,Color32::from_white_alpha(18));}
            let active=match slot{
                WEATHER=>weather_snapshot.is_some(),
                QUOTA=>count>0,
                FOCUS=>self.settings.show_focus&&self.pomo.running,
                MUSIC=>self.settings.show_music&&self.media.is_music(),
                BATTERY=>self.settings.show_battery&&self.resident_power.0.is_some(),
                MAIL=>self.settings.show_mail&&self.mail_unread_count()>0,
                HEALTH=>self.settings.show_health&&!self.health_badges().is_empty(),
                TODO=>self.settings.show_todo&&self.todo_due_count()>0,
                SHELF=>self.settings.show_shelf&&self.t<self.shelf_badge_until,
                CALENDAR=>self.settings.show_calendar&&self.t<self.calendar_badge_until,
                _=>false,
            };
            let c=card.faded(if active{1.0}else{0.38}).ghost();
            match slot{
                WEATHER=>if let Some(snapshot)=weather_snapshot.as_ref(){let mut paint=layout.clone();paint.widths[WEATHER]=weather_width;self.paint_pill_weather(&c,x,&paint,snapshot);}else{crate::weather::ui::icon(&c,x,15.0,16.0,crate::weather::Kind::Partly);},
                QUOTA if count>0=>self.draw_quota_tray(&c,x),
                QUOTA=>{c.ring(x,15.0,10.0,2.0,render::theme::GRAY);c.text(x,18.0,"AI",7.0,W::Bold,render::theme::WHITE,H::Center);},
                FOCUS=>crate::app::focus_bar::draw(&c,x,15.0,self.t,&self.pomo,self.phase_secs(self.pomo.phase),false),
                MUSIC=>paint::playback_bars_box(&c,x,15.0,self.t,self.media.playing,render::theme::WHITE,GLYPH,MUSIC_H),
                BATTERY=>paint::battery(&c,x,15.0,self.resident_power.0,self.resident_power.1,BATTERY_SCALE),
                MAIL|HEALTH|TODO|SHELF|CALENDAR=>self.paint_badge(&c,slot,x,render::theme::WHITE),
                _=>{}
            }
            if response.clicked(){
                if let Some(page)=match slot{MAIL=>Some(crate::app::MAIL_PAGE),HEALTH=>Some(crate::app::WATER_PAGE),TODO=>Some(crate::app::TODO_PAGE),SHELF=>Some(crate::app::SHELF_PAGE),CALENDAR=>Some(crate::app::CALENDAR_PAGE),_=>None}{
                    self.settings_open=false;self.show_page(page);
                }
            }
            response.on_hover_text(label(slot));
            member_x+=width+PREVIEW_GAP;
        }
        for &slot in &order{
            let Some(x)=layout.x[slot] else {continue;};
            let x=cx+x;
            let width=match slot{CLOCK=>layout.widths[CLOCK],BATTERY=>20.0,WEATHER=>layout.widths[WEATHER],PET=>32.0,MUSIC=>GLYPH,FOCUS=>crate::app::focus_bar::WIDTH,AGENT=>layout.widths[AGENT],_=>{
                let left=layout.quota_slots().filter_map(|i|layout.x[i].map(|x|x-layout.widths[i]/2.0)).fold(f32::INFINITY,f32::min);
                let right=layout.quota_slots().filter_map(|i|layout.x[i].map(|x|x+layout.widths[i]/2.0)).fold(f32::NEG_INFINITY,f32::max);
                right-left
            }};
            let left=if slot==QUOTA{cx+layout.quota_slots().filter_map(|i|layout.x[i].map(|x|x-layout.widths[i]/2.0)).fold(f32::INFINITY,f32::min)}else{x-width/2.0};
            let rect=card.rect(left-5.0,1.0,width+10.0,28.0);
            row_bounds.push((slot,rect));
            let response=ui.interact(rect,ui.id().with(("resident-icon",slot)),egui::Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::Grab);
            response.dnd_set_drag_payload(slot);
            if response.drag_stopped(){released=Some(slot);}
            let is_dragged=dragged.as_deref()==Some(&slot);
            let dx=if is_dragged{pointer.map(|p|((p.x-origin.x)/scale-(left+width/2.0)).clamp(-left,layout.width-left-width)).unwrap_or(0.0)}else{0.0};
            if is_dragged {ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);}
            else if response.hovered(){ui.painter().rect_filled(rect,6.0,Color32::from_white_alpha(18));}
            let active=match slot{CLOCK=>self.settings.show_clock,BATTERY=>self.settings.show_battery&&self.resident_power.0.is_some(),WEATHER=>weather_snapshot.is_some(),PET=>self.settings.pet,QUOTA=>count>0,MUSIC=>self.settings.show_music&&self.media.is_music(),FOCUS=>self.settings.show_focus&&self.pomo.running,AGENT=>self.agent_bar_visible(),_=>true};
            let c=card.shifted(dx,0.0).faded(if active||is_dragged{1.0}else{0.38}).ghost();
            match slot{
                BATTERY=>paint::battery(&c,x,15.0,self.resident_power.0,self.resident_power.1,1.0),
                CLOCK=>{
                    let text=self.settings.clock_text(&chrono::Local::now().format("%H:%M:%S").to_string());
                    paint::clock(&c,x-paint::clock_width(&c,&text,render::theme::size::PILL_CLOCK)/2.0,21.0,&text,render::theme::size::PILL_CLOCK);
                },
                WEATHER=>if let Some(snapshot)=weather_snapshot.as_ref(){self.paint_pill_weather(&c,x,&layout,snapshot);}else{crate::weather::ui::icon(&c,x,15.0,16.0,crate::weather::Kind::Partly);},
                PET=>self.pet.draw(&c,x,self.pet.clock,self.skin_palette()),
                MUSIC=>paint::playback_bars_box(&c,x,15.0,self.t,self.media.playing,render::theme::WHITE,GLYPH,MUSIC_H),
                FOCUS=>crate::app::focus_bar::draw(&c,x,15.0,self.t,&self.pomo,self.phase_secs(self.pomo.phase),false),
                QUOTA if count>0=>self.draw_quota_bar(&c,cx,&layout,&indices),
                AGENT=>self.draw_agent_light(&c,x,15.0),
                _=>{c.ring(x,15.0,10.0,2.0,render::theme::GRAY);c.text(x,18.0,"AI",7.0,W::Bold,render::theme::WHITE,H::Center);},
            }
            response.on_hover_text(label(slot));
        }
        // 勿扰月亮：页面图标由设置侧栏代表，预览不再重复绘制。
        for badge in &badges {
            let Some(x)=layout.x[badge.slot] else {continue;};
            let x=cx+x;
            let rect=card.rect(x-badge.width/2.0-2.0,1.0,badge.width+4.0,28.0);
            let response=ui.interact(rect,ui.id().with(("page-badge-preview",badge.slot)),egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.hovered(){ui.painter().rect_filled(rect,6.0,Color32::from_white_alpha(18));}
            let active=self.badge_active(badge.slot);
            let c=card.faded(if active{1.0}else{0.38}).ghost();
            self.paint_badge(&c,badge.slot,x,render::theme::WHITE);
            if response.on_hover_text(self.badge_label(badge.slot)).clicked(){
                self.dnd.toggle(self.t);
            }
        }
        // 拖动目标按落点决定；拖动过程中容器边界与成员位置保持不变。
        // 指针移出岛体时不提交排列，只在最近的岛体边缘画一条提示竖线。
        let drag_slot=dragged.as_deref().copied().or(released);
        let tray_zone=tray_rect.expand(6.0);
        let in_tray=pointer.is_some_and(|point|tray_zone.contains(point));
        let tray_target=if let (Some(slot),Some(point))=(drag_slot,pointer) {
            if tray_order.contains(&slot) && in_tray {
                let others:Vec<(usize,Rect)>=member_bounds.iter().filter(|(member,_)|*member!=slot).copied().collect();
                Some(others.iter().find(|(_,rect)|point.x<=rect.center().x).map(|(member,rect)|(*member,false,rect.left()))
                    .or_else(||others.last().map(|(member,rect)|(*member,true,rect.right())))
                    .unwrap_or((slot,true,tray_rect.left()+8.0)))
            } else { None }
        } else { None };
        let row_target=if let (Some(slot),Some(point))=(drag_slot,pointer) {
            if body.contains(point) && !tray_zone.contains(point) {
                let others:Vec<(usize,Rect)>=row_bounds.iter().filter(|(member,_)|*member!=slot).copied().collect();
                let hit=others.iter().find(|(_,rect)|point.x<=rect.center().x).map(|(member,rect)|(*member,false,rect.left()))
                    .or_else(||others.last().map(|(member,rect)|(*member,true,rect.right())));
                hit.and_then(|(target,after,line_x)|order.iter().position(|value|*value==target).map(|index|(slot,index+usize::from(after),line_x)))
            } else { None }
        } else { None };
        let edge_line=if dragged.is_some() && tray_target.is_none() && row_target.is_none() {
            pointer.map(|point|if point.x<body.center().x{body.left()+8.0}else{body.right()-8.0})
        } else { None };
        if dragged.is_some() {
            if let Some((_,_,line_x))=tray_target {
                ui.painter().line_segment([Pos2::new(line_x,tray_rect.top()+3.0),Pos2::new(line_x,tray_rect.bottom()-3.0)],egui::Stroke::new(2.0,render::theme::BLUE));
            }
            if let Some((_,_,line_x))=row_target {
                ui.painter().line_segment([Pos2::new(line_x,body.top()+4.0),Pos2::new(line_x,body.bottom()-4.0)],egui::Stroke::new(2.0,render::theme::BLUE));
            }
            if let Some(line_x)=edge_line {
                ui.painter().line_segment([Pos2::new(line_x,body.top()+4.0),Pos2::new(line_x,body.bottom()-4.0)],egui::Stroke::new(2.0,render::theme::BLUE));
            }
        }
        // 松手时才提交，避免拖动中边界随成员进出而移动。
        if let Some(slot)=released {
            if pointer.is_some() {
                let inside=in_tray;
                match slot{
                    WEATHER=>self.settings.tray_weather=inside,
                    QUOTA=>self.settings.tray_quota=inside,
                    FOCUS=>self.settings.tray_focus=inside,
                    MAIL=>self.settings.tray_mail=inside,
                    HEALTH=>self.settings.tray_health=inside,
                    TODO=>self.settings.tray_todo=inside,
                    SHELF=>self.settings.tray_shelf=inside,
                    CALENDAR=>self.settings.tray_calendar=inside,
                    MUSIC=>self.settings.tray_music=inside,
                    BATTERY=>self.settings.tray_battery=inside,
                    _=>{}
                }
            }
            if let Some((target,after,_))=tray_target {
                if let Some(from)=tray_order.iter().position(|value|*value==slot) {
                    let mut reordered=tray_order.clone();
                    reordered.remove(from);
                    if let Some(index)=reordered.iter().position(|value|*value==target) {
                        reordered.insert(index+usize::from(after),slot);
                        self.settings.tray_order=reordered;
                    }
                }
            }
            if let Some((slot,mut target,_))=row_target {
                if let Some(from)=order.iter().position(|value|*value==slot) {
                    if from<target{target-=1;}
                    order.remove(from);
                    order.insert(target.min(order.len()),slot);
                    self.settings.resident_order=order;
                }
            }
        }
        if let Some(palette)=self.skin_palette() {
            let end=crate::render::skin_paint::shape_count(ui.ctx(),ui.layer_id());
            crate::render::skin_paint::recolor_layer(ui.ctx(),ui.layer_id(),&[(content_from,end)],&palette);
        }
    }
}
