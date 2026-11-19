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
    h.app.settings.visible_pages=[false;10];h.app.settings.visible_pages[crate::app::MAIL_PAGE]=true;
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
        if !on {for page in 0..10 {if page!=crate::app::MUSIC_PAGE {h.app.settings.visible_pages[page]=false;}}}
        for tab in 0..=crate::extras::SETTINGS_TAB_CLIPBOARD {
            h.app.settings_tab=tab;
            for _ in 0..2 {let _=h.ctx.run(egui::RawInput::default(),|ctx|h.app.settings_ui(ctx,false));}
            assert_eq!(h.app.settings_tab,tab,"绘制分页 {tab} 不应自行跳转");
        }
    }
}

/// 设置窗口拉到最小（600×450）时，侧栏 16 项都在可见区内，「关于」不会被挤出去。
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

fn settings_text(tab:usize)->Vec<String>{
    let mut h=H::new();h.app.settings_tab=tab;h.app.open_settings();h.ctx.set_embed_viewports(false);
    let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
    egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
        let id=viewport.ids.this;
        let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,1600.0))),
            viewports:std::iter::once((id,egui::ViewportInfo::default())).collect(),..Default::default()};
        *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
    });
    for _ in 0..3 {let _=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));}
    let frame=output.lock().unwrap().take().unwrap();
    let texts=frame.shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)=>Some(t.galley.text().to_string()),_=>None}).collect();
    texts
}

/// 「岛体」页收拢外观、岛上显示和功能页排布；展开、避让等行为项在「行为」页。
#[test]
fn body_page_has_appearance_residents_and_feature_pages(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let body=settings_text(crate::extras::SETTINGS_TAB_BODY);
    for label in ["岛体外观","岛上显示","功能页面","皮肤","形状","大小","显示屏","停靠","恢复居中","岛仔吉祥物","岛体预览","恢复默认","时间","天气"] {
        assert!(body.iter().any(|text|text==label),"岛体页缺少「{label}」");
    }
    assert!(!body.iter().any(|text|text=="岛体的外观、常驻内容与功能页排布。行为相关设置在「行为」。"),"页标题下不放说明文字");
    for label in ["展开与收起","自动避让","极简刘海","全屏时隐藏"] {
        assert!(!body.iter().any(|text|text==label),"岛体页不应显示行为项「{label}」");
    }
    let behavior=settings_text(crate::extras::SETTINGS_TAB_BEHAVIOR);
    for label in ["展开与收起","避让","动画速度","展开后停留","全屏时隐藏","顶栏适配","自动避让","常用浏览器"] {
        assert!(behavior.iter().any(|text|text==label),"行为页缺少「{label}」");
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

/// 「提醒」页只做跨来源提醒策略；事件开关与提示音在「AI 会话」页。
#[test]
fn reminder_page_keeps_policy_and_ai_page_owns_event_hooks(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let reminders=settings_text(crate::extras::SETTINGS_TAB_REMINDERS);
    for label in ["提醒方式","系统状态","即时通讯","AI 会话消息","Windows 通知","横幅停留","自动展开","重复提醒","会话数","最近消息行数","会话记录保留"] {
        assert!(reminders.iter().any(|text|text==label),"提醒页缺少「{label}」");
    }
    assert!(!reminders.iter().any(|text|text=="消息或事件触发时，岛怎么提醒。"),"页标题下不放说明文字");
    for label in ["等待用户选择","提示音","中断","闲置"] {
        assert!(!reminders.iter().any(|text|text==label),"提醒页不应再显示「{label}」");
    }
    for label in ["检查间隔","服务"] {
        assert!(!reminders.iter().any(|text|text==label),"提醒页不应包含功能模块设置「{label}」");
    }
    let agent=settings_text(15);
    for label in ["Hooks","提醒事件","提示音","等待用户选择","完成","中断","失败","闲置","开始","提交","审批","错误"] {
        assert!(agent.iter().any(|text|text==label),"AI 会话页缺少「{label}」");
    }
    for label in ["横幅停留","提醒方式","显示","会话数","最近消息行数"] {
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
    let pages:Vec<_>=(0..=crate::extras::SETTINGS_TAB_CLIPBOARD).filter_map(crate::extras::settings_tab_module).collect();
    assert_eq!(crate::extras::settings_tab_module(crate::extras::SETTINGS_TAB_BODY),None);
    assert_eq!(crate::extras::settings_tab_module(crate::extras::SETTINGS_TAB_BEHAVIOR),None);
    assert_eq!(crate::extras::settings_tab_module(crate::extras::SETTINGS_TAB_CLIPBOARD),None);
    assert_eq!(pages.len(),crate::app::PAGE_NAMES.len(),"功能页到设置页的映射数量不变");
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
fn hold_seconds_apply_to_banners_and_pages_only(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.t=10.0;h.app.settings.notice_hold_secs=2.5;
    h.app.settings.wechat_notifications=true;h.app.receive_wechat_reminder();
    assert_eq!(h.app.alerts.back().unwrap().duration,2.5);
    assert_eq!(h.app.notification_bell_until,12.5);
    h.app.alerts.clear();h.app.settings.read_notifications=true;
    h.app.receive_notification("QQ".into(),"QQ".into(),"新消息".into());
    assert_eq!(h.app.alerts.back().unwrap().duration,2.5);
    h.app.agent_alert("AI","完成",6.0);
    assert_eq!(h.app.alerts.back().unwrap().duration,6.0,"AI 会话提醒使用事件自己的时长");
    h.app.settings.page_hold_secs=7.0;
    h.app.show_page(0);
    assert_eq!(h.app.forced_open_until_for_tests(),17.0);
    h.app.t=20.0;h.app.reveal_unread();
    assert_eq!(h.app.forced_open_until_for_tests(),27.0);
}
