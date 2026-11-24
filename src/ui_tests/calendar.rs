//! 日历页、本地日程与导入。
use super::*;

/// #200：维语、藏语歌曲在收起的岛上歌词和音乐页里的显示，导出修复前后对照图。
/// 运行：设置 DAOZAI_SCRIPT_PREVIEW_DIR 后执行 `cargo test complex_script_lyrics_preview -- --ignored`。
#[test]
#[ignore="按需导出维语、藏语歌词的修复前后对照图"]
fn complex_script_lyrics_preview(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_SCRIPT_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let songs=[
        ("uyghur","سۆيگۈ دېگەن شۇنداق بولامدۇ","رەيىمجان ئابدۇكېيۇم",
            "[00:01]سۆيگۈ دېگەن شۇنداق بولامدۇ\n[00:05]يايرىم سېنى سېغىندىم شۇنچە (2024)\n[00:09]كۆڭلۈمدىكى سۆزلەرنى ساڭا ئېيتاي دېسەم سەن يوق\n[00:13]سۆيگۈ دېگەن شۇنداق بولامدۇ"),
        ("tibetan","ཕ་ཡུལ་གྱི་དྲན་པ།","ཚེ་དབང་ལྷ་མོ།",
            "[00:01]བཀྲ་ཤིས་བདེ་ལེགས།\n[00:05]སྐྱིད་པོའི་གྲོགས་པོ་ཁྱེད་རང་ཡིན།\n[00:09]ཕ་ཡུལ་གྱི་རི་བོ་མཐོན་པོ།\n[00:13]སྙིང་གི་གཏམ་བརྗོད།"),
    ];
    for fixed in [false,true] {
        crate::render::rtl_text::BYPASS.store(!fixed,std::sync::atomic::Ordering::Relaxed);
        for (name,title,artist,lrc) in songs {
            for open in [false,true] {
                let mut h=H::new();h.app.settings.pet=false;
                if !fixed {h.ctx.set_fonts(crate::render::theme::fonts_without_tibetan());}
                h.app.opts.fetch_lyrics=true;h.app.settings.island_lyrics=true;h.app.show_lyrics=true;
                let media=crate::smtc::NowPlaying{source:"cloudmusic.exe".into(),title:title.into(),artist:artist.into(),duration:215.0,playing:true,position:7.0,
                    playback_type:crate::smtc::MediaKind::Music,..Default::default()};
                h.media=media.clone();h.frame(vec![]);
                h.app.ly=crate::app::LyricsState{key:format!("{}|{}|{:.0}",media.title,media.artist,media.duration),
                    song:Some(crate::app::LyricSong{source:media.source.clone(),title:media.title.clone(),artist:media.artist.clone(),duration:media.duration,song_id:None}),
                    lyrics:Some(crate::lyrics::parse_lrc(lrc)),..Default::default()};
                if open {h.app.show_page(0);}
                let (w,hgt)=(crate::app::CANVAS_W,WIN_H);
                let mut raster=feature_raster_sized(w,hgt);
                for frame in 0..90 {
                    h.t+=1.0/60.0;
                    let env=Env{t:h.t,clock:"22:08:00".into(),today:"2026-11-16".into(),date:"11月16日 周一".into(),media:h.media.clone(),bt_ready:true,..Default::default()};
                    let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,
                        viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
                    let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{
                        h.app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,hgt)),&env);
                    });});
                    raster.update_textures(&output.textures_delta);
                    if frame==89 {
                        raster.buf.fill([0.20,0.24,0.32,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                        let image=image::RgbaImage::from_raw((w*2.0)as u32,(hgt*2.0)as u32,rgba).unwrap();
                        let crop_h=if open {(h.app.expanded_height()*2.0+24.0)as u32}else{110};
                        let file=format!("{name}-{}-{}.png",if open{"page"}else{"island"},if fixed{"after"}else{"before"});
                        image::imageops::crop_imm(&image,200,0,880,crop_h).to_image().save(dest.join(file)).unwrap();
                    }
                    raster.free_textures(&output.textures_delta);
                }
            }
        }
    }
    crate::render::rtl_text::BYPASS.store(false,std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn changing_qq_metadata_cannot_spawn_lyrics_workers_while_one_is_running(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let (release,wait)=std::sync::mpsc::channel();
    let worker=std::thread::spawn(move||{let _=wait.recv();});
    let id=worker.thread().id();h.app.ly_worker=Some(worker);
    h.app.opts.fetch_lyrics=true;h.app.settings.island_lyrics=true;
    for step in 0..1440 {
        h.media=crate::smtc::NowPlaying{source:"QQMusic.exe".into(),title:format!("歌曲{}",step%2),artist:"歌手".into(),duration:120.0,playing:true,..Default::default()};
        h.t=step as f64*0.25;h.frame(vec![]);
        assert_eq!(h.app.ly_worker.as_ref().unwrap().thread().id(),id,"旧请求未结束时不得另开取词线程");
        assert!(h.app.ly.rx.is_none(),"不能为抖动的歌曲堆积请求");
    }
    release.send(()).unwrap();h.app.ly_worker.take().unwrap().join().unwrap();
}

#[test]
fn meeting_notice_can_close_without_deleting_event_or_other_reminders(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.frame(vec![]);
    let event=crate::mail::Meeting{all_day:false,title:"当前会议".into(),start:chrono::Local::now().timestamp()+600,end:None,at:String::new(),uid:"dismiss-current".into()};
    h.app.calendar.events.push(event.clone());
    h.app.mail.fired.insert(crate::calendar::dedupe_key(&event));
    h.app.meeting_alert(event,"飞书".into(),7);
    h.app.meeting_alert(crate::mail::Meeting{all_day:false,title:"下一场会议".into(),start:chrono::Local::now().timestamp()+1200,end:None,at:String::new(),uid:"dismiss-next".into()},"钉钉".into(),7);
    for _ in 0..45{h.frame(vec![]);}
    let page=h.app.page;
    let button=h.ctx.read_response(egui::Id::new(("card",true,"meeting-dismiss"))).expect("会议提醒必须有关闭按钮").rect;
    h.click(button.center());
    assert!(h.app.notice.is_none());assert_eq!(h.app.page,page);
    assert_eq!(h.app.calendar.events.len(),1,"关闭提醒不能删除日程");
    h.frame(vec![]);
    assert_eq!(h.app.notice.as_ref().unwrap().meeting.as_ref().unwrap().0.uid,"dismiss-next");
}

#[test]
fn calendar_hides_ended_events_and_keeps_all_day_out_of_featured_card() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let day=chrono::NaiveDate::from_ymd_opt(2026,10,28).unwrap();
    let at=|hour|day.and_hms_opt(hour,0,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap().timestamp();
    h.app.calendar.view.day=day;h.app.calendar.view.month=crate::calendar::month_start(day);h.app.calendar.view.loaded=Some(h.app.calendar.view.month);
    h.app.calendar.view.events=vec![
        crate::mail::Meeting{uid:"all-day".into(),title:"全天安排".into(),start:at(0),end:None,all_day:true,at:String::new()},
        crate::mail::Meeting{uid:"past".into(),title:"过去的安排".into(),start:at(7),end:Some(at(8)),all_day:false,at:String::new()},
        crate::mail::Meeting{uid:"future".into(),title:"下一场安排".into(),start:at(11),end:Some(at(12)),all_day:false,at:String::new()},
    ];
    h.app.open_calendar(None);h.hover(H::local(300.0,40.0),1.0);
    let contains=|h:&H,text:&str|h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(t) if t.galley.text()==text));
    assert!(!contains(&h,"过去的安排"));assert!(contains(&h,"全天安排"));
    assert!(h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(t) if t.galley.text()=="11:00" && (t.galley.job.sections[0].format.font_id.size-14.0).abs()<0.01)));
    assert!(h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(t) if t.galley.text()=="全天安排" && t.galley.job.sections[0].format.font_id.size<14.0)),"全天日程不应占用展开卡片");
    h.click(H::local(270.0,43.0));h.frame(vec![]);assert!(contains(&h,"过去的安排"));
    h.click(H::local(270.0,43.0));h.frame(vec![]);assert!(!contains(&h,"过去的安排"));
}

#[test]
fn local_calendar_failed_save_keeps_draft_and_does_not_change_events() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    std::fs::create_dir_all(crate::model::config_dir().join("local-calendar.json")).unwrap();
    h.app.open_calendar_editor(None);h.app.calendar.draft.as_mut().unwrap().title="不能丢失的草稿".into();
    h.app.save_calendar_editor();
    assert!(h.app.calendar.local.is_empty());let draft=h.app.calendar.draft.as_ref().unwrap();
    assert_eq!(draft.title,"不能丢失的草稿");assert!(!draft.error.is_empty());
}

#[test]
#[ignore="按需测量 ICS 导入后日历页的绘制耗时"]
fn imported_ics_calendar_frame_cost() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (count,same_day) in [(0,false),(100,false),(1000,false),(1000,true),(5000,true)] {
        let mut h=H::new();h.app.settings.pet=false;
        let start=chrono::Local::now().date_naive().and_hms_opt(12,0,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap().with_timezone(&chrono::Utc);
        let mut ics=String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n");
        for index in 0..count {
            let at=start+chrono::Duration::days(if same_day {0} else {index%365});
            ics.push_str(&format!("BEGIN:VEVENT\r\nUID:import-{index}\r\nSUMMARY:导入日程 {index}\r\nDTSTART:{}\r\nDTEND:{}\r\nEND:VEVENT\r\n",at.format("%Y%m%dT%H%M%SZ"),(at+chrono::Duration::hours(1)).format("%Y%m%dT%H%M%SZ")));
        }
        ics.push_str("END:VCALENDAR\r\n");
        let at=std::time::Instant::now();h.app.import_ics(&[("sample.ics".into(),ics)]);let import=at.elapsed();
        assert_eq!(h.app.calendar.local.len(),count as usize);
        h.app.open_calendar(None);h.hover(H::local(300.0,40.0),0.2);
        let at=std::time::Instant::now();for _ in 0..3 {h.frame(vec![]);}let frames=at.elapsed()/3;
        h.app.show_page(0);h.hover(H::local(300.0,40.0),0.2);
        let at=std::time::Instant::now();for _ in 0..3 {h.frame(vec![]);}let other=at.elapsed()/3;
        println!("count={count} same_day={same_day} import_ms={:.2} calendar_frame_ms={:.2} other_frame_ms={:.2}",import.as_secs_f64()*1000.0,frames.as_secs_f64()*1000.0,other.as_secs_f64()*1000.0);
    }
}

#[test]
fn ics_import_preserves_recurring_overnight_and_all_day_events_on_reimport() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let day=chrono::Local::now().date_naive();
    let utc=|hour,minute|day.and_hms_opt(hour,minute,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap().with_timezone(&chrono::Utc);
    let start=utc(12,0);let overnight=utc(23,30);
    let ics=format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:daily\r\nSUMMARY:每日安排\r\nDTSTART:{}\r\nDTEND:{}\r\nRRULE:FREQ=DAILY;COUNT=3\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:overnight\r\nSUMMARY:跨夜安排\r\nDTSTART:{}\r\nDTEND:{}\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:all-day\r\nSUMMARY:两天休假\r\nDTSTART;VALUE=DATE:{}\r\nDTEND;VALUE=DATE:{}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",start.format("%Y%m%dT%H%M%SZ"),(start+chrono::Duration::hours(1)).format("%Y%m%dT%H%M%SZ"),overnight.format("%Y%m%dT%H%M%SZ"),(overnight+chrono::Duration::hours(1)).format("%Y%m%dT%H%M%SZ"),day.format("%Y%m%d"),(day+chrono::Duration::days(2)).format("%Y%m%d"));
    let files=vec![("sample.ics".into(),ics)];
    h.app.import_ics(&files);assert_eq!(h.app.calendar.local.len(),5);
    h.app.import_ics(&files);assert_eq!(h.app.calendar.local.len(),5,"重复导入应跳过已有日程");
    h.app.calendar.local=crate::calendar::local::load().unwrap();
    assert_eq!(h.app.calendar_entries(day).len(),3);
    assert_eq!(h.app.calendar_entries(day+chrono::Duration::days(1)).len(),3);
    assert_eq!(h.app.calendar_entries(day+chrono::Duration::days(2)).len(),1);
    assert!(h.app.calendar_entries(day+chrono::Duration::days(3)).is_empty());
}

#[test]
fn local_calendar_editor_makes_storage_scope_visible_with_connected_accounts() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let cfg=crate::calendar::Config{provider:crate::calendar::Provider::Feishu,username:"test".into(),href:None,server:None};
    crate::calendar::save_config(&cfg).unwrap();h.app.calendar=crate::app::CalendarState::load();
    let remote=crate::mail::Meeting{all_day:false,title:"飞书会议".into(),start:chrono::Local::now().timestamp()+3600,end:None,at:String::new(),uid:"remote-event".into()};
    h.app.calendar.events.push(remote.clone());
    h.app.open_calendar(None);h.hover(H::local(300.0,40.0),1.0);
    h.click(H::local(124.0,18.0));h.hover(H::local(300.0,40.0),0.3);
    assert!(h.app.calendar.draft.is_some());
    let visible=|h:&H,text:&str|h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(t) if t.galley.text()==text));
    assert!(visible(&h,"仅保存在本机"));assert!(visible(&h,"保存到本机"));
    h.click(H::local(334.0,199.0));h.frame(vec![]);
    assert!(!h.app.calendar.draft.as_ref().unwrap().error.is_empty());
    assert!(visible(&h,"仅保存在本机"),"填写错误时仍需说明保存范围");
    h.app.calendar.draft.as_mut().unwrap().title="自己的安排".into();
    h.click(H::local(334.0,199.0));h.frame(vec![]);
    assert!(h.app.calendar.draft.is_none());assert_eq!(h.app.calendar.local.len(),1);
    assert_eq!(crate::calendar::local::load().unwrap()[0].event.title,"自己的安排");
    assert_eq!(h.app.calendar.events.len(),1);assert_eq!(h.app.calendar.events[0].uid,remote.uid);
    assert_eq!(crate::calendar::load_config().unwrap(),cfg);
    let uid=h.app.calendar.local[0].event.uid.clone();h.app.open_calendar_editor(Some(&uid));h.frame(vec![]);
    assert!(visible(&h,"仅保存在本机"));assert!(visible(&h,"保存到本机"));
}

#[test]
fn local_calendar_can_create_edit_reload_and_remind_without_accounts() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.open_calendar(None);h.app.open_calendar_editor(None);
    assert!(h.app.is_editing());
    h.app.save_calendar_editor();assert!(h.app.calendar.local.is_empty());
    assert!(!h.app.calendar.draft.as_ref().unwrap().error.is_empty());
    let start=chrono::Local::now()+chrono::Duration::minutes(5);
    let end=start+chrono::Duration::hours(1);
    let draft=h.app.calendar.draft.as_mut().unwrap();draft.title="自己的安排".into();
    draft.start_date=start.format("%Y-%m-%d").to_string();draft.start_time=start.format("%H:%M").to_string();
    draft.end_date=end.format("%Y-%m-%d").to_string();draft.end_time=end.format("%H:%M").to_string();
    h.app.save_calendar_editor();
    assert!(h.app.calendar.draft.is_none());assert_eq!(h.app.calendar.local.len(),1);
    let uid=h.app.calendar.local[0].event.uid.clone();
    assert_eq!(crate::calendar::local::load().unwrap()[0].event.title,"自己的安排");
    let entries=h.app.calendar_entries(start.date_naive());assert_eq!(entries.len(),1);assert!(matches!(entries[0].sources.as_slice(),[crate::calendar::ui::Source::Local]));
    h.frame(vec![]);
    assert!(h.app.history.iter().any(|alert|alert.meeting.as_ref().is_some_and(|(event,source)|event.uid==uid && source=="本机日程")));
    h.app.open_calendar_editor(Some(&uid));h.app.calendar.draft.as_mut().unwrap().title="改后的安排".into();h.app.save_calendar_editor();
    let saved=crate::calendar::local::load().unwrap();assert_eq!(saved.len(),1);assert_eq!(saved[0].event.uid,uid);assert_eq!(saved[0].event.title,"改后的安排");
    h.app.open_calendar_editor(Some(&uid));h.hover(H::local(300.0,40.0),1.0);
    h.click(H::local(84.0,199.0));assert_eq!(h.app.calendar.local.len(),1,"删除先确认");
    h.click(H::local(84.0,199.0));assert!(h.app.calendar.local.is_empty());assert!(crate::calendar::local::load().unwrap().is_empty());
}

#[test]
fn calendar_month_errors_remain_visible_until_retry_or_month_change(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let cfg=crate::calendar::Config{provider:crate::calendar::Provider::WeCom,username:"test".into(),href:None,server:None};
    crate::calendar::save_config(&cfg).unwrap();h.app.calendar=crate::app::CalendarState::load();
    h.app.request_calendar_month(true);h.app.pending.clear();
    let month=h.app.calendar.view.month;
    h.app.calendar_event(crate::calendar::Event::MonthLoaded{config:cfg,month,result:Err("日程时间无法解析".into())});
    h.app.request_calendar_month(false);
    assert!(!h.app.calendar.view.loading,"失败后不能逐帧重试，覆盖错误状态");
    assert!(h.app.pending.is_empty());
    h.app.request_calendar_month(true);
    assert!(h.app.calendar.view.loading,"用户手动重试应立即生效");
}

#[test]
fn calendar_combines_platforms_and_settings_tabs_do_not_filter_events() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    let feishu=crate::calendar::Config{provider:crate::calendar::Provider::Feishu,username:"飞书账号".into(),href:None,server:None};
    let ding=crate::calendar::Config{provider:crate::calendar::Provider::DingTalk,username:"钉钉账号".into(),..feishu.clone()};
    crate::calendar::save_config(&feishu).unwrap();crate::calendar::save_config(&ding).unwrap();
    h.app.calendar=crate::app::CalendarState::load();
    let day=chrono::Local::now().date_naive();
    let event=|uid:&str|crate::mail::Meeting{all_day:false,end:None,title:uid.into(),start:chrono::Local::now().timestamp()+60,at:"18:00".into(),uid:uid.into()};
    for (cfg,uid) in [(&feishu,"飞书会议"),(&ding,"钉钉会议")] {
        h.app.calendar_event(crate::calendar::Event::Synced{config:cfg.clone(),events:vec![event(uid)],at:"17:00".into()});
    }
    assert_eq!(h.app.calendar_entries(day).len(),2);
    h.frame(vec![]);
    for provider in [feishu.provider,ding.provider] {
        assert!(h.app.history.iter().any(|alert|alert.meeting.as_ref().is_some_and(|(_,source)|source==provider.label())),"所有平台都应提醒");
    }
    h.app.select_calendar_provider(feishu.provider);
    assert_eq!(h.app.calendar_entries(day).len(),2);
    assert_eq!(h.app.calendar.form.username,feishu.username);
    h.app.select_calendar_provider(crate::calendar::Provider::WeCom);
    assert_eq!(h.app.calendar_entries(day).len(),2);
    assert!(h.app.calendar.form.username.is_empty());
    h.app.request_calendar_month(true);
    h.app.calendar_event(crate::calendar::Event::MonthLoaded{config:feishu.clone(),month:crate::calendar::month_start(day),result:Ok(vec![event("飞书月历")])});
    assert!(h.app.calendar.view.loading,"另一平台尚未返回，仍应保持同步状态");
    h.app.calendar_event(crate::calendar::Event::MonthLoaded{config:ding.clone(),month:crate::calendar::month_start(day),result:Err("钉钉暂时不可用".into())});
    assert!(!h.app.calendar.view.loading);
    assert_eq!(h.app.calendar_entries(day).len(),2,"单平台失败不能清掉另一平台或自己的提醒缓存");
    let shared=event("同一个会议");
    for config in [&feishu,&ding] {
        h.app.calendar_event(crate::calendar::Event::MonthLoaded{config:config.clone(),month:crate::calendar::month_start(day),result:Ok(vec![shared.clone()])});
    }
    let entries=h.app.calendar_entries(day);
    assert_eq!(entries.len(),1);
    assert_eq!(entries[0].sources.len(),2,"同一个会议只显示一次，但保留两个平台的来源");
    h.app.select_calendar_provider(feishu.provider);
    h.app.forget_calendar();
    assert_eq!(h.app.calendar_entries(day).len(),1);
    h.app.calendar_event(crate::calendar::Event::Synced{config:feishu,events:vec![event("旧连接结果")],at:"17:01".into()});
    assert_eq!(h.app.calendar_entries(day).len(),1);
}

#[test]
fn switching_music_source_discards_previous_lyrics() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"cloudmusic.exe".into(),title:"Track".into(),artist:"Artist".into(),duration:120.0,..Default::default()};
    h.frame(vec![]);
    for source in ["QQMusic.exe","cloudmusic.exe","汽水音乐","KuGou.exe","QQMusic.exe"] {
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:01]旧来源歌词"));
        h.media.source=source.into();
        h.frame(vec![]);
        assert!(h.app.ly.lyrics.is_none(),"切换到 {source} 后必须重新选择歌词来源");
    }
}

#[test]
fn closing_a_player_then_starting_native_netease_restores_music_and_lyrics(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{source:"SodaMusic.exe".into(),title:"旧歌曲".into(),artist:"歌手".into(),playing:true,duration:120.0,..Default::default()};h.frame(vec![]);
    h.app.set_lyrics("旧歌曲","歌手",120.0,crate::lyrics::parse_lrc("[00:01]旧歌词"));
    h.media=Default::default();h.frame(vec![]);assert!(h.app.media.title.is_empty());assert!(h.app.ly.lyrics.is_none());
    h.media=crate::smtc::NowPlaying{source:"cloudmusic.exe".into(),song_id:Some(123),title:"新歌曲".into(),artist:"歌手".into(),playing:true,position:7.0,duration:120.0,..Default::default()};h.frame(vec![]);
    h.app.set_lyrics("新歌曲","歌手",120.0,crate::lyrics::parse_lrc("[00:05]新歌词\n[00:10]下一句"));h.frame(vec![]);
    assert_eq!(h.app.media.title,"新歌曲");assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("新歌词"));
    h.media.song_id=Some(124);h.frame(vec![]);assert!(h.app.ly.lyrics.is_none(),"歌名相同但歌曲 ID 不同仍需重新取词");
}

#[test]
fn quality_late_calendar_response_does_not_restore_disconnected_account() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let config=crate::calendar::Config{provider:crate::calendar::Provider::Feishu,username:"old".into(),href:None,server:None};
    h.app.forget_calendar();
    h.app.calendar_event(crate::calendar::Event::Synced{config:config.clone(),events:vec![],at:"12:00".into()});
    assert!(!h.app.calendar.connected && h.app.calendar.last_sync.is_empty());
    h.app.calendar.config=Some(crate::calendar::Config{username:"new".into(),..config.clone()});
    h.app.calendar.connected=true;
    h.app.calendar_event(crate::calendar::Event::Failed{config,message:"旧连接失败".into()});
    assert!(h.app.calendar.connected && h.app.calendar.error.is_empty());
}

#[test]
fn kugou_external_seek_updates_lyrics_in_both_directions(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{source:"KuGou.exe".into(),title:"Track".into(),artist:"Artist".into(),duration:120.0,playing:true,..Default::default()};
    h.frame(vec![]);
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]开头\n[00:30]中段\n[01:00]后段"));
    for (position,line) in [(65.0,"后段"),(10.0,"开头"),(35.0,"中段")] {
        h.media.position=position;h.media.sampled_at=Some(std::time::Instant::now());h.frame(vec![]);
        assert_eq!(h.app.island_lyric().map(|(_,text)|text),Some(line),"酷狗跳转后应立即切换到对应歌词");
    }
}

#[test]
fn music_clients_keep_lyrics_across_two_repeat_cycles(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for source in ["kugou","QQMusic.exe","cloudmusic.exe","汽水音乐","Spotify.exe","kwmusic.exe"] {
        let mut h=H::new();h.app.settings.island_lyrics=true;
        h.media=crate::smtc::NowPlaying{source:source.into(),title:"循环测试".into(),artist:"歌手".into(),duration:120.0,playing:true,..Default::default()};
        h.frame(vec![]);h.app.set_lyrics("循环测试","歌手",120.0,crate::lyrics::parse_lrc("[00:00]开头\n[00:05]第一段\n[01:50]尾声"));
        for (position,line) in [(119.0,"尾声"),(0.0,"开头"),(8.0,"第一段"),(119.0,"尾声"),(0.0,"开头"),(8.0,"第一段")] {
            h.media.position=position;h.media.sampled_at=Some(std::time::Instant::now());h.frame(vec![]);
            assert_eq!(h.app.island_lyric().map(|(_,text)|text),Some(line),"{source} 第二、三遍也必须显示相应歌词");
        }
    }
}

#[test]
fn failed_lyrics_can_be_retried_from_the_music_page(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),duration:120.0,..Default::default()};
    h.app.ly.key="Track|Artist|120".into();
    h.app.ly.error=Some(crate::lyrics::FetchError::Unavailable("HTTP 502".into()));
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="暂未获取歌词，请重试")),"展开内应保留歌词错误提示");
    h.click(H::local(310.0,68.0));
    assert!(h.app.ly.key.is_empty(),"点击错误提示后应允许重新请求当前歌曲");
}

#[test]
fn music_lyrics_show_in_a_separate_right_column() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:01]上一句\n[00:05]当前歌词\n[00:10]下一句"));
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    let current=h.shapes.iter().find_map(|shape|match &shape.shape {
        egui::Shape::Text(text) if text.galley.text()=="当前歌词"=>Some(text.pos),_=>None,
    }).expect("展开页应显示同步歌词");
    assert!(current.x>=H::local(243.0,0.0).x,"歌词必须位于播放器右侧");
    let title=h.shapes.iter().find_map(|shape|match &shape.shape {
        egui::Shape::Text(text) if text.galley.text()=="Track"=>Some(text.pos),_=>None,
    }).unwrap();
    assert!(title.x<H::local(233.0,0.0).x);
    h.click(H::local(74.0,118.0));
    assert!(!h.app.show_lyrics);
    h.frame(vec![]);
    assert!(!h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()=="当前歌词")));
}

#[test]
fn disabling_lyrics_disconnects_pending_workers(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"KuGou.exe".into(),title:"测试歌曲".into(),artist:"歌手".into(),..Default::default()};
    h.frame(vec![]);h.app.opts.fetch_lyrics=true;
    h.app.show_lyrics=false;h.app.settings.island_lyrics=false;
    h.app.ly.key="测试歌曲|歌手|0".into();h.app.ly.loading=true;
    let (network,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    let (native,rx)=std::sync::mpsc::channel();h.app.ly.native_rx=Some(rx);
    h.frame(vec![]);
    assert!(native.send(None).is_err(),"关闭歌词后应停止本机轮询");
    assert!(network.send(Err(crate::lyrics::FetchError::NotFound)).is_err());
    assert!(h.app.ly.key.is_empty(),"再次开启时应能重新加载歌词");
}

#[test]
fn late_native_lyrics_bypass_pending_network_for_both_clients(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for source in ["KuGou.exe","SodaMusic.exe"] {
        let mut h=H::new();
        h.media=crate::smtc::NowPlaying{source:source.into(),title:"测试歌曲".into(),artist:"歌手".into(),..Default::default()};
        h.frame(vec![]);h.app.opts.fetch_lyrics=true;h.app.show_lyrics=true;
        h.app.ly.key="测试歌曲|歌手|0".into();h.app.ly.loading=true;
        let (network,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
        let (native,rx)=std::sync::mpsc::channel();h.app.ly.native_rx=Some(rx);
        native.send(Some(crate::lyrics::parse_lrc("[00:01.00]本机歌词"))).unwrap();h.frame(vec![]);
        assert!(!h.app.ly.loading,"本机歌词不能等待在线请求结束");
        assert_eq!(h.app.ly.lyrics.as_ref().unwrap().lines[0].text,"本机歌词");
        assert!(network.send(Err(crate::lyrics::FetchError::NotFound)).is_err(),"迟到的网络结果必须丢弃");
    }
}

#[test]
fn kugou_fallback_waits_for_late_native_lyrics(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"kugou".into(),title:"测试歌曲".into(),artist:"歌手".into(),position:22.8,..Default::default()};
    h.frame(vec![]);h.app.opts.fetch_lyrics=true;h.app.show_lyrics=true;h.app.ly.key="测试歌曲|歌手|0".into();
    let (tx,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    tx.send(Ok((crate::lyrics::parse_lrc("[00:23.930]测试歌词"),true))).unwrap();h.frame(vec![]);
    assert!(h.app.ly.retry_at.is_some(),"临时通用歌词必须继续等待迟到的酷狗原生歌词");
    h.t=h.app.ly.retry_at.unwrap()+0.1;h.frame(vec![]);
    assert_eq!(h.app.ly.lyrics.as_ref().unwrap().lines[0].t,23.930,"接回原生歌词期间不能清空现有歌词");assert!(!h.app.ly.loading);
    let (tx,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    tx.send(Ok((crate::lyrics::parse_lrc("[00:22.991]测试歌词"),false))).unwrap();h.frame(vec![]);
    assert!((h.app.ly.lyrics.as_ref().unwrap().lines[0].t-22.991).abs()<0.001);assert!(h.app.ly.retry_at.is_none(),"接回原生歌词后应停止重查");
}

#[test]
fn kugou_missing_lyrics_rechecks_late_cache_without_flashing_loading() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"kugou".into(),title:"不存在的测试歌曲".into(),artist:"测试歌手".into(),duration:120.0,..Default::default()};
    h.frame(vec![]);h.app.opts.fetch_lyrics=true;h.app.show_lyrics=true;
    h.app.ly.key="不存在的测试歌曲|测试歌手|120".into();
    let (tx,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    tx.send(Err(crate::lyrics::FetchError::NotFound)).unwrap();h.frame(vec![]);
    let retry=h.app.ly.retry_at.expect("酷狗尚未写入缓存时仍需自动检查");
    assert!((retry-h.t-5.0).abs()<0.001);
    // 歌词通道全进程只有 1 个名额，前面测试的歌词线程可能还在查网络；占着时这一帧会直接跳过。
    let until=std::time::Instant::now()+std::time::Duration::from_secs(20);
    while crate::resources::LYRICS.active()>0 && std::time::Instant::now()<until{std::thread::sleep(std::time::Duration::from_millis(10));}
    assert_eq!(crate::resources::LYRICS.active(),0,"歌词名额被其他测试长期占用");
    h.t=retry+0.1;h.frame(vec![]);
    assert!(!h.app.ly.loading,"检查本机缓存不能反复闪出加载提示");
    assert_eq!(h.app.ly.error,Some(crate::lyrics::FetchError::NotFound));
    let (tx,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    tx.send(Ok((crate::lyrics::parse_lrc("[00:01.00]迟到的原生歌词"),false))).unwrap();h.frame(vec![]);
    assert!(h.app.ly.lyrics.is_some());assert!(h.app.ly.error.is_none());assert!(h.app.ly.retry_at.is_none());
}

#[test]
fn island_lyrics_pause_returns_to_resident_and_resume_follows_current_position(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|error|error.into_inner());
    let visible=|h:&H,label:&str|h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()==label));
    for synced in [true,false] {
        let mut h=H::new();
        h.app.settings.tray_weather=false;h.app.settings.tray_focus=false;
        h.power=(Some(75),false);h.app.pomo.running=true;h.app.pomo.remaining=600.0;
        h.app.weather.snapshot=Some(crate::weather::Snapshot{temp:21.0,..Default::default()});
        h.media=crate::smtc::NowPlaying{source:"cloudmusic.exe".into(),title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,..Default::default()};
        for _ in 0..90 {h.frame(vec![]);}
        let resident_width=h.app.current_width();let resident_height=h.hit.height();
        h.app.settings.island_lyrics=true;h.media.playing=true;
        let lyrics=if synced {crate::lyrics::parse_lrc("[00:05]当前歌词\n[00:10]下一句歌词")}else{crate::lyrics::from_plain("没有时间戳的歌词")};
        h.app.set_lyrics("Track","Artist",120.0,lyrics);
        for _ in 0..90 {h.frame(vec![]);}
        assert!(visible(&h,if synced{"当前歌词"}else{"Track"}));
        assert_eq!(h.app.media_poll_interval(),crate::smtc::POLL_LYRICS);
        h.media.playing=false;h.frame(vec![]);
        assert!(h.app.island_lyric().is_none(),"暂停后应退出收起态歌词显示");
        for _ in 0..90 {h.frame(vec![]);}
        assert!(!visible(&h,"当前歌词") && !visible(&h,"Track"));
        assert!((h.app.current_width()-resident_width).abs()<2.0,"暂停后应恢复原常驻宽度");
        assert!((h.hit.height()-resident_height).abs()<0.1);
        assert!(visible(&h,"09:41"));
        assert!(visible(&h,"21°"),"普通常驻仍应显示天气");
        let secs=h.app.pomo.remaining.ceil() as u32;
        assert!(visible(&h,&format!("{:02}:{:02}",secs/60,secs%60)),"暂停音乐不能隐藏专注倒计时");
        assert!(h.app.settings.island_lyrics && h.app.ly.lyrics.is_some(),"暂停只切换显示形态，应保留开关和歌词缓存");
        assert_eq!(h.app.media_poll_interval(),crate::smtc::POLL_BACKGROUND);
        h.media.position=12.0;for _ in 0..30 {h.frame(vec![]);}
        assert!(!visible(&h,"下一句歌词"));
        h.media.playing=true;for _ in 0..90 {h.frame(vec![]);}
        assert!(visible(&h,if synced{"下一句歌词"}else{"Track"}),"继续播放后应按最新进度恢复歌词或歌名");
        assert!(!visible(&h,"09:41"));
        assert_eq!(h.app.media_poll_interval(),crate::smtc::POLL_LYRICS);
    }
}

#[test]
fn music_lyrics_island_toggle_is_independent_and_persistent() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.show_lyrics=false;h.app.settings.lyrics_enabled=false;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,playing:true,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]当前歌词\n[00:10]下一句"));
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.click(H::local(313.0,118.0));
    assert!(h.app.settings.island_lyrics);
    assert!(!h.app.show_lyrics && !h.app.settings.lyrics_enabled);
    let saved=crate::model::Settings::load();
    assert!(saved.island_lyrics && !saved.lyrics_enabled);
    h.hover(Pos2::new(WIN_W/2.0,PH+crate::app::STATUS_H+30.0),1.0);
    assert!(!h.app.open);
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("当前歌词"));
    let pet_width=if h.app.pet_enabled(){48.0}else{0.0};
    assert!((h.app.current_width()-crate::app::LYRIC_BAR_W-pet_width).abs()<2.0);
}

#[test]
fn island_lyrics_arrive_just_before_vocals_and_finish_fading_before_the_timestamp(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:9.4,duration:120.0,playing:true,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]当前歌词\n[00:10]下一句"));
    for _ in 0..60 {h.frame(vec![]);}
    for step in 1..=30 {
        h.media.position=9.4+step as f64/60.0;
        h.frame(vec![]);
        if step==6 {assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("当前歌词"),"不能提前半秒就跳到下一句");}
    }
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("下一句"),"下一句应在开唱前提前出现");
    let text=h.shapes.iter().find_map(|shape|match &shape.shape {
        egui::Shape::Text(text) if text.galley.text()=="下一句"=>Some(text),_=>None,
    }).expect("开唱前应能看到下一句");
    assert!(text.opacity_factor>0.95,"开唱前应完成淡入，不能等到开唱后才清晰");
    h.media.playing=false;h.frame(vec![]);
    assert!(h.app.island_lyric().is_none(),"暂停时应退出收起态歌词显示");
    h.media.playing=true;h.frame(vec![]);
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("下一句"),"继续播放时不应跳回上一句");
    h.media.position=4.5;h.frame(vec![]);
    assert!(h.app.island_lyric().is_none(),"前奏不能过早显示第一句");
    h.media.position=4.75;h.frame(vec![]);
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("当前歌词"),"第一句也应提前出现");
}

#[test]
fn expanded_and_segmented_lyrics_share_the_early_display_time(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:9.8,duration:120.0,playing:true,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]当前歌词\n[00:10]下一句"));
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    let next=h.shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Text(text)if text.galley.text()=="下一句"=>Some(text),_=>None}).unwrap();
    assert_eq!(next.galley.job.sections[0].format.font_id.size,12.0,"展开歌词也应在开唱前高亮下一句");
    h.hover(Pos2::new(WIN_W/2.0,PH+crate::app::STATUS_H+30.0),1.0);
    let line="愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光";
    h.media.position=6.0;h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc(&format!("[00:05]{line}\n[00:15]下一句")));
    for _ in 0..60{h.frame(vec![]);}
    let shown=|h:&H|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if line.contains(t.galley.text())&&!t.galley.text().is_empty()=>Some(t.galley.text().to_owned()),_=>None}).unwrap();
    let first=shown(&h);
    let boundary=5.0+10.0*first.chars().count()as f64/line.chars().count()as f64;
    h.media.position=boundary-0.2;for _ in 0..30{h.frame(vec![]);}
    assert_ne!(shown(&h),first,"长句的后半段也应使用相同的提前量");
}

#[test]
fn island_lyrics_center_segments_in_a_fixed_height(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,playing:true,..Default::default()};
    for line in ["晚风","愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光","晚风"] {
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc(&format!("[00:05]{line}\n[00:15]下一句")));
        for _ in 0..90 {
            h.frame(vec![]);
            assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="09:41")),"切换显示段时应保持歌词布局");
        }
        let text=h.shapes.iter().find_map(|shape|match &shape.shape {
            egui::Shape::Text(text) if !text.galley.text().is_empty() && line.contains(text.galley.text())=>Some(text),_=>None,
        }).expect("岛上应显示当前歌词的一段");
        let center=text.pos.x+text.galley.rect.center().x;
        let offset=if h.app.pet_enabled(){24.0}else{0.0};
        assert!((center-WIN_W/2.0-offset).abs()<1.5,"歌词应在内容区居中");
        assert!(!text.galley.elided,"显示段应完整显示");
        assert_eq!(text.galley.rows.len(),1,"每次只显示一段歌词");
        assert!((h.hit.height()-(crate::app::BAR_H+4.0)).abs()<0.1,"岛应保持固定高度");
        assert!(text.visual_bounding_rect().bottom()<=h.hit.bottom(),"文字应完整显示在岛内");
    }
}

#[test]
fn island_lyrics_keep_title_fallback_and_intro_and_gaps(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:1.0,duration:120.0,playing:true,..Default::default()};
    for state in 0..6 {
        h.app.ly=Default::default();
        h.app.ly.key="Track|Artist|120".into();
        h.media.position=if state==1 {7.0}else{1.0};
        match state {
            0=>h.app.ly.lyrics=Some(crate::lyrics::parse_lrc("[00:05]第一句")),
            1=>h.app.ly.lyrics=Some(crate::lyrics::parse_lrc("[00:01]第一句\n[00:05]\n[00:10]下一句")),
            2=>h.app.ly.loading=true,
            3=>h.app.ly.error=Some(crate::lyrics::FetchError::NotFound),
            4=>h.app.ly.error=Some(crate::lyrics::FetchError::Unavailable("503".into())),
            _=>h.app.ly.lyrics=Some(crate::lyrics::from_plain("没有时间戳")),
        }
        for _ in 0..90 {h.frame(vec![]);}
        if let Some(error)=&h.app.ly.error {
            assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==error.label())),"歌词错误提示只应在展开内显示");
        }
        assert!((h.app.current_width()-crate::app::LYRIC_BAR_W).abs()<2.0,"状态 {state} 应保持音乐信息岛宽度");
        assert!(h.region.height()>30.0);
        if state>=2 {
            assert!(h.app.compact_notices(),"歌名兜底仍属于岛上歌词模式");
            assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="Track"&&t.opacity_factor>0.95)),"状态 {state} 应显示当前歌名");
        } else {
            let label=if state==0{"前奏"}else{"间奏"};
            assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==label)),"同步歌词的前奏和间奏仍应显示状态");
        }
    }
}

#[test]
fn island_lyrics_title_fallback_shows_cover_and_elides_long_names(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"没有歌词的钢琴曲".repeat(20),artist:"示例演奏者".into(),playing:true,artwork:Some(std::sync::Arc::new(egui::ColorImage::new([8,8],egui::Color32::BLUE))),..Default::default()};
    for _ in 0..90 {h.frame(vec![]);}
    let text=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==h.media.title=>Some(t),_=>None}).expect("无歌词时应绘制当前歌名");
    assert_eq!(text.galley.rows.len(),1,"长歌名只显示一行");
    assert!(text.galley.elided,"长歌名应在末尾省略");
    assert!(h.region.contains_rect(text.visual_bounding_rect()),"长歌名不能超出岛体");
    let cover=h.app.music_artwork.as_ref().unwrap().1.id();
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r)if r.brush.as_ref().is_some_and(|b|b.fill_texture_id==cover)&&h.region.contains_rect(r.rect))),"无歌词时仍应绘制封面");
}

#[test]
fn island_lyrics_title_fallback_follows_track_changes_and_normal_mode(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"上一首".into(),artist:"示例歌手".into(),position:7.0,duration:120.0,playing:true,..Default::default()};
    h.app.set_lyrics("上一首","示例歌手",120.0,crate::lyrics::parse_lrc("[00:05]上一首的歌词"));
    for _ in 0..90 {h.frame(vec![]);}
    h.media.title="下一首纯音乐".into();h.media.playing=false;
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.app.island_lyric().is_none(),"换歌后不能显示上一首歌词");
    h.media.playing=true;for _ in 0..60 {h.frame(vec![]);}
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==h.media.title)),"继续播放且歌词未到达时应显示新歌名");
    h.media.playing=false;
    h.app.set_lyrics("下一首纯音乐","示例歌手",120.0,crate::lyrics::parse_lrc("[00:05]新歌的当前歌词"));
    for _ in 0..30 {h.frame(vec![]);}
    assert!(h.app.island_lyric().is_none(),"暂停时歌词到达也应保持常驻形态");
    h.media.playing=true;for _ in 0..30 {h.frame(vec![]);}
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="新歌的当前歌词")),"继续播放后应自动恢复当前歌词");
    for mode in 0..3 {
        h.app.settings.island_lyrics=mode!=0;
        h.app.settings.visible_pages[0]=mode!=1;
        if mode==2 {h.media=Default::default();}
        for _ in 0..90 {h.frame(vec![]);}
        assert!(!h.app.compact_notices(),"关闭歌词、隐藏音乐页或播放器退出后应恢复常驻，状态 {mode}");
    }
}

#[test]
#[ignore="按需导出岛上歌词的实际渲染画面"]
fn island_lyrics_layout_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_LYRIC_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    let mut cover=egui::ColorImage::new([64,64],egui::Color32::BLACK);
    for y in 0..64 {for x in 0..64 {
        let moon=(x as i32-44).pow(2)+(y as i32-18).pow(2)<64;
        cover[(x,y)]=if moon {egui::Color32::from_rgb(255,226,174)}
            else if y>44+x/5 {egui::Color32::from_rgb(29,61,70)}
            else {egui::Color32::from_rgb(68+y as u8,114+y as u8,151+y as u8/2)};
    }}
    let cover=std::sync::Arc::new(cover);
    for name in ["lyrics","loading","missing","no-cover","long-title","paused","intro","gap","idle"] {
        let mut h=H::new();h.app.settings.island_lyrics=true;
        h.media=crate::smtc::NowPlaying{title:"晚风与星光".into(),artist:"示例歌手".into(),position:7.0,duration:120.0,playing:name!="paused",artwork:Some(cover.clone()),..Default::default()};
        if name=="long-title" {h.media.title="愿每一个认真生活的人都能在平凡的日子里找到属于自己的光（钢琴独奏版）".into();}
        h.app.ly.key=format!("{}|{}|120",h.media.title,h.media.artist);
        match name {
            "lyrics"|"paused"=>h.app.set_lyrics("晚风与星光","示例歌手",120.0,crate::lyrics::parse_lrc("[00:05]愿你在平凡的日子里，找到自己的光")),
            "loading"=>h.app.ly.loading=true,
            "intro"=>h.app.set_lyrics("晚风与星光","示例歌手",120.0,crate::lyrics::parse_lrc("[00:20]晚风")),
            "gap"=>h.app.set_lyrics("晚风与星光","示例歌手",120.0,crate::lyrics::parse_lrc("[00:01]第一句\n[00:05]\n[00:10]下一句")),
            "idle"=>h.media=Default::default(),
            _=>h.app.ly.error=Some(crate::lyrics::FetchError::NotFound),
        }
        if name=="no-cover" {h.media.artwork=None;}
        let mut raster=crate::preview_raster::Raster::new(1280,540);
        for frame in 0..90 {
            let t=frame as f64/60.0;
            let env=Env{t,clock:"09:41:00".into(),today:"2026-11-06".into(),date:"11月6日 周五".into(),media:h.media.clone(),..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H))),time:Some(t),
                viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=ui.max_rect();h.app.frame(ui,win,&env);});
            });
            raster.update_textures(&output.textures_delta);
            if frame==89 {
                raster.buf.fill([0.95,0.96,0.98,1.0]);
                raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                let image=image::RgbaImage::from_raw(1280,540,rgba).unwrap();
                image::imageops::crop_imm(&image,220,0,840,110).to_image().save(dest.join(format!("{name}.png"))).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }
}

#[test]
fn music_lyrics_keep_syncing_when_only_the_collapsed_lyrics_are_enabled() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.opts.fetch_lyrics=true;h.app.show_lyrics=false;h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,playing:true,..Default::default()};
    let (tx,rx)=std::sync::mpsc::channel();
    h.app.ly.key="Track|Artist|120".into();h.app.ly.loading=true;h.app.ly.rx=Some(rx);
    tx.send(Ok((crate::lyrics::parse_lrc("[00:05]当前歌词\n[00:10]下一句"),false))).unwrap();
    h.frame(vec![]);
    assert!(!h.app.ly.loading);
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("当前歌词"));
    h.media.position=12.0;h.frame(vec![]);
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("下一句"));
}

#[test]
fn music_lyrics_respect_the_switch_and_do_not_show_stale_or_unsynced_lines() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,playing:true,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]当前歌词"));
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()>30.0,"主动开启岛上歌词后应可在收起状态阅读");
    h.app.settings.island_lyrics=false;
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.app.island_lyric().is_none() && !h.app.compact_notices());
    h.app.settings.island_lyrics=true;
    h.media.title="Another track".into();h.frame(vec![]);
    assert!(h.app.island_lyric().is_none(),"换歌后不能显示上一首的歌词");
    h.app.set_lyrics("Another track","Artist",120.0,crate::lyrics::from_plain("没有时间戳的歌词"));h.frame(vec![]);
    assert!(h.app.island_lyric().is_none(),"不能将纯文本歌词伪装成当前句");
}

#[test]
fn quiet_water_hours_cover_overnight(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.water_start=22;h.app.settings.water_end=7;
    assert!(h.app.water_hours(23));assert!(h.app.water_hours(2));assert!(!h.app.water_hours(12));
}

#[test]
fn starting_timer_keeps_lyrics_album_and_mascot_with_a_compact_countdown(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.island_lyrics=true;h.app.settings.pet=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,
        artwork:Some(std::sync::Arc::new(egui::ColorImage::new([32,32],egui::Color32::from_rgb(70,120,220)))),..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]晚风吹过了山岗\n[00:15]下一句"));
    for _ in 0..120 {h.frame(vec![]);}
    for phase in [crate::app::Phase::Focus,crate::app::Phase::Rest] {
        h.app.pomo.phase=phase;h.app.pomo.remaining=h.app.phase_secs(phase);h.app.pomo.running=true;
        for _ in 0..120 {
            h.frame(vec![]);
            assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("晚风吹过了山岗"),"计时不能关闭岛上歌词");
            assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="晚风吹过了山岗" && t.galley.rows.len()==1)));
            assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="09:41")));
        }
        let texture=h.app.music_artwork.as_ref().unwrap().1.id();
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.brush.as_ref().is_some_and(|b|b.fill_texture_id==texture))));
        let secs=h.app.pomo.remaining.ceil() as u32;let countdown=format!("{:02}:{:02}",secs/60,secs%60);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==countdown)),"歌词模式中也应显示计时");
        assert!(h.app.current_width()<=PW+0.1);assert!((h.hit.height()-34.0).abs()<0.1);
    }
    h.app.pomo.running=false;for _ in 0..90 {h.frame(vec![]);}
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("晚风吹过了山岗"));
}

#[test]
fn lyric_app_notification_respects_preview_and_restores_current_lyrics(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for preview in [false,true]{
        let mut h=H::new();h.app.settings.island_lyrics=true;h.app.settings.read_notifications=true;h.app.settings.notification_preview=preview;
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]首句歌词\n[00:20]后续歌词"));
        for _ in 0..60{h.frame(vec![]);}
        h.app.receive_notification("测试消息".into(),"notice.test".into(),"下午一起开会".into());
        h.frame(vec![]);let at=h.app.notice.as_ref().unwrap().at;
        let mut slopes=Vec::new();
        for elapsed in [0.35,0.60,1.0]{
            h.t=at+elapsed;h.frame(vec![]);
            let slope=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Path(p)if p.points.len()==17=>Some(p.points[13].y-p.points[16].y),_=>None}).expect("应绘制通知铃铛");
            slopes.push(slope);
        }
        assert!(slopes[0]*slopes[1]<0.0,"铃铛应向左右两个方向摆动");assert!(slopes[2].abs()<0.01,"摆动结束后应回正");
        for _ in 0..60{h.frame(vec![]);}
        let visible=|h:&H,label:&str|h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==label&&t.opacity_factor>0.9&&s.clip_rect.contains_rect(t.visual_bounding_rect())));
        assert_eq!(visible(&h,"下午一起开会"),preview);
        assert_eq!(visible(&h,"首句歌词"),!preview);
        assert_eq!(h.app.compact_notices(),!preview);
        assert!(h.region.height()<70.0,"通知应留在岛内，不能自动弹出下方卡片");
        assert!(!h.app.open);
        if !preview {assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Circle(c)if c.fill==egui::Color32::from_rgb(255,72,72))),"通知铃铛应有红点");}
        let active=h.app.notice.as_ref().unwrap();h.t=active.at+active.duration-0.1;h.media.position=25.0;
        h.frame(vec![]);assert!(h.app.notice.is_some());
        for _ in 0..90{
            h.frame(vec![]);
            if h.app.notice.is_none(){assert!(!visible(&h,"Track")&&!visible(&h,"Artist"),"恢复歌词不能闪出播放器");}
        }
        assert!(h.app.notice.is_none());assert!(!visible(&h,"下午一起开会"));assert!(visible(&h,"后续歌词"));
    }
}

#[test]
fn full_notifications_return_when_island_lyrics_are_not_visible(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (lyrics,music) in [(false,false),(false,true),(true,false)]{
        let mut h=H::new();h.app.settings.pet=true;h.app.settings.island_lyrics=lyrics;
        if music {h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};}
        for _ in 0..90{h.frame(vec![]);}h.app.alert("大写锁定","已开启",5,false);for _ in 0..60{h.frame(vec![]);}
        for label in ["大写锁定","已开启"]{assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==label&&t.opacity_factor>0.9&&s.clip_rect.contains(t.visual_bounding_rect().center()))),"非岛上歌词状态应直接显示通知文案：{label}");}
        assert!((h.app.current_width()-crate::app::HUD_W-44.0).abs()<1.0,"完整提示保留吉祥物与文案的宽度");
    }
}

#[test]
fn lyric_notifications_extend_right_and_keep_the_lyrics_in_place(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let text=|h:&H,label:&str|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label&&t.opacity_factor>0.9&&s.clip_rect.contains(t.visual_bounding_rect().center())=>Some(t.visual_bounding_rect()),_=>None}).unwrap_or_else(||panic!("应保留可见内容：{label}"));
        let mut h=H::new();h.app.settings.pet=true;h.app.settings.island_lyrics=true;h.power=(Some(75),false);
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗\n[01:00]下一句"));
        for _ in 0..90{h.frame(vec![]);}
        let label="晚风吹过了山岗";let original=text(&h,label);let right=WIN_W/2.0+h.app.current_width()/2.0;
        h.app.alert("大写锁定","已开启",5,false);
        for _ in 0..60{h.frame(vec![]);assert!(text(&h,label).center().distance(original.center())<0.2,"提示不能挤动或替换原内容");}
        assert!((h.app.current_width()-(right-WIN_W/2.0)*2.0-44.0).abs()<1.0,"通知图标占用 44 点宽度");
        assert!(h.region.right()<right+64.0);assert!(!h.app.open);
        h.t+=3.0;for _ in 0..100{h.frame(vec![]);}
        assert!(text(&h,label).center().distance(original.center())<0.2);
}

#[test]
fn an_open_page_and_lyrics_can_coexist_with_notifications_and_the_bell(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗\n[01:00]下一句"));
    for _ in 0..90{h.frame(vec![]);}
    h.app.notification_apps.items.push(("mail.test".into(),"邮件".into(),true));h.app.settings.read_notifications=true;h.app.receive_notification("邮件".into(),"mail.test".into(),"收到新邮件".into());h.frame(vec![]);
    assert!(h.app.island_lyric().is_some(),"通知铃铛不能关闭岛上歌词");
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.hover(H::local(300.0,60.0),0.5);assert!(h.app.open);
    h.app.alert("键盘输入语言","中文",5,false);for _ in 0..60{h.frame(vec![]);}
    assert!(h.app.open);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="Track")),"展开页面保留歌曲信息");
    assert!(!h.app.compact_notices(),"展开页面时不使用岛上歌词的小图标提示");
}

#[test]
fn calendar_notice_opens_the_calendar_instead_of_mail() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    assert!(h.app.settings.calendar_auto_sync);
    h.app.settings.calendar_auto_sync=false;
    h.app.settings.save();
    assert!(!crate::model::Settings::load().calendar_auto_sync,"自动同步选择应持久保存");
    let start=chrono::Local::now().timestamp()+600;
    h.app.calendar.config=Some(crate::calendar::Config{provider:crate::calendar::Provider::Feishu,username:"demo".into(),href:None,server:None});
    h.app.calendar_event(crate::calendar::Event::Synced{config:h.app.calendar.config.clone().unwrap(),events:vec![crate::mail::Meeting{ all_day:false, end:None,title:"产品评审".into(),start,at:"10-31 15:00".into(),uid:"calendar-review".into()}],at:"10-31 14:50".into()});
    for _ in 0..60 {h.frame(vec![]);}
    h.click_notice_badge();
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="查看")),"会议提醒按钮使用「查看」");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="产品评审")),"会议名称应独立显示");
    h.click_text("查看");
    assert!(!h.app.settings_open,"会议提醒不应打开设置页");
    assert_eq!(h.app.page,7,"查看会议必须展开岛上的日程页");
    assert!(h.app.open);
    let date=chrono::DateTime::from_timestamp(start,0).unwrap().with_timezone(&chrono::Local).date_naive();
    assert_eq!(h.app.calendar.view.day,date);
    assert_eq!(h.app.calendar.view.focus.as_deref(),Some(crate::calendar::dedupe_key(&h.app.calendar.events[0])).as_deref());
    assert_ne!(h.app.page,5);
}

#[test]
fn mail_meeting_notice_opens_its_calendar_day() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.mail.recent.push(crate::mail::Item {received_unix: None, account:"demo@example.com".into(),from:"项目组".into(),subject:"会议邀请".into(),html: String::new().into(), summary:String::new().into(),links:vec![],code:None,time:"14:50".into(),uid:9,
        meeting:Some(crate::mail::Meeting{ all_day:false, end:None,title:"产品评审".into(),start:chrono::Local::now().timestamp()+600,at:"10-31 15:00".into(),uid:"mail-review".into()})});
    for _ in 0..60 {h.frame(vec![]);}
    h.click_text("查看");
    assert_eq!(h.app.page,7);
    assert!(h.app.open);
    assert_ne!(h.app.page,5);
    let entries=h.app.calendar_entries(h.app.calendar.view.day);
    assert_eq!(entries.len(),1);
    assert!(matches!(entries[0].sources[0],crate::calendar::ui::Source::Mail("demo@example.com")));
    assert!(!h.app.settings_open);
}

#[test]
fn calendar_merges_matching_invites_and_preserves_each_source() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let start=chrono::Local::now().timestamp()+600;
    let event=crate::mail::Meeting{ all_day:false, end:None,title:"产品评审".into(),start,at:String::new(),uid:"shared-review".into()};
    h.app.calendar.events=vec![event.clone()];h.app.calendar.view.events=vec![event.clone()];
    for account in ["work@example.com","personal@example.com"] {
        h.app.mail.recent.push(crate::mail::Item {received_unix: None, account:account.into(),from:"项目组".into(),subject:"会议邀请".into(),html: String::new().into(), summary:String::new().into(),links:vec![],code:None,time:String::new(),uid:9,meeting:Some(event.clone())});
    }
    let day=chrono::DateTime::from_timestamp(start,0).unwrap().with_timezone(&chrono::Local).date_naive();
    for provider in crate::calendar::Provider::ALL {
        h.app.calendar.config=Some(crate::calendar::Config{provider,username:"demo".into(),href:None,server:None});
        let entries=h.app.calendar_entries(day);
        assert_eq!(entries.len(),1,"同一场会议只占一行");
        assert_eq!(entries[0].sources.len(),3,"工作日历及两个邮箱应保留各自来源");
        assert!(matches!(entries[0].sources[0],crate::calendar::ui::Source::Calendar(Some(p)) if p==provider));
    }
    h.app.calendar.events.clear();h.app.calendar.view.events.clear();
    for (i,item) in h.app.mail.recent.iter_mut().enumerate(){let event=item.meeting.as_mut().unwrap();event.uid.clear();event.title=format!("不同会议 {i}");}
    assert_eq!(h.app.calendar_entries(day).len(),2,"缺少会议编号时不能只按相同时间误合并");
}

#[test]
fn calendar_dates_are_readable_and_source_labels_do_not_take_a_second_line() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let day=chrono::NaiveDate::from_ymd_opt(2026,11,15).unwrap();
    let event=crate::mail::Meeting{ all_day:false, end:None,title:"产品评审".into(),start:day.and_hms_opt(14,0,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap().timestamp(),at:"11-15 14:00".into(),uid:"readable-calendar".into()};
    h.app.calendar.config=Some(crate::calendar::Config{provider:crate::calendar::Provider::Feishu,username:"demo".into(),href:None,server:None});
    h.app.calendar.view.events=vec![event.clone()];
    h.app.mail.recent.push(crate::mail::Item {received_unix: None, account:"work@example.com".into(),from:"项目组".into(),subject:"会议邀请".into(),html: String::new().into(), summary:String::new().into(),links:vec![],code:None,time:String::new(),uid:1,meeting:Some(event.clone())});
    h.app.open_calendar(Some(event));h.hover(H::local(300.0,40.0),1.0);
    let date=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()=="15"=>Some(t),_=>None}).expect("月历应显示日期");
    assert!(date.galley.job.sections[0].format.font_id.size>=10.0,"日期数字需要明显放大");
    let date_rect=|label:&str|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect()),_=>None}).expect("日期应可见");
    let (ten,eleven,seventeen)=(date_rect("10"),date_rect("11"),date_rect("17"));
    assert!(eleven.left()-ten.right()>=6.0,"相邻日期需要横向留白：{ten:?} {eleven:?}");
    assert!(seventeen.top()-ten.bottom()>=7.0,"相邻周需要纵向留白：{ten:?} {seventeen:?}");
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="飞书" || t.galley.text().starts_with("邮件 · "))),"来源应使用标题前的 Logo，详细文字保留在悬停提示中");
}

#[test]
fn calendar_selected_schedule_expands_in_place_with_two_rows() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let day=chrono::NaiveDate::from_ymd_opt(2026,10,28).unwrap();
    let start=day.and_hms_opt(10,0,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap().timestamp();
    h.app.calendar.view.day=day;h.app.calendar.view.month=crate::calendar::month_start(day);h.app.calendar.view.loaded=Some(h.app.calendar.view.month);
    h.app.calendar.view.events=(0..7).map(|i|crate::mail::Meeting{ all_day:false,title:format!("会议 {i}"),start:start+i*3600,end:Some(start+i*3600+2700),at:String::new(),uid:format!("layout-{i}")}).collect();
    h.app.open_calendar(None);h.hover(H::local(300.0,40.0),1.0);
    fn find<'a>(shapes:&'a [egui::epaint::ClippedShape],label:&str)->&'a egui::epaint::TextShape {
        shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==label=>Some(t),_=>None}).unwrap()
    }
    let size=|shapes:&[egui::epaint::ClippedShape],label:&str|find(shapes,label).galley.job.sections[0].format.font_id.size;
    assert!((size(&h.shapes,"10:00")-14.0).abs()<0.01,"展开卡片的开始时间与名称同字号");
    assert!((size(&h.shapes,"会议 0")-14.0).abs()<0.01);
    assert!(size(&h.shapes,"会议 0")>size(&h.shapes,"会议 1"),"未选中行保持紧凑");
    assert!(find(&h.shapes,"10:45 结束").visual_bounding_rect().bottom()<find(&h.shapes,"会议 1").visual_bounding_rect().top(),"结束时间在下一行");
    assert!(find(&h.shapes,"会议 4").visual_bounding_rect().bottom()<H::local(0.0,205.0).y);
    h.hover(H::local(300.0,194.0),0.25);h.wheel(-40.0);
    assert_eq!(h.app.calendar.view.first,1,"列表底部也应滚动日程");assert_eq!(h.app.page,7);
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="会议 0")),"滚动不应把卡片置顶");
    h.click(H::local(260.0,138.0));h.hover(H::local(300.0,194.0),0.2);
    assert!((size(&h.shapes,"15:00")-14.0).abs()<0.01,"点击的卡片在原位置放大");
    assert!(size(&h.shapes,"会议 5")>size(&h.shapes,"会议 1"),"选中卡片不应置顶");
    h.wheel(-40.0);
    assert!((size(&h.shapes,"15:00")-14.0).abs()<0.01,"选中后滚动仍保持展开");
    h.app.open_calendar(Some(h.app.calendar.view.events[6].clone()));h.hover(H::local(300.0,40.0),0.3);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="会议 6")),"提醒定位的末条日程应完整可见");
}

#[test]
fn calendar_month_navigation_and_today_select_the_right_day() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.open_calendar(None);
    h.app.select_calendar_day(chrono::NaiveDate::from_ymd_opt(2026,12,31).unwrap());
    h.hover(H::local(300.0,60.0),1.0);
    h.click(H::local(129.0,80.0));
    assert_eq!(h.app.calendar.view.month,chrono::NaiveDate::from_ymd_opt(2027,1,1).unwrap());
    let point=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()=="15"=>Some(t.visual_bounding_rect().center()),_=>None}).expect("日期应显示在岛上");
    h.click(point);
    assert_eq!(h.app.calendar.view.day,chrono::NaiveDate::from_ymd_opt(2027,1,15).unwrap());
    h.click(H::local(67.0,18.0));
    assert_eq!(h.app.calendar.view.day,chrono::NaiveDate::from_ymd_opt(2026,10,28).unwrap());
    assert_eq!(h.app.page,7);assert!(!h.app.settings_open);
}

#[test]
fn calendar_month_results_preserve_reminders_and_ignore_old_requests() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.calendar.config=Some(crate::calendar::Config{provider:crate::calendar::Provider::Feishu,username:"demo".into(),href:None,server:None});
    let upcoming=crate::mail::Meeting{ all_day:false, end:None,title:"明天会议".into(),start:chrono::Local::now().timestamp()+3600,at:String::new(),uid:"upcoming".into()};
    h.app.calendar.events=vec![upcoming.clone()];
    let month=chrono::NaiveDate::from_ymd_opt(2024,2,1).unwrap();
    h.app.select_calendar_day(month);
    let config=h.app.calendar.config.clone().unwrap();
    h.app.calendar_event(crate::calendar::Event::MonthLoaded{config:config.clone(),month,result:Ok(vec![])});
    assert_eq!(h.app.calendar.events,vec![upcoming]);
    assert_eq!(h.app.calendar.view.loaded,Some(month));
    let next=chrono::NaiveDate::from_ymd_opt(2024,3,1).unwrap();h.app.select_calendar_day(next);
    h.app.calendar_event(crate::calendar::Event::MonthLoaded{config:config.clone(),month,result:Ok(vec![])});
    assert!(h.app.calendar.view.loading,"旧请求不能结束新月份的加载");
    let mut other=config.clone();other.username="another-account".into();
    h.app.calendar_event(crate::calendar::Event::MonthLoaded{config:other,month:next,result:Ok(vec![])});
    assert!(h.app.calendar.view.loading,"其他账号的旧结果应被忽略");
    h.app.calendar_event(crate::calendar::Event::MonthLoaded{config,month:next,result:Err("连接暂不可用".into())});
    assert!(!h.app.calendar.view.loading);assert!(!h.app.calendar.view.error.is_empty());
}

#[test]
fn calendar_island_collapses_and_reopens_on_the_same_page() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.open_calendar(None);h.hover(H::local(300.0,60.0),1.0);
    assert_eq!(h.app.page,7);assert!(h.app.open);
    assert!(h.hit.bottom()>=crate::app::CALENDAR_H,"月历最后一行必须位于可操作区域内");
    h.hover(Pos2::new(600.0,WIN_H+30.0),5.0);assert!(!h.app.open);
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);assert!(h.app.open);
    assert_eq!(h.app.page,7);assert!(!h.app.settings_open);
}

#[test]
#[ignore="按需导出岛内日程的实际渲染画面"]
fn calendar_layout_gallery() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_CALENDAR_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    for mode in ["agenda","dingtalk","wecom","icloud","exchange","empty","loading","failed","disconnected","local-editor","local","local-all-day","mixed"] {
        let mut h=H::new();
        let day=chrono::NaiveDate::from_ymd_opt(2026,11,2).unwrap();
        h.app.calendar.view.day=day;h.app.calendar.view.month=crate::calendar::month_start(day);
        h.app.open_calendar(None);
        let provider=match mode{"dingtalk"=>crate::calendar::Provider::DingTalk,"wecom"=>crate::calendar::Provider::WeCom,"icloud"=>crate::calendar::Provider::ICloud,"exchange"=>crate::calendar::Provider::Exchange,_=>crate::calendar::Provider::Feishu};
        if !["disconnected","local-editor","local","local-all-day"].contains(&mode){h.app.calendar.config=Some(crate::calendar::Config{provider,username:"demo".into(),href:None,server:None});h.app.calendar.connected=true;}
        h.app.calendar.view.loaded=Some(crate::calendar::month_start(day));
        if ["agenda","dingtalk","wecom","icloud","exchange","mixed"].contains(&mode) {
            for (i,(hour,minute,title)) in [(14,30,"产品方案评审"),(16,0,"下一阶段的设计方案与开发排期对齐"),(17,0,"项目进度同步")].into_iter().enumerate(){
                let at=day.and_hms_opt(hour,minute,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap();
                h.app.calendar.view.events.push(crate::mail::Meeting{ all_day:false,end:Some(at.timestamp()+2700),title:title.into(),start:at.timestamp(),at:at.format("%m-%d %H:%M").to_string(),uid:format!("sample-{i}")});
            }
            for d in [4,9,12,18,20,25] {let date=chrono::NaiveDate::from_ymd_opt(2026,11,d).unwrap();h.app.calendar.view.events.push(crate::mail::Meeting{ all_day:false, end:None,title:"项目安排".into(),start:date.and_hms_opt(10,0,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap().timestamp(),at:String::new(),uid:format!("other-{d}")});}
            for (i,(hour,title,account)) in [(15,"客户沟通：合同条款确认","work@example.com"),(16,"交付验收会议","team@example.com")].into_iter().enumerate(){
                let at=day.and_hms_opt(hour,30,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap();
                h.app.mail.recent.push(crate::mail::Item {received_unix: None, account:account.into(),from:"项目组".into(),subject:"会议邀请".into(),html: String::new().into(), summary:String::new().into(),links:vec![],code:None,time:String::new(),uid:i as u32,
                    meeting:Some(crate::mail::Meeting{ all_day:false, end:None,title:title.into(),start:at.timestamp(),at:at.format("%m-%d %H:%M").to_string(),uid:format!("mail-{i}")})});
            }
            h.app.mail.recent.push(crate::mail::Item {received_unix: None, account:"work@example.com".into(),from:"项目组".into(),subject:"会议邀请".into(),html: String::new().into(), summary:String::new().into(),links:vec![],code:None,time:String::new(),uid:3,meeting:Some(h.app.calendar.view.events[1].clone())});
        }
        if ["local","local-all-day","mixed"].contains(&mode){
            let mut draft=crate::calendar::local::Draft::new(day);draft.title="散步与读书".into();draft.start_time="15:00".into();draft.end_time="16:00".into();
            draft.all_day=mode!="local";h.app.calendar.local.push(draft.entry().unwrap());
        }
        if mode=="mixed" {
            let start=day.and_hms_opt(9,0,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap().timestamp();
            h.app.calendar.view.events.push(crate::mail::Meeting{all_day:false,title:"早上的会议".into(),start,end:Some(start+3600),at:String::new(),uid:"past".into()});
        }
        if mode=="local-editor"{h.app.open_calendar_editor(None);}
        if mode=="loading"{h.app.calendar.view.loaded=None;h.app.calendar.view.loading=true;}
        if mode=="failed"{h.app.calendar.view.loaded=None;h.app.calendar.view.error="连不上服务器，检查网络".into();}
        let mut raster=crate::preview_raster::Raster::new(1280,600);
        for f in 0..90 {
            let t=f as f64/60.0;
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(WIN_W,WIN_H))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
            let env=Env{t,today:"2026-11-02".into(),date:"11月2日 周一".into(),clock:"14:15:00".into(),battery:Some(76),charging:true,cursor:Some(H::local(300.0,40.0)),..Default::default()};
            let out=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{let win=ui.max_rect();h.app.frame(ui,win,&env);});});
            raster.update_textures(&out.textures_delta);
            if f==89 {
                if mode=="local-all-day" {
                    assert!(out.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(t) if t.galley.text()=="散步与读书")),"只有全天日程时也必须显示普通日程行");
                }
                raster.buf.fill([0.035,0.055,0.09,1.0]);raster.draw(&h.ctx.tessellate(out.shapes,out.pixels_per_point),out.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);let image=image::RgbaImage::from_raw(1280,600,rgba).unwrap();
                image::imageops::crop_imm(&image,180,0,920,510).to_image().save(dest.join(format!("calendar-{mode}.png"))).unwrap();
            }
            raster.free_textures(&out.textures_delta);
        }
    }
}

#[test]
fn calendar_reminder_fires_once_and_dedupes_with_mail() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.app.mail.accounts=vec![crate::mail::Account {custom: None, email:"a@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
    h.app.calendar.config=Some(crate::calendar::Config{provider:crate::calendar::Provider::Feishu,username:"demo".into(),href:None,server:None});
    h.app.settings.calendar_prompt = false;
    let start = chrono::Local::now().timestamp() + 600;
    let at = "10-30 10:00".to_string();
    let meeting = |title: &str| crate::mail::Meeting { all_day:false, end:None, title: title.into(), start, at: at.clone(), uid: "evt-shared-1".into() };
    // 邮件里先来一份，日历再来同一个 UID + DTSTART：只弹一次。
    h.app.mail_event(crate::mail::Event::Mail {
        item: crate::mail::Item {
            received_unix: None, account: "a@qq.com".into(),
            from: "腾讯企业邮".into(),
            subject: "周会".into(),
            time: "09:41".into(),
            code: None,
            html: String::new().into(), summary: String::new().into(),
            links: vec![],
            meeting: Some(meeting("周会")),
            uid: 7,
        },
        new: true,
    });
    h.app.calendar_event(crate::calendar::Event::Synced {
        config:h.app.calendar.config.clone().unwrap(),
        events: vec![meeting("周会（飞书）")],
        at: "10-30 09:00".into(),
    });
    h.frame(vec![]);
    h.frame(vec![]);
    let hits = h.app.history.iter().filter(|a| a.title == "会议提醒").count();
    assert_eq!(hits, 1, "同一场会只提醒一次");
    assert_eq!(h.app.history.iter().find(|a|a.title=="会议提醒").unwrap().page,7,"会议提醒应指向岛内日程页");
    // 再同步一次、再过几帧，也不重复。
    h.app.calendar_event(crate::calendar::Event::Synced { config:h.app.calendar.config.clone().unwrap(),events: vec![meeting("周会（飞书）")], at: "10-30 09:05".into() });
    h.frame(vec![]);
    assert_eq!(h.app.history.iter().filter(|a| a.title == "会议提醒").count(), 1);
}

#[test]
fn calendar_setup_does_not_overlay_music() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.app.settings.calendar_prompt = true;
    h.app.calendar.config = None;
    h.hover(Pos2::new(WIN_W / 2.0, 12.0), 1.0);
    h.hover(H::local(300.0, 60.0), 0.6);
    h.click(H::local(40.0, 16.0));
    assert!(!h.app.settings_open, "音乐页不应出现日历接入悬浮入口");
    h.app.open_settings_category(3);
    assert!(h.app.settings_open, "仍可从设置访问日程");
    assert_eq!(h.app.settings_tab, 6, "日程位于通知、邮件与日程分组");
}

#[test]
fn meeting_notice_prioritizes_the_start_time_and_keeps_actions_aligned(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for ppp in [1.0,1.25,1.5,2.0]{
        let mut h=H::new();h.ctx.set_pixels_per_point(ppp);h.app.settings.pet=false;
        let start=chrono::Local::now()+chrono::Duration::minutes(10);
        h.app.meeting_alert(crate::mail::Meeting{ all_day:false,title:"产品评审".into(),start:start.timestamp(),end:None,at:start.format("%m-%d %H:%M").to_string(),uid:"meeting-layout".into()},"邮件 · product-and-design-team@company-example.com".into(),7);
        for _ in 0..90{h.frame(vec![]);}
        h.click_notice_badge();
        assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().starts_with("邮件 · "))),"软件名称仅用于识别图标，不占用提醒正文");
        let clock=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==start.format("%H:%M").to_string()=>Some((t.visual_bounding_rect(),s.clip_rect,t.pos.x,t.galley.job.sections[0].format.font_id.size)),_=>None}).unwrap();
        let countdown=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text().ends_with("分钟后")=>Some((t.visual_bounding_rect(),t.galley.job.sections[0].format.color)),_=>None}).unwrap();
        assert!(clock.0.right()+12.0<=countdown.0.left(),"缩放 {ppp}：开始时刻与倒计时应保持间距");
        assert!(clock.1.contains_rect(clock.0),"大号时刻不能被提醒条裁切");
        assert_eq!(countdown.1,egui::Color32::WHITE,"倒计时使用白色，强调色保留给来源 Logo");
        let title=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()=="产品评审"=>Some((t.pos.x,t.galley.job.sections[0].format.font_id.size)),_=>None}).unwrap();
        assert!(clock.3>=28.0 && clock.3>title.1,"开始时刻应比事项名称更醒目");
        assert!((title.0-clock.2).abs()<0.5,"开始时刻与标题应沿同一列对齐");
        let date=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text().contains('月') && t.galley.text().contains('日')=>Some((t.visual_bounding_rect(),t.pos.x)),_=>None}).unwrap();
        let button=h.ctx.read_response(egui::Id::new(("card",true,"meeting-action"))).unwrap().rect;
        assert!((date.0.center().y-button.center().y).abs()<0.5,"日期与查看按钮应垂直居中对齐");
        assert!(((date.1-clock.1.left())-(clock.1.right()-button.right())).abs()<0.5,"卡片左右应保留相同边距");
    }
}

#[test]
fn meeting_long_titles_scroll_to_the_end_while_short_titles_stay_still(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for title in ["简短评审","产品设计方案评审及下一阶段开发排期与交付验收安排确认"]{
        let mut h=H::new();h.app.settings.pet=false;
        h.app.meeting_alert(crate::mail::Meeting{ all_day:false,title:title.into(),start:chrono::Local::now().timestamp()+600,end:None,at:String::new(),uid:title.into()},"飞书".into(),7);
        for _ in 0..45{h.frame(vec![]);}
        let title_shape=|h:&H|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==title=>Some((t.pos.x,t.visual_bounding_rect(),s.clip_rect)),_=>None}).expect("标题应保留完整内容，不应提前省略");
        let initial=title_shape(&h);let height=h.hit.height();
        h.t+=0.2;h.frame(vec![]);assert!((title_shape(&h).0-initial.0).abs()<0.5,"开始滚动前应停留片刻");
        h.t+=5.0;h.frame(vec![]);let middle=title_shape(&h);
        h.t+=4.0;h.frame(vec![]);let end=title_shape(&h);
        if title=="简短评审" {assert!((end.0-initial.0).abs()<0.5,"短标题保持静止");}
        else {
            assert!(middle.0<initial.0-20.0,"长标题应向左滚动显示后半段");
            assert!((initial.2.center()-end.2.center()).length()<0.1 && (initial.2.size()-end.2.size()).length()<0.1,"滚动不应改变标题的显示范围");
            assert!(end.1.right()<=end.2.right()+1.0 && end.1.right()>=end.2.right()-5.0,"应能看到标题末尾");
            h.t+=0.6;h.frame(vec![]);assert!((title_shape(&h).0-end.0).abs()<0.5,"滚到末尾后应停留，不能继续滑出");
        }
        assert!((h.hit.height()-height).abs()<0.5,"滚动不应改变提醒高度");
    }
}

#[test]
fn calendar_notice_overrides_island_lyrics_and_restores_the_current_line(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=false;h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:180.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",180.0,crate::lyrics::parse_lrc("[00:00]首句歌词\n[00:20]后续歌词"));
    for _ in 0..60{h.frame(vec![]);}
    assert!(h.app.compact_notices());
    h.app.meeting_alert(crate::mail::Meeting{ all_day:false,title:"优先会议".into(),start:chrono::Local::now().timestamp()+600,end:None,at:String::new(),uid:"priority".into()},"飞书".into(),7);
    for _ in 0..90{h.frame(vec![]);}
    assert!(!h.app.compact_notices(),"日程提醒不应缩成歌词旁的小图标");
    assert!(h.hit.height()>130.0,"应直接展示完整 A 版提醒");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="优先会议")));
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="查看")));
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="首句歌词")));
    let active=h.app.notice.as_ref().unwrap();h.t=active.at+active.duration-1.0/30.0;h.media.position=25.0;
    for _ in 0..90{
        h.frame(vec![]);
        if h.app.notice.is_none(){assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="Track" || t.galley.text()=="Artist")),"恢复歌词时不能闪出展开的播放器");}
    }
    assert!(h.app.notice.is_none() && h.app.settings.island_lyrics && h.app.compact_notices());
    assert!(h.hit.height()<35.0,"提醒结束后应恢复歌词条高度");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="后续歌词")),"应恢复当前进度的歌词");
}

#[test]
fn calendar_overnight_appears_on_both_days_but_not_after_end() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let first=chrono::NaiveDate::from_ymd_opt(2026,11,3).unwrap();
    h.app.calendar.view.loaded=Some(crate::calendar::month_start(first));
    let start=first.and_hms_opt(23,30,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap().timestamp();
    h.app.calendar.view.events.push(crate::mail::Meeting{ all_day:false,title:"跨夜".into(),start,end:Some(start+3600),at:String::new(),uid:"overnight".into()});
    assert_eq!(h.app.calendar_entries(first).len(),1);
    assert_eq!(h.app.calendar_entries(first.succ_opt().unwrap()).len(),1);
    assert_eq!(h.app.calendar_entries(first+chrono::Duration::days(2)).len(),0);
}

#[test]
fn calendar_month_refresh_removes_cancelled_cached_event() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let day=chrono::NaiveDate::from_ymd_opt(2026,11,3).unwrap();
    let start=day.and_hms_opt(16,0,0).unwrap().and_local_timezone(chrono::Local).earliest().unwrap().timestamp();
    h.app.calendar.events.push(crate::mail::Meeting{all_day:false,title:"已取消".into(),start,end:None,at:String::new(),uid:"removed".into()});
    h.app.calendar.view.loaded=Some(crate::calendar::month_start(day));
    h.app.calendar.view.events.clear();
    assert!(h.app.calendar_entries(day).is_empty(),"月份同步后不能让提醒缓存恢复已取消日程");
}

#[test]
fn calendar_sync_result_does_not_enqueue_another_sync() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let config=crate::calendar::Config{provider:crate::calendar::Provider::Feishu,username:"test".into(),href:None,server:None};
    h.app.calendar.config=Some(config.clone());h.app.open_calendar(None);h.app.pending.clear();
    h.app.calendar.view.loading=false;
    h.app.calendar_event(crate::calendar::Event::Synced{config,events:vec![],at:"12:00".into()});
    assert!(!h.app.pending.iter().any(|cmd|matches!(cmd,crate::app::Cmd::CalendarMonth(_))),"同步结果不能触发下一轮同步，否则后台会持续请求");
}

#[test]
fn calendar_add_button_does_not_overlap_the_battery_ring(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for seconds in [false,true] {
    for (index,(day_number,date,period,twelve,battery,scale)) in [
        (6,"11月6日 周五",1,false,Some(76),2.0),
        (28,"11月28日 周六",1,true,Some(76),1.25),
        (28,"11月28日 周六",2,false,Some(76),1.5),
        (6,"11月6日 周五",0,false,Some(76),1.0),
        (28,"11月28日 周六",0,true,Some(76),2.0),
        (6,"11月6日 周五",1,true,None,1.25),
        (28,"11月28日 周六",2,true,None,1.5),
        (28,"11月28日 周六",0,false,None,1.0),
    ].into_iter().enumerate(){
    let mut h=H::new();
    h.app.settings.skin=crate::model::Skin::Cream;h.app.settings.island_shape=crate::model::IslandShape::Pill;
    h.app.settings.clock_period=period;h.app.settings.clock_12h=twelve;h.app.settings.pet=false;
    h.app.settings.clock_seconds=seconds;
    let day=chrono::NaiveDate::from_ymd_opt(2026,11,day_number).unwrap();
    h.app.calendar.view.day=day;h.app.calendar.view.month=crate::calendar::month_start(day);h.app.calendar.view.loaded=Some(h.app.calendar.view.month);
    h.app.show_page(7);
    let mut raster=feature_raster();let mut shapes=Vec::new();
    for _ in 0..90 {
        h.t+=1.0/60.0;
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::app::CANVAS_W,WIN_H))),time:Some(h.t),focused:true,
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()})).collect(),..Default::default()};
        let env=Env{t:h.t,clock:"17:43:00".into(),date:date.into(),today:day.to_string(),cursor:Some(Pos2::new(WIN_W/2.0,12.0)),battery,charging:true,..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H)),&env);});});
        raster.update_textures(&output.textures_delta);shapes=output.shapes;
        raster.free_textures(&output.textures_delta);
    }
    if let Some(path)=std::env::var_os("DAOZAI_CALENDAR_HEADER_PREVIEW").filter(|_|index==0&&!seconds) {
        raster.buf.fill([0.15,0.15,0.15,1.0]);raster.draw(&h.ctx.tessellate(shapes.clone(),2.0),2.0);
        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
        let image=image::RgbaImage::from_raw((crate::app::CANVAS_W*2.0) as u32,(WIN_H*2.0) as u32,rgba).unwrap();
        image::imageops::crop_imm(&image,200,0,880,470).to_image().save(path).unwrap();
    }
    let text=shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Text(text) if text.galley.text()=="＋ 添加日程"=>Some(text.visual_bounding_rect()),_=>None}).unwrap();
    let button=shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Rect(rect) if rect.rect.contains(text.center()) && (rect.rect.width()-54.0).abs()<0.1 && (rect.rect.height()-18.0).abs()<0.1=>Some(rect.rect),_=>None}).unwrap();
    if battery.is_some(){
        let ring=shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Circle(circle) if (circle.radius-8.2*crate::bar_layout::BATTERY_SCALE).abs()<0.1 && circle.center.y<crate::app::STATUS_H=>Some(Rect::from_center_size(circle.center,Vec2::splat(circle.radius*2.0+circle.stroke.width))),_=>None}).unwrap();
        assert!(!button.expand(3.0).intersects(ring),"日程添加按钮与电量圆环应留有间距：按钮 {button:?}，圆环 {ring:?}");
    }
    let clock=h.app.settings.clock_text("17:43:00");
    for text in shapes.iter().filter_map(|shape|match &shape.shape{egui::Shape::Text(text) if text.galley.text()==date || clock.split_whitespace().any(|part|part==text.galley.text())=>Some(text),_=>None}){
        let bounds=text.visual_bounding_rect();
        assert!(!button.expand(3.0).intersects(bounds),"添加日程按钮不能覆盖日期或时间：{}",text.galley.text());
        assert!(bounds.right()<h.app.content_origin(Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H))).x+355.0,"日期与时间应留出右侧播放和设置按钮的位置");
    }
    }
    }
}

#[test]
fn calendar_signature_matches_without_copying(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let h=H::new();
    let known=h.app.calendar_configs();
    assert!(h.app.calendar_configs_match(&known));
    assert_eq!(h.app.calendar_configs_match(&[]),known.is_empty());
}
