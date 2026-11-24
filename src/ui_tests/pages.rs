//! 页面导航与可见性。
use super::*;

#[test]
fn arrows_and_dots_switch_pages() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.hover(Pos2::new(WIN_W / 2.0, 12.0), 1.0);
    h.click(H::local(398.0, 68.0));
    assert_eq!(h.app.page, 1);
    h.hover(H::local(300.0, 60.0), 0.8);
    h.click(H::local(21.0, 68.0));
    assert_eq!(h.app.page, 0);
    h.hover(H::local(300.0, 60.0), 0.8);
    h.click(H::local(398.0, 68.0));
    h.hover(H::local(300.0, 60.0), 0.8);
    h.click(H::local(398.0, 68.0));
    assert_eq!(h.app.page, 2);
}

#[test]
fn wheel_navigation_reaches_every_page_in_both_directions(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.shelf.items.clear();h.app.todos.items.clear();
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.hover(H::local(210.0,18.0),0.3);
    for (delta,pages) in [(-40.0,[1,2,3,11,4,5,6,7,8,9,10,0]),(40.0,[10,9,8,7,6,5,4,11,3,2,1,0])] {
        for page in pages {
            h.wheel(delta);
            assert_eq!(h.app.page,page,"滚轮翻页不能被空的置物架或待办卡住");
            h.hover(H::local(210.0,18.0),0.25);
        }
    }
}

#[test]
fn wheel_from_tall_page_allows_movement_in_previous_page_bounds() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    let edge=H::local(70.0,165.0);
    let inner=H::local(70.0,80.0);
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.app.go(6,1.0);
    h.hover(edge,0.8);
    h.wheel(40.0);
    assert_eq!(h.app.page,5);
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.app.open && !h.hit.contains(edge),"原位保留翻页意图，岛外仍可点击桌面");
    assert!((h.app.h.pos-(PH+crate::island::STATUS_H)).abs()<0.1 && (h.hit.height()-PH-crate::island::STATUS_H-2.0).abs()<0.1,"矮页应使用自身高度");
    h.hover(H::local(180.0,174.0),0.4);
    assert!(h.app.open,"在上一页范围内向侧下方移动仍可阅读");
    h.hover(H::local(220.0,PH-1.0+crate::island::PAGE_RAISE),0.4);
    h.hover(H::local(250.0,157.0),0.4);
    assert!(h.app.open,"轻微穿过新页边缘再移回来不应误收起");
    h.wheel(40.0);
    assert_eq!(h.app.page,4);
    h.hover(inner,0.5);
    assert!(h.app.open && !h.hit.contains(edge),"移入新页后恢复该页高度");
    h.hover(edge,0.5);
    assert!(!h.app.open,"重新移出新页应正常收起");

    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.app.go(6,1.0);
    h.hover(H::local(70.0,PH-1.0+crate::island::PAGE_RAISE),0.8);
    h.wheel(40.0);
    h.hover(edge,0.4);
    assert!(h.app.open,"翻页时光标刚好在新边缘内，也能顺势移到旧页范围");
    h.hover(H::local(-25.0,165.0),0.5);
    assert!(!h.app.open,"直接移出原岛体也应正常收起");
}

#[test]
fn side_docks_keep_both_edges_when_turning_to_a_shorter_page() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dock in [crate::model::Dock::Left,crate::model::Dock::Right] {
        let mut h=H::new();
        h.app.settings.dock=dock;
        let x=if dock==crate::model::Dock::Left {1.0}else{crate::island::side_dock::SIDE_W-1.0};
        let resident=Pos2::new(x,crate::island::side_dock::SIDE_H/2.0);
        for _ in 0..60 {side_frame(&mut h,Some(resident));}
        assert!(h.app.open);
        for sign in [-1.0,1.0] {
            h.app.go(6,1.0);
            let tall=side_frame(&mut h,Some(resident)).hit;
            let edge=tall.center()+Vec2::new(0.0,sign*((PH+crate::island::STATUS_H)/2.0+8.0));
            for _ in 0..45 {side_frame(&mut h,Some(edge));}
            side_frame_events(&mut h,Some(edge),vec![egui::Event::MouseWheel {
                unit:egui::MouseWheelUnit::Point,delta:Vec2::new(0.0,40.0),modifiers:Default::default(),
            }]);
            assert_eq!(h.app.page,5);
            let moved=edge+Vec2::new(65.0,8.0*sign);
            for _ in 0..45 {side_frame(&mut h,Some(moved));}
            let out=side_frame(&mut h,Some(moved));
            assert!(out.expanded && !out.hit.contains(moved) && out.page_scroll_area.is_some_and(|r|r.contains(moved)),
                "{dock:?} 原页上下边缘的空白处应可移动并续滚");
            assert!((h.app.h.pos-(PH+crate::island::STATUS_H)).abs()<0.1);
            let out=side_frame(&mut h,Some(tall.center()));
            assert!(out.expanded && !out.hit.contains(edge),"{dock:?} 移入矮页后应恢复高度");
        }
    }
}

#[test]
fn wheel_clipboard_stays_at_both_edges() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.show_page(crate::island::CLIPBOARD_PAGE);
    for count in [crate::clipboard::page::CLIP_ROWS+1] {
        h.app.clipboard.lock().items.clear();
        for i in 0..count { h.app.clipboard.lock().push(&i.to_string(), "09:00"); }
        let last=count.saturating_sub(crate::clipboard::page::CLIP_ROWS);
        for (delta, first) in [(40.0,0),(-40.0,last),(-40.0,last),(40.0,0),(40.0,0)] {
            h.hover(H::local(300.0,60.0),0.4);
            h.wheel(delta);
            assert_eq!(h.app.page,crate::island::CLIPBOARD_PAGE);
            assert_eq!(h.app.clip_first,first);
        }
        for (delta,page) in [(-40.0,4),(40.0,3)]{
            h.app.show_page(crate::island::CLIPBOARD_PAGE);h.hover(H::local(210.0,18.0),0.4);h.wheel(delta);
            assert_eq!(h.app.page,page,"剪贴板列表外仍应允许滚轮翻页");
        }
    }
}

/// 剪贴板是独立功能页，不再挂在置物架的分栏里；监听开关只跟随自己的页面。
#[test]
fn clipboard_is_a_standalone_page(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.show_page(crate::island::CLIPBOARD_PAGE);
    assert_eq!(h.app.page,crate::island::CLIPBOARD_PAGE);
    h.hover(H::local(300.0,60.0),0.5);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="复制的文字会出现在这里")),"剪贴板页应有空状态");
    h.app.show_page(crate::island::SHELF_PAGE);
    h.hover(H::local(300.0,60.0),0.5);
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="剪贴板")),"置物架页不再有剪贴板分栏");
    h.app.set_page_visible(crate::island::CLIPBOARD_PAGE,false);
    h.app.set_page_visible(crate::island::SHELF_PAGE,true);
    assert!(!h.app.clipboard_active(),"置物架开启不再启动剪贴板监听");
}

#[test]
fn wheel_handles_small_deltas_without_skipping_pages_on_bursts(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.hover(H::local(210.0,18.0),0.3);
    for _ in 0..16 {h.wheel(-0.5);}
    assert_eq!(h.app.page,1,"高精度滚轮的小幅输入应能累积成一次翻页");
    h.hover(H::local(210.0,18.0),0.3);
    for _ in 0..5 {h.wheel(-40.0);}
    assert_eq!(h.app.page,2,"一次连续滚动不能瞬间跳过多个页面");
}

#[test]
fn page_visibility_is_saved_and_sets_the_starting_page() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    assert_eq!(h.app.settings.visible_pages,[true;12]);
    assert!(h.app.settings.show_battery);
    h.app.settings.show_battery=false;
    h.app.set_page_visible(0,false);
    h.app.set_page_visible(3,false);
    h.app.settings.save();
    let settings=crate::model::Settings::load();
    assert!(!settings.show_battery);
    assert_eq!(settings.visible_pages,[false,true,true,false,true,true,true,true,true,true,true,true]);
    let restarted=App::new(&h.ctx,Options{fetch_lyrics:false,pet_idle:1e9});
    assert_eq!(restarted.page(),1);
    assert_eq!(restarted.settings.visible_pages,settings.visible_pages);
    assert!(!restarted.settings.show_battery);
    std::fs::write(crate::model::config_dir().join("settings.txt"),"visible_pages=000000\n").unwrap();
    assert_eq!(crate::model::Settings::load().visible_pages,[true,false,false,false,false,false,false,false,false,false,false,false]);
    std::fs::write(crate::model::config_dir().join("settings.txt"),"visible_pages=1000000\n").unwrap();
    assert_eq!(crate::model::Settings::load().visible_pages,[true,false,false,false,false,false,false,true,true,true,true,true],"旧版页面开关应保留，并开启新增的日程页、天气页、AI 会话页、Windows 通知页与剪贴板页");
    std::fs::write(crate::model::config_dir().join("settings.txt"),"visible_pages=invalid\n").unwrap();
    assert_eq!(crate::model::Settings::load().visible_pages,[true;12]);
}

#[test]
fn page_visibility_skips_hidden_pages_in_every_navigation_control() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    for page in [1,3,4,6,7,8,9,10,11] {h.app.set_page_visible(page,false);}
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    for expected in [2,5,0] {
        h.click(H::local(398.0,68.0));
        assert_eq!(h.app.page,expected);
        h.hover(H::local(300.0,60.0),0.7);
    }
    h.click(H::local(PW/2.0+6.0,132.8));
    assert_eq!(h.app.page,2,"底部圆点应仅包含启用的页面");
    h.hover(H::local(300.0,60.0),0.7);
    h.frame(vec![egui::Event::Key{key:egui::Key::ArrowRight,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    assert_eq!(h.app.page,5);
    h.frame(vec![egui::Event::Key{key:egui::Key::ArrowRight,physical_key:None,pressed:false,repeat:false,modifiers:Default::default()}]);
    h.hover(H::local(300.0,60.0),0.7);
    h.frame(vec![egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:Vec2::new(0.0,40.0),modifiers:Default::default()}]);
    assert_eq!(h.app.page,2,"滚轮应跳过隐藏页面");
    h.click(H::local(21.0,68.0));
    assert_eq!(h.app.page,0);
}

#[test]
fn page_visibility_moves_away_from_hidden_page_and_keeps_one_page() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.go(4,1.0);
    h.app.adding=true;
    h.app.set_page_visible(4,false);
    assert_eq!(h.app.page,0);
    assert!(!h.app.is_editing());
    for page in [0,1,3,5,6,7,8,9,10,11] {h.app.set_page_visible(page,false);}
    assert_eq!(h.app.page,2);
    h.app.set_page_visible(2,false);
    assert!(h.app.settings.visible_pages[2],"不能关闭最后一个页面");
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.click(H::local(398.0,68.0));
    h.click(H::local(21.0,68.0));
    assert_eq!(h.app.page,2);
    h.app.set_page_visible(5,true);
    h.hover(H::local(300.0,60.0),0.7);
    h.click(H::local(398.0,68.0));
    assert_eq!(h.app.page,5,"重新开启的页面应恢复导航");
}

#[test]
fn page_visibility_does_not_reopen_hidden_pages_through_shortcuts_or_drops() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.set_page_visible(5,false);
    h.app.show_page(5);
    assert_eq!(h.app.page,0);
    assert!(h.app.settings_open);
    assert_eq!(h.app.settings_tab,7,"隐藏页面的入口应进入对应设置");
    h.app.set_page_visible(3,false);
    let file=crate::model::config_dir().join("shelf-hidden.txt");
    std::fs::create_dir_all(crate::model::config_dir()).unwrap();
    std::fs::write(&file,"test").unwrap();
    h.app.on_drop(vec![file]);
    assert_eq!(h.app.page,0);
    assert!(h.app.shelf.items.is_empty());
}

#[test]
fn hidden_components_do_not_keep_static_pages_animating(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=false;
    h.app.show_page(4);h.hover(H::local(300.0,60.0),2.0);
    assert!(!h.animating);
    let mut moving=crate::visuals::anim::Spring::new(0.0,230.0,0.9);moving.target=200.0;
    h.app.shelf_x.insert("hidden.txt".into(),moving);
    h.frame(vec![]);
    assert!(!h.animating,"离开置物页后，未完成的文件动画不能让其他页面持续刷新");
    h.app.shelf_x.clear();h.app.pomo.running=true;
    h.hover(H::local(300.0,60.0),1.5);
    assert!(!h.animating,"待办页没有显示专注动画，计时继续但不应强制连续刷新");
}

#[test]
#[ignore="按需导出每周待办与剪贴板的实际画面"]
fn feature_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_FEATURE_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let today="2026-11-04";// 周三
    let todos=||{
        let mut a=crate::todo::TodoItem::new("写周报");a.repeat=0b001_0000;
        let mut b=crate::todo::TodoItem::new("站会");b.repeat=crate::todo::list::REPEAT_WORKDAYS;b.due=Some("2026-11-04 09:30".into());b.done_on=Some("2026-11-04".into());
        let mut c=crate::todo::TodoItem::new("健身");c.repeat=0b000_0101;
        let d=crate::todo::TodoItem::new("回复设计评审意见");
        vec![a,b,c,d]
    };
    let mut h=H::new();let mut r=feature_raster();h.app.todos.items=todos();h.app.show_page(4);
    shoot_feature(&mut h,&mut r,&dest,"todo-page",90,Some(H::local(120.0,60.0)),today);
    let mut h=H::new();let mut r=feature_raster();h.app.todos.items=todos();h.app.show_page(4);
    shoot_feature(&mut h,&mut r,&dest,"todo-editor-0",30,Some(H::local(120.0,60.0)),today);
    h.app.open_task_dialog(2);
    shoot_feature(&mut h,&mut r,&dest,"todo-editor",60,Some(H::local(120.0,60.0)),today);
    let mut h=H::new();let mut r=feature_raster();h.app.todos.items=todos();h.app.show_page(4);
    shoot_feature(&mut h,&mut r,&dest,"todo-picker-0",30,Some(H::local(120.0,60.0)),today);
    h.app.open_task_reminder(3);
    shoot_feature(&mut h,&mut r,&dest,"todo-picker",60,Some(H::local(120.0,60.0)),today);
    // 剪贴板
    let mut h=H::new();let mut r=feature_raster();h.app.show_page(crate::island::CLIPBOARD_PAGE);
    shoot_feature(&mut h,&mut r,&dest,"clip-empty",60,Some(H::local(120.0,60.0)),today);
    {let mut hist=h.app.clipboard.lock();hist.items.clear();
        for (i,t) in ["https://github.com/Renly1994/xiaotang-daozai/releases","会议纪要：\n1. 周三前确认岛仔竖版方案\n2. 剪贴板记录上线","13800138000","SELECT * FROM todo WHERE repeat > 0 ORDER BY due;","验证码 482913"].iter().enumerate(){hist.push(t,&format!("09:{:02}",30+i));}
        hist.items[3].pinned=true;}
    shoot_feature(&mut h,&mut r,&dest,"clip-list",30,Some(H::local(200.0,66.0)),today);
    h.app.show_page(crate::island::SHELF_PAGE);
    let file=std::env::temp_dir().join("岛仔周报.docx");std::fs::write(&file,b"x").unwrap();h.app.shelf.items=vec![file];
    shoot_feature(&mut h,&mut r,&dest,"shelf-files",30,Some(H::local(120.0,60.0)),today);
    h.app.confirm_clear=Some(h.app.t);
    shoot_feature(&mut h,&mut r,&dest,"shelf-confirm",5,Some(H::local(120.0,60.0)),today);
    h.app.settings.clipboard_history=false;h.app.show_page(crate::island::CLIPBOARD_PAGE);
    shoot_feature(&mut h,&mut r,&dest,"clip-off",30,Some(H::local(120.0,60.0)),today);
}

#[test]
fn wheel_short_lists_turn_pages(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    for count in [0,1,5]{
        h.app.shelf.items=(0..count).map(|i|std::path::PathBuf::from(format!("scroll-test-{i}.txt"))).collect();
        for (delta,page) in [(-40.0,11),(40.0,2)]{
            h.app.show_page(3);h.hover(H::local(300.0,60.0),0.5);h.wheel(delta);
            assert_eq!(h.app.page,page,"置物架未溢出应翻页，条数 {count}");
        }
    }
    for count in [0,1,3]{
        h.app.todos.items=(0..count).map(|i|crate::todo::TodoItem{text:format!("任务 {i}"),..Default::default()}).collect();
        for (delta,page) in [(-40.0,5),(40.0,11)]{
            h.app.show_page(4);h.hover(H::local(300.0,60.0),0.5);h.wheel(delta);
            assert_eq!(h.app.page,page,"待办未溢出应翻页，条数 {count}");
        }
    }
    for count in [0,1,crate::clipboard::page::CLIP_ROWS]{
        h.app.clipboard.lock().items.clear();
        for i in 0..count{h.app.clipboard.lock().push(&i.to_string(),"09:00");}
        for (delta,page) in [(-40.0,4),(40.0,3)]{
            h.app.show_page(crate::island::CLIPBOARD_PAGE);h.hover(H::local(300.0,60.0),0.5);h.wheel(delta);
            assert_eq!(h.app.page,page,"剪贴板未溢出应翻页，条数 {count}");
        }
    }
    let day=chrono::NaiveDate::from_ymd_opt(2026,10,28).unwrap();
    let start=day.and_hms_opt(10,0,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap().timestamp();
    h.app.calendar.view.day=day;h.app.calendar.view.month=crate::calendar::month_start(day);h.app.calendar.view.loaded=Some(h.app.calendar.view.month);
    for (count,ended,all_day) in [(0,false,false),(1,false,false),(5,false,false),(8,true,false),(4,false,true)]{
        h.app.calendar.view.events=(0..count).map(|i|crate::mail::Meeting{all_day,title:format!("日程 {i}"),start:if ended{start-7200}else{start+i*60},end:Some(if ended{start-3600}else{start+3600}),at:String::new(),uid:format!("wheel-{i}")}).collect();
        for (delta,page) in [(-40.0,8),(40.0,6)]{
            h.app.show_page(7);h.hover(H::local(300.0,160.0),0.5);h.wheel(delta);
            assert_eq!(h.app.page,page,"日历可见列表未溢出应翻页，条数 {count}");
        }
    }
    h.app.calendar.view.events=(0..7).map(|i|crate::mail::Meeting{all_day:false,title:format!("日程 {i}"),start:start+i*60,end:Some(start+3600),at:String::new(),uid:format!("wheel-long-{i}")}).collect();
    h.app.show_page(7);
    for (delta,first) in [(40.0,0),(-40.0,1),(-40.0,1),(40.0,0),(40.0,0)]{
        h.hover(H::local(300.0,160.0),0.5);h.wheel(delta);
        assert_eq!(h.app.page,7,"日历溢出后边界也应留在本页");assert_eq!(h.app.calendar.view.first,first);
    }
}
