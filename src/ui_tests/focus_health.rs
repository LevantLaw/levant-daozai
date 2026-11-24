//! 专注、健康提醒与吉祥物。
use super::*;

#[test]
fn quality_distinct_reminders_are_queued_instead_of_overwritten() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let meeting=crate::mail::Meeting{ all_day:false,title:"会议一".into(),start:chrono::Local::now().timestamp()+600,end:None,at:String::new(),uid:"one".into()};
    h.app.meeting_alert(meeting.clone(),"飞书".into(),7);
    h.app.meeting_alert(crate::mail::Meeting{ all_day:false,title:"会议二".into(),uid:"two".into(),..meeting.clone()},"飞书".into(),7);
    assert_eq!(h.app.alerts.len(),2,"同时到期的两场会议都需要提醒");
    h.app.meeting_alert(meeting,"飞书".into(),7);assert_eq!(h.app.alerts.len(),2,"同一会议重复更新应合并");
    h.app.alert("待办到时间了","任务一",4,true);h.app.alert("待办到时间了","任务二",4,true);
    assert_eq!(h.app.alerts.iter().filter(|a|a.title=="待办到时间了").count(),2);
    h.app.alert("系统音量","10%",0,false);h.app.alert("系统音量","20%",0,false);
    assert_eq!(h.app.alerts.iter().filter(|a|a.title=="系统音量").count(),1);
}

#[test]
fn island_position_drag_tracks_screen_coordinates_and_saves_on_release(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let bounds=crate::os::display_layout::Bounds{left:-2560,top:-400,right:0,bottom:1040};
    for dock in crate::model::Dock::ALL {for ppp in [1.0,1.5] {
        let mut h=H::new();h.app.settings.dock=dock;h.app.settings.island_offset=-40;h.app.settings.save();
        // 起点离居中足够远，90 点位移不会落进居中吸附范围。
        let start=island_drag_header(&mut h,bounds,ppp);
        let before=island_drag_frame(&mut h,bounds,ppp,Some(start),Some(true));
        let delta=if dock.is_side(){Vec2::new(0.0,90.0)}else{Vec2::new(90.0,0.0)};
        let end=start+delta;
        island_drag_frame(&mut h,bounds,ppp,Some(end),None);
        assert!(h.app.settings.island_offset>-40,"{dock:?} / {ppp}：拖动应同步现有位置参数");
        assert_eq!(crate::model::Settings::load().island_offset,-40,"拖动途中不能反复写配置");
        let offset=h.app.settings.island_offset;
        for _ in 0..30{island_drag_frame(&mut h,bounds,ppp,Some(end),None);}
        let after=island_drag_frame(&mut h,bounds,ppp,Some(end),None);
        let movement=before.min-after.min;
        assert!(((if dock.is_side(){movement.y}else{movement.x})-90.0).abs()<6.0,"{dock:?} / {ppp}：窗口应跟随实际鼠标位移，允许整数位置刻度带来的取整误差");
        assert_eq!(if dock.is_side(){movement.x}else{movement.y},0.0,"拖动不能脱离停靠边缘");
        assert_eq!(h.app.settings.island_offset,offset,"窗口跟随鼠标移动后，原地持有不能发生累积漂移");
        island_drag_frame(&mut h,bounds,ppp,Some(end),Some(false));
        assert_eq!(crate::model::Settings::load().island_offset,offset,"松手后应保存位置");
        assert_eq!((h.app.settings.dock,h.app.settings.display_id.as_str()),(dock,""));
    }}
}

#[test]
fn island_position_drag_clamps_to_existing_limits_and_stays_resident(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let bounds=crate::os::display_layout::Bounds{left:0,top:0,right:1920,bottom:1080};
    for dock in crate::model::Dock::ALL {for shape in crate::model::IslandShape::ALL {
        let mut h=H::new();h.app.settings.dock=dock;h.app.settings.island_shape=shape;
        let start=island_drag_header(&mut h,bounds,1.0);
        island_drag_frame(&mut h,bounds,1.0,Some(start),Some(true));
        let end=if dock.is_side(){Pos2::new(start.x,3000.0)}else{Pos2::new(1855.0,start.y)};
        for _ in 0..120{island_drag_frame(&mut h,bounds,1.0,Some(end),None);}
        assert!(!h.app.open,"{dock:?} 拖动期间应保持常驻形态");
        assert!(h.app.progress()<0.01,"拖动期间不应保留展开面板");
        assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="10月28日 周三")),"拖动期间不应显示展开顶栏");
        assert_eq!(h.app.settings.island_offset,crate::model::ISLAND_OFFSET_MAX);
        let end=if dock.is_side(){Pos2::new(start.x,65.0)}else{Pos2::new(65.0,start.y)};
        island_drag_frame(&mut h,bounds,1.0,Some(end),None);
        assert_eq!(h.app.settings.island_offset,-crate::model::ISLAND_OFFSET_MAX);
        assert_eq!(h.app.settings.dock,dock);
        island_drag_frame(&mut h,bounds,1.0,Some(end),Some(false));
        let offset=h.app.settings.island_offset;
        for _ in 0..40{island_drag_frame(&mut h,bounds,1.0,Some(start),None);}
        assert_eq!(h.app.settings.island_offset,offset,"松手后不能继续跟着鼠标移动");
    }}
}

#[test]
fn island_position_drag_below_a_top_bar_switches_edges_like_the_bare_screen(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    use crate::model::{Dock,Settings};
    let monitor=crate::os::display_layout::Bounds{left:0,top:0,right:1920,bottom:1080};
    let display=crate::os::display_layout::Display{id:"d".into(),label:"d".into(),bounds:monitor,work_top:28,primary:true};
    let bounds=crate::os::display_layout::placement_bounds(&display,true);
    for to in [Dock::Left,Dock::Right] {
        let mut h=H::new();h.app.settings.save();
        let start=island_drag_header(&mut h,bounds,1.0);
        island_drag_frame(&mut h,bounds,1.0,Some(start),Some(true));
        let size=Vec2::new(1920.0,1052.0);
        let end=if to==Dock::Left{Pos2::new(0.0,size.y*0.6)}else{Pos2::new(size.x,size.y*0.6)};
        for _ in 0..30{island_drag_frame(&mut h,bounds,1.0,Some(end),None);}
        assert_eq!(h.app.settings.dock,to);
        let back=Pos2::new(size.x*0.5,0.0);
        for _ in 0..30{island_drag_frame(&mut h,bounds,1.0,Some(back),None);}
        assert_eq!(h.app.settings.dock,Dock::Top,"拖回顶栏下沿应能回到顶部");
        island_drag_frame(&mut h,bounds,1.0,Some(back),Some(false));
        assert_eq!(Settings::load().dock,Dock::Top);
        assert!(Settings::load().avoid_top_bar,"拖动不改变避开顶栏设置");
    }
}

#[test]
fn island_position_drag_switches_between_all_three_edges_and_saves_on_release(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    use crate::model::{Dock,IslandShape,Settings};
    let bounds=crate::os::display_layout::Bounds{left:-2560,top:-400,right:0,bottom:1040};
    for from in Dock::ALL {for to in Dock::ALL.into_iter().filter(|to|*to!=from) {for ppp in [1.0,1.5] {for shape in IslandShape::ALL {
        let mut h=H::new();h.app.settings.dock=from;h.app.settings.island_shape=shape;h.app.settings.save();
        let start=island_drag_header(&mut h,bounds,ppp);
        island_drag_frame(&mut h,bounds,ppp,Some(start),Some(true));
        let size=Vec2::new(2560.0,1440.0)/ppp;
        let end=match to {Dock::Top=>Pos2::new(size.x*0.6,0.0),Dock::Left=>Pos2::new(0.0,size.y*0.6),Dock::Right=>Pos2::new(size.x,size.y*0.6)};
        for _ in 0..30{island_drag_frame(&mut h,bounds,ppp,Some(end),None);}
        assert_eq!(h.app.settings.dock,to,"{from:?} → {to:?} / {ppp} / {shape:?}");
        assert!(!h.app.open && h.app.progress()<0.01,"切换停靠边后仍应显示常驻形态");
        assert_eq!(Settings::load().dock,from,"拖动中不能写入停靠设置");
        let offset=h.app.settings.island_offset;
        for _ in 0..15{island_drag_frame(&mut h,bounds,ppp,Some(end),None);}
        assert_eq!(h.app.settings.island_offset,offset,"换边后持有不能漂移");
        island_drag_frame(&mut h,bounds,ppp,Some(end),Some(false));
        let saved=Settings::load();assert_eq!((saved.dock,saved.island_offset),(to,offset));
        for _ in 0..30{island_drag_frame(&mut h,bounds,ppp,Some(end),None);}
        assert!(!h.app.open,"松手后应留在常驻形态");
        let screen=island_drag_frame(&mut h,bounds,ppp,Some(end),None);
        let resident=h.hit.center()-screen.min.to_vec2();
        for _ in 0..30{island_drag_frame(&mut h,bounds,ppp,Some(resident),None);}
        assert!(!h.app.open,"松手后光标留在岛上不能立即展开");
        for _ in 0..30{island_drag_frame(&mut h,bounds,ppp,None,None);}
        for _ in 0..60{island_drag_frame(&mut h,bounds,ppp,Some(resident),None);}
        assert!(h.app.open,"光标离开再悬停，应能正常展开");
        assert_eq!(h.app.settings.display_id,"");
    }}}}
}

#[test]
fn island_position_drag_keeps_the_edge_in_the_middle_and_does_not_flap_at_corners(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    use crate::model::Dock;
    let bounds=crate::os::display_layout::Bounds{left:0,top:0,right:1920,bottom:1080};
    let mut h=H::new();let start=island_drag_header(&mut h,bounds,1.0);
    island_drag_frame(&mut h,bounds,1.0,Some(start),Some(true));
    for (point,dock) in [(Pos2::new(500.0,500.0),Dock::Top),(Pos2::new(0.0,500.0),Dock::Left),
        (Pos2::new(960.0,540.0),Dock::Left),(Pos2::new(1250.0,540.0),Dock::Right),(Pos2::new(1000.0,540.0),Dock::Right),
        (Pos2::new(700.0,540.0),Dock::Left),(Pos2::new(4.0,3.0),Dock::Left),
        (Pos2::new(3.0,4.0),Dock::Left),(Pos2::new(100.0,0.0),Dock::Top),
        (Pos2::new(1920.0,500.0),Dock::Right),(Pos2::new(1918.0,3.0),Dock::Right),
        (Pos2::new(1500.0,0.0),Dock::Top)] {
        for _ in 0..5{island_drag_frame(&mut h,bounds,1.0,Some(point),None);}
        assert_eq!(h.app.settings.dock,dock,"光标={point:?}");
        assert!(!h.app.open);
    }
    island_drag_frame(&mut h,bounds,1.0,None,Some(false));
    island_drag_frame(&mut h,bounds,1.0,Some(start),Some(false));
    let dock=h.app.settings.dock;let offset=h.app.settings.island_offset;
    island_drag_frame(&mut h,bounds,1.0,Some(Pos2::new(0.0,500.0)),None);
    assert_eq!((h.app.settings.dock,h.app.settings.island_offset),(dock,offset),"松手后不能继续拖动");
}

#[test]
fn island_position_drag_lifts_on_hold_glides_smoothly_and_snaps_to_center(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let bounds=crate::os::display_layout::Bounds{left:0,top:0,right:1920,bottom:1080};
    let mut h=H::new();h.app.settings.island_offset=-30;h.app.settings.save();
    let start=island_drag_header(&mut h,bounds,1.0);
    island_drag_frame(&mut h,bounds,1.0,Some(start),Some(true));
    // 原地按住片刻即拿起：面板开始收起，位置不变。
    for _ in 0..20{island_drag_frame(&mut h,bounds,1.0,Some(start),None);}
    assert!(!h.app.open && h.app.progress()<0.5,"按住不动也应拿起岛体");
    assert_eq!(h.app.settings.island_offset,-30);
    // 窗口平滑跟随：一帧跳过去的光标，窗口要分几帧追上，且不按整数刻度跳。
    let travel=1920.0/2.0-crate::os::display_layout::ISLAND_HALF_EXTENT;
    let to=|units:f32|Pos2::new(start.x+(units+30.0)/100.0*travel,start.y);
    island_drag_frame(&mut h,bounds,1.0,Some(to(-10.0)),None);
    island_drag_frame(&mut h,bounds,1.0,Some(to(-10.0)),None);
    let mid=h.app.placement_offset();
    assert!(mid> -0.30 && mid< -0.10,"窗口应在途中，而不是瞬间到位：{mid}");
    for _ in 0..40{island_drag_frame(&mut h,bounds,1.0,Some(to(-10.0)),None);}
    assert!((h.app.placement_offset()+0.10).abs()<0.003,"持有后应追上光标");
    // 居中附近吸附，离开吸附范围后恢复跟随。
    assert!(h.app.center_guide>0.2 && h.app.center_guide<0.9 && h.app.center_lock_a<0.05,"接近居中时准线渐显，但还没点亮：{}",h.app.center_guide);
    island_drag_frame(&mut h,bounds,1.0,Some(to(2.0)),None);
    assert_eq!(h.app.settings.island_offset,0,"靠近居中应吸附");
    assert!(h.app.center_locked,"吸附后应标记为已居中");
    for _ in 0..30{island_drag_frame(&mut h,bounds,1.0,Some(to(2.0)),None);}
    assert!(h.app.center_guide>0.9 && h.app.center_lock_a>0.9,"居中时准线应清晰可见并切换为已居中配色");
    // 吸住后有滞回：稍微拖出一点仍停在中间，拖得更远才松开。
    island_drag_frame(&mut h,bounds,1.0,Some(to(7.0)),None);
    assert_eq!(h.app.settings.island_offset,0,"居中后轻微偏移不应松开");
    island_drag_frame(&mut h,bounds,1.0,Some(to(12.0)),None);
    assert_eq!(h.app.settings.island_offset,12);
    assert!(!h.app.center_locked);
    island_drag_frame(&mut h,bounds,1.0,Some(to(-28.5)),None);
    assert_eq!(h.app.settings.island_offset,-30,"靠近原位置应吸附回去");
    island_drag_frame(&mut h,bounds,1.0,Some(to(2.0)),None);
    island_drag_frame(&mut h,bounds,1.0,Some(to(2.0)),Some(false));
    assert_eq!(crate::model::Settings::load().island_offset,0,"松手保存吸附后的位置");
    for _ in 0..90{island_drag_frame(&mut h,bounds,1.0,Some(to(2.0)),None);}
    assert_eq!(h.app.placement_offset(),0.0,"落位后回到保存的位置");
}

#[test]
fn island_position_drag_ignores_jitter_and_preserves_header_buttons(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let bounds=crate::os::display_layout::Bounds{left:0,top:0,right:1920,bottom:1080};
    let mut h=H::new();let start=island_drag_header(&mut h,bounds,1.0);
    island_drag_frame(&mut h,bounds,1.0,Some(start),Some(true));
    island_drag_frame(&mut h,bounds,1.0,Some(start+Vec2::splat(1.0)),None);
    island_drag_frame(&mut h,bounds,1.0,Some(start+Vec2::splat(1.0)),Some(false));
    assert_eq!(h.app.settings.island_offset,0,"轻微手抖不能改变位置");
    let screen=island_drag_frame(&mut h,bounds,1.0,Some(start),None);
    let gear=settings_button(&h)-screen.min.to_vec2();
    island_drag_frame(&mut h,bounds,1.0,Some(gear),Some(true));
    island_drag_frame(&mut h,bounds,1.0,Some(gear),Some(false));
    assert!(h.app.settings_open,"时间区域拖动不能拦截设置按钮");
    assert_eq!(h.app.settings.island_offset,0);
    h.app.settings_open=false;
    for (page,x,expected) in [(3,330.0,crate::island::Cmd::AddFiles),(6,319.0,crate::island::Cmd::QuotaRefresh)] {
        h.app.show_page(page);
        for _ in 0..60{island_drag_frame(&mut h,bounds,1.0,Some(start),None);}
        let screen=island_drag_frame(&mut h,bounds,1.0,Some(start),None);
        let point=H::local(x,18.0)-screen.min.to_vec2();
        island_drag_frame(&mut h,bounds,1.0,Some(point),Some(true));
        island_drag_frame(&mut h,bounds,1.0,Some(point),Some(false));
        assert!(h.cmds.contains(&expected),"顶部操作按钮应正常响应");
        assert_eq!(h.app.settings.island_offset,0);
    }
}

#[test]
fn browser_minimal_preserves_preferences_and_restores_after_leaving_the_browser(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=true;h.browser_at_top=Some(true);
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.hit.height()>20.0,"开关默认关闭时不应自动改变模式");
    h.app.settings.browser_minimal=true;h.app.settings.save();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]当前歌词"));
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.hit.height()<10.0,"浏览器顶部受遮挡时应收为细刘海，歌词和吉祥物不能撑大它");
    assert!(h.app.settings.pet,"自动避让不能修改吉祥物设置");
    let saved=crate::model::Settings::load();
    assert!(saved.browser_minimal && saved.pet,"应只持久化开关，保留吉祥物设置");
    h.browser_at_top=None;for _ in 0..30 {h.frame(vec![]);}
    assert!(h.hit.height()<10.0,"打开岛仔设置或托盘时应保留浏览器避让状态");
    h.browser_at_top=Some(false);for _ in 0..90 {h.frame(vec![]);}
    assert!(h.app.pet_enabled() && h.hit.height()>20.0,"离开浏览器或还原窗口后应恢复");
    h.browser_at_top=Some(true);for _ in 0..90 {h.frame(vec![]);}
    h.app.settings.browser_minimal=false;for _ in 0..90 {h.frame(vec![]);}
    assert!(h.app.pet_enabled() && h.hit.height()>20.0,"关闭开关后应立即恢复");
}

#[test]
fn wheel_page_transition_keeps_interactive_frames_after_input_stops() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.hover(H::local(210.0,18.0),0.3);
    assert!(!h.page_transition);
    h.wheel(-40.0);
    assert_eq!(h.app.page,1);
    assert!(h.page_transition);
    for _ in 0..18 {
        h.frame(vec![]);
        assert!(h.page_transition,"松开滚轮后，仍然可见的翻页动画应继续逐帧显示");
    }
    for _ in 0..24 {h.frame(vec![]);}
    assert!(!h.page_transition,"翻页结束后应恢复原刷新策略");
}

#[test]
fn timer_counts_elapsed_time_after_sleep() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.frame(vec![]);
    h.app.pomo.running = true;
    h.app.pomo.remaining = 120.0;
    h.t += 60.0;
    h.frame(vec![]);
    assert!(h.app.pomo.remaining < 61.0, "挂起恢复后必须补算实际流逝时间");
}

#[test]
fn reminders_are_visible_while_collapsed_and_can_be_disabled(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.t+=2701.0;h.frame(vec![]);assert!(h.app.notice.is_some());assert!(h.cmds.iter().any(|c|matches!(c,crate::island::Cmd::Notify(t,..)if t.contains("喝水"))));
    h.app.water.remind=false;h.app.settings.sit_remind=false;h.app.notice=None;h.cmds.clear();h.t+=2800.0;h.frame(vec![]);assert!(h.app.notice.is_none());
}

#[test]
fn water_serving_and_undo_are_consistent(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(2,1.0);h.hover(H::local(300.0,60.0),0.8);
    h.app.settings.serving_ml=350;h.click(H::local(70.0,110.0));assert_eq!(h.app.water.drunk_ml,350);assert_eq!(h.app.water.hours[9],350);
    h.click(H::local(191.0,110.0));assert_eq!(h.app.water.drunk_ml,0);assert_eq!(h.app.water.hours[9],0);
}

#[test]
fn mascot_rest_and_focus_have_distinct_states(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.opts.pet_idle=45.0;h.frame(vec![]);
    assert_eq!(h.app.pet_state(),crate::island::extras::PetState::Awake);
    h.t+=50.0;h.frame(vec![]);assert_eq!(h.app.pet_state(),crate::island::extras::PetState::Dozing);
    h.t+=140.0;h.frame(vec![]);assert_eq!(h.app.pet_state(),crate::island::extras::PetState::Sleeping);
    h.app.pomo.running=true;assert_eq!(h.app.pet_state(),crate::island::extras::PetState::Focused);
}

#[test]
fn singing_microphone_moves_relative_to_the_face_and_disappears_on_pause(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.pet=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    for _ in 0..120 {h.frame(vec![]);}
    let head=|shapes:&[egui::epaint::ClippedShape]|shapes.iter().find_map(|s|match &s.shape {
        egui::Shape::Circle(circle) if (circle.radius-2.6).abs()<0.01=>Some(circle.center),_=>None,
    });
    let mut offsets=Vec::new();
    for _ in 0..120 {
        h.frame(vec![]);
        let head=head(&h.shapes).expect("演唱应显示球头麦克风");
        let eyes:Vec<_>=h.shapes.iter().filter_map(|s|match &s.shape {
            egui::Shape::Path(p) if p.points.len()==13 && (p.stroke.width-2.8).abs()<0.01=>Some(p.points.iter().map(|p|p.y).sum::<f32>()/13.0),_=>None,
        }).collect();
        assert_eq!(eyes.len(),2);offsets.push(head.y-(eyes[0]+eyes[1])/2.0);
    }
    if crate::island::bar_layout::animations_enabled(){
        let range=offsets.iter().copied().fold(f32::NEG_INFINITY,f32::max)-offsets.iter().copied().fold(f32::INFINITY,f32::min);
        assert!(range>0.4,"麦克风需要有独立动态");
    }
    h.media.playing=false;for _ in 0..90 {h.frame(vec![]);}
    assert!(head(&h.shapes).is_none(),"暂停后应收起麦克风");
}

#[test]
fn mascot_wakes_before_returning_to_idle(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.opts.pet_idle=45.0;
    h.frame(vec![]);h.t+=190.0;for _ in 0..60 {h.frame(vec![]);}
    assert_eq!(format!("{:?}",h.app.pet_state()),"Sleeping");
    h.cursor=Some(Pos2::new(550.0,100.0));h.frame(vec![egui::Event::PointerMoved(h.cursor.unwrap())]);
    assert_eq!(format!("{:?}",h.app.pet_state()),"Waking");
    for _ in 0..180 {h.frame(vec![]);}
    assert_eq!(format!("{:?}",h.app.pet_state()),"Awake");
}

#[test]
fn mascot_keeps_task_celebration_until_panel_closes(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.todos.items.push(crate::model::TodoItem{text:"完成测试任务".into(),done_on:None,due:None,reminded:false,..Default::default()});
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(4,1.0);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(250.0,47.0));
    assert!(h.app.todos.items[0].done_on.is_some());
    h.hover(H::local(300.0,60.0),4.0);assert_eq!(format!("{:?}",h.app.pet_state()),"Happy");
    h.cursor=None;for _ in 0..240 {h.frame(vec![]);}
    assert_eq!(format!("{:?}",h.app.pet_state()),"Awake");
}

#[test]
fn mascot_drink_nods_and_focus_completion_celebrates(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);h.app.drink_water();
    assert_eq!(format!("{:?}",h.app.pet_state()),"Satisfied");
    for _ in 0..240 {h.frame(vec![]);}
    h.app.pomo.running=true;h.app.pomo.remaining=0.01;h.frame(vec![]);
    assert_eq!(format!("{:?}",h.app.pet_state()),"Happy");
}

#[test]
fn mascot_system_idle_wakes_once_then_follows_pointer(){
    use crate::island::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.opts.pet_idle=45.0;
    h.system_idle=Some(200.0);h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Sleeping);
    h.system_idle=Some(0.0);h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Waking);
    for _ in 0..180 {h.frame(vec![]);}assert_eq!(h.app.pet_state(),PetState::Awake);
    h.cursor=Some(Pos2::new(WIN_W/2.0,50.0));
    for _ in 0..60 {h.frame(vec![]);}assert_eq!(h.app.pet_state(),PetState::Curious);assert!(!h.app.open);
}

#[test]
fn idle_mascot_stays_static_until_hovered(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    for _ in 0..180 {h.frame(vec![]);}
    assert!(!h.animating,"空闲时吉祥物应停在静态姿势");
    let clock=h.app.pet.clock;
    h.frame(vec![]);
    assert_eq!(h.app.pet.clock,clock,"没有事件时吉祥物时钟不前进");
    h.cursor=Some(Pos2::new(WIN_W/2.0,50.0));
    h.frame(vec![]);
    assert!(h.animating,"悬停时吉祥物应开始动画");
    assert!(h.app.pet.clock>clock);
}

#[test]
#[ignore = "按需导出吉祥物的实际渲染画面"]
fn mascot_expression_gallery(){
    use crate::island::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::env::var_os("DAOZAI_PET_PREVIEW_DIR").map(std::path::PathBuf::from)
        .unwrap_or_else(||std::env::temp_dir().join(format!("daozai-pet-preview-{}",std::process::id())));
    std::fs::create_dir_all(&dest).unwrap();
    let states=[("awake",PetState::Awake),("curious",PetState::Curious),("waking",PetState::Waking),
        ("happy",PetState::Happy),("satisfied",PetState::Satisfied),("music",PetState::Music),
        ("worried",PetState::Worried),("focused",PetState::Focused),("dozing",PetState::Dozing),("sleeping",PetState::Sleeping)];
    for (name,state) in states {
        let mut h=H::new();h.app.pet.react(state);
        let mut raster=crate::preview_raster::Raster::new(1280,380);
        let last=if state==PetState::Waking{42}else{79};
        for frame in 0..=last {
            let t=frame as f64/60.0;
            let env=Env{t,clock:"09:41:00".into(),today:"2026-10-31".into(),date:"10月31日 周六".into(),..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H))),time:Some(t),
                viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{
                    let win=ui.max_rect();h.app.frame(ui,win,&env);
                });
            });
            raster.update_textures(&output.textures_delta);
            if frame==last {
                raster.buf.fill([0.09,0.11,0.18,1.0]);
                raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                assert!(rgba.chunks_exact(4).any(|p|p[0]>230&&p[1]>230&&p[2]>230),"表情画面不能空白：{name}");
                image::save_buffer(dest.join(format!("{name}.png")),&rgba,1280,380,image::ColorType::Rgba8).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }
    eprintln!("表情预览：{}",dest.display());
}

#[test]
#[ignore="按需导出接入程序后的十二组表情"]
fn mascot_integrated_expression_gallery(){
    use crate::island::pet_expressions::Expression;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_PET_INTEGRATED_PREVIEW").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    for (name,skin) in [("classic",crate::model::Skin::Classic),("cream",crate::model::Skin::Cream),("glass",crate::model::Skin::GlassLight)] {
        let mut h=H::new();h.app.settings.pet=true;h.app.set_minimal(false);h.app.settings.skin=skin;
        let mut raster=crate::preview_raster::Raster::new(1280,540);
        let mut sequence=0;
        for (index,expression) in Expression::ALL.into_iter().enumerate() {
            for frame in 0..=(expression.duration()*20.0).ceil() as usize {
                if frame==1 {h.app.pet.expressions.play(expression);}
                let t=sequence as f64/20.0;sequence+=1;
                let env=Env{t,clock:"09:41:00".into(),today:"2026-11-06".into(),date:"11月6日 周五".into(),idle_seconds:Some(0.0),..Default::default()};
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H))),time:Some(t),
                    viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
                let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{
                    let win=ui.max_rect();h.app.frame(ui,win,&env);
                });});
                raster.update_textures(&output.textures_delta);
                if frame>0 {
                    raster.buf.fill([0.95,0.96,0.98,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);let image=image::RgbaImage::from_raw(1280,540,rgba).unwrap();
                    image::imageops::crop_imm(&image,440,0,400,100).to_image().save(dest.join(format!("{name}-{index:02}-{frame:03}.png"))).unwrap();
                }
                raster.free_textures(&output.textures_delta);
            }
        }
    }
}

#[test]
fn minimal_takes_precedence_over_mascot(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.pet=true;h.app.set_minimal(true);
    for _ in 0..120 {h.frame(vec![]);}assert!(h.region.height()<20.0,"启用极简和吉祥物时必须显示细线");
    h.hover(Pos2::new(WIN_W/2.0,2.0),1.0);assert!(h.app.progress()>0.9,"极简线仍要能悬停展开");
}

#[test]
fn lunch_break_silences_water_and_sit_reminders(){
    use crate::island::SIT_TITLE;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.system_idle=Some(0.0);h.frame(vec![]);
    // 测试时钟固定在 09:41，把午休设成 09:30–10:00。
    h.app.settings.lunch_rest=true;h.app.settings.lunch_start=9*60+30;h.app.settings.lunch_end=10*60;
    h.app.water.remind=true;h.app.last_reminder=h.app.t-2.0*3600.0;h.app.sit_start=h.app.t-2.0*3600.0;
    h.frame(vec![]);assert!(h.app.notice.is_none(),"午休中不应弹喝水或久坐提醒");
    h.app.settings.lunch_rest=false;h.frame(vec![]);
    assert!(h.app.notice.is_some(),"关闭午休后应恢复提醒");
    h.app.notice=None;h.app.alerts.clear();h.app.water.remind=false;h.app.confirm_health_reminder(crate::island::extras::HealthReminder::Sit);h.app.sit_start=h.t-2.0*3600.0;
    h.app.settings.lunch_rest=true;h.app.settings.lunch_start=10*60;h.app.settings.lunch_end=13*60;
    h.frame(vec![]);assert!(h.app.notice.as_ref().is_some_and(|a|a.title==SIT_TITLE),"午休以外照常提醒");
}

#[test]
fn health_time_subpage_edits_lunch_and_backs_out_one_level(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.app.open_settings_category(1);for _ in 0..30{h.frame(vec![]);}
    h.hover(H::local(300.0,60.0),0.3);h.click(H::local(200.0,113.0));assert!(h.app.health_time,"点提醒时段行进入二级页");
    let start=h.app.settings.lunch_start;h.hover(H::local(300.0,60.0),0.3);h.click(H::local(225.0,91.0));assert_eq!(h.app.settings.lunch_start,start+15);
    h.hover(H::local(300.0,60.0),0.3);h.click(H::local(353.0,69.0));assert!(!h.app.settings.lunch_rest,"午休开关可关闭");
    h.hover(H::local(300.0,60.0),0.3);h.click(H::local(20.0,18.0));assert!(!h.app.health_time);assert_eq!(h.app.subpage,Some(2),"返回先回到健康提醒设置");
}

#[test]
fn sit_reminder_waits_for_confirmation_before_restarting(){
    use crate::island::SIT_TITLE;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.system_idle=Some(0.0);h.frame(vec![]);h.app.water.remind=false;
    h.t+=59.0*60.0;h.frame(vec![]);assert!(h.app.notice.is_none(),"未满 60 分钟不提醒: {:?}",h.app.notice.as_ref().map(|a|(a.title.clone(),a.body.clone())));
    h.t+=61.0;h.frame(vec![]);
    assert!(h.app.notice.as_ref().is_some_and(|a|a.title==SIT_TITLE));
    assert!(h.cmds.iter().any(|c|matches!(c,crate::island::Cmd::Notify(t,..)if t==SIT_TITLE)));
    let count=h.app.history.len();
    h.screen_locked=true;h.system_idle=Some(40.0*60.0);h.t+=500.0*60.0;h.frame(vec![]);
    assert!(h.app.notice.is_none());assert_eq!(h.app.history.len(),count,"锁屏和长时间未处理都只保留一次提醒");
    h.app.confirm_health_reminder(crate::island::extras::HealthReminder::Sit);assert_eq!(h.app.sat_minutes(),0);
    h.screen_locked=false;h.system_idle=Some(0.0);h.t+=59.0*60.0;h.frame(vec![]);assert!(h.app.notice.is_none());
    h.t+=61.0;h.frame(vec![]);assert!(h.app.notice.as_ref().is_some_and(|a|a.title==SIT_TITLE));
}

#[test]
fn focus_completion_offers_starting_the_break(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);h.app.pomo.running=true;h.app.pomo.remaining=1.0;h.t+=2.0;h.frame(vec![]);
    assert!(h.app.pomo.phase==crate::island::Phase::Rest);assert!(!h.app.pomo.running);assert!(h.app.notice.is_some());
    for _ in 0..60 {h.frame(vec![]);}h.click_text("开始");assert!(h.app.pomo.running);
}

#[test]
fn focus_timer_gets_room_in_the_collapsed_island(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.tray_focus=false;for _ in 0..90 {h.frame(vec![]);}
    let before=h.app.current_width();h.app.pomo.running=true;
    for _ in 0..100 {h.frame(vec![]);}assert!(h.app.current_width()>before+20.0);assert!(h.app.pomo.remaining<1500.0);
}

#[test]
fn switching_to_rest_waits_for_the_start_button(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(1,1.0);h.hover(H::local(260.0,70.0),1.0);
    h.click(H::local(296.0,106.5));
    assert!(h.app.pomo.phase==crate::island::Phase::Rest);
    assert!(!h.app.pomo.running,"切换阶段应等待用户点击开始");
}

#[test]
fn focus_page_selects_stopped_phases_and_starts_only_from_play(){
    use crate::island::Phase;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.focus_minutes=30;h.app.settings.rest_minutes=7;
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(1,1.0);h.hover(H::local(260.0,70.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="休息")));
    h.click(H::local(336.0,106.5));
    assert!(h.app.pomo.phase==Phase::Rest && !h.app.pomo.running);
    assert_eq!(h.app.pomo.remaining,420.0,"休息采用设置中的时长");
    h.click(H::local(264.0,106.5));assert!(h.app.pomo.running);
    h.click(H::local(264.0,106.5));assert!(!h.app.pomo.running);
    let paused=h.app.pomo.remaining;for _ in 0..30 {h.frame(vec![]);}assert_eq!(h.app.pomo.remaining,paused);
    h.click(H::local(264.0,106.5));assert!(h.app.pomo.running && h.app.pomo.phase==Phase::Rest);
    h.click(H::local(336.0,106.5));assert!(h.app.pomo.running,"点击当前阶段不重置计时");
    h.click(H::local(192.0,106.5));assert!(h.app.pomo.phase==Phase::Focus && !h.app.pomo.running);
    assert_eq!(h.app.pomo.remaining,1800.0);
    h.click(H::local(264.0,106.5));assert!(h.app.pomo.running);
    h.click(H::local(336.0,106.5));assert!(h.app.pomo.phase==Phase::Rest && !h.app.pomo.running);
    h.click(H::local(264.0,106.5));assert!(h.app.pomo.running);
    h.click(H::local(329.0,50.0));assert!(!h.app.pomo.running && h.app.pomo.phase==Phase::Rest);
    assert_eq!(h.app.pomo.remaining,420.0,"重置应保留休息阶段");
}

#[test]
fn collapsed_focus_uses_tomato_and_rest_uses_a_white_cup(){
    use crate::island::Phase;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.tray_focus=false;
    for phase in [Phase::Focus,Phase::Rest] {
        h.app.pomo.phase=phase;h.app.pomo.remaining=h.app.phase_secs(phase);h.app.pomo.running=true;
        for _ in 0..120 {h.frame(vec![]);}
        let secs=h.app.pomo.remaining.ceil() as u32;let text=format!("{:02}:{:02}",secs/60,secs%60);
        let color=if phase==Phase::Focus{crate::visuals::theme::RED_TOP}else{crate::visuals::theme::SOFT_GREEN};
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==text && t.fallback_color==color)),"常驻倒计时颜色应跟随阶段");
        let tomato=h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Mesh(m) if m.vertices.iter().any(|v|v.color==crate::visuals::theme::RED_TOP)));
        if phase==Phase::Focus {assert!(tomato,"专注时应显示红色番茄水位");}
        else {
            assert!(!tomato,"休息时应替换番茄");
            assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.fill==egui::Color32::from_rgb(245,242,234) && (7.0..13.0).contains(&r.rect.width()) && (5.0..10.0).contains(&r.rect.height()))),"休息时应显示白瓷茶杯");
        }
        assert!((h.hit.height()-34.0).abs()<0.1,"图标不能撑高常驻岛");
    }
}

#[test]
fn focus_and_rest_completion_prepare_the_configured_next_phase_without_starting(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.settings.rest_minutes=7;h.app.settings.focus_minutes=30;
    for _ in 0..5 {
        h.app.pomo.phase=crate::island::Phase::Focus;h.app.pomo.running=true;h.app.pomo.remaining=0.01;h.frame(vec![]);
        assert!(h.app.pomo.phase==crate::island::Phase::Rest && !h.app.pomo.running);
        assert_eq!(h.app.pomo.remaining,420.0);assert!(h.app.notice.is_some());
        h.app.pomo.running=true;h.app.pomo.remaining=0.01;h.frame(vec![]);
        assert!(h.app.pomo.phase==crate::island::Phase::Focus && !h.app.pomo.running);
        assert_eq!(h.app.pomo.remaining,1800.0);
    }
}

#[test]
fn six_minutes_without_interaction_do_not_restart_static_animation_or_grow_textures(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=false;h.app.water.remind=false;
    for _ in 0..120{h.frame(vec![]);}
    let textures=h.ctx.tex_manager().read().num_allocated();
    for second in 3..=363 {
        h.t=second as f64;h.system_idle=Some(second as f64);h.frame(vec![]);
        assert!(!h.animating,"空闲到第 {second} 秒不应重新进入连续动画");
        assert_eq!(h.ctx.tex_manager().read().num_allocated(),textures,"空闲纹理数不能持续增长");
    }
}
