//! 待办、置物架与任务编辑。
use super::*;

#[test]
fn shelf_drop_zone_leaves_normal_hover_and_text_selection_unchanged(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    for _ in 0..90{h.frame(vec![]);}let zone=h.file_drop_zone.unwrap();
    assert_eq!(zone.width(),PW);assert_eq!(zone.height(),96.0);
    let point=zone.right_bottom()-Vec2::splat(12.0);assert!(!h.hit.contains(point));
    h.hover(point,0.8);assert!(!h.app.open);
    h.frame(vec![egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
    h.hover(point,0.8);assert!(!h.app.open,"按住鼠标拖选文字不应打开置物架");
    h.frame(vec![egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
    h.app.settings.visible_pages[3]=false;h.frame(vec![]);assert!(h.file_drop_zone.is_none());
}

#[test]
fn shelf_file_drag_opens_below_the_island_and_cancellation_restores_the_page(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.go(1,1.0);for _ in 0..90{h.frame(vec![]);}
    let point=h.file_drop_zone.unwrap().right_bottom()-Vec2::splat(12.0);
    assert!(!h.hit.contains(point));h.cursor=Some(point);
    h.hovered_files.push(egui::HoveredFile{path:Some(crate::model::config_dir().join("拖入文件.txt")),..Default::default()});
    h.frame(vec![]);assert!(h.app.open && h.app.page==3 && h.app.drag_hot,"文件进入感应区应立即展开置物架");
    for _ in 0..60{h.frame(vec![]);}assert!(h.app.open);
    h.hovered_files.clear();h.cursor=None;for _ in 0..60{h.frame(vec![]);}
    assert!(!h.app.drag_hot && !h.app.open);assert_eq!(h.app.page,1);
    assert!(h.app.shelf.items.is_empty(),"取消拖入不应保存文件");
}

#[test]
fn shelf_quick_drop_in_the_capture_zone_shows_the_added_file(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    let path=crate::model::config_dir().join("快速拖入.txt");std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(&path,"test").unwrap();
    h.cursor=Some(h.file_drop_zone.unwrap().right_bottom()-Vec2::splat(12.0));
    h.dropped_files.push(egui::DroppedFile{path:Some(path.clone()),..Default::default()});
    for _ in 0..90{h.frame(vec![]);}
    assert!(h.app.open && h.app.page==3);assert_eq!(h.app.shelf.items,vec![path.clone()]);assert!(path.exists());
    let mut h=H::new();for _ in 0..90{h.frame(vec![]);}
    h.cursor=Some(h.file_drop_zone.unwrap().right_bottom()-Vec2::splat(12.0));h.app.settings.visible_pages[3]=false;
    h.dropped_files.push(egui::DroppedFile{path:Some(path),..Default::default()});h.frame(vec![]);
    assert!(!h.app.open && !h.app.settings_open,"关闭置物架后应忽略拖入");
}

#[test]
fn wheel_overflowing_shelf_stays_at_both_edges(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    for count in [6,9]{
        h.app.shelf.items=(0..count).map(|i|std::path::PathBuf::from(format!("scroll-test-{i}.txt"))).collect();
        h.app.shelf_first=0;h.app.show_page(3);
        let last=count-5;
        for (delta,first) in [(40.0_f32,0),(-40.0,1),(-400.0,last),(-40.0,last),(40.0,last-1),(400.0,0),(40.0,0)]{
            h.hover(H::local(300.0,60.0),0.7);
            for _ in 0..(delta.abs() as usize/40){h.wheel(delta.signum()*40.0);}
            assert_eq!(h.app.page,3,"置物架溢出后连续滚动或停顿再滚都应留在本页，条数 {count}");
            assert_eq!(h.app.shelf_first,first);
        }
        for (delta,page) in [(-40.0,11),(40.0,2)]{
            h.hover(H::local(300.0,60.0),0.7);
            h.click(H::local(if delta<0.0 {398.0}else{21.0},68.0));
            assert_eq!(h.app.page,page,"置物架溢出后仍应允许箭头切页");
            h.app.show_page(3);h.hover(H::local(210.0,18.0),0.3);h.wheel(delta);
            assert_eq!(h.app.page,page,"置物架顶栏仍应允许滚轮翻页");
            h.app.show_page(3);
        }
    }
}

#[test]
fn wheel_overflowing_todos_stay_at_both_edges(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    for count in [4,8]{
        h.app.todos.items=(0..count).map(|i|crate::model::TodoItem{text:format!("任务 {i}"),..Default::default()}).collect();
        h.app.todo_first=0;h.app.show_page(4);
        let last=count-3;
        for (delta,first) in [(40.0_f32,0),(-40.0,1),(-400.0,last),(-40.0,last),(40.0,last-1),(400.0,0),(40.0,0)]{
            h.hover(H::local(300.0,60.0),0.3);
            for _ in 0..(delta.abs() as usize/40){h.wheel(delta.signum()*40.0);}
            assert_eq!(h.app.page,4,"待办溢出后滚动到首尾都不应翻页，条数 {count}");
            assert_eq!(h.app.todo_first,first);
        }
        for (delta,page) in [(-40.0,5),(40.0,11)]{
            h.app.show_page(4);h.hover(H::local(100.0,60.0),0.3);h.wheel(delta);
            assert_eq!(h.app.page,page,"待办列表外仍应允许滚轮翻页");
        }
    }
}

#[test]
fn empty_todo_draft_can_be_cancelled_by_clicking_elsewhere(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for text in ["","   "]{
        let mut h=H::new();let mut r=feature_raster();h.app.todos.items=vec![crate::model::TodoItem{text:"保留任务".into(),due:None,done_on:None,reminded:false,..Default::default()}];h.app.todos.save();
        h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(70.0,109.0));
        assert!(h.app.adding,"点击添加后应保留空白输入框");
        if !text.is_empty(){h.frame(vec![egui::Event::Text(text.into())]);}
        h.cmds.clear();h.click(H::local(65.0,60.0));
        assert!(!h.app.is_editing(),"空白草稿点击岛内其他位置应退出");
        assert!(h.app.buf.is_empty());assert_eq!(h.app.todos.items.len(),1);assert_eq!(h.app.todos.items[0].text,"保留任务");
        assert_eq!(crate::model::TodoList::load().items.len(),1,"不能保存空白待办");
        h.hover(H::local(65.0,60.0),0.3);
        assert!(!h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Focus)),"取消后不能反复激活窗口");
    }
}

#[test]
fn todo_ime_first_backspace_keeps_committed_text(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(70.0,109.0));
    h.frame(vec![egui::Event::Text("保留文字".into())]);
    h.frame(vec![egui::Event::Key{key:egui::Key::Backspace,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()},egui::Event::Ime(egui::ImeEvent::Enabled)]);
    assert_eq!(h.app.buf,"保留文字","开始组合输入的同一帧，退格只能交给输入法");
    h.frame(vec![egui::Event::Ime(egui::ImeEvent::Disabled)]);
    h.frame(vec![egui::Event::Key{key:egui::Key::Backspace,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    assert_eq!(h.app.buf,"保留文","结束组合输入后普通退格仍应正常删除");
}

#[test]
fn todo_ime_focus_change_preserves_empty_draft(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(70.0,109.0));h.cmds.clear();
    h.frame(vec![egui::Event::Ime(egui::ImeEvent::Enabled),egui::Event::WindowFocused(false)]);
    assert!(h.app.adding,"输入法候选窗口造成的失焦不能取消空白待办");
    assert!(h.app.open);
    assert!(!h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Focus)),"不能和输入法窗口反复争抢焦点");
    h.app.foreground_changed();h.frame(vec![]);
    assert!(h.app.adding,"输入法组合输入期间的前台切换应保留草稿");
    h.frame(vec![egui::Event::WindowFocused(true),egui::Event::Ime(egui::ImeEvent::Preedit("zhong".into()))]);
    h.frame(vec![egui::Event::Ime(egui::ImeEvent::Commit("中文".into())),egui::Event::Ime(egui::ImeEvent::Disabled)]);
    assert_eq!(h.app.buf,"中文");assert!(h.app.todos.items.is_empty());
}

#[test]
fn todo_ime_confirmation_does_not_submit_or_cancel_the_draft(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for key in [egui::Key::Enter,egui::Key::Escape]{
        let mut h=H::new();h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(70.0,109.0));
        h.frame(vec![egui::Event::Ime(egui::ImeEvent::Enabled)]);
        h.frame(vec![egui::Event::Key{key,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()},egui::Event::Ime(egui::ImeEvent::Commit("中文".into())),egui::Event::Ime(egui::ImeEvent::Disabled)]);
        assert!(h.app.adding,"输入法确认或取消候选不能同时结束待办编辑");
        assert_eq!(h.app.buf,"中文","输入法确认后应保留正文，按键 {key:?}");assert!(h.app.todos.items.is_empty());
        h.frame(vec![egui::Event::Key{key,physical_key:None,pressed:false,repeat:false,modifiers:Default::default()}]);
        h.frame(vec![egui::Event::Key{key:egui::Key::Enter,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
        assert_eq!(h.app.todos.items.len(),1,"退出组合输入后回车应正常保存");assert_eq!(h.app.todos.items[0].text,"中文");
    }
}

#[test]
fn todo_ime_candidate_closing_does_not_discard_the_empty_draft(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(70.0,109.0));
    h.frame(vec![egui::Event::Ime(egui::ImeEvent::Enabled)]);
    h.frame(vec![egui::Event::Ime(egui::ImeEvent::Disabled),egui::Event::WindowFocused(false)]);
    assert!(h.app.adding,"关闭输入法候选窗口的同一帧不能取消空白草稿");
    h.frame(vec![egui::Event::WindowFocused(true)]);h.app.foreground_changed();h.frame(vec![]);
    assert!(!h.app.adding,"之后真正切换应用时仍应取消空白草稿");
}

#[test]
fn todo_ime_event_variants_preserve_mixed_text_and_resume_plain_input(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let cases=vec![
        ("无显式开始事件",vec![vec![egui::ImeEvent::Preedit("ni".into())],vec![egui::ImeEvent::Commit("你".into()),egui::ImeEvent::Disabled]],"你"),
        ("一次提交多个字符",vec![vec![egui::ImeEvent::Enabled,egui::ImeEvent::Preedit("にほん".into())],vec![egui::ImeEvent::Commit("日本".into()),egui::ImeEvent::Disabled]],"日本"),
        ("同帧提交后继续组合",vec![vec![egui::ImeEvent::Enabled,egui::ImeEvent::Preedit("하".into())],vec![egui::ImeEvent::Commit("한".into()),egui::ImeEvent::Disabled,egui::ImeEvent::Enabled,egui::ImeEvent::Preedit("그".into())],vec![egui::ImeEvent::Commit("글".into()),egui::ImeEvent::Disabled]],"한글"),
        ("取消候选",vec![vec![egui::ImeEvent::Enabled,egui::ImeEvent::Preedit("pin".into())],vec![egui::ImeEvent::Preedit(String::new()),egui::ImeEvent::Disabled]],""),
    ];
    for (name,frames,expected) in cases {
        let mut h=H::new();h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(70.0,109.0));
        h.frame(vec![egui::Event::Text("中文 ABC ".into())]);
        for events in frames {h.frame(events.into_iter().map(egui::Event::Ime).collect());}
        assert_eq!(h.app.buf,format!("中文 ABC {expected}"),"事件模式：{name}");
        assert!(h.app.adding);assert!(h.app.todos.items.is_empty());
        h.frame(vec![egui::Event::Text("x".into())]);
        h.frame(vec![egui::Event::Key{key:egui::Key::Backspace,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
        assert_eq!(h.app.buf,format!("中文 ABC {expected}"),"组合输入结束后应恢复普通退格：{name}");
        h.frame(vec![egui::Event::Key{key:egui::Key::Enter,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
        assert_eq!(h.app.todos.items.len(),1,"组合输入结束后应能保存：{name}");
    }
}

#[test]
fn blank_todo_draft_closes_when_window_focus_is_lost(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(70.0,109.0));
    h.cmds.clear();h.frame(vec![egui::Event::WindowFocused(false)]);
    assert!(!h.app.is_editing(),"点击其他软件后空白草稿应自动退出");
    assert!(h.app.todos.items.is_empty());
    assert!(!h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Focus)),"失去焦点时不能把窗口再次激活");
}

#[test]
fn todo_input_requests_window_focus_once_and_preserves_typed_drafts(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.cmds.clear();h.click(H::local(70.0,109.0));
    assert_eq!(h.cmds.iter().filter(|cmd|matches!(cmd,crate::app::Cmd::Focus)).count(),1,"新建时只应请求一次窗口焦点");
    h.click(H::local(280.0,46.0));assert!(h.app.adding,"点击输入框不能取消草稿");
    h.frame(vec![egui::Event::Text("尚未保存的内容".into())]);h.cmds.clear();
    h.click(H::local(65.0,60.0));h.frame(vec![egui::Event::WindowFocused(false)]);
    for _ in 0..20{h.frame(vec![]);}
    assert!(h.app.adding);assert_eq!(h.app.buf,"尚未保存的内容");assert!(h.app.todos.items.is_empty());
    assert!(!h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Focus)),"已有内容失焦后应保留草稿，不能抢回焦点");
}

#[test]
fn switching_apps_cancels_only_empty_new_todos(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for text in ["","   ","保留未提交的内容"]{
        let mut h=H::new();h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(70.0,109.0));
        if !text.is_empty(){h.frame(vec![egui::Event::Text(text.into())]);}
        h.cmds.clear();h.app.foreground_changed();h.frame(vec![]);
        assert_eq!(h.app.adding,!text.trim().is_empty());
        assert_eq!(h.app.buf,if text.trim().is_empty(){""}else{text});
        assert!(h.app.todos.items.is_empty());
        assert!(!h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Focus)));
    }
}

#[test]
fn drink_adds_water_and_todo_can_be_added() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.hover(Pos2::new(WIN_W / 2.0, 12.0), 1.0);
    h.app.go(2, 1.0);
    h.hover(H::local(300.0, 60.0), 0.8);
    h.click(H::local(70.0, 110.0));
    assert_eq!(h.app.water.drunk_ml, 250);
    assert_eq!(h.app.water.hours[9], 250);

    h.app.go(4, 1.0);
    h.hover(H::local(300.0, 60.0), 0.8);
    h.click(H::local(70.0, 109.0));
    assert!(h.app.adding);
    h.hover(H::local(300.0, 60.0), 0.4);
    h.frame(vec![egui::Event::Text("写周报".into())]);
    h.frame(vec![egui::Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }]);
    h.hover(H::local(300.0, 60.0), 0.1);
    assert_eq!(h.app.todos.items.first().map(|t| t.text.as_str()), Some("写周报"));
    assert!(!h.app.adding);
}

#[test]
fn shelf_remove_keeps_original_file() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let file=crate::model::config_dir().join("保留文件.txt");std::fs::create_dir_all(file.parent().unwrap()).unwrap();std::fs::write(&file,"内容").unwrap();
    h.app.shelf.add(&file).unwrap();h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(3,1.0);h.hover(H::local(300.0,60.0),0.8);
    h.click(H::local(233.0,39.0));assert!(h.app.shelf.items.is_empty());assert!(file.exists());assert!(crate::model::Shelf::load().items.is_empty());
}

#[test]
fn shelf_duplicate_drop_reports_existing_files_and_preserves_the_current_page() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let dir=crate::model::config_dir();std::fs::create_dir_all(&dir).unwrap();
    let files:Vec<_>=(0..7).map(|i|{let path=dir.join(format!("文件-{i}.txt"));std::fs::write(&path,"内容").unwrap();path}).collect();
    h.app.on_drop(files.clone());h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.shelf_first=0;
    h.app.on_drop(vec![files[0].clone()]);h.hover(H::local(300.0,60.0),0.3);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="该文件已在置物架中")));
    h.app.on_drop(vec![files[0].clone(),files[1].clone()]);h.hover(H::local(300.0,60.0),0.3);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="这些文件已在置物架中")));
    assert_eq!(h.app.shelf_first,0,"重复添加不应将列表跳到最后");
    assert_eq!(h.app.shelf.items,files);assert_eq!(crate::model::Shelf::load().items,files);
}

#[test]
fn shelf_drop_distinguishes_new_duplicate_and_missing_files() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let dir=crate::model::config_dir();std::fs::create_dir_all(&dir).unwrap();
    let old=dir.join("已添加.txt");let new=dir.join("新文件.txt");let missing=dir.join("已移动.txt");
    for file in [&old,&new]{std::fs::write(file,"内容").unwrap();}
    h.app.on_drop(vec![old.clone()]);h.app.on_drop(vec![old.clone(),new.clone(),new.clone(),missing.clone()]);
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="已添加 1 个文件，2 个已存在，1 个无法添加")));
    assert_eq!(h.app.shelf.items,vec![old.clone(),new]);
    std::fs::remove_file(&old).unwrap();h.app.on_drop(vec![old]);h.hover(H::local(300.0,60.0),0.3);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="文件不存在或无法访问")));
}

#[test]
fn todo_due_roundtrip_and_fires_only_once(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.todos.items.push(crate::model::TodoItem{text:"会议 | 准备".into(),done_on:None,due:Some("2026-10-28 09:40".into()),reminded:false,..Default::default()});h.app.todos.save();
    let loaded=crate::model::TodoList::load();assert_eq!(loaded.items[0].text,"会议 | 准备");assert_eq!(loaded.items[0].due.as_deref(),Some("2026-10-28 09:40"));
    h.frame(vec![]);assert!(h.app.todos.items[0].reminded);let n=h.cmds.len();h.frame(vec![]);assert_eq!(n,h.cmds.len());assert!(crate::model::TodoList::load().items[0].reminded);
}

#[test]
fn todo_completed_items_move_down_and_undo_restores_the_original_order(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.todos.items=["已完成甲","待做甲","周一重复","已完成乙","每日待办"].into_iter().map(crate::model::TodoItem::new).collect();
    h.app.todos.items[0].done_on=Some("2026-10-28".into());h.app.todos.items[3].done_on=Some("2026-10-28".into());h.app.todos.items[2].repeat=1;h.app.todos.items[4].repeat=crate::model::REPEAT_ALL;
    let original=h.app.todos.items.iter().map(|t|t.text.clone()).collect::<Vec<_>>();assert_eq!(h.app.todos.display_order("2026-10-28"),vec![1,4,2,0,3],"今天未完成优先，其次其他日期任务，已完成沉底");
    h.app.todos.items[0].toggle("2026-10-28");assert_eq!(h.app.todos.display_order("2026-10-28"),vec![0,1,4,2,3],"取消完成应恢复原有相对顺序");
    h.app.todos.items[4].toggle("2026-10-28");assert_eq!(h.app.todos.display_order("2026-10-28"),vec![0,1,2,3,4]);assert_eq!(h.app.todos.display_order("2026-10-29"),vec![0,1,4,2,3],"重复任务到下一天应回到未完成区");
    assert_eq!(h.app.todos.items.iter().map(|t|t.text.clone()).collect::<Vec<_>>(),original,"只改变显示顺序，不能移动原数据");
}

#[test]
fn todo_edit_choice_can_edit_other_rows_and_completed_items(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.todos.items=["第一项","第二项","已完成任务"].into_iter().map(crate::model::TodoItem::new).collect();h.app.todos.items[2].done_on=Some("2026-10-28".into());
    h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click_text("改一下");
    assert!(h.app.editing.is_none(),"多条待办应先选择，不能默认修改第一项");h.frame(vec![]);h.click_text("第二项");
    assert_eq!(h.app.editing,Some(1));h.app.buf="第二项已修改".into();h.frame(vec![egui::Event::Key{key:egui::Key::Enter,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    assert_eq!(h.app.todos.items[0].text,"第一项");assert_eq!(h.app.todos.items[1].text,"第二项已修改");assert!(h.app.todos.items[2].done_on.is_some());
    h.frame(vec![]);h.click_text("改一下");h.frame(vec![]);h.click_text("已完成任务");assert_eq!(h.app.editing,Some(2));
    h.frame(vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);assert!(!h.app.is_editing());assert!(h.app.todos.items[2].done_on.is_some());
}

#[test]
fn todo_edit_choice_keeps_a_single_completed_task_editable(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.todos.items=vec![crate::model::TodoItem::new("已完成任务")];h.app.todos.items[0].done_on=Some("2026-10-28".into());
    h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click_text("改一下");assert_eq!(h.app.editing,Some(0),"只有一条时可直接编辑，完成状态不能禁用编辑");
}

#[test]
fn todo_edit_choice_reaches_scrolled_items_and_cancels_without_changes(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.todos.items=(0..7).map(|i|crate::model::TodoItem::new(format!("任务 {i}"))).collect();
    h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click_text("改一下");assert!(h.app.editing.is_none());
    h.hover(H::local(300.0,60.0),0.2);for _ in 0..7{h.wheel(-24.0);}assert_eq!(h.app.page(),4,"选择事项时滚到末尾也不能翻页");h.frame(vec![]);h.click_text("任务 6");assert_eq!(h.app.editing,Some(6));
    h.frame(vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);assert!(!h.app.is_editing());assert_eq!(h.app.todos.items[6].text,"任务 6");assert!(h.app.todos.items.iter().all(|t|t.done_on.is_none()));
}

#[test]
fn todo_completed_reordering_does_not_complete_the_next_item_on_a_double_click(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.todos.items=["任务甲","任务乙","任务丙"].into_iter().map(crate::model::TodoItem::new).collect();h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);
    let point=H::local(212.0,46.0);h.click(point);h.frame(vec![]);h.click(point);h.frame(vec![]);
    assert!(h.app.todos.items[0].done_on.is_some());assert!(h.app.todos.items[1].done_on.is_none(),"首项下移后，快速连点不能误完成下一项");
    h.click_text("任务甲");h.frame(vec![]);assert!(h.app.todos.items.iter().all(|t|t.done_on.is_none()));assert_eq!(h.app.todos.display_order("2026-10-28"),vec![0,1,2]);
}

#[test]
fn todo_edit_choice_can_cancel_without_toggling_or_deleting_items(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.todos.items=["任务甲","任务乙"].into_iter().map(crate::model::TodoItem::new).collect();h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);
    h.click_text("改一下");h.frame(vec![]);h.click_text("取消");assert!(!h.app.is_editing());assert_eq!(h.app.todos.items.len(),2);assert!(h.app.todos.items.iter().all(|t|t.done_on.is_none()));
    h.frame(vec![]);h.click_text("改一下");h.frame(vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);assert!(!h.app.is_editing());assert_eq!(h.app.todos.items.len(),2);
}

#[test]
fn todo_edit_choice_keeps_the_selected_item_when_an_earlier_item_is_deleted(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.todos.items=["第一项","第二项","第三项"].into_iter().map(crate::model::TodoItem::new).collect();h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);
    h.click_text("改一下");h.frame(vec![]);h.click_text("第二项");h.frame(vec![]);h.click(h.ctx.read_response(egui::Id::new(("card",true,("todo-delete",0usize)))).unwrap().rect.center());
    assert_eq!(h.app.editing,Some(0),"删除前面的条目后，编辑目标仍须是第二项");h.app.buf="第二项已修改".into();h.frame(vec![egui::Event::Key{key:egui::Key::Enter,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);assert_eq!(h.app.todos.items.iter().map(|t|t.text.as_str()).collect::<Vec<_>>(),["第二项已修改","第三项"]);
}

#[test]
#[ignore="按需导出待办完成排序和点选编辑的实际画面"]
fn todo_edit_choice_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_TODO_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for skin in [crate::model::Skin::Classic,crate::model::Skin::Cream]{for choosing in [false,true]{
        let mut h=H::new();h.app.settings.skin=skin;h.app.todos.items=["整理会议资料","确认本周安排","已经完成的任务"].into_iter().map(crate::model::TodoItem::new).collect();h.app.todos.items[2].done_on=Some("2026-11-06".into());h.app.show_page(4);h.app.todo_choosing=choosing;
        let mut raster=feature_raster();shoot_feature(&mut h,&mut raster,&dest,&format!("todo-{}-{}",skin.key(),if choosing{"choose"}else{"list"}),90,Some(H::local(175.0,70.0)),"2026-11-06");
    }}
}

#[test]
fn task_editor_saves_and_cancels_inside_island(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.app.todos.items.push(crate::model::TodoItem{text:"准备会议".into(),done_on:None,due:None,reminded:false,..Default::default()});h.app.go(4,1.0);h.hover(H::local(300.0,60.0),0.8);h.app.open_task_dialog(0);
    h.hover(H::local(300.0,60.0),0.2);assert!(h.app.is_editing());h.click(H::local(326.0,110.0));assert!(!h.app.is_editing());assert_eq!(h.app.todos.items[0].text,"准备会议");
    h.app.open_task_dialog(0);h.frame(vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);assert!(!h.app.is_editing());
}

#[test]
fn adding_a_todo_saves_content_then_offers_an_optional_reminder() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for remind in [false,true] {
        let mut h=H::new();h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);
        h.click(H::local(70.0,109.0));h.frame(vec![egui::Event::Text("提交产品方案".into())]);
        h.frame(vec![egui::Event::Key{key:egui::Key::Enter,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
        assert_eq!(h.app.todos.items[0].text,"提交产品方案");
        assert!(h.app.todos.items[0].due.is_none());
        assert!(h.app.task_dialog.is_some(),"保存内容后应进入可选提醒步骤");
        assert_eq!(crate::model::TodoList::load().items[0].text,"提交产品方案");
        h.frame(vec![egui::Event::Key{key:egui::Key::Enter,physical_key:None,pressed:false,repeat:false,modifiers:Default::default()}]);
        if remind {
            let initial=chrono::NaiveDateTime::parse_from_str(&h.app.task_dialog.as_ref().unwrap().picker.due(),"%Y-%m-%d %H:%M").unwrap();
            h.hover(H::local(235.0,77.0),0.4);h.wheel(-40.0);h.picker_click("完成");
            let expected=(initial+chrono::Duration::days(1)).format("%Y-%m-%d %H:%M").to_string();
            assert_eq!(h.app.todos.items[0].due.as_deref(),Some(expected.as_str()));
        }else{h.picker_click("暂不提醒");assert!(h.app.todos.items[0].due.is_none());}
        assert!(h.app.task_dialog.is_none());
        assert!(h.app.todos.items[0].done_on.is_none());
        assert_eq!(crate::model::TodoList::load().items[0].due,h.app.todos.items[0].due);
    }
}

#[test]
fn todo_reminder_picker_is_embedded_in_the_island(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.todos.items=vec![crate::model::TodoItem{text:"准备会议".into(),done_on:None,due:None,reminded:false,..Default::default()}];
    h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);
    h.ctx.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(|_,_|panic!("待办提醒必须在岛内选择，不能创建独立窗口"));
    h.click(H::local(343.0,46.0));
    for _ in 0..90 {h.frame(vec![]);}
    assert_eq!(h.app.expanded_height(),PH+crate::app::STATUS_H,"右侧滚轮应保持待办页原来的高度");
    assert!(h.hit.height()<=PH+crate::app::STATUS_H+2.0,"选择时间不能撑大岛体");
    for label in ["准备会议","已完成","取消","完成","暂不提醒"] {
        let (rect,clip)=h.shapes.iter().find_map(|s|match &s.shape {
            egui::Shape::Text(t) if t.galley.text()==label=>Some((t.visual_bounding_rect(),s.clip_rect)),_=>None,
        }).unwrap_or_else(||panic!("岛内缺少选项：{label}"));
        assert!(h.hit.contains_rect(rect),"选项 {label} 应在岛体命中区域内");
        assert!(h.region.contains_rect(rect) && clip.contains_rect(rect),"选项 {label} 不能被窗口或岛体裁剪");
    }
    h.frame(vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    assert!(h.app.task_dialog.is_none());assert!(h.app.todos.items[0].due.is_none());
}

#[test]
fn reminder_wheels_select_date_hour_and_minute_without_typing(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let day=chrono::Local::now().date_naive()+chrono::Duration::days(2);let original=format!("{day} 09:00");
    h.app.todos.items=vec![crate::model::TodoItem{text:"准备方案".into(),due:Some(original.clone()),done_on:None,reminded:false,..Default::default()}];
    h.app.open_task_reminder(0);
    for x in [235.0,290.0,346.0] {h.hover(H::local(x,77.0),0.4);h.wheel(-40.0);}
    assert_eq!(h.app.todos.items[0].due,Some(original),"调整时不能提前写入提醒");
    h.picker_click("完成");
    let next=day+chrono::Duration::days(1);
    assert_eq!(h.app.todos.items[0].due,Some(format!("{next} 10:01")));
    assert!(h.app.task_dialog.is_none());
}

#[test]
fn reminder_wheels_accumulate_small_trackpad_movements(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let day=chrono::Local::now().date_naive()+chrono::Duration::days(2);
    h.app.todos.items=vec![crate::model::TodoItem{text:"检查进度".into(),due:Some(format!("{day} 10:30")),done_on:None,reminded:false,..Default::default()}];
    h.app.open_task_reminder(0);h.hover(H::local(346.0,77.0),1.0);
    for _ in 0..80{h.wheel(-0.5);}
    h.picker_click("完成");
    assert_eq!(h.app.todos.items[0].due,Some(format!("{day} 10:31")));
    assert_eq!(h.app.page(),4,"操作时间滚轮不能触发翻页");
}

#[test]
fn reminder_picker_rejects_past_time_and_cancel_preserves_the_task(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let old=format!("{} 09:00",chrono::Local::now().date_naive()-chrono::Duration::days(1));
    h.app.todos.items=vec![crate::model::TodoItem{text:"保留任务".into(),due:Some(old.clone()),done_on:None,reminded:true,..Default::default()}];
    h.app.open_task_reminder(0);h.picker_click("完成");h.picker_frame(vec![]);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="请选择晚于当前的时间")));
    assert_eq!(h.app.todos.items[0].due,Some(old.clone()));
    h.picker_click("取消");assert!(h.app.task_dialog.is_none());assert_eq!(h.app.todos.items[0].due,Some(old));
}

#[test]
fn shelf_remove_icon_stays_centered_on_hover_and_preserves_the_file(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for ppp in [1.0,1.25,1.5,2.0]{
        let mut h=H::new();h.ctx.set_pixels_per_point(ppp);
        let path=crate::model::config_dir().join("设计说明.txt");std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(&path,"保留原文件").unwrap();
        h.app.shelf.items=vec![path.clone()];h.app.show_page(3);
        for pointer in [H::local(300.0,60.0),H::local(233.0,39.0)]{
            h.hover(pointer,1.0);
            let button=h.ctx.read_response(egui::Id::new(("card",true,("shelf-remove",&path)))).unwrap().rect;
            let ink=h.shapes.iter().filter_map(|s|match &s.shape{
                egui::Shape::LineSegment{points,..} if points.iter().all(|p|button.shrink(2.0).contains(*p)) && (points[0].x-points[1].x).abs()>2.0 && (points[0].y-points[1].y).abs()>2.0=>Some(Rect::from_two_pos(points[0],points[1])),_=>None,
            }).fold(Rect::NOTHING,|a,b|a.union(b));
            assert!(ink.is_positive() && ink.center().distance(button.center())<0.4,"置物架移除图案应与按钮同心");
        }
        h.click(H::local(233.0,39.0));assert!(h.app.shelf.items.is_empty());assert!(path.exists());
    }
}

#[test]
fn side_dock_file_drag_opens_the_shelf_and_drop_saves_it(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let (w,hgt)=(crate::app::side_dock::SIDE_W,crate::app::side_dock::SIDE_H);
    let path=crate::model::config_dir().join("侧边拖入.txt");std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(&path,"x").unwrap();
    for dock in [crate::model::Dock::Left,crate::model::Dock::Right] {
        let mut h=H::new();h.app.settings.dock=dock;
        let edge=if dock==crate::model::Dock::Left{10.0}else{w-10.0};
        let at=Pos2::new(edge,hgt/2.0);
        let mut frame=|h:&mut H,hovered:bool,dropped:bool|{
            h.t+=1.0/60.0;
            let env=Env{t:h.t,clock:"09:41:00".into(),cursor:Some(at),..Default::default()};
            let mut raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,..Default::default()};
            if hovered {raw.hovered_files.push(egui::HoveredFile{path:Some(path.clone()),..Default::default()});}
            if dropped {raw.dropped_files.push(egui::DroppedFile{path:Some(path.clone()),..Default::default()});}
            let app=&mut h.app;let mut out=None;
            let _=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{out=Some(app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(w,hgt)),&env));});});
            out.unwrap()
        };
        let out=frame(&mut h,false,false);
        let zone=out.file_drop_zone.expect("置物架开启时侧边也有拖放感应区");
        assert!(zone.contains(at),"{dock:?} 感应区应覆盖贴边的胶囊：{zone:?}");
        frame(&mut h,true,false);
        assert!(h.app.drag_hot && h.app.page==3,"{dock:?} 文件拖到胶囊上应切到置物架");
        for _ in 0..40 {frame(&mut h,true,false);}
        assert!(h.app.open);
        frame(&mut h,false,true);
        assert!(h.app.shelf.items.contains(&path),"{dock:?} 松手后应放进置物架");
        h.app.shelf.items.clear();h.app.shelf.save();
    }
}

#[test]
fn shelf_menu_last_action_fits_inside_window_and_works(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.shelf.items=vec![r"folder\menu-test.zip".into()];
    h.app.show_page(3);
    let point=H::local(209.5,101.0);
    h.hover(point,1.2);
    for pressed in [true,false]{
        h.frame(vec![egui::Event::PointerButton{pos:point,button:egui::PointerButton::Secondary,pressed,modifiers:Default::default()}]);
    }
    h.hover(point,1.5);
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==r"folder\menu-test.zip")));
    let label=h.shapes.iter().find_map(|s|match &s.shape{
        egui::Shape::Text(t) if t.galley.text()=="从置物架移除"=>Some(Rect::from_min_size(t.pos,t.galley.size())),
        _=>None,
    }).expect("右键菜单应显示移除操作");
    assert!(h.region.contains_rect(h.app.shelf_menu_rect.unwrap() + h.ctx.style().visuals.popup_shadow.margin()));
    assert!(h.app.shelf_menu_rect.unwrap().contains_rect(label));
    h.hover(label.center(),1.0);
    assert!(h.app.open);
    h.click(label.center());
    assert!(h.app.shelf.items.is_empty());
}
