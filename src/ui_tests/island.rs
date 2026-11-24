//! 岛体布局、停靠、悬停与动效。
use super::*;

#[test]
fn island_meshes_stay_valid_after_long_uptime_and_repeated_hover(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.page=0;h.power=(Some(100),true);
    let mut frames=0;
    for days in [0.0,1.0,3.0,7.0,30.0] {
        h.t=days*86400.0;
        for skin in crate::model::Skin::ALL {
            h.app.settings.skin=skin;
            for scale in [1.0,1.25,1.5] {
                h.ctx.set_pixels_per_point(scale);
                for frame in 0..120 {
                    let point=if frame%60<30{Pos2::new(WIN_W/2.0,15.0)}else{Pos2::new(900.0,250.0)};
                    h.cursor=Some(point);h.frame(vec![egui::Event::PointerMoved(point)]);
                    let meshes=h.ctx.tessellate(h.shapes.clone(),h.ctx.pixels_per_point());
                    for primitive in meshes {if let egui::epaint::Primitive::Mesh(mesh)=primitive.primitive {
                        assert!(mesh.is_valid(),"运行 {days} 天后的网格索引必须有效");
                        assert!(mesh.vertices.iter().all(|v|v.pos.is_finite()&&v.uv.is_finite()),"运行 {days} 天后展开不能出现非有限坐标");
                        assert!(mesh.vertices.iter().all(|v|v.pos.x.abs()<10000.0&&v.pos.y.abs()<10000.0),"运行 {days} 天后顶点不能拉到窗口远处");
                    }}
                    frames+=1;
                }
            }
        }
    }
    eprintln!("长运行绘制检查：{frames} 帧；覆盖 0、1、3、7、30 天及全部皮肤、三种缩放。");
}

#[test]
fn resized_font_atlases_stop_reuploading_and_release_unused_sizes(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let ctx=egui::Context::default();crate::visuals::theme::load_fonts(&ctx);
    ctx.set_embed_viewports(false);
    let child=egui::ViewportId::from_hash_of("font-upload-settings");
    let children=std::sync::Arc::new(Mutex::new(Vec::new()));let capture=children.clone();
    egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
        let id=viewport.ids.this;
        let input=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(640.0,270.0))),max_texture_side:Some(2048),
            viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.0),..Default::default()})).collect(),..Default::default()};
        capture.lock().unwrap().push(ctx.run(input,|ctx|(viewport.viewport_ui_cb)(ctx)));
    });
    let frame=|ppp:f32,settings:bool|{
        let mut viewports=std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect::<egui::ViewportIdMap<_>>();
        if settings{viewports.insert(child,egui::ViewportInfo{native_pixels_per_point:Some(1.0),..Default::default()});}
        let input=egui::RawInput{viewports,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(640.0,270.0))),max_texture_side:Some(2048),..Default::default()};
        let draw=|ctx:&egui::Context|{egui::CentralPanel::default().show(ctx,|ui|{ui.label("专注时间 25:00 · 休息 · 音乐与歌词");});};
        let root=ctx.run(input,|ctx|{draw(ctx);if settings{ctx.show_viewport_immediate(child,Default::default(),|ctx,_|draw(ctx));}});
        let mut outputs=std::mem::take(&mut *children.lock().unwrap());outputs.push(root);
        outputs
    };
    for settings in [false,true,false] {
        for ppp in [1.0,1.4,1.2,0.8,1.4,1.0] {
            for _ in 0..3{frame(ppp,settings);}
            let start=std::time::Instant::now();let mut bytes=0usize;
            for _ in 0..60 {for output in frame(ppp,settings){
                bytes+=output.textures_delta.set.iter().map(|(_,delta)|delta.image.width()*delta.image.height()*4).sum::<usize>();
            }}
            println!("缩放={ppp:.2} 设置={settings} 60帧上传={bytes}字节 耗时={:?}",start.elapsed());
            assert_eq!(bytes,0,"大小稳定后不能重复上传字形图集，包括设置同时打开时");
            let textures=ctx.tex_manager();let textures=textures.read();
            let fonts=textures.allocated().filter(|(_,meta)|meta.name.starts_with("egui_font_texture")).count();
            assert!(fonts<=if settings&&ppp!=1.0{2}else{1},"旧倍率的字体贴图应释放：{fonts}");
        }
    }
}

#[test]
fn glass_pill_rim_stays_on_outline_while_collapsing() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for ppp in [1.0,1.25,1.5,2.0] {
        let mut h=H::new();
        h.app.settings.skin=crate::model::Skin::GlassLight;
        h.app.settings.island_shape=crate::model::IslandShape::Pill;
        h.ctx.set_zoom_factor(ppp);
        h.hover(Pos2::new(WIN_W/2.0,15.0),1.5);
        assert!(h.app.progress()>0.99);
        h.cursor=None;
        let mut raster=crate::preview_raster::Raster::new((crate::island::CANVAS_W*ppp) as usize,(WIN_H*ppp) as usize);
        let mut checked=0;
        for frame in 0..100 {
            h.frame(vec![egui::Event::PointerGone]);
            let rims=h.shapes.iter().filter_map(|s|match &s.shape {
                egui::Shape::Rect(r) if r.rect.width()>100.0 && r.rect.height()>=10.0=>Some((s.clone(),r.rect,if r.stroke.is_empty(){r.fill.a() as f32/255.0}else{0.0})),
                _=>None,
            }).collect::<Vec<_>>();
            for (shape,rect,alpha) in rims {
                checked+=1;
                raster.buf.fill([0.0;4]);
                raster.draw(&h.ctx.tessellate(vec![shape],ppp),ppp);
                let y=(rect.center().y*ppp).floor() as usize;
                let left=((rect.left()+4.0)*ppp).ceil() as usize;
                let right=((rect.right()-4.0)*ppp).floor() as usize;
                for x in left..right {
                    assert!((raster.buf[y*raster.w+x][3]-alpha).abs()<0.02,"胶囊中心出现接缝：ppp={ppp}, frame={frame}, rect={rect:?}, x={x}, alpha={alpha}, actual={}",raster.buf[y*raster.w+x][3]);
                }
            }
        }
        assert!(checked>=100,"应覆盖完整收起动画");
    }
}

#[test]
fn oled_shift_waits_for_the_panel_to_finish_closing() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.oled_shift=true;
    h.app.show_page(0);
    h.hover(H::local(200.0,60.0),5.0);
    assert!(h.app.open);
    h.cursor=Some(Pos2::new(600.0,180.0));
    for _ in 0..120 {
        h.frame(vec![]);
        if !h.app.open {break;}
    }
    assert!(!h.app.open);
    assert!(h.app.progress()>0.1,"测试应停在收起动画尚未完成时");
    assert!(!h.app.oled_shift_idle(&h.ctx),"收起动画仍有 {:.1}% 展开程度时不能恢复位移",h.app.progress()*100.0);
    h.hover(Pos2::new(600.0,180.0),1.0);
    assert!(h.app.oled_shift_idle(&h.ctx),"动画完成后应恢复位移");
}

#[test]
fn oled_shift_pauses_during_hover_intent() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.oled_shift=true;
    h.hover(Pos2::new(600.0,180.0),1.0);
    assert!(h.app.oled_shift_idle(&h.ctx));
    h.hover(Pos2::new(WIN_W/2.0,12.0),0.05);
    assert!(!h.app.open,"悬停意图缓冲期间尚未展开");
    assert!(h.hit.contains(Pos2::new(WIN_W/2.0,12.0)),"鼠标已进入岛体交互区域");
    assert!(!h.app.oled_shift_idle(&h.ctx),"悬停交互期间不能移动岛体");
}

#[test]
fn oled_shift_pauses_on_the_first_pointer_press_frame() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();let away=Pos2::new(600.0,180.0);
    h.hover(away,1.0);assert!(h.app.oled_shift_idle(&h.ctx));
    h.frame(vec![egui::Event::PointerButton{pos:away,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
    assert!(!h.app.oled_shift_idle(&h.ctx),"按下的首帧必须暂停位移");
    h.frame(vec![]);assert!(!h.app.oled_shift_idle(&h.ctx),"按住期间必须暂停位移");
    h.frame(vec![egui::Event::PointerButton{pos:away,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
    assert!(h.app.oled_shift_idle(&h.ctx),"松开且没有交互后应恢复位移");
}

#[test]
fn oled_shift_waits_for_side_panels_to_finish_closing() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for dock in [crate::model::Dock::Left,crate::model::Dock::Right] {
        let mut h=H::new();h.app.settings.dock=dock;h.app.show_page(0);
        let inside=Some(Pos2::new(if dock==crate::model::Dock::Left {100.0}else{400.0},300.0));
        for _ in 0..300 {side_frame(&mut h,inside);}
        assert!(h.app.open);
        for _ in 0..120 {side_frame(&mut h,None);if !h.app.open {break;}}
        assert!(!h.app.open && h.app.progress()>0.1);
        assert!(!h.app.oled_shift_idle(&h.ctx),"{dock:?} 收起动画期间不能移动");
        for _ in 0..120 {side_frame(&mut h,None);}
        assert!(h.app.oled_shift_idle(&h.ctx),"{dock:?} 动画完成后应恢复位移");
    }
}

#[test]
fn hover_expands_with_overshoot_and_leave_collapses() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.hover(Pos2::new(WIN_W / 2.0, 12.0), 0.1);
    assert!(h.app.progress() < 0.1, "意图缓冲期内不应展开");
    let mut peak: f32 = 0.0;
    for _ in 0..60 {
        h.hover(Pos2::new(WIN_W / 2.0, 12.0), 1.0 / 60.0);
        peak = peak.max(h.app.progress());
    }
    assert!(peak > 1.002 && peak < 1.02, "展开应有轻微过冲, peak={peak}");
    assert!((h.app.progress() - 1.0).abs() < 0.02);
    h.cursor = Some(Pos2::new(WIN_W / 2.0, PH + crate::island::STATUS_H + 30.0));
    for _ in 0..40 {
        h.frame(vec![]);
    }
    assert!(h.app.progress() < 0.02, "移开后应收起");
}

#[test]
fn browser_minimal_closes_the_open_island_but_still_allows_deliberate_hover(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.browser_minimal=true;
    h.app.show_page(0);h.hover(H::local(300.0,60.0),0.8);
    assert!(h.app.open);
    h.browser_at_top=Some(true);h.frame(vec![]);
    assert!(!h.app.open,"切到浏览器时不能继续保留展开状态遮挡标签栏");
    assert!(!h.hit.contains(Pos2::new(WIN_W/2.0,12.0)),"收起动画期间也应让标签栏接收点击");
    h.hover(Pos2::new(WIN_W/2.0,2.0),0.25);
    assert!(!h.app.open,"短暂碰到刘海不能误触");
    h.hover(Pos2::new(WIN_W/2.0,2.0),0.6);
    assert!(h.app.open,"仍应能主动悬停打开岛");
}

#[test]
fn minimal_hover_excludes_the_surrounding_browser_tabs() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.set_minimal(true);
    for _ in 0..90 {h.frame(vec![]);}
    for cursor in [
        Pos2::new(WIN_W/2.0,crate::island::BAR_H_MIN+1.0),
        Pos2::new(WIN_W/2.0+crate::island::BAR_W_MIN/2.0+2.0,2.0),
        Pos2::new(WIN_W/2.0-crate::island::BAR_W_MIN/2.0-2.0,2.0),
    ] {
        assert!(!h.hit.contains(cursor),"刘海外的标签栏应接收鼠标操作");
        h.hover(cursor,1.0);
        assert!(!h.app.open,"鼠标未触碰避让细线时不得展开");
    }
}

#[test]
fn minimal_hover_requires_continuous_deliberate_contact() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.set_minimal(true);
    for _ in 0..90 {h.frame(vec![]);}
    let inside=Pos2::new(WIN_W/2.0,2.0);
    h.hover(inside,0.25);
    assert!(!h.app.open,"短暂经过避让细线不得展开");
    h.hover(Pos2::new(WIN_W/2.0,8.0),0.1);
    h.hover(inside,0.25);
    assert!(!h.app.open,"移出刘海应重新计算停留时间");
    h.hover(inside,0.7);
    assert!(h.app.open,"在刘海内持续停留仍应展开");
    h.click(H::local(398.0,68.0));
    assert_eq!(h.app.page,1,"展开后应能正常操作面板");
}

#[test]
fn minimal_hover_does_not_catch_tabs_during_collapse() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.set_minimal(true);
    h.hover(Pos2::new(WIN_W/2.0,2.0),1.0);
    assert!(h.app.open);
    h.cursor=Some(Pos2::new(WIN_W/2.0,PH+crate::island::STATUS_H+30.0));
    for _ in 0..30 {
        h.frame(vec![]);
        if !h.app.open {break;}
    }
    assert!(!h.app.open);
    assert!(h.app.progress()>0.04,"检查收起动画仍在进行时的命中范围");
    let tab=Pos2::new(WIN_W/2.0,12.0);
    assert!(!h.hit.contains(tab),"收起动画不得继续拦截标签栏");
    h.hover(tab,1.0);
    assert!(!h.app.open);
}

#[test]
fn wheel_scrolls_lists_only_inside_content_and_respects_edges(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.todos.items=(0..4).map(|i|crate::todo::TodoItem{text:format!("任务 {i}"),done_on:None,due:None,reminded:false,..Default::default()}).collect();
    h.app.shelf.items=(0..6).map(|i|std::path::PathBuf::from(format!("scroll-test-{i}.txt"))).collect();
    for (page,next) in [(3,11),(4,5)] {
        h.app.show_page(page);h.hover(H::local(300.0,60.0),1.0);
        h.wheel(-40.0);
        assert_eq!(h.app.page,page,"有更多内容时应先滚动列表");
        assert_eq!(if page==3 {h.app.shelf_first}else{h.app.todo_first},1);
        h.hover(H::local(300.0,60.0),0.25);h.wheel(-40.0);
        assert_eq!(h.app.page,page,"内容溢出时滚到底也应留在本页");
        h.app.show_page(page);h.hover(H::local(210.0,18.0),0.3);h.wheel(-40.0);
        assert_eq!(h.app.page,next,"顶栏滚轮不应被内容列表拦截");
    }
}

#[test]
#[ignore = "按需导出常驻组件排布与补位动画"]
fn resident_layout_gallery(){
    fn capture(h:&mut H,raster:&mut crate::preview_raster::Raster,t:f64,draw:bool)->Vec<u8>{
        let env=Env{t,clock:"09:41:00".into(),today:"2026-10-31".into(),date:"10月31日 周六".into(),battery:h.power.0,charging:h.power.1,media:h.media.clone(),bt_ready:true,..Default::default()};
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H))),time:Some(t),
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=ui.max_rect();h.app.frame(ui,win,&env);});});
        raster.update_textures(&output.textures_delta);let mut rgba=Vec::new();
        if draw {raster.buf.fill([0.09,0.11,0.18,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);raster.to_rgba8(&mut rgba);}
        raster.free_textures(&output.textures_delta);rgba
    }
    fn demo(battery:bool,accounts:usize,music:bool,focus:bool,pet:bool)->H{
        let mut h=H::new();h.app.settings.show_battery=battery;h.app.settings.pet=pet;h.app.settings.tray_quota=false;h.app.settings.tray_focus=false;h.app.settings.tray_weather=false;h.power=(Some(80),true);h.quota_accounts(accounts);
        for (i,a) in h.app.quota.config.accounts.iter().enumerate(){
            if let Some(Ok(snapshot))=h.app.quota.rows.get_mut(&a.id){
                snapshot.windows[0].remaining=82.0-i as f32*11.0;
                if i%2==0 {snapshot.windows.push(crate::quota::Window{label:"5 小时".into(),seconds:Some(18000),remaining:58.0,reset:None,amount:None});}
            }
        }
        if music {h.media=crate::music::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};}
        if focus {h.app.pomo.running=true;h.app.pomo.remaining=1283.0;}
        if pet {h.app.pet.react(crate::island::extras::PetState::Happy);}
        h
    }
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_RESIDENT_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let scenes=[(true,1,false,false,false),(false,1,false,false,false),(true,3,false,false,false),(false,3,true,false,false),
        (true,3,true,false,false),(false,3,true,true,false),(false,5,false,false,false),(false,2,false,false,true)];
    for (i,(battery,accounts,music,focus,pet)) in scenes.into_iter().enumerate(){
        let mut h=demo(battery,accounts,music,focus,pet);let mut raster=crate::preview_raster::Raster::new(1280,380);
        for frame in 0..90 {
            let rgba=capture(&mut h,&mut raster,frame as f64/60.0,frame==89);
            if frame==89 {image::save_buffer(dest.join(format!("layout-{i}.png")),&rgba,1280,380,image::ColorType::Rgba8).unwrap();}
        }
    }
    let mut h=demo(true,3,true,false,false);let mut raster=crate::preview_raster::Raster::new(1280,380);
    for frame in 0..240 {
        if frame==60 {h.app.settings.show_battery=false;}if frame==180 {h.app.settings.show_battery=true;}
        let rgba=capture(&mut h,&mut raster,frame as f64/60.0,frame%3==0);
        if frame%3==0 {
            let image=image::RgbaImage::from_raw(1280,380,rgba).unwrap();
            image::imageops::crop_imm(&image,200,0,880,80).to_image().save(dest.join(format!("frame-{frame:03}.png"))).unwrap();
        }
    }
    eprintln!("常驻组件预览：{}",dest.display());
}

#[test]
fn code_copy_confirmation_stays_below_the_island_and_does_not_capture_hover() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for detail in [false,true] {
        let mut h=H::new();
        h.app.mail.accounts=vec![crate::mail::Account {custom: None, email:"a@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
        h.app.mail.recent=vec![crate::mail::Item {received_unix: None, account:"a@qq.com".into(),from:"GitHub".into(),subject:"登录验证码".into(),time:"09:41".into(),code:Some("482913".into()),html: String::new().into(), summary:"请勿向他人透露验证码。".into(),links:vec![],meeting:None,uid:42}];
        for item in h.app.mail.recent.clone(){h.app.mail.privacy.observe(&item,true,h.app.t,chrono::Utc::now().timestamp());}
        if detail {h.app.mail.open_uid=Some(("a@qq.com".into(),42));}
        h.app.show_page(5);h.hover(H::local(300.0,60.0),4.2);
        let copy=H::local(80.0,if detail{212.0}else{70.0});
        h.click(copy);h.hover(copy,0.35);
        let confirmation=|h:&H|h.shapes.iter().find_map(|shape|match &shape.shape {
            egui::Shape::Text(text) if text.galley.text()=="验证码已复制"=>Some(text.visual_bounding_rect()),_=>None,
        });
        let bounds=confirmation(&h).expect("复制后应显示确认提示");
        assert!(bounds.top()>PH+4.0,"确认提示应在岛下方：{bounds:?}");
        assert!(h.region.contains_rect(bounds),"原生窗口区域应包含提示，避免被裁掉");
        assert!(!h.hit.intersects(bounds),"确认提示不应拦截鼠标或维持岛的展开");
        h.hover(Pos2::new(600.0,180.0),0.8);
        assert!(h.app.progress()<0.02);
        assert!(confirmation(&h).is_some(),"岛收起后确认提示仍应显示到正常结束");
        h.hover(Pos2::new(600.0,180.0),2.0);
        assert!(confirmation(&h).is_none());
        assert!(h.region.bottom()<100.0,"提示结束后应释放额外窗口区域");
    }
}

#[test]
fn remove_icons_are_centered_in_their_buttons_at_each_scale(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for scale in [1.0,1.25,1.5,2.0]{
        let mut h=H::new();h.ctx.set_pixels_per_point(scale);
        h.app.todos.items=["第一项","第二项","第三项"].into_iter().map(|text|crate::todo::TodoItem{text:text.into(),due:None,done_on:None,reminded:false,..Default::default()}).collect();
        h.app.show_page(4);h.hover(H::local(300.0,60.0),1.0);
        for i in 0..3usize{
            let button=h.ctx.read_response(egui::Id::new(("card",true,("todo-delete",i)))).unwrap().rect;
            let ink=h.shapes.iter().filter_map(|s|match &s.shape{
                egui::Shape::Text(t) if t.galley.text()=="×" && button.contains(t.visual_bounding_rect().center())=>Some(t.visual_bounding_rect()),
                egui::Shape::LineSegment{points,..} if points.iter().all(|p|button.shrink(2.0).contains(*p)) && (points[0].x-points[1].x).abs()>2.0 && (points[0].y-points[1].y).abs()>2.0=>Some(Rect::from_two_pos(points[0],points[1])),
                _=>None,
            }).fold(Rect::NOTHING,|a,b|a.union(b));
            assert!(ink.is_positive(),"删除按钮应有可见图案");
            assert!(ink.center().distance(button.center())<0.4,"缩放 {scale}：删除图案中心 {:?} 偏离按钮中心 {:?}",ink.center(),button.center());
        }
        h.click(H::local(361.5,68.0));
        assert_eq!(h.app.todos.items.iter().map(|t|t.text.as_str()).collect::<Vec<_>>(),["第一项","第三项"]);
        assert!(h.app.todos.items.iter().all(|t|t.done_on.is_none()),"删除入口不能同时勾选其他待办");
    }
}

#[test]
#[ignore="按需检查全部页面与设置的实际排版"]
fn ui_alignment_layout_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_UI_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    fn fixture()->H{
        let mut h=H::new();h.app.settings.pet=false;h.app.settings.island_lyrics=false;h.power=(Some(76),false);
        h.media=crate::music::NowPlaying{title:"愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光".into(),artist:"The Lanterns · Live Session".into(),duration:240.0,position:65.0,playing:true,..Default::default()};
        h.app.todos.items=["整理会议资料","Review design details","确认项目需求与下一阶段开发排期"].into_iter().map(|text|crate::todo::TodoItem{text:text.into(),due:None,done_on:None,reminded:false,..Default::default()}).collect();
        h.app.shelf.items=["项目设计方案与交付说明.pdf","design-preview.png","本周工作安排.docx","project-files.zip","notes.md"].into_iter().map(std::path::PathBuf::from).collect();
        h.app.mail.accounts=vec![crate::mail::Account {custom: None, email:"product-team@example.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
        h.app.mail.recent=(0..3).map(|uid|crate::mail::Item {received_unix: None, account:"product-team@example.com".into(),from:"产品与设计协作团队".into(),subject:"请确认下阶段的设计方案和开发排期，评审前同步最新进度".into(),time:"11-01 09:41".into(),code:if uid==0{Some("123456".into())}else{None},html: String::new().into(), summary:"这是一段邮件正文，需要确认当前需求与后续的产品设计安排。".repeat(20).into(),links:vec![],meeting:None,uid}).collect();
        h.quota_accounts(3);h
    }
    let collapsed=["collapsed","volume","caps","language","power","network","meeting","meeting-dingtalk","meeting-wecom","bt-headphones","bt-keyboard","bt-mouse","bt-gamepad","bt-phone","bt-other","bt-speaker"];
    let scenes=["page-0","page-1","page-2","page-3","page-4","page-5","page-6","page-7","lyrics","todo-empty","shelf-empty","mail-empty","mail-detail","reminder","water-chart","focus-running"];
    for ppp in [1.25,2.0]{for name in scenes.into_iter().chain(collapsed){
        let mut h=fixture();let page=name.strip_prefix("page-").and_then(|p|p.parse::<usize>().ok()).unwrap_or(match name{"lyrics"=>0,"shelf-empty"=>3,"mail-empty"|"mail-detail"=>5,_=>4});
        if name=="todo-empty"{h.app.todos.items.clear();}if name=="shelf-empty"{h.app.shelf.items.clear();}if name=="mail-empty"{h.app.mail.accounts.clear();}
        if !collapsed.contains(&name){h.app.show_page(page);}
        if name=="water-chart"{h.app.show_page(2);h.app.water_view=1;h.app.water.drunk_ml=1250;h.app.water.hours[9]=250;h.app.water.hours[8]=500;h.app.water.hours[7]=500;}
        if name=="focus-running"{h.app.show_page(1);h.app.pomo.running=true;h.app.pomo.remaining=1283.0;}
        if let Some(kind)=name.strip_prefix("bt-"){
            let kind=match kind{"headphones"=>crate::os::bt::DeviceKind::Headphones,"keyboard"=>crate::os::bt::DeviceKind::Keyboard,"mouse"=>crate::os::bt::DeviceKind::Mouse,"gamepad"=>crate::os::bt::DeviceKind::Gamepad,"phone"=>crate::os::bt::DeviceKind::Phone,"speaker"=>crate::os::bt::DeviceKind::Speaker,_=>crate::os::bt::DeviceKind::Other};
            h.app.bluetooth_alert(&crate::os::bt::Device{id:"preview".into(),name:"我的蓝牙设备 · 超长设备名称与连接状态".into(),battery:Some(100),kind},true);
        }
        match name{"volume"=>{h.app.volume_ui=Some((64,false));h.app.alert("系统音量","64%",0,false);},"caps"=>h.app.alert("大写锁定","已开启",0,false),"language"=>h.app.alert("键盘输入语言","中文（简体，中国）",0,false),"network"=>h.app.alert("网络连接","Wi-Fi · 网络已连接",0,false),"power"=>h.app.alert("电源已连接","当前电量 76%",0,false),"meeting"=>h.app.meeting_alert(crate::mail::Meeting{ all_day:false,title:"产品方案评审与下阶段开发排期确认".into(),start:chrono::Local::now().timestamp()+600,end:None,at:"11-01 10:00".into(),uid:"preview".into()},"邮件 · product-and-design-team@company-example.com".into(),5),_=>{}}
        if name=="meeting-dingtalk" || name=="meeting-wecom" {
            h.app.meeting_alert(crate::mail::Meeting{all_day:false,title:"项目评审与下阶段安排".into(),start:chrono::Local::now().timestamp()+600,end:None,at:String::new(),uid:name.into()},if name=="meeting-dingtalk"{"钉钉"}else{"企业微信"}.into(),7);
        }
        if name=="lyrics"{h.app.show_lyrics=true;h.app.set_lyrics(&h.media.title,&h.media.artist,240.0,crate::music::lyrics::parse_lrc("[00:00]慢慢走过这段时光\n[01:00]愿每个认真生活的人都能找到属于自己的光\n[01:10]愿风带你去远方"));}
        if name=="mail-detail"{h.app.mail.open_uid=Some(("product-team@example.com".into(),0));}
        if name=="reminder"{h.app.open_task_reminder(0);}
        let (width,height)=((WIN_W*ppp)as u32,(WIN_H*ppp)as u32);let mut raster=crate::preview_raster::Raster::new(width as usize,height as usize);
        for f in 0..90{
            let t=f as f64/60.0;let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(WIN_W,WIN_H))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
            let env=Env{t,clock:"09:41:00".into(),today:"2026-11-01".into(),date:"11月1日 周日".into(),cursor:if collapsed.contains(&name){None}else{Some(H::local(150.0,40.0))},media:h.media.clone(),battery:h.power.0,bt_ready:true,..Default::default()};
            let out=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{let win=ui.max_rect();h.app.frame(ui,win,&env);});});raster.update_textures(&out.textures_delta);
            if f==89{raster.buf.fill([0.07,0.09,0.13,1.0]);raster.draw(&h.ctx.tessellate(out.shapes,out.pixels_per_point),out.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(format!("{name}-{}.png",(ppp*100.0)as u32)),&rgba,width,height,image::ColorType::Rgba8).unwrap();}
            raster.free_textures(&out.textures_delta);
        }
    }}
    for width in [600.0,690.0]{for tab in 0..12{
        let mut h=fixture();h.app.settings_tab=tab;h.app.open_settings();h.ctx.set_embed_viewports(false);
        let size=egui::vec2(width,if width==600.0{450.0}else{560.0});let ppp=1.25;
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{let id=viewport.ids.this;let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,size)),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(ppp),focused:Some(true),..Default::default()})).collect(),focused:true,..Default::default()};*capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));});
        let (w,height)=((size.x*ppp)as u32,(size.y*ppp)as u32);let mut raster=crate::preview_raster::Raster::new(w as usize,height as usize);
        for f in 0..5{let root=h.ctx.run(egui::RawInput::default(),|ctx|h.app.settings_ui(ctx,false));raster.update_textures(&root.textures_delta);let child=output.lock().unwrap().take().unwrap();raster.update_textures(&child.textures_delta);
            if f==4{raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(format!("settings-{tab}-{}.png",width as u32)),&rgba,w,height,image::ColorType::Rgba8).unwrap();}
            raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
        }
    }}
}

#[test]
fn resident_components_can_be_dragged_saved_and_reset(){
    use crate::island::bar_layout::{CLOCK,BATTERY,WEATHER};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(h:&mut H,events:Vec<egui::Event>,slot:usize)->[Pos2;2]{
        let mut points=[Pos2::ZERO;2];
        h.t+=0.02;
        let out=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,300.0))),time:Some(h.t),events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|{h.app.resident_order_settings(ui);for (index,slot) in [slot,BATTERY].into_iter().enumerate(){points[index]=ui.ctx().read_response(ui.id().with(("resident-icon",slot))).expect("暂未显示的组件也应可以独立拖动").rect.center();}});});
        h.shapes=out.shapes;points
    }
    fn point(h:&H,label:&str)->Pos2{h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap()}
    for slot in [CLOCK,WEATHER] {
    let mut h=H::new();
    draw(&mut h,vec![],slot);let [from,to]=draw(&mut h,vec![],slot);
    let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    draw(&mut h,vec![egui::Event::PointerMoved(from),button(from,true)],slot);
    draw(&mut h,vec![egui::Event::PointerMoved(from+egui::vec2(0.0,10.0))],slot);
    draw(&mut h,vec![egui::Event::PointerMoved(to)],slot);
    for _ in 0..3{draw(&mut h,vec![],slot);}
    draw(&mut h,vec![button(to,false)],slot);
    assert_eq!(h.app.settings.resident_order.first(),Some(&slot));
    assert!(h.app.settings.resident_order.contains(&BATTERY));
    h.app.settings.save();assert_eq!(crate::model::Settings::load().resident_order,h.app.settings.resident_order);
    let reset=point(&h,"恢复默认");draw(&mut h,vec![egui::Event::PointerMoved(reset),button(reset,true)],slot);draw(&mut h,vec![button(reset,false)],slot);
    assert!(h.app.settings.resident_order.is_empty());
    }
}

#[test]
fn resident_drag_inserts_on_the_pointer_side(){
    use crate::island::bar_layout::{BATTERY,CLOCK};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(h:&mut H,events:Vec<egui::Event>)->(Pos2,Pos2){
        let mut points=(Pos2::ZERO,Pos2::ZERO);
        h.t+=0.02;
        h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,300.0))),time:Some(h.t),events,..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                h.app.resident_order_settings(ui);
                points.0=ui.ctx().read_response(ui.id().with(("resident-icon",CLOCK))).expect("时钟应可拖动").rect.center();
                points.1=ui.ctx().read_response(ui.id().with(("resident-icon",BATTERY))).expect("电量应可拖动").rect.center();
            });
        });
        points
    }
    let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    for (offset,expected) in [(-12.0,[CLOCK,BATTERY]),(12.0,[BATTERY,CLOCK])] {
        let mut h=H::new();h.app.settings.pet=false;
        draw(&mut h,vec![]);let (clock,battery)=draw(&mut h,vec![]);
        draw(&mut h,vec![egui::Event::PointerMoved(clock),button(clock,true)]);
        draw(&mut h,vec![egui::Event::PointerMoved(clock+egui::vec2(0.0,10.0))]);
        let target=battery+egui::vec2(offset,0.0);
        draw(&mut h,vec![egui::Event::PointerMoved(target)]);
        draw(&mut h,vec![egui::Event::PointerMoved(target)]);
        draw(&mut h,vec![button(target,false)]);
        assert_eq!(h.app.settings.resident_order.get(..2),Some(expected.as_slice()),"按指针在目标左右半边决定插入位置");
    }
}

#[test]
fn native_app_icon_uses_bounded_resolution(){
    let icon=crate::visuals::theme::app_icon();
    assert!(icon.width<=128 && icon.height<=128,"原生窗口图标不需要常驻完整的 1024 像素品牌原图");
    assert_eq!(icon.rgba.len(),(icon.width*icon.height*4)as usize);
    assert!(std::sync::Arc::ptr_eq(&icon,&crate::visuals::theme::app_icon()));
}

#[test]
fn hover_tooltips_fit_native_region_and_release_it_after_leaving(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for shelf in [false,true] {
        let mut h=H::new();
        let label;
        let pointer;
        if shelf {
            let path=crate::model::config_dir().join("开发项目源代码与当前维护的项目文件夹");
            std::fs::create_dir_all(&path).unwrap();
            label=path.display().to_string();h.app.shelf.add(&path).unwrap();
            h.app.show_page(3);h.hover(H::local(300.0,60.0),1.0);
            pointer=H::local(210.0,66.0);
        } else {
            h.app.settings.island_lyrics=true;
            h.media.title="测试歌曲".into();h.media.artist="歌手".into();h.media.duration=240.0;h.media.playing=true;
            h.app.set_lyrics("测试歌曲","歌手",240.0,crate::music::lyrics::parse_lrc("[00:00]测试歌词"));
            h.app.alert("邮件验证码","收到邮件，验证码 123456，请在十分钟内使用",5,false);
            h.hover(Pos2::new(600.0,220.0),1.0);
            label="邮件验证码\n收到邮件，验证码 123456，请在十分钟内使用".to_string();
            pointer=Pos2::new(WIN_W/2.0+(h.app.current_width()-44.0)/2.0+26.0,15.0);
        }
        h.hover(pointer,0.4);
        let hit_before_tooltip=h.hit;
        h.hover(pointer,1.1);
        let bounds=h.shapes.iter().find_map(|s|match &s.shape {
            egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect()),_=>None,
        }).expect("悬停后应显示完整提示");
        assert!(h.region.contains_rect(bounds),"悬停提示不能被原生窗口裁掉：{bounds:?} / {:?}",h.region);
        assert_eq!(h.hit,hit_before_tooltip,"提示不应扩大岛体交互区域");
        h.hover(Pos2::new(900.0,250.0),3.0);
        assert!(h.region.bottom()<100.0,"提示消失后应恢复收起区域");
    }
}

#[test]
fn avoidance_picker_add_remove_and_save() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.ctx.set_visuals(egui::Visuals::dark());
    assert!(h.app.settings.avoid_browsers);
    assert!(h.app.settings.avoid_apps.is_empty());
    h.app.settings.browser_minimal=true;
    h.app.avoidance.entries=vec![
        crate::model::Candidate{path:r"D:\Quark\quark.exe".into(),name:"夸克浏览器".into(),title:"新标签页".into(),browser:true},
        crate::model::Candidate{path:r"C:\Windows\System32\notepad.exe".into(),name:"记事本".into(),title:"未命名 - 记事本".into(),browser:false},
    ];
    fn draw(h:&mut H,events:Vec<egui::Event>)->egui::FullOutput {
        h.t+=1.0/60.0;
        h.ctx.run(egui::RawInput{time:Some(h.t),screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,560.0))),events,..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                ui.heading("常规");
                h.app.avoidance.rows(ui,&mut h.app.settings);
            });
            h.app.avoidance.dialog(ctx,&mut h.app.settings);
        })
    }
    fn click(h:&mut H,point:Pos2){
        draw(h,vec![egui::Event::PointerMoved(point),egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        draw(h,vec![egui::Event::PointerMoved(point),egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
    }
    fn point(out:&egui::FullOutput,label:&str)->Pos2 {
        out.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap_or_else(||panic!("未找到 {label}"))
    }
    h.app.avoidance.open=true;
    let mut raster=crate::preview_raster::Raster::new(600,560);
    let mut out=draw(&mut h,vec![]);raster.update_textures(&out.textures_delta);
    for _ in 0..40 {std::thread::sleep(std::time::Duration::from_millis(10));out=draw(&mut h,vec![]);raster.update_textures(&out.textures_delta);}
    assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="浏览器规则已覆盖")));
    if let Some(dest)=std::env::var_os("DAOZAI_AVOID_PREVIEW_DIR") {
        std::fs::create_dir_all(&dest).unwrap();raster.buf.fill([0.0,0.0,0.0,1.0]);
        raster.draw(&h.ctx.tessellate(out.shapes.clone(),out.pixels_per_point),out.pixels_per_point);
        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(std::path::Path::new(&dest).join("添加应用.png"),&rgba,600,560,image::ColorType::Rgba8).unwrap();
    }
    click(&mut h,point(&out,"记事本"));
    let out=draw(&mut h,vec![]);
    click(&mut h,point(&out,"添加"));
    assert!(!h.app.avoidance.open);
    assert_eq!(h.app.settings.avoid_apps.len(),1);
    h.app.settings.avoid_browsers=false;h.app.settings.save();
    let loaded=crate::model::Settings::load();
    assert!(!loaded.avoid_browsers);assert!(loaded.browser_minimal);
    assert_eq!(loaded.avoid_apps,h.app.settings.avoid_apps);
    let mut out=draw(&mut h,vec![]);raster.update_textures(&out.textures_delta);
    for _ in 0..40{std::thread::sleep(std::time::Duration::from_millis(20));out=draw(&mut h,vec![]);raster.update_textures(&out.textures_delta);}
    assert!(!out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="记事本")),"图标区不应常驻显示应用名称");
    if let Some(dest)=std::env::var_os("DAOZAI_AVOID_PREVIEW_DIR") {
        raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(out.shapes.clone(),out.pixels_per_point),out.pixels_per_point);
        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(std::path::Path::new(&dest).join("图标区域.png"),&rgba,600,560,image::ColorType::Rgba8).unwrap();
    }
    click(&mut h,point(&out,"×"));assert!(h.app.settings.avoid_apps.is_empty());
    h.app.settings.save();assert!(crate::model::Settings::load().avoid_apps.is_empty());

    h.app.settings.avoid_browsers=true;
    h.app.settings.avoid_apps.push(("D:/one.exe".into(),"已登记".into()));
    for (path,name,browser) in [("d:/ONE.exe","已登记",false),("D:/two.exe","工具二",false),
        ("D:/three.exe","工具三",false),("D:/browser.exe","浏览器",true)] {
        h.app.avoidance.entries.push(crate::model::Candidate{path:path.into(),name:name.into(),title:"插件面板".into(),browser});
    }
    h.app.avoidance.open=true;
    draw(&mut h,vec![]);
    let out=draw(&mut h,vec![]);
    click(&mut h,point(&out,"搜索应用或窗口标题"));
    draw(&mut h,vec![egui::Event::Text("插件面板".into())]);
    let out=draw(&mut h,vec![]);
    click(&mut h,point(&out,"全部添加"));
    // 全部添加需要二次确认：第一次只出现确认条，不添加。
    assert!(h.app.avoidance.open);assert_eq!(h.app.settings.avoid_apps.len(),1,"确认前不能添加");
    let out=draw(&mut h,vec![]);
    click(&mut h,point(&out,"确定添加"));
    assert!(!h.app.avoidance.open);
    assert_eq!(h.app.settings.avoid_apps.iter().map(|(p,_)|p.as_str()).collect::<Vec<_>>(),["D:/one.exe","D:/two.exe","D:/three.exe"]);
    h.app.settings.save();assert_eq!(crate::model::Settings::load().avoid_apps,h.app.settings.avoid_apps);
}

#[test]
fn avoidance_icon_area_stays_one_row_and_scrolls() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.ctx.set_visuals(egui::Visuals::dark());h.app.settings.browser_minimal=true;
    fn draw(h:&mut H,events:Vec<egui::Event>)->egui::FullOutput {
        h.t+=1.0/60.0;
        h.ctx.run(egui::RawInput{time:Some(h.t),screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(420.0,560.0))),events,..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{h.app.avoidance.rows(ui,&mut h.app.settings);});
        })
    }
    fn text(out:&egui::FullOutput,label:&str)->Rect {
        out.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect()),_=>None}).unwrap()
    }
    let footer="切到插件浮窗时，宿主窗口仍遮住岛就会继续避让。";
    h.app.settings.avoid_apps=vec![("D:/sample/A.exe".into(),"A应用".into())];
    draw(&mut h,vec![]);let one=draw(&mut h,vec![]);let baseline=text(&one,footer).top();
    let cross=text(&one,"×").center();
    assert!(one.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Circle(c) if c.radius<=7.1 && c.center.distance(cross)<3.0)),"移除角标应为小圆点，不能被按钮最小尺寸撑大");
    h.app.settings.avoid_apps=(0..30).map(|i|(format!("D:/sample/{i}.exe"),format!("{}应用",char::from_u32(65+i).unwrap()))).collect();
    draw(&mut h,vec![]);let many=draw(&mut h,vec![]);
    assert!((text(&many,footer).top()-baseline).abs()<1.0,"应用增多时始终保持单行高度");
    let first=text(&many,"A");let pointer=first.center();
    draw(&mut h,vec![egui::Event::PointerMoved(pointer),egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:egui::vec2(0.0,-150.0),modifiers:Default::default()}]);
    let mut scrolled=draw(&mut h,vec![]);for _ in 0..30{scrolled=draw(&mut h,vec![]);}
    assert!((text(&scrolled,footer).top()-text(&many,footer).top()).abs()<1.0,"滚动图标不能移动区域下方内容");
    assert!(scrolled.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="A" && t.visual_bounding_rect().left()<first.left()-10.0)) || !scrolled.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="A")),"应能滚动查看后续应用");
}

#[test]
fn expanded_status_row_keeps_entries_clickable_and_time_short(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.clock_12h=true;h.app.settings.clock_period=2;h.app.settings.clock_seconds=true;
    h.media=crate::music::NowPlaying{source:"cloudmusic.exe".into(),title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    h.app.windows_unread=true;
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    let rect=|h:&H,name|h.ctx.read_response(egui::Id::new(("card",true,name))).unwrap_or_else(||panic!("展开态缺少 {name}")).rect;
    for (name,r) in [("bell-mail",rect(&h,"bell-mail")),("header-music",rect(&h,"header-music")),("agent-pill",rect(&h,"agent-pill"))] {
        assert!(r.center().y<crate::island::STATUS_H,"{name} 应留在顶部状态行：{r:?}");
    }
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="09:41"&&t.visual_bounding_rect().center().y<crate::island::STATUS_H)),"展开态只显示时:分");
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().contains("上午")||t.galley.text()=="9:41:00")),"展开态不受时段和秒数设置影响");
    h.click(rect(&h,"header-music").center());
    assert_eq!(h.app.page(),0,"展开态音乐入口可点");
    h.hover(H::local(300.0,60.0),0.5);
    h.click(rect(&h,"bell-mail").center());
    assert_eq!(h.app.page(),crate::island::WINDOWS_PAGE,"展开态铃铛可点");
}

#[test]
#[ignore="按需导出左右停靠的实际画面"]
fn side_dock_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_FEATURE_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let (w,hgt)=(crate::island::side_dock::SIDE_W,crate::island::side_dock::SIDE_H);
    for (dock,shape,skin) in [(crate::model::Dock::Left,crate::model::IslandShape::Notch,crate::model::Skin::Classic),(crate::model::Dock::Right,crate::model::IslandShape::Pill,crate::model::Skin::Cream)] {
        let tag=format!("{}-{}",dock.key(),shape.key());
        let mut h=H::new();let mut r=feature_raster_sized(w,hgt);
        h.app.settings.dock=dock;h.app.settings.island_shape=shape;h.app.settings.skin=skin;h.app.settings.pet=false;
        h.app.media.title="晴天".into();h.app.media.artist="周杰伦".into();h.app.media.playing=true;h.app.media.source="cloudmusic.exe".into();
        let away=Some(Pos2::new(w/2.0,hgt-10.0));
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-collapsed"),60,away);
        let edge_x=if dock==crate::model::Dock::Left{8.0}else{w-8.0};
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-hover"),8,Some(Pos2::new(edge_x,hgt/2.0)));
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-opening"),12,Some(Pos2::new(edge_x,hgt/2.0)));
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-open"),70,Some(Pos2::new(edge_x,hgt/2.0)));
        h.app.show_page(4);
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-todo"),50,Some(Pos2::new(edge_x,hgt/2.0)));
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-closed"),320,away);
        h.app.alert("待办到时间了","回复设计评审意见",4,false);
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-notice"),40,away);
    }
    // 播放音乐 + 专注中 + 毛玻璃；极简模式；侧边提示。
    SIDE_MUSIC.store(true,std::sync::atomic::Ordering::Relaxed);
    let mut h=H::new();let mut r=feature_raster_sized(w,hgt);
    h.app.settings.dock=crate::model::Dock::Left;h.app.settings.skin=crate::model::Skin::GlassDark;
    h.app.pomo.running=true;h.app.pomo.remaining=1430.0;
    side_quota_fixture(&mut h);
    let away=Some(Pos2::new(w/2.0,hgt-10.0));
    shoot_side(&mut h,&mut r,&dest,"side-left-glass-busy",60,away);
    shoot_side(&mut h,&mut r,&dest,"side-left-glass-open",70,Some(Pos2::new(8.0,hgt/2.0)));
    shoot_side(&mut h,&mut r,&dest,"side-left-glass-closed",120,away);
    h.app.code_copied("1");
    shoot_side(&mut h,&mut r,&dest,"side-left-toast",20,away);
    SIDE_MUSIC.store(false,std::sync::atomic::Ordering::Relaxed);
    let mut h=H::new();let mut r=feature_raster_sized(w,hgt);
    h.app.settings.dock=crate::model::Dock::Right;h.app.set_minimal(true);
    shoot_side(&mut h,&mut r,&dest,"side-right-minimal",60,away);
}

#[test]
fn side_dock_auto_minimal_follows_avoidance(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let (w,hgt)=(crate::island::side_dock::SIDE_W,crate::island::side_dock::SIDE_H);
    let mut h=H::new();h.app.settings.dock=crate::model::Dock::Right;h.app.settings.browser_minimal=true;
    let away=Some(Pos2::new(w/2.0,hgt-5.0));
    for _ in 0..60 {side_frame(&mut h,away);}
    let normal=side_frame(&mut h,away).hit;
    h.browser_at_top=Some(true);
    let mut out=side_frame_with(&mut h,away);
    for _ in 0..60 {out=side_frame_with(&mut h,away);}
    assert!(out.hit.width()<normal.width() && out.hit.height()<normal.height(),"避让时收成贴边细线：{:?} → {:?}",normal,out.hit);
    h.browser_at_top=Some(false);
    for _ in 0..60 {out=side_frame_with(&mut h,away);}
    assert!((out.hit.height()-normal.height()).abs()<1.0,"离开后恢复：{:?}",out.hit);
}

#[test]
fn side_dock_opens_from_the_edge_and_collapses_back(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let (w,hgt)=(crate::island::side_dock::SIDE_W,crate::island::side_dock::SIDE_H);
    for (dock,shape) in [(crate::model::Dock::Left,crate::model::IslandShape::Notch),(crate::model::Dock::Right,crate::model::IslandShape::Pill)] {
        let mut h=H::new();h.app.settings.dock=dock;h.app.settings.island_shape=shape;
        let away=Some(Pos2::new(w/2.0,hgt-5.0));
        let mut out=side_frame(&mut h,away);
        for _ in 0..60 {out=side_frame(&mut h,away);}
        assert!(!h.app.open);
        // 收起：竖向胶囊贴边、竖直居中。
        assert!(out.hit.height()>out.hit.width(),"{dock:?} 收起时是竖向：{:?}",out.hit);
        assert!((out.hit.center().y-hgt/2.0).abs()<1.0);
        let edge=if dock==crate::model::Dock::Left{0.5}else{w-0.5};
        assert!(out.hit.contains(Pos2::new(edge,hgt/2.0)),"{dock:?} 光标推到屏幕边缘也能命中：{:?}",out.hit);
        assert!(out.region.contains_rect(out.hit.shrink(0.5)));
        // 推到边缘停留 → 展开，面板从边缘滑出且完整留在画布内。
        for _ in 0..60 {out=side_frame(&mut h,Some(Pos2::new(edge,hgt/2.0)));}
        assert!(h.app.open && h.app.progress()>0.95,"{dock:?} 应展开");
        assert!(out.hit.width()>crate::island::PW-1.0 && out.hit.height()<200.0,"{:?}",out.hit);
        assert!(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt)).contains_rect(out.region),"区域不能超出画布：{:?}",out.region);
        if dock==crate::model::Dock::Left {assert!(out.hit.left()<=0.5)} else {assert!(out.hit.right()>=w-0.5)}
        // 离开 → 收起。
        for _ in 0..90 {out=side_frame(&mut h,away);}
        assert!(!h.app.open && h.app.progress()<0.05);
        assert!(out.hit.width()<60.0);
    }
}

/// 内存实测：模拟长时间听歌（不断出现新汉字）并浏览各页面，输出进程内存、字形图集和贴图占用。
/// 运行：DAOZAI_ATLAS=4096 DAOZAI_PPP=1.5 cargo test --lib memory_profile -- --ignored --nocapture --test-threads=1
#[test]
#[ignore="内存实测，按需运行"]
fn memory_profile_long_listening_session(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let side:usize=std::env::var("DAOZAI_ATLAS").ok().and_then(|s|s.parse().ok()).unwrap_or(4096);
    let ppp:f32=std::env::var("DAOZAI_PPP").ok().and_then(|s|s.parse().ok()).unwrap_or(1.5);
    let songs:usize=std::env::var("DAOZAI_SONGS").ok().and_then(|s|s.parse().ok()).unwrap_or(120);
    // 匿名内存≈Windows 的私有内存；文件映射（程序代码、系统字体）可在进程间共享，不计入私有内存。
    fn rss()->String{
        let status=std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        let kb=|key:&str|status.lines().find_map(|l|l.strip_prefix(key)?.trim().trim_end_matches(" kB").trim().parse::<f64>().ok()).unwrap_or(0.0)/1024.0;
        format!("匿名={:>6.1}MB 文件映射={:>6.1}MB",kb("RssAnon:"),kb("RssFile:"))
    }
    fn report(label:&str,h:&H){
        let [w,hgt]=h.ctx.fonts(|f|f.font_image_size());
        let tex:usize=h.ctx.tex_manager().read().allocated().map(|(_,m)|m.bytes_used()).sum();
        let galleys=h.ctx.fonts(|f|f.num_galleys_in_cache());
        // 图集在 CPU 端按 f32 存覆盖率，显卡端再按 RGBA 存一份。
        println!("{label:<16} {} 图集={w}x{hgt}（CPU≈{:.1}MB）贴图合计={:.1}MB 文字缓存={galleys}",rss(),(w*hgt*4) as f64/1048576.0,tex as f64/1048576.0);
    }
    println!("{:<16} {}","测试进程基线",rss());
    let mut h=H::new();h.app.settings.island_lyrics=true;h.app.show_lyrics=true;
    println!("{:<16} {}","创建界面后",rss());
    let mut seed=12345u64;let mut next=||{seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);(seed>>33) as u32};
    let run=|h:&mut H,media:&crate::music::NowPlaying|{
        h.t+=1.0/60.0;
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W,WIN_H))),time:Some(h.t),max_texture_side:Some(side),
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
        let env=Env{t:h.t,clock:"09:41:00".into(),today:"2026-10-28".into(),date:"10月28日 周三".into(),media:media.clone(),cursor:h.cursor,bt_ready:true,..Default::default()};
        let app=&mut h.app;
        let out=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let _=app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H)),&env);});});
        h.shapes=out.shapes;
    };
    let idle=crate::music::NowPlaying::default();
    for _ in 0..30 {run(&mut h,&idle);}
    report("启动后",&h);
    for page in 0..8 {h.app.show_page(page);for _ in 0..20 {run(&mut h,&idle);}}
    report("浏览8个页面后",&h);
    for song in 0..songs {
        // 常用字约 3000 个：每首歌 40 句、每句 8–12 字，从中随机取，接近真实歌词的用字分布。
        let lines:Vec<String>=(0..40).map(|_|(0..8+next()%5).map(|_|char::from_u32(0x4E00+next()%3000).unwrap()).collect()).collect();
        let lrc=lines.iter().enumerate().map(|(i,l)|format!("[{:02}:{:02}.00]{l}\n",i*3/60,i*3%60)).collect::<String>();
        let title=format!("测试歌曲{song}");
        h.app.set_lyrics(&title,"歌手",200.0,crate::music::lyrics::parse_lrc(&lrc));
        if song==songs/2 {h.app.show_page(0);}
        for i in 0..40 {
            let media=crate::music::NowPlaying{title:title.clone(),artist:"歌手".into(),duration:200.0,position:i as f64*3.0+0.5,playing:false,..Default::default()};
            for _ in 0..2 {run(&mut h,&media);}
        }
        if (song+1)%20==0 {report(&format!("听完{}首",song+1),&h);}
    }
}

#[test]
fn stale_exit_results_preserve_new_windows_and_unidentified_accounts(){
    use crate::island::{Messenger,Unread};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.wechat_notifications=true;
    let old=Unread{source:Messenger::Wechat,pid:100,window:11};
    h.app.receive_wechat_reminder_from(old.pid,old.window);h.app.receive_wechat_reminder_from(100,12);
    h.app.receive_wechat_reminder();h.app.discard_unread(old);
    assert_eq!(h.app.pending_messengers,[Unread{window:12,..old},Unread::of(Messenger::Wechat)]);
    h.app.discard_unread(Unread{window:12,..old});h.frame(vec![]);
    assert_eq!(h.app.pending_messengers,[Unread::of(Messenger::Wechat)]);
    h.app.discard_unread(Unread::of(Messenger::Wechat));h.frame(vec![]);
    assert!(h.app.pending_messengers.is_empty());
}
