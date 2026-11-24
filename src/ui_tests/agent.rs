//! AI 会话与审批。
use super::*;

#[test]
fn ai_stale_hides_without_expanding(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.notifications=true;
    h.app.agent.config.enabled=true;
    h.app.settings.visible_pages[crate::app::AI_PAGE]=true;
    h.app.agent.bridge=crate::agent::bridge::Bridge::start(h.ctx.clone()).ok();
    let (port,token)=h.app.agent.bridge.as_ref().map(|bridge|(bridge.port,bridge.token.clone())).unwrap();
    post_agent_event(port,&token,"SessionStart",serde_json::json!({"session_id":"ui-stale","cwd":"E:\\demo\\project"}));
    post_agent_event(port,&token,"UserPromptSubmit",serde_json::json!({"session_id":"ui-stale","prompt":"wait"}));
    for _ in 0..60 {h.frame(vec![]);if !h.app.agent.store.visible().is_empty(){break;}}
    assert!(!h.app.open&&!h.app.agent.detail);
    h.app.t=700.0;
    h.app.agent_tick(&h.ctx);
    h.app.t=731.0;
    h.app.agent_tick(&h.ctx);
    assert!(!h.app.open&&!h.app.agent.detail,"可能中断不应展开");
    assert!(h.app.notice.is_none()&&h.app.alerts.is_empty(),"可能中断不应出横幅");
    assert!(h.app.agent.store.visible().is_empty(),"提醒后会话从列表隐藏");
}

#[test]
fn ai_question_keeps_detail_open_until_answered(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.agent.bridge=crate::agent::bridge::Bridge::start(h.ctx.clone()).ok();
    let (port,token)=h.app.agent.bridge.as_ref().map(|bridge|(bridge.port,bridge.token.clone())).unwrap();
    post_agent_hook(port,&token,"ui-q1");
    for _ in 0..600 {h.frame(vec![]);if h.app.open&&h.app.agent.detail{break;}}
    assert!(h.app.open&&h.app.agent.detail,"等待回答应直接展开详情");
    assert!(h.app.notice.is_none()&&h.app.alerts.is_empty(),"等待回答不应先出横幅");
    assert_eq!(h.app.page,crate::app::AI_PAGE);
    assert_eq!(h.app.agent.selected.as_deref(),Some("claude:ui-q1"));
    h.cursor=None;
    for _ in 0..360 {h.frame(vec![]);}
    assert!(h.app.open,"光标离开和停留时长都不应自动收起提问详情");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="甲")),"选项直接画在详情里");
    h.click_text("甲");
    h.click_text("提交");
    assert_eq!(h.app.agent.store.waiting_count(),0,"在岛上直接选择即可回答");
    assert!(!h.app.agent_waiting_pinned());
    assert!(!h.app.agent.detail);
}

#[test]
fn ai_question_inline_input_enter_keeps_text_and_does_not_submit(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.agent.bridge=crate::agent::bridge::Bridge::start(h.ctx.clone()).ok();
    let (port,token)=h.app.agent.bridge.as_ref().map(|bridge|(bridge.port,bridge.token.clone())).unwrap();
    post_agent_hook(port,&token,"ui-free");
    for _ in 0..600 {h.frame(vec![]);if h.app.open&&h.app.agent.detail{break;}}
    assert!(h.app.agent.detail,"提问详情应已打开");
    for _ in 0..120 {h.frame(vec![]);if h.app.reveal>0.95 {break;}}
    assert!((h.app.expanded_height()-(crate::app::STATUS_H+214.0)).abs()<0.01,"两个选项时详情不应按最大高度展开");
    let (_,free_y)=h.app.agent_free_row().expect("自由输入行应已画出");
    h.click(H::local(200.0,free_y+11.0));
    h.frame(vec![]);
    assert!(h.app.agent.free_focus,"点击自由输入行应获得焦点");
    h.frame(vec![egui::Event::Text("hello".into())]);
    assert_eq!(h.app.agent.free,"hello","文字应先进入输入框");
    h.frame(vec![egui::Event::Key{key:egui::Key::Enter,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    h.frame(vec![]);
    assert!(!h.app.agent.free_focus,"回车应结束输入并释放焦点");
    assert_eq!(h.app.agent.free,"hello","回车后输入内容仍留在行内");
    assert_eq!(h.app.agent.store.waiting_count(),1,"回车不能提交");
    assert!(h.app.agent.detail,"回车后详情仍保持");
    h.click_text("提交");
    assert_eq!(h.app.agent.store.waiting_count(),0,"点提交才真正提交");
    assert!(!h.app.agent.detail);
}

#[test]
fn ai_detail_closes_when_pending_disappears(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.agent.config.notify.done=false;
    h.app.agent.bridge=crate::agent::bridge::Bridge::start(h.ctx.clone()).ok();
    let (port,token)=h.app.agent.bridge.as_ref().map(|bridge|(bridge.port,bridge.token.clone())).unwrap();
    post_agent_hook(port,&token,"ui-gone");
    for _ in 0..600 {h.frame(vec![]);if h.app.open&&h.app.agent.detail{break;}}
    assert!(h.app.agent.detail,"等待回答应展开详情");
    post_agent_event(port,&token,"Stop",serde_json::json!({"session_id":"ui-gone","last_assistant_message":"已处理"}));
    for _ in 0..600 {h.frame(vec![]);if !h.app.agent.detail{break;}}
    assert!(!h.app.agent.detail,"待处理事件消失后应退回列表");
    assert!(h.app.agent.selected.is_none());
    assert_eq!(h.app.page,crate::app::AI_PAGE);
}

#[test]
fn ai_approval_keeps_detail_open_until_decided(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.agent.bridge=crate::agent::bridge::Bridge::start(h.ctx.clone()).ok();
    let (port,token)=h.app.agent.bridge.as_ref().map(|bridge|(bridge.port,bridge.token.clone())).unwrap();
    post_agent_permission_hook(port,&token,"ui-p1");
    for _ in 0..600 {h.frame(vec![]);if h.app.open&&h.app.agent.detail{break;}}
    assert!(h.app.open&&h.app.agent.detail,"等待审批应直接展开详情");
    assert!(h.app.notice.is_none()&&h.app.alerts.is_empty(),"等待审批不应先出横幅");
    assert_eq!(h.app.page,crate::app::AI_PAGE);
    assert_eq!(h.app.agent.selected.as_deref(),Some("claude:ui-p1"));
    h.cursor=None;
    for _ in 0..360 {h.frame(vec![]);}
    assert!(h.app.open,"光标离开和停留时长都不应自动收起审批详情");
    assert!((h.app.expanded_height()-(crate::app::STATUS_H+186.0)).abs()<0.01,"单行命令的审批详情应收缩到一行高度");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="允许一次")),"审批按钮直接画在详情里");
    h.click_text("允许一次");
    assert_eq!(h.app.agent.store.waiting_count(),0,"在岛上直接批准即可放行");
    assert!(!h.app.agent_waiting_pinned());
    assert!(!h.app.agent.detail);
}

#[test]
fn ai_waiting_expands_directly(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.agent.bridge=crate::agent::bridge::Bridge::start(h.ctx.clone()).ok();
    let (port,token)=h.app.agent.bridge.as_ref().map(|bridge|(bridge.port,bridge.token.clone())).unwrap();
    post_agent_hook(port,&token,"ui-q2");
    for _ in 0..600 {h.frame(vec![]);if h.app.open&&h.app.agent.detail{break;}}
    assert!(h.app.open&&h.app.agent.detail,"等待回答直接展开详情");
    assert!(h.app.notice.is_none()&&h.app.alerts.is_empty(),"等待回答不排横幅");
    assert_eq!(h.app.page,crate::app::AI_PAGE);
    assert_eq!(h.app.agent.selected.as_deref(),Some("claude:ui-q2"));
}

#[test]
fn ai_waiting_expansion_waits_for_todo_editing(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.agent.bridge=crate::agent::bridge::Bridge::start(h.ctx.clone()).ok();
    let (port,token)=h.app.agent.bridge.as_ref().map(|bridge|(bridge.port,bridge.token.clone())).unwrap();
    h.app.adding=true;h.app.buf="草稿".into();
    post_agent_hook(port,&token,"ui-edit");
    for _ in 0..60 {h.frame(vec![]);}
    assert!(!h.app.open&&!h.app.agent.detail,"编辑时等待事件应排队");
    assert!(h.app.agent_open_after_notice.is_some(),"编辑结束后仍应展开");
    h.app.cancel_edit();
    for _ in 0..60 {h.frame(vec![]);if h.app.open&&h.app.agent.detail{break;}}
    assert!(h.app.open&&h.app.agent.detail,"编辑结束后应展开详情");
}

#[test]
fn ai_second_waiting_event_opens_after_the_first_is_answered(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.agent.bridge=crate::agent::bridge::Bridge::start(h.ctx.clone()).ok();
    let (port,token)=h.app.agent.bridge.as_ref().map(|bridge|(bridge.port,bridge.token.clone())).unwrap();
    post_agent_hook(port,&token,"ui-a");
    post_agent_hook(port,&token,"ui-b");
    for _ in 0..60 {h.frame(vec![]);if h.app.open&&h.app.agent.detail{break;}}
    assert!(h.app.agent.detail,"第一条等待事件应直接展开");
    let first=h.app.agent.selected.clone().unwrap();
    assert!(h.app.agent_open_after_notice.is_some(),"第二条等待事件应排队");
    for _ in 0..360 {h.frame(vec![]);if h.app.reveal>0.95&&h.app.h.settled(h.app.expanded_height()){break;}}
    assert!(h.app.agent.detail,"展开完成后详情应仍打开");
    h.click_text("甲");
    h.click_text("提交");
    for _ in 0..60 {h.frame(vec![]);if h.app.agent.detail&&h.app.agent.selected.as_deref()!=Some(first.as_str()){break;}}
    assert!(h.app.agent.detail,"答完第一条后应展开第二条");
    assert_ne!(h.app.agent.selected.as_deref(),Some(first.as_str()));
    assert_eq!(h.app.agent.store.waiting_count(),1);
}

#[test]
fn ai_sound_follows_master_sound_and_light_visibility(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.sound=true;
    assert!(h.app.agent_sound_allowed());
    h.app.settings.notifications=false;
    assert!(!h.app.agent_sound_allowed(),"提醒总开关关闭时不出声");
    h.app.settings.notifications=true;h.app.settings.sound=false;
    assert!(!h.app.agent_sound_allowed(),"声音关闭时不出声");
    h.app.settings.sound=true;h.app.agent.config.display.bar_indicator=false;
    assert!(!h.app.agent_sound_allowed(),"状态灯隐藏时不出声");
    h.app.agent.config.display.bar_indicator=true;
    assert!(h.app.agent_sound_allowed());
}

#[test]
fn collapsed_agent_light_opens_ai_page(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    for _ in 0..10 {h.frame(vec![]);}
    let light=h.ctx.read_response(egui::Id::new(("card",true,"agent-pill"))).expect("收起态应显示 AI 状态灯").rect;
    h.click(light.center());
    assert_eq!(h.app.page,crate::app::AI_PAGE);
    assert!(h.app.open,"点状态灯应直接打开 AI 页");
}

#[test]
fn side_agent_light_opens_ai_page(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let (w,hgt)=(crate::app::side_dock::SIDE_W,crate::app::side_dock::SIDE_H);
    let mut h=H::new();h.app.settings.dock=crate::model::Dock::Left;
    let away=Some(Pos2::new(w/2.0,hgt-5.0));
    for _ in 0..60 {side_frame(&mut h,away);}
    let item=h.ctx.read_response(egui::Id::new(("card",true,"side-agent"))).expect("侧边收起态应显示 AI 状态灯").rect;
    let p=item.center();
    let button=|pressed|egui::Event::PointerButton{pos:p,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    side_frame_events(&mut h,Some(p),vec![egui::Event::PointerMoved(p)]);
    side_frame_events(&mut h,Some(p),vec![button(true)]);
    side_frame_events(&mut h,Some(p),vec![button(false)]);
    assert_eq!(h.app.page,crate::app::AI_PAGE);
    assert!(h.app.open,"点侧边状态灯应直接打开 AI 页");
}
