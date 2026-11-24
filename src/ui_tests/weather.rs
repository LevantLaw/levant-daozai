//! 天气页、预报与城市设置。
use super::*;

#[test]
fn weather_content_tap_switches_both_ways_with_a_short_transition(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (x,y) in [(100.0,75.0),(270.0,82.0),(314.0,178.0)] {
        let mut h=H::new();h.app.settings.pet=false;
        h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"杭州".into(),temp:19.0,sunrise:Some("06:52".into()),sunset:Some("17:21".into()),..Default::default()});
        h.app.show_page(8);let p=H::local(x,y);h.hover(p,1.0);h.click(p);
        assert!(h.app.weather.solar_open,"天气内容区 ({x},{y}) 应可以轻点切换");
        h.hover(p,0.05);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains("返回天气")&&t.fallback_color.a()>0&&t.fallback_color.a()<255)),"切换时应有短暂淡入动效");
        h.hover(p,0.3);h.click(p);assert!(!h.app.weather.solar_open,"日照内容区也可以轻点返回");
        h.hover(p,0.3);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="19°"&&t.fallback_color.a()==255)),"动效完成后应恢复清晰的天气概览");
    }
}

#[test]
fn weather_content_tap_keeps_other_controls_independent(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (x,y,target) in [(375.0,crate::island::WEATHER_H-7.0,8),(394.0,18.0,8),(398.0,88.0,9),(20.0,88.0,7)] {
        let mut h=H::new();h.app.settings.pet=false;
        h.media=crate::music::NowPlaying{source:"cloudmusic.exe".into(),title:"歌曲".into(),artist:"歌手".into(),playing:true,..Default::default()};
        h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"杭州".into(),temp:19.0,..Default::default()});
        h.app.show_page(8);h.hover(H::local(250.0,65.0),1.0);
        let point=if x==394.0 {settings_button(&h)}else{H::local(x,y)};
        h.click(point);
        assert!(!h.app.weather.solar_open,"操作 ({x},{y}) 不应触发日照切换");
        assert_eq!(h.app.page(),target,"操作 ({x},{y}) 的页面跳转应保持不变");
        if y>200.0 {assert!(h.cmds.contains(&crate::island::Cmd::OpenUrl("https://open-meteo.com/".into())));}
        if x==394.0 {assert!(h.app.settings_open);}
    }
}

#[test]
fn weather_week_shows_all_seven_days(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"杭州".into(),temp:19.0,
        days:["今天","明天","周五","周六","周日","周一","周二"].into_iter().map(|label|crate::weather::Day{label:label.into(),high:23.0,low:15.0,..Default::default()}).collect(),
        at:chrono::Utc::now().timestamp(),..Default::default()});
    h.app.show_page(8);h.hover(H::local(250.0,65.0),1.0);
    for label in ["今天","明天","周五","周六","周日","周一","周二"] {
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==label)),"七天预报缺少 {label}");
    }
}

#[test]
fn weather_week_keeps_each_forecast_date_visible(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.weather.snapshot=Some(crate::weather::parse("杭州",&serde_json::json!({"current":{"temperature_2m":19},
        "daily":{"time":["2026-11-28","2026-11-29","2026-11-30","2026-12-01","2026-12-02","2026-12-03","2026-12-04"]}})).unwrap());
    h.app.show_page(8);h.hover(H::local(250.0,65.0),1.0);
    for date in ["11/28","11/29","11/30","12/01","12/02","12/03","12/04"] {
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==date)),"七天预报缺少日期 {date}");
    }
    for s in &h.shapes {if let egui::Shape::Text(t)=&s.shape {assert!(s.clip_rect.contains_rect(t.visual_bounding_rect()),"日期与预报文字不应裁切：{}",t.galley.text());}}
}

#[test]
fn weather_today_opens_solar_details_and_returns_to_forecast(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"杭州".into(),temp:19.0,sunrise:Some("06:52".into()),sunset:Some("17:21".into()),sun_times:None,..Default::default()});
    h.app.show_page(8);h.hover(H::local(250.0,65.0),1.0);h.click(H::local(100.0,75.0));h.hover(H::local(100.0,75.0),0.3);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains("返回天气"))),"点击当天区域后应进入日出日落视图");
    for phrase in ["日出","日落","06:52","17:21","日照进度待更新"] {
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains(phrase))),"旧缓存日照视图缺少 {phrase}");
    }
    assert!(!h.app.weather.loading,"浏览日照视图不应额外发起网络请求");
    h.click(H::local(80.0,44.0));
    assert!(!h.app.weather.solar_open);
    h.hover(H::local(80.0,44.0),0.3);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="日出 06:52")),"返回后应显示天气视图的日照入口");
}

#[test]
fn weather_side_bar_shows_a_clickable_temperature_on_both_edges(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dock in [crate::model::Dock::Left,crate::model::Dock::Right] {
        let mut h=H::new();h.app.settings.dock=dock;
        h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"杭州".into(),temp:-12.0,code:0,day:true,..Default::default()});
        for _ in 0..60{side_frame(&mut h,Some(Pos2::new(250.0,580.0)));}
        let response=h.ctx.read_response(egui::Id::new(("card",true,"weather-side"))).expect("两侧都应提供常驻天气入口");
        let temp=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()=="12°"=>Some((s.clip_rect,t.visual_bounding_rect())),_=>None}).expect("负温度应完整显示");
        assert!(temp.0.contains_rect(temp.1),"{dock:?} 负温度不应裁切");
        let click=response.rect.center();
        side_frame(&mut h,Some(click));
        for pressed in [true,false] {
            h.t+=1.0/60.0;
            let env=Env{t:h.t,clock:"09:41:00".into(),cursor:Some(click),..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(500.0,600.0))),time:Some(h.t),focused:true,
                events:vec![egui::Event::PointerButton{pos:click,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}],..Default::default()};
            let _=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{h.app.frame(ui,ui.max_rect(),&env);});});
        }
        assert_eq!(h.app.page(),8,"{dock:?} 点击侧边温度应进入天气页");
        assert!(!h.app.weather.solar_open);
        h.app.settings.visible_pages[8]=false;
        assert!(!h.app.side_items().contains(&crate::island::side_dock::SideItem::Weather));
        h.app.settings.visible_pages[8]=true;h.app.set_minimal(true);
        assert!(!h.app.side_items().contains(&crate::island::side_dock::SideItem::Weather));
    }
}

#[test]
fn weather_solar_view_preserves_readable_times_across_skins_and_scales(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    use chrono::TimeZone;
    let now=chrono_tz::Asia::Shanghai.with_ymd_and_hms(2026,10,28,9,41,0).single().unwrap().with_timezone(&chrono::Local);
    for skin in crate::model::Skin::ALL {for scale in [1.0,1.25,1.5] {
        let mut h=H::new();h.app.settings.skin=skin;h.ctx.set_pixels_per_point(scale);
        h.app.weather.snapshot=Some(crate::weather::parse("杭州",&serde_json::json!({"timezone":"Asia/Shanghai","current":{"temperature_2m":19},
            "daily":{"time":["2026-10-28"],"sunrise":["2026-10-28T06:52"],"sunset":["2026-10-28T17:21"]}})).unwrap());
        h.app.weather.solar_open=true;h.app.show_page(8);
        for _ in 0..60 {
            h.t+=1.0/60.0;
            let cursor=H::local(180.0,185.0);
            let env=Env{t:h.t,clock:now.format("%H:%M:%S").to_string(),today:now.format("%Y-%m-%d").to_string(),date:now.format("%m月%d日").to_string(),cursor:Some(cursor),bt_ready:true,..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(h.t),focused:true,
                events:vec![egui::Event::PointerMoved(cursor)],..Default::default()};
            h.shapes=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{
                h.app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H)),&env);
            });}).shapes;
        }
        for phrase in ["返回天气","距离日落","07:40","白昼中点","12:06","06:52","17:21","白昼 10小时29分"] {
            assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains(phrase))),"{skin:?} / {scale} 日照视图缺少 {phrase}");
        }
        for s in &h.shapes {if let egui::Shape::Text(t)=&s.shape {assert!(s.clip_rect.contains_rect(t.visual_bounding_rect()),"日照文字不应裁切：{}",t.galley.text());}}
    }}
}

#[test]
#[ignore="按需导出七日预报、日照和两侧常驻天气的实际画面"]
fn weather_foundation_preview(){
    use crate::model::{Dock,Skin};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_WEATHER_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for (name,dock,solar,clock,skin) in [
        ("weather-week",Dock::Top,false,"14:08:00",Skin::ALL[0]),
        ("weather-sun",Dock::Top,true,"14:08:00",Skin::ALL[0]),
        ("weather-night",Dock::Top,true,"20:08:00",Skin::ALL[0]),
        ("weather-side-left",Dock::Left,false,"14:08:00",Skin::ALL[0]),
        ("weather-side-right",Dock::Right,false,"14:08:00",Skin::ALL[0]),
        ("weather-week-cream",Dock::Top,false,"14:08:00",Skin::Cream),
        ("weather-sun-cream",Dock::Top,true,"14:08:00",Skin::Cream),
        ("weather-week-glass",Dock::Top,false,"14:08:00",Skin::GlassDark),
        ("weather-sun-glass",Dock::Top,true,"14:08:00",Skin::GlassDark),
        ("weather-city-picker",Dock::Top,false,"14:08:00",Skin::Classic),
    ] {
        let mut h=H::new();h.app.settings.dock=dock;h.app.settings.skin=skin;h.app.settings.pet=false;
        let mut snapshot=crate::weather::parse("杭州",&serde_json::json!({"timezone":"Asia/Shanghai",
            "current":{"temperature_2m":19,"apparent_temperature":18,"relative_humidity_2m":58,"wind_speed_10m":12,"weather_code":2,"is_day":1},
            "daily":{"time":["2026-11-11","2026-11-12","2026-11-13","2026-11-14","2026-11-15","2026-11-16","2026-11-17"],
                "weather_code":[2,0,3,61,80,2,0],"temperature_2m_max":[23,24,22,19,18,20,22],"temperature_2m_min":[15,16,14,13,12,12,14],
                "precipitation_probability_max":[10,0,20,70,55,10,0],"sunrise":["2026-11-11T06:52"],"sunset":["2026-11-11T17:21"]}})).unwrap();
        if clock=="20:08:00" {snapshot.day=false;}
        h.app.weather.snapshot=Some(snapshot);
        if name=="weather-city-picker" {
            let city=|label:&str,region:&str,lat:f64|crate::weather::Position{lat,lon:120.0,label:label.into(),region:region.into()};
            h.app.settings.weather_saved=vec![city("杭州","浙江 · 中国",30.27),city("上海","上海 · 中国",31.23),city("哈尔滨","黑龙江 · 中国",45.75)];
            h.app.settings.weather_city="杭州".into();h.app.settings.weather_position=Some(h.app.settings.weather_saved[0].clone());
            h.app.weather.snapshot.as_mut().unwrap().position=h.app.settings.weather_position.clone();
            h.app.weather.picker_open=true;
        }
        if dock==Dock::Top {h.app.weather.solar_open=solar;h.app.show_page(8);}
        let (w,hgt)=if dock==Dock::Top {(crate::island::CANVAS_W,WIN_H)}else{(crate::island::side_dock::SIDE_W,crate::island::side_dock::SIDE_H)};
        let mut raster=feature_raster_sized(w,hgt);
        let frames=90;
        for frame in 0..frames {
            h.t+=1.0/60.0;
            let cursor=Some(if dock==Dock::Top{H::local(180.0,185.0)}else{Pos2::new(250.0,580.0)});
            let env=Env{t:h.t,clock:clock.into(),today:"2026-11-11".into(),date:"11月11日 周三".into(),cursor,battery:Some(76),bt_ready:true,..Default::default()};
            let events=vec![egui::Event::PointerMoved(cursor.unwrap())];
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,
                events,viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{
                h.app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(if dock==Dock::Top{WIN_W}else{w},hgt)),&env);
            });});
            raster.update_textures(&output.textures_delta);
            if frame==frames-1 {
                raster.buf.fill([0.20,0.24,0.32,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                let image=image::RgbaImage::from_raw((w*2.0)as u32,(hgt*2.0)as u32,rgba).unwrap();
                let (x,y,crop_w,crop_h)=if dock==Dock::Top {(200,0,880,(h.app.expanded_height()*2.0+24.0)as u32)}else{
                    let rect=h.app.island_rect(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt)),h.app.w.pos,h.app.h.pos);
                    (if dock==Dock::Left{0}else{880},((rect.top()-12.0)*2.0)as u32,120,((rect.height()+24.0)*2.0)as u32)
                };
                let file=format!("{name}.png");
                image::imageops::crop_imm(&image,x,y,crop_w,crop_h).to_image().save(dest.join(file)).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }
}

#[test]
#[ignore="按需导出进入日照视图时的涟漪与太阳滑入过程"]
fn weather_solar_entry_preview(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_WEATHER_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let mut h=H::new();h.app.settings.pet=false;
    h.app.weather.snapshot=Some(crate::weather::parse("杭州",&serde_json::json!({"timezone":"Asia/Shanghai",
        "current":{"temperature_2m":19,"apparent_temperature":18,"relative_humidity_2m":58,"wind_speed_10m":12,"weather_code":2,"is_day":1},
        "daily":{"time":["2026-11-11"],"weather_code":[2],"temperature_2m_max":[23],"temperature_2m_min":[15],
            "precipitation_probability_max":[10],"sunrise":["2026-11-11T06:52"],"sunset":["2026-11-11T17:21"]}})).unwrap());
    h.app.show_page(8);
    let (w,hgt)=(crate::island::CANVAS_W,WIN_H);
    let mut raster=feature_raster_sized(w,hgt);
    let click=H::local(330.0,94.0);
    // 第 60 帧按下、61 帧松开；之后按时间点导出。
    let shots=[(70,"weather-entry-a"),(85,"weather-entry-b"),(110,"weather-entry-c"),(160,"weather-entry-d")];
    for frame in 0..161 {
        h.t+=1.0/60.0;
        let env=Env{t:h.t,clock:"14:08:00".into(),today:"2026-11-11".into(),date:"11月11日 周三".into(),cursor:Some(click),battery:Some(76),bt_ready:true,..Default::default()};
        let mut events=vec![egui::Event::PointerMoved(click)];
        if frame==60||frame==61 {events.push(egui::Event::PointerButton{pos:click,button:egui::PointerButton::Primary,pressed:frame==60,modifiers:Default::default()});}
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,events,
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{
            h.app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,hgt)),&env);
        });});
        raster.update_textures(&output.textures_delta);
        if let Some((_,name))=shots.iter().find(|(at,_)|*at==frame) {
            raster.buf.fill([0.20,0.24,0.32,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
            let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            let image=image::RgbaImage::from_raw((w*2.0)as u32,(hgt*2.0)as u32,rgba).unwrap();
            image::imageops::crop_imm(&image,200,0,880,(h.app.expanded_height()*2.0+24.0)as u32).to_image().save(dest.join(format!("{name}.png"))).unwrap();
        }
        raster.free_textures(&output.textures_delta);
    }
    assert!(h.app.weather.solar_open,"点击天气视图后应进入日照视图");
}

#[test]
fn weather_hidden_page_opens_its_settings_without_panicking(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.set_page_visible(8,false);h.app.show_page(8);
    assert!(h.app.settings_open);assert_eq!(h.app.settings_tab,12);
}

#[test]
fn weather_settings_maps_to_the_module_switch(){
    assert_eq!(crate::island::extras::settings_tab_module(12),Some(8));
}

#[test]
fn weather_city_draft_keeps_the_saved_location_until_confirmed(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.weather_city="杭州".into();h.app.settings.wecom_notifications=true;
    h.app.settings.feishu_notifications=true;h.app.settings.dingtalk_notifications=true;
    h.app.settings.save();h.app.weather.city_input="北京".into();
    h.frame(vec![]);
    assert_eq!(h.app.settings.weather_city,"杭州");
    let saved=crate::model::Settings::load();
    assert_eq!(saved.weather_city,"杭州");
    assert!(saved.wecom_notifications && saved.feishu_notifications && saved.dingtalk_notifications);
    h.app.set_page_visible(8,false);h.app.add_weather_city();
    assert_eq!(h.app.settings.weather_city,"杭州");assert!(!h.app.weather.loading);
}

#[test]
fn weather_cities_switch_from_the_island_title_and_keep_their_own_cache(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.pet=false;
    let hz=crate::weather::Position{lat:30.27,lon:120.15,label:"杭州".into(),region:"浙江 · 中国".into()};
    let bj=crate::weather::Position{lat:39.90,lon:116.40,label:"北京".into(),region:"北京 · 中国".into()};
    // 设置页添加两个城市：只加入列表，搜索框清空。
    for city in [&hz,&bj] {
        h.app.weather.city_input=city.label.clone();h.app.weather.selected_city=Some(city.clone());
        assert!(h.app.add_weather_city());
        assert!(h.app.settings.weather_city.is_empty(),"添加城市只加入列表，不切换当前城市");
        assert!(h.app.weather.city_input.is_empty());
    }
    assert_eq!(h.app.settings.weather_saved,vec![hz.clone(),bj.clone()]);
    h.app.weather.city_input=hz.label.clone();h.app.weather.selected_city=Some(hz.clone());
    assert!(!h.app.add_weather_city(),"重复城市不再添加");
    assert_eq!(crate::model::Settings::load().weather_saved,vec![hz.clone(),bj.clone()],"城市列表应保存到设置");
    // 打开天气页，点标题展开下拉框，选「杭州」。
    h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"北京".into(),position:Some(bj.clone()),temp:-3.0,code:71,day:true,at:chrono::Utc::now().timestamp(),..Default::default()});
    h.app.show_page(8);h.hover(H::local(250.0,65.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="当地")),"标题保持「当地天气」");
    h.click(H::local(115.0,112.0));h.hover(H::local(330.0,200.0),0.4);
    assert!(h.app.weather.picker_open,"点地点行展开城市下拉");assert!(!h.app.weather.solar_open,"点地点行不应切到日照");
    for phrase in ["自动定位","杭州","北京","添加或管理城市"] {
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains(phrase))),"下拉框缺少 {phrase}");
    }
    let row=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()=="杭州"=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap();
    h.click(row);h.hover(row,0.3);
    assert_eq!(h.app.settings.weather_city,"杭州");assert!(!h.app.weather.picker_open);assert!(!h.app.weather.solar_open);
    assert!(h.app.weather.snapshot.is_none(),"杭州没有缓存时不能显示北京的天气");
    // 点面板外收起，不触发日照视图。
    h.click(H::local(209.0,147.0));h.hover(H::local(330.0,200.0),0.4);assert!(h.app.weather.picker_open,"无数据时入口在页面中下方");
    h.click(H::local(330.0,200.0));h.hover(H::local(330.0,200.0),0.3);
    assert!(!h.app.weather.picker_open);assert!(!h.app.weather.solar_open,"点外面只收起下拉框");
    // 删除当前城市：回到自动定位。
    h.app.remove_weather_city(&hz);
    assert_eq!(h.app.settings.weather_saved,vec![bj.clone()]);assert!(h.app.settings.weather_city.is_empty());assert!(h.app.settings.weather_position.is_none());
}

#[test]
fn weather_negative_temperature_draws_a_hyphen_length_minus_on_the_digit_midline(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_weather=false;
    h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"哈尔滨".into(),temp:-6.0,code:71,day:true,at:chrono::Utc::now().timestamp(),..Default::default()});
    for _ in 0..60{h.frame(vec![]);}
    let digits=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()=="6°"=>Some(t.visual_bounding_rect()),_=>None}).expect("胶囊应显示 6°");
    let bar=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Rect(r) if (digits.left()-r.rect.right()).abs()<4.0 && r.rect.right()<digits.center().x && r.rect.width()>r.rect.height()*1.5 && r.rect.height()<4.0=>Some(r.rect),_=>None}).expect("6° 左侧应有手绘减号");
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains('−'))),"不再使用过长的排版减号");
    assert!(bar.width()<digits.height()*0.6,"减号长度与连字符相当");
    assert!((bar.center().y-digits.center().y).abs()<digits.height()*0.2,"减号在数字中线上");
}

#[test]
fn weather_city_dropdown_selects_the_region_without_applying_the_draft(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let correct=crate::weather::Position{lat:23.02677,lon:113.13148,label:"佛山市".into(),region:"广东 · 中国".into()};
    let wrong=crate::weather::Position{lat:28.73401,lon:98.80612,label:"佛山".into(),region:"迪庆藏族自治州 · 云南 · 中国".into()};
    h.app.settings.weather_city="杭州".into();h.app.weather.manual_city=true;h.app.weather.city_input="佛山".into();
    h.app.weather.cities.seed("佛山",vec![correct.clone(),wrong]);
    fn draw(h:&mut H,events:Vec<egui::Event>){
        h.t+=1.0/60.0;h.app.t=h.t;
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(560.0,380.0))),time:Some(h.t),events,..Default::default()};
        h.shapes=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().show(ctx,|ui|{h.app.weather_city_picker(ui);});}).shapes;
    }
    for _ in 0..3 {draw(&mut h,vec![]);}
    h.ctx.memory_mut(|m|m.open_popup(egui::Id::new("weather-city-options")));
    for _ in 0..3 {draw(&mut h,vec![]);}
    let point=h.shapes.iter().find_map(|s|match &s.shape {egui::Shape::Text(text) if text.galley.text()=="佛山市"=>Some(text.visual_bounding_rect().center()),_=>None}).expect("下拉框必须显示完整城市名");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="广东 · 中国")),"下拉框必须显示省份");
    draw(&mut h,vec![egui::Event::PointerMoved(point),egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
    draw(&mut h,vec![egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
    assert_eq!(h.app.weather.selected_city.as_ref(),Some(&correct));assert_eq!(h.app.weather.draft_city(),Some("佛山市"));
    assert_eq!(h.app.settings.weather_city,"杭州","选择候选后仍需应用确认");assert!(!h.app.weather.loading);
    assert!(!h.ctx.memory(|m|m.is_popup_open(egui::Id::new("weather-city-options"))));
    h.app.settings.weather_city="佛山市".into();h.app.settings.weather_position=Some(correct.clone());h.app.settings.save();
    let saved=crate::model::Settings::load();assert_eq!(saved.weather_position,Some(correct));
    h.app.weather.city_input="伦敦".into();assert_eq!(h.app.weather.draft_city(),None,"改写输入后必须重新选择地区");
}

#[test]
fn weather_attribution_opens_its_source_without_changing_pages(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.show_page(8);h.hover(H::local(300.0,60.0),1.0);
    h.click(H::local(375.0,crate::island::WEATHER_H-7.0));
    assert!(h.cmds.contains(&crate::island::Cmd::OpenUrl("https://open-meteo.com/".into())));
    assert_eq!(h.app.page(),8);
}

#[test]
fn weather_refresh_keeps_music_and_settings_shortcuts_separate(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::music::NowPlaying{source:"cloudmusic.exe".into(),title:"歌曲".into(),artist:"歌手".into(),playing:true,..Default::default()};
    h.app.show_page(8);h.hover(H::local(300.0,60.0),1.0);
    let rect=|name|h.ctx.read_response(egui::Id::new(("card",true,name))).unwrap().rect;
    let refresh=rect("weather-refresh");
    assert!(!refresh.intersects(rect("header-music")));
    assert!(!refresh.intersects(rect("page-settings")));
}

#[test]
fn weather_resident_settings_preserve_old_orders_and_the_status_switch(){
    use crate::island::bar_layout::{AGENT,BATTERY,CLOCK,FOCUS,MUSIC,PET,QUOTA,WEATHER};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let path=crate::model::config_dir().join("settings.txt");std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path,"resident_order=7,6,2,0,9,1\nshow_weather=0\n").unwrap();
    h.app.settings=crate::model::Settings::load();
    assert_eq!(h.app.settings.resident_order,[FOCUS,MUSIC,QUOTA,CLOCK,WEATHER,PET,BATTERY,AGENT],"旧排列应在时钟后插入天气、末尾补 AI 会话并保留其余顺序");
    h.app.weather.snapshot=Some(crate::weather::Snapshot{temp:9.0,..Default::default()});
    assert!(h.app.weather_pill().is_none(),"状态开关关闭后天气不占常驻位置");
    assert!(h.app.module_on(8),"关闭常驻天气应保留天气页面");
    h.app.settings.save();h.app.settings=crate::model::Settings::load();
    assert!(h.app.weather_pill().is_none(),"重启后应保留常驻天气开关");
    for invalid in ["0,0,2,5,6,7,9","0,1,2,5,6,7,99","0,1,2","invalid",""] {
        std::fs::write(&path,format!("resident_order={invalid}\n")).unwrap();
        assert!(crate::model::Settings::load().resident_order.is_empty(),"无效排列应使用默认位置");
    }
}

#[test]
fn weather_status_switch_controls_the_badge_and_keeps_the_page_available(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(h:&mut H,events:Vec<egui::Event>){
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();h.t+=0.02;let time=h.t;
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,1900.0))),time:Some(time),events:events.clone(),viewports:std::iter::once((id,egui::ViewportInfo::default())).collect(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let _=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));
        h.shapes=output.lock().unwrap().take().unwrap().shapes;
    }
    for theme in [crate::island::settings_theme::SettingsTheme::Dark,crate::island::settings_theme::SettingsTheme::Light] {
        let mut h=H::new();h.app.settings.settings_theme=theme;h.app.settings_tab=crate::island::extras::SETTINGS_TAB_BODY;h.app.open_settings();h.ctx.set_embed_viewports(false);
        h.app.weather.snapshot=Some(crate::weather::Snapshot{temp:9.0,..Default::default()});
        for _ in 0..3 {draw(&mut h,vec![]);}
        let text_rect=|h:&H,label:&str|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect()),_=>None}).unwrap();
        let heading=text_rect(&h,"岛上显示");let bottom=text_rect(&h,"功能页面").top();
        let title=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()=="天气"&&t.pos.x>=heading.left()&&t.pos.y>heading.bottom()&&t.pos.y<bottom=>Some(t.visual_bounding_rect()),_=>None}).expect("岛上显示应有独立的天气开关");
        let point=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Rect(r) if (r.rect.width()-34.0).abs()<0.01&&(r.rect.height()-20.0).abs()<0.01&&r.rect.center().y>title.top()&&r.rect.center().y<title.bottom()+24.0=>Some(r.rect.center()),_=>None}).unwrap();
        for expected in [false,true] {
            for pressed in [true,false] {draw(&mut h,vec![egui::Event::PointerMoved(point),egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}]);}
            assert_eq!(h.app.weather_pill().is_some(),expected);
            assert_eq!(h.app.side_items().contains(&crate::island::side_dock::SideItem::Weather),expected);
            assert!(h.app.module_on(8),"关闭状态组件后天气页仍可使用");
            h.app.settings=crate::model::Settings::load();assert_eq!(h.app.weather_pill().is_some(),expected,"天气开关应自动保存");
        }
    }
}

#[test]
fn weather_page_shows_the_local_snapshot_and_forecast(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.pet=false;
    h.app.weather.snapshot=Some(crate::weather::Snapshot{condition:None,attributions:vec![],place:"杭州".into(),position:None,auto:false,temp:9.0,feels:Some(5.6),humidity:Some(58),wind:Some(13.9),code:51,day:true,high:Some(15.2),low:Some(6.8),
        sunrise:Some("06:52".into()),sunset:Some("17:21".into()),sun_times:None,
        days:vec![
            crate::weather::Day{label:"今天".into(),date:"2026-11-09".into(),high:15.2,low:6.8,code:51,rain:60},
            crate::weather::Day{label:"明天".into(),date:"2026-11-10".into(),high:14.5,low:4.7,code:2,rain:0},
            crate::weather::Day{label:"周三".into(),date:"2026-11-11".into(),high:17.4,low:5.0,code:2,rain:0},
            crate::weather::Day{label:"周四".into(),date:"2026-11-12".into(),high:19.2,low:8.2,code:61,rain:25},
        ],at:chrono::Utc::now().timestamp()});
    h.app.show_page(8);h.hover(H::local(300.0,60.0),1.0);
    assert_eq!(h.app.page,8,"天气应是第九个页面");
    for phrase in ["9°","毛毛雨","杭州","今天","明天","周三","周四","60%","体感","6°","湿度","58%","日出 06:52","日落 17:21"]{
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains(phrase))),"天气页缺少：{phrase}");
    }
    for s in &h.shapes{if let egui::Shape::Text(t)=&s.shape{assert!(s.clip_rect.contains_rect(t.visual_bounding_rect()),"天气页文字被裁切：{}",t.galley.text());}}
    if let Some(dest)=std::env::var_os("DAOZAI_WEATHER_PREVIEW_DIR"){
        let dest=std::path::PathBuf::from(dest);std::fs::create_dir_all(&dest).unwrap();
        let mut raster=feature_raster();shoot_feature(&mut h,&mut raster,&dest,"weather",30,Some(H::local(300.0,60.0)),"2026-11-09");
    }
}

#[test]
fn weather_page_explains_when_data_is_missing(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.weather.error=Some("网络不可用".into());
    h.app.show_page(8);h.hover(H::local(300.0,60.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="天气暂时取不到")));
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains("网络不可用"))));
}

#[test]
fn weather_chip_sits_after_the_clock_on_the_resident_bar(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_weather=false;
    h.app.weather.snapshot=Some(crate::weather::Snapshot{condition:None,attributions:vec![],place:"杭州".into(),position:None,auto:false,temp:9.0,feels:Some(5.6),humidity:Some(58),wind:Some(13.9),code:51,day:true,high:Some(15.2),low:Some(6.8),sunrise:Some("06:52".into()),sunset:Some("17:21".into()),sun_times:None,days:vec![],at:chrono::Utc::now().timestamp()});
    for _ in 0..60{h.frame(vec![]);}
    let layout=h.app.resident.current();
    let clock=layout.x[crate::island::bar_layout::CLOCK].expect("时钟应常驻");
    let chip=layout.x[crate::island::bar_layout::WEATHER].expect("有快照时天气徽标应紧跟时钟占位");
    assert!(chip>clock+22.0+6.9,"天气应在时钟右侧：时钟 {clock}，天气 {chip}");
    let clock_text=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()=="09:41"=>Some(t.visual_bounding_rect()),_=>None}).expect("常驻栏应有时钟");
    let temp=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()=="9°"=>Some(t.visual_bounding_rect()),_=>None}).expect("常驻栏应在时间后显示当前温度");
    assert!(temp.left()>clock_text.right()+4.0,"温度应与时钟留出间距：时钟右边 {:?}，温度左边 {:?}",clock_text.right(),temp.left());
    assert!(temp.center().y<30.0,"温度应与图标同一行");
    h.click(temp.center());
    assert_eq!(h.app.page(),8,"点温度也能打开天气页");
}

#[test]
fn weather_chip_hides_without_data_or_when_the_page_is_hidden(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_weather=false;
    for _ in 0..60{h.frame(vec![]);}
    assert!(h.app.resident.current().x[crate::island::bar_layout::WEATHER].is_none(),"没有快照时不占位");
    h.app.weather.snapshot=Some(crate::weather::Snapshot{condition:None,attributions:vec![],place:"杭州".into(),position:None,auto:false,temp:9.0,feels:None,humidity:None,wind:None,code:0,day:true,high:None,low:None,sunrise:None,sunset:None,sun_times:None,days:vec![],at:chrono::Utc::now().timestamp()});
    for _ in 0..60{h.frame(vec![]);}
    assert!(h.app.resident.current().x[crate::island::bar_layout::WEATHER].is_some(),"有快照时占位");
    h.app.settings.visible_pages[8]=false;
    for _ in 0..60{h.frame(vec![]);}
    assert!(h.app.resident.current().x[crate::island::bar_layout::WEATHER].is_none(),"天气页被隐藏后不占位");
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="9°")),"隐藏后不再画温度");
}

#[test]
fn tray_weather_appears_on_snapshot_change_and_hides_after_hold(){
    use crate::island::bar_layout::TRAY;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;
    for _ in 0..30{h.frame(vec![]);}
    assert!(h.app.resident.current().x[TRAY].is_none(),"没有事件时事件容器不占位");
    h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"杭州".into(),temp:9.0,code:0,day:true,..Default::default()});
    for _ in 0..20{h.frame(vec![]);}
    assert_eq!(h.app.tray_active(),Some(crate::island::event_tray::Item::Weather));
    assert!(h.app.resident.current().x[TRAY].is_some(),"天气更新后事件容器应占位");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="9°")),"容器里显示温度");
    h.t+=11.0;
    for _ in 0..3{h.frame(vec![]);}
    assert!(h.app.tray_active().is_none());
    assert!(h.app.resident.current().x[TRAY].is_none(),"事件窗口结束后容器收回");
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="9°")),"事件结束后不再显示天气");
}

#[test]
#[ignore="按需导出常驻天气徽标预览"]
fn weather_resident_bar_preview(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_WEATHER_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let mut h=H::new();h.app.settings.pet=false;
    h.app.weather.snapshot=Some(crate::weather::Snapshot{condition:None,attributions:vec![],place:"杭州".into(),position:None,auto:false,temp:9.0,feels:Some(5.6),humidity:Some(58),wind:Some(13.9),code:51,day:true,high:Some(15.2),low:Some(6.8),sunrise:Some("06:52".into()),sunset:Some("17:21".into()),sun_times:None,days:vec![],at:chrono::Utc::now().timestamp()});
    let mut raster=crate::preview_raster::Raster::new(1280,380);
    for frame in 0..90{
        let t=frame as f64/60.0;let env=Env{t,clock:"18:07:00".into(),battery:Some(80),charging:false,..Default::default()};
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(640.0,190.0))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{h.app.frame(ui,ui.max_rect(),&env);});});
        raster.update_textures(&output.textures_delta);
        if frame==89{
            raster.buf.fill([0.09,0.11,0.18,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            image::save_buffer(dest.join("weather-bar.png"),&rgba,1280,380,image::ColorType::Rgba8).unwrap();
        }
    }
    // 设置页「常驻排列」预览：天气徽标有独立位置。换一个 H，避免复用纹理缓存。
    let mut h=H::new();h.app.settings.pet=false;h.quota_accounts(1);
    h.app.weather.snapshot=Some(crate::weather::Snapshot{condition:None,attributions:vec![],place:"杭州".into(),position:None,auto:false,temp:9.0,feels:Some(5.6),humidity:Some(58),wind:Some(13.9),code:51,day:true,high:Some(15.2),low:Some(6.8),sunrise:Some("06:52".into()),sunset:Some("17:21".into()),sun_times:None,days:vec![],at:chrono::Utc::now().timestamp()});
    let mut raster=crate::preview_raster::Raster::new(1200,300);
    for frame in 0..3{
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(600.0,150.0))),time:Some(frame as f64),
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().show(ctx,|ui|{h.app.resident_order_settings(ui);});});
        raster.update_textures(&output.textures_delta);
        if frame==2{
            raster.buf.fill([0.13,0.13,0.15,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            image::save_buffer(dest.join("weather-bar-order.png"),&rgba,1200,300,image::ColorType::Rgba8).unwrap();
        }
        raster.free_textures(&output.textures_delta);
    }
}

#[test]
fn weather_icons_stay_bright_on_light_skins(){
    use crate::visuals::skin_paint::{map_color,CREAM,GLASS_LIGHT,GLASS_DARK};
    let luma=|c:egui::Color32|0.299*c.r() as f32+0.587*c.g() as f32+0.114*c.b() as f32;
    let cloud=egui::Color32::from_rgb(238,242,250);let sun=egui::Color32::from_rgb(255,196,72);
    for palette in [CREAM,GLASS_LIGHT]{
        // 通用换色会把白云反成深色、把太阳压成暗金；天气图标改用自己的浅色版配色。
        assert!(luma(map_color(cloud,&palette))<90.0);
        assert!(luma(crate::weather::ui::light_tone(cloud,&palette))>190.0,"浅色皮肤下的云应是淡蓝灰");
        let bright=crate::weather::ui::light_tone(sun,&palette);
        assert!(bright.r()>=250&&bright.g()>=160&&bright.b()<80,"太阳保持明亮暖黄：{bright:?}");
        let half=crate::weather::ui::light_tone(sun.gamma_multiply(0.5),&palette);assert_eq!(half.a(),sun.gamma_multiply(0.5).a(),"保留透明度");
    }
    assert_eq!(crate::weather::ui::light_tone(cloud,&GLASS_DARK),map_color(cloud,&GLASS_DARK),"深色皮肤不受影响");
}

#[test]
#[ignore="按需导出常驻天气的全部图标样式"]
fn weather_icon_atlas_preview(){
    use crate::{visuals::paint::{Card,H as Align},visuals::theme::{self,W},weather::{self,Snapshot}};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_WEATHER_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let cases=[("晴 · 白天",0,true,"0"),("晴 · 夜间",0,false,"0"),("晴间多云 · 白天",2,true,"1 / 2"),
        ("晴间多云 · 夜间",2,false,"1 / 2"),("阴天",3,true,"3"),("雾 / 雾凇",45,true,"45 / 48"),
        ("细雨 / 冻毛毛雨",51,true,"51 / 53 / 55 / 56 / 57"),("雨 / 冻雨",63,true,"61 / 63 / 65 / 66 / 67"),("阵雨",80,true,"80 / 81 / 82"),
        ("雪 / 阵雪",73,true,"71 / 73 / 75 / 77 / 85 / 86"),("雷雨 / 冰雹",95,true,"95 / 96 / 99"),("未知代码 · 回退云图标",200,true,"其他代码")];
    for (name,palette,ppp) in [("weather-icons-dark",None,2.0f32),("weather-icons-cream",Some(crate::visuals::skin_paint::CREAM),2.0),("weather-icons-glass-light",Some(crate::visuals::skin_paint::GLASS_LIGHT),2.0),("weather-icons-dark-1x",None,1.0),("weather-icons-dark-125",None,1.25),("weather-icons-dark-150",None,1.5)] {
        let (pw,ph)=((924.0*ppp) as usize,(700.0*ppp) as usize);
        let mut h=H::new();h.app.settings.pet=false;
        let mut layout=crate::island::bar_layout::Layout::new(false,0,false,false,false,false,true,false);layout.widths[crate::island::bar_layout::WEATHER]=50.0;
        let mut raster=crate::preview_raster::Raster::new(pw,ph);
        for frame in 0..4 {
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(924.0,700.0))),time:Some(frame as f64),
                viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
            let mut output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{
                let c=Card::new(ui,Pos2::ZERO,ui.max_rect());c.fill(0.0,0.0,924.0,700.0,0.0,egui::Color32::from_rgb(14,15,19));
                c.text(20.0,38.0,"常驻天气图标",22.0,W::Bold,theme::WHITE,Align::Left);
                c.text(20.0,61.0,"11类图标 · 顶部横排与侧边竖排 · 当前实际绘制样式",11.5,W::Regular,theme::GRAY,Align::Left);
                for (index,&(label,code,day,codes)) in cases.iter().enumerate(){
                    let tile=c.shifted(20.0+(index%3)as f32*298.0,84.0+(index/3)as f32*150.0);
                    tile.fill(0.0,0.0,286.0,138.0,11.0,egui::Color32::from_rgb(25,25,29));
                    tile.text(14.0,23.0,label,12.5,W::Semibold,theme::WHITE,Align::Left);
                    crate::weather::ui::icon(&tile,32.0,63.0,33.0,weather::kind(code,day));
                    let snapshot=Snapshot{place:"杭州".into(),temp:if code==73{-6.0}else{19.0},code,day,at:chrono::Utc::now().timestamp(),..Default::default()};
                    tile.fill(65.0,46.0,148.0,30.0,15.0,egui::Color32::BLACK);
                    let bar=tile.shifted(65.0,46.0);bar.text(32.0,21.0,"10:18",15.5,W::Bold,theme::WHITE,Align::Center);
                    h.app.paint_pill_weather(&bar,105.0,&layout,&snapshot);
                    tile.fill(234.0,40.0,36.0,47.0,18.0,egui::Color32::BLACK);
                    h.app.paint_side_weather(&tile.shifted(234.0,40.0),18.0,23.0,&snapshot);
                    for (x,label) in [(32.0,"放大"),(139.0,"顶部 15px"),(252.0,"侧边")]{tile.text(x,104.0,label,9.5,W::Regular,theme::GRAY,Align::Center);}
                    tile.text(14.0,125.0,&format!("WMO  {codes}"),8.5,W::Regular,theme::DIM,Align::Left);
                }
                c.text(20.0,690.0,"无数据、关闭天气或极简模式时不显示；超过半小时未更新时，温度转灰。",10.5,W::Regular,theme::GRAY,Align::Left);
                // 和岛上一样按图层换色，天气图标登记的浅色配色才会生效。
                if let Some(palette)=palette {let layer=ui.layer_id();crate::visuals::skin_paint::recolor_layer(ctx,layer,&[(0,crate::visuals::skin_paint::shape_count(ctx,layer))],&palette);}
            });});
            raster.update_textures(&output.textures_delta);
            if frame==3 {raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(format!("{name}.png")),&rgba,pw as u32,ph as u32,image::ColorType::Rgba8).unwrap();}
            raster.free_textures(&output.textures_delta);
        }
    }
}

#[test]
#[ignore="按需导出天气设置页，检查排版"]
fn weather_settings_preview(){
    use crate::island::settings_theme::SettingsTheme;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_WEATHER_SETTINGS_PREVIEW").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for (tag,theme,dropdown) in [("dark",SettingsTheme::Dark,false),("light",SettingsTheme::Light,false),("dropdown-dark",SettingsTheme::Dark,true),("dropdown-light",SettingsTheme::Light,true)] {
        let mut h=H::new();h.app.settings.weather_city="杭州".into();h.app.weather.city_input="杭州".into();h.app.weather.manual_city=true;h.app.settings.settings_theme=theme;
        h.ctx.style_mut(|style|style.animation_time=0.0);
        let current=crate::weather::Position{lat:30.27,lon:120.15,label:"杭州".into(),region:"浙江 · 中国".into()};
        h.app.weather.selected_city=Some(current.clone());h.app.weather.active_position=Some(current.clone());
        h.app.settings.weather_position=Some(current.clone());h.app.settings.weather_saved=vec![current.clone(),crate::weather::Position{lat:31.23,lon:121.47,label:"上海".into(),region:"上海 · 中国".into()}];
        h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"杭州".into(),temp:19.0,code:2,day:true,feels:Some(18.0),humidity:Some(58),wind:Some(12.0),high:Some(23.0),low:Some(15.0),
            sunrise:Some("06:52".into()),sunset:Some("17:21".into()),at:chrono::Utc::now().timestamp(),..Default::default()});
        h.app.weather.snapshot.as_mut().unwrap().position=Some(current);
        if dropdown {
            h.app.weather.city_input="佛山".into();
            h.app.weather.cities.seed("佛山",vec![
                crate::weather::Position{lat:23.02677,lon:113.13148,label:"佛山市".into(),region:"广东 · 中国".into()},
                crate::weather::Position{lat:28.73401,lon:98.80612,label:"佛山".into(),region:"迪庆藏族自治州 · 云南 · 中国".into()},
                crate::weather::Position{lat:30.04572,lon:106.41138,label:"佛山".into(),region:"重庆市 · 中国".into()},
            ]);
        }
        h.app.settings_tab=12;h.app.open_settings();h.ctx.set_embed_viewports(false);
        let children=std::sync::Arc::new(Mutex::new(Vec::new()));let capture=children.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(800.0,620.0))),
                viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
            capture.lock().unwrap().push(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let mut raster=crate::preview_raster::Raster::new(1600,1240);
        for frame in 0..4 {
            if dropdown && frame==1 {h.ctx.memory_mut(|m|m.open_popup(egui::Id::new("weather-city-options")));}
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(frame as f64),..Default::default()};
            let root=h.ctx.run(raw,|ctx|h.app.settings_ui(ctx,false));
            let child_frames:Vec<_>=children.lock().unwrap().drain(..).collect();
            for child in &child_frames {raster.update_textures(&child.textures_delta);}
            raster.update_textures(&root.textures_delta);
            for child in child_frames {
                if frame==3 {raster.buf.fill([0.1,0.1,0.1,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(format!("weather-settings-{tag}.png")),&rgba,1600,1240,image::ColorType::Rgba8).unwrap();}
                raster.free_textures(&child.textures_delta);
            }
            raster.free_textures(&root.textures_delta);
        }
        assert!(dest.join(format!("weather-settings-{tag}.png")).exists(),"必须导出真实设置窗口");
    }
}
