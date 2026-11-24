//! 设置、主题、托盘与时钟。
use super::*;

#[test]
fn focus_durations_support_direct_input_and_keep_the_current_timer(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.open_settings_category(2);h.hover(H::local(200.0,60.0),1.0);
    let remaining=h.app.pomo.remaining;
    for (label,input,expected,rest) in [("25 分钟","45",45,false),("5 分钟","12",12,true),("45 分钟","999",180,false),("12 分钟","0",1,true),("180 分钟","abc",180,false)]{
        h.cmds.clear();h.click_text(label);h.frame(vec![]);
        assert!(h.app.is_editing(),"单击时长应进入输入状态");
        assert!(h.cmds.iter().any(|cmd|matches!(cmd,crate::island::Cmd::Focus)),"岛内输入必须取得原生键盘焦点");
        h.frame(vec![egui::Event::Text(input.into())]);
        h.frame(vec![egui::Event::Key{key:egui::Key::Enter,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
        h.frame(vec![]);
        let settings=crate::model::Settings::load();
        assert_eq!(if rest{settings.rest_minutes}else{settings.focus_minutes},expected,"输入需校验范围并保存");
        assert!(!h.app.is_editing());assert_eq!(h.app.pomo.remaining,remaining,"调整时长不能重置当前计时");
    }
    h.click_text("180 分钟");h.frame(vec![]);h.frame(vec![egui::Event::Text("30".into())]);
    h.frame(vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    h.frame(vec![]);assert_eq!(h.app.settings.focus_minutes,180,"Esc 应取消本次输入");
    h.click_text("180 分钟");h.frame(vec![]);h.frame(vec![egui::Event::Text("50".into())]);
    h.click(H::local(268.0,62.0));h.frame(vec![]);
    assert_eq!(h.app.settings.focus_minutes,51,"点击加号应先保存输入，再增加一分钟");
    assert_eq!(crate::model::Settings::load().focus_minutes,51);
    h.click_text("51 分钟");h.frame(vec![]);h.frame(vec![egui::Event::Text("40".into())]);
    h.click(H::local(50.0,65.0));h.frame(vec![]);
    assert_eq!(h.app.settings.focus_minutes,40,"点击别处应保存输入");
    assert!(!h.app.is_editing());
}

#[test]
fn enlarged_island_preserves_font_resolution_with_a_separate_settings_window(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.open_settings_category(2);h.hover(H::local(200.0,60.0),1.0);
    h.app.open_settings();h.app.settings_tab=2;h.ctx.set_embed_viewports(false);
    let captured=std::sync::Arc::new(Mutex::new(None));let capture=captured.clone();
    egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
        let id=viewport.ids.this;
        let input=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(690.0,560.0))),
            viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.0),..Default::default()})).collect(),..Default::default()};
        *capture.lock().unwrap()=Some(ctx.run(input,|ctx|(viewport.viewport_ui_cb)(ctx)));
    });
    let mut raster=crate::preview_raster::Raster::new(WIN_W as usize,WIN_H as usize);
    raster.update_textures(&egui::TexturesDelta{set:vec![(egui::TextureId::default(),egui::epaint::ImageDelta::full(h.ctx.fonts(|fonts|fonts.image()),egui::TextureOptions::LINEAR))],..Default::default()});
    for scale in [1.0,1.2,1.4] {
        raster.w=(WIN_W*scale) as usize;raster.h=(WIN_H*scale) as usize;raster.buf.resize(raster.w*raster.h,[0.0;4]);
        for frame in 0..3 {
            h.t+=1.0/60.0;
            let input=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),
                time:Some(h.t),events:vec![egui::Event::PointerMoved(H::local(70.0,40.0))],viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()})).collect(),..Default::default()};
            let root=h.ctx.run(input,|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{
                    let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));
                    h.app.frame(ui,win,&Env{t:h.t,cursor:Some(H::local(70.0,40.0)),clock:"09:41:00".into(),..Default::default()});
                });
                h.app.settings_ui(ctx,false);
            });
            let child=captured.lock().unwrap().take().unwrap();
            assert_eq!(child.pixels_per_point,1.0,"设置窗口不跟随岛体变大");
            for (shapes,ppp) in [(&root.shapes,scale),(&child.shapes,1.0)] {
                for shape in shapes {if let egui::Shape::Text(text)=&shape.shape {
                    assert!((text.galley.pixels_per_point-ppp).abs()<0.0001,"各窗口应使用各自物理尺寸的字形");
                }}
            }
            raster.update_textures(&child.textures_delta);raster.update_textures(&root.textures_delta);
            if frame==2 {if let Some(dest)=std::env::var_os("DAOZAI_FOCUS_SCALE_PREVIEW") {
                let dest=std::path::PathBuf::from(dest);std::fs::create_dir_all(&dest).unwrap();
                raster.buf.fill([0.18,0.20,0.24,1.0]);raster.draw(&h.ctx.tessellate(root.shapes,root.pixels_per_point),root.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                image::save_buffer(dest.join(format!("focus-size-{}.png",(scale*100.0) as u32)),&rgba,(WIN_W*scale) as u32,(WIN_H*scale) as u32,image::ColorType::Rgba8).unwrap();
            }}
            raster.free_textures(&root.textures_delta);
            raster.free_textures(&child.textures_delta);
        }
    }
}

#[test]
fn opaque_surface_gets_a_contour_with_every_skin_and_dock(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for skin in crate::model::Skin::ALL {for dock in crate::model::Dock::ALL {for island_shape in crate::model::IslandShape::ALL {for opaque_surface in [false,true] {
        let mut h=H::new();h.app.settings.skin=skin;h.app.settings.dock=dock;h.app.settings.island_shape=island_shape;h.app.settings.glass_blur=false;
        let size=if dock.is_side(){Vec2::new(crate::island::side_dock::SIDE_W,crate::island::side_dock::SIDE_H)}else{Vec2::new(WIN_W,WIN_H)};
        let mut outlined=false;
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,size)),..Default::default()};
        let _=h.ctx.run(raw,|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                let win=Rect::from_min_size(ui.max_rect().min,size);
                let out=h.app.frame(ui,win,&Env{opaque_surface,..Default::default()});
                outlined=out.glass.is_some_and(|shapes|!shapes.is_empty());
            });
        });
        assert_eq!(outlined,opaque_surface,"皮肤={skin:?}，停靠={dock:?}，形状={island_shape:?}");
    }}}}
}

#[test]
fn top_bar_modes_preserve_legacy_settings_and_save_the_selected_display_mode(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let _h=H::new();
    use crate::model::Settings;
    let path=crate::model::config_dir().join("settings.txt");std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bounds=crate::os::display_layout::Bounds{left:-1920,top:-400,right:0,bottom:680};
    let display=crate::os::display_layout::Display{id:"test".into(),label:"测试屏".into(),bounds,work_top:-368,primary:true};
    for enabled in [false,true] {
        std::fs::write(&path,format!("avoid_top_bar={}\n",enabled as u8)).unwrap();
        let mut settings=Settings::load();
        assert_eq!(settings.avoid_top_bar,enabled);assert!(!settings.top_bar_on_top,"旧配置默认沿用避让方式");
        for on_top in [false,true] {
            settings.top_bar_on_top=on_top;settings.save();let saved=Settings::load();
            assert_eq!((saved.avoid_top_bar,saved.top_bar_on_top),(enabled,on_top));
            assert_eq!(crate::os::display_layout::placement_bounds(&display,saved.avoids_top_bar()).top,if enabled&&!on_top{-368}else{-400},"置顶方式应回到屏幕顶边；关闭适配后不避让");
        }
    }
}

#[test]
fn top_bar_settings_switch_modes_and_respect_the_master_switch(){
    fn draw(h:&mut H,events:Vec<egui::Event>,raster:&mut crate::preview_raster::Raster)->egui::FullOutput {
        h.t+=0.1;let time=h.t;
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,time:Some(time),focused:true,events:events.clone(),screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,1800.0))),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.0),..Default::default()})).collect(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let root=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));raster.update_textures(&root.textures_delta);
        let child=output.lock().unwrap().take().unwrap();raster.update_textures(&child.textures_delta);child
    }
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dark in [true,false] {
        let mut h=H::new();h.app.open_settings();h.app.settings_tab=crate::island::extras::SETTINGS_TAB_BEHAVIOR;h.ctx.set_embed_viewports(false);
        h.app.settings.settings_theme=if dark{crate::island::settings_theme::SettingsTheme::Dark}else{crate::island::settings_theme::SettingsTheme::Light};
        let mut raster=crate::preview_raster::Raster::new(690,1800);
        for _ in 0..3{let _=draw(&mut h,vec![],&mut raster);}
        for (on_top,label) in [(true,"岛仔置顶"),(false,"避开顶栏")] {
            let output=draw(&mut h,vec![],&mut raster);
            let position=output.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label=>{
                assert!(s.clip_rect.contains_rect(t.visual_bounding_rect()),"显示方式按钮不能裁切");Some(t.visual_bounding_rect().center())
            },_=>None}).expect("显示方式应提供避让和置顶两个选项");
            for pressed in [true,false] {let _=draw(&mut h,vec![egui::Event::PointerMoved(position),egui::Event::PointerButton{pos:position,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}],&mut raster);}
            assert_eq!(h.app.settings.top_bar_on_top,on_top);assert_eq!(crate::model::Settings::load().top_bar_on_top,on_top,"切换后立即保存");
            if let Some(dest)=std::env::var_os("DAOZAI_TOP_BAR_PREVIEW_DIR") {
                let output=draw(&mut h,vec![],&mut raster);raster.buf.fill(if dark{[0.08,0.08,0.09,1.0]}else{[0.96,0.96,0.97,1.0]});
                raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);let dest=std::path::PathBuf::from(dest);std::fs::create_dir_all(&dest).unwrap();
                image::save_buffer(dest.join(format!("top-bar-{}-{}.png",if dark{"dark"}else{"light"},if on_top{"on-top"}else{"avoid"})),&rgba,690,1800,image::ColorType::Rgba8).unwrap();
            }
        }
        h.app.settings.avoid_top_bar=false;
        let output=draw(&mut h,vec![],&mut raster);
        assert!(!output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="岛仔置顶")),"关闭适配后收起显示方式选项");
    }
}

#[test]
fn settings_keep_dark_buttons_when_system_theme_changes() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let h=H::new();
    for theme in [egui::Theme::Light,egui::Theme::Dark,egui::Theme::Light] {
        let _=h.ctx.run(egui::RawInput{system_theme:Some(theme),..Default::default()},|ctx| {
            assert!(ctx.style().visuals.widgets.inactive.weak_bg_fill.r()<100,"系统浅色模式不能把深色设置里的按钮变成白底");
        });
    }
}

#[test]
fn expanded_settings_routes_preserve_inline_controls(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (page,tab) in crate::island::PAGE_SETTINGS_TABS.into_iter().enumerate(){
        let mut h=H::new();h.app.show_page(page);h.hover(H::local(200.0,40.0),1.0);
        h.click(settings_button(&h));
        assert!(h.app.settings_open,"页面 {page} 应打开设置窗口");
        assert_eq!(h.app.settings_tab,tab);assert_eq!(h.app.page,page);
        assert!(h.app.subpage.is_none());
    }
    for (page,point) in [(1,H::local(111.0,18.0)),(2,H::local(149.0,110.0))]{
        let mut h=H::new();h.app.show_page(page);h.hover(H::local(200.0,40.0),1.0);
        h.click(point);assert_eq!(h.app.subpage,Some(page));assert!(!h.app.settings_open);
        h.click(settings_button(&h));assert!(h.app.settings_open);
        assert_eq!(h.app.subpage,Some(page),"打开完整设置后应保留岛内调整页");
    }
    let mut h=H::new();h.app.show_page(3);h.hover(H::local(200.0,40.0),1.0);
    h.click(H::local(330.0,18.0));assert!(h.cmds.iter().any(|cmd|matches!(cmd,crate::island::Cmd::AddFiles)));
    assert!(!h.app.settings_open);
    h.app.show_page(6);h.hover(H::local(200.0,40.0),1.0);h.click(H::local(319.0,18.0));
    assert!(h.cmds.iter().any(|cmd|matches!(cmd,crate::island::Cmd::QuotaRefresh)));assert!(!h.app.settings_open);
}

#[test]
fn settings_opening_restores_focus_once_without_becoming_topmost() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.settings_theme=crate::island::settings_theme::SettingsTheme::Dark;
    h.ctx.set_embed_viewports(false);
    let frame=|h:&mut H,focused:bool| {
        egui::Context::set_immediate_viewport_renderer(move |ctx,mut viewport| {
            let id=viewport.ids.this;
            let mut raw=egui::RawInput{viewport_id:id,focused,system_theme:Some(egui::Theme::Light),..Default::default()};
            raw.viewports.insert(id,egui::ViewportInfo{focused:Some(focused),minimized:Some(!focused),..Default::default()});
            let _=ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx));
        });
        let id=egui::ViewportId::from_hash_of("daozai-settings");
        let mut raw=egui::RawInput::default();
        raw.viewports.insert(id,egui::ViewportInfo{focused:Some(focused),..Default::default()});
        let mut output=h.ctx.run(raw,|ctx|h.app.settings_ui(ctx,false));
        let viewport=output.viewport_output.remove(&id).unwrap();
        assert!(viewport.commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark))),"浅色系统下窗口标题栏和边框也应使用深色");
        (viewport.builder,viewport.commands)
    };
    h.app.open_mail_settings();
    let (builder,commands)=frame(&mut h,false);
    assert_eq!(builder.visible,Some(false),"设置窗口应隐藏创建，避免首帧绘制前露出白色背景");
    assert_eq!(builder.active,Some(false),"隐藏窗口不能提前激活");
    let show=commands.iter().position(|cmd|matches!(cmd,egui::ViewportCommand::Visible(true))).expect("首帧绘制后应显示设置窗口");
    let focus=commands.iter().position(|cmd|matches!(cmd,egui::ViewportCommand::Focus)).unwrap();
    let theme=commands.iter().position(|cmd|matches!(cmd,egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark))).unwrap();
    assert!(theme<show,"显示窗口前应设置深色边框，避免白边闪现");
    assert!(show<focus,"应先显示已绘制的窗口，再获取焦点");
    assert_eq!(builder.window_level,Some(egui::WindowLevel::Normal),"打开设置时应保持普通窗口层级");
    assert!(commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Minimized(false))),"应恢复已最小化的设置窗口：{commands:?}");
    assert!(commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Focus)),"应将焦点交给设置窗口");
    let (builder,commands)=frame(&mut h,true);
    assert!(!commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Visible(_))),"正常重绘不应重复显示窗口，避免打断最小化");
    assert_eq!(builder.window_level,Some(egui::WindowLevel::Normal),"操作设置时也不能切换到置顶层");
    assert!(!commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Focus)),"正常绘制不应反复抢焦点");
    let (builder,commands)=frame(&mut h,false);
    assert_eq!(builder.window_level,Some(egui::WindowLevel::Normal),"切回其他程序后应解除设置置顶");
    assert!(!commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Focus)));
    h.app.open_mail_settings();
    let (builder,commands)=frame(&mut h,false);
    assert_eq!(builder.window_level,Some(egui::WindowLevel::Normal),"再次唤起设置仍须保持普通窗口层级");
    assert!(commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Minimized(false))));
    assert!(commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Focus)),"设置已打开时仍应响应再次唤起");
    h.app.settings_open=false;
    let _=h.ctx.run(egui::RawInput::default(),|ctx|h.app.settings_ui(ctx,false));
    h.app.open_settings();
    let (builder,commands)=frame(&mut h,false);
    assert_eq!(builder.visible,Some(false),"关闭后重新创建也应隐藏到首帧完成");
    assert!(commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Visible(true))),"重开设置应正常显示");
}

#[test]
fn resident_fifth_account_stays_clear_of_clock_and_opens(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.pet=false;h.app.settings.show_battery=false;h.app.settings.tray_quota=false;h.quota_accounts(5);
    for _ in 0..90 {h.frame(vec![]);}
    let text_rect=|label:&str|h.shapes.iter().find_map(|shape|match &shape.shape{
        egui::Shape::Text(text) if text.galley.text()==label=>Some(text.visual_bounding_rect()),_=>None,
    }).expect("常驻文字应可见");
    let last=h.app.resident.current().x[crate::island::bar_layout::quota_slot(4)].unwrap()+WIN_W/2.0;
    assert!(last+13.5+8.0<text_rect("09:41").left());
    h.click(Pos2::new(last,h.region.center().y));assert_eq!(h.app.page(),6);
}

#[test]
fn battery_module_hides_power_in_collapsed_and_expanded_headers() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for expanded in [false,true] {
        let mut h=H::new();
        h.app.settings.pet=false;
        h.app.settings.show_battery=false;
        if expanded {h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);} else {h.frame(vec![]);}
        let baseline=visible_shapes(&h.shapes);
        h.power=(Some(80),true);
        h.t-=1.0/60.0;h.frame(vec![]);
        assert!(visible_shapes(&h.shapes)==baseline,"关闭电量后，本机电池状态不应影响界面布局");
        h.app.settings.show_battery=true;
        h.t-=1.0/60.0;h.frame(vec![]);
        assert!(visible_shapes(&h.shapes)!=baseline,"开启后应显示电量");
        h.power=(None,true);
        h.t-=1.0/60.0;h.frame(vec![]);
        assert!(visible_shapes(&h.shapes)==baseline,"没有电池的设备应自动隐藏电量并释放占位");
    }
}

#[test]
fn battery_module_can_disable_only_the_power_warning() {
    use crate::island::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.show_battery=false;
    h.power=(Some(10),false);h.frame(vec![]);
    assert_eq!(h.app.pet_state(),PetState::Awake);
    h.network=Some(false);h.frame(vec![]);
    assert_eq!(h.app.pet_state(),PetState::Worried);
}

#[test]
fn mascot_stays_beside_the_clock_and_all_resident_components(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=false;h.app.settings.tray_quota=false;h.app.settings.tray_focus=false;h.power=(Some(80),true);h.quota_accounts(5);
    h.media=crate::music::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    h.app.pomo.running=true;h.app.pomo.remaining=1200.0;
    for _ in 0..120 {h.frame(vec![]);}let width=h.app.current_width();
    h.app.settings.pet=true;for _ in 0..120 {h.frame(vec![]);}
    assert!((h.app.current_width()-width-(crate::island::bar_layout::slot_width(crate::island::bar_layout::PET)+crate::island::bar_layout::GAP)).abs()<0.1,"岛仔应有独立常驻位置");
    assert_eq!(h.quota_centers().len(),5,"五个常驻账号均显示额度环");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="09:41")),"岛仔与时钟共存");
    assert!(h.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Path(p) if p.points.len()==13 && (p.stroke.width-3.6).abs()<0.01)).count()>=2,"没有闲置或悬停也应显示眼睛");
}

#[test]
fn mascot_atlas_respects_visibility_and_skin_switches(){
    use crate::island::pet_expressions::Expression;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=true;h.app.set_minimal(false);
    for _ in 0..90 {h.frame(vec![]);}
    for skin in [crate::model::Skin::Classic,crate::model::Skin::Cream,crate::model::Skin::GlassLight] {
        h.app.settings.skin=skin;
        for expression in Expression::ALL {
            h.app.pet.expressions.play(expression);for _ in 0..30 {h.frame(vec![]);}
            assert_eq!(h.app.pet.expressions.current(),Some(expression));
            let atlas=h.ctx.tex_manager().read().allocated().find_map(|(id,meta)|(meta.name=="daozai-expressions").then_some(*id)).unwrap();
            let drawn=h.shapes.iter().find_map(|shape|match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id==atlas=>Some(mesh),_=>None,
            }).expect("新表情应画在常驻岛上");
            assert!(h.region.contains_rect(drawn.calc_bounds()),"新表情不能撑出岛体");
            // 每个表情单独一张序列图：横向铺满整张，纹理只有一列大小。
            assert!(drawn.vertices[0].uv.x.abs()<0.001);
            assert_eq!(h.ctx.tex_manager().read().meta(atlas).unwrap().size,[144,1440],"只保留正在播放的表情");
        }
    }
    h.app.settings.pet=false;h.frame(vec![]);assert!(h.app.pet.expressions.current().is_none());
    h.app.settings.pet=true;h.app.pet.expressions.play(Expression::Heart);h.app.set_minimal(true);
    h.frame(vec![]);assert!(h.app.pet.expressions.current().is_none());
}

#[test]
fn water_settings_are_in_the_island_and_editable(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.app.go(2,1.0);h.hover(H::local(300.0,60.0),0.8);h.click(H::local(145.0,110.0));assert_eq!(h.app.page,2);assert_eq!(h.app.subpage,Some(2));assert_eq!(h.app.settings_category,1);
    h.hover(H::local(300.0,60.0),0.8);h.click(H::local(357.0,69.0));assert_eq!(h.app.settings.serving_ml,300);
    // 标题旁的切换在设置子页同样可用：切到久坐后第一行就是久坐时长，最后一行时段两边共用。
    h.click(H::local(136.0,19.0));assert!(h.app.sit_mode);for _ in 0..40{h.frame(vec![]);}
    h.hover(H::local(300.0,60.0),0.3);h.click(H::local(357.0,47.0));assert_eq!(h.app.settings.sit_minutes,65);
    h.hover(H::local(300.0,60.0),0.3);h.click(H::local(357.0,113.0));assert!(h.app.health_time,"最后一行进入提醒时段二级页");
    let end=h.app.settings.water_end;h.hover(H::local(300.0,60.0),0.3);h.click(H::local(357.0,47.0));assert_eq!(h.app.settings.water_end,(end+1)%24);
}

#[test]
fn sit_reminder_waits_for_focus_and_stops_with_module_or_switch(){
    use crate::island::{Phase,SIT_TITLE,WATER_PAGE};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.system_idle=Some(0.0);h.frame(vec![]);h.app.water.remind=false;
    h.app.pomo.phase=Phase::Focus;h.app.pomo.remaining=1e7;h.app.pomo.running=true;
    h.t+=61.0*60.0;h.frame(vec![]);assert!(h.app.notice.is_none(),"专注中不打扰: {:?}",h.app.notice.as_ref().map(|a|(a.title.clone(),a.body.clone())));
    h.app.pomo.running=false;h.frame(vec![]);assert!(h.app.notice.as_ref().is_some_and(|a|a.title==SIT_TITLE),"专注结束后补上提醒");
    h.app.notice=None;h.app.settings.sit_remind=false;h.t+=2.0*3600.0;h.frame(vec![]);assert!(h.app.notice.is_none());
    h.app.settings.sit_remind=true;h.app.set_page_visible(WATER_PAGE,false);h.t+=2.0*3600.0;h.frame(vec![]);assert!(h.app.notice.is_none(),"模块关闭即不再提醒");
}

#[test]
fn rest_settings_keep_the_existing_duration_and_save_the_new_key(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let _h=H::new();
    assert_eq!(crate::model::Settings::default().rest_minutes,5);
    let path=crate::model::config_dir().join("settings.txt");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path,"focus_minutes=30\nshort_minutes=7\nlong_minutes=17\n").unwrap();
    let settings=crate::model::Settings::load();assert_eq!(settings.rest_minutes,7);
    settings.save();let saved=std::fs::read_to_string(&path).unwrap();
    assert!(saved.lines().any(|line|line=="rest_minutes=7"));
    assert!(!saved.contains("short_minutes") && !saved.contains("long_minutes"));
    let reloaded=crate::model::Settings::load();assert_eq!(reloaded.rest_minutes,7);assert_eq!(reloaded.focus_minutes,30);
}

#[test]
fn display_choice_saves_and_keeps_disconnected_selection(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings_open=true;h.app.settings_tab=crate::island::extras::SETTINGS_TAB_BODY;h.ctx.set_embed_viewports(false);
    let bounds=crate::os::display_layout::Bounds{left:0,top:0,right:1920,bottom:1080};
    h.app.displays=vec![
        crate::os::display_layout::Display{id:"internal".into(),label:"显示器 1 · 主屏".into(),bounds,work_top:0,primary:true},
        crate::os::display_layout::Display{id:r"\\?\DISPLAY#external".into(),label:"显示器 2 · 外接屏".into(),bounds,work_top:0,primary:false},
    ];
    let mut raster=crate::preview_raster::Raster::new(690,560);
    fn frame(h:&mut H,raster:&mut crate::preview_raster::Raster,events:Vec<egui::Event>){
        h.t+=1.0/60.0;let time=h.t;
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            *capture.lock().unwrap()=Some(ctx.run(egui::RawInput{time:Some(time),viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,560.0))),events:events.clone(),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.0),..Default::default()})).collect(),..Default::default()},|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let root=h.ctx.run(egui::RawInput{time:Some(time),..Default::default()},|ctx|h.app.settings_ui(ctx,false));let child=output.lock().unwrap().take().unwrap();
        raster.update_textures(&root.textures_delta);raster.update_textures(&child.textures_delta);h.shapes=child.shapes;
    }
    // 设置页内容超过一屏时先向下滚动，直到目标完整出现在窗口内。
    fn reveal(h:&mut H,raster:&mut crate::preview_raster::Raster,label:&str)->Option<Rect>{
        let find=|h:&H|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label=>Some(t.visual_bounding_rect()),_=>None}).filter(|r|r.bottom()<550.0);
        for _ in 0..20 {
            if let Some(r)=find(h){return Some(r);}
            frame(h,raster,vec![egui::Event::PointerMoved(Pos2::new(450.0,300.0)),egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:Vec2::new(0.0,-60.0),modifiers:Default::default()}]);
            for _ in 0..6{frame(h,raster,vec![]);}
        }
        find(h)
    }
    fn click(h:&mut H,raster:&mut crate::preview_raster::Raster,label:&str){
        let pos=reveal(h,raster,label).map(|r|r.center()).unwrap_or_else(||panic!("未找到：{label}"));
        for pressed in [true,false]{frame(h,raster,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}]);}
        frame(h,raster,vec![]);
    }
    for _ in 0..3{frame(&mut h,&mut raster,vec![]);}
    assert!(h.app.settings.display_id.is_empty());click(&mut h,&mut raster,"跟随主屏");
    for _ in 0..20{frame(&mut h,&mut raster,vec![]);}
    if let Some(path)=std::env::var_os("DAOZAI_DISPLAY_PREVIEW"){
        raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(h.shapes.clone(),1.0),1.0);
        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(path,&rgba,690,560,image::ColorType::Rgba8).unwrap();
    }
    click(&mut h,&mut raster,"显示器 2 · 外接屏");assert_eq!(h.app.settings.display_id,r"\\?\DISPLAY#external");
    assert_eq!(crate::model::Settings::load().display_id,h.app.settings.display_id);
    h.app.displays.pop();frame(&mut h,&mut raster,vec![]);
    reveal(&mut h,&mut raster,"所选屏幕未连接");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="所选屏幕未连接")));
    assert_eq!(crate::os::display_layout::selected(&h.app.displays,&h.app.settings.display_id).unwrap().id,"internal");
    click(&mut h,&mut raster,"所选屏幕未连接");click(&mut h,&mut raster,"跟随主屏");
    assert!(crate::model::Settings::load().display_id.is_empty());
}

#[test]
fn focus_settings_are_inside_focus_page(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(1,1.0);h.hover(H::local(300.0,60.0),0.8);h.click(H::local(110.0,18.0));assert_eq!(h.app.page,1);assert_eq!(h.app.subpage,Some(1));
}

#[test]
fn todo_clock_button_sets_a_reminder_without_completing_the_task() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.todos.items=vec![crate::todo::TodoItem{text:"已有任务".into(),done_on:None,due:None,reminded:false,..Default::default()}];
    h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(343.0,46.0));
    assert!(h.app.task_dialog.is_some(),"每项待办应有可见的提醒入口");
    assert!(h.app.todos.items[0].done_on.is_none(),"时钟入口不能同时勾选任务");
    h.frame(vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    assert!(h.app.task_dialog.is_none());assert_eq!(h.app.todos.items[0].text,"已有任务");
}

#[test]
fn reminder_wheels_support_clicks_and_dragging(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let day=chrono::Local::now().date_naive()+chrono::Duration::days(2);
    h.app.todos.items=vec![crate::todo::TodoItem{text:"确认方案".into(),due:Some(format!("{day} 10:30")),done_on:None,reminded:false,..Default::default()}];
    h.app.open_task_reminder(0);h.hover(H::local(300.0,60.0),1.0);
    h.click(H::local(346.0,96.0));h.click(H::local(290.0,58.0));
    let start=H::local(235.0,77.0);h.cursor=Some(start);
    h.frame(vec![egui::Event::PointerMoved(start),egui::Event::PointerButton{pos:start,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
    for y in [59.0,41.0]{let point=H::local(235.0,y);h.cursor=Some(point);h.frame(vec![egui::Event::PointerMoved(point)]);}
    let end=H::local(235.0,41.0);
    h.frame(vec![egui::Event::PointerButton{pos:end,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
    h.picker_click("完成");
    assert_eq!(h.app.todos.items[0].due,Some(format!("{} 09:31",day+chrono::Duration::days(2))));
    assert!(h.app.todos.items[0].done_on.is_none());
}

#[test]
fn page_order_persists_and_controls_navigation() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.page_order=[5,2,0,1,3,11,4,6,7,8,9,10];
    h.app.settings.save();
    let settings=crate::model::Settings::load();
    assert_eq!(settings.page_order,[5,2,0,1,3,11,4,6,7,8,9,10]);
    let restarted=App::new(&h.ctx,Options{fetch_lyrics:false,pet_idle:1e9});
    assert_eq!(restarted.page(),5);
    for page in [1,3,4,6,7,8,9,10,11] {h.app.set_page_visible(page,false);}
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    for expected in [5,2,0] {
        h.click(H::local(398.0,68.0));
        assert_eq!(h.app.page,expected);
        h.hover(H::local(300.0,60.0),0.7);
    }
    h.click(H::local(21.0,68.0));
    assert_eq!(h.app.page,2);
    h.app.set_page_visible(2,false);
    assert_eq!(h.app.page,5);
    for invalid in ["0,0,1,2,3,4,5,6","8,1,2,3,4,5,6,7","0,1","invalid"] {
        std::fs::write(crate::model::config_dir().join("settings.txt"),format!("page_order={invalid}\n")).unwrap();
        assert_eq!(crate::model::Settings::load().page_order,[0,1,2,3,11,4,5,6,7,8,9,10]);
    }
}

#[test]
fn page_order_drag_keeps_draft_until_saved() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings_tab=crate::island::extras::SETTINGS_TAB_BODY;h.app.open_settings();h.ctx.set_embed_viewports(false);
    fn draw(h:&mut H,events:Vec<egui::Event>){
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        h.t+=0.02;let time=h.t;
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,3400.0))),time:Some(time),events:events.clone(),viewports:std::iter::once((id,egui::ViewportInfo::default())).collect(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let _=h.ctx.run(egui::RawInput::default(),|ctx|h.app.settings_ui(ctx,false));
        let child=output.lock().unwrap().take().unwrap();h.shapes=child.shapes;
    }
    for _ in 0..3 {draw(&mut h,vec![]);}
    let point=|h:&H,label:&str|h.shapes.iter().find_map(|s|match &s.shape {egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect().left_center()),_=>None}).unwrap_or_else(||panic!("缺少 {label}"));
    let from=point(&h,"播放控制与同步歌词")+egui::vec2(-18.0,-6.0);
    let to=point(&h,"喝水与久坐提醒");
    let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    draw(&mut h,vec![egui::Event::PointerMoved(from),button(from,true)]);
    draw(&mut h,vec![egui::Event::PointerMoved(from+egui::vec2(0.0,10.0))]);
    draw(&mut h,vec![egui::Event::PointerMoved(to)]);
    assert_eq!(h.app.settings.page_order,[0,1,2,3,11,4,5,6,7,8,9,10]);
    draw(&mut h,vec![button(to,false)]);
    assert_eq!(h.app.settings.page_order,[0,1,2,3,11,4,5,6,7,8,9,10],"拖动只更新草稿，保存前不改设置");
    assert_eq!(h.app.pending_page_order,Some([1,2,0,3,11,4,5,6,7,8,9,10]));
    assert_eq!(crate::model::Settings::load().page_order,[0,1,2,3,11,4,5,6,7,8,9,10]);
    let save=point(&h,"保存");
    draw(&mut h,vec![egui::Event::PointerMoved(save)]);
    draw(&mut h,vec![button(save,true)]);
    draw(&mut h,vec![button(save,false)]);
    assert_eq!(h.app.settings.page_order,[1,2,0,3,11,4,5,6,7,8,9,10]);
    assert_eq!(h.app.pending_page_order,None);
    assert_eq!(crate::model::Settings::load().page_order,h.app.settings.page_order);
}

#[test]
fn general_settings_controls_fit_when_resizing_to_minimum_width() {
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.open_settings();
    h.app.settings.skin=crate::model::Skin::GlassDark;
    h.ctx.set_embed_viewports(false);
    for (width,scale) in [(800.0,1.0),(600.0,2.0),(690.0,1.25)] {
        let output=std::sync::Arc::new(Mutex::new(None));
        let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let input=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(width,1800.0))),
                viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()})).collect(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(input,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        // 皮肤、大小与位置在「岛体」，展开时长与避让在「行为」。
        for (tab,labels) in [(crate::island::extras::SETTINGS_TAB_BODY,vec!["经典","奶油","毛玻璃·浅","毛玻璃·深","悬浮云","恢复默认","恢复居中"]),(crate::island::extras::SETTINGS_TAB_BEHAVIOR,vec!["慢","标准","快","展开后停留","自动避让"])] {
        h.app.settings_tab=tab;
        for _ in 0..3 {let _=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));}
        let frame=output.lock().unwrap().take().unwrap();
        for label in labels {
            let shape=frame.shapes.iter().find(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==label))
                .unwrap_or_else(||panic!("窗口宽 {width} 时缺少控件 {label}"));
            assert!(shape.clip_rect.contains_rect(shape.shape.visual_bounding_rect()),"窗口宽 {width} 时 {label} 被裁切");
        }
        for shape in &frame.shapes {
            if let egui::Shape::Rect(r)=&shape.shape {
                if r.rect.size()==egui::vec2(34.0,20.0) {
                    assert!(r.rect.right()<=shape.clip_rect.right(),"窗口宽 {width} 时开关右侧被裁切");
                }
            }
        }
        }
    }
}

#[test]
fn settings_theme_follows_system_and_preserves_island_style() {
    use crate::island::settings_theme::SettingsTheme;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.open_settings();h.ctx.set_embed_viewports(false);
    assert_eq!(h.app.settings.settings_theme,SettingsTheme::System);
    assert_eq!(SettingsTheme::parse("unknown"),SettingsTheme::System);
    let mut raster=crate::preview_raster::Raster::new(690,1240);
    for (mode,system,dark) in [(SettingsTheme::System,false,false),(SettingsTheme::System,true,true),
        (SettingsTheme::System,false,false),(SettingsTheme::Dark,false,true),(SettingsTheme::Light,true,false)] {
        h.app.settings.settings_theme=mode;
        h.app.settings.save();assert_eq!(crate::model::Settings::load().settings_theme,mode);
        h.ctx.data_mut(|d|d.insert_temp(egui::Id::new("settings-system-theme"),(std::time::Instant::now(),system)));
        let output=std::sync::Arc::new(std::sync::Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,1240.0))),
                viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.0),..Default::default()})).collect(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        for tab in (0..12).chain([crate::island::extras::SETTINGS_TAB_BODY,crate::island::extras::SETTINGS_TAB_BEHAVIOR,crate::island::extras::SETTINGS_TAB_CLIPBOARD]) {
            h.app.settings_tab=tab;
            // AI 额度的添加账号表单默认收起，点「＋ 添加账号」才展开；展开后再检查表单行对齐。
            h.app.quota.connect_open=tab==11;
            // 添加邮箱、添加日历的表单在弹窗里，打开后再检查表单行对齐。
            h.app.mail.form.open=tab==7;
            h.app.calendar.form.open=tab==8;
            for frame in 0..3 {
                h.ctx.data_mut(|d|d.insert_temp(egui::Id::new("settings-system-theme"),(std::time::Instant::now(),system)));
                let root=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));
                assert!(h.ctx.style().visuals.dark_mode,"设置页主题不能泄漏到岛体");
                let commands=&root.viewport_output[&egui::ViewportId::from_hash_of("daozai-settings")].commands;
                assert!(commands.iter().any(|c|matches!(c,egui::ViewportCommand::SetTheme(t) if *t==if dark{egui::SystemTheme::Dark}else{egui::SystemTheme::Light})),"原生边框必须与页面一致");
                let child=output.lock().unwrap().take().unwrap();
                if frame==2 {
                    let pairs:&[(&str,&str)]=match tab {
                        1=>&[("时间偏移","0.35 秒")],
                        2=>&[("专注","25 分钟"),("休息","5 分钟")],
                        3=>&[("每日目标","2000 ml"),("每次记录","250 ml"),("提醒间隔","45 分钟"),("提醒时段","9 时")],
                        7=>&[("邮箱","输入邮箱地址"),("授权码","输入邮箱授权码")],
                        8=>&[("会议提前","15 分钟"),("用户名","输入用户名或粘贴账号信息"),("密码","输入平台生成的密码")],
                        11=>&[("服务","Codex"),("备注","例如：个人、工作、备用"),("选择该账号的登录文件","选择…")],
                        _=>&[],
                    };
                    for (label,value) in pairs {
                        let centers=|needle:&str|child.shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==needle=>Some(t.visual_bounding_rect().center().y),_=>None}).collect::<Vec<_>>();
                        let distance=centers(label).iter().flat_map(|a|centers(value).into_iter().map(move|b|(a-b).abs())).fold(f32::INFINITY,f32::min);
                        assert!(distance<=1.0,"设置页 {tab} 的 {label} 与 {value} 未居中：{distance}");
                    }
                }
                if (tab==0||tab==crate::island::extras::SETTINGS_TAB_BODY) && frame==2 {
                    let center=|label:&str|child.shapes.iter().find_map(|s|match &s.shape {
                        egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect().center().y),_=>None,
                    }).unwrap();
                    let rows=if tab==0 {vec![("时间制式",vec!["24 小时","12 小时"]),("时段文字",vec!["不显示","上午 / 下午","详细时段"])]} else {vec![("显示屏",vec!["跟随主屏"])]};
                    for (label,choices) in rows {
                        for choice in choices {assert!((center(label)-center(choice)).abs()<1.0,"{label}与{choice}应垂直居中：{} / {}",center(label),center(choice));}
                    }
                }
                let background=if dark{egui::Color32::from_rgb(25,25,27)}else{egui::Color32::from_rgb(246,247,249)};
                assert!(child.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.fill==background && r.rect.width()>400.0)));
                raster.update_textures(&root.textures_delta);raster.update_textures(&child.textures_delta);
                if frame==2 {
                    if let Some(dest)=std::env::var_os("DAOZAI_THEME_PREVIEW_DIR") {
                        let dest=std::path::PathBuf::from(dest);std::fs::create_dir_all(&dest).unwrap();
                        raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);
                        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                        image::save_buffer(dest.join(format!("settings-{}-{tab}.png",if dark{"dark"}else{"light"})),&rgba,690,1240,image::ColorType::Rgba8).unwrap();
                    }
                }
                raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
            }
        }
    }
}

#[test]
#[ignore = "按需导出关于页支持作者预览"]
fn about_support_preview() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_SUPPORT_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    let mut h=H::new();h.app.open_settings();h.app.settings_tab=9;h.ctx.set_embed_viewports(false);
    let output=std::sync::Arc::new(std::sync::Mutex::new(None));let capture=output.clone();
    egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
        let id=viewport.ids.this;
        let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,560.0))),
            viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.0),..Default::default()})).collect(),..Default::default()};
        *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
    });
    let mut raster=crate::preview_raster::Raster::new(690,560);
    for dark in [true,false] { for open in [false,true] {
        h.app.settings.settings_theme=if dark{crate::island::settings_theme::SettingsTheme::Dark}else{crate::island::settings_theme::SettingsTheme::Light};
        h.ctx.data_mut(|data|data.insert_temp(egui::Id::new("support-author-open"),open));
        for frame in 0..4 {
            let root=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));
            let child=output.lock().unwrap().take().unwrap();
            raster.update_textures(&root.textures_delta);raster.update_textures(&child.textures_delta);
            if frame==3 {
                let has_wechat=child.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()=="微信支付"));
                assert_eq!(has_wechat,open);
                for label in [if open{"返回"}else{"请喝咖啡 · 支持作者"},"完全自愿 · 感谢每一份支持 · 不影响功能使用"] {
                    let shape=child.shapes.iter().find(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()==label)).expect("按钮和说明必须可见");
                    assert!(shape.clip_rect.contains_rect(shape.shape.visual_bounding_rect()),"默认窗口中内容不能被裁切：{label}");
                }
                raster.buf.fill([0.0,0.0,0.0,1.0]);
                raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                image::save_buffer(dest.join(format!("about-{}-{}.png",if dark{"dark"}else{"light"},if open{"open"}else{"closed"})),&rgba,690,560,image::ColorType::Rgba8).unwrap();
            }
            raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
        }
    } }
}

#[test]
fn clock_seconds_formats_and_persists(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();assert!(!h.app.settings.clock_seconds);
    assert_eq!(h.app.settings.clock_text("23:59:58"),"23:59");
    h.app.settings.clock_seconds=true;
    for (input,expected) in [("00:00:00","00:00:00"),("23:59:59","23:59:59"),("13:05","13:05:00"),("invalid","--:--:--")] {
        assert_eq!(h.app.settings.clock_text(input),expected);
    }
    h.app.settings.clock_12h=true;h.app.settings.clock_period=1;
    for (input,expected) in [("00:00:01","上午 12:00:01"),("12:00:00","下午 12:00:00"),("13:05:08","下午 1:05:08")] {
        assert_eq!(h.app.settings.clock_text(input),expected);
    }
    h.app.settings.clock_period=2;
    assert_eq!(h.app.settings.clock_text("23:59:59"),"晚上 11:59:59");
    for enabled in [true,false] {
        h.app.settings.clock_seconds=enabled;h.app.settings.save();
        assert_eq!(crate::model::Settings::load().clock_seconds,enabled);
    }
}

#[test]
fn clock_residency_only_hides_the_collapsed_clock(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (seconds,twelve,period) in [(false,false,0),(true,false,0),(true,true,2)] {
        let mut h=H::new();h.app.settings.pet=false;h.power=(Some(76),false);h.quota_accounts(2);
        h.app.settings.clock_seconds=seconds;h.app.settings.clock_12h=twelve;h.app.settings.clock_period=period;
        h.app.weather.snapshot=Some(crate::weather::Snapshot{temp:19.0,..Default::default()});
        let digits=if seconds {if twelve{"9:41:00"}else{"09:41:00"}}else{"09:41"};
        let visible=|h:&H|h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==digits));
        for _ in 0..90{h.frame(vec![]);}assert!(visible(&h));let width=h.app.resident.target.width;
        h.app.settings.show_clock=false;
        for _ in 0..90{h.frame(vec![]);}
        assert!(!visible(&h));assert!(h.app.resident.target.x[crate::island::bar_layout::CLOCK].is_none());
        assert!(h.app.resident.target.width<width,"关闭时间应收回常驻占位");
        h.hover(Pos2::new(WIN_W/2.0,15.0),1.0);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="09:41")),"展开顶栏仍应显示时间");
        h.cursor=None;for _ in 0..120{h.frame(vec![]);}
        for dock in [crate::model::Dock::Left,crate::model::Dock::Right] {
            h.app.settings.dock=dock;
            for _ in 0..90{side_frame(&mut h,Some(Pos2::new(250.0,580.0)));}
            assert!(!h.app.side_items().contains(&crate::island::side_dock::SideItem::Clock));
            assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="41")),"侧边收起态也应隐藏时间");
        }
        h.app.settings.show_clock=true;h.app.settings.dock=crate::model::Dock::Top;
        for _ in 0..90{h.frame(vec![]);}
        assert!(visible(&h));assert_eq!(h.app.resident.target.width,width,"重新开启时间应恢复布局");
    }
}

#[test]
fn clock_residency_switch_in_island_body_saves(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(h:&mut H,raster:&mut crate::preview_raster::Raster,events:Vec<egui::Event>){
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();h.t+=0.02;let time=h.t;
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            *capture.lock().unwrap()=Some(ctx.run(egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(780.0,1400.0))),time:Some(time),events:events.clone(),viewports:std::iter::once((id,egui::ViewportInfo::default())).collect(),..Default::default()},|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let root=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));let child=output.lock().unwrap().take().unwrap();
        raster.update_textures(&root.textures_delta);raster.update_textures(&child.textures_delta);h.shapes=child.shapes;
    }
    for dark in [true,false] {
        let mut h=H::new();h.app.settings.settings_theme=if dark{crate::island::settings_theme::SettingsTheme::Dark}else{crate::island::settings_theme::SettingsTheme::Light};
        h.app.settings_tab=crate::island::extras::SETTINGS_TAB_BODY;h.app.open_settings();h.ctx.set_embed_viewports(false);
        // 时间开关在「常驻显示」分组里。
        let mut raster=crate::preview_raster::Raster::new(780,1400);
        assert!(h.app.settings.show_clock,"已有配置默认保持时间常驻");
        for _ in 0..3{draw(&mut h,&mut raster,vec![]);}
        let title=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()=="时间"=>Some(t.visual_bounding_rect()),_=>None}).expect("状态组件应提供时间开关");
        let point=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Rect(r)if (r.rect.width()-34.0).abs()<0.01&&(r.rect.height()-20.0).abs()<0.01&&r.rect.center().y>title.top()&&r.rect.center().y<title.bottom()+24.0=>Some(r.rect.center()),_=>None}).unwrap();
        for expected in [false,true] {
            for pressed in [true,false]{draw(&mut h,&mut raster,vec![egui::Event::PointerMoved(point),egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}]);}
            draw(&mut h,&mut raster,vec![]);
            assert_eq!(h.app.settings.show_clock,expected);assert_eq!(crate::model::Settings::load().show_clock,expected,"常驻时间开关应自动保存");
            // 预览会随组件数量整体缩放，时钟字号按比例缩小；滑块上的「25:00」字号明显更小。
            let preview=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text().contains(':')&&t.galley.job.sections.first().is_some_and(|s|s.format.font_id.size>crate::visuals::theme::size::PILL_CLOCK*0.8&&s.format.font_id.size<=crate::visuals::theme::size::PILL_CLOCK+0.01)=>Some(t.fallback_color.a()),_=>None}).expect("常驻排列应保留时间的位置");
            assert_eq!(preview==255,expected,"关闭时间后排列预览应置灰");
            if let Some(dest)=std::env::var_os("DAOZAI_CLOCK_RESIDENCY_PREVIEW") {
                let dest=std::path::PathBuf::from(dest);std::fs::create_dir_all(&dest).unwrap();
                raster.buf.fill([0.08,0.08,0.09,1.0]);raster.draw(&h.ctx.tessellate(h.shapes.clone(),1.0),1.0);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                image::save_buffer(dest.join(format!("settings-{}-{}.png",if dark{"dark"}else{"light"},if expected{"on"}else{"off"})),&rgba,780,1400,image::ColorType::Rgba8).unwrap();
            }
        }
    }
}

#[test]
fn clock_seconds_keep_resident_spacing_and_digit_width(){
    use crate::island::bar_layout::{CLOCK,WEATHER};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for period in 0..=2 {for twelve in [false,true] {
        let mut h=H::new();h.app.settings.pet=false;h.app.settings.clock_seconds=true;h.app.settings.tray_weather=false;
        h.app.settings.clock_period=period;h.app.settings.clock_12h=twelve;
        h.quota_accounts(3);h.power=(Some(76),true);
        h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"杭州".into(),temp:19.0,..Default::default()});
        for reversed in [false,true] {
            h.app.settings.resident_order=crate::island::bar_layout::DEFAULT_ORDER.to_vec();
            if reversed {h.app.settings.resident_order.reverse();}
            for _ in 0..90 {h.frame(vec![]);}
            let digits=if twelve {"9:41:00"}else{"09:41:00"};
            let text=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==digits=>Some(t.visual_bounding_rect()),_=>None}).expect("收起时应显示秒数");
            let temp=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()=="19°"=>Some(t.visual_bounding_rect()),_=>None}).unwrap();
            assert!(!text.expand(4.0).intersects(temp),"时钟和天气应保持间距");
            let layout=h.app.resident.current();
            assert!((layout.x[CLOCK].unwrap()-layout.x[WEATHER].unwrap()).abs()>(layout.widths[CLOCK]+layout.widths[WEATHER])/2.0,"排列反转后仍需预留秒数宽度");
        }
        let _=h.ctx.run(Default::default(),|ctx|{egui::CentralPanel::default().show(ctx,|ui|{
            let c=crate::visuals::paint::Card::new(ui,Pos2::ZERO,ui.max_rect());
            let widths:Vec<_>=(0..60).map(|second|crate::visuals::paint::clock_width(&c,&h.app.settings.clock_text(&format!("09:41:{second:02}")),crate::visuals::theme::size::PILL_CLOCK)).collect();
            assert!(widths.iter().all(|w|(w-widths[0]).abs()<0.01),"秒数变化不应改变时钟占位");
        });});
        h.app.show_page(0);h.hover(H::local(200.0,40.0),1.0);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="09:41")),"展开后顶栏只显示时:分");
        h.t+=5.0;
        for dock in [crate::model::Dock::Left,crate::model::Dock::Right] {
            h.app.settings.dock=dock;
            for _ in 0..90 {side_frame(&mut h,Some(Pos2::new(250.0,580.0)));}
            assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="00")),"侧边收起时应显示秒数");
        }
    }}
}

#[test]
fn clock_seconds_setting_toggles_and_saves(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.open_settings();h.ctx.set_embed_viewports(false);
    let mut raster=crate::preview_raster::Raster::new(780,1800);
    fn frame(h:&mut H,raster:&mut crate::preview_raster::Raster,events:Vec<egui::Event>){
        h.t+=1.0/60.0;let time=h.t;
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            *capture.lock().unwrap()=Some(ctx.run(egui::RawInput{time:Some(time),viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(780.0,1800.0))),events:events.clone(),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.0),..Default::default()})).collect(),..Default::default()},|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let root=h.ctx.run(egui::RawInput{time:Some(time),..Default::default()},|ctx|h.app.settings_ui(ctx,false));let child=output.lock().unwrap().take().unwrap();
        raster.update_textures(&root.textures_delta);raster.update_textures(&child.textures_delta);h.shapes=child.shapes;
    }
    for _ in 0..3{frame(&mut h,&mut raster,vec![]);}
    let label=|h:&H|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()=="显示秒数"&&s.clip_rect.contains_rect(t.visual_bounding_rect())=>Some(t.visual_bounding_rect()),_=>None});
    assert!(!h.app.settings.clock_seconds);
    for enabled in [true,false] {
        let title=label(&h).expect("常规设置应提供显示秒数开关");
        let toggle=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Rect(r)if (r.rect.width()-34.0).abs()<0.1&&(r.rect.height()-20.0).abs()<0.1&&(r.rect.top()-title.top()).abs()<18.0=>Some(r.rect.center()),_=>None}).expect("秒数开关应与文字对齐");
        for pressed in [true,false]{frame(&mut h,&mut raster,vec![egui::Event::PointerMoved(toggle),egui::Event::PointerButton{pos:toggle,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}]);}
        frame(&mut h,&mut raster,vec![]);
        assert_eq!(h.app.settings.clock_seconds,enabled);assert_eq!(crate::model::Settings::load().clock_seconds,enabled);
        if let Some(dest)=std::env::var_os("DAOZAI_CLOCK_PREVIEW_DIR") {
            let dest=std::path::PathBuf::from(dest);std::fs::create_dir_all(&dest).unwrap();
            raster.buf.fill([0.08,0.08,0.09,1.0]);raster.draw(&h.ctx.tessellate(h.shapes.clone(),1.0),1.0);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            image::save_buffer(dest.join(if enabled{"settings-on.png"}else{"settings-off.png"}),&rgba,780,1800,image::ColorType::Rgba8).unwrap();
        }
    }
}

#[test]
fn clock_12h_formats_boundaries_and_persists() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    assert!(!h.app.settings.clock_12h);
    assert_eq!(h.app.settings.clock_text("23:59:00"),"23:59");
    assert_eq!(h.app.settings.clock_period,0);
    h.app.settings.clock_12h=true;
    assert_eq!(h.app.settings.clock_text("00:00"),"12:00");
    assert_eq!(h.app.settings.clock_text("23:59"),"11:59");
    h.app.settings.clock_period=1;
    for (input,expected) in [("00:00:00","上午 12:00"),("11:59:00","上午 11:59"),("12:00:00","下午 12:00"),("13:05:00","下午 1:05"),("23:59:00","下午 11:59")] {
        assert_eq!(h.app.settings.clock_text(input),expected);
    }
    assert_eq!(h.app.settings.clock_text("13:05"),"下午 1:05");
    h.app.settings.save();
    assert!(crate::model::Settings::load().clock_12h);
    h.app.settings.pet=false;h.app.settings.tray_quota=false;
    h.quota_accounts(5);
    for _ in 0..90 { h.frame(vec![]); }
    let text_rect=|label:&str|h.shapes.iter().find_map(|s|match &s.shape {
        egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect()),_=>None,
    }).unwrap();
    assert!(h.app.resident.current().x[crate::island::bar_layout::quota_slot(4)].unwrap()+WIN_W/2.0+13.5<text_rect("上午").left());
    h.hover(H::local(200.0,40.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="9:41")));
    h.app.settings.clock_period=0;
    h.app.settings.clock_12h=false;
    h.app.settings.save();
    assert!(!crate::model::Settings::load().clock_12h);
    h.frame(vec![]);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="09:41")));
}

#[test]
fn clock_period_boundaries_and_both_formats() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    for twelve in [false,true] {
        h.app.settings.clock_12h=twelve;
        h.app.settings.clock_period=2;
        for (time,label) in [("00:00","凌晨"),("05:59","凌晨"),("06:00","上午"),("10:59","上午"),("11:00","中午"),("12:59","中午"),("13:00","下午"),("16:59","下午"),("17:00","傍晚"),("18:59","傍晚"),("19:00","晚上"),("23:59","晚上")] {
            assert!(h.app.settings.clock_text(time).starts_with(&format!("{label} ")));
        }
        assert_eq!(h.app.settings.clock_text("19:30"),if twelve {"晚上 7:30"}else{"晚上 19:30"});
        for mode in 0..=2 {
            h.app.settings.clock_period=mode;
            h.app.settings.save();
            let loaded=crate::model::Settings::load();
            assert_eq!(loaded.clock_period,mode);
            assert_eq!(loaded.clock_12h,twelve);
        }
    }
}

#[test]
fn clock_period_visually_centers_with_digits() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let h=H::new();
    let mut raster=crate::preview_raster::Raster::new(420,230);
    for scale in [1.0,1.25,1.5,2.0] {
        let output=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(420.0,230.0))),..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                let mut c=crate::visuals::paint::Card::new(ui,Pos2::ZERO,ui.max_rect());c.s=scale;
                for (i,period) in ["凌晨","上午","中午","下午","傍晚","晚上"].iter().enumerate() {
                    for (j,size) in [12.5,15.5].iter().enumerate() {
                        crate::visuals::paint::clock(&c,10.0+j as f32*100.0,18.0+i as f32*18.0,&format!("{period} 12:59"),*size);
                    }
                }
            });
        });
        let texts:Vec<_>=output.shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)=>Some(t),_=>None}).collect();
        for pair in texts.chunks_exact(2) {
            assert!((pair[0].visual_bounding_rect().center().y-pair[1].visual_bounding_rect().center().y).abs()<0.1,"时段文字与数字应按可见字形垂直居中，缩放 {scale}");
        }
        raster.update_textures(&output.textures_delta);
        if scale==2.0 {
            if let Some(dest)=std::env::var_os("DAOZAI_CLOCK_ALIGNMENT_PREVIEW") {
                raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                image::save_buffer(dest,&rgba,420,230,image::ColorType::Rgba8).unwrap();
            }
        }
    }
}

#[test]
fn compact_font_atlas_preserves_settings_pixels_at_multiple_scales(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(limit:usize,scale:f32)->Vec<Vec<u8>>{
        let mut h=H::new();h.app.open_settings();h.ctx.set_embed_viewports(false);
        let output=std::sync::Arc::new(std::sync::Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,max_texture_side:Some(limit),screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,560.0))),
                viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()})).collect(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let mut raster=crate::preview_raster::Raster::new((690.0*scale)as usize,(560.0*scale)as usize);let mut result=Vec::new();
        for tab in 0..12 {h.app.settings_tab=tab;
            for frame in 0..3 {
                let root=h.ctx.run(egui::RawInput{max_texture_side:Some(limit),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()})).collect(),..Default::default()},|ctx|h.app.settings_ui(ctx,false));
                let mut child=output.lock().unwrap().take().unwrap();
                raster.update_textures(&root.textures_delta);raster.update_textures(&child.textures_delta);
                if frame==2 {
                    // 常驻预览读取真实时钟，排除跨分钟变化；固定时钟已由主界面对照覆盖。
                    if tab==10{child.shapes.retain(|s|!matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().len()==5 && t.galley.text().as_bytes()[2]==b':'));}
                    raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);result.push(rgba);
                }
                raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
            }
        }result
    }
    for scale in [1.0,1.5,2.0] {
        let before=draw(16384,scale);
        for limit in [4096,2048] {let after=draw(limit,scale);
        for (tab,(before,after)) in before.iter().zip(&after).enumerate(){
            // 图集坐标改变只允许少量通道有 1/255 的浮点采样舍入差异。
            let max=before.iter().zip(after).map(|(a,b)|a.abs_diff(*b)).max().unwrap_or(0);
            let count=before.iter().zip(after).filter(|(a,b)|a!=b).count();assert!(max<=1 && count<before.len()/1000,"设置页 {tab} 在图集上限 {limit}、缩放 {scale} 下出现可见差异：{max}/{count}");
        }}
    }
}

#[test]
#[ignore = "按需导出五套皮肤的预览图：DAOZAI_SKIN_PREVIEW_DIR=目录 cargo test skin_gallery -- --ignored"]
fn skin_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_SKIN_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    let (w,hgt)=((crate::island::CANVAS_W*2.0) as usize,(WIN_H*2.0) as usize);
    // 日落渐变 + 几条城市剪影，用来观察半透明皮肤的通透感。
    let backdrop=|raster:&mut crate::preview_raster::Raster|{
        for y in 0..hgt {for x in 0..w {
            let t=y as f32/hgt as f32;let s=x as f32/w as f32;
            let mut c=[0.98-0.25*t+0.05*s,0.62+0.12*t-0.1*s,0.45+0.35*t,1.0];
            if (y>hgt*7/10) && ((x/37)%3!=0) && y>hgt*7/10+((x*7919)%97) {c=[0.30,0.33,0.45,1.0];}
            let cloud=((x as f32/90.0).sin()*(y as f32/40.0).cos()).max(0.0);if y<hgt/2 {for k in 0..3{c[k]=c[k]+(1.0-c[k])*cloud*0.6;}}
            if x%160<12 && y<hgt*6/10 {c=[0.18,0.20,0.30,1.0];}
            raster.buf[y*w+x]=c;
        }}
    };
    for (skin,shape) in crate::model::Skin::ALL.into_iter().flat_map(|s|crate::model::IslandShape::ALL.map(|p|(s,p))) {
        for open in [false,true] {
            let mut h=H::new();h.app.settings.skin=skin;h.app.settings.island_shape=shape;h.app.settings.pet=true;h.app.opts.pet_idle=0.0;
            h.media=crate::music::NowPlaying{title:"Paper Boats".into(),artist:"The Lanterns".into(),position:64.0,duration:201.0,playing:true,..Default::default()};
            let mut raster=crate::preview_raster::Raster::new(w,hgt);
            for frame in 0..120 {
                let t=frame as f64/60.0;
                let cursor=open.then_some(Pos2::new(WIN_W/2.0,14.0));
                let env=Env{t,clock:"11:13:00".into(),today:"2026-10-26".into(),date:"10月26日 周一".into(),media:h.media.clone(),cursor,backdrop:Some([0.85,0.62,0.6]),glass_blur:true,..Default::default()};
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(t),
                    viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
                let mut glass=None;
                let output=h.ctx.run(raw,|ctx|{
                    egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));glass=h.app.frame(ui,win,&env).glass;});
                });
                raster.update_textures(&output.textures_delta);
                if open && frame==100 {h.app.show_page(1);}
                if open && frame==104 && shape==crate::model::IslandShape::Notch {
                    backdrop(&mut raster);
                    raster.draw(&h.ctx.tessellate(output.shapes.clone(),output.pixels_per_point),output.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    let image=image::RgbaImage::from_raw(w as u32,hgt as u32,rgba).unwrap();
                    image::imageops::crop_imm(&image,120,0,1040,400).to_image().save(dest.join(format!("{}-switch.png",skin.key()))).unwrap();
                }
                if frame==119 {
                    backdrop(&mut raster);
                    // 模拟系统实时模糊：只有岛体轮廓内的背景被模糊（与实机的合成层一致）。
                    if let Some(shapes)=glass {
                        let pieces=crate::island::blur_pieces(&shapes,2.0);
                        let mut blurred=raster.buf.clone();
                        for _ in 0..3 {
                            let src=blurred.clone();let r=18i32;
                            for y in 0..hgt as i32 {for x in 0..w as i32 {let mut s=[0.0f32;4];let mut n=0.0;for dx in (-r..=r).step_by(3){let xx=(x+dx).clamp(0,w as i32-1);let c=src[(y as usize)*w+xx as usize];for k in 0..4{s[k]+=c[k];}n+=1.0;}blurred[(y as usize)*w+x as usize]=s.map(|v|v/n);}}
                            let src=blurred.clone();
                            for y in 0..hgt as i32 {for x in 0..w as i32 {let mut s=[0.0f32;4];let mut n=0.0;for dy in (-r..=r).step_by(3){let yy=(y+dy).clamp(0,hgt as i32-1);let c=src[(yy as usize)*w+x as usize];for k in 0..4{s[k]+=c[k];}n+=1.0;}blurred[(y as usize)*w+x as usize]=s.map(|v|v/n);}}
                        }
                        for y in 0..hgt {for x in 0..w {
                            let (px,py)=(x as f32+0.5,y as f32+0.5);
                            let inside=pieces.iter().any(|p|{px>=p.x&&px<p.x+p.w&&py>=p.y&&py<p.y+p.h&&{let cx=px.clamp(p.x+p.radius,p.x+p.w-p.radius);let cy=py.clamp(p.y+p.radius,p.y+p.h-p.radius);(px-cx).powi(2)+(py-cy).powi(2)<=p.radius*p.radius}});
                            if inside {raster.buf[y*w+x]=blurred[y*w+x];}
                        }}
                    }
                    raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    let image=image::RgbaImage::from_raw(w as u32,hgt as u32,rgba).unwrap();
                    let crop=if open{image::imageops::crop_imm(&image,120,0,1040,400)}else{image::imageops::crop_imm(&image,300,0,680,110)};
                    crop.to_image().save(dest.join(format!("{}-{}-{}.png",skin.key(),shape.key(),if open{"open"}else{"closed"}))).unwrap();
                }
                raster.free_textures(&output.textures_delta);
            }
        }
    }
}

#[test]
#[ignore = "按需导出：带 AI 额度与吉祥物的收起态，以及设置页常驻排列预览"]
fn skin_gallery_resident(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_SKIN_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    for skin in crate::model::Skin::ALL {
        for shape in crate::model::IslandShape::ALL {
            let mut h=H::new();h.app.settings.skin=skin;h.app.settings.island_shape=shape;h.app.opts.pet_idle=0.0;
            h.quota_accounts(2);h.app.quota.config.colors.insert("preview-0".into(),crate::quota::Accent::Blue);
            // 岛体收起态
            let (w,hgt)=((crate::island::CANVAS_W*2.0) as usize,(WIN_H*2.0) as usize);
            let mut raster=crate::preview_raster::Raster::new(w,hgt);
            for frame in 0..90 {
                let t=frame as f64/60.0;
                let env=Env{t,clock:"21:54:00".into(),today:"2026-11-04".into(),date:"11月4日 周三".into(),backdrop:Some([0.55,0.68,0.75]),glass_blur:true,..Default::default()};
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(t),
                    viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
                let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));h.app.frame(ui,win,&env);});});
                raster.update_textures(&output.textures_delta);
                if frame==89 {
                    raster.buf.fill([0.62,0.72,0.78,1.0]);
                    raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    let image=image::RgbaImage::from_raw(w as u32,hgt as u32,rgba).unwrap();
                    image::imageops::crop_imm(&image,300,0,680,90).to_image().save(dest.join(format!("bar-{}-{}.png",skin.key(),shape.key()))).unwrap();
                }
                raster.free_textures(&output.textures_delta);
            }
            // 设置页常驻排列预览
            let mut raster=crate::preview_raster::Raster::new(1200,300);
            for frame in 0..3 {
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(600.0,150.0))),time:Some(frame as f64),
                    viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
                let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().show(ctx,|ui|{h.app.resident_order_settings(ui);});});
                raster.update_textures(&output.textures_delta);
                if frame==2 {
                    raster.buf.fill([0.13,0.13,0.15,1.0]);
                    raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    image::RgbaImage::from_raw(1200,300,rgba).unwrap().save(dest.join(format!("resident-{}-{}.png",skin.key(),shape.key()))).unwrap();
                }
                raster.free_textures(&output.textures_delta);
            }
        }
    }
}

#[test]
#[ignore = "按需导出：浅色皮肤下每个页面的展开态，用于排查配色"]
fn skin_gallery_pages(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_SKIN_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    let skins=std::env::var("DAOZAI_SKINS").unwrap_or_else(|_|"glass_light,cream,classic".into());
    for skin in skins.split(',').map(crate::model::Skin::parse) {
        for page in 0..9usize {
            let mut h=H::new();h.app.settings.skin=skin;h.app.settings.island_shape=crate::model::IslandShape::Pill;
            h.quota_accounts(2);
            h.app.todos.items.push(crate::todo::TodoItem{text:"整理岛仔皮肤反馈".into(),done_on:None,due:Some("2026-11-05 10:00".into()),reminded:false,..Default::default()});
            h.app.todos.items.push(crate::todo::TodoItem{text:"已完成的事项".into(),done_on:Some("2026-11-04".into()),due:None,reminded:false,..Default::default()});
            h.media=crate::music::NowPlaying{title:"Paper Boats".into(),artist:"The Lanterns".into(),position:64.0,duration:201.0,playing:true,..Default::default()};
            let (w,hgt)=((crate::island::CANVAS_W*2.0) as usize,(WIN_H*2.0) as usize);
            let mut raster=crate::preview_raster::Raster::new(w,hgt);
            for frame in 0..140 {
                let t=frame as f64/60.0;
                if frame==40 { if page==8 {h.app.open_calendar_editor(None);} else {h.app.show_page(page);} }
                let env=Env{t,clock:"22:46:00".into(),today:"2026-11-04".into(),date:"11月4日 周三".into(),media:h.media.clone(),cursor:Some(Pos2::new(WIN_W/2.0,20.0)),backdrop:Some([0.8,0.84,0.88]),glass_blur:true,..Default::default()};
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(t),
                    viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
                let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));h.app.frame(ui,win,&env);});});
                raster.update_textures(&output.textures_delta);
                if frame==139 {
                    raster.buf.fill([0.80,0.84,0.88,1.0]);
                    raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    let image=image::RgbaImage::from_raw(w as u32,hgt as u32,rgba).unwrap();
                    image::imageops::crop_imm(&image,160,0,960,500).to_image().save(dest.join(format!("page-{}-{page}.png",skin.key()))).unwrap();
                }
                raster.free_textures(&output.textures_delta);
            }
        }
    }
}

#[test]
#[ignore = "按需导出设置页预览：DAOZAI_SETTINGS_PREVIEW_DIR=目录 cargo test settings_gallery -- --ignored"]
fn settings_gallery(){
    use crate::island::settings_theme::SettingsTheme;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_SETTINGS_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    let width:f32=std::env::var("DAOZAI_SETTINGS_WIDTH").ok().and_then(|v|v.parse().ok()).unwrap_or(690.0);
    let (w,hgt)=(width,2400.0f32);
    let scale=1.5f32;
    for (mode,dark) in [(SettingsTheme::Dark,true),(SettingsTheme::Light,false)] {
        for tab in [0usize,16,14,6,15,2,3,4,5,17,1,7,8,11,12,9] {
            let mut h=H::new();h.app.open_settings();h.ctx.set_embed_viewports(false);
            h.quota_accounts(2);
            h.app.settings.settings_theme=mode;h.app.settings_tab=tab;
            let output=std::sync::Arc::new(std::sync::Mutex::new(None));let capture=output.clone();
            egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
                let id=viewport.ids.this;
                let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(w,hgt))),
                    viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()})).collect(),..Default::default()};
                *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
            });
            let mut raster=crate::preview_raster::Raster::new((w*scale) as usize,(hgt*scale) as usize);
            for frame in 0..4 {
                let root=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));
                raster.update_textures(&root.textures_delta);
                let child=output.lock().unwrap().take().unwrap();
                raster.update_textures(&child.textures_delta);
                if frame==3 {
                    raster.buf.fill(if dark{[0.08,0.08,0.09,1.0]}else{[0.96,0.96,0.97,1.0]});
                    raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    let image=image::RgbaImage::from_raw((w*scale) as u32,(hgt*scale) as u32,rgba).unwrap();
                    // 去掉页面下方的空白
                    let bg=image.get_pixel(((w-20.0)*scale) as u32,((hgt-5.0)*scale) as u32).0;
                    let mut bottom=image.height()-1;
                    'scan: while bottom>200 { for x in (270*scale as u32)..image.width()-10 { if image.get_pixel(x,bottom).0!=bg {break 'scan;} } bottom-=1; }
                    image::imageops::crop_imm(&image,0,0,image.width(),(bottom+20).min(image.height())).to_image()
                        .save(dest.join(format!("settings-{}-{tab:02}.png",if dark{"dark"}else{"light"}))).unwrap();
                }
                raster.free_textures(&child.textures_delta);
            }
        }
    }
}

#[test]
fn dragging_a_component_into_the_tray_toggles_membership(){
    use crate::island::bar_layout::{QUOTA,WEATHER};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(h:&mut H,events:Vec<egui::Event>)->[Pos2;2]{
        let mut points=[Pos2::ZERO;2];
        h.t+=0.02;
        h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,300.0))),time:Some(h.t),events,..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                h.app.resident_order_settings(ui);
                for (index,slot) in [WEATHER,QUOTA].into_iter().enumerate(){
                    points[index]=ui.ctx().read_response(ui.id().with(("resident-icon",slot))).expect("天气与额度都应有拖拽入口").rect.center();
                }
            });
        });
        points
    }
    let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    let mut h=H::new();
    draw(&mut h,vec![]);let [weather,_]=draw(&mut h,vec![]);
    assert!(h.app.settings.tray_weather&&h.app.settings.tray_quota,"默认三个成员都在事件容器");
    draw(&mut h,vec![egui::Event::PointerMoved(weather),button(weather,true)]);
    let outside=weather+egui::vec2(0.0,60.0);
    draw(&mut h,vec![egui::Event::PointerMoved(outside)]);
    draw(&mut h,vec![egui::Event::PointerMoved(outside)]);
    draw(&mut h,vec![button(outside,false)]);
    assert!(!h.app.settings.tray_weather,"拖出事件区后改为常驻");
    assert!(h.app.settings.resident_order.is_empty(),"拖入拖出不应改变排列");
    let [weather,quota]=draw(&mut h,vec![]);
    draw(&mut h,vec![egui::Event::PointerMoved(weather),button(weather,true)]);
    let below=weather+egui::vec2(0.0,60.0);let near_tray=Pos2::new(quota.x,below.y);
    draw(&mut h,vec![egui::Event::PointerMoved(below)]);
    draw(&mut h,vec![egui::Event::PointerMoved(near_tray)]);
    draw(&mut h,vec![egui::Event::PointerMoved(quota)]);
    draw(&mut h,vec![button(quota,false)]);
    assert!(h.app.settings.tray_weather,"拖回事件区后恢复事件显示");
    assert!(h.app.settings.resident_order.is_empty());
}

#[test]
fn tray_membership_is_committed_on_release(){
    use crate::island::bar_layout::{CLOCK,WEATHER};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(h:&mut H,events:Vec<egui::Event>)->(Pos2,Pos2){
        let mut points=(Pos2::ZERO,Pos2::ZERO);
        h.t+=0.02;
        h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,300.0))),time:Some(h.t),events,..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                h.app.resident_order_settings(ui);
                points.0=ui.ctx().read_response(ui.id().with(("resident-icon",WEATHER))).expect("天气在容器里").rect.center();
                points.1=ui.ctx().read_response(ui.id().with(("resident-icon",CLOCK))).expect("时钟在常驻行").rect.center();
            });
        });
        points
    }
    let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    let mut h=H::new();h.app.settings.pet=false;
    draw(&mut h,vec![]);let (weather,clock)=draw(&mut h,vec![]);
    draw(&mut h,vec![egui::Event::PointerMoved(weather),button(weather,true)]);
    draw(&mut h,vec![egui::Event::PointerMoved(weather+egui::vec2(0.0,10.0))]);
    for _ in 0..3{draw(&mut h,vec![egui::Event::PointerMoved(clock)]);}
    assert!(h.app.settings.tray_weather,"拖动过程中成员关系不变，容器边界不跳");
    draw(&mut h,vec![button(clock,false)]);
    assert!(!h.app.settings.tray_weather,"松手后才按落点移出容器");
}

#[test]
fn tray_flags_round_trip(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let _h=H::new();
    let defaults=crate::model::Settings::default();
    assert!(defaults.tray_weather&&defaults.tray_quota&&defaults.tray_focus,"默认三个成员都在事件容器");
    assert!(!defaults.tray_mail&&!defaults.tray_health&&!defaults.tray_shelf&&!defaults.tray_todo&&!defaults.tray_calendar&&!defaults.tray_music&&!defaults.tray_battery,"页面徽标、音乐与电量默认常驻");
    let mut settings=crate::model::Settings::default();
    settings.tray_weather=false;settings.tray_quota=false;settings.tray_focus=true;
    settings.tray_mail=true;settings.tray_health=true;settings.tray_shelf=true;settings.tray_todo=true;settings.tray_calendar=true;settings.tray_music=true;settings.tray_battery=true;
    settings.tray_order=vec![crate::island::bar_layout::MUSIC,crate::island::bar_layout::WEATHER,crate::island::bar_layout::QUOTA,crate::island::bar_layout::FOCUS,crate::island::bar_layout::MAIL,crate::island::bar_layout::HEALTH,crate::island::bar_layout::TODO,crate::island::bar_layout::SHELF,crate::island::bar_layout::CALENDAR,crate::island::bar_layout::BATTERY];
    settings.save();
    let loaded=crate::model::Settings::load();
    assert!(!loaded.tray_weather&&!loaded.tray_quota&&loaded.tray_focus,"原有开关应持久化");
    assert!(loaded.tray_mail&&loaded.tray_health&&loaded.tray_shelf&&loaded.tray_todo&&loaded.tray_calendar&&loaded.tray_music&&loaded.tray_battery,"新增开关应持久化");
    assert_eq!(loaded.tray_order,settings.tray_order,"容器顺序应持久化");
    let path=crate::model::config_dir().join("settings.txt");
    std::fs::write(&path,"tray_order=1,2,3\n").unwrap();
    assert!(crate::model::Settings::load().tray_order.is_empty(),"无效容器顺序回退默认");
}

#[test]
fn tray_members_show_together_in_the_dragged_order(){
    use crate::island::bar_layout::{MUSIC,QUOTA,FOCUS,MAIL,HEALTH,TODO,SHELF,CALENDAR,WEATHER,BATTERY};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.pet=false;h.app.settings.tray_music=true;
    h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"杭州".into(),temp:9.0,code:0,day:true,..Default::default()});
    h.media=crate::music::NowPlaying{source:"cloudmusic.exe".into(),title:"Track A".into(),artist:"Artist".into(),playing:true,..Default::default()};
    for _ in 0..20{h.frame(vec![]);}
    // 同一轮刷新里天气和音乐都变化，两个事件同时进入显示窗口。
    h.app.weather.snapshot=Some(crate::weather::Snapshot{place:"杭州".into(),temp:10.0,code:0,day:true,..Default::default()});
    h.media.title="Track B".into();
    for _ in 0..5{h.frame(vec![]);}
    let items=h.app.tray_active_items();
    assert!(items.contains(&crate::island::event_tray::Item::Weather)&&items.contains(&crate::island::event_tray::Item::Music),"重叠事件应同时显示");
    assert_eq!(items.first(),Some(&crate::island::event_tray::Item::Weather),"默认天气排在音乐之前");
    let both=h.app.tray_width(&h.ctx).expect("两个成员同时显示时应有容器宽度");
    assert!(both>crate::island::bar_layout::slot_width(MUSIC)+crate::island::event_tray::ITEM_GAP,"容器宽度应容纳两个成员");
    // 拖动后的顺序决定并排顺序。
    h.app.settings.tray_order=vec![MUSIC,QUOTA,FOCUS,MAIL,HEALTH,TODO,SHELF,CALENDAR,WEATHER,BATTERY];
    for _ in 0..3{h.frame(vec![]);}
    let items=h.app.tray_active_items();
    assert_eq!(items.first(),Some(&crate::island::event_tray::Item::Music),"音乐应排到最前");
    assert_eq!(items.get(1),Some(&crate::island::event_tray::Item::Weather),"天气紧随其后");
}

#[test]
#[cfg(windows)]
fn settings_window_memory_restores_only_the_child_viewport_and_keeps_live_resizing(){
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.ctx.set_embed_viewports(false);h.app.open_settings();
    let original=(h.app.settings.island_size,h.app.settings.dock,h.app.settings.display_id.clone());
    let size=std::sync::Arc::new(std::sync::Mutex::new(egui::vec2(900.0,600.0)));
    let resized=size.clone();
    egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
        let id=viewport.ids.this;let size=*resized.lock().unwrap();
        let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,size)),
            viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.0),
                inner_rect:Some(Rect::from_min_size(egui::pos2(100.0,140.0),size)),
                outer_rect:Some(Rect::from_min_size(egui::pos2(92.0,100.0),size+egui::vec2(16.0,48.0))),..Default::default()})).collect(),..Default::default()};
        let _=ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx));
    });
    let frame=|h:&mut H|h.ctx.run(egui::RawInput{viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(1.75),..Default::default()})).collect(),..Default::default()},|ctx|h.app.settings_ui(ctx,false));
    let _=frame(&mut h);h.app.settings_open=false;let _=frame(&mut h);
    *size.lock().unwrap()=egui::vec2(690.0,560.0);h.app.open_settings();let root=frame(&mut h);
    assert!(root.viewport_output.iter().filter(|(id,_)|**id!=egui::ViewportId::ROOT).flat_map(|(_,v)|&v.commands).any(|c|matches!(c,egui::ViewportCommand::OuterPosition(_))),"重开时应恢复子窗口");
    assert!(!root.viewport_output.get(&egui::ViewportId::ROOT).unwrap().commands.iter().any(|c|matches!(c,egui::ViewportCommand::OuterPosition(_)|egui::ViewportCommand::InnerSize(_))));
    *size.lock().unwrap()=egui::vec2(1100.0,700.0);let live=frame(&mut h);
    assert!(!live.viewport_output.values().flat_map(|v|&v.commands).any(|c|matches!(c,egui::ViewportCommand::InnerSize(_))),"正常拖动不能每帧强制恢复旧尺寸");
    assert_eq!((h.app.settings.island_size,h.app.settings.dock,h.app.settings.display_id.clone()),original);
}

#[test]
fn closed_modules_stop_their_reminders_and_timers(){
    use crate::island::{FOCUS_PAGE,TODO_PAGE,WATER_PAGE};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for open in [true,false] {
        let mut h=H::new();h.frame(vec![]);
        h.app.pomo.running=true;h.app.pomo.remaining=0.5;
        for page in [FOCUS_PAGE,TODO_PAGE,WATER_PAGE] {h.app.set_page_visible(page,open);}
        h.app.water.remind=true;h.app.water.drunk_ml=0;h.app.last_reminder=h.t-1e6;
        h.app.todos.items.push(crate::todo::TodoItem{text:"交周报".into(),due:Some("2026-10-28 09:40".into()),..Default::default()});
        if !open {assert!(!h.app.pomo.running,"关闭专注模块时立即停止计时");h.app.pomo.running=true;}
        for _ in 0..60 {h.frame(vec![]);}
        assert_eq!(raised(&h,"喝水时间到了"),open,"饮水模块开启={open}");
        assert_eq!(raised(&h,"待办到时间了"),open,"待办模块开启={open}");
        assert_eq!(raised(&h,"专注计时"),open,"专注模块开启={open}");
        if !open {assert!(!h.app.pomo.running && h.app.todos.items[0].due.is_some() && !h.app.todos.items[0].reminded,"关闭期间不消耗提醒，重新开启后仍会提醒");}
    }
}

#[test]
fn background_services_follow_the_module_switch(){
    use crate::island::{CLIPBOARD_PAGE,MAIL_PAGE,QUOTA_PAGE,SHELF_PAGE};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.mail_enabled=true;h.app.settings.clipboard_history=true;h.app.quota.config.enabled=true;
    assert!(h.app.mail_active() && h.app.clipboard_active() && h.app.quota_active());
    for page in [MAIL_PAGE,QUOTA_PAGE,SHELF_PAGE,CLIPBOARD_PAGE] {h.app.set_page_visible(page,false);}
    assert!(!h.app.mail_active() && !h.app.clipboard_active() && !h.app.quota_active(),"页面开关就是模块总开关");
    h.frame(vec![]);
    assert!(!h.app.clipboard.is_enabled(),"剪贴板关闭后不再记录");
    for page in [MAIL_PAGE,QUOTA_PAGE,SHELF_PAGE,CLIPBOARD_PAGE] {h.app.set_page_visible(page,true);}
    h.app.settings.mail_enabled=false;h.app.settings.clipboard_history=false;h.app.quota.config.enabled=false;
    assert!(!h.app.mail_active() && !h.app.clipboard_active() && !h.app.quota_active(),"模块内的开关仍然有效");
}

#[test]
fn ai_monitor_follows_its_module_switch(){
    use crate::island::AI_PAGE;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let ctx=h.ctx.clone();
    assert!(h.app.agent.active,"AI 会话页默认开启时启动监控");
    h.app.settings.visible_pages[AI_PAGE]=false;
    h.app.reconcile_agent_monitor(&ctx);
    assert!(!h.app.agent.active && h.app.agent.store.sessions.is_empty(),"关闭页面后停止监控并清空会话");
    h.app.settings.visible_pages[AI_PAGE]=true;
    h.app.reconcile_agent_monitor(&ctx);
    assert!(h.app.agent.active,"重新开启页面后恢复监控");
}

#[test]
fn every_module_settings_tab_maps_to_its_page(){
    // 标题直接读侧栏数据，不再维护一份平行的 titles 数组。
    let mut pages:Vec<_>=(0..=crate::island::extras::SETTINGS_TAB_WINDOWS).filter_map(crate::island::extras::settings_tab_module).collect();
    for tab in 0..=crate::island::extras::SETTINGS_TAB_WINDOWS {
        if let Some(page)=crate::island::extras::settings_tab_module(tab) {
            let title=crate::island::extras::settings_tab_group_for_tests(tab);
            assert_eq!(title.chars().next(),crate::island::PAGE_NAMES[page].chars().next(),"{title} ↔ {}",crate::island::PAGE_NAMES[page]);
        }
    }
    pages.sort();assert_eq!(pages,(0..crate::island::PAGE_NAMES.len()).collect::<Vec<_>>(),"每个功能页面各有一个设置分页");
}
