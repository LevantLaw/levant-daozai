use super::*;

fn mail_item(uid:u32)->crate::mail::Item {
    crate::mail::Item{received_unix:None,account:"test@qq.com".into(),from:"产品团队".into(),subject:format!("邮件 {uid}"),time:"09:41".into(),code:None,html:String::new().into(),summary:String::new().into(),links:vec![],meeting:None,uid}
}

fn add_unread_mail(h:&mut H) {
    h.app.mail.accounts=vec![crate::mail::Account{custom:None,email:"test@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
    h.app.mail.recent=vec![mail_item(7)];
    h.app.mail.manage.data.insert(("test@qq.com".into(),crate::mail::manage::Folder::Inbox),
        crate::mail::manage::Snapshot{validity:1,entries:vec![(7,false)],items:vec![]});
}

fn add_due_todo(h:&mut H) {
    h.app.todos.items.push(crate::todo::TodoItem{text:"检查周报".into(),done_on:None,due:Some("2026-10-28 09:40".into()),reminded:true,..Default::default()});
}

fn slots(h:&H)->Vec<usize>{h.app.page_badges().iter().map(|badge|badge.slot).collect()}

fn badge_rect(h:&H,slot:usize)->Rect {
    h.ctx.read_response(egui::Id::new(("card",true,("page-badge",slot))))
        .unwrap_or_else(||panic!("缺少页面徽标 {slot}")).rect
}

#[test]
fn mail_and_todo_badges_open_their_pages(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.frame(vec![]);
    assert!(h.app.page_badges().is_empty(),"没有未读和到期项时不占位");
    add_unread_mail(&mut h);h.frame(vec![]);
    assert_eq!(slots(&h),[crate::island::layout::bar::MAIL],"未读邮件显示信封");
    h.click(badge_rect(&h,crate::island::layout::bar::MAIL).center());
    assert_eq!(h.app.page(),crate::island::MAIL_PAGE);

    let mut h=H::new();h.frame(vec![]);
    add_due_todo(&mut h);h.frame(vec![]);
    assert_eq!(slots(&h),[crate::island::layout::bar::TODO],"到期未完成显示清单");
    h.click(badge_rect(&h,crate::island::layout::bar::TODO).center());
    assert_eq!(h.app.page(),crate::island::TODO_PAGE);
}

#[test]
fn shelf_and_calendar_badges_are_brief(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.frame(vec![]);
    h.app.shelf_badge_until=h.app.t+crate::island::page_badges::HOLD;
    h.app.calendar_badge_until=h.app.t+crate::island::page_badges::HOLD;
    h.frame(vec![]);
    assert!(slots(&h).contains(&crate::island::layout::bar::SHELF) && slots(&h).contains(&crate::island::layout::bar::CALENDAR),"拖入文件和会议提醒后短暂出现");
    h.click(badge_rect(&h,crate::island::layout::bar::CALENDAR).center());
    assert_eq!(h.app.page(),crate::island::CALENDAR_PAGE);

    let mut h=H::new();h.frame(vec![]);
    h.app.shelf_badge_until=h.app.t+crate::island::page_badges::HOLD;
    h.frame(vec![]);
    assert!(slots(&h).contains(&crate::island::layout::bar::SHELF));
    h.t+=crate::island::page_badges::HOLD+1.0;h.frame(vec![]);
    assert!(!slots(&h).contains(&crate::island::layout::bar::SHELF),"置物架徽标只显示十秒");
}

#[test]
fn dnd_badge_and_button_follow_the_system_focus_state(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.dnd=crate::island::dnd::Dnd::fake(true);h.frame(vec![]);
    assert_eq!(slots(&h),[crate::island::layout::bar::DND],"勿扰开启时月亮常驻");
    h.click(badge_rect(&h,crate::island::layout::bar::DND).center());
    assert!(!h.app.dnd.active);
    h.frame(vec![]);
    assert!(!slots(&h).contains(&crate::island::layout::bar::DND),"关闭后月亮消失");

    let mut h=H::new();h.app.dnd=crate::island::dnd::Dnd::fake(true);h.app.show_page(0);
    // 等展开动画落定后再取按钮位置。
    h.hover(H::local(200.0,60.0),1.0);
    let dnd=h.ctx.read_response(egui::Id::new(("card",true,"page-dnd"))).expect("展开后应有勿扰按钮");
    let gear=h.ctx.read_response(egui::Id::new(("card",true,"page-settings"))).expect("展开后应有设置按钮");
    assert!(dnd.rect.right()<=gear.rect.left(),"勿扰按钮在设置齿轮左侧");
    assert!(dnd.rect.center().y<crate::island::STATUS_H&&gear.rect.center().y<crate::island::STATUS_H,"勿扰和设置都在展开顶行");
    h.click(dnd.rect.center());
    assert!(!h.app.dnd.active,"展开后也能关闭勿扰");
    h.frame(vec![]);h.frame(vec![]);
    assert!(h.ctx.read_response(egui::Id::new(("card",true,"page-dnd"))).is_some(),"勿扰关闭后月亮仍常驻在设置左侧");

    h.app.settings.show_dnd=false;h.frame(vec![]);h.frame(vec![]);
    assert!(h.ctx.read_response(egui::Id::new(("card",true,"page-dnd"))).is_none(),"关闭开关后不显示勿扰按钮");
}

#[test]
fn expanded_status_row_compresses_gaps_before_shrinking_icons(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.pet=true;
    h.app.settings.tray_weather=false;
    h.app.settings.tray_focus=false;
    h.app.settings.tray_mail=false;
    h.app.settings.tray_health=false;
    h.app.settings.tray_calendar=false;
    h.app.settings.tray_music=false;
    h.power=(Some(80),true);
    h.media=crate::music::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    h.app.pomo.running=true;h.app.pomo.remaining=600.0;
    h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"杭州".into(),temp:-12.0,..Default::default()});
    add_unread_mail(&mut h);
    h.app.calendar_badge_until=h.app.t+crate::island::page_badges::HOLD;
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    let available=crate::island::PW-16.0-crate::island::layout::bar::TAIL_W;
    let layout=&h.app.resident.target;
    let slots=layout.x.iter().filter(|x|x.is_some()).count();
    let natural=layout.x.iter().zip(&layout.widths).filter_map(|(x,w)|x.map(|_|*w)).sum::<f32>()
        +2.0*crate::island::layout::bar::EDGE_PAD+crate::island::layout::bar::GAP*(slots-1) as f32;
    assert!(natural>available,"用例应触发间距压缩：自然宽 {natural}，可用 {available}");
    assert!(layout.width<=available+0.01,"压缩间距后应放进状态行：{} <= {available}",layout.width);
    assert!(h.app.resident.position(crate::island::layout::bar::BATTERY).is_finite());
}

#[test]
fn tray_mail_shows_on_new_mail_and_opens_the_mail_page(){
    use crate::island::layout::bar::{MAIL,TRAY};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_mail=true;
    for _ in 0..20{h.frame(vec![]);}
    assert!(h.app.resident.current().x[TRAY].is_none(),"没有邮件事件时容器不占位");
    add_unread_mail(&mut h);
    for _ in 0..5{h.frame(vec![]);}
    assert_eq!(h.app.tray_active(),Some(crate::island::event_tray::Item::Mail));
    assert!(h.app.resident.current().x[TRAY].is_some(),"新邮件时容器占位");
    assert!(!slots(&h).contains(&MAIL),"邮箱进容器后行内不再显示徽标");
    h.click(badge_rect(&h,MAIL).center());
    assert_eq!(h.app.page(),crate::island::MAIL_PAGE,"点容器里的信封打开邮箱页");
}

#[test]
fn page_badge_settings_round_trip(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.show_mail=false;
    h.app.settings.show_health=false;
    h.app.settings.show_todo=false;
    h.app.settings.show_shelf=false;
    h.app.settings.show_calendar=false;
    h.app.settings.show_dnd=false;
    h.app.settings.save();
    let loaded=crate::model::Settings::load();
    assert!(!loaded.show_mail && !loaded.show_health && !loaded.show_todo && !loaded.show_shelf && !loaded.show_calendar && !loaded.show_dnd);
    assert!(crate::model::Settings::default().show_mail && crate::model::Settings::default().show_dnd,"新开关默认开启");
    h.app.settings.show_mail=true;h.app.settings.save();
    assert!(crate::model::Settings::load().show_mail);
}

#[test]
fn tray_members_can_be_reordered_by_dragging(){
    use crate::island::layout::bar::{FOCUS,QUOTA,WEATHER};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(h:&mut H,events:Vec<egui::Event>)->(Pos2,Pos2,Rect){
        let mut out=(Pos2::ZERO,Pos2::ZERO,Rect::NOTHING);
        h.t+=0.02;
        let frame=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,300.0))),time:Some(h.t),events,..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                h.app.resident_order_settings(ui);
                out.0=ui.ctx().read_response(ui.id().with(("resident-icon",WEATHER))).expect("天气在容器里").rect.center();
                let focus=ui.ctx().read_response(ui.id().with(("resident-icon",FOCUS))).expect("专注在容器里").rect;
                out.1=focus.center();out.2=focus;
            });
        });
        h.shapes=frame.shapes;
        out
    }
    let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    let mut h=H::new();
    draw(&mut h,vec![]);let (weather,focus,_)=draw(&mut h,vec![]);
    draw(&mut h,vec![egui::Event::PointerMoved(weather),button(weather,true)]);
    draw(&mut h,vec![egui::Event::PointerMoved(weather+egui::vec2(0.0,10.0))]);
    let target=focus+egui::vec2(12.0,0.0);
    let (_,_,focus_rect)=draw(&mut h,vec![egui::Event::PointerMoved(target)]);
    let line=h.shapes.iter().find_map(|s|match &s.shape{
        egui::Shape::LineSegment{points,..} if (points[0].x-points[1].x).abs()<0.5 && (points[0].y-points[1].y).abs()>10.0=>Some(points[0].x),
        _=>None,
    });
    assert!(line.is_some_and(|x|(x-focus_rect.right()).abs()<1.5),"拖动时应在目标成员右侧画竖线：{line:?} / {:?}",focus_rect.right());
    for _ in 0..3{draw(&mut h,vec![]);}
    draw(&mut h,vec![button(target,false)]);
    assert_eq!(&h.app.settings.tray_order[..3],[QUOTA,FOCUS,WEATHER],"容器内拖动后按目标位置排序");
}

#[test]
fn tray_mail_stays_until_the_mail_is_read(){
    use crate::island::event_tray::{Item,HOLD};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_mail=true;
    add_unread_mail(&mut h);
    for _ in 0..5{h.frame(vec![]);}
    assert_eq!(h.app.tray_active(),Some(Item::Mail));
    h.t+=HOLD+5.0;
    for _ in 0..5{h.frame(vec![]);}
    assert_eq!(h.app.tray_active(),Some(Item::Mail),"未读邮件在事件容器里持续到已读");
    badge_rect(&h,crate::island::layout::bar::MAIL);
    h.app.mail.recent.clear();
    for _ in 0..5{h.frame(vec![]);}
    assert_eq!(h.app.tray_active(),None,"已读后事件图标消失");
    assert!(h.ctx.read_response(egui::Id::new(("card",true,("page-badge",crate::island::layout::bar::MAIL)))).is_none(),"已读后事件图标不再绘制");
}

#[test]
fn tray_event_starts_counting_after_the_banner(){
    use crate::island::event_tray::{Item,HOLD};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_shelf=true;
    for _ in 0..20{h.frame(vec![]);}
    h.app.alert("系统音量","50%",0,false);h.app.tick_alerts();
    assert!(h.app.notice.is_some());
    h.app.shelf_badge_until=h.app.t+10.0;
    for _ in 0..5{h.frame(vec![]);}
    assert!(!h.app.tray_active_items().contains(&Item::Shelf),"横幅期间不开始计时");
    h.app.t+=3.0;h.app.tick_alerts();
    assert!(h.app.notice.is_none());
    for _ in 0..3{h.frame(vec![]);}
    assert!(h.app.tray_active_items().contains(&Item::Shelf),"横幅结束后开始显示");
    h.t+=HOLD-1.0;
    for _ in 0..3{h.frame(vec![]);}
    assert!(h.app.tray_active_items().contains(&Item::Shelf),"计时从横幅结束后开始");
    h.t+=2.0;
    for _ in 0..3{h.frame(vec![]);}
    assert!(!h.app.tray_active_items().contains(&Item::Shelf));
}

#[test]
fn tray_focus_opens_the_focus_page(){
    use crate::island::FOCUS_PAGE;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_focus=true;
    for _ in 0..10{h.frame(vec![]);}
    h.app.pomo.running=true;
    for _ in 0..5{h.frame(vec![]);}
    let rect=h.ctx.read_response(egui::Id::new(("card",true,"tray-focus"))).expect("专注应在事件容器里").rect;
    h.click(rect.center());
    assert_eq!(h.app.page,FOCUS_PAGE);
}

#[test]
fn calendar_badge_lasts_until_the_meeting_starts(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.t=10.0;h.app.t=10.0;
    let start=chrono::Local::now().timestamp()+30;
    h.app.meeting_alert(crate::mail::Meeting{all_day:false,title:"站会".into(),start,end:None,at:String::new(),uid:"badge-until-start".into()},"本机日程".into(),7);
    assert!(h.app.calendar_badge_until>=h.app.t+25.0,"会议徽标保留到会议开始");
    assert!(slots(&h).contains(&crate::island::layout::bar::CALENDAR));
    h.t+=31.0;
    h.frame(vec![]);
    assert!(!slots(&h).contains(&crate::island::layout::bar::CALENDAR),"会议开始后徽标消失");
}

#[test]
fn tray_calendar_shows_on_calendar_reminder_and_opens_the_calendar_page(){
    use crate::island::layout::bar::{CALENDAR,TRAY};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_calendar=true;
    for _ in 0..20{h.frame(vec![]);}
    assert!(h.app.resident.current().x[TRAY].is_none(),"没有日程事件时容器不占位");
    h.app.calendar_badge_until=h.app.t+crate::island::page_badges::HOLD;
    for _ in 0..5{h.frame(vec![]);}
    assert_eq!(h.app.tray_active(),Some(crate::island::event_tray::Item::Calendar));
    assert!(h.app.resident.current().x[TRAY].is_some(),"日程提醒时容器占位");
    assert!(!slots(&h).contains(&CALENDAR),"日程进容器后行内不再显示徽标");
    h.click(badge_rect(&h,CALENDAR).center());
    assert_eq!(h.app.page(),crate::island::CALENDAR_PAGE,"点容器里的日历打开日程页");
}

#[test]
fn dragging_outside_the_preview_keeps_the_insertion_line(){
    use crate::island::layout::bar::{CLOCK,WEATHER};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(h:&mut H,events:Vec<egui::Event>)->(Pos2,Pos2){
        let mut out=(Pos2::ZERO,Pos2::ZERO);
        h.t+=0.02;
        let frame=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,300.0))),time:Some(h.t),events,..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                h.app.resident_order_settings(ui);
                out.0=ui.ctx().read_response(ui.id().with(("resident-icon",WEATHER))).expect("天气在容器里").rect.center();
                out.1=ui.ctx().read_response(ui.id().with(("resident-icon",CLOCK))).expect("时钟在常驻行").rect.center();
            });
        });
        h.shapes=frame.shapes;
        out
    }
    let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    let mut h=H::new();h.app.settings.pet=false;
    draw(&mut h,vec![]);let (weather,_)=draw(&mut h,vec![]);
    draw(&mut h,vec![egui::Event::PointerMoved(weather),button(weather,true)]);
    draw(&mut h,vec![egui::Event::PointerMoved(weather+egui::vec2(0.0,10.0))]);
    let outside=weather+egui::vec2(0.0,150.0);
    draw(&mut h,vec![egui::Event::PointerMoved(outside)]);
    let line=h.shapes.iter().find_map(|s|match &s.shape{
        egui::Shape::LineSegment{points,..} if (points[0].x-points[1].x).abs()<0.5 && (points[0].y-points[1].y).abs()>10.0=>Some(points[0].x),
        _=>None,
    });
    assert!(line.is_some(),"指针移出岛体后仍应画插入竖线");
    draw(&mut h,vec![button(outside,false)]);
    assert!(!h.app.settings.tray_weather,"拖出岛体松手后离开容器");
}

#[test]
fn island_preview_keeps_page_icons_out_of_the_row(){
    use crate::island::layout::bar::{CALENDAR,DND,HEALTH,MAIL,SHELF,TODO};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(h:&mut H,events:Vec<egui::Event>)->(Option<Pos2>,Option<Rect>) {
        let mut moon=None;let mut mail=None;
        h.t+=0.02;
        h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,300.0))),time:Some(h.t),events,..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                h.app.resident_order_settings(ui);
                for slot in [MAIL,HEALTH,TODO,SHELF,CALENDAR] {
                    assert!(ui.ctx().read_response(ui.id().with(("page-badge-preview",slot))).is_none(),"页面图标不画在常驻行");
                }
                moon=ui.ctx().read_response(ui.id().with(("page-badge-preview",DND))).map(|response|response.rect.center());
                mail=ui.ctx().read_response(ui.id().with(("resident-icon",MAIL))).map(|response|response.rect);
            });
        });
        (moon,mail)
    }
    let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    let mut h=H::new();h.app.dnd=crate::island::dnd::Dnd::fake(false);
    draw(&mut h,vec![]);
    let (moon,mail)=draw(&mut h,vec![]);
    assert!(mail.is_none(),"页面徽标不在常驻行里重复画");
    let moon=moon.expect("勿扰月亮仍在预览里");
    draw(&mut h,vec![egui::Event::PointerMoved(moon)]);
    draw(&mut h,vec![button(moon,true)]);
    draw(&mut h,vec![button(moon,false)]);
    assert!(h.app.dnd.active,"点击预览里的月亮切换勿扰");
    h.app.settings.tray_mail=true;
    draw(&mut h,vec![]);
    assert!(draw(&mut h,vec![]).1.is_some(),"进入事件容器后按成员显示");
    h.app.settings.tray_mail=false;
    draw(&mut h,vec![]);
    assert!(draw(&mut h,vec![]).1.is_none(),"退出事件容器后不在常驻行补画");
}

#[test]
fn island_preview_fits_every_enabled_component_in_a_narrow_settings_page(){
    use crate::island::layout::bar::{AGENT,BATTERY,CLOCK,DND,FOCUS,MUSIC,PET,QUOTA,WEATHER};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.show_battery=true;
    h.app.settings.show_clock=true;
    h.app.settings.tray_weather=false;h.app.settings.tray_quota=false;h.app.settings.tray_focus=false;
    h.app.settings.tray_music=false;h.app.settings.tray_battery=false;
    let mut inside=true;
    h.t+=0.02;
    h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(360.0,300.0))),time:Some(h.t),..Default::default()},|ctx|{
        egui::CentralPanel::default().show(ctx,|ui|{
            let page=ui.available_rect_before_wrap();
            h.app.resident_order_settings(ui);
            for slot in [WEATHER,QUOTA,FOCUS,MUSIC,BATTERY,CLOCK,PET,AGENT] {
                let rect=ui.ctx().read_response(ui.id().with(("resident-icon",slot))).unwrap_or_else(||panic!("预览缺少组件 {slot}")).rect;
                inside&=page.contains_rect(rect);
            }
            let moon=ui.ctx().read_response(ui.id().with(("page-badge-preview",DND))).expect("预览缺少勿扰月亮").rect;
            inside&=page.contains_rect(moon);
        });
    });
    assert!(inside,"预览必须完整显示所有已开启组件，不能被裁掉");
}
