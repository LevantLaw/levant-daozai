//! 额度页、账号连接与额度环。
use super::*;

#[test]
fn gemini_connect_form_opens_default_browser_without_claiming_connection(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dark in [true,false]{
        let mut h=H::new();h.ctx.set_visuals(if dark{egui::Visuals::dark()}else{egui::Visuals::light()});
        h.app.settings_open=true;h.app.quota.provider=crate::quota::Provider::Gemini;h.app.quota.connect_open=true;
        let mut raster=crate::preview_raster::Raster::new(640,760);
        for _ in 0..4{
            let out=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(640.0,760.0))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|h.app.quota_settings(ui));});
            raster.update_textures(&out.textures_delta);h.shapes=out.shapes;
        }
        let texts=h.shapes.iter().filter_map(|s|if let egui::Shape::Text(t)=&s.shape{Some(t.galley.text())}else{None}).collect::<Vec<_>>();
        assert!(texts.contains(&"打开默认浏览器登录"),"Gemini 应在系统默认浏览器中登录");
        assert!(texts.contains(&"连接已登录的 Chrome"),"Gemini 应支持本人授权连接现有 Chrome 会话");
        assert!(texts.iter().any(|s|s.contains("chrome://inspect/#remote-debugging")),"首次连接应提供 Chrome 原生设置入口");
        assert!(!texts.iter().any(|s|s.contains("自动连接")),"打开官网不能表示岛仔已获得登录会话");
        assert!(!texts.iter().any(|s|s.contains("Kimi 浏览器扩展")),"Gemini 连接不应要求额外安装扩展");
        assert!(texts.contains(&"Google 邮箱"));
        let original=h.app.quota.config.clone();
        let point=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()=="打开默认浏览器登录"=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap();
        for pressed in [true,false]{
            let _=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(640.0,760.0))),events:vec![egui::Event::PointerMoved(point),egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}],..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|h.app.quota_settings(ui));});
        }
        assert!(h.app.pending.contains(&crate::island::Cmd::OpenUrl("https://gemini.google.com/usage".into())));
        assert!(h.app.quota.gemini_connecting.is_none());assert_eq!(h.app.quota.config,original);
        if let Some(dest)=std::env::var_os("DAOZAI_GEMINI_PREVIEW_DIR"){
            let dest=std::path::PathBuf::from(dest);std::fs::create_dir_all(&dest).unwrap();
            raster.buf.fill([0.08,0.09,0.12,1.0]);raster.draw(&h.ctx.tessellate(h.shapes.clone(),1.0),1.0);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            image::save_buffer(dest.join(if dark{"dark.png"}else{"light.png"}),&rgba,640,760,image::ColorType::Rgba8).unwrap();
        }
    }
}

#[test]
fn gemini_closed_dialogs_discard_completed_logins_without_saving(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for close_settings in [false,true]{
        let mut h=H::new();let original=h.app.quota.config.clone();h.app.settings_open=true;h.app.quota.connect_open=true;
        let (connect,tx)=crate::quota::gemini::Connect::pending();h.app.quota.gemini_connecting=Some(connect);
        tx.send(Ok(crate::quota::gemini::simulated_connected())).unwrap();
        if close_settings{h.app.settings_open=false;}else{h.app.quota_close_connection();}
        h.app.quota_tick();assert!(h.app.quota.gemini_connecting.is_none());assert_eq!(h.app.quota.config,original);
        assert!(h.app.quota.rows.is_empty());assert!(!crate::model::config_dir().join("credentials").exists(),"关闭后不能保存晚到的会话");
    }
}

#[test]
fn resident_quota_moves_left_and_releases_battery_space(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_quota=false;
    h.power=(Some(80),true);h.quota_accounts(1);
    for _ in 0..90 {h.frame(vec![]);}
    let before=h.quota_centers()[0];let width=h.app.current_width();assert!(before>WIN_W/2.0);
    h.app.settings.show_battery=false;for _ in 0..90 {h.frame(vec![]);}
    let after=h.quota_centers()[0];assert!(after<WIN_W/2.0,"关闭电量后，额度应补到时间左侧");
    assert!(h.app.current_width()<width-20.0,"关闭组件后应释放实际占位");
    h.app.settings.show_battery=true;h.power=(None,false);for _ in 0..90 {h.frame(vec![]);}
    assert!((h.quota_centers()[0]-after).abs()<0.2,"无电池设备应采用相同补位布局");
    h.click(Pos2::new(after,15.0));assert_eq!(h.app.page(),6,"补位后仍可点击额度圆环");
}

#[test]
fn resident_quotas_coexist_with_music_and_focus(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_quota=false;h.app.settings.tray_focus=false;h.quota_accounts(3);
    h.media=crate::music::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    h.app.pomo.running=true;h.app.pomo.remaining=600.0;
    for _ in 0..90 {h.frame(vec![]);}
    assert_eq!(h.quota_centers().len(),3,"播放和专注状态不应挤掉常驻额度");
    assert!(h.app.current_width()<PW,"多组件应在单行内排布");
    h.app.set_minimal(true);for _ in 0..90 {h.frame(vec![]);}
    assert!(h.quota_centers().is_empty());assert!(h.region.height()<20.0,"仍须尊重极简开关");
}

#[test]
fn quota_text_stays_inside_the_inner_ring_at_zero_and_full_usage() {
    use crate::quota::{Event,Snapshot,Window};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dual in [true,false] {for remaining in [72.0,0.0,100.0] {
        let mut h=H::new();h.quota_accounts(1);
        let reset=Some((chrono::Local::now()+chrono::Duration::days(400)).timestamp());
        let mut windows=vec![Window{label:"本周".into(),seconds:Some(604800),remaining,reset,amount:None}];
        if dual {windows.insert(0,Window{label:"5 小时".into(),seconds:Some(18000),remaining,reset,amount:None});}
        h.app.quota_event(Event{id:"preview-0".into(),result:Ok(Snapshot{windows,..Default::default()})});
        h.app.show_page(6);h.hover(H::local(300.0,60.0),1.0);
        let center=H::local(103.0,105.0);
        let inner=h.shapes.iter().filter_map(|shape|match &shape.shape {
            egui::Shape::Circle(circle) if circle.center.distance(center)<0.5 && circle.radius>15.0 && circle.stroke.width>0.0=>Some(circle.radius-circle.stroke.width/2.0),
            egui::Shape::Path(path) if path.points.len()>2 && path.stroke.width>0.0=>{
                let radius=path.points[0].distance(center);
                ((15.0..35.0).contains(&radius) && path.points.iter().all(|point|(point.distance(center)-radius).abs()<0.5)).then_some(radius-path.stroke.width/2.0)
            },
            _=>None,
        }).reduce(f32::min).unwrap_or(35.0);
        for shape in &h.shapes {if let egui::Shape::Text(text)=&shape.shape {
            let rect=text.visual_bounding_rect();
            if rect.center().distance(center)<35.0 {
                for corner in [rect.left_top(),rect.right_top(),rect.left_bottom(),rect.right_bottom()] {
                    assert!(corner.distance(center)+0.5<inner,"{} 的文字不得碰到额度环，双周期={dual}，剩余={remaining}",text.galley.text());
                }
            }
        }}
        let texts:Vec<_>=h.shapes.iter().filter_map(|shape|match &shape.shape {
            egui::Shape::Text(text) if text.visual_bounding_rect().left()>=H::local(157.0,0.0).x && text.visual_bounding_rect().top()>=68.0 && text.visual_bounding_rect().bottom()<=144.0=>Some((text.galley.text().to_owned(),text.visual_bounding_rect())),_=>None,
        }).collect();
        for (i,(a,rect)) in texts.iter().enumerate(){for (b,other) in &texts[i+1..]{assert!(!rect.intersects(*other),"{a} 与 {b} 不得重叠");}}
        let label=if dual {"5 小时"}else{"本周剩余"};
        assert!(h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()==label)),"周期说明应完整可见");
    }}
}

#[test]
fn kiro_cancelled_or_closed_connections_cannot_save_late_results(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for close_settings in [false,true]{
        let mut h=H::new();let saved=h.app.quota.config.clone();h.app.settings_open=true;h.app.quota.connect_open=true;
        let(connect,tx)=crate::quota::kiro::Connect::pending(None);h.app.quota.kiro_connecting=Some(connect);
        if close_settings{h.app.settings_open=false;}else{h.app.quota.connect_open=false;}
        h.app.quota_tick();assert!(h.app.quota.kiro_connecting.as_ref().unwrap().cancelled());
        tx.send(Ok(crate::quota::kiro::Connected{source:"kiro.json".into(),label:"Kiro".into(),snapshot:kiro_snapshot(900.0,1000.0)})).unwrap();
        h.app.quota_tick();assert_eq!(h.app.quota.config,saved);assert!(h.app.quota.kiro_connecting.is_none());assert!(h.app.quota.rows.is_empty());
    }
}

#[test]
fn kiro_connection_save_failure_keeps_the_original_account_and_cache(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings_open=true;h.app.quota.connect_open=true;
    let snapshot=kiro_snapshot(900.0,1000.0);let owner=snapshot.kiro.as_ref().unwrap().owner.clone();
    let account=crate::quota::Account{id:"kiro".into(),provider:crate::quota::Provider::Kiro,alias:"工作".into(),source:"old.json".into(),slot:Some(owner),enabled:true,pinned:true};
    h.app.quota.config.accounts.push(account.clone());h.app.quota.rows.insert(account.id.clone(),Ok(snapshot.clone()));let saved=h.app.quota.config.clone();
    let(connect,tx)=crate::quota::kiro::Connect::pending(Some(account));h.app.quota.kiro_connecting=Some(connect);
    tx.send(Ok(crate::quota::kiro::Connected{source:"new.json".into(),label:"新备注".into(),snapshot})).unwrap();
    let original=std::env::var_os("DAOZAI_DATA_DIR").unwrap();std::fs::create_dir_all(&original).unwrap();let blocked=std::path::PathBuf::from(&original).join("kiro-blocked");std::fs::write(&blocked,"blocked").unwrap();std::env::set_var("DAOZAI_DATA_DIR",&blocked);
    h.app.quota_tick();std::env::set_var("DAOZAI_DATA_DIR",original);
    assert_eq!(h.app.quota.config,saved);assert!(h.app.quota.note.contains("配置目录"));assert_eq!(h.app.quota.rows["kiro"].as_ref().unwrap().windows[0].amount.as_ref().unwrap().remaining,900.0);
}

#[test]
fn kiro_ring_uses_logo_for_all_point_balances_and_skins(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for skin in crate::model::Skin::ALL{for remaining in [0.0,0.01,99.9,1000.0,10000.0]{
        let mut h=H::new();h.app.settings.skin=skin;let snapshot=kiro_snapshot(remaining,10000.0);let full=format!("{} 点",snapshot.windows[0].value_text());
        h.app.quota.config.accounts.push(crate::quota::Account{id:"kiro".into(),provider:crate::quota::Provider::Kiro,alias:"Kiro".into(),source:"kiro.json".into(),slot:None,enabled:true,pinned:true});h.app.quota.config.selected="kiro".into();
        h.app.quota_event(crate::quota::Event{id:"kiro".into(),result:Ok(snapshot)});h.app.show_page(6);h.hover(H::local(300.0,60.0),0.7);
        let find=|wanted:&str|h.shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Text(t)if t.galley.text()==wanted=>Some(t.visual_bounding_rect()),_=>None}).unwrap_or_else(||panic!("缺少文本 {wanted}"));
        let center=H::local(103.0,105.0);
        let logo=h.shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Mesh(mesh)=>{let rect=mesh.calc_bounds();(rect.center().distance(center)<0.8&&(30.0..36.0).contains(&rect.width())).then_some(rect)},_=>None}).expect("圆环中心应显示 Kiro 标识");
        for corner in [logo.left_top(),logo.right_top(),logo.left_bottom(),logo.right_bottom()]{assert!(corner.distance(center)<27.5,"{skin:?} 的标识应位于圆环内");}
        assert!(!find("月度额度剩余").intersects(find(&full)),"{skin:?} {remaining} 点的标签重叠");
    }}
}

#[test]
fn kiro_logo_and_point_details_fit_the_card(){
    use crate::quota::{Amount,Event,Provider,Snapshot,Window};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.quota.config.accounts.push(crate::quota::Account{id:"kiro".into(),provider:Provider::Kiro,alias:"Kiro".into(),source:"kiro.json".into(),slot:None,enabled:true,pinned:true});
    h.app.quota.config.selected="kiro".into();
    let reset=Some((chrono::Local::now()+chrono::Duration::days(20)).timestamp());
    h.app.quota_event(Event{id:"kiro".into(),result:Ok(Snapshot{kiro:kiro_snapshot(904.09,1000.0).kiro,plan:"KIRO PRO".into(),at:chrono::Utc::now().timestamp(),
        windows:vec![Window{label:"月度额度".into(),seconds:None,remaining:90.4,reset,amount:Some(Amount::new(904.09,1000.0))}],..Default::default()})});
    h.app.show_page(6);h.hover(H::local(300.0,60.0),1.0);
    let center=H::local(103.0,105.0);
    let inner=h.shapes.iter().filter_map(|shape|match &shape.shape{
        egui::Shape::Circle(circle) if circle.center.distance(center)<0.5 && circle.radius>15.0 && circle.stroke.width>0.0=>Some(circle.radius-circle.stroke.width/2.0),
        egui::Shape::Path(path) if path.points.len()>2 && path.stroke.width>0.0=>{let radius=path.points[0].distance(center);((15.0..35.0).contains(&radius) && path.points.iter().all(|point|(point.distance(center)-radius).abs()<0.5)).then_some(radius-path.stroke.width/2.0)},
        _=>None}).reduce(f32::min).unwrap_or(35.0);
    let text=|wanted:&str|h.shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Text(text)if text.galley.text()==wanted=>Some(text.visual_bounding_rect()),_=>None});
    let logo=h.shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Mesh(mesh)=>{let rect=mesh.calc_bounds();(rect.center().distance(center)<0.8&&(30.0..36.0).contains(&rect.width())).then_some(rect)},_=>None}).expect("圆环中心应显示 Kiro 标识");
    for corner in [logo.left_top(),logo.right_top(),logo.left_bottom(),logo.right_bottom()]{assert!(corner.distance(center)+0.5<inner,"标识应位于额度环内");}
    let center_area=Rect::from_center_size(center,Vec2::splat(40.0));
    assert!(!h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(t) if center_area.intersects(t.visual_bounding_rect()))),"圆环中心应只显示 Kiro 标识");
    let label=text("月度额度剩余").expect("周期说明");let value=text("904.1 / 1000 点").expect("剩余量与总额");
    assert!(!label.intersects(value),"说明与数值不得重叠");
    assert!(h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()=="Kiro · KIRO PRO")),"套餐名应显示");
    if let Some(dest)=std::env::var_os("DAOZAI_KIRO_PREVIEW_DIR"){
        let mut raster=crate::preview_raster::Raster::new((WIN_W*2.0) as usize,(WIN_H*2.0) as usize);
        let t=h.t+1.0/60.0;
        let env=Env{t,clock:"09:41:00".into(),today:"2026-10-31".into(),date:"10月31日 周六".into(),battery:h.power.0,charging:h.power.1,media:h.media.clone(),bt_ready:true,..Default::default()};
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H))),time:Some(t),
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=ui.max_rect();h.app.frame(ui,win,&env);});});
        raster.update_textures(&output.textures_delta);
        raster.buf.fill([0.09,0.11,0.18,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
        image::save_buffer(std::path::PathBuf::from(&dest).join("kiro-credits.png"),&rgba,raster.w as u32,raster.h as u32,image::ColorType::Rgba8).unwrap();
        raster.free_textures(&output.textures_delta);
    }
}

#[test]
fn kiro_credits_follow_the_ring_on_the_resident_bar(){
    use crate::quota::{Amount,Event,Provider,Snapshot,Window};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_quota=false;
    h.app.quota.config.accounts.push(crate::quota::Account{id:"kiro".into(),provider:Provider::Kiro,alias:"Kiro".into(),source:"kiro.json".into(),slot:None,enabled:true,pinned:true});
    h.app.quota.config.selected="kiro".into();
    h.app.quota_event(Event{id:"kiro".into(),result:Ok(Snapshot{kiro:kiro_snapshot(904.09,1000.0).kiro,plan:"KIRO PRO".into(),at:chrono::Utc::now().timestamp(),
        windows:vec![Window{label:"月度额度".into(),seconds:None,remaining:90.4,reset:None,amount:Some(Amount::new(904.09,1000.0))}],..Default::default()})});
    for _ in 0..60{h.frame(vec![]);}
    assert_eq!(h.app.quota_bar_indices().len(),1,"点数的 Kiro 账号应常驻");
    let slot=crate::island::layout::bar::quota_slot(0);
    let layout=h.app.resident.current();
    let extra=layout.widths[slot]-27.0;
    assert!(extra>10.0,"环右侧要为点数让出位置：{:?}",layout.widths[slot]);
    let ring=WIN_W/2.0+layout.x[slot].unwrap()-extra/2.0;
    let text=h.shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Text(t)if t.galley.text()=="904"=>Some(t.visual_bounding_rect()),_=>None}).expect("常驻栏应在环右侧显示剩余点数");
    assert!(text.center().y<30.0,"点数应与图标同一行");
    assert!((text.left()-ring-15.5).abs()<2.5,"点数紧跟环右侧：ring={ring} text={:?}",text);
    h.click(text.center());
    assert_eq!(h.app.page(),6,"点数字也能打开额度页");
}

#[test]
#[ignore="按需导出 Kiro 积分池、万元额套餐及设置明细"]
fn kiro_credit_pools_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_KIRO_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for(skin,name)in [(crate::model::Skin::Classic,"dark"),(crate::model::Skin::GlassLight,"light")]{for pools in [false,true]{
        let mut h=H::new();h.app.settings.skin=skin;let mut snapshot=kiro_snapshot(10000.0,10000.0);
        if pools{snapshot=crate::quota::parse(crate::quota::Provider::Kiro,&serde_json::json!({"userInfo":{"userId":"ui-kiro-user"},"subscriptionInfo":{"subscriptionTitle":"KIRO PRO"},"usageBreakdownList":[{"resourceType":"CREDIT","usageLimitWithPrecision":1000,"currentUsageWithPrecision":1000,"nextDateReset":1796083200_i64,"freeTrialInfo":{"freeTrialStatus":"ACTIVE","usageLimit":500,"currentUsage":100,"freeTrialExpiry":1827619200_i64},"bonuses":[{"status":"ACTIVE","displayName":"加购点数","usageLimit":125,"currentUsage":25,"expiresAt":1827619200_i64}]}]})).unwrap();}
        let id="kiro".to_owned();h.app.quota.config.accounts.push(crate::quota::Account{id:id.clone(),provider:crate::quota::Provider::Kiro,alias:"Kiro".into(),source:"kiro.json".into(),slot:None,enabled:true,pinned:true});h.app.quota.config.selected=id.clone();h.app.quota_event(crate::quota::Event{id,result:Ok(snapshot)});h.app.show_page(6);
        let mut raster=feature_raster();shoot_feature(&mut h,&mut raster,&dest,&format!("kiro-{name}-{}",if pools{"pools"}else{"power"}),75,Some(H::local(300.0,60.0)),"2026-11-11");
        if pools{
            h.app.settings.settings_theme=if name=="light"{crate::island::settings_theme::SettingsTheme::Light}else{crate::island::settings_theme::SettingsTheme::Dark};h.app.settings_tab=11;h.app.open_settings();h.ctx.set_embed_viewports(false);
            let size=egui::vec2(650.0,650.0);let ppp=1.5;let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
            egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{let id=viewport.ids.this;let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,size)),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};*capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));});
            let mut raster=crate::preview_raster::Raster::new(975,975);
            for frame in 0..6{let root=h.ctx.run(egui::RawInput::default(),|ctx|h.app.settings_ui(ctx,false));raster.update_textures(&root.textures_delta);let child=output.lock().unwrap().take().unwrap();raster.update_textures(&child.textures_delta);
                if frame==5{raster.buf.fill([0.1,0.1,0.1,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(format!("kiro-{name}-settings.png")),&rgba,975,975,image::ColorType::Rgba8).unwrap();}raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
            }
        }
    }}
}

#[test]
#[ignore = "按需导出 Kiro 常驻栏预览"]
fn kiro_resident_bar_preview(){
    use crate::quota::{Amount,Event,Provider,Snapshot,Window};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_KIRO_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.show_battery=true;h.quota_accounts(1);
    h.app.quota.config.accounts.push(crate::quota::Account{id:"kiro".into(),provider:Provider::Kiro,alias:"Kiro".into(),source:"kiro.json".into(),slot:None,enabled:true,pinned:true});
    h.app.quota_event(Event{id:"kiro".into(),result:Ok(Snapshot{kiro:kiro_snapshot(904.09,1000.0).kiro,plan:"KIRO PRO".into(),at:chrono::Utc::now().timestamp(),
        windows:vec![Window{label:"月度额度".into(),seconds:None,remaining:90.4,reset:None,amount:Some(Amount::new(904.09,1000.0))}],..Default::default()})});
    let wb=h.app.quota.config.connect_workbuddy("preview".into(),"WorkBuddy".into());
    h.app.quota_event(Event{id:wb,result:crate::quota::parse(Provider::WorkBuddy,&serde_json::json!({"code":0,"data":{"Packages":[{"CycleRemainCapacity":"1970"}],"CreditGroups":[{"Kind":"plan_base","CycleRemainCapacity":"1970"}]}}))});
    let mut raster=crate::preview_raster::Raster::new(1280,380);
    for frame in 0..90{
        let t=frame as f64/60.0;let env=Env{t,clock:"12:17:00".into(),battery:Some(80),charging:true,..Default::default()};
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(640.0,190.0))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{h.app.frame(ui,ui.max_rect(),&env);});});
        raster.update_textures(&output.textures_delta);
        if frame==89{
            raster.buf.fill([0.09,0.11,0.18,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            image::save_buffer(dest.join("kiro-bar.png"),&rgba,1280,380,image::ColorType::Rgba8).unwrap();
        }
    }
}

#[test]
#[ignore="按需导出 Kiro 模拟数据与连接状态界面，不读取真实账号"]
fn kiro_simulated_states_gallery(){
    use crate::quota::{Account,Event,Provider};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_KIRO_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let at=chrono::DateTime::parse_from_rfc3339("2026-11-11T01:41:00Z").unwrap().timestamp();
    let expires=chrono::DateTime::parse_from_rfc3339("2027-11-01T00:00:00Z").unwrap().timestamp();
    for(skin,theme)in [(crate::model::Skin::Classic,"dark"),(crate::model::Skin::GlassLight,"light")]{
        for scene in ["monthly","pools","overage","small","power","loading","expired","stale","details","connecting","cancelled","identity"]{
            let mut h=H::new();h.app.settings.skin=skin;h.app.settings.pet=false;
            let mut value=serde_json::json!({"userInfo":{"userId":"simulation-user-a"},"subscriptionInfo":{"subscriptionTitle":"KIRO PRO"},"usageBreakdownList":[{"resourceType":"CREDIT","usageLimitWithPrecision":1000.0,"currentUsageWithPrecision":95.91,"nextDateReset":1796083200_i64}]});
            if matches!(scene,"pools"|"details"){
                value["usageBreakdownList"][0]["currentUsageWithPrecision"]=serde_json::json!(1000.0);
                value["usageBreakdownList"][0]["freeTrialInfo"]=serde_json::json!({"freeTrialStatus":"ACTIVE","usageLimit":500,"currentUsage":100,"freeTrialExpiry":expires});
                value["usageBreakdownList"][0]["bonuses"]=serde_json::json!([{"status":"ACTIVE","displayName":"加购点数","usageLimit":125,"currentUsage":25,"expiresAt":expires}]);
            }else if scene=="overage"{
                value["usageBreakdownList"][0]["currentUsageWithPrecision"]=serde_json::json!(1080.0);
                value["usageBreakdownList"][0]["currentOveragesWithPrecision"]=serde_json::json!(80.0);
                value["usageBreakdownList"][0]["overageCapWithPrecision"]=serde_json::json!(200.0);
                value["overageConfiguration"]=serde_json::json!({"overageStatus":"ENABLED"});
            }else if scene=="small"{value["usageBreakdownList"][0]["currentUsageWithPrecision"]=serde_json::json!(999.99);}
            else if scene=="power"{value["subscriptionInfo"]["subscriptionTitle"]=serde_json::json!("KIRO POWER");value["usageBreakdownList"][0]["usageLimitWithPrecision"]=serde_json::json!(10000.0);value["usageBreakdownList"][0]["currentUsageWithPrecision"]=serde_json::json!(0.0);}
            let mut snapshot=crate::quota::parse(Provider::Kiro,&value).unwrap();snapshot.at=at;
            let id="kiro-simulation".to_owned();let owner=snapshot.kiro.as_ref().unwrap().owner.clone();
            h.app.quota.config.accounts.push(Account{id:id.clone(),provider:Provider::Kiro,alias:"Kiro".into(),source:"simulation-kiro.json".into(),slot:Some(owner),enabled:true,pinned:true});h.app.quota.config.selected=id.clone();
            if !matches!(scene,"loading"|"expired"|"connecting"|"cancelled"){
                h.app.quota_event(Event{id:id.clone(),result:Ok(snapshot)});
            }
            match scene{
                "loading"=>{h.app.quota.busy.insert(id.clone());},
                "expired"=>h.app.quota_event(Event{id:id.clone(),result:Err("Kiro 登录已过期，请打开 Kiro 重新登录后刷新额度".into())}),
                "stale"=>{h.app.quota_event(Event{id:id.clone(),result:Err("无法连接 Kiro 额度服务，请检查网络或代理".into())});assert_eq!(h.app.quota.rows[&id].as_ref().unwrap().at,at);assert!(h.app.quota.stale.contains_key(&id));},
                "identity"=>{
                    let account=h.app.quota.config.accounts[0].clone();let mut other=value.clone();other["userInfo"]["userId"]=serde_json::json!("simulation-user-b");let other=crate::quota::parse(Provider::Kiro,&other).unwrap();
                    let error=crate::quota::kiro::check_owner(&account,&other).unwrap_err();h.app.quota_event(Event{id:id.clone(),result:Err(error)});
                    assert_eq!(h.app.quota.rows[&id].as_ref().unwrap().kiro.as_ref().unwrap().owner,account.slot.unwrap());
                },
                _=>{}
            }
            let settings=matches!(scene,"details"|"connecting"|"cancelled"|"identity");
            if settings{
                h.app.settings.settings_theme=if theme=="light"{crate::island::settings_theme::SettingsTheme::Light}else{crate::island::settings_theme::SettingsTheme::Dark};h.app.settings_tab=11;h.app.open_settings();h.ctx.set_embed_viewports(false);
                let _pending=if matches!(scene,"connecting"|"cancelled"){
                    h.app.quota.config.accounts.clear();h.app.quota.provider=Provider::Kiro;h.app.quota.alias="个人账号".into();h.app.quota.connect_open=true;
                    let(mut connect,tx)=crate::quota::kiro::Connect::pending(None);if scene=="cancelled"{connect.cancel();}h.app.quota.kiro_connecting=Some(connect);Some(tx)
                }else{None};
                let size=egui::vec2(690.0,620.0);let ppp=1.5;let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
                egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{let id=viewport.ids.this;let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,size)),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};*capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));});
                let mut raster=crate::preview_raster::Raster::new(1035,930);
                for frame in 0..8{
                    let root=h.ctx.run(egui::RawInput::default(),|ctx|h.app.settings_ui(ctx,false));raster.update_textures(&root.textures_delta);let child=output.lock().unwrap().take().unwrap();raster.update_textures(&child.textures_delta);
                    if frame==7{
                        if scene=="identity"{let count=child.shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)=>Some(t.galley.text().matches("月度额度").count()),_=>None}).sum::<usize>();assert_eq!(count,1,"普通套餐只显示一行月度额度");}
                        raster.buf.fill([0.20,0.24,0.32,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(format!("{theme}-{scene}.png")),&rgba,1035,930,image::ColorType::Rgba8).unwrap();}
                    raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
                }
            }else{
                h.app.show_page(6);let mut raster=feature_raster();
                for frame in 0..75{
                    h.t+=1.0/60.0;let cursor=Some(H::local(300.0,60.0));
                    let env=Env{t:h.t,clock:"09:41:00".into(),today:"2026-11-11".into(),date:"11月11日 周三".into(),cursor,bt_ready:true,..Default::default()};
                    let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(h.t),events:vec![egui::Event::PointerMoved(cursor.unwrap())],viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
                    let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H)),&env);});});raster.update_textures(&output.textures_delta);
                    if frame==74{
                        let texts=output.shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)=>Some(t.galley.text()),_=>None}).collect::<Vec<_>>().join(" ");
                        if scene=="small"{assert!(texts.contains("0.01"));}if scene=="loading"{assert!(texts.contains("正在更新"));}if scene=="expired"{assert!(texts.contains("登录已过期"));}
                        raster.buf.fill([0.20,0.24,0.32,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                        let image=image::RgbaImage::from_raw(raster.w as u32,raster.h as u32,rgba).unwrap();image::imageops::crop_imm(&image,200,0,880,380).to_image().save(dest.join(format!("{theme}-{scene}.png"))).unwrap();
                    }raster.free_textures(&output.textures_delta);
                }
            }
        }
    }
}

#[test]
#[ignore="按需导出 Kiro 常驻刘海岛和胶囊岛，不读取真实账号"]
fn kiro_resident_states_gallery(){
    use crate::quota::{Account,Event,Provider};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_KIRO_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for(skin,theme)in [(crate::model::Skin::Classic,"dark"),(crate::model::Skin::GlassLight,"light")]{for mixed in [false,true]{
        let mut h=H::new();h.app.settings.skin=skin;h.app.settings.pet=false;h.app.settings.show_battery=true;
        h.app.settings.island_shape=if mixed{crate::model::IslandShape::Pill}else{crate::model::IslandShape::Notch};
        if mixed{h.quota_accounts(1);}
        let id="kiro-resident".to_owned();let snapshot=kiro_snapshot(904.09,1000.0);
        h.app.quota.config.accounts.push(Account{id:id.clone(),provider:Provider::Kiro,alias:"Kiro".into(),source:"simulation-kiro.json".into(),slot:Some(snapshot.kiro.as_ref().unwrap().owner.clone()),enabled:true,pinned:true});
        h.app.quota_event(Event{id:id.clone(),result:Ok(snapshot)});
        if mixed{let wb=h.app.quota.config.connect_workbuddy("preview".into(),"WorkBuddy".into());h.app.quota_event(Event{id:wb,result:crate::quota::parse(Provider::WorkBuddy,&serde_json::json!({"code":0,"data":{"Packages":[{"CycleRemainCapacity":"1970"}],"CreditGroups":[{"Kind":"plan_base","CycleRemainCapacity":"1970"}]}}))});}
        h.app.quota.config.selected=id.clone();let mut raster=crate::preview_raster::Raster::new(1280,380);
        for frame in 0..90{
            let t=frame as f64/60.0;let env=Env{t,clock:"12:17:00".into(),today:"2026-11-11".into(),date:"11月11日 周三".into(),battery:Some(80),charging:true,..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(640.0,190.0))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{h.app.frame(ui,ui.max_rect(),&env);});});raster.update_textures(&output.textures_delta);
            if frame==89{
                assert_eq!(h.app.quota_bar_indices().len(),if mixed{3}else{1});
                assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="904")),"常驻栏应显示剩余点数");
                raster.buf.fill([0.20,0.24,0.32,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                let image=image::RgbaImage::from_raw(1280,380,rgba).unwrap();let width=(h.app.current_width()*2.0+72.0).ceil().clamp(300.0,1280.0) as u32;
                image::imageops::crop_imm(&image,(1280-width)/2,0,width,130).to_image().save(dest.join(format!("{theme}-{}.png",if mixed{"residentmixed"}else{"resident"}))).unwrap();
            }raster.free_textures(&output.textures_delta);
        }
    }}
}

#[test]
fn quota_update_status_is_visible_for_every_provider_and_keeps_failed_data(){
    use crate::quota::{Provider,Event,Snapshot,Window};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let at=chrono::Utc::now().timestamp()-60;
    let time=chrono::DateTime::from_timestamp(at,0).unwrap().with_timezone(&chrono::Local).format("%H:%M").to_string();
    for provider in Provider::ALL{
        let mut h=H::new();h.quota_accounts(1);h.app.quota.config.accounts[0].provider=provider;
        let mut snapshot=match provider{
            Provider::Kiro=>kiro_snapshot(720.0,1000.0),
            Provider::DeepSeek=>crate::quota::parse(provider,&serde_json::json!({"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"4.18","granted_balance":"0","topped_up_balance":"4.18"}]})).unwrap(),
            Provider::WorkBuddy=>crate::quota::parse(provider,&serde_json::json!({"code":0,"data":{"Packages":[{"CycleRemainCapacity":"2000"}]}})).unwrap(),
            _=>Snapshot{windows:vec![Window{label:"5 小时".into(),seconds:Some(18000),remaining:72.0,reset:None,amount:None},Window{label:"本周".into(),seconds:Some(604800),remaining:44.0,reset:None,amount:None}],..Default::default()},
        };
        snapshot.at=at;h.app.quota_event(Event{id:"preview-0".into(),result:Ok(snapshot)});
        h.app.show_page(6);
        if let Some(dest)=std::env::var_os("DAOZAI_QUOTA_STATUS_PREVIEW_DIR"){
            let dest=std::path::PathBuf::from(dest);std::fs::create_dir_all(&dest).unwrap();let mut raster=feature_raster();
            shoot_feature(&mut h,&mut raster,&dest,&format!("quota-{}",provider.name()),90,Some(H::local(300.0,60.0)),"2026-11-07");
        }
        h.hover(H::local(300.0,60.0),1.0);
        assert_eq!(h.app.expanded_height(),176.0+crate::island::STATUS_H,"{provider:?} 额度页应采用同宽加高的布局");
        assert!((h.app.current_width()-PW).abs()<0.1,"{provider:?} 额度页应保持原有宽度：{}",h.app.current_width());
        let expected=if provider==Provider::Claude{format!("本地记录 · {time}")}else if provider==Provider::DeepSeek{format!("CNY · {time} 更新")}else{format!("{time} 更新")};
        let status=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==expected=>Some(t.visual_bounding_rect()),_=>None}).unwrap_or_else(||panic!("{provider:?} 缺少更新时间：{expected}"));
        assert!(status.bottom()+6.0<=H::local(0.0,h.app.expanded_height()-crate::island::STATUS_H-9.2).y,"{provider:?} 更新时间 {status:?} 应与分页圆点留出间距");
        for shape in &h.shapes{if let egui::Shape::Text(text)=&shape.shape{
            if !text.galley.text().is_empty()&&text.galley.text()!=expected&&text.visual_bounding_rect().left()>=H::local(125.0,0.0).x{
                assert!(shape.clip_rect.contains(text.visual_bounding_rect().center()),"{provider:?} 的 {} 应完整显示在面板内",text.galley.text());
                assert!(status.top()-text.visual_bounding_rect().bottom()>=8.0,"{provider:?} 的更新时间 {status:?} 应与 {} {:?} 留出独立行距",text.galley.text(),text.visual_bounding_rect());
            }
        }}
        h.app.quota.busy.insert("preview-0".into());h.frame(vec![]);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains(&time)&&t.galley.text().contains(if provider==Provider::Claude{"读取中"}else{"正在更新"}))),"刷新中仍须保留已有记录时间");
        h.app.quota_event(Event{id:"preview-0".into(),result:Err("连接超时".into())});h.frame(vec![]);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==format!("上次记录 · {time}"))),"{provider:?} 刷新失败后应明确旧数据时间");
        assert_eq!(h.app.quota.rows["preview-0"].as_ref().unwrap().at,at,"刷新失败不能刷新记录时间");
    }
}

#[test]
fn quota_page_navigation_tracks_the_taller_panel(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.quota_accounts(1);
    h.app.show_page(6);h.hover(H::local(300.0,60.0),0.8);
    assert!((h.hit.height()-208.0).abs()<0.2,"额度页新增的底部空间应属于命中区域");
    let dot=h.shapes.iter().find_map(|shape|match &shape.shape{
        egui::Shape::Rect(rect) if (rect.rect.width()-4.0).abs()<0.1&&(rect.rect.height()-4.0).abs()<0.1&&(rect.rect.center().y-198.8).abs()<0.1=>Some(rect.rect.center()),_=>None,
    }).expect("分页圆点应随加高的面板下移");
    h.click(dot);assert_ne!(h.app.page,6,"新增底部区域中的分页应可点击");
    h.app.show_page(6);h.hover(H::local(300.0,60.0),0.8);
    h.click(H::local(21.0,88.0));assert_eq!(h.app.page,5);
    h.hover(H::local(300.0,60.0),0.8);assert_eq!(h.app.expanded_height(),PH+crate::island::STATUS_H);
    h.click(H::local(398.0,68.0));assert_eq!(h.app.page,6);
    h.hover(H::local(300.0,60.0),0.8);
    h.click(H::local(398.0,88.0));assert_eq!(h.app.page,crate::island::CALENDAR_PAGE);
}

#[test]
fn quota_update_status_marks_previous_dates_and_never_shows_epoch_time(){
    use crate::quota::{Event,Snapshot,Window};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.quota_accounts(1);
    let at=chrono::Local::now()-chrono::Duration::days(1);
    let snapshot=Snapshot{at:at.timestamp(),windows:vec![Window{label:"本周".into(),seconds:Some(604800),remaining:44.0,reset:None,amount:None}],..Default::default()};
    h.app.quota_event(Event{id:"preview-0".into(),result:Ok(snapshot.clone())});h.app.show_page(6);h.hover(H::local(300.0,60.0),0.6);
    let expected=format!("{} 更新",at.format("%m/%d %H:%M"));
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==expected)),"跨天记录应包含日期");
    h.app.quota_event(Event{id:"preview-0".into(),result:Ok(Snapshot{at:0,..snapshot})});h.frame(vec![]);
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains("1970")||t.galley.text().ends_with(" 更新"))),"缺少时间时不能伪造更新时间");
}

#[test]
fn claude_local_quota_shows_the_record_time_and_does_not_invent_resets(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.quota_accounts(1);h.app.quota.config.accounts[0].provider=crate::quota::Provider::Claude;
    let at=chrono::Utc::now().timestamp()-60;h.app.quota_event(crate::quota::Event{id:"preview-0".into(),result:Ok(crate::quota::Snapshot{at,windows:vec![
        crate::quota::Window{label:"5 小时".into(),seconds:Some(18000),remaining:89.0,reset:None,amount:None},crate::quota::Window{label:"本周".into(),seconds:Some(604800),remaining:24.0,reset:None,amount:None}],..Default::default()})});
    h.app.show_page(6);let mut raster=feature_raster();let mut shapes=Vec::new();
    for _ in 0..90{
        h.t+=1.0/60.0;let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(h.t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
        let env=Env{t:h.t,clock:"20:30:00".into(),date:"11月6日 周五".into(),today:"2026-11-06".into(),cursor:Some(Pos2::new(WIN_W/2.0,12.0)),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H)),&env);});});raster.update_textures(&output.textures_delta);shapes=output.shapes;raster.free_textures(&output.textures_delta);
    }
    let texts:Vec<_>=shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)=>Some(t),_=>None}).collect();
    let label=texts.iter().find(|t|t.galley.text()=="Claude · 本地记录").expect("应明确标注本地来源");
    let time=chrono::DateTime::from_timestamp(at,0).unwrap().with_timezone(&chrono::Local).format("%H:%M").to_string();
    let timestamp=texts.iter().find(|t|t.galley.text()==format!("本地记录 · {time}")).expect("应显示官方缓存的实际记录时间");assert!(!label.visual_bounding_rect().intersects(timestamp.visual_bounding_rect()));assert_eq!(texts.iter().filter(|t|t.galley.text()=="重置时间未知").count(),2);
    if let Some(path)=std::env::var_os("DAOZAI_CLAUDE_LOCAL_PREVIEW"){
        raster.buf.fill([0.15,0.15,0.15,1.0]);raster.draw(&h.ctx.tessellate(shapes,2.0),2.0);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);let image=image::RgbaImage::from_raw((crate::island::CANVAS_W*2.0) as u32,(WIN_H*2.0) as u32,rgba).unwrap();image::imageops::crop_imm(&image,200,0,880,350).to_image().save(path).unwrap();
    }
}

#[test]
fn quota_accounts_switch_independently_and_hide_with_the_page() {
    use crate::quota::{Account,Provider};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    for id in ["personal","work"] {h.app.quota.config.accounts.push(Account{id:id.into(),provider:Provider::Codex,alias:id.into(),source:format!("{id}.json").into(),slot:None,enabled:true,pinned:true});}
    h.app.quota.config.selected="personal".into();
    h.app.quota_event(crate::quota::Event{id:"personal".into(),result:crate::quota::parse(Provider::Codex,&serde_json::json!({"rate_limit":{"secondary_window":{"used_percent":20,"limit_window_seconds":604800}}}))});
    h.app.quota_event(crate::quota::Event{id:"work".into(),result:Err("登录已失效".into())});
    assert_eq!(h.app.quota.rows["personal"].as_ref().unwrap().windows[0].remaining,80.0);
    assert!(h.app.quota.rows["work"].is_err());assert_eq!(h.app.quota_bar_indices().len(),2);
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.show_page(6);h.hover(H::local(300.0,60.0),0.7);
    h.click(H::local(202.0,39.0));assert_eq!(h.app.quota.config.selected,"work");
    assert_eq!(crate::quota::Config::load().selected,"work");
    h.app.set_page_visible(6,false);assert!(h.app.quota_bar_indices().is_empty());assert_ne!(h.app.page,6);
    h.app.quota.config.accounts.retain(|a|a.id!="work");h.app.quota_event(crate::quota::Event{id:"work".into(),result:Ok(Default::default())});
    assert!(h.app.quota.rows["work"].is_err(),"移除账号后不得接收晚到的额度数据");
}

#[test]
#[ignore = "按需导出 DeepSeek 余额界面"]
fn deepseek_balance_gallery(){
    use crate::quota::{Provider,Event};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_DEEPSEEK_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for skin in [crate::model::Skin::Classic,crate::model::Skin::Cream,crate::model::Skin::GlassLight,crate::model::Skin::GlassDark]{
    for (name,amount,stale,count) in [("normal","4.18",false,2),("large","12800.00",false,2),("overflow","128000.00",false,5),("stale","12800.00",true,2),("loading","4.18",false,2),("error","4.18",false,2)] {for open in [false,true]{
        let mut h=H::new();h.app.settings.pet=false;h.app.settings.skin=skin;h.app.settings.show_battery=true;h.quota_accounts(count);
        h.app.quota.config.accounts[1].provider=Provider::Claude;h.app.quota.config.accounts[1].alias="Claude".into();h.app.quota.config.accounts[0].alias="Codex".into();
        let id=h.app.quota.config.connect_deepseek("preview".into(),"DeepSeek".into());
        let snapshot=crate::quota::parse(Provider::DeepSeek,&serde_json::json!({"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":amount,"topped_up_balance":amount,"granted_balance":"0.00"}]})).unwrap();
        h.app.quota_event(Event{id:id.clone(),result:Ok(snapshot)});if stale{h.app.quota_event(Event{id:id.clone(),result:Err("无法连接".into())});}
        if name=="loading"{h.app.quota.rows.remove(&id);h.app.quota.busy.insert(id.clone());}
        if name=="error"{h.app.quota.rows.remove(&id);h.app.quota_event(Event{id,result:Err("暂时无法查询余额，请稍后重试".into())});}
        if open{h.app.show_page(6);}
        let mut raster=crate::preview_raster::Raster::new(1280,380);
        for frame in 0..90{
            let t=frame as f64/60.0;let env=Env{t,clock:"09:41:00".into(),battery:Some(80),charging:true,cursor:if open{Some(Pos2::new(320.0,60.0))}else{None},..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(640.0,190.0))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{h.app.frame(ui,ui.max_rect(),&env);});});
            raster.update_textures(&output.textures_delta);
            if frame==89{
                raster.buf.fill([0.09,0.11,0.18,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                image::save_buffer(dest.join(format!("{}-{name}-{}.png",skin.key(),if open{"open"}else{"closed"})),&rgba,1280,380,image::ColorType::Rgba8).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }}}
}

#[test]
fn deepseek_compact_amount_fits_text_and_opens_account(){
    use crate::quota::{Provider,Event};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.quota_accounts(5);h.app.settings.pet=false;h.app.settings.tray_quota=false;h.app.settings.resident_order=vec![1,9,2,0,5,6,7];
    let id=h.app.quota.config.connect_deepseek("preview".into(),"DeepSeek".into());let mut width:Option<f32>=None;let mut amount_rect=Rect::NOTHING;
    for (amount,label) in [("4.18","¥4.1"),("12800.00","¥1.28万"),("128000.00","¥12.8万")]{
        let snapshot=crate::quota::parse(Provider::DeepSeek,&serde_json::json!({"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":amount,"topped_up_balance":amount,"granted_balance":"0.00"}]})).unwrap();h.app.quota_event(Event{id:id.clone(),result:Ok(snapshot)});
        for _ in 0..90{h.frame(vec![]);}
        let text_rect=|label:&str|h.shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Text(text) if text.galley.text()==label=>Some(text.visual_bounding_rect()),_=>None}).expect("常驻金额应可见");
        let fonts:Vec<_>=h.shapes.iter().filter_map(|shape|match &shape.shape{egui::Shape::Text(text) if text.galley.text()==label||text.galley.text()=="09:41"=>Some(text.galley.job.sections[0].format.font_id.clone()),_=>None}).collect();assert_eq!(fonts.len(),2);assert_eq!(fonts[0],fonts[1],"余额与时钟使用相同字体和字号");
        amount_rect=text_rect(label);assert!(amount_rect.right()<h.region.right());
        let gap=text_rect("09:41").left()-amount_rect.right();assert!((10.0..20.0).contains(&gap),"金额与时钟之间不应预留大片空白：{gap}");
        if let Some(short_width)=width{assert!(h.app.current_width()>short_width+10.0,"短金额必须释放多余占位");}else{width=Some(h.app.current_width());}
    }
    h.click(amount_rect.center());assert_eq!(h.app.page(),6);assert_eq!(h.app.quota.config.selected,id);
}

#[test]
#[ignore = "按需导出 DeepSeek 连接状态提示"]
fn deepseek_connection_preview(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.quota.provider=crate::quota::Provider::DeepSeek;h.app.quota.note="DeepSeek 已连接".into();
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_DEEPSEEK_PREVIEW_DIR").expect("需指定预览目录"));
    let mut raster=crate::preview_raster::Raster::new(1200,1400);
    for frame in 0..8{
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(600.0,700.0))),time:Some(frame as f64/60.0),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().show(ctx,|ui|{h.app.quota_settings(ui);});});raster.update_textures(&output.textures_delta);
        if frame==7{raster.buf.fill([0.1,0.1,0.1,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join("connection-status.png"),&rgba,1200,1400,image::ColorType::Rgba8).unwrap();}
        raster.free_textures(&output.textures_delta);
    }
}

#[test]
fn deepseek_resident_preview_does_not_overlap_icons(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.quota_accounts(2);h.app.settings.skin=crate::model::Skin::Classic;
    let id=h.app.quota.config.connect_deepseek("preview".into(),"DeepSeek".into());h.app.quota_event(crate::quota::Event{id,result:crate::quota::parse(crate::quota::Provider::DeepSeek,&serde_json::json!({"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"12800","topped_up_balance":"12800","granted_balance":"0"}]}))});
    let mut raster=crate::preview_raster::Raster::new(1200,380);
    for frame in 0..8{
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(600.0,190.0))),time:Some(frame as f64/60.0),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().show(ctx,|ui|{h.app.resident_order_settings(ui);});});raster.update_textures(&output.textures_delta);
        if frame==7{
            let circles:Vec<_>=output.shapes.iter().filter_map(|shape|if let egui::Shape::Circle(c)=&shape.shape{Some(c)}else{None}).collect();
            let whale=output.shapes.iter().find_map(|shape|if let egui::Shape::Mesh(mesh)=&shape.shape{let rect=mesh.calc_bounds();(rect.width()>14.0&&rect.width()<=24.5&&(rect.width()-rect.height()).abs()<2.0).then_some(rect)}else{None}).expect("DeepSeek 图标");
            let claude=circles.iter().find(|c|c.stroke.color==egui::Color32::from_rgb(255,116,26)&&c.radius>6.0).expect("Claude 圆环");
            assert!(whale.left()>claude.center.x+claude.radius+1.0,"DeepSeek 不应覆盖前一枚图标");
            if let Some(dest)=std::env::var_os("DAOZAI_DEEPSEEK_PREVIEW_DIR"){
                raster.buf.fill([0.1,0.1,0.1,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(std::path::PathBuf::from(dest).join("resident-preview.png"),&rgba,1200,380,image::ColorType::Rgba8).unwrap();
            }
        }
        raster.free_textures(&output.textures_delta);
    }
}

#[test]
fn quota_cards_expose_names_colors_settings_and_connection(){
    fn frame(h:&mut H,events:Vec<egui::Event>){
        h.t+=1.0/60.0;let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(600.0,800.0))),time:Some(h.t),events,..Default::default()};
        let out=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().show(ctx,|ui|{h.app.quota_settings(ui);});});h.shapes=out.shapes;
    }
    fn click(h:&mut H,label:&str){
        let pos=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect().center()),_=>None}).expect("操作入口应可见");
        frame(h,vec![egui::Event::PointerMoved(pos)]);
        frame(h,vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        frame(h,vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        frame(h,vec![]);
    }
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.quota_accounts(2);let id=h.app.quota.config.connect_deepseek("fixture".into(),"工作".into());h.app.quota_event(crate::quota::Event{id:id.clone(),result:Err("API Key 验证失败".into())});
    open_quota_details(&h);
    for _ in 0..8{frame(&mut h,vec![]);}
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="备注"||t.galley.text()=="服务")));
    for label in ["颜色","监控","工作","移除","当前"]{assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==label)),"应直接显示 {label}");}
    click(&mut h,"工作");
    frame(&mut h,vec![egui::Event::Key{key:egui::Key::A,physical_key:None,pressed:true,repeat:false,modifiers:egui::Modifiers{ctrl:true,command:true,..Default::default()}},egui::Event::Text("备用账号".into())]);
    assert_eq!(h.app.quota.config.accounts.iter().find(|a|a.id==id).unwrap().alias,"备用账号","账号名称应可在行内修改");
    click(&mut h,"监控");assert!(!h.app.quota.config.accounts[0].enabled);click(&mut h,"监控");
    let handles:Vec<_>=h.shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()=="::"=>Some(t.visual_bounding_rect().center()),_=>None}).collect();
    let from=handles[0];let to=handles[1];
    frame(&mut h,vec![egui::Event::PointerMoved(from),egui::Event::PointerButton{pos:from,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
    frame(&mut h,vec![egui::Event::PointerMoved(from+egui::vec2(12.0,0.0))]);
    frame(&mut h,vec![egui::Event::PointerMoved(to)]);
    frame(&mut h,vec![egui::Event::PointerButton{pos:to,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
    assert_eq!(h.app.quota.config.accounts[1].id,"preview-0","拖放应改变卡片顺序");
    h.app.quota.config.move_account("preview-0","preview-1");for _ in 0..3{frame(&mut h,vec![]);}
    click(&mut h,"常驻岛上");assert!(!h.app.quota.config.accounts[0].pinned);
    click(&mut h,"重试");assert!(h.app.pending.iter().any(|cmd|matches!(cmd,crate::island::Cmd::QuotaRefresh)));assert!(!h.app.quota.connect_open);
    click(&mut h,"重新连接");assert!(h.app.quota.connect_open);assert_eq!(h.app.quota.connect_target.as_deref(),Some("preview-0"));
    h.app.quota.connect_open=false;for _ in 0..8{frame(&mut h,vec![]);}click(&mut h,"＋ 添加账号");assert!(h.app.quota.connect_open);assert!(h.app.quota.connect_target.is_none());assert!(h.app.quota.alias.is_empty());
    h.app.quota.connect_open=false;for _ in 0..8{frame(&mut h,vec![]);}assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="···"||t.galley.text()=="设置")));
}

#[test]
#[ignore = "按需导出卡片式 AI 额度设置页"]
fn quota_cards_settings_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_DEEPSEEK_PREVIEW_DIR").expect("需指定预览目录"));
    for (name,width,light,dialog) in [("dark",690.0,false,false),("light",690.0,true,false),("wide",900.0,false,false),("connect",690.0,false,true),("narrow",600.0,false,false)]{
        let mut h=H::new();h.quota_accounts(2);h.app.quota.config.accounts[0].alias="本机账号".into();h.app.quota.config.accounts[1].alias="桌面版 1".into();
        for (i,remaining) in [54.0,72.0].into_iter().enumerate(){let account_id=h.app.quota.config.accounts[i].id.clone();h.app.quota_event(crate::quota::Event{id:account_id,result:Ok(crate::quota::Snapshot{windows:vec![crate::quota::Window{label:"5 小时".into(),seconds:Some(18000),remaining:remaining+20.0,reset:None,amount:None},crate::quota::Window{label:"本周".into(),seconds:Some(604800),remaining,reset:None,amount:None}],at:chrono::Utc::now().timestamp(),..Default::default()})});}
        let id=h.app.quota.config.connect_deepseek("preview".into(),"DeepSeek".into());h.app.quota_event(crate::quota::Event{id,result:Err("API Key 验证失败，请重新连接账号".into())});
        h.app.quota.connect_open=dialog;h.app.quota.provider=crate::quota::Provider::DeepSeek;
        h.app.settings.settings_theme=if light{crate::island::settings_theme::SettingsTheme::Light}else{crate::island::settings_theme::SettingsTheme::Dark};h.app.settings_tab=11;h.app.open_settings();h.ctx.set_embed_viewports(false);
        let size=egui::vec2(width,650.0);let ppp=1.5;let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{let id=viewport.ids.this;let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,size)),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(ppp),focused:Some(true),..Default::default()})).collect(),focused:true,..Default::default()};*capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));});
        let (w,height)=((size.x*ppp)as u32,(size.y*ppp)as u32);let mut raster=crate::preview_raster::Raster::new(w as usize,height as usize);
        for frame in 0..8{let root=h.ctx.run(egui::RawInput{viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()},|ctx|h.app.settings_ui(ctx,false));raster.update_textures(&root.textures_delta);let child=output.lock().unwrap().take().unwrap();raster.update_textures(&child.textures_delta);
            if frame==7{raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(format!("quota-cards-{name}.png")),&rgba,w,height,image::ColorType::Rgba8).unwrap();}
            raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
        }
    }
}

#[test]
fn side_dock_stacks_ai_quota_and_opens_the_account(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let (w,hgt)=(crate::island::layout::dock::SIDE_W,crate::island::layout::dock::SIDE_H);
    let mut h=H::new();h.app.settings.dock=crate::model::Dock::Left;
    let (wb,_)=side_quota_fixture(&mut h);
    let away=Some(Pos2::new(w/2.0,hgt-5.0));
    for _ in 0..60 {side_frame(&mut h,away);}
    let items=h.app.quota_side_items();
    assert_eq!(items.len(),h.app.quota.config.pinned_indices().len());
    let order:Vec<_>=items.iter().map(|(i,_)|h.app.quota.config.accounts[*i].provider).collect();
    assert!(items.iter().any(|(i,_)|h.app.quota.config.accounts[*i].id==wb),"WorkBuddy 应在竖向额度里");
    // 胶囊变长以容纳额度，仍完整留在画布内并竖直居中。
    let out=side_frame(&mut h,away);
    assert!(out.hit.height()>32.0+38.0*2.0,"{:?} {order:?}",out.hit);
    assert!(out.hit.top()>0.0 && out.hit.bottom()<hgt);
    // 点击 WorkBuddy 那一项（按竖排顺序算出它的中心）：选中该账号并打开 AI 额度页。
    let top=h.app.island_rect(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt)),h.app.w.pos,h.app.h.pos).top();
    let total=h.app.side_bar_len()-14.0;
    let mut y=top+(h.app.h.pos-total)/2.0;
    for item in h.app.side_items() {
        let height=match item {crate::island::layout::dock::SideItem::Quota(i,hh) if h.app.quota.config.accounts[i].id==wb=>{y+=hh/2.0;break;},crate::island::layout::dock::SideItem::Quota(_,hh)=>hh,crate::island::layout::dock::SideItem::Clock=>32.0,crate::island::layout::dock::SideItem::Weather=>40.0,crate::island::layout::dock::SideItem::Battery=>20.0,crate::island::layout::dock::SideItem::Music=>18.0,crate::island::layout::dock::SideItem::Focus|crate::island::layout::dock::SideItem::Bell=>22.0,crate::island::layout::dock::SideItem::Agent=>30.0,crate::island::layout::dock::SideItem::Badge(_)=>18.0};
        y+=height;
    }
    let click=Pos2::new(18.0,y);
    side_frame(&mut h,Some(click));
    {
        let (w,hgt)=(w,hgt);
        h.t+=1.0/60.0;
        let env=Env{t:h.t,clock:"09:41:00".into(),cursor:Some(click),..Default::default()};
        for events in [vec![egui::Event::PointerMoved(click),egui::Event::PointerButton{pos:click,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}],vec![egui::Event::PointerButton{pos:click,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]] {
            h.t+=1.0/60.0;
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,events,..Default::default()};
            let app=&mut h.app;let env=Env{t:h.t,..env.clone()};
            let _=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(w,hgt)),&env);});});
        }
    }
    assert_eq!(h.app.quota.config.selected,wb);
    assert_eq!(h.app.page(),6);
}

#[test]
fn workbuddy_and_deepseek_balances_stay_ordered_and_clickable(){
    use crate::quota::{Provider,Event};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.quota_accounts(5);h.app.settings.pet=false;h.app.settings.tray_quota=false;
    let wb=h.app.quota.config.connect_workbuddy("fixture".into(),"WorkBuddy".into());
    let ds=h.app.quota.config.connect_deepseek("fixture".into(),"DeepSeek".into());
    h.app.quota_event(Event{id:wb.clone(),result:crate::quota::parse(Provider::WorkBuddy,&serde_json::json!({"code":0,"data":{"Packages":[{"CycleRemainCapacity":"28560"}]}}))});
    h.app.quota_event(Event{id:ds.clone(),result:crate::quota::parse(Provider::DeepSeek,&serde_json::json!({"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"4.18","granted_balance":"0","topped_up_balance":"4.18"}]}))});
    for _ in 0..90{h.frame(vec![]);}
    let rect=|label:&str|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label=>Some(t.visual_bounding_rect()),_=>None}).expect("两个余额应同时显示");
    let credits=rect("2.85万");let balance=rect("¥4.1");assert!(credits.right()+20.0<balance.left());
    assert_eq!(h.app.quota_bar_indices().iter().skip(5).map(|i|h.app.quota.config.accounts[*i].provider).collect::<Vec<_>>(),vec![Provider::WorkBuddy,Provider::DeepSeek]);
    h.click(credits.center());assert_eq!(h.app.quota.config.selected,wb);assert_eq!(h.app.page(),6);
}

#[test]
#[ignore = "按需导出 WorkBuddy 与 DeepSeek 并排界面"]
fn workbuddy_balance_gallery(){
    use crate::quota::{Provider,Event};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_DEEPSEEK_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for skin in [crate::model::Skin::Classic,crate::model::Skin::Cream,crate::model::Skin::GlassLight,crate::model::Skin::GlassDark]{
    for (name,amount,stale,count) in [("workbuddy","4.18",false,3)] {for open in [false,true]{
        let mut h=H::new();h.app.settings.pet=false;h.app.settings.skin=skin;h.app.settings.show_battery=true;h.quota_accounts(count);
        h.app.quota.config.accounts[1].provider=Provider::Claude;h.app.quota.config.accounts[1].alias="Claude".into();h.app.quota.config.accounts[0].alias="Codex".into();
        let id=h.app.quota.config.connect_deepseek("preview".into(),"DeepSeek".into());
        let snapshot=crate::quota::parse(Provider::DeepSeek,&serde_json::json!({"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":amount,"topped_up_balance":amount,"granted_balance":"0.00"}]})).unwrap();
        h.app.quota_event(Event{id:id.clone(),result:Ok(snapshot)});if stale{h.app.quota_event(Event{id:id.clone(),result:Err("无法连接".into())});}
        if name=="loading"{h.app.quota.rows.remove(&id);h.app.quota.busy.insert(id.clone());}
        if name=="error"{h.app.quota.rows.remove(&id);h.app.quota_event(Event{id,result:Err("暂时无法查询余额，请稍后重试".into())});}
        let wb=h.app.quota.config.connect_workbuddy("preview".into(),"WorkBuddy".into());
        h.app.quota_event(Event{id:wb,result:crate::quota::parse(Provider::WorkBuddy,&serde_json::json!({"code":0,"data":{"Packages":[{"CycleRemainCapacity":"2000"}],"CreditGroups":[{"Kind":"plan_base","CycleRemainCapacity":"500"},{"Kind":"benefit","CycleRemainCapacity":"1500"}]}}))});
        if open{h.app.show_page(6);}
        let mut raster=crate::preview_raster::Raster::new(1280,380);
        for frame in 0..90{
            let t=frame as f64/60.0;let env=Env{t,clock:"09:41:00".into(),battery:Some(80),charging:true,cursor:if open{Some(Pos2::new(320.0,60.0))}else{None},..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(640.0,190.0))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{h.app.frame(ui,ui.max_rect(),&env);});});
            raster.update_textures(&output.textures_delta);
            if frame==89{
                raster.buf.fill([0.09,0.11,0.18,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                image::save_buffer(dest.join(format!("{}-{name}-{}.png",skin.key(),if open{"open"}else{"closed"})),&rgba,1280,380,image::ColorType::Rgba8).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }}}
}

#[test]
fn claude_local_only_settings_show_notice_without_manual_reset_controls(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn frame(h:&mut H,raster:&mut crate::preview_raster::Raster){
        h.t+=1.0/60.0;
        let output=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(640.0,760.0))),time:Some(h.t),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|h.app.quota_settings(ui));});
        raster.update_textures(&output.textures_delta);h.shapes=output.shapes;
    }
    fn capture(h:&H,raster:&mut crate::preview_raster::Raster,name:&str){
        if let Some(dir)=std::env::var_os("DAOZAI_CLAUDE_SETTINGS_PREVIEW"){
            let dir=std::path::PathBuf::from(dir);std::fs::create_dir_all(&dir).unwrap();
            raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(h.shapes.clone(),1.0),1.0);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            image::save_buffer(dir.join(format!("{name}.png")),&rgba,640,760,image::ColorType::Rgba8).unwrap();
        }
    }
    for dark in [true,false]{
        let mut h=H::new();h.ctx.set_visuals(if dark{egui::Visuals::dark()}else{egui::Visuals::light()});
        let account=crate::quota::Account{id:"local-test".into(),provider:crate::quota::Provider::Claude,alias:"Claude 本地账号".into(),source:"fixture/config.json".into(),slot:Some("fixture".into()),enabled:true,pinned:true};
        h.app.quota.config.accounts.push(account.clone());
        let snapshot=crate::quota::Snapshot{at:chrono::Utc::now().timestamp(),windows:vec![crate::quota::Window{label:"5 小时".into(),seconds:Some(18000),remaining:37.5,reset:None,amount:None},crate::quota::Window{label:"本周".into(),seconds:Some(604800),remaining:62.0,reset:None,amount:None}],..Default::default()};
        h.app.quota.rows.insert(account.id.clone(),Ok(snapshot.clone()));open_quota_details(&h);let mut raster=crate::preview_raster::Raster::new(640,760);
        for connection in [false,true]{
            h.app.quota.provider=crate::quota::Provider::Claude;h.app.quota.connect_open=connection;
            for _ in 0..4{frame(&mut h,&mut raster);}
            for phrase in ["由于 Claude 对网络使用环境要求较高","只读取本机用量记录","重置时间未知","本地读取不构成账号安全保证"]{
                let text=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text().contains(phrase)=>Some((s.clip_rect,t.visual_bounding_rect())),_=>None}).unwrap_or_else(||panic!("缺少说明：{phrase}"));
                assert!(text.0.contains_rect(text.1),"说明不能被裁切：{phrase}");
            }
            for phrase in ["手动设置重置时间","保存时间","清除手动时间","默认预选","手动填写"]{
                assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains(phrase))),"已撤回的手动时间入口及提示不应显示：{phrase}");
            }
            assert_eq!(h.app.quota.rows[&account.id].as_ref().unwrap().windows,snapshot.windows,"撤回输入功能不能影响额度数据");
            let name=format!("{}-{}",if connection{"connection"}else{"account"},if dark{"dark"}else{"light"});capture(&h,&mut raster,&name);
        }
        assert!(h.app.quota.gemini_connecting.is_none()&&h.app.quota.workbuddy_connecting.is_none()&&h.app.quota.login.is_none());
    }

}

#[test]
fn tray_quota_shows_during_bell_and_opens_the_quota_page(){
    use crate::island::layout::bar::TRAY;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.quota_accounts(1);
    for _ in 0..30{h.frame(vec![]);}
    h.app.windows_unread=true;
    if let Some(Ok(snapshot))=h.app.quota.rows.get_mut("preview-0"){snapshot.windows[0].reset=Some(chrono::Utc::now().timestamp()+3600);}
    for _ in 0..20{h.frame(vec![]);}
    assert!(h.app.quota_bar_indices().is_empty(),"有铃铛时行内额度让位");
    assert_eq!(h.app.tray_active(),Some(crate::island::event_tray::Item::Quota));
    assert!(h.app.resident.current().x[crate::island::layout::bar::QUOTA].is_none(),"额度不在常驻行");
    assert!(h.app.resident.current().x[TRAY].is_some(),"额度变化时容器占位");
    let center=*h.quota_centers().first().expect("容器里应画出额度环");
    h.click(Pos2::new(center,15.0));
    assert_eq!(h.app.page(),6,"点容器里的额度环打开额度页");
}

#[test]
fn quota_acceptance_monthly_limit_is_visible_on_the_card() {
    use crate::quota::{Provider,Account,plans};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let snapshot=plans::parse(plans::PlanProvider::OpenCodeGo,&serde_json::json!({"usage":{"rolling":{"percent":20},"weekly":{"percent":30},"monthly":{"percent":100}}})).unwrap();
    h.app.quota.config.accounts=vec![Account{id:"preview".into(),provider:Provider::OpenCodeGo,alias:"OpenCode Go".into(),source:Default::default(),slot:None,enabled:true,pinned:false}];
    h.app.quota.config.selected="preview".into();h.app.quota.rows.insert("preview".into(),Ok(snapshot));h.app.show_page(6);h.hover(H::local(230.0,65.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("本月"))),"月额度已经用完，但卡片没有直接显示本月限制");
}

#[test]
fn quota_acceptance_closing_settings_cancels_the_key_dialog() {
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings_tab=11;h.app.open_settings();h.ctx.set_embed_viewports(false);
    h.app.quota.provider=crate::quota::Provider::OpenCodeGo;h.app.quota.connect_open=true;h.app.quota.api_key="synthetic".into();
    let (connection,cancelled)=crate::quota::api::Connect::pending_test();h.app.quota.api_connecting=Some(connection);
    egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{let id=viewport.ids.this;let _=ctx.run(egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,700.0))),viewports:std::iter::once((id,egui::ViewportInfo{events:vec![egui::ViewportEvent::Close],..Default::default()})).collect(),..Default::default()},|ctx|(viewport.viewport_ui_cb)(ctx));});
    let _=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));
    assert!(cancelled.load(std::sync::atomic::Ordering::Acquire));assert!(h.app.quota.api_connecting.is_none());
    assert!(!h.app.settings_open);assert!(!h.app.quota.connect_open&&h.app.quota.api_key.is_empty(),"关闭设置窗口后，密钥输入和连接状态仍未取消");
}

#[test]
fn quota_acceptance_relay_subscription_exhausted_week_is_visible() {
    use crate::quota::{Provider,Account,relay};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let snapshot=relay::parse_sub2api(&serde_json::json!({"mode":"unrestricted","isValid":true,"unit":"USD","remaining":0,"subscription":{"daily_limit_usd":10,"daily_usage_usd":2,"weekly_limit_usd":20,"weekly_usage_usd":20}})).unwrap();
    h.app.quota.config.accounts=vec![Account{id:"relay".into(),provider:Provider::Sub2Api,alias:"工作中转".into(),source:"https://relay.example".into(),slot:None,enabled:true,pinned:false}];h.app.quota.config.selected="relay".into();h.app.quota.rows.insert("relay".into(),Ok(snapshot));h.app.show_page(6);h.hover(H::local(230.0,65.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("每周"))),"卡片必须显示已耗尽的套餐周期");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="0%")),"套餐剩余额度应显示百分比");
}

#[test]
fn quota_acceptance_relay_key_only_response_shows_balance_unavailable(){
    use crate::quota::{Provider,Account,relay};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let snapshot=relay::parse_new_api(&serde_json::json!({"code":true,"data":{"object":"token_usage","unlimited_quota":true,"total_used":2}})).unwrap();
    h.app.quota.config.accounts=vec![Account{id:"relay".into(),provider:Provider::NewApi,alias:"测试站点".into(),source:"https://relay.example".into(),slot:None,enabled:true,pinned:true}];
    h.app.quota.config.selected="relay".into();h.app.quota.rows.insert("relay".into(),Ok(snapshot));h.app.show_page(6);h.hover(H::local(230.0,65.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="余额暂不可查询")));
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("不限额")||t.galley.text().contains("未设置限额"))));
}

#[test]
fn quota_acceptance_new_api_user_balance_is_visible(){
    use crate::quota::{Provider,Account,relay};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let snapshot=relay::parse_new_api_user(&serde_json::json!({"success":true,"data":{"quota":550000,"used_quota":1000000}}),&serde_json::json!({"success":true,"data":{"quota_per_unit":500000,"quota_display_type":"USD"}})).unwrap();
    h.app.quota.config.accounts=vec![Account{id:"relay-user".into(),provider:Provider::NewApi,alias:"测试站点".into(),source:"https://relay.example".into(),slot:None,enabled:true,pinned:true}];
    h.app.quota.config.selected="relay-user".into();h.app.quota.rows.insert("relay-user".into(),Ok(snapshot));h.app.show_page(6);h.hover(H::local(230.0,65.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="1.1 USD")));
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="余额暂不可查询")));
}

#[test]
fn quota_new_api_user_connection_switches_without_reusing_api_key(){
    fn draw(h:&mut H,raster:&mut crate::preview_raster::Raster,events:Vec<egui::Event>)->egui::FullOutput{
        h.t+=0.2;let output=h.ctx.run(egui::RawInput{time:Some(h.t),screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(780.0,850.0))),events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|h.app.quota_settings(ui));});
        raster.update_textures(&output.textures_delta);output
    }
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dark in [true,false]{
        let mut h=H::new();h.ctx.set_visuals(if dark{egui::Visuals::dark()}else{egui::Visuals::light()});
        h.app.settings_open=true;h.app.quota.provider=crate::quota::Provider::NewApi;h.app.quota.connect_open=true;h.app.quota.source="https://relay.example/tenant".into();h.app.quota.api_key="synthetic-api-key".into();
        let mut raster=crate::preview_raster::Raster::new(780,850);
        let _=draw(&mut h,&mut raster,vec![]);let _=draw(&mut h,&mut raster,vec![]);let output=draw(&mut h,&mut raster,vec![]);
        let pos=output.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()=="用户 Token（账户余额）"=>Some(t.visual_bounding_rect().center()),_=>None}).expect("应提供用户 Token 连接入口");
        let _=draw(&mut h,&mut raster,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        let _=draw(&mut h,&mut raster,vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        assert!(h.app.quota.relay_user_token);assert!(h.app.quota.api_key.is_empty());assert_eq!(h.app.quota.source,"https://relay.example/tenant");
        h.app.quota.relay_user_id="42".into();h.app.quota.api_key="synthetic-user-access-token".into();
        let _=draw(&mut h,&mut raster,vec![]);let output=draw(&mut h,&mut raster,vec![]);
        assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="用户 ID")));
        assert!(!output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains("synthetic-user-access-token"))));
        if let Some(dest)=std::env::var_os("DAOZAI_NEW_API_PREVIEW"){
            raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            let path=std::path::PathBuf::from(dest);std::fs::create_dir_all(&path).unwrap();image::save_buffer(path.join(if dark{"new-api-user-dark.png"}else{"new-api-user-light.png"}),&rgba,780,850,image::ColorType::Rgba8).unwrap();
        }
        h.app.quota_close_connection();assert!(h.app.quota.api_key.is_empty()&&h.app.quota.relay_user_id.is_empty());assert!(!h.app.quota.relay_user_token);
    }
}

#[test]
fn quota_cc_switch_general_and_custom_connections_render_in_both_themes(){
    use crate::quota::{Provider,relay};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for provider in [Provider::RelayGeneral,Provider::RelayCustom]{for dark in [true,false]{
        let mut h=H::new();h.ctx.set_visuals(if dark{egui::Visuals::dark()}else{egui::Visuals::light()});
        h.app.settings_open=true;h.app.quota.connect_open=true;h.app.quota.provider=provider;h.app.quota.relay_manual=true;h.app.quota.alias="我的中转站".into();h.app.quota.source="https://relay.example".into();h.app.quota.api_key="synthetic-private-key".into();h.app.quota.relay_script=relay::script::DEFAULT_SCRIPT.into();
        let mut raster=crate::preview_raster::Raster::new(780,850);
        for frame in 0..4{
            let output=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(780.0,850.0))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|h.app.quota_settings(ui));});raster.update_textures(&output.textures_delta);
            if frame==3{
                let texts:Vec<_>=output.shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)=>Some(t.galley.text()),_=>None}).collect();let has_button=texts.contains(&"验证并连接");assert!(!texts.iter().any(|s|s.contains("synthetic-private-key")));
                if provider==Provider::RelayCustom{assert!(texts.contains(&"查询脚本"));}
                if let Some(dest)=std::env::var_os("DAOZAI_CC_SWITCH_PREVIEW"){
                    raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);let path=std::path::PathBuf::from(dest);std::fs::create_dir_all(&path).unwrap();image::save_buffer(path.join(format!("{}-{}.png",if provider==Provider::RelayCustom{"custom"}else{"general"},if dark{"dark"}else{"light"})),&rgba,780,850,image::ColorType::Rgba8).unwrap();
                }
                assert!(has_button,"{provider:?} 连接按钮应可见");
            }
        }
        h.app.quota_close_connection();assert!(h.app.quota.relay_script.is_empty()&&h.app.quota.api_key.is_empty());
    }}
}

#[test]
fn quota_acceptance_grouped_connections_preserve_saved_sources() {
    use crate::quota::{Provider,Account};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for provider in [Provider::ZCode,Provider::Glm,Provider::NewApi,Provider::OneApi,Provider::Sub2Api] {
        let mut h=H::new();h.app.settings_open=true;h.app.quota.provider=provider;h.app.quota.connect_open=true;
        let source=if provider.relay(){"https://relay.example/path"}else if provider==Provider::Glm{"global"}else{"fixture/credentials.json"};
        let account=Account{id:"existing".into(),provider,alias:"原账号".into(),source:source.into(),slot:Some("existing-reference".into()),enabled:true,pinned:false};
        h.app.quota.config.accounts=vec![account.clone()];h.app.quota.config.selected=account.id.clone();h.app.quota.connect_target=Some(account.id.clone());h.app.quota.source=source.into();
        for _ in 0..3{
            let output=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(780.0,850.0))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|h.app.quota_settings(ui));});h.shapes=output.shapes;
        }
        let expected=if provider.relay(){"中转站"}else{"GLM / ZCode"};
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==expected)),"应显示统一入口：{expected}");
        assert_eq!(h.app.quota.provider,provider);assert_eq!(h.app.quota.source,source);assert_eq!(h.app.quota.config.accounts,[account]);
        if provider.relay(){assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="高级设置")));}
    }
}

#[test]
fn quota_glm_get_key_link_follows_region_without_submitting_the_key(){
    fn draw(h:&mut H,events:Vec<egui::Event>)->egui::FullOutput{
        h.t+=0.2;
        h.ctx.run(egui::RawInput{time:Some(h.t),screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(780.0,850.0))),events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|h.app.quota_settings(ui));})
    }
    fn click(h:&mut H,output:&egui::FullOutput,label:&str)->egui::FullOutput{
        let pos=output.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap_or_else(||panic!("找不到控件：{label}"));
        let _=draw(h,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        let _=draw(h,vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        let _=draw(h,vec![]);draw(h,vec![])
    }
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dark in [true,false]{
        let mut h=H::new();h.ctx.set_visuals(if dark{egui::Visuals::dark()}else{egui::Visuals::light()});
        h.app.settings_open=true;h.app.quota.provider=crate::quota::Provider::Glm;h.app.quota.connect_open=true;h.app.quota.source="cn".into();h.app.quota.api_key="synthetic-glm-key".into();
        let _=draw(&mut h,vec![]);let _=draw(&mut h,vec![]);let mut output=draw(&mut h,vec![]);
        for (region,url) in [("中国区","https://bigmodel.cn/coding-plan/personal/overview"),("国际区","https://z.ai/manage-apikey/apikey-list")]{
            output=click(&mut h,&output,region);h.app.pending.clear();output=click(&mut h,&output,"获取密钥");
            assert!(h.app.pending.iter().any(|cmd|matches!(cmd,crate::island::Cmd::OpenUrl(target)if target==url)));
            assert_eq!(h.app.quota.api_key,"synthetic-glm-key");assert!(h.app.quota.api_connecting.is_none());assert!(h.app.quota.config.accounts.is_empty());
        }
    }
}

#[test]
fn quota_relay_automatic_is_the_default_and_manual_types_are_hidden() {
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings_open=true;h.app.quota.provider=crate::quota::Provider::NewApi;h.app.quota.connect_open=true;
    for _ in 0..3{
        let output=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(780.0,850.0))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|h.app.quota_settings(ui));});h.shapes=output.shapes;
    }
    let texts=h.shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)=>Some(t.galley.text()),_=>None}).collect::<Vec<_>>();
    assert!(texts.contains(&"自动识别站点类型"));assert!(texts.contains(&"高级设置"));
    assert!(!texts.contains(&"站点类型")&&!texts.contains(&"One API")&&!texts.contains(&"Sub2API"));
}

#[test]
fn quota_relay_manual_types_require_advanced_settings_and_preserve_the_site(){
    fn draw(h:&mut H,events:Vec<egui::Event>)->egui::FullOutput{
        h.t+=0.2;
        h.ctx.run(egui::RawInput{time:Some(h.t),screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(780.0,850.0))),events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|h.app.quota_settings(ui));})
    }
    fn click(h:&mut H,output:&egui::FullOutput,label:&str)->egui::FullOutput{
        let pos=output.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap_or_else(||panic!("找不到控件：{label}"));
        let _=draw(h,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        let _=draw(h,vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        let _=draw(h,vec![]);draw(h,vec![])
    }
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dark in [true,false]{
        let mut h=H::new();h.ctx.set_visuals(if dark{egui::Visuals::dark()}else{egui::Visuals::light()});
        h.app.settings_open=true;h.app.quota.provider=crate::quota::Provider::NewApi;h.app.quota.connect_open=true;h.app.quota.source="https://relay.example/tenant/v1".into();
        let mut raster=crate::preview_raster::Raster::new(780,850);
        let first=draw(&mut h,vec![]);raster.update_textures(&first.textures_delta);
        let second=draw(&mut h,vec![]);raster.update_textures(&second.textures_delta);
        let output=draw(&mut h,vec![]);raster.update_textures(&output.textures_delta);
        if let Some(dest)=std::env::var_os("DAOZAI_RELAY_PREVIEW"){
            let jobs=h.ctx.tessellate(output.shapes.clone(),output.pixels_per_point);raster.draw(&jobs,output.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            let path=std::path::PathBuf::from(dest);std::fs::create_dir_all(&path).unwrap();image::save_buffer(path.join(if dark{"connect-auto-dark.png"}else{"connect-auto-light.png"}),&rgba,780,850,image::ColorType::Rgba8).unwrap();
        }
        let output=click(&mut h,&output,"高级设置");let output=click(&mut h,&output,"自动识别（推荐）");let output=click(&mut h,&output,"Sub2API");
        assert!(h.app.quota.relay_manual);assert_eq!(h.app.quota.provider,crate::quota::Provider::Sub2Api);assert_eq!(h.app.quota.source,"https://relay.example/tenant/v1");
        let output=click(&mut h,&output,"Sub2API");let _=click(&mut h,&output,"自动识别（推荐）");assert!(!h.app.quota.relay_manual);
    }
}

#[test]
fn quota_acceptance_details_open_the_correct_account() {
    use crate::quota::{Provider,Account,relay};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let s=relay::parse_sub2api(&serde_json::json!({"mode":"unrestricted","isValid":true,"unit":"USD","balance":0})).unwrap();
    let a=Account{id:"selected-relay".into(),provider:Provider::Sub2Api,alias:"原站点".into(),source:"https://relay.example".into(),slot:None,enabled:true,pinned:false};
    h.app.quota.config.accounts=vec![a.clone()];h.app.quota.config.selected=a.id.clone();h.app.quota.rows.insert(a.id.clone(),Ok(s));h.app.show_page(6);h.hover(H::local(230.0,65.0),1.0);
    h.click_text("明细 ›");assert!(h.app.settings_open);assert_eq!(h.app.settings_tab,11);assert_eq!(h.app.quota.config.selected,a.id);
}

#[test]
fn quota_acceptance_period_colors_match_the_drawn_rings() {
    use crate::quota::{Accent,Account,Provider,Snapshot,Window};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let id="period-colors";
    h.app.quota.config.accounts=vec![Account{id:id.into(),provider:Provider::MiniMax,alias:"MiniMax".into(),source:Default::default(),slot:None,enabled:true,pinned:false}];
    h.app.quota.config.selected=id.into();h.app.quota.config.colors.insert(id.into(),Accent::Pink);
    let outer=egui::Color32::from_rgb(255,43,132);let inner=crate::visuals::paint::mix(outer,egui::Color32::WHITE,0.16);
    for seconds in [[None,None],[Some(18000),Some(604800)]] {
        h.app.quota.rows.insert(id.into(),Ok(Snapshot{windows:vec![
            Window{label:"当前周期".into(),seconds:seconds[0],remaining:62.0,reset:None,amount:None},
            Window{label:"本周".into(),seconds:seconds[1],remaining:35.0,reset:None,amount:None},
        ],extra:vec![Window{label:"工具".into(),seconds:None,remaining:80.0,reset:None,amount:None}],..Default::default()}));
        let output=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(400.0,200.0))),..Default::default()},|ctx|{
            egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{
                let c=crate::visuals::paint::Card::new(ui,Pos2::ZERO,ui.max_rect());h.app.page_quota(&c);
            });
        });
        let expected=if seconds[0].is_none(){[inner,outer,crate::visuals::theme::GRAY]}else{[outer,inner,crate::visuals::theme::GRAY]};
        for (i,color) in expected.into_iter().enumerate(){
            let center=egui::pos2(159.0,79.0+i as f32*23.0);
            let dot=output.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Circle(c)if(c.radius-2.5).abs()<0.01&&c.center.distance(center)<0.01=>Some(c.fill),_=>None}).expect("应显示周期色点");
            assert_eq!(dot,color,"周期色点应对应实际圆环，未画入圆环的工具额度使用灰色");
        }
    }
}

#[test]
#[ignore="按需导出套餐、中转、深浅主题和连接窗口的实际界面"]
fn quota_acceptance_gallery() {
    use crate::quota::{Provider,Snapshot,Account,plans,relay};
    use serde_json::json;
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_QUOTA_PREVIEW").unwrap());std::fs::create_dir_all(&dest).unwrap();
    let providers=[Provider::OpenCodeGo,Provider::MiniMax,Provider::Glm,Provider::NewApi,Provider::OneApi,Provider::Sub2Api];
    let snapshots:Vec<Snapshot>=vec![
        plans::parse(plans::PlanProvider::OpenCodeGo,&json!({"usage":{"rolling":{"percent":25,"resetsAt":"2030-01-01T00:00:00Z"},"weekly":{"percent":50,"resetsAt":"2030-01-03T00:00:00Z"},"monthly":{"percent":100,"resetsAt":"2030-02-01T00:00:00Z"}}})).unwrap(),
        plans::parse(plans::PlanProvider::MiniMaxChina,&json!({"model_remains":[{"model_name":"general","current_interval_remaining_percent":62,"current_weekly_remaining_percent":35}]})).unwrap(),
        plans::parse(plans::PlanProvider::GlmChina,&json!({"data":{"limits":[{"type":"TOKENS_LIMIT","percentage":30,"unit":3,"number":5,"nextResetTime":1900000000000i64},{"type":"CREDIT_LIMIT","percentage":52,"unit":6,"number":1,"nextResetTime":1900604800000i64},{"type":"TIME_LIMIT","percentage":10}]}})).unwrap(),
        relay::parse_new_api(&json!({"code":true,"data":{"object":"token_usage","total_granted":500000,"total_available":250000,"total_used":250000,"unlimited_quota":false,"user_quota":{"total_available":100000,"total_used":400000}}})).unwrap(),
        relay::parse_one_api(&json!({"object":"billing_subscription","hard_limit_usd":100}),&json!({"object":"list","total_usage":2500})).unwrap(),
        relay::parse_sub2api(&json!({"mode":"unrestricted","isValid":true,"unit":"USD","remaining":0,"subscription":{"daily_limit_usd":10,"daily_usage_usd":5,"weekly_limit_usd":20,"weekly_usage_usd":20}})).unwrap(),
    ];
    for (index,provider) in providers.into_iter().enumerate() {
        let mut h=H::new();h.app.settings.pet=false;
        let a=Account{id:"preview".into(),provider,alias:provider.name().into(),source:if provider.relay(){"https://relay.example".into()}else{"cn".into()},slot:Some("synthetic-reference".into()),enabled:true,pinned:true};
        h.app.quota.config.accounts=vec![a.clone()];h.app.quota.config.selected=a.id.clone();h.app.quota.rows.insert(a.id,Ok(snapshots[index].clone()));
        h.app.show_page(6);let mut raster=feature_raster();
        shoot_feature(&mut h,&mut raster,&dest,&format!("island-{index}"),60,Some(H::local(240.0,65.0)),"2026-11-10");
    }
    for (group,indices) in [("plans",vec![0usize,1,2]),("relay",vec![3usize,4,5])] {
        for light in [false,true] {
            let mut h=H::new();h.app.settings.settings_theme=if light{crate::island::settings_theme::SettingsTheme::Light}else{crate::island::settings_theme::SettingsTheme::Dark};
            for &i in &indices {
                let id=format!("preview-{i}");h.app.quota.config.accounts.push(Account{id:id.clone(),provider:providers[i],alias:providers[i].name().into(),source:if providers[i].relay(){"https://relay.example".into()}else{"cn".into()},slot:Some("synthetic-reference".into()),enabled:true,pinned:true});
                h.app.quota.rows.insert(id.clone(),Ok(snapshots[i].clone()));if h.app.quota.config.selected.is_empty(){h.app.quota.config.selected=id;}
            }
            h.app.settings_tab=11;h.app.open_settings();h.ctx.set_embed_viewports(false);
            let size=egui::vec2(780.0,1080.0);let ppp=1.25;let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
            egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{let id=viewport.ids.this;let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,size)),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(ppp),focused:Some(true),..Default::default()})).collect(),focused:true,..Default::default()};*capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));});
            let mut raster=crate::preview_raster::Raster::new((size.x*ppp)as usize,(size.y*ppp)as usize);
            for f in 0..5{
                let root=h.ctx.run(egui::RawInput{viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()},|ctx|h.app.settings_ui(ctx,false));
                raster.update_textures(&root.textures_delta);let child=output.lock().unwrap().take().unwrap();raster.update_textures(&child.textures_delta);
                if f==4{raster.buf.fill([0.1,0.1,0.11,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(format!("settings-{group}-{}.png",if light{"light"}else{"dark"})),&rgba,(size.x*ppp)as u32,(size.y*ppp)as u32,image::ColorType::Rgba8).unwrap();}
                raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
            }
        }
    }
    for (i,provider) in providers.into_iter().enumerate() {
        let mut h=H::new();h.app.quota.provider=provider;h.app.quota.connect_open=true;
        h.app.settings.settings_theme=crate::island::settings_theme::SettingsTheme::Dark;h.app.settings_tab=11;h.app.open_settings();h.ctx.set_embed_viewports(false);
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{let id=viewport.ids.this;*capture.lock().unwrap()=Some(ctx.run(egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,600.0))),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.5),focused:Some(true),..Default::default()})).collect(),..Default::default()},|ctx|(viewport.viewport_ui_cb)(ctx)));});
        let mut raster=crate::preview_raster::Raster::new(1035,900);
        for f in 0..5{
            let root=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));raster.update_textures(&root.textures_delta);let child=output.lock().unwrap().take().unwrap();raster.update_textures(&child.textures_delta);
            if f==4{raster.buf.fill([0.1,0.1,0.11,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(format!("connect-{i}.png")),&rgba,1035,900,image::ColorType::Rgba8).unwrap();}
            raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
        }
    }
    let images:Vec<_>=(0..6).map(|i|image::open(dest.join(format!("island-{i}.png"))).unwrap().to_rgba8()).collect();
    let width=images.iter().map(|i|i.width()).max().unwrap();let height=images.iter().map(|i|i.height()).max().unwrap();
    let mut sheet=image::RgbaImage::from_pixel(width*2,height*3,image::Rgba([18,20,26,255]));
    for (i,img) in images.iter().enumerate(){image::imageops::replace(&mut sheet,img,((i%2)as u32*width)as i64,((i/2)as u32*height)as i64);}sheet.save(dest.join("island-overview.png")).unwrap();
}
