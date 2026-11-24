//! 音乐页、歌词与音量。
use super::*;

#[test]
fn web_video_is_hidden_without_changing_music_preferences(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.opts.fetch_lyrics=true;h.app.show_lyrics=true;
    h.app.ly.key="Previous Song|Artist|120".into();
    h.app.ly.lyrics=Some(crate::lyrics::parse_lrc("[00:01.00]previous song"));
    h.app.ly.error=Some(crate::lyrics::FetchError::Unavailable("503".into()));
    let (tx,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    h.media=crate::smtc::NowPlaying{title:"小唐的Token的抖音 - 抖音".into(),playing:true,duration:17.0,..Default::default()};
    h.frame(vec![]);
    assert!(h.app.media.title.is_empty() && !h.app.media.playing,"视频标题和播放状态不应进入岛内音乐页面");
    assert!(h.app.ly.key.is_empty(),"视频不应建立歌曲取词任务");
    assert!(h.app.ly.lyrics.is_none() && h.app.ly.error.is_none(),"切到视频应清除上一首歌词和歌词错误");
    assert!(tx.send(Ok((crate::lyrics::parse_lrc("[00:01.00]late song"),false))).is_err(),"视频不能接收旧歌曲迟到的查询结果");
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.click(H::local(74.0,118.0));
    assert!(h.app.show_lyrics,"视频页面的标签不应切换用户的歌曲歌词偏好");
    h.click(H::local(210.5,118.0));
    assert!(!h.cmds.iter().any(|cmd|matches!(cmd,crate::island::Cmd::Media(_))),"音乐页不应控制被隐藏的视频");
    assert!(!matches!(h.app.pet_state(),crate::island::extras::PetState::Music),"视频不应触发听歌表情");
    h.app.opts.fetch_lyrics=false;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),duration:120.0,..Default::default()};
    h.hover(Pos2::new(WIN_W/2.0,12.0),0.5);
    h.click(H::local(74.0,118.0));
    assert!(!h.app.show_lyrics,"回到歌曲时应恢复原来的词按钮");
}

#[test]
fn web_video_does_not_override_the_minimal_island(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.set_minimal(true);
    h.media=crate::smtc::NowPlaying{title:"小唐的Token的抖音 - 抖音".into(),playing:true,duration:17.0,..Default::default()};
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()<20.0,"视频不能触发歌曲专用的收起态");
}

#[test]
fn minimal_mode_stays_a_line_with_music_and_after_media_changes(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.set_minimal(true);
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()<20.0,"播放音乐时仍应遵守极简开关");
    assert!(h.hit.height()<=crate::island::BAR_H_MIN+0.1,"鼠标命中范围也应收回细线");
    h.media.title="Another Track".into();h.media.playing=false;
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()<20.0,"暂停或切歌不能覆盖极简开关");
    h.media=Default::default();
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()<20.0,"没有媒体时恢复用户选择的极简状态");
}

#[test]
fn toggling_minimal_changes_the_island_while_media_is_present(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    for _ in 0..90 {h.frame(vec![]);}assert!(h.region.height()>30.0);
    h.app.set_minimal(true);for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()<20.0,"开启极简后应收成细线");
    h.app.set_minimal(false);for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()>30.0,"关闭极简后应恢复普通岛体");
}

#[test]
fn delayed_media_samples_keep_the_live_position_and_follow_pause_and_seek(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:9.2,duration:120.0,playing:true,sampled_at:Some(std::time::Instant::now()-std::time::Duration::from_millis(600)),..Default::default()};
    h.frame(vec![]);
    assert!(h.app.media.position>=9.79,"媒体位置必须包含送到界面之前经过的时间");
    h.media.playing=false;h.media.position=9.4;h.frame(vec![]);
    assert_eq!(h.app.media.position,9.4,"暂停不能继续外推");
    h.media.playing=true;h.media.position=4.0;h.media.sampled_at=Some(std::time::Instant::now());h.frame(vec![]);
    assert!((4.0..4.2).contains(&h.app.media.position),"跳转后应使用新的播放位置");
    h.media.position=119.9;h.media.sampled_at=Some(std::time::Instant::now()-std::time::Duration::from_secs(1));h.frame(vec![]);
    assert_eq!(h.app.media.position,120.0,"外推不能超过歌曲结尾");
}

#[test]
fn music_without_a_timeline_does_not_invent_elapsed_time(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"QQMusic.exe".into(),title:"歌曲".into(),playing:true,playback_type:crate::smtc::MediaKind::Music,..Default::default()};
    for _ in 0..240{h.frame(vec![]);}
    assert_eq!(h.app.media.title,"歌曲");assert_eq!(h.app.media.position,0.0);assert_eq!(h.app.media.duration,0.0);
}

#[test]
fn kugou_position_advances_without_duration_and_respects_pause_and_seek(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"kugou".into(),title:"Track".into(),artist:"Artist".into(),position:42.0,duration:0.0,playing:true,sampled_at:Some(std::time::Instant::now()-std::time::Duration::from_millis(600)),..Default::default()};
    h.frame(vec![]);
    assert!(h.app.media.position>=42.59,"未知总时长不能阻止真实进度外推");
    h.media.playing=false;h.frame(vec![]);assert_eq!(h.app.media.position,42.0);
    h.media.playing=true;h.media.position=12.0;h.media.sampled_at=Some(std::time::Instant::now());h.frame(vec![]);
    assert!((12.0..12.2).contains(&h.app.media.position),"拖动后必须跟随新的实际位置");
}

#[test]
fn music_volume_popup_opens_and_mute_restores_the_selected_players_level(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"QQMusic.exe".into(),title:"Track".into(),artist:"Artist".into(),duration:120.0,volume:Some((60,false)),..Default::default()};
    h.app.volume_ui=Some((45,false));
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.click(H::local(367.0,118.0));
    assert!(h.app.music_volume_open);
    assert!(h.cmds.is_empty(),"打开浮条不应改变音量或静音");
    h.hover(H::local(367.0,92.0),0.2);h.click(H::local(367.0,92.0));
    assert!(h.cmds.contains(&crate::island::Cmd::MusicVolume(crate::smtc::Transport::SetMute(true),"QQMusic.exe".into())));
    assert!(!h.cmds.contains(&crate::island::Cmd::ToggleMute));
    assert_eq!(h.app.music_volume_ui,Some((60,true)));
    assert_eq!(h.app.volume_ui,Some((45,false)));
    h.click(H::local(367.0,92.0));
    assert_eq!(h.app.music_volume_ui,Some((60,false)),"恢复时沿用静音前的音量");
    assert!(h.cmds.contains(&crate::island::Cmd::MusicVolume(crate::smtc::Transport::SetMute(false),"QQMusic.exe".into())));
}

#[test]
fn music_volume_popup_slider_unmutes_and_scrolls_without_changing_pages(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"QQMusic.exe".into(),title:"Track".into(),artist:"Artist".into(),volume:Some((60,true)),..Default::default()};
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.click(H::local(367.0,118.0));h.hover(H::local(367.0,53.5),0.2);
    h.click(H::local(367.0,53.5));
    assert_eq!(h.app.music_volume_ui,Some((50,false)));
    assert!(h.cmds.contains(&crate::island::Cmd::MusicVolume(crate::smtc::Transport::SetVolume(50),"QQMusic.exe".into())));
    h.wheel(120.0);assert_eq!(h.app.music_volume_ui,Some((54,false)));assert_eq!(h.app.page,0);
    h.click(H::local(367.0,32.0));assert_eq!(h.app.music_volume_ui,Some((100,false)));
    h.click(H::local(367.0,75.0));assert_eq!(h.app.music_volume_ui,Some((0,false)));
    h.click(H::local(387.0,68.0));assert_eq!(h.app.page,0,"浮条遮挡部分不能穿透触发翻页");
    h.click(H::local(387.0,18.0));assert!(!h.app.settings_open,"浮条遮挡部分不能穿透打开设置");
}

#[test]
fn music_volume_popup_closes_on_escape_outside_click_source_change_and_collapse(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"QQMusic.exe".into(),title:"Track".into(),artist:"Artist".into(),volume:Some((60,false)),..Default::default()};
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.click(H::local(367.0,118.0));
    h.frame(vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    assert!(!h.app.music_volume_open);assert!(h.app.open,"Esc 只关闭浮条，保留音乐面板");
    h.click(H::local(367.0,118.0));h.click(H::local(180.0,45.0));assert!(!h.app.music_volume_open);
    h.click(H::local(367.0,118.0));h.media.source="cloudmusic.exe".into();h.frame(vec![]);assert!(!h.app.music_volume_open);
    h.click(H::local(367.0,118.0));h.app.show_page(1);assert!(!h.app.music_volume_open);
    h.app.show_page(0);h.hover(H::local(180.0,45.0),1.0);h.click(H::local(367.0,118.0));
    h.hover(Pos2::new(0.0,250.0),5.0);assert!(!h.app.music_volume_open);
}

#[test]
#[ignore="按需导出音乐音量浮条实际画面：DAOZAI_MUSIC_VOLUME_PREVIEW=目录"]
fn music_volume_popup_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_MUSIC_VOLUME_PREVIEW").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for (name,muted,lyrics) in [("volume",false,false),("muted",true,false),("lyrics",false,true)] {
        let mut h=H::new();h.ctx.set_pixels_per_point(2.0);h.app.settings.pet=false;h.app.show_lyrics=lyrics;
        h.media=crate::smtc::NowPlaying{source:"cloudmusic.exe".into(),title:"晴天".into(),artist:"周杰伦".into(),position:92.0,duration:269.0,volume:Some((62,muted)),..Default::default()};
        h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
        if lyrics{h.app.set_lyrics("晴天","周杰伦",269.0,crate::lyrics::parse_lrc("[00:00]午后的阳光落在窗边\n[01:30]风把回忆吹向远方\n[01:40]慢慢走过这段时光"));}
        h.click(H::local(367.0,118.0));h.hover(H::local(330.0,112.0),0.25);
        let ppp=h.ctx.pixels_per_point();let mut raster=crate::preview_raster::Raster::new((crate::island::CANVAS_W*ppp) as usize,(WIN_H*ppp) as usize);
        raster.update_textures(&egui::TexturesDelta{set:vec![(egui::TextureId::default(),egui::epaint::ImageDelta::full(h.ctx.fonts(|f|f.image()),egui::TextureOptions::LINEAR))],..Default::default()});
        raster.buf.fill([0.10,0.12,0.16,1.0]);raster.draw(&h.ctx.tessellate(h.shapes.clone(),ppp),ppp);
        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
        let image=image::RgbaImage::from_raw(raster.w as u32,raster.h as u32,rgba).unwrap();
        image::imageops::crop_imm(&image,((WIN_W-crate::island::PW)/2.0*ppp-12.0) as u32,0,((crate::island::PW+12.0)*ppp+12.0) as u32,((crate::island::PH+12.0)*ppp) as u32).to_image().save(dest.join(format!("{name}.png"))).unwrap();
    }
}

#[test]
fn music_volume_wheel_and_player_switch_keep_system_volume_untouched(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"QQMusic.exe".into(),title:"Track".into(),artist:"Artist".into(),volume:Some((60,true)),..Default::default()};
    h.app.volume_ui=Some((45,false));h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.hover(H::local(367.0,118.0),0.2);h.wheel(120.0);
    assert!(h.cmds.contains(&crate::island::Cmd::MusicVolume(crate::smtc::Transport::SetVolume(64),"QQMusic.exe".into())),"音量={:?} 命令={:?} 页={} 指针={:?}",h.app.music_volume_ui,h.cmds,h.app.page,h.cursor);
    assert_eq!(h.app.music_volume_ui,Some((64,false)));
    h.media.source="cloudmusic.exe".into();h.media.volume=Some((22,false));h.frame(vec![]);
    assert_eq!(h.app.music_volume_ui,Some((22,false)),"换播放器立即更新独立音量");
    assert_eq!(h.app.volume_ui,Some((45,false)));
    h.media.volume=None;h.cmds.clear();h.click(H::local(367.0,118.0));h.wheel(120.0);
    assert!(h.cmds.is_empty(),"没有应用会话时不可回退到系统音量");
}

#[test]
fn music_uses_system_artwork_and_clears_it_when_the_next_song_has_none(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    let image=std::sync::Arc::new(egui::ColorImage::new([32,32],egui::Color32::BLUE));
    h.media=crate::smtc::NowPlaying{title:"First".into(),artist:"Artist".into(),artwork:Some(image.clone()),..Default::default()};
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    let (cached,texture)=h.app.music_artwork.as_ref().expect("播放页应使用系统封面");
    assert!(std::sync::Arc::ptr_eq(cached,&image));
    let texture_id=texture.id();
    h.frame(vec![]);
    assert_eq!(h.app.music_artwork.as_ref().unwrap().1.id(),texture_id,"进度更新不应重复上传纹理");
    h.media=crate::smtc::NowPlaying{title:"Second".into(),artist:"Artist".into(),..Default::default()};
    h.frame(vec![]);
    assert!(h.app.music_artwork.is_none(),"缺少封面的下一首歌应恢复占位图");
}

#[test]
fn transient_lyric_failures_retry_but_missing_songs_do_not_loop(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (error,retry) in [(crate::lyrics::FetchError::Unavailable("503".into()),true),(crate::lyrics::FetchError::NotFound,false),(crate::lyrics::FetchError::IncompleteMetadata,false)] {
        let mut h=H::new();
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),duration:120.0,..Default::default()};
        h.app.opts.fetch_lyrics=true;h.app.show_lyrics=true;
        h.app.ly.key="Track|Artist|120".into();
        let (tx,rx)=std::sync::mpsc::channel();
        h.app.ly.rx=Some(rx);h.app.ly.loading=true;
        tx.send(Err(error.clone())).unwrap();
        h.frame(vec![]);
        assert_eq!(h.app.ly.error,Some(error));
        assert_eq!(h.app.ly.retry_at.is_some(),retry);
        assert!(!h.app.ly.loading);
        if let Some(at)=h.app.ly.retry_at {assert!((at-h.t-30.0).abs()<0.001,"网络恢复重试应有间隔");}
    }
}

#[test]
fn music_lyric_timing_follows_saved_delay_and_preserves_playback_position(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:9.8,duration:120.0,playing:true,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]第一句\n[00:05]当前歌词\n[00:10]下一句"));
    h.frame(vec![]);
    assert_eq!(h.app.settings.lyric_delay_ms,-350,"未设置时应保留原有提前 0.35 秒的预设");
    assert!((h.app.lyric_position()-10.15).abs()<0.001,"默认应提前 0.35 秒显示，且只应用一次");
    for (delay,position,expected) in [(500,10.2,Some("当前歌词")),(-500,9.8,Some("下一句")),(0,9.8,Some("当前歌词")),(500,0.2,None),(500,0.6,Some("第一句"))]{
        std::fs::write(crate::model::config_dir().join("settings.txt"),format!("lyric_delay_ms={delay}\nisland_lyrics=1\n")).unwrap();
        h.app.settings=crate::model::Settings::load();
        h.media.position=position;h.frame(vec![]);
        assert_eq!(h.app.island_lyric().map(|(_,line)|line),expected,"延时 {delay} ms，播放位置 {position}");
        assert!((h.app.media.position-position).abs()<0.001,"调整歌词不应改变播放进度");
        h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
        for text in ["第一句","当前歌词","下一句"]{
            let active=h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(t) if t.galley.text()==text && t.galley.job.sections[0].format.font_id.size==12.0));
            assert_eq!(active,expected==Some(text),"展开歌词也应使用相同的显示时间");
        }
        h.app.settings.save();
        assert_eq!(crate::model::Settings::load().lyric_delay_ms,delay,"重启后应保留时间调整");
    }
    for (input,expected) in [("invalid",-350),("999999",5000),("-999999",-5000),("",-350)]{
        std::fs::write(crate::model::config_dir().join("settings.txt"),format!("lyric_delay_ms={input}\n")).unwrap();
        assert_eq!(crate::model::Settings::load().lyric_delay_ms,expected);
    }
}

#[test]
fn music_lyric_timing_controls_match_focus_save_and_fit_the_small_settings_window(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings_tab=1;h.app.open_settings();h.ctx.set_embed_viewports(false);
    fn frame(h:&mut H,events:Vec<egui::Event>)->Vec<egui::epaint::ClippedShape>{
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,450.0))),
                viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.25),focused:Some(true),..Default::default()})).collect(),
                focused:true,events:events.clone(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let raw=egui::RawInput{viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(1.25),..Default::default()})).collect(),..Default::default()};
        let _=h.ctx.run(raw,|ctx|h.app.settings_ui(ctx,false));
        let child=output.lock().unwrap().take().unwrap();child.shapes
    }
    let text_rect=|shapes:&[egui::epaint::ClippedShape],text:&str|shapes.iter().find_map(|shape|match &shape.shape{
        egui::Shape::Text(t)if t.galley.text()==text=>Some(t.visual_bounding_rect()),_=>None,
    }).unwrap_or_else(||panic!("设置中缺少 {text}"));
    for _ in 0..3{frame(&mut h,vec![]);}
    let music=frame(&mut h,vec![]);
    text_rect(&music,"0.35 秒");
    assert_eq!(h.app.settings.lyric_delay_ms,-350,"仅打开设置不能将 0.35 秒舍入为 0.4 秒");
    for button in ["−","+"]{text_rect(&music,button);}
    h.app.settings_tab=2;
    for _ in 0..3{frame(&mut h,vec![]);}
    let focus=frame(&mut h,vec![]);
    for button in ["−","+"]{
        let a=text_rect(&music,button);let b=text_rect(&focus,button);
        assert!((a.left()-b.left()).abs()<0.1 && (a.width()-b.width()).abs()<0.1 && (a.height()-b.height()).abs()<0.1,"歌词和专注应共用同样的数值控件及右侧对齐");
    }
    h.app.settings_tab=1;
    for _ in 0..3{frame(&mut h,vec![]);}
    for (button,expected) in [("+",-450),("−",-350),("−",-250),("+",-350)]{
        let shapes=frame(&mut h,vec![]);
        let rect=text_rect(&shapes,button);
        assert!(Rect::from_min_max(egui::pos2(180.0,0.0),egui::pos2(600.0,450.0)).contains_rect(rect),"时间调整按钮应完整显示");
        let pos=rect.center();
        frame(&mut h,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        frame(&mut h,vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        assert_eq!(h.app.settings.lyric_delay_ms,expected);
        assert_eq!(crate::model::Settings::load().lyric_delay_ms,expected,"按钮调整后应自动保存");
        text_rect(&frame(&mut h,vec![]),&format!("{:.2} 秒",-expected as f64/1000.0));
    }
}

#[test]
fn lyric_short_gap_and_line_switch_keep_text_visible(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:2.0,playing:true,..Default::default()};
    h.app.set_lyrics("Track","Artist",0.0,crate::lyrics::parse_lrc("[00:01]上一句\n[00:05]下一句"));
    for _ in 0..60{h.frame(vec![]);}
    for (position,expected) in [(3.0,"上一句"),(4.64,"上一句"),(4.7,"下一句"),(4.72,"下一句"),(4.9,"下一句")]{
        h.media.position=position;h.frame(vec![]);
        let text=h.shapes.iter().find_map(|shape|match &shape.shape{
            egui::Shape::Text(text)if text.galley.text()==expected=>Some(text),_=>None,
        }).expect("短停顿及换句时必须持续显示歌词");
        assert!(text.opacity_factor>0.95,"换句不能淡出歌词，位置 {position}，透明度 {}",text.opacity_factor);
    }
}

#[test]
fn island_lyric_segments_follow_playback_pause_and_seek(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    let line="愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光";
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:6.0,duration:120.0,playing:true,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc(&format!("[00:05]{line}\n[00:15]下一句")));
    let shown=|h:&H|h.shapes.iter().find_map(|s|match &s.shape {egui::Shape::Text(t)=>Some(t.galley.text().to_owned()),_=>None}).unwrap();
    for _ in 0..90 {h.frame(vec![]);}let first=shown(&h);
    h.media.position=14.0;for _ in 0..30 {h.frame(vec![]);}let last=shown(&h);
    assert_ne!(first,last,"播放到后半句时应切换显示段");
    assert_eq!(format!("{first}{last}"),line,"前后显示段应覆盖完整歌词");
    h.media.playing=false;for _ in 0..120 {h.frame(vec![]);}
    assert!(h.app.island_lyric().is_none(),"暂停时应恢复常驻形态");
    h.media.playing=true;for _ in 0..30 {h.frame(vec![]);}assert_eq!(shown(&h),last,"继续播放时应恢复对应显示段");
    h.media.position=6.0;for _ in 0..30 {h.frame(vec![]);}assert_eq!(shown(&h),first,"拖回前半句应恢复对应显示段");
    h.media.position=16.0;for _ in 0..30 {h.frame(vec![]);}assert_eq!(shown(&h),"下一句");
}

#[test]
fn music_header_shortcut_returns_from_every_page_without_controlling_playback() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for playing in [true,false] {
        let mut h=H::new();h.app.settings.page_order=[5,2,0,1,3,11,4,6,7,8,9,10];
        h.media=crate::smtc::NowPlaying{source:"cloudmusic.exe".into(),title:"Track".into(),artist:"Artist".into(),playing,duration:120.0,..Default::default()};
        for page in 1..crate::island::PAGE_NAMES.len() {
            h.app.show_page(page);h.hover(H::local(300.0,60.0),0.8);h.cmds.clear();
            let music=h.ctx.read_response(egui::Id::new(("card",true,"header-music"))).expect("展开态状态行应显示音乐入口").rect;
            h.click(music.center());
            assert_eq!(h.app.page,0,"播放状态 {playing} 时应从页面 {page} 直达音乐页");
            assert!(h.app.open);assert!(!h.app.settings_open);assert_eq!(h.app.media.playing,playing);
            assert!(h.cmds.iter().all(|cmd|!matches!(cmd,crate::island::Cmd::Media(_)|crate::island::Cmd::MusicVolume(..))),"返回音乐页不能改变播放或音量");
        }
    }
}

#[test]
fn music_header_shortcut_keeps_hidden_music_page_disabled() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    h.app.set_page_visible(0,false);h.app.show_page(4);h.hover(H::local(300.0,60.0),0.8);
    let music=h.ctx.read_response(egui::Id::new(("card",true,"header-music"))).expect("音乐入口仍应可见").rect;
    h.click(music.center());
    assert_eq!(h.app.page,4);assert!(!h.app.settings.visible_pages[0]);
    assert!(h.app.settings_open);assert_eq!(h.app.settings_tab,1);
}

#[test]
fn music_header_shortcut_is_absent_without_music_and_keeps_settings_clickable() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for media in [crate::smtc::NowPlaying::default(),crate::smtc::NowPlaying{title:"Video".into(),artist:"Channel".into(),playing:true,playback_type:crate::smtc::MediaKind::Video,..Default::default()}] {
        let mut h=H::new();h.media=media;h.app.show_page(4);h.hover(H::local(300.0,60.0),0.8);
        assert!(h.ctx.read_response(egui::Id::new(("card",true,"header-music"))).is_none(),"没有音乐时状态行不显示音乐入口");
        h.click(H::local(340.0,18.0));assert_eq!(h.app.page,4);assert!(!h.app.settings_open);
        h.click(settings_button(&h));assert!(h.app.settings_open);assert_eq!(h.app.settings_tab,4);
    }
}

#[test]
fn mascot_and_album_share_the_single_line_lyric_island(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=true;h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,
        artwork:Some(std::sync::Arc::new(egui::ColorImage::new([32,32],egui::Color32::from_rgb(70,120,220)))),..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]晚风吹过了山岗\n[00:15]下一句"));
    for _ in 0..120 {h.frame(vec![]);}
    assert!((h.app.current_width()-348.0).abs()<0.1,"专辑、岛仔与原有歌词宽度一起保留");
    let texture=h.app.music_artwork.as_ref().unwrap().1.id();
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.brush.as_ref().is_some_and(|b|b.fill_texture_id==texture))),"专辑封面不能被岛仔替代");
    let text=h.shapes.iter().find_map(|s|match &s.shape {egui::Shape::Text(t) if t.galley.text()=="晚风吹过了山岗"=>Some(t),_=>None}).unwrap();
    assert_eq!(text.galley.rows.len(),1,"保留单行歌词");
    assert!((text.pos.x+text.galley.rect.center().x-WIN_W/2.0-24.0).abs()<1.5,"歌词在封面和岛仔右侧居中");
    assert!(h.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Path(p) if p.points.len()==13 && (2.79..=3.61).contains(&p.stroke.width))).count()>=2);
}

#[test]
fn music_mascot_puts_away_the_microphone_during_notices_and_their_fade_out(){
    use crate::island::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=true;h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]晚风吹过了山岗\n[00:15]下一句"));
    let microphone=|h:&H|h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Circle(c)if(c.radius-2.6).abs()<0.01));
    for _ in 0..90{h.frame(vec![]);}
    assert_eq!(h.app.pet_state(),PetState::Music);assert!(microphone(&h));
    for (title,body) in [("大写锁定","已开启"),("大写锁定","已关闭"),("键盘输入语言","中文"),("系统音量","音量 60%"),("网络连接","Wi-Fi · 已连接"),("电源已连接","当前电量 80%"),("蓝牙设备","耳机")]{
        h.app.alert(title,body,5,false);
        for _ in 0..60{h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Awake,"{title} 出现时应使用普通表情");assert!(!microphone(&h),"{title} 期间不能带着演唱道具");}
        h.t+=2.0;h.frame(vec![]);
        assert!(h.app.notice.is_none());
        assert_eq!(h.app.pet_state(),PetState::Awake,"提示淡出期间也不能提前开始演唱");assert!(!microphone(&h));
        for _ in 0..90{h.frame(vec![]);}
        assert_eq!(h.app.pet_state(),PetState::Music);assert!(microphone(&h),"提示完全结束后应恢复听歌动作");
        assert!(h.media.playing);assert!(h.app.island_lyric().is_some());
    }
    h.app.pet.react(PetState::Satisfied);h.app.alert("大写锁定","已开启",5,false);h.frame(vec![]);
    assert_eq!(h.app.pet_state(),PetState::Satisfied,"已有的喝水反馈应保留优先级");
}

#[test]
fn music_mascot_plays_new_expressions_and_resumes_after_interruptions(){
    use crate::island::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.pet=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:300.0,..Default::default()};
    for _ in 0..1080 {h.frame(vec![]);}
    let expression=h.app.pet.expressions.current().expect("听歌状态也应显示新表情");
    assert_eq!(h.app.pet_state(),PetState::Music);
    let atlas=h.ctx.tex_manager().read().allocated().find_map(|(id,meta)|(meta.name=="daozai-expressions").then_some(*id)).unwrap();
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Mesh(mesh)if mesh.texture_id==atlas)));
    h.app.alert("系统音量","音量 60%",5,false);h.frame(vec![]);
    assert_eq!(h.app.pet_state(),PetState::Awake);assert!(h.app.pet.expressions.current().is_none());
    for _ in 0..300 {h.frame(vec![]);if h.app.pet_state()==PetState::Music {break;}}
    assert_eq!(h.app.pet.expressions.current(),Some(expression),"通知结束后继续原来的听歌表情");
    h.media.playing=false;h.frame(vec![]);assert!(h.app.pet.expressions.current().is_none());
    for _ in 0..60 {h.frame(vec![]);}
    h.media.playing=true;h.frame(vec![]);assert_eq!(h.app.pet.expressions.current(),Some(expression));
    h.app.pomo.running=true;h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Focused);
    assert!(h.app.pet.expressions.current().is_none());
    h.app.pomo.running=false;h.frame(vec![]);assert_eq!(h.app.pet.expressions.current(),Some(expression));
    h.app.settings.pet=false;h.frame(vec![]);assert!(h.app.pet.expressions.current().is_none());
}

#[test]
fn mascot_reacts_to_music_and_respects_focus(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};h.frame(vec![]);
    assert_eq!(format!("{:?}",h.app.pet_state()),"Music");
    h.app.pomo.running=true;h.frame(vec![]);assert_eq!(format!("{:?}",h.app.pet_state()),"Focused");
    h.app.pomo.running=false;h.media.playing=false;h.frame(vec![]);assert_eq!(format!("{:?}",h.app.pet_state()),"Awake");
}

#[test]
fn mascot_power_and_network_recover_without_stale_warning(){
    use crate::island::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.power=(Some(15),false);h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Worried);
    h.power.1=true;h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Awake);
    h.network=Some(false);h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Worried);
    h.network=Some(true);h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Awake);
    h.power=(None,false);h.network=None;h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Awake);
}

#[test]
fn timer_keeps_the_existing_lyric_segments_and_content_bounds(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let line="愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光";
    for pet in [true,false] {
        let mut h=H::new();h.app.settings.island_lyrics=true;h.app.settings.pet=pet;
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:6.0,duration:120.0,playing:true,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc(&format!("[00:05]{line}\n[00:15]下一句")));
        let shown=|h:&H|h.shapes.iter().find_map(|s|match &s.shape {
            egui::Shape::Text(t) if !t.galley.text().is_empty() && line.contains(t.galley.text())=>Some((t.galley.text().to_owned(),t.visual_bounding_rect(),t.galley.rows.len(),t.galley.elided)),_=>None,
        }).unwrap();
        for position in [6.0,14.0] {
            h.media.position=position;h.app.pomo.running=false;
            for _ in 0..90 {h.frame(vec![]);}let expected=shown(&h).0;
            h.app.pomo.running=true;for _ in 0..120 {h.frame(vec![]);}
            let (text,bounds,rows,elided)=shown(&h);assert_eq!(text,expected,"计时不应改变歌词分段");
            assert_eq!(rows,1);assert!(!elided);
            let secs=h.app.pomo.remaining.ceil() as u32;let countdown=format!("{:02}:{:02}",secs/60,secs%60);
            let timer=h.shapes.iter().find_map(|s|match &s.shape {egui::Shape::Text(t) if t.galley.text()==countdown=>Some(t.visual_bounding_rect()),_=>None}).unwrap();
            assert!(bounds.right()+20.0<timer.left(),"歌词与倒计时之间应留出波形和图标的位置");
            assert!(timer.right()<h.hit.right() && bounds.top()>=h.hit.top() && bounds.bottom()<h.hit.bottom());
            assert!((h.app.current_width()-if pet{418.0}else{370.0}).abs()<0.1);
        }
    }
}

#[test]
fn collapsed_focus_animates_without_music_or_mascot_and_stops_in_minimal_mode(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=false;h.app.settings.tray_focus=false;h.app.pomo.running=true;
    for _ in 0..120 {h.frame(vec![]);}
    if crate::island::bar_layout::animations_enabled(){assert!(h.animating,"关闭吉祥物后，常驻番茄的水面仍应轻微波动");}
    h.app.set_minimal(true);for _ in 0..120 {h.frame(vec![]);}
    assert!(!h.animating,"极简细线隐藏计时图标后，应停止其动画刷新");
}

#[test]
fn fluid_music_seek_draws_solid_played_segment(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for lyrics in [false,true] {
        let mut h=H::new();
        h.app.settings.skin=crate::model::Skin::Fluid;
        h.app.show_lyrics=lyrics;
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:30.0,duration:120.0,..Default::default()};
        h.hover(Pos2::new(WIN_W/2.0,12.0),1.2);
        let (x,w,y)=if lyrics{(44.0,176.0,82.0)}else{(96.0,221.0,92.6)};
        let start=H::local(x,y);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r)
            if (r.rect.left()-start.x).abs()<1.0 && (r.rect.center().y-start.y).abs()<1.0
            && (r.rect.width()-w*0.25).abs()<1.0 && r.fill.a()==255 && r.fill.r()>160)));
    }
}

#[test]
fn music_seek_drag_previews_and_commits_on_release(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (source,lyrics) in [("",false),("",true),("cloudmusic.exe",false),("cloudmusic.exe",true),("bodian_pc.exe",false),("bodian_pc.exe",true),("kwmusic.exe",false),("kwmusic.exe",true)] {
        let mut h=H::new();h.app.show_lyrics=lyrics;
        h.media=crate::smtc::NowPlaying{source:source.into(),title:"Track".into(),artist:"Artist".into(),position:12.0,duration:120.0,..Default::default()};
        h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
        let (x,w,y)=if lyrics{(44.0,176.0,82.0)}else{(96.0,221.0,92.6)};
        let press=H::local(x+w*0.1,y);h.cursor=Some(press);
        h.frame(vec![egui::Event::PointerMoved(press)]);
        h.frame(vec![egui::Event::PointerButton{pos:press,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        let to=H::local(x+w*0.75,y);h.cursor=Some(to);
        h.frame(vec![egui::Event::PointerMoved(to)]);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="01:30")),"{source} 拖动中应显示目标时间");
        assert!(!h.cmds.iter().any(|c|matches!(c,crate::island::Cmd::Media(crate::smtc::Transport::Seek(_)))),"拖动中不应反复发送跳转");
        h.frame(vec![egui::Event::PointerButton{pos:to,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        assert!(h.cmds.iter().any(|c|matches!(c,crate::island::Cmd::Media(crate::smtc::Transport::Seek(t)) if (*t-90.0).abs()<0.1)),"释放应提交目标进度");
    }
}

#[test]
fn old_and_new_qqmusic_seek_and_hold_the_target_after_release(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for source in ["QQMusic.exe","Tencent.QQMusic"] {
        let mut h=H::new();h.app.show_lyrics=false;
        h.media=crate::smtc::NowPlaying{source:source.into(),title:"Track".into(),artist:"Artist".into(),position:12.0,duration:120.0,playback_type:crate::smtc::MediaKind::Music,..Default::default()};
        h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
        let (x,w,y)=(96.0,221.0,92.6);
        let press=H::local(x+w*0.1,y);h.cursor=Some(press);
        h.frame(vec![egui::Event::PointerMoved(press)]);
        h.frame(vec![egui::Event::PointerButton{pos:press,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        let to=H::local(x+w*0.75,y);h.cursor=Some(to);
        h.frame(vec![egui::Event::PointerMoved(to)]);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="01:30")),"{source} 拖动中显示目标时间");
        h.frame(vec![egui::Event::PointerButton{pos:to,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        assert!(h.cmds.iter().any(|c|matches!(c,crate::island::Cmd::Media(crate::smtc::Transport::Seek(t)) if (*t-90.0).abs()<0.1)),"{source} 松手提交跳转");
        // 播放器还没回报新位置：松手后仍显示目标时间，不弹回 00:12。
        h.frame(vec![]);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="01:30")),"{source} 松手后保持目标时间");
        assert!(crate::smtc::is_qqmusic(source));
    }
}

#[test]
fn volume_hud_drag_maps_pointer_to_percentage(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.alert("系统音量","音量 0%",5,false);
    for _ in 0..60 {h.frame(vec![]);}
    assert!(!h.app.open);
    h.click_notice_badge();
    // 缩短后的系统音量条仍支持点击和拖动。
    let (x0,y)=if h.app.compact_notices(){(WIN_W/2.0+(h.app.current_width()-44.0)/2.0+8.0+crate::island::HUD_TRACK_X,55.0)}else{(WIN_W/2.0-h.app.current_width()/2.0+h.app.pet_space()+crate::island::HUD_TRACK_X,19.0)};
    let press=Pos2::new(x0+52.0,y);
    h.cursor=Some(press);h.frame(vec![egui::Event::PointerMoved(press)]);
    h.frame(vec![egui::Event::PointerButton{pos:press,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
    let to=Pos2::new(x0+105.0,y);
    h.cursor=Some(to);h.frame(vec![egui::Event::PointerMoved(to)]);
    h.frame(vec![egui::Event::PointerButton{pos:to,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
    let last=h.cmds.iter().rev().find_map(|c|match c{crate::island::Cmd::SetVolume(v)=>Some(*v),_=>None});
    assert_eq!(last,Some(70));
}

#[test]
fn bluetooth_audio_connection_takes_precedence_over_volume_notice(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for volume_already_visible in [false,true] {
        let mut h=H::new();h.frame(vec![]);
        h.app.alert("系统音量","音量 80%",5,false);
        if volume_already_visible {h.frame(vec![]);}
        h.bt=vec![crate::os::bt::Device{id:"audio".into(),name:"Shokz".into(),battery:Some(80),kind:crate::os::bt::DeviceKind::Bone}];
        h.frame(vec![]);
        assert_eq!(h.app.notice.as_ref().map(|a|a.title.as_str()),Some("蓝牙设备"),"耳机连接应优先于音量提示");
        assert!(!h.app.alerts.iter().any(|a|a.title=="系统音量"),"切换耳机产生的音量提示不应延后补弹");
        h.t+=3.0;h.frame(vec![]);
        h.app.alert("系统音量","音量 60%",5,false);h.frame(vec![]);
        assert_eq!(h.app.notice.as_ref().map(|a|a.body.as_str()),Some("音量 60%"),"连接后手动调节音量应正常提示");
    }
}

#[test]
fn bluetooth_keyboard_connection_keeps_unrelated_volume_notice(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.frame(vec![]);
    h.app.alert("系统音量","音量 60%",5,false);h.frame(vec![]);
    h.bt=vec![crate::os::bt::Device{id:"keyboard".into(),name:"Keyboard".into(),battery:None,kind:crate::os::bt::DeviceKind::Keyboard}];h.frame(vec![]);
    assert_eq!(h.app.notice.as_ref().map(|a|a.title.as_str()),Some("系统音量"));
    assert!(h.app.alerts.iter().any(|a|a.title=="蓝牙设备"));
}

#[test]
#[ignore="按需导出歌词模式下正文开关的通知画面"]
fn lyric_app_notification_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_APP_NOTICE_PREVIEW").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for preview in [false,true]{
        let mut h=H::new();h.app.settings.island_lyrics=true;h.app.settings.read_notifications=true;h.app.settings.notification_preview=preview;
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗"));
        let mut raster=crate::preview_raster::Raster::new(1440,405);
        for frame in 0..90{
            if frame==30{h.app.receive_notification("微信".into(),"wechat.preview".into(),"下午三点开会，记得带上电脑".into());}
            let t=frame as f64/60.0;
            let env=Env{t,clock:"09:41:00".into(),media:h.media.clone(),..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(1.5),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),&env);});});
            raster.update_textures(&output.textures_delta);
            if frame==89{
                raster.buf.fill([0.94,0.95,0.97,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);let image=image::RgbaImage::from_raw(1440,405,rgba).unwrap();
                image::imageops::crop_imm(&image,105,0,960,150).to_image().save(dest.join(if preview{"正文开启.png"}else{"正文关闭.png"})).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }
}

#[test]
fn notification_style_follows_the_current_lyric_mode_without_a_new_notice(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗"));
    h.app.alert("大写锁定","已开启",5,false);
    for (lyrics,playing) in [(false,true),(true,true),(true,false),(true,true),(false,true)]{
        h.app.settings.island_lyrics=lyrics;h.media.playing=playing;for _ in 0..25{h.frame(vec![]);}
        assert_eq!(h.app.compact_notices(),lyrics&&playing);
        let has_title=h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="大写锁定"&&t.opacity_factor>0.9));
        assert_eq!(has_title,!(lyrics&&playing),"当前通知应跟随岛上歌词的显示状态切换样式");
    }
}

#[test]
fn lyric_caps_lock_notice_keeps_one_glass_outline(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for skin in [crate::model::Skin::GlassLight,crate::model::Skin::GlassDark]{
    for shape in crate::model::IslandShape::ALL{for ppp in [1.0,1.25,1.5,2.0]{
        let mut h=H::new();h.app.settings.pet=false;h.app.settings.skin=skin;h.app.settings.island_shape=shape;h.app.settings.island_lyrics=true;h.app.settings.glass_blur=true;
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗"));
        let mut raster=crate::preview_raster::Raster::new((crate::island::CANVAS_W*ppp) as usize,(WIN_H*ppp) as usize);
        for frame in 0..360{
            h.t=frame as f64/60.0;
            if frame==60{h.app.alert("大写锁定","已开启",5,false);}if frame==105{h.app.alert("大写锁定","已关闭",5,false);}
            let env=Env{t:h.t,clock:"09:41:00".into(),media:h.media.clone(),backdrop:Some([0.94,0.95,0.97]),glass_blur:true,..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(h.t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
            let mut glass=None;
            let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{glass=h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),&env).glass;});});
            let contours=glass.unwrap();
            assert_eq!(contours.len(),1,"{skin:?} {shape:?} 倍率 {ppp} 第 {frame} 帧：提示与原岛应共用外轮廓，避免拼接处出现内圈");
            if shape==crate::model::IslandShape::Pill{
                let rims=output.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.stroke.width>0.0&&r.rect.width()>50.0&&r.rect.height()>=20.0)).count();
                assert_eq!(rims,1,"胶囊连接处不能重复绘制玻璃描边");
            }
            raster.update_textures(&output.textures_delta);let primitives=h.ctx.tessellate(output.shapes,ppp);
            if ppp==1.5{if let Some(name)=match frame{45=>Some("before"),90=>Some("on"),135=>Some("off"),359=>Some("after"),_=>None}{if let Some(dest)=std::env::var_os("DAOZAI_CAPS_LOCK_PREVIEW_DIR"){
                let dest=std::path::PathBuf::from(dest);std::fs::create_dir_all(&dest).unwrap();raster.buf.fill([0.94,0.95,0.97,1.0]);raster.draw(&primitives,ppp);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);let image=image::RgbaImage::from_raw(raster.w as u32,raster.h as u32,rgba).unwrap();
                image::imageops::crop_imm(&image,(140.0*ppp) as u32,0,(440.0*ppp) as u32,(80.0*ppp) as u32).to_image().save(dest.join(format!("{}-{}-{name}.png",skin.key(),shape.key()))).unwrap();
            }}}
            raster.free_textures(&output.textures_delta);
        }
        assert!(h.app.notice.is_none());assert!((h.app.current_width()-crate::island::LYRIC_BAR_W).abs()<1.0,"提示结束后应恢复原有宽度");
    }}}
}

#[test]
fn bluetooth_notification_keeps_its_full_message_outside_lyric_mode(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();for _ in 0..90 {h.frame(vec![]);}
    h.app.alert("蓝牙设备","OpenMove by Shokz · 电量 80%",5,false);
    for _ in 0..60 {h.frame(vec![]);}
    assert!((h.app.current_width()-crate::island::HUD_W-h.app.pet_space()).abs()<1.0);
    assert!(h.region.height()>60.0,"两行蓝牙提示保持完整高度");
    h.click_notice_badge();
    for label in ["OpenMove by Shokz","已连接","80%"]{assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==label)),"点击后可查看设备信息：{label}");}
}

#[test]
fn hud_repeated_volume_changes_do_not_restart_entrance(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.frame(vec![]);
    h.app.alert("系统音量","音量 20%",5,false);
    for _ in 0..45 {h.frame(vec![]);}
    let before=h.app.current_width();
    h.app.alert("系统音量","音量 30%",5,false);
    h.frame(vec![]);
    assert!((h.app.current_width()-before).abs()<5.0);
    assert_eq!(h.app.notice.as_ref().unwrap().body,"音量 30%");
}

#[test]
fn auto_minimal_music_stops_hidden_playback_animations(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.set_minimal(true);
    h.media=crate::smtc::NowPlaying{title:"歌曲".into(),artist:"歌手".into(),playing:true,..Default::default()};
    for _ in 0..180{h.frame(vec![]);}
    assert!(!h.app.open);
    assert!(!h.animating,"极简模式隐藏媒体组件后，不应继续为播放动画刷新");
}

#[test]
fn tray_music_and_battery_leave_the_resident_row_and_show_on_change(){
    use crate::island::bar_layout::{BATTERY,MUSIC,TRAY};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_music=true;h.app.settings.tray_battery=true;
    h.power=(Some(80),false);
    h.media=crate::smtc::NowPlaying{source:"cloudmusic.exe".into(),title:"Track A".into(),artist:"Artist".into(),playing:true,..Default::default()};
    for _ in 0..30{h.frame(vec![]);}
    let layout=h.app.resident.current();
    assert!(layout.x[MUSIC].is_none(),"音乐进容器后不在常驻行");
    assert!(layout.x[BATTERY].is_none(),"电量进容器后不在常驻行");
    h.media.title="Track B".into();
    for _ in 0..5{h.frame(vec![]);}
    assert_eq!(h.app.tray_active(),Some(crate::island::event_tray::Item::Music),"换歌时容器显示音乐");
    assert!(h.app.resident.current().x[TRAY].is_some(),"换歌时容器占位");
    h.t+=crate::island::event_tray::HOLD+1.0;
    for _ in 0..3{h.frame(vec![]);}
    assert!(h.app.tray_active().is_none(),"事件窗口结束后容器收回");
    h.power=(Some(79),false);
    for _ in 0..5{h.frame(vec![]);}
    assert_eq!(h.app.tray_active(),Some(crate::island::event_tray::Item::Battery),"电量变化时容器显示电量");
}

#[test]
fn media_sampling_is_fast_only_while_the_music_page_is_watched(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"QQMusic.exe".into(),title:"歌曲".into(),artist:"歌手".into(),duration:120.0,playing:true,..Default::default()};
    h.frame(vec![]);
    h.app.open=false;
    assert_eq!(h.app.media_poll_interval(),crate::smtc::POLL_BACKGROUND,"收起且不显示歌词时只需跟上歌名");
    h.app.settings.island_lyrics=true;h.frame(vec![]);h.app.open=false;
    assert_eq!(h.app.media_poll_interval(),crate::smtc::POLL_LYRICS,"收起显示歌词时次快");
    h.app.open=true;h.app.page=0;
    assert_eq!(h.app.media_poll_interval(),crate::smtc::POLL_FAST,"展开音乐页保持原来的 250 毫秒");
    h.app.page=1;
    assert_eq!(h.app.media_poll_interval(),crate::smtc::POLL_BACKGROUND,"展开其他页面时不必按最快频率采样");
}
