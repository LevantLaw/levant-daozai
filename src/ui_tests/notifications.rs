//! 通知、横幅与即时通讯。
use super::*;

#[test]
fn minimal_mode_keeps_focus_running_and_returns_after_notifications(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.set_minimal(true);h.app.pomo.running=true;h.app.pomo.remaining=120.0;
    for _ in 0..90 {h.frame(vec![]);}assert!(h.region.height()<20.0,"专注不能覆盖极简开关");
    h.app.alert("系统音量","音量 50%",5,false);
    for _ in 0..60 {h.frame(vec![]);}assert!(h.region.height()>30.0,"临时提醒仍应可见");
    for _ in 0..240 {h.frame(vec![]);}assert!(h.region.height()<20.0,"提醒结束后应回到细线");
    assert!(h.app.pomo.running&&h.app.pomo.remaining<115.0,"极简仅改变显示，计时继续运行");
}

#[test]
fn navigation_wraps_between_pages_and_notification_settings_remain_accessible() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.click(H::local(21.0,68.0));assert_eq!(h.app.page,crate::island::WINDOWS_PAGE);
    h.hover(H::local(300.0,60.0),0.8);h.click(H::local(398.0,88.0));assert_eq!(h.app.page,0);
    h.app.open_settings_category(3);assert!(h.app.settings_open);assert_eq!(h.app.settings_tab,6);
}

#[test]
fn mascot_feedback_plays_beside_notice_and_respects_switches(){
    use crate::island::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.drink_water();h.app.alert("待办到时间了","测试提醒",4,false);
    for _ in 0..60 {h.frame(vec![]);}assert_eq!(h.app.pet_state(),PetState::Satisfied);
    assert!(h.app.notice.is_some());
    let atlas=h.ctx.tex_manager().read().allocated().find_map(|(id,meta)|(meta.name=="daozai-expressions").then_some(*id)).expect("表情图集已加载");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Mesh(mesh) if mesh.texture_id==atlas)),"通知期间岛仔仍应可见");
    for _ in 0..240 {h.frame(vec![]);}assert_eq!(h.app.pet_state(),PetState::Awake);
    h.app.notice=None;for _ in 0..240 {h.frame(vec![]);}assert_eq!(h.app.pet_state(),PetState::Awake);
    h.app.drink_water();h.app.set_minimal(true);h.frame(vec![]);assert!(h.app.pet.reaction.is_none());
    h.app.set_minimal(false);h.app.settings.pet=false;h.app.drink_water();h.frame(vec![]);assert!(h.app.pet.reaction.is_none());
}

#[test]
fn minimal_island_hides_messenger_notices_until_expanded(){
    // 群友截图：避让细线下来电 / 新消息后，细线变成带微信图标的方块。细线照常弹横幅，收起后只留细线，未读不常挂。
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    use crate::island::Messenger;
    for (source,shape) in [(Messenger::Wechat,crate::model::IslandShape::Notch),(Messenger::Wechat,crate::model::IslandShape::Pill),(Messenger::Qq,crate::model::IslandShape::Pill)] {
        let mut h=H::new();
        h.app.settings.wechat_notifications=true;h.app.settings.read_notifications=true;h.app.set_minimal(true);h.app.settings.island_shape=shape;
        for _ in 0..60 {h.frame(vec![]);}
        let idle=h.region;
        match source {
            Messenger::Wechat=>h.app.receive_wechat_reminder_from(4242,0x1234),
            Messenger::Qq=>h.app.receive_notification("QQ".into(),"QQ".into(),"QQ · 新消息".into()),
            Messenger::WeCom|Messenger::Feishu|Messenger::DingTalk=>h.app.receive_messenger_reminder(crate::island::Unread::of(source)),
        }
        for _ in 0..30 {h.frame(vec![]);}
        assert!(h.app.notice.as_ref().and_then(|a|a.messenger())==Some(source),"{source:?} {shape:?}：避让细线照常弹横幅");
        for _ in 0..(8*60) {h.frame(vec![]);}
        assert!(h.app.notice.is_none(),"{source:?}：横幅按时收起");
        assert_eq!(h.app.pending_messengers.len(),1,"{source:?}：未读仍要记住");
        assert_eq!(h.region,idle,"{source:?} {shape:?}：收起时仍只有细线");
        // 鼠标移上去展开：未读胶囊出现在面板旁，可以点。
        h.hover(Pos2::new(WIN_W/2.0,2.0),0.8);h.hover(Pos2::new(WIN_W/2.0,40.0),1.0);
        assert!(h.app.open && h.app.progress()>0.9,"{source:?}：悬停应展开");
        assert!(h.app.messenger_appear()>0.9 && h.app.pending_messengers.len()==1,"{source:?}：展开后未读胶囊在面板旁显示");
    }
}

/// 微信语音来电：响铃期间任务栏持续闪动，按秒渲染岛体，排查样式问题。
#[test]
#[ignore="生成来电响铃时的岛体逐帧画面，供人工查看"]
fn preview_wechat_call_ringing(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::env::var_os("DAOZAI_CALL_PREVIEW_DIR").map(std::path::PathBuf::from)
        .unwrap_or_else(||std::env::temp_dir().join(format!("daozai-call-preview-{}",std::process::id())));
    std::fs::create_dir_all(&dest).unwrap();
    // 单条提醒 / 响铃期间反复提醒 / 来电窗口在另一个进程里（被当成第二个账号）。
    for (scenario,minimal) in [("single",false),("single",true),("ringing",false),("ringing",true),("twoproc",false),("twoproc",true)] {
        let mode=format!("{scenario}-{}",if minimal{"minimal"}else{"normal"});
        let mut h=H::new();h.app.settings.wechat_notifications=true;h.app.set_minimal(minimal);
        let mut raster=crate::preview_raster::Raster::new(1280,380);
        for frame in 0..=(16*60) {
            let t=frame as f64/60.0;
            match scenario {
                "single" if frame==60=>h.app.receive_wechat_reminder_from(4242,0x1234),
                "ringing" if frame>=60 && t<12.0 && frame%330==60=>h.app.receive_wechat_reminder_from(4242,0x1234),
                "twoproc" if frame==60=>h.app.receive_wechat_reminder_from(4242,0x1234),
                "twoproc" if frame==120=>h.app.receive_wechat_reminder_from(4343,0x5678),
                _=>{}
            }
            let env=Env{t,clock:"09:41:00".into(),today:"2026-10-31".into(),date:"10月31日 周六".into(),..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H))),time:Some(t),
                viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=ui.max_rect();h.app.frame(ui,win,&env);});
            });
            raster.update_textures(&output.textures_delta);
            if frame%60==0 {
                raster.buf.fill([0.09,0.11,0.18,1.0]);
                raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                image::save_buffer(dest.join(format!("{mode}-{:04.1}s.png",t)),&rgba,1280,380,image::ColorType::Rgba8).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }
    eprintln!("来电预览：{}",dest.display());
}

#[test]
fn repeated_system_changes_merge_without_losing_history(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.alert("系统音量","音量 20%",5,false);h.frame(vec![]);
    h.app.alert("系统音量","音量 30%",5,false);h.frame(vec![]);
    assert_eq!(h.app.notice.as_ref().unwrap().body,"音量 30%");assert_eq!(h.app.history.len(),2);assert!(h.app.alerts.is_empty());
}

#[test]
fn windows_notification_echo_does_not_duplicate_own_reminders(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.alert("喝水时间到了","喝点水吧",2,true);
    h.app.settings.read_notifications=true;h.app.receive_notification("Windows 通知".into(),"test".into(),"喝水时间到了 · 喝点水吧".into());assert_eq!(h.app.history.len(),1);
    assert_eq!(h.app.alerts.len(),1,"岛仔发出的系统通知不能再次进入岛内通知队列");
    h.app.notification_apps.items.push(("mail".into(),"邮件".into(),true));h.app.receive_notification("邮件".into(),"mail".into(),"收到一封新邮件".into());assert_eq!(h.app.alerts.len(),2);assert!(h.app.notification_bell_until>h.app.t);
}

#[test]
fn sit_notice_and_page_buttons_reset_the_timer(){
    use crate::island::SIT_TITLE;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.system_idle=Some(0.0);h.frame(vec![]);
    h.app.sit_start=h.t-70.0*60.0;h.app.alert(SIT_TITLE,"已连续坐了 70 分钟",2,false);for _ in 0..60 {h.frame(vec![]);}
    h.click_text("知道了");assert!(h.app.notice.is_none());assert!((h.app.sit_due()-h.t-3600.0).abs()<5.0);assert_eq!(h.app.water.drunk_ml,0);
    h.app.alert(SIT_TITLE,"已连续坐了 70 分钟",2,false);for _ in 0..60 {h.frame(vec![]);}
    h.click_text("起身了");assert!(h.app.notice.is_none());assert_eq!(h.app.sat_minutes(),0);assert_eq!(h.app.water.drunk_ml,0);
    // 健康页：点「久坐」分段切换，同一位置的主按钮变成「起身了」、开关管久坐；点「喝水」切回
    h.app.sit_start=h.t-30.0*60.0;
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(2,1.0);h.hover(H::local(300.0,60.0),0.8);
    h.click(H::local(124.0,22.0));assert!(h.app.sit_mode);h.hover(H::local(300.0,90.0),0.8);
    h.click(H::local(70.0,110.0));assert_eq!(h.app.sat_minutes(),0);assert_eq!(h.app.water.drunk_ml,0);
    let water_remind=h.app.water.remind;
    h.click(H::local(179.0,46.0));assert!(!h.app.settings.sit_remind);assert_eq!(h.app.water.remind,water_remind,"久坐模式的开关只管久坐");
    assert!(!crate::model::Settings::load().sit_remind,"重新加载后应保留久坐开关");
    h.click(H::local(97.0,22.0));assert!(!h.app.sit_mode);h.hover(H::local(300.0,90.0),0.8);
    h.click(H::local(70.0,110.0));assert_eq!(h.app.water.drunk_ml,250,"回到喝水后按钮可用");
}

#[test]
fn water_notice_buttons_work_without_opening_panel(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.alert("喝水时间到了","该喝水了",2,false);for _ in 0..60 {h.frame(vec![]);}
    h.click_text("喝一口");assert_eq!(h.app.water.drunk_ml,250);assert!(h.app.notice.is_none());
}

#[test]
fn app_notifications_show_temporary_bell_in_minimal_mode(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.set_minimal(true);h.frame(vec![]);
    h.app.settings.read_notifications=true;h.app.receive_notification("邮件".into(),"mail.app".into(),"私人内容".into());
    assert!(h.app.notification_apps.items[0].2);assert!(h.app.notification_bell_until>h.app.t);assert_eq!(h.app.alerts.len(),1);
    assert!(!std::fs::read_to_string(crate::model::config_dir().join("notification-apps.json")).unwrap().contains("私人内容"));
    for _ in 0..90 {h.frame(vec![]);}assert!(h.region.height()>20.0,"极简模式收到通知时应临时展开");
    h.t+=5.0;for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()>20.0,"横幅结束后 Windows 未读铃铛仍驻岛");
    h.app.windows_unread=false;for _ in 0..90 {h.frame(vec![]);}assert!(h.region.height()<20.0,"打开通知页后应恢复极简模式");
    h.app.notification_apps.items[0].2=false;let until=h.app.notification_bell_until;
    h.app.settings.read_notifications=true;h.app.receive_notification("邮件".into(),"mail.app".into(),"私人内容".into());assert_eq!(h.app.notification_bell_until,until);
}

#[test]
fn windows_notifications_share_one_bell_and_skip_qq(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.read_notifications=true;h.app.settings.qq_notifications=true;
    h.app.receive_notification("邮件".into(),"mail.app".into(),"新邮件".into());
    h.app.receive_notification("日历".into(),"calendar.app".into(),"会议提醒".into());
    assert_eq!(h.app.windows_notices.len(),2);
    assert_eq!(h.app.windows_notices[0].app,"邮件");
    assert!(h.app.windows_unread);
    h.app.receive_notification("QQ".into(),"QQ".into(),"新消息".into());
    assert_eq!(h.app.windows_notices.len(),2,"QQ 使用独立未读图标，不进入 Windows 通知页");
    assert!(h.app.pending_messengers.iter().any(|u|u.source==crate::island::Messenger::Qq));
    h.app.settings.visible_pages[crate::island::WINDOWS_PAGE]=false;
    h.app.receive_notification("天气".into(),"weather.app".into(),"降雨提醒".into());
    assert_eq!(h.app.windows_notices.len(),2,"功能页关闭时不再收集");
    h.app.settings.visible_pages[crate::island::WINDOWS_PAGE]=true;
    h.app.show_page(crate::island::WINDOWS_PAGE);
    assert_eq!(h.app.page,crate::island::WINDOWS_PAGE);
    assert!(!h.app.windows_unread,"打开通知页后清除未读标记");
    h.app.receive_notification("天气".into(),"weather.app".into(),"降雨提醒".into());
    assert!(!h.app.windows_unread,"停留在通知页时新通知不再点亮铃铛");
    h.app.go(0,1.0);h.app.forced_open_until=0.0;h.cursor=None;
    for _ in 0..120 {h.frame(vec![]);}
    h.app.receive_notification("天气".into(),"weather.app".into(),"降雨提醒".into());
    h.app.alerts.clear();h.app.notice=None;h.app.notification_bell_until=0.0;
    for _ in 0..60 {h.frame(vec![]);}
    assert!(h.app.windows_unread,"离开通知页后新通知重新点亮铃铛");
    let bell_x=WIN_W/2.0+h.app.resident.current().x[crate::island::layout::bar::BELL].unwrap();
    h.click(Pos2::new(bell_x,15.0));
    assert_eq!(h.app.page,crate::island::WINDOWS_PAGE,"点击驻岛铃铛应打开 Windows 通知页");
}

#[test]
fn windows_notice_page_lists_recent_notifications_and_can_clear_them(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.read_notifications=true;
    for i in 0..5 {h.app.receive_notification(format!("应用{i}"),format!("app{i}"),format!("消息{i}"));}
    h.app.alerts.clear();h.app.notice=None;h.app.windows_unread=true;
    h.app.show_page(crate::island::WINDOWS_PAGE);h.hover(H::local(300.0,80.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="5 条通知")));
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="应用0")));
    h.wheel(-40.0);h.wheel(-40.0);
    assert!(h.app.windows_first>0,"超出三行时滚轮翻看");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="应用4")));
    h.click_text("清空");
    assert!(h.app.windows_notices.is_empty());
    h.frame(vec![]);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="还没有收到 Windows 通知")));
}

#[test]
fn bluetooth_connection_soon_after_initial_scan_is_not_suppressed(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.frame(vec![]);
    h.t=2.0;
    h.bt=vec![crate::os::bt::Device{id:"shokz".into(),name:"Shokz".into(),battery:Some(100),kind:crate::os::bt::DeviceKind::Bone}];
    h.frame(vec![]);
    assert_eq!(h.app.notice.as_ref().map(|a|a.title.as_str()),Some("蓝牙设备"));
}

#[test]
fn bluetooth_notices_track_multiple_devices_independently(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    let audio=crate::os::bt::Device{id:"audio".into(),name:"Shokz".into(),battery:None,kind:crate::os::bt::DeviceKind::Bone};
    let keyboard=crate::os::bt::Device{id:"keyboard".into(),name:"QingLong 87".into(),battery:None,kind:crate::os::bt::DeviceKind::Keyboard};
    h.bt=vec![audio.clone()];h.frame(vec![]);
    h.bt.push(keyboard.clone());h.t=3.0;h.frame(vec![]);
    let notice=h.app.notice.as_ref().unwrap();
    assert_eq!(notice.body,"QingLong 87");
    assert_eq!(notice.bluetooth,Some((crate::os::bt::DeviceKind::Keyboard,true)));
    h.t=7.0;h.bt=vec![keyboard];h.frame(vec![]);
    let notice=h.app.notice.as_ref().unwrap();
    assert_eq!(notice.body,"Shokz · 已断开");
    assert_eq!(notice.bluetooth,Some((crate::os::bt::DeviceKind::Bone,false)));
}

#[test]
fn closing_app_notification_keeps_queued_messages_and_the_resident_bell(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for lyrics in [false,true]{
        let mut h=H::new();h.app.settings.island_lyrics=lyrics;h.app.settings.read_notifications=true;h.app.settings.notification_preview=true;
        if lyrics {
            h.media=crate::music::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
            h.app.set_lyrics("Track","Artist",120.0,crate::music::lyrics::parse_lrc("[00:00]晚风吹过了山岗"));
        }
        for _ in 0..60{h.frame(vec![]);}
        h.app.receive_notification("测试消息".into(),"notice.test".into(),"第一条消息".into());
        for _ in 0..60{h.frame(vec![]);}
        let bell=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Path(p)if p.points.len()==17=>Some(p.visual_bounding_rect().center()),_=>None}).unwrap();
        let close=h.ctx.read_response(egui::Id::new(("card",true,"external-dismiss"))).unwrap().rect;
        assert!(bell.x>close.right(),"通知铃铛应位于岛内最右侧");
        h.app.receive_notification("测试消息".into(),"notice.test".into(),"第二条消息".into());
        h.click(close.center());
        assert!(h.app.notice.is_none());assert!(h.app.notification_bell_until<=h.t,"关闭正文也应结束铃铛摆动");
        h.frame(vec![]);assert_eq!(h.app.notice.as_ref().unwrap().body,"第二条消息");
        for _ in 0..30{h.frame(vec![]);}
        h.click_text("关闭");h.cursor=None;
        for _ in 0..60{h.frame(vec![]);}
        assert!(h.app.notice.is_none());assert!(h.app.hud_notice.is_none());
        assert!(h.app.windows_unread,"关闭横幅后仍保留驻岛铃铛");
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Circle(c)if c.fill==egui::Color32::from_rgb(255,72,72))),"未读时铃铛保留红点");
        h.app.show_page(crate::island::WINDOWS_PAGE);h.app.forced_open_until=0.0;h.cursor=None;
        for _ in 0..120{h.frame(vec![]);}
        assert!(!h.app.windows_unread);
        assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Circle(c)if c.fill==egui::Color32::from_rgb(255,72,72))),"打开通知页后不再残留铃铛红点");
    }
}

#[test]
fn windows_notice_view_opens_the_notification_page(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.read_notifications=true;h.app.settings.notification_preview=true;
    for _ in 0..60{h.frame(vec![]);}
    h.app.receive_notification("天气".into(),"weather.app".into(),"降雨提醒".into());
    for _ in 0..30{h.frame(vec![]);}
    let view=h.ctx.read_response(egui::Id::new(("card",true,"external-view"))).expect("外部通知横幅应有查看按钮").rect;
    h.click(view.center());
    assert_eq!(h.app.page,crate::island::WINDOWS_PAGE);
    assert!(h.app.notice.is_none());
    assert!(!h.app.windows_unread);
}

#[test]
#[ignore="按需导出普通模式与岛上歌词模式的通知画面"]
fn notice_mode_layout_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_NOTICE_MODES_PREVIEW").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for lyrics in [false,true]{
        let mut h=H::new();h.app.settings.pet=true;h.app.settings.island_lyrics=lyrics;h.app.pomo.running=lyrics;
        h.media=crate::music::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::music::lyrics::parse_lrc("[00:00]晚风吹过了山岗"));
        let mut raster=crate::preview_raster::Raster::new(1920,405);
        for frame in 0..120{
            let t=frame as f64/60.0;if frame==30{h.app.alert("大写锁定","已开启",5,false);}
            let env=Env{t,clock:"09:41:00".into(),today:"2026-11-01".into(),date:"11月1日 周日".into(),media:h.media.clone(),..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(1.5),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),&env);});});
            raster.update_textures(&output.textures_delta);
            if frame==119{
                raster.buf.fill([0.94,0.95,0.97,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);let image=image::RgbaImage::from_raw(1920,405,rgba).unwrap();
                image::imageops::crop_imm(&image,105,0,960,140).to_image().save(dest.join(if lyrics{"岛上歌词模式.png"}else{"普通模式.png"})).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }
}

#[test]
#[ignore="按需录制右侧通知与原岛内容同时显示的实际画面"]
fn right_notice_preview(){
    use std::io::Write;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_RIGHT_NOTICE_PREVIEW").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    h.app.settings.pet=true;h.app.settings.island_lyrics=true;h.app.pomo.running=true;
    h.media=crate::music::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::music::lyrics::parse_lrc("[00:00]晚风吹过了山岗\n[01:00]下一句"));
    let mut video=std::process::Command::new(std::env::var_os("DAOZAI_FFMPEG").unwrap_or_else(||"ffmpeg".into())).args(["-y","-loglevel","error","-f","rawvideo","-pix_fmt","rgba","-s","1920x405","-r","30","-i","-","-vf","crop=1440:210:105:0","-c:v","libx264","-crf","19","-pix_fmt","yuv420p"]).arg(dest.join("右侧通知演示.mp4")).stdin(std::process::Stdio::piped()).spawn().unwrap();
    let mut pipe=video.stdin.take().unwrap();let mut raster=crate::preview_raster::Raster::new(1920,405);
    for frame in 0..420{
        let t=frame as f64/30.0;
        if frame==90{h.app.alert("大写锁定","已开启",5,false);}
        if frame==240{h.app.alert("蓝牙设备","OpenMove by Shokz · 电量 80%",5,false);}
        let env=Env{t,clock:"09:41:00".into(),today:"2026-11-01".into(),date:"11月1日 周日".into(),media:h.media.clone(),..Default::default()};
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(1.5),..Default::default()})).collect(),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),&env);});});
        raster.update_textures(&output.textures_delta);raster.buf.fill([0.94,0.95,0.97,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);pipe.write_all(&rgba).unwrap();
        if let Some(name)=match frame{60=>Some("原岛"),125=>Some("大小写提示"),270=>Some("蓝牙提示"),390=>Some("恢复原岛"),_=>None}{let image=image::RgbaImage::from_raw(1920,405,rgba).unwrap();image::imageops::crop_imm(&image,105,0,1440,210).to_image().save(dest.join(format!("{name}.png"))).unwrap();}
        raster.free_textures(&output.textures_delta);
    }
    drop(pipe);assert!(video.wait().unwrap().success());
}

#[test]
fn hud_expands_smoothly_from_collapsed_width(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    for _ in 0..60 { h.frame(vec![]); }
    let before=h.app.current_width();
    h.app.alert("大写锁定","已开启",5,false);
    h.frame(vec![]);
    assert!(h.app.current_width()-before < 30.0, "提示条不能在第一帧突然跳到目标宽度");
}

#[test]
fn hud_shrinks_smoothly_after_expiration(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    for _ in 0..90 {h.frame(vec![]);}let collapsed=h.app.current_width();
    h.app.alert("大写锁定","已开启",5,false);
    for _ in 0..60 {h.frame(vec![]);}
    let before=h.app.current_width();
    h.t+=2.0;
    h.frame(vec![]);
    assert!(h.app.notice.is_none());
    assert!(h.app.current_width()>collapsed+5.0);
    assert!(h.app.current_width()<before);
}

#[test]
fn bluetooth_badge_follows_connect_and_disconnect(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    // 首次扫描已连接的设备，记录但不作为新连接提醒。
    h.bt=vec![crate::os::bt::Device{id:"buds".into(),name:"Buds".into(),battery:Some(80),kind:crate::os::bt::DeviceKind::Earbuds}];h.frame(vec![]);
    assert!(h.app.notice.is_none());
    assert_eq!(h.app.bt_shown.first().map(|d|d.name.as_str()),Some("Buds"));
    // 之后断开、再连上，各弹一条 HUD 通知。
    h.t=20.0;h.bt.clear();h.frame(vec![]);
    assert_eq!(h.app.notice.as_ref().map(|a|a.body.as_str()),Some("Buds · 已断开"));
    assert!(h.app.bt_shown.is_empty());
    h.t=30.0;h.bt=vec![crate::os::bt::Device{id:"buds".into(),name:"Buds".into(),battery:Some(80),kind:crate::os::bt::DeviceKind::Earbuds}];h.frame(vec![]);
    assert_eq!(h.app.notice.as_ref().map(|a|a.body.as_str()),Some("Buds · 电量 80%"));
    // 没有设备时不留残留状态。
    h.t=40.0;h.bt.clear();h.frame(vec![]);
    assert!(h.app.bt_shown.is_empty()&&h.app.notice.as_ref().is_none_or(|a|a.title!="蓝牙设备"||a.body=="Buds · 已断开"));
}

#[test]
fn language_notice_text_is_vertically_centered() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.frame(vec![]);
    h.app.alert("键盘输入语言","英语",5,false);
    for _ in 0..60 {h.frame(vec![]);}
    h.click_notice_badge();
    fn find(shapes:&[egui::epaint::ClippedShape])->Option<Rect>{
        shapes.iter().find_map(|s|match &s.shape {
            egui::Shape::Text(text) if text.galley.text()=="英语"=>Some(text.visual_bounding_rect()),
            _=>None,
        })
    }
    let text=find(&h.shapes).expect("输入语言状态应可见");
    assert!((text.center().y-19.0).abs()<1.0,"文字应与图标垂直居中：{text:?}");
}

#[test]
fn todo_notice_view_locates_the_reminded_task() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.todos.items=(0..7).map(|i|crate::todo::TodoItem{text:format!("任务 {i}"),done_on:None,due:if i==6{Some("2026-10-28 09:40".into())}else{None},reminded:false,..Default::default()}).collect();
    for _ in 0..60 {h.frame(vec![]);}
    h.click_text("查看");
    assert_eq!(h.app.page,4);
    assert!(h.app.todo_first>=4,"打开待办页时应能看到收到提醒的任务");
}

#[test]
fn new_notification_sways_twice_then_stops_and_restarts_on_the_next_one() {
    for (i,expected_positive) in [true,false,true,false].into_iter().enumerate(){
        let angle=crate::visuals::paint::bell_swing(0.95*(i as f64*0.25+0.125));
        assert_eq!(angle>0.0,expected_positive);
    }
    assert_eq!(crate::visuals::paint::bell_swing(1.0),0.0);
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.settings.read_notifications=true;h.app.receive_notification("测试通知".into(),"test.notification".into(),"第一条".into());
    let first=h.app.notification_bell_at;
    for _ in 0..90{h.frame(vec![]);}
    h.app.settings.read_notifications=true;h.app.receive_notification("测试通知".into(),"test.notification".into(),"第二条".into());
    assert!(h.app.notification_bell_at>first+1.0);
    h.frame(vec![]);
    assert!(h.app.notification_bell_until>h.t);
}

#[test]
fn hud_titles_and_states_share_the_icon_center_without_overlapping() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (title,body,left,right) in [
        ("网络连接","Wi-Fi · 网络已连接","Wi-Fi","网络已连接"),
        ("网络连接","以太网 · 网络已断开","以太网","网络已断开"),
        ("大写锁定","已开启","大写锁定","已开启"),
        ("键盘输入语言","中文","输入语言","中文"),
        ("电源已连接","当前电量 76%","电源已连接","当前电量 76%"),
        ("已切换电池供电","当前电量 76%","电池供电","当前电量 76%"),
    ] {
        let mut h=H::new();h.frame(vec![]);h.app.alert(title,body,5,false);
        for _ in 0..60{h.frame(vec![]);}
        h.click_notice_badge();
        let find=|needle:&str|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==needle=>Some(t.visual_bounding_rect()),_=>None}).unwrap_or_else(||panic!("未显示状态文字：{needle}"));
        let l=find(left);let r=find(right);
        assert!((l.center().y-19.0).abs()<1.0 && (r.center().y-19.0).abs()<1.0,"{title} 应垂直居中");
        assert!(l.right()+8.0<=r.left(),"{title} 的标题和状态应留出间距");
    }
}

#[test]
fn compact_canvas_preserves_panel_and_notice_pixels(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(width:f32,page:Option<usize>,notice:bool)->Vec<u8>{
        let mut h=H::new();h.app.settings.island_lyrics=true;
        let media=crate::music::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::music::lyrics::parse_lrc("[00:00]晚风吹过了山岗\n[01:00]下一句"));
        if let Some(page)=page{h.app.show_page(page);}
        let mut raster=crate::preview_raster::Raster::new(1280,270);
        let mut rgba=Vec::new();
        for frame in 0..180 {
            if notice && frame==60{h.app.alert("收到新邮件","会议通知及验证码 834921",5,true);}
            let click=(notice && (frame==90 || frame==91)).then(||Pos2::new(WIN_W/2.0+(h.app.current_width()-44.0)/2.0+26.0,15.0));
            let t=frame as f64/60.0;
            let env=Env{t,clock:"09:41:00".into(),today:"2026-10-28".into(),date:"10月28日 周三".into(),media:media.clone(),cursor:click.or_else(||page.map(|_|H::local(300.0,40.0))),..Default::default()};
            let output=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(width,WIN_H))),time:Some(t),events:click.map(|pos|vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:frame==90,modifiers:Default::default()}]).unwrap_or_default(),..Default::default()},|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),&env);});
            });
            raster.update_textures(&output.textures_delta);
            if [30,65,95,179].contains(&frame){
                raster.buf.fill([0.0;4]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut pixels=Vec::new();raster.to_rgba8(&mut pixels);
                if page.is_none() && notice && frame==95 {
                    if let Some(dir)=std::env::var_os("DAOZAI_GRAPHICS_PREVIEW") {let dir=std::path::PathBuf::from(dir);std::fs::create_dir_all(&dir).unwrap();image::save_buffer(dir.join(format!("canvas-{width}.png")),&pixels,1280,270,image::ColorType::Rgba8).unwrap();}
                }
                rgba.extend(pixels);
            }
        }
        rgba
    }
    for page in std::iter::once(None).chain((0..8).map(Some)) {
        for notice in [false,true] {
            assert_eq!(draw(1280.0,page,notice),draw(960.0,page,notice),"缩减透明余量不能改变页面 {page:?} 和通知 {notice} 的像素");
        }
    }
}

#[test]
fn external_notice_respects_permissions_masks_preview_and_never_reemits(){
 let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
 h.app.receive_notification("系统音量".into(),"external.test".into(),"验证码 123456".into());assert!(h.app.alerts.is_empty());
 h.app.settings.read_notifications=true;
 h.app.receive_notification("系统音量".into(),"external.test".into(),"验证码 123456".into());
 let a=h.app.alerts.back().unwrap();assert!(!a.is_hud());assert!(!a.body.contains("123456"));assert_eq!(a.page,usize::MAX);
 assert!(h.app.history.is_empty());assert!(!h.app.pending.iter().any(|c|matches!(c,crate::island::Cmd::Notify(..))));
 h.app.tick_alerts();assert!(h.app.notice.is_some());h.app.settings.read_notifications=false;h.app.tick_alerts();assert!(h.app.notice.is_none());
 h.app.settings.read_notifications=true;h.app.settings.notification_preview=true;
 h.app.receive_notification("测试".into(),"external.test".into(),"验证码 123456，请勿泄露".into());assert!(!h.app.alerts.back().unwrap().body.contains("123456"));
 h.app.tick_alerts();h.app.t+=4.6;h.app.tick_alerts();assert!(h.app.notice.is_none());
}

#[test]
fn wechat_notifications_work_without_toast_permission_and_stop_when_disabled(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.receive_wechat_reminder();assert!(h.app.alerts.is_empty());
    h.app.settings.wechat_notifications=true;
    h.app.settings.save();assert!(crate::model::Settings::load().wechat_notifications);
    assert!(!h.app.settings.read_notifications);
    h.app.receive_wechat_reminder();h.app.tick_alerts();
    let notice=h.app.notice.as_ref().unwrap();
    assert_eq!(notice.title,"微信");assert_eq!(notice.body,"有新消息 · 请到微信查看");
    assert!(h.app.history.is_empty());
    assert!(!h.app.pending.iter().any(|c|matches!(c,crate::island::Cmd::Notify(..))));
    h.app.settings.wechat_notifications=false;h.app.tick_alerts();assert!(h.app.notice.is_none());
    h.app.settings.wechat_notifications=true;h.app.settings.notifications=false;
    h.app.receive_wechat_reminder();assert!(h.app.alerts.is_empty());
}

#[test]
fn banner_waits_until_island_collapses(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.t=10.0;h.app.open=true;
    h.app.alert("待办到时间了","任务",4,false);
    h.app.tick_alerts();
    assert!(h.app.notice.is_none(),"展开期间横幅排队");
    h.app.t=14.0;h.app.tick_alerts();
    assert!(h.app.notice.is_none(),"展开期间横幅不开始计时");
    h.app.open=false;h.app.t=14.1;h.app.tick_alerts();
    assert!(h.app.notice.is_some(),"收起后显示横幅");
    h.app.t=22.0;h.app.tick_alerts();
    assert!(h.app.notice.is_some(),"横幅从显示时开始计时");
    h.app.t=22.2;h.app.tick_alerts();
    assert!(h.app.notice.is_none(),"横幅显示满时长后收起");
}

#[test]
fn hud_alerts_are_not_held_while_island_is_open(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.t=10.0;h.app.open=true;
    h.app.alert("系统音量","50%",5,false);
    h.app.tick_alerts();
    assert!(h.app.notice.is_some(),"HUD 不因展开而延后");
    h.app.t=12.6;h.app.tick_alerts();
    assert!(h.app.notice.is_none(),"HUD 仍按自身时长过期");
}

#[test]
fn ai_done_opens_session_list_without_banner(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.agent.bridge=crate::ai_session::bridge::Bridge::start(h.ctx.clone()).ok();
    let (port,token)=h.app.agent.bridge.as_ref().map(|bridge|(bridge.port,bridge.token.clone())).unwrap();
    post_agent_event(port,&token,"SessionStart",serde_json::json!({"session_id":"ui-done","cwd":"E:\\demo\\project"}));
    post_agent_event(port,&token,"Stop",serde_json::json!({"session_id":"ui-done","last_assistant_message":"已修复"}));
    for _ in 0..600 {h.frame(vec![]);if h.app.open&&h.app.page==crate::island::AI_PAGE{break;}}
    assert!(h.app.open,"完成事件应直接打开 AI 会话页");
    assert_eq!(h.app.page,crate::island::AI_PAGE);
    assert!(!h.app.agent.detail,"完成事件不应展开会话详情");
    assert!(h.app.agent.selected.is_none());
    assert!(h.app.notice.is_none()&&h.app.alerts.is_empty(),"完成事件不应出横幅");
}

#[test]
fn ai_waiting_alert_respects_notifications_master(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.agent.bridge=crate::ai_session::bridge::Bridge::start(h.ctx.clone()).ok();
    let (port,token)=h.app.agent.bridge.as_ref().map(|bridge|(bridge.port,bridge.token.clone())).unwrap();
    h.app.settings.notifications=false;
    post_agent_hook(port,&token,"ui-muted");
    for _ in 0..10 {h.frame(vec![]);}
    assert!(!h.app.open&&h.app.alerts.is_empty()&&h.app.notice.is_none(),"提醒总开关关闭时不展开、不排队");
    assert_eq!(h.app.agent.store.waiting_count(),1,"状态灯仍按会话状态显示");
}

#[test]
fn compact_font_atlas_preserves_panel_and_notice_pixels(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(texture_side:usize,page:Option<usize>,notice:bool)->Vec<u8>{
        let mut h=H::new();h.app.settings.island_lyrics=true;
        let media=crate::music::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::music::lyrics::parse_lrc("[00:00]晚风吹过了山岗\n[01:00]下一句"));
        if let Some(page)=page{h.app.show_page(page);}
        let mut raster=crate::preview_raster::Raster::new(1280,270);
        let mut rgba=Vec::new();
        for frame in 0..180 {
            if notice && frame==60{h.app.alert("收到新邮件","会议通知及验证码 834921",5,true);}
            let click=(notice && (frame==90 || frame==91)).then(||Pos2::new(WIN_W/2.0+(h.app.current_width()-44.0)/2.0+26.0,15.0));
            let t=frame as f64/60.0;
            let env=Env{t,clock:"09:41:00".into(),today:"2026-10-28".into(),date:"10月28日 周三".into(),media:media.clone(),cursor:click.or_else(||page.map(|_|H::local(300.0,40.0))),..Default::default()};
            let output=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(960.0,WIN_H))),max_texture_side:Some(texture_side),time:Some(t),events:click.map(|pos|vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:frame==90,modifiers:Default::default()}]).unwrap_or_default(),..Default::default()},|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),&env);});
            });
            raster.update_textures(&output.textures_delta);
            if [30,65,95,179].contains(&frame){
                raster.buf.fill([0.0;4]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut pixels=Vec::new();raster.to_rgba8(&mut pixels);
                if page.is_none() && notice && frame==95 {
                    if let Some(dir)=std::env::var_os("DAOZAI_GRAPHICS_PREVIEW") {let dir=std::path::PathBuf::from(dir);std::fs::create_dir_all(&dir).unwrap();image::save_buffer(dir.join(format!("atlas-{texture_side}.png")),&pixels,1280,270,image::ColorType::Rgba8).unwrap();}
                }
                rgba.extend(pixels);
            }
        }
        rgba
    }
    for page in std::iter::once(None).chain((0..8).map(Some)) {
        for notice in [false,true] {
            let reference=draw(16384,page,notice);
            for limit in [4096,2048] {let other=draw(limit,page,notice);
                let max=reference.iter().zip(&other).map(|(a,b)|a.abs_diff(*b)).max().unwrap_or(0);let count=reference.iter().zip(&other).filter(|(a,b)|a!=b).count();
                // 图集坐标改变只允许少量通道有 1/255 的浮点采样舍入差异（实测最多约 0.005% 的通道），肉眼不可见。
                assert!(max<=1 && count<reference.len()/1000,"图集上限 {limit} 改变了页面 {page:?}、通知 {notice} 的像素：{max}/{count}");}
        }
    }
}

#[test]
fn bluetooth_switch_clears_notices_and_restarts_with_fresh_baseline(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    let device=crate::os::bt::Device{id:"keyboard".into(),name:"Keyboard".into(),battery:None,kind:crate::os::bt::DeviceKind::Keyboard};
    h.app.bluetooth_alert(&device,true);h.frame(vec![]);
    h.app.bluetooth_alert(&device,false);
    h.app.settings.bluetooth_notifications=false;h.frame(vec![]);
    assert!(h.app.notice.as_ref().is_none_or(|a|a.title!="蓝牙设备"));
    assert!(!h.app.alerts.iter().any(|a|a.title=="蓝牙设备"));
    h.app.bluetooth_alert(&device,true);
    assert!(!h.app.alerts.iter().any(|a|a.title=="蓝牙设备"));
    h.app.settings.save();assert!(!crate::model::Settings::load().bluetooth_notifications);
    h.bt=vec![device];h.app.settings.bluetooth_notifications=true;h.frame(vec![]);
    assert!(h.app.notice.as_ref().is_none_or(|a|a.title!="蓝牙设备"));
    h.bt.clear();h.frame(vec![]);
    assert!(h.app.notice.as_ref().is_some_and(|a|a.title=="蓝牙设备") || h.app.alerts.iter().any(|a|a.title=="蓝牙设备"));
}

#[test]
fn minimal_side_pill_returns_to_line_while_wechat_is_unread(){
    use crate::model::{Dock,IslandShape};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dock in [Dock::Right,Dock::Left] {
        let mut h=H::new();
        h.app.settings.dock=dock;
        h.app.settings.island_shape=IslandShape::Pill;
        h.app.set_minimal(true);
        h.app.settings.wechat_notifications=true;
        for _ in 0..90 {side_frame_with(&mut h,None);}
        let idle=side_frame_with(&mut h,None);
        h.app.receive_wechat_reminder_from(4242,0x1234);
        for _ in 0..30 {side_frame_with(&mut h,None);}
        assert_eq!(h.app.notice.as_ref().and_then(|a|a.messenger()),Some(crate::island::Messenger::Wechat));
        for _ in 0..480 {side_frame_with(&mut h,None);}
        let collapsed=side_frame_with(&mut h,None);
        assert!(h.app.notice.is_none() && collapsed.notice_hit.is_none());
        assert_eq!(h.app.pending_messengers.len(),1);
        assert_eq!(collapsed.hit,idle.hit,"{dock:?} 有未读时也应收回细线");
        assert_eq!(collapsed.region,idle.region);
        let edge=Some(idle.hit.center());
        for _ in 0..90 {side_frame_with(&mut h,edge);}
        assert!(h.app.open && side_frame_with(&mut h,edge).notice_hit.is_some());
    }
}

#[test]
fn automatic_minimal_hides_unread_hit_area_until_reopened(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dock in [crate::model::Dock::Top,crate::model::Dock::Left,crate::model::Dock::Right] {
        let mut h=H::new();
        h.app.settings.dock=dock;
        h.app.settings.browser_minimal=true;
        h.app.settings.wechat_notifications=true;
        h.app.settings.read_notifications=true;
        h.app.receive_wechat_reminder();
        h.app.receive_notification("QQ".into(),"QQ".into(),"测试".into());
        h.app.alerts.clear();
        h.app.notice=None;
        h.app.notification_bell_until=0.0;
        h.browser_at_top=Some(true);
        for _ in 0..90 {side_frame_with(&mut h,None);}
        let out=side_frame_with(&mut h,None);
        assert!(out.notice_hit.is_none(),"{dock:?} 避让时未读仍占用鼠标区域");
        assert!(out.click_through,"{dock:?} 细条可感应悬停，点击应交给底下的程序");
        assert_eq!(h.app.pending_messengers.len(),2);
        h.app.show_page(0);
        for _ in 0..60 {side_frame_with(&mut h,None);}
        let opened=side_frame_with(&mut h,None);
        assert!(opened.notice_hit.is_some() && !opened.click_through);
        h.browser_at_top=Some(false);
        for _ in 0..360 {side_frame_with(&mut h,None);}
        assert!(!h.app.open && side_frame_with(&mut h,None).notice_hit.is_some());
    }
}

#[test]
fn notification_preview_masks_fullwidth_and_zero_width_codes(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.read_notifications=true;h.app.settings.notification_preview=true;
    for body in ["验证码 １２３４５６，请勿泄露","验证码 12\u{200b}3456，请勿泄露"] {
        h.app.receive_notification("测试".into(),"external.test".into(),body.into());
        assert_eq!(h.app.alerts.back().unwrap().body,"验证码 ••••••，请勿泄露");
    }
}

#[test]
#[ignore="按需导出提醒与账号状态提示的实际样式"]
fn messenger_notice_status_gallery(){
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_MESSENGER_PREVIEW").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    for case in 0..15{
        let mut h=H::new();h.app.settings.pet=true;h.app.settings.wechat_notifications=true;
        h.app.settings.read_notifications=true;h.app.settings.notification_preview=true;
        h.app.settings.bluetooth_notifications=true;
        let ppp=2.0;let (w,height)=(crate::island::CANVAS_W,WIN_H);
        let mut raster=crate::preview_raster::Raster::new((w*ppp) as usize,(height*ppp) as usize);
        for frame in 0..90{
            h.t=frame as f64/60.0;
            if frame==40{match case{
                0=>h.app.receive_wechat_reminder_from(100,0),
                1=>h.app.receive_notification("QQ".into(),"QQ".into(),"有新消息 · 请到 QQ 查看".into()),
                2=>{for pid in [100,200,300]{h.app.receive_wechat_reminder_from(pid,0);}h.app.alerts.clear();h.app.notice=None;h.app.notification_bell_until=0.0;},
                3=>h.app.messenger_unavailable("QQ 账号已退出"),
                4=>h.app.messenger_unavailable("微信 账号已退出"),
                5=>h.app.messenger_unavailable("暂时无法确认此 QQ 消息的账号"),
                6=>h.app.messenger_unavailable("QQ 窗口暂时无法打开"),
                7=>{h.app.show_page(4);h.app.messenger_unavailable("QQ 账号已退出");},
                8=>h.app.alert("大写锁定","已开启",5,false),
                9=>h.app.alert("系统音量","音量 60%",5,false),
                10=>h.app.bluetooth_alert(&crate::os::bt::Device{id:"preview".into(),name:"耳机".into(),battery:Some(80),kind:crate::os::bt::DeviceKind::Headphones},true),
                11=>h.app.alert("电量偏低","剩余 15% · 请连接电源",5,false),
                12=>{
                    let start=chrono::Local::now()+chrono::Duration::minutes(10);
                    h.app.meeting_alert(crate::mail::Meeting{all_day:false,title:"团队会议".into(),start:start.timestamp(),end:None,at:start.format("%m-%d %H:%M").to_string(),uid:"preview".into()},"本机日程".into(),7);
                },
                13=>h.app.alert("喝水时间到了","休息一下，喝 250 ml 水吧",2,false),
                14=>h.app.alert("待办到时间了","整理本周资料",4,false),
                _=>unreachable!(),
            }}
            let cursor=(case==7).then(||H::local(160.0,12.0));
            let env=Env{t:h.t,clock:"09:41:00".into(),today:"2026-11-08".into(),date:"11月8日 周日".into(),cursor,volume:(case==9).then_some((60,false)),media:h.media.clone(),..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,height))),time:Some(h.t),focused:true,
                events:cursor.map(|p|vec![egui::Event::PointerMoved(p)]).unwrap_or_default(),
                viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{
                h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),&env);
            });});
            raster.update_textures(&output.textures_delta);
            if frame==89{
                raster.buf.fill([0.93,0.95,0.97,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                let image=image::RgbaImage::from_raw((w*ppp)as u32,(height*ppp)as u32,rgba).unwrap();
                image::imageops::crop_imm(&image,100,0,1560,if case==7||case==12{440}else{260}).to_image().save(dest.join(format!("{case:02}.png"))).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }
}

#[test]
fn qq_unknown_reminders_survive_viewing_other_accounts(){
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.read_notifications=true;
    h.app.receive_notification("QQ".into(),"QQ".into(),"未确认账号的消息".into());
    h.app.viewed_client(crate::island::Messenger::Qq,200);
    assert_eq!(h.app.pending_messengers,[crate::island::Unread::of(crate::island::Messenger::Qq)]);
}

#[test]
fn messenger_window_replacement_removes_stale_banner_targets_without_clearing_other_accounts(){
    use crate::island::{Cmd,Messenger,Unread};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for source in Messenger::ALL{
        let mut h=H::new();h.app.settings.read_notifications=true;h.app.settings.wechat_notifications=true;h.app.settings.wecom_notifications=true;h.app.settings.feishu_notifications=true;h.app.settings.dingtalk_notifications=true;
        let old=Unread{source,pid:100,window:11};let other=Unread{source,pid:200,window:22};let current=Unread{window:33,..old};
        let receive=|app:&mut App,unread:Unread|match source{
            Messenger::Wechat|Messenger::WeCom|Messenger::Feishu|Messenger::DingTalk=>app.receive_messenger_reminder(unread),
            Messenger::Qq=>app.receive_notification_from("QQ".into(),"QQ".into(),"有新消息".into(),Some(unread)),
        };
        receive(&mut h.app,old);h.frame(vec![]);receive(&mut h.app,other);receive(&mut h.app,current);
        assert_eq!(h.app.pending_messengers,[current,other]);
        assert!(h.app.alerts.iter().chain(h.app.notice.iter()).all(|alert|alert.unread!=Some(old)),"窗口被新消息替换后，横幅不能继续引用已经失效的目标");
        assert!(h.app.alerts.iter().chain(h.app.notice.iter()).any(|alert|alert.unread==Some(other)),"其他账号的横幅必须保留");
        h.app.discard_unread(other);for _ in 0..45{h.frame(vec![]);}
        h.click_text("查看");assert!(h.cmds.contains(&Cmd::ViewMessenger(current)));
    }
}

#[test]
fn qq_bound_notices_open_their_original_account_and_clear_independently(){
    use crate::island::{Cmd,Messenger,Unread};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.read_notifications=true;
    let a=Unread{source:Messenger::Qq,pid:100,window:11};let b=Unread{source:Messenger::Qq,pid:200,window:22};
    h.app.receive_notification_from("QQ".into(),"QQ".into(),"有新消息".into(),Some(a));h.frame(vec![]);
    h.app.receive_notification_from("QQ".into(),"QQ".into(),"有新消息".into(),Some(b));
    assert_eq!(h.app.pending_messengers,[a,b]);
    for _ in 0..45{h.frame(vec![]);}
    h.click_text("查看");assert!(h.cmds.contains(&Cmd::ViewMessenger(a)),"横幅应打开它自己的账号，不能取最近新增的另一个账号");
    h.app.viewed_client(Messenger::Qq,200);h.frame(vec![]);
    assert_eq!(h.app.pending_messengers,[a]);
    h.cmds.clear();h.click_text("查看");assert!(h.cmds.contains(&Cmd::ViewMessenger(a)));
    h.app.discard_unread(a);h.frame(vec![]);assert!(h.app.pending_messengers.is_empty());assert!(h.app.notice.is_none());
}

#[test]
fn qq_unknown_notice_can_close_without_dismissing_a_bound_account(){
    use crate::island::{Messenger,Unread};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.read_notifications=true;
    h.app.receive_notification("QQ".into(),"QQ".into(),"未确认账号".into());h.frame(vec![]);
    let b=Unread{source:Messenger::Qq,pid:200,window:22};
    h.app.receive_notification_from("QQ".into(),"QQ".into(),"另一账号的消息".into(),Some(b));
    for _ in 0..45{h.frame(vec![]);}
    h.click_text("关闭");h.frame(vec![]);
    assert_eq!(h.app.pending_messengers,[b]);
}

#[test]
fn qq_expired_unidentified_notice_can_be_reopened_and_explicitly_cleared(){
    use crate::island::{Messenger,Unread};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.read_notifications=true;
    let unread=Unread::of(Messenger::Qq);
    h.app.receive_notification("QQ".into(),"QQ".into(),"测试消息".into());
    for _ in 0..360{h.frame(vec![]);}assert!(h.app.notice.is_none());
    h.app.reveal_messenger_notice(unread);for _ in 0..45{h.frame(vec![]);}
    h.click_text("关闭");h.frame(vec![]);assert!(h.app.pending_messengers.is_empty());
}

#[test]
fn qq_unknown_reminders_are_not_numbered_as_another_known_account(){
    use crate::island::{Messenger,Unread};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.read_notifications=true;
    for (pid,window) in [(100,11),(0,0),(200,22)]{
        h.app.receive_notification_from("QQ".into(),"QQ".into(),"有新消息".into(),(pid!=0).then_some(Unread{source:Messenger::Qq,pid,window}));
    }
    assert_eq!((h.app.unread_tag(0),h.app.unread_tag(1),h.app.unread_tag(2)),(Some(1),None,Some(2)));
    h.app.alerts.clear();h.app.notice=None;h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    assert!(h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text)if text.galley.text()=="?")));
}

#[test]
fn unavailable_messenger_status_stays_visible_when_the_island_is_collapsed(){
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for message in ["QQ 账号已退出","微信 账号已退出","暂时无法确认此 QQ 消息的账号"]{
        let mut h=H::new();h.hover(Pos2::new(600.0,180.0),1.0);
        h.app.messenger_unavailable(message);h.hover(Pos2::new(600.0,180.0),0.3);
        let bounds=h.shapes.iter().find_map(|shape|match &shape.shape{
            egui::Shape::Text(text) if text.galley.text()==message=>Some(text.visual_bounding_rect()),_=>None,
        }).expect("岛收起时仍应完整显示账号状态提示");
        assert!(h.app.progress()<0.02,"提示不应展开岛体");
        assert!(h.region.contains_rect(bounds),"原生窗口区域应包含提示");
        assert!(!h.hit.intersects(bounds),"提示不应拦截鼠标");
    }
}

#[test]
fn exited_messenger_unreads_clear_their_badges_and_notifications(){
    use crate::island::{Messenger,Unread};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.wechat_notifications=true;h.app.settings.read_notifications=true;
    h.app.receive_wechat_reminder_from(100,11);h.frame(vec![]);
    assert_eq!(h.app.notice.as_ref().and_then(|notice|notice.messenger()),Some(Messenger::Wechat));
    h.app.receive_wechat_reminder_from(200,22);h.app.receive_wechat_reminder_from(300,33);
    h.app.receive_notification("QQ".into(),"QQ".into(),"测试".into());
    for (pid,window) in [(200,22),(100,11)]{
        h.app.discard_unread(Unread{source:Messenger::Wechat,pid,window});h.frame(vec![]);
        assert!(h.app.pending_messengers.contains(&Unread{source:Messenger::Wechat,pid:300,window:33}));
        assert_eq!(h.app.notice.as_ref().and_then(|notice|notice.messenger()),Some(Messenger::Wechat));
    }
    h.app.discard_unread(Unread{source:Messenger::Wechat,pid:300,window:33});h.frame(vec![]);
    assert_eq!(h.app.pending_messengers,[Unread::of(Messenger::Qq)]);
    assert!(h.app.alerts.iter().all(|notice|notice.messenger()!=Some(Messenger::Wechat)));
    assert_ne!(h.app.notice.as_ref().and_then(|notice|notice.messenger()),Some(Messenger::Wechat));
    h.app.discard_unread(Unread::of(Messenger::Qq));h.frame(vec![]);
    assert!(h.app.pending_messengers.is_empty());
    assert!(h.app.alerts.iter().all(|notice|notice.messenger()!=Some(Messenger::Qq)));
    assert_ne!(h.app.notice.as_ref().and_then(|notice|notice.messenger()),Some(Messenger::Qq));
}

#[test]
fn messenger_notifications_keep_separate_badges_until_the_client_is_viewed(){
    use crate::island::{Cmd,Messenger,Unread};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.wechat_notifications=true;h.app.settings.read_notifications=true;
    h.frame(vec![]);h.app.receive_wechat_reminder();
    for _ in 0..60{h.frame(vec![]);}
    h.click_text("查看");assert!(h.cmds.contains(&Cmd::ViewMessenger(Unread::of(Messenger::Wechat))));
    h.click_text("关闭");assert!(h.app.notice.is_none());
    h.cursor=None;h.app.receive_notification("QQ".into(),"QQ".into(),"小林 · 测试".into());
    for _ in 0..360{h.frame(vec![]);}
    assert!(h.app.notice.is_none());
    let sources=|h:&H|h.app.pending_messengers.iter().map(|u|u.source).collect::<Vec<_>>();
    assert_eq!(sources(&h),[Messenger::Wechat,Messenger::Qq]);
    // 收起时两条未读叠成一个入口，点它展开面板，再在旁边的胶囊里逐个点开。
    h.frame(vec![]);
    let stack=h.ctx.read_response(egui::Id::new(("card",true,"messenger-stack"))).expect("叠放入口可点击").rect.center();
    h.click(stack);for _ in 0..60{h.frame(vec![]);}
    assert!(h.app.open);
    for source in [Messenger::Wechat,Messenger::Qq]{
        h.frame(vec![]);
        let pos=h.ctx.read_response(egui::Id::new(("card",true,source.name()))).expect("胶囊图标可点击").rect.center();
        h.cmds.clear(); h.click(pos);assert!(h.cmds.contains(&Cmd::ViewMessenger(Unread::of(source))),"{source:?} {pos:?} {:?}",h.cmds);
        h.app.viewed_messenger(source);h.frame(vec![]);
    }
    assert!(h.app.pending_messengers.is_empty());
    h.app.receive_wechat_reminder();h.app.receive_wechat_reminder();
    assert_eq!(sources(&h),[Messenger::Wechat]);
    h.app.settings.wechat_notifications=false;h.frame(vec![]);assert!(h.app.pending_messengers.is_empty());
}

#[test]
fn messenger_capsule_grows_during_opening(){
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.wechat_notifications=true;
    h.app.receive_wechat_reminder();
    h.app.alerts.clear();
    h.app.notice=None;
    for opening in [true,false,true,false] {
        h.cursor=opening.then_some(Pos2::new(WIN_W/2.0,12.0));
        let mut sampled=false;
        for _ in 0..90 {
            h.frame(vec![]);
            let appear=h.app.messenger_appear();
            if (0.02..0.95).contains(&appear) {
                let button=h.ctx.read_response(egui::Id::new(("card",true,"微信".to_string()))).unwrap();
                sampled|=(1.0..crate::island::UNREAD_W-1.0).contains(&button.rect.width());
            }
        }
        assert!(sampled && h.app.open==opening,"每次展开与收回都应经过中间尺寸");
    }
}

#[test]
fn messenger_accounts_get_number_tags_and_clear_one_by_one(){
    use crate::island::{Cmd,Messenger,Unread};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.wechat_notifications=true;h.app.settings.read_notifications=true;
    h.app.receive_wechat_reminder_from(100,11);h.app.receive_wechat_reminder_from(200,22);h.app.receive_wechat_reminder_from(100,12);
    h.app.receive_notification("QQ".into(),"QQ".into(),"测试".into());
    assert_eq!(h.app.pending_messengers,[Unread{source:Messenger::Wechat,pid:100,window:12},Unread{source:Messenger::Wechat,pid:200,window:22},Unread::of(Messenger::Qq)]);
    assert_eq!((h.app.unread_tag(0),h.app.unread_tag(1),h.app.unread_tag(2)),(Some(1),Some(2),None),"只有同一来源多个账号才编号");
    h.app.alerts.clear();h.app.notice=None;
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.5);
    let rect=|h:&H,name:&str|h.ctx.read_response(egui::Id::new(("card",true,name.to_string()))).unwrap_or_else(||panic!("{name}")).rect;
    let panel=h.app.island_rect(Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),crate::island::PW,h.app.expanded_height());
    let (a,b,q)=(rect(&h,"微信"),rect(&h,"微信2"),rect(&h,"QQ"));
    assert!(a.bottom()<=b.top()+0.5 && b.bottom()<=q.top()+0.5);
    let inset=h.app.unread_top_inset();assert!(inset>0.0,"顶部停靠时胶囊离开屏幕上沿");
    assert!((a.top()-panel.top()-inset-2.0).abs()<1.0 && (q.bottom()-panel.bottom()+2.0).abs()<1.0,"三个图标时胶囊下沿与面板对齐 {a:?} {q:?} {panel:?}");
    h.cmds.clear();h.click(b.center());
    assert!(h.cmds.contains(&Cmd::ViewMessenger(Unread{source:Messenger::Wechat,pid:200,window:22})));
    h.app.viewed_client(Messenger::Wechat,200);
    assert_eq!(h.app.pending_messengers,[Unread{source:Messenger::Wechat,pid:100,window:12},Unread::of(Messenger::Qq)]);
    assert_eq!(h.app.unread_tag(0),None,"剩一个账号就不再编号");
    h.app.viewed_client(Messenger::Qq,4321);
    assert!(h.app.pending_messengers.contains(&Unread::of(Messenger::Qq)),"查看其他 QQ 不能清除无法归属的提醒");
    h.app.discard_unread(Unread::of(Messenger::Qq));
    assert_eq!(h.app.pending_messengers,[Unread{source:Messenger::Wechat,pid:100,window:12}]);
    // 四个以上：胶囊仍与面板等高，滚轮翻到后面的图标。
    for pid in [300,400,500]{h.app.receive_wechat_reminder_from(pid,0);}
    h.hover(Pos2::new(WIN_W/2.0,12.0),0.2);
    let capsule=rect(&h,"微信");
    let last=|h:&H|rect(h,"微信4");
    let before=last(&h);
    assert!(before.height()<1.0 || before.top()>=panel.bottom()-3.0,"第四个先藏在下面");
    h.hover(Pos2::new(capsule.center().x,panel.top()+30.0),0.1);
    for _ in 0..10{h.wheel(-40.0);}
    h.frame(vec![]);
    assert!(last(&h).height()>20.0 && last(&h).bottom()<=panel.bottom(),"滚动后露出第四个 {:?}",last(&h));
}

#[test]
fn messenger_notifications_stay_right_and_move_outside_expanded_panel(){
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.wechat_notifications=true;
    h.frame(vec![]);h.app.receive_wechat_reminder();
    for _ in 0..360{h.frame(vec![]);}
    let badge=|h:&H|h.ctx.read_response(egui::Id::new(("card",true,"微信"))).unwrap().rect;
    assert!(badge(&h).left()>WIN_W/2.0);
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.5);
    assert!(h.app.open && badge(&h).left()>WIN_W/2.0);
    assert!((badge(&h).left()-(WIN_W/2.0+crate::island::PW/2.0+crate::island::UNREAD_GAP)).abs()<1.0,"展开后与面板留一小段间距 {:?}",badge(&h));
    let center=badge(&h).center();
    assert!(h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Rect(r)
        if r.rect.contains(center) && (r.rect.width()-crate::island::UNREAD_W).abs()<0.1 && (r.rect.height()-crate::island::UNREAD_W).abs()<0.1)),"一个图标时是正圆");
    h.click(badge(&h).center());
    assert!(h.cmds.contains(&crate::island::Cmd::ViewMessenger(crate::island::Unread::of(crate::island::Messenger::Wechat))));
}

#[test]
fn messenger_notifications_align_with_system_banner(){
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.wechat_notifications=true;
    h.app.settings.read_notifications=true;
    h.frame(vec![]);h.app.receive_wechat_reminder();
    for _ in 0..360{h.frame(vec![]);}
    h.app.receive_notification("ChatGPT".into(),"ChatGPT".into(),"测试".into());
    for _ in 0..60{h.frame(vec![]);}
    let rect=|name|h.ctx.read_response(egui::Id::new(("card",true,name))).unwrap().rect;
    let badge=rect("微信");let close=rect("external-dismiss");
    assert!(badge.left()>close.right());
    assert!((badge.center().y-close.center().y).abs()<1.1);
}

#[test]
fn messenger_notifications_side_docks_keep_one_column(){
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dock in [crate::model::Dock::Left,crate::model::Dock::Right]{
        let mut h=H::new();h.app.settings.dock=dock;h.app.settings.wechat_notifications=true;h.app.settings.read_notifications=true;
        h.app.receive_wechat_reminder();h.app.receive_notification("QQ".into(),"QQ".into(),"测试".into());
        for _ in 0..720{h.frame(vec![]);}
        let win=Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H));
        let stack=h.ctx.read_response(egui::Id::new(("card",true,"messenger-stack"))).unwrap().rect;
        let bar=h.app.island_rect(win,h.app.w.pos,h.app.h.pos);
        assert!((stack.center().x-bar.center().x).abs()<1.0 && stack.top()>=bar.bottom()-0.5,"收起时叠放在竖胶囊下方");
        h.click(stack.center());for _ in 0..90{h.frame(vec![]);}
        let rect=|name|h.ctx.read_response(egui::Id::new(("card",true,name))).unwrap().rect;
        let a=rect("微信");let b=rect("QQ");
        assert_eq!(a.center().x,b.center().x);assert!(a.bottom()<=b.top()+0.01);
        let panel=h.app.island_rect(win,crate::island::PW,h.app.expanded_height());
        if dock==crate::model::Dock::Left {assert!(a.left()>=panel.right()+crate::island::UNREAD_GAP-0.5,"左停靠：胶囊在面板右侧");}
        else {assert!(a.right()<=panel.left()-crate::island::UNREAD_GAP+0.5,"右停靠：胶囊在面板左侧");}
        assert!((a.top()-panel.top()-2.0).abs()<1.0 && h.app.unread_top_inset()==0.0,"侧边停靠时顶端与面板对齐");
    }
}

#[test]
#[ignore="按需导出微信 / QQ 未读胶囊的实际画面"]
fn unread_gallery(){
    use crate::island::Messenger;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_FEATURE_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let fill=|h:&mut H,n:usize|{
        h.app.settings.wechat_notifications=true;h.app.settings.read_notifications=true;h.app.settings.pet=false;
        let all=[(Messenger::Wechat,101),(Messenger::Qq,0),(Messenger::Wechat,202),(Messenger::Wechat,303),(Messenger::Qq,0)];
        for (source,pid) in all.into_iter().take(n){
            if source==Messenger::Wechat {h.app.receive_wechat_reminder_from(pid,0);} else {h.app.receive_notification("QQ".into(),"QQ".into(),"测试".into());}
        }
        h.app.alerts.clear();h.app.notice=None;h.app.notification_bell_until=0.0;
    };
    for dock in [crate::model::Dock::Top,crate::model::Dock::Left,crate::model::Dock::Right]{
        let key=dock.key();
        for n in [1,2,3,4]{
            let mut h=H::new();h.app.settings.dock=dock;fill(&mut h,n);
            let side=dock.is_side();
            let (w,hgt)=if side {(crate::island::layout::dock::SIDE_W,crate::island::layout::dock::SIDE_H)} else {(crate::island::CANVAS_W,WIN_H)};
            let mut r=if side {feature_raster_sized(w,hgt)} else {feature_raster()};
            let away=Some(Pos2::new(if side {w/2.0} else {WIN_W/2.0},hgt-6.0));
            let near=if dock==crate::model::Dock::Left {Pos2::new(8.0,hgt/2.0)} else if dock==crate::model::Dock::Right {Pos2::new(w-8.0,hgt/2.0)} else {Pos2::new(WIN_W/2.0,12.0)};
            if n!=3 {shoot_unread(&mut h,&mut r,&dest,&format!("unread-{key}-{n}-collapsed"),60,away);}
            shoot_unread(&mut h,&mut r,&dest,&format!("unread-{key}-{n}-open"),80,Some(near));
        }
        // 微信多开：两个微信账号；两个微信账号 + QQ。
        for (tag,qq) in [("multi2",false),("multi3",true)]{
            let mut h=H::new();h.app.settings.dock=dock;fill(&mut h,0);
            h.app.receive_wechat_reminder_from(101,0);h.app.receive_wechat_reminder_from(202,0);
            if qq {h.app.receive_notification("QQ".into(),"QQ".into(),"测试".into());}
            h.app.alerts.clear();h.app.notice=None;h.app.notification_bell_until=0.0;
            let side=dock.is_side();
            let (w,hgt)=if side {(crate::island::layout::dock::SIDE_W,crate::island::layout::dock::SIDE_H)} else {(crate::island::CANVAS_W,WIN_H)};
            let mut r=if side {feature_raster_sized(w,hgt)} else {feature_raster()};
            let away=Some(Pos2::new(if side {w/2.0} else {WIN_W/2.0},hgt-6.0));
            let near=if dock==crate::model::Dock::Left {Pos2::new(8.0,hgt/2.0)} else if dock==crate::model::Dock::Right {Pos2::new(w-8.0,hgt/2.0)} else {Pos2::new(WIN_W/2.0,12.0)};
            shoot_unread(&mut h,&mut r,&dest,&format!("unread-{key}-{tag}-collapsed"),60,away);
            shoot_unread(&mut h,&mut r,&dest,&format!("unread-{key}-{tag}-open"),80,Some(near));
        }
    }
}
