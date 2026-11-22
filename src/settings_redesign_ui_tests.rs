use super::*;

/// 旧版「新邮件提醒」关着的用户：升级后邮箱模块关闭，内部开关恢复为开，不会突然开始收信。
#[test]
fn merged_mail_switch_migrates_old_off_to_module_off(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.settings.visible_pages[crate::app::MAIL_PAGE]=true;h.app.settings.mail_enabled=false;
    assert!(!h.app.mail_active());
    h.app.migrate_merged_switches();
    assert!(!h.app.settings.visible_pages[crate::app::MAIL_PAGE],"旧的新邮件提醒关闭，迁移为邮箱模块关闭");
    assert!(h.app.settings.mail_enabled,"内部开关恢复为开，以后只由模块开关控制");
    assert!(!h.app.mail_active(),"迁移前后都不收信");
    assert!(crate::model::Settings::load().mail_enabled,"迁移结果要保存");
    h.app.set_page_visible(crate::app::MAIL_PAGE,true);
    assert!(h.app.mail_active(),"重新打开模块即恢复收信，不再需要第二个开关");
}

#[test]
fn merged_quota_switch_migrates_old_off_to_module_off(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.settings.visible_pages[crate::app::QUOTA_PAGE]=true;h.app.quota.config.enabled=false;
    h.app.migrate_merged_switches();
    assert!(!h.app.settings.visible_pages[crate::app::QUOTA_PAGE]);
    assert!(h.app.quota.config.enabled);
    assert!(!h.app.quota_active());
}

/// 已经开着的用户不受影响。
#[test]
fn merged_switches_keep_enabled_users_unchanged(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    let pages=h.app.settings.visible_pages;
    h.app.settings.mail_enabled=true;h.app.quota.config.enabled=true;
    h.app.migrate_merged_switches();
    assert_eq!(h.app.settings.visible_pages,pages);
    assert!(h.app.settings.mail_enabled && h.app.quota.config.enabled);
}

/// 只剩邮箱一个页面时模块关不掉：保持原来的“页面在、不收信”，不改变行为。
#[test]
fn merged_mail_switch_keeps_last_page_behavior(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.settings.visible_pages=[false;12];h.app.settings.visible_pages[crate::app::MAIL_PAGE]=true;
    h.app.settings.mail_enabled=false;
    h.app.migrate_merged_switches();
    assert!(h.app.settings.visible_pages[crate::app::MAIL_PAGE]);
    assert!(!h.app.settings.mail_enabled,"关不掉模块时保留旧的内部开关");
    assert!(!h.app.mail_active());
}

/// 外观、功能页面并入岛体；专注/健康、待办/置物架拆开；旧序号仍能打开对应页面。
#[test]
fn settings_tab_titles_follow_new_structure(){
    use crate::extras::settings_tab_group_for_tests as group;
    assert_eq!(group(2),"专注");assert_eq!(group(3),"健康");
    assert_eq!(group(4),"待办");assert_eq!(group(5),"置物架");
    assert_eq!(group(6),"提醒");assert_eq!(group(7),"邮箱");assert_eq!(group(8),"日程");
    assert_eq!(group(0),"通用");assert_eq!(group(13),"岛体","外观并入岛体");
    assert_eq!(group(14),"行为");assert_eq!(group(10),"岛体","功能页面并入岛体");
    assert_eq!(group(9),"关于");assert_eq!(group(15),"AI 会话");
    assert_eq!(group(16),"岛体");assert_eq!(group(crate::extras::SETTINGS_TAB_CLIPBOARD),"剪贴板");
}

/// 每个分页都能正常绘制（含功能关闭时的提示），不会因为挪动布局而崩溃。
#[test]
fn settings_every_tab_renders_with_modules_on_and_off(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for on in [true,false] {
        let mut h=H::new();h.app.open_settings();h.ctx.set_embed_viewports(true);
        if !on {for page in 0..12 {if page!=crate::app::MUSIC_PAGE {h.app.settings.visible_pages[page]=false;}}}
        for tab in 0..=crate::extras::SETTINGS_TAB_WINDOWS {
            h.app.settings_tab=tab;
            for _ in 0..2 {let _=h.ctx.run(egui::RawInput::default(),|ctx|h.app.settings_ui(ctx,false));}
            assert_eq!(h.app.settings_tab,tab,"绘制分页 {tab} 不应自行跳转");
        }
    }
}

/// Windows 通知的系统开关放在提醒页：开启即自动监听，胶囊显示监听状态。
#[test]
fn reminder_windows_section_has_system_switch_and_listener_chip(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let texts=settings_text(crate::extras::SETTINGS_TAB_REMINDERS);
    for label in ["Windows 通知","系统通知","关闭后所有应用的横幅都会被 Windows 拦截；开启后岛仔自动监听。"] {
        assert!(texts.iter().any(|text|text==label),"提醒页缺少「{label}」");
    }
    assert!(texts.iter().any(|text|text=="监听中"||text=="未监听"),"系统通知需要监听状态胶囊");
}

/// 设置窗口拉到最小（600×450）时，侧栏 17 项都在可见区内，「关于」不会被挤出去。
#[test]
fn settings_sidebar_keeps_about_visible_at_minimum_height(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.open_settings();h.ctx.set_embed_viewports(false);
    let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
    egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
        let id=viewport.ids.this;
        let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,450.0))),
            viewports:std::iter::once((id,egui::ViewportInfo::default())).collect(),..Default::default()};
        *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
    });
    for _ in 0..3 {let _=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));}
    let frame=output.lock().unwrap().take().unwrap();
    for label in ["应用","通用","岛体","行为","提醒","功能","天气","剪贴板","其他","关于"] {
        let rect=frame.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==label && t.pos.x<180.0=>Some(t.visual_bounding_rect()),_=>None})
            .unwrap_or_else(||panic!("侧栏缺少 {label}"));
        assert!(rect.bottom()<=450.0,"{label} 超出窗口底部：{rect:?}");
    }
}

fn settings_text(tab:usize)->Vec<String>{settings_text_with(tab,|_|{})}
fn settings_text_with(tab:usize,prepare:impl FnOnce(&H))->Vec<String>{
    settings_frame_with(tab,prepare).shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)=>Some(t.galley.text().to_string()),_=>None}).collect()
}
/// 渲染一帧设置子视口，返回这一帧的形状。
fn settings_frame_mut(h:&mut H,events:Vec<egui::Event>)->egui::FullOutput{
    let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
    h.t+=0.02;let time=h.t;
    egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
        let id=viewport.ids.this;
        let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,2600.0))),time:Some(time),events:events.clone(),viewports:std::iter::once((id,egui::ViewportInfo::default())).collect(),..Default::default()};
        *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
    });
    let _=h.ctx.run(egui::RawInput::default(),|ctx|h.app.settings_ui(ctx,false));
    let frame=output.lock().unwrap().take().unwrap();
    frame
}

fn settings_frame_with(tab:usize,prepare:impl FnOnce(&H))->egui::FullOutput{
    let mut h=H::new();h.app.settings_tab=tab;h.app.open_settings();h.ctx.set_embed_viewports(false);
    prepare(&h);
    let mut frame=settings_frame_mut(&mut h,vec![]);
    for _ in 1..3 {frame=settings_frame_mut(&mut h,vec![]);}
    frame
}

/// 「岛体」页收拢外观、岛上显示和功能页排布；展开、避让等行为项在「行为」页。
#[test]
fn body_page_has_appearance_residents_and_feature_pages(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let body=settings_text(crate::extras::SETTINGS_TAB_BODY);
    for label in ["岛体外观","岛上显示","常驻显示","事件显示","功能页面","皮肤","形状","大小","显示屏","停靠","恢复居中","岛体预览","恢复默认","AI 会话状态灯","岛仔吉祥物","电量","时间","勿扰模式","天气","音乐","专注","邮箱","健康","待办","置物架","日程","剪贴板"] {
        assert!(body.iter().any(|text|text==label),"岛体页缺少「{label}」");
    }
    assert!(!body.iter().any(|text|text.starts_with("组件开关")),"分组标题下不应再套一层折叠区");
    assert!(!body.iter().any(|text|text=="基础组件"),"岛上显示已改为两个分组");
    assert!(!body.iter().any(|text|text=="岛体的外观、常驻内容与功能页排布。行为相关设置在「行为」。"),"页标题下不放说明文字");
    for label in ["展开与收起","自动避让","极简刘海","全屏时隐藏"] {
        assert!(!body.iter().any(|text|text==label),"岛体页不应显示行为项「{label}」");
    }
    let behavior=settings_text(crate::extras::SETTINGS_TAB_BEHAVIOR);
    for label in ["岛体组件","横幅提醒","系统提醒","展开与收起","避让","动画速度","展开后停留","提醒总开关","横幅停留","全屏时隐藏","顶栏适配","自动避让","常用浏览器"] {
        assert!(behavior.iter().any(|text|text==label),"行为页缺少「{label}」");
    }
    for label in ["通知","提醒方式"] {
        assert!(!behavior.iter().any(|text|text==label),"行为页不应再显示「{label}」");
    }
    assert!(!behavior.iter().any(|text|text=="OLED 像素偏移"),"OLED 像素偏移已并入通用-高级");
    assert!(!behavior.iter().any(|text|text=="位置"),"位置已并入岛体外观");
    assert!(!behavior.iter().any(|text|text=="极简刘海"),"极简刘海已移除");
    let general=settings_text(0);
    for label in ["自动避让","岛体外观","岛上显示"] {
        assert!(!general.iter().any(|text|text==label),"通用页不应显示「{label}」");
    }
    assert!(general.iter().any(|text|text=="OLED 像素偏移"),"OLED 像素偏移应在通用-高级");
}

/// 「岛上显示」分成常驻显示与事件显示；默认设置下音乐在常驻组，天气/额度/专注在事件组。
#[test]
fn resident_groups_show_persistent_and_event_components(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let frame=settings_frame_with(crate::extras::SETTINGS_TAB_BODY,|_|{});
    let body:Vec<&str>=frame.shapes.iter().filter_map(|s|match &s.shape{
        egui::Shape::Text(t) if t.pos.x>200.0=>Some(t.galley.text()),
        _=>None,
    }).collect();
    let pos=|label:&str|body.iter().position(|text|*text==label).unwrap_or_else(||panic!("缺少「{label}」"));
    assert!(pos("常驻显示")<pos("事件显示"));
    for label in ["AI 会话状态灯","岛仔吉祥物","电量","时间","勿扰模式","音乐","邮箱","健康","待办","置物架","日程"] {
        assert!(pos("常驻显示")<pos(label) && pos(label)<pos("事件显示"),"「{label}」应在常驻显示组");
    }
    for label in ["天气","AI 额度","专注"] {
        assert!(pos("事件显示")<pos(label),"「{label}」应在事件显示组");
    }
}

/// 分组标题比子项更醒目；子项因拖动手柄向右缩进。
#[test]
fn resident_group_rows_are_nested_under_the_group_label(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let frame=settings_frame_with(crate::extras::SETTINGS_TAB_BODY,|_|{});
    let text=|label:&str|frame.shapes.iter().find_map(|s|match &s.shape{
        egui::Shape::Text(t) if t.galley.text()==label && t.pos.x>200.0=>Some(t),
        _=>None,
    }).unwrap_or_else(||panic!("缺少「{label}」"));
    let size=|label:&str|text(label).galley.job.sections.first().map(|s|s.format.font_id.size).unwrap_or(0.0);
    assert!(size("时间")<size("常驻显示"),"分组子项标题应更小：{} vs {}",size("时间"),size("常驻显示"));
    assert!(text("时间").visual_bounding_rect().left()>text("常驻显示").visual_bounding_rect().left()+8.0,"分组子项应缩进");
}

/// 「提醒事件」沿用「岛上显示」的二级分组样式：子项标题更小并缩进一级。
#[test]
fn agent_notify_groups_use_nested_rows_like_island_groups(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let frame=settings_frame_with(15,|_|{});
    let text=|label:&str|frame.shapes.iter().find_map(|s|match &s.shape{
        egui::Shape::Text(t) if t.galley.text()==label && t.pos.x>200.0=>Some(t),
        _=>None,
    }).unwrap_or_else(||panic!("缺少「{label}」"));
    let size=|label:&str|text(label).galley.job.sections.first().map(|s|s.format.font_id.size).unwrap_or(0.0);
    for (group,row) in [("展开提醒","用户选择"),("横幅提醒","完成")] {
        assert!(size(row)<size(group),"「{group}」子项标题应更小：{} vs {}",size(row),size(group));
        assert!(text(row).visual_bounding_rect().left()>text(group).visual_bounding_rect().left()+8.0,"「{group}」子项应缩进");
    }
}

/// 两个分组默认展开；点一个分组标题只收起该组。
#[test]
fn resident_groups_default_expanded_and_collapse_independently(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings_tab=crate::extras::SETTINGS_TAB_BODY;h.app.open_settings();h.ctx.set_embed_viewports(false);
    let mut frame=settings_frame_mut(&mut h,vec![]);
    for _ in 0..2 {frame=settings_frame_mut(&mut h,vec![]);}
    let has=|frame:&egui::FullOutput,label:&str|frame.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==label && t.pos.x>200.0));
    assert!(has(&frame,"常驻显示") && has(&frame,"事件显示"));
    assert!(has(&frame,"时间") && has(&frame,"天气"),"两个分组默认展开");
    let point=frame.shapes.iter().find_map(|s|match &s.shape{
        egui::Shape::Text(t) if t.galley.text()=="常驻显示" && t.pos.x>200.0=>Some(t.visual_bounding_rect().left_center()),
        _=>None,
    }).unwrap();
    let button=|pressed|egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    settings_frame_mut(&mut h,vec![egui::Event::PointerMoved(point)]);
    settings_frame_mut(&mut h,vec![button(true)]);
    settings_frame_mut(&mut h,vec![button(false)]);
    frame=settings_frame_mut(&mut h,vec![]);
    assert!(!has(&frame,"时间"),"点「常驻显示」后收起该组");
    assert!(has(&frame,"天气"),"「事件显示」不受影响");
    assert!(has(&frame,"常驻显示"),"收起后标题仍在");
}

/// 「提醒」页只做跨来源提醒策略；事件开关与提示音在「AI 会话」页。
#[test]
fn reminder_page_keeps_policy_and_ai_page_owns_event_hooks(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let reminders=settings_text(crate::extras::SETTINGS_TAB_REMINDERS);
    for label in ["系统状态","即时通讯","AI 会话消息","Windows 通知","自动展开","会话数","最近消息行数","会话记录保留"] {
        assert!(reminders.iter().any(|text|text==label),"提醒页缺少「{label}」");
    }
    for label in ["提醒方式","横幅停留","重复提醒","提醒总开关"] {
        assert!(!reminders.iter().any(|text|text==label),"提醒页不应再显示「{label}」");
    }
    assert!(!reminders.iter().any(|text|text=="消息或事件触发时，岛怎么提醒。"),"页标题下不放说明文字");
    for label in ["用户选择","展开提醒","横幅提醒","提示音","中断","闲置"] {
        assert!(!reminders.iter().any(|text|text==label),"提醒页不应再显示「{label}」");
    }
    for label in ["检查间隔","服务"] {
        assert!(!reminders.iter().any(|text|text==label),"提醒页不应包含功能模块设置「{label}」");
    }
    let agent=settings_text(15);
    for label in ["Hooks","提醒事件","展开提醒","横幅提醒","用户选择","审批","完成","中断","失败","闲置","提示音","开始","提交","错误"] {
        assert!(agent.iter().any(|text|text==label),"AI 会话页缺少「{label}」");
    }
    for label in ["横幅停留","提醒方式","重复提醒","显示","会话数","最近消息行数"] {
        assert!(!agent.iter().any(|text|text==label),"AI 会话页不应显示「{label}」");
    }
}

/// 专注/健康、待办/置物架拆成独立页面，剪贴板单独成页。
#[test]
fn feature_pages_split_into_separate_settings_pages(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let focus=settings_text(2);
    assert!(!focus.iter().any(|text|text=="喝水提醒"),"专注页不应包含健康设置");
    let health=settings_text(3);
    for label in ["喝水提醒","久坐提醒","提醒时段"] {
        assert!(health.iter().any(|text|text==label),"健康页缺少「{label}」");
    }
    assert!(!health.iter().any(|text|text=="休息"),"健康页不应包含专注设置");
    let todo=settings_text(4);
    assert!(!todo.iter().any(|text|text.contains("当前")),"待办页不应包含置物架设置");
    let shelf=settings_text(5);
    assert!(shelf.iter().any(|text|text.contains("当前")),"置物架页应显示条目数");
    assert!(!shelf.iter().any(|text|text=="记录剪贴板"),"置物架页不应包含剪贴板设置");
    let clipboard=settings_text(crate::extras::SETTINGS_TAB_CLIPBOARD);
    for label in ["记录剪贴板","打开剪贴板"] {
        assert!(clipboard.iter().any(|text|text==label),"剪贴板页缺少「{label}」");
    }
    assert!(!clipboard.iter().any(|text|text=="在置物架的「剪贴板」分栏里查看最近复制的文字。"),"页标题下不放说明文字");
}

#[test]
fn island_body_tab_is_not_a_feature_page(){
    let pages:Vec<_>=(0..=crate::extras::SETTINGS_TAB_WINDOWS).filter_map(crate::extras::settings_tab_module).collect();
    assert_eq!(crate::extras::settings_tab_module(crate::extras::SETTINGS_TAB_BODY),None);
    assert_eq!(crate::extras::settings_tab_module(crate::extras::SETTINGS_TAB_BEHAVIOR),None);
    assert_eq!(crate::extras::settings_tab_module(crate::extras::SETTINGS_TAB_CLIPBOARD),Some(crate::app::CLIPBOARD_PAGE));
    assert_eq!(pages.len(),crate::app::PAGE_NAMES.len(),"每个功能页各有一个设置分页");
}

#[test]
fn motion_and_hold_settings_default_clamp_and_persist(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let _h=H::new();
    use crate::model::{MotionSpeed,Settings};
    let defaults=Settings::default();
    assert_eq!((defaults.notice_hold_secs,defaults.page_hold_secs,defaults.motion_speed),(4.5,4.0,MotionSpeed::Normal));
    assert!(MotionSpeed::Slow.factor()<MotionSpeed::Normal.factor());
    assert!(MotionSpeed::Normal.factor()<MotionSpeed::Fast.factor());
    let path=crate::model::config_dir().join("settings.txt");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path,"notice_hold_secs=99\npage_hold_secs=oops\nmotion_speed=fast\n").unwrap();
    let loaded=Settings::load();
    assert_eq!((loaded.notice_hold_secs,loaded.page_hold_secs,loaded.motion_speed),(15.0,4.0,MotionSpeed::Fast));
    std::fs::write(&path,"notice_hold_secs=-3\npage_hold_secs=0\nmotion_speed=turbo\n").unwrap();
    let loaded=Settings::load();
    assert_eq!((loaded.notice_hold_secs,loaded.page_hold_secs,loaded.motion_speed),(1.0,1.0,MotionSpeed::Normal));
    std::fs::write(&path,"notice_hold_secs=NaN\n").unwrap();
    assert_eq!(Settings::load().notice_hold_secs,4.5);
    let mut saved=Settings::default();
    saved.notice_hold_secs=2.5;saved.page_hold_secs=6.0;saved.motion_speed=MotionSpeed::Slow;saved.save();
    let text=std::fs::read_to_string(&path).unwrap();
    for line in ["notice_hold_secs=2.5","page_hold_secs=6","motion_speed=slow"] {
        assert!(text.contains(line),"保存的设置缺少 {line}");
    }
    assert_eq!(Settings::load().motion_speed,MotionSpeed::Slow);
}

#[test]
fn resident_music_and_focus_switches_default_on_and_persist(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let _h=H::new();
    use crate::model::Settings;
    let defaults=Settings::default();
    assert!(defaults.show_music && defaults.show_focus);
    let path=crate::model::config_dir().join("settings.txt");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path,"show_music=0\nshow_focus=0\n").unwrap();
    let loaded=Settings::load();
    assert!(!loaded.show_music && !loaded.show_focus);
    let mut saved=Settings::default();
    saved.show_music=false;saved.show_focus=false;saved.save();
    let text=std::fs::read_to_string(&path).unwrap();
    for line in ["show_music=0","show_focus=0"] {assert!(text.contains(line),"保存的设置缺少 {line}");}
}

#[test]
fn hold_seconds_apply_to_banners_and_pages_only(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.t=10.0;h.app.settings.notice_hold_secs=2.5;
    h.app.settings.wechat_notifications=true;h.app.receive_wechat_reminder();
    assert_eq!(h.app.alerts.back().unwrap().duration,2.5);
    assert_eq!(h.app.notification_bell_until,0.0,"即时通讯不点亮铃铛");
    h.app.alerts.clear();h.app.settings.read_notifications=true;
    h.app.receive_notification("QQ".into(),"QQ".into(),"新消息".into());
    assert_eq!(h.app.alerts.back().unwrap().duration,2.5);
    assert_eq!(h.app.notification_bell_until,0.0,"QQ 不点亮铃铛");
    h.app.notification_apps.items.push(("weather.app".into(),"天气".into(),true));
    h.app.receive_notification("天气".into(),"weather.app".into(),"降雨提醒".into());
    assert_eq!(h.app.notification_bell_until,12.5,"非即时通讯的 Windows 通知点亮铃铛");
    h.app.agent_alert("AI","完成",crate::agent_ui::AgentAlert{key:"s1".into(),kind:crate::agent::store::NoticeKind::Done},h.app.t);
    assert_eq!(h.app.alerts.back().unwrap().duration,2.5,"AI 横幅使用横幅停留时长");
    h.app.settings.page_hold_secs=7.0;
    h.app.show_page(0);
    assert_eq!(h.app.forced_open_until_for_tests(),17.0);
    h.app.t=20.0;h.app.reveal_unread();
    assert_eq!(h.app.forced_open_until_for_tests(),27.0);
}
