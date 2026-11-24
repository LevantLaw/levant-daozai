//! 无窗口交互测试：直接向 egui 注入输入事件，验证悬停展开、切页、按钮。

use crate::island::{App, Env, Options, PH, PW, WIN_H, WIN_W};

mod enterprise_messenger;

mod calendar_compatibility;

mod doubao;

mod weather_source;

mod lyrics_switch;

mod health_reminder;

mod settings_redesign;

mod page_badges;

mod quota;
mod calendar;
mod mail;
mod weather;
mod music;
mod notifications;
mod settings;
mod agent;
mod todo_shelf;
mod focus_health;
mod pages;
mod island;

use eframe::egui::{self, Pos2, Rect, Vec2};

use std::sync::Mutex;

pub(crate) static ENV_LOCK: Mutex<()> = Mutex::new(());

fn editing_local_calendar_clears_outdated_reminders() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for reschedule in [false,true] {
        let mut h=H::new();
        let mut draft=crate::calendar::local::Draft::new(chrono::Local::now().date_naive()+chrono::Duration::days(1));
        draft.title="原日程".into();
        let entry=draft.entry().unwrap();
        h.app.calendar.local=vec![entry.clone()];
        h.app.alert("专注计时","休息",1,false);
        h.frame(vec![]);
        h.app.meeting_alert(entry.event.clone(),"本机日程".into(),7);
        let mut other=entry.event.clone();other.uid="另一日程".into();
        h.app.meeting_alert(other,"本机日程".into(),7);
        h.app.open_calendar_editor(Some(&entry.event.uid));
        let draft=h.app.calendar.draft.as_mut().unwrap();
        if reschedule {
            let day=(chrono::Local::now().date_naive()+chrono::Duration::days(2)).to_string();
            draft.start_date=day.clone();draft.end_date=day;
        } else {draft.reminder=None;}
        h.app.save_calendar_editor();
        assert!(h.app.calendar.draft.is_none());
        assert!(!h.app.notice.as_ref().is_some_and(|a|a.meeting.as_ref().is_some_and(|(m,_)|m.uid==entry.event.uid)),"修改后不能继续显示旧提醒");
        assert!(!h.app.alerts.iter().any(|a|a.meeting.as_ref().is_some_and(|(m,_)|m.uid==entry.event.uid)),"修改后不能弹出排队中的旧提醒");
        assert!(h.app.alerts.iter().any(|a|a.meeting.as_ref().is_some_and(|(m,_)|m.uid=="另一日程")));
    }
}

struct H {
    ctx: egui::Context,
    app: App,
    t: f64,
    cursor: Option<Pos2>,
    cmds:Vec<crate::island::Cmd>,
    region:Rect,
    hit:Rect,
    animating:bool,
    page_transition:bool,
    shapes:Vec<egui::epaint::ClippedShape>,
    bt: Vec<crate::os::bt::Device>,
    media: crate::smtc::NowPlaying,
    power: (Option<u8>, bool),
    network: Option<bool>,
    system_idle: Option<f64>,
    screen_locked: bool,
    fullscreen: bool,
    browser_at_top: Option<bool>,
    file_drop_zone:Option<Rect>,
    hovered_files:Vec<egui::HoveredFile>,
    dropped_files:Vec<egui::DroppedFile>,
}

impl H {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("daozai-test-{}-{}", std::process::id(), rand_id()));
        std::env::set_var("DAOZAI_DATA_DIR", &dir);
        let ctx = egui::Context::default();
        let app = App::new(&ctx, Options { fetch_lyrics: false, pet_idle: 1e9 });
        Self { ctx, app, t: 0.0, cursor: None,cmds:vec![],region:Rect::NOTHING,hit:Rect::NOTHING,animating:false,page_transition:false,shapes:Vec::new(), bt:Vec::new(),media:Default::default(),power:(None,false),network:None,system_idle:None,screen_locked:false,fullscreen:false,browser_at_top:None,file_drop_zone:None,hovered_files:vec![],dropped_files:vec![] }
    }
    fn frame(&mut self, events: Vec<egui::Event>) {
        self.t += 1.0 / 60.0;
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(crate::island::CANVAS_W, WIN_H))),
            time: Some(self.t),
            events,
            hovered_files:self.hovered_files.clone(),
            dropped_files:std::mem::take(&mut self.dropped_files),
            focused: true,
            ..Default::default()
        };
        self.app.filter_todo_input(&mut raw);
        let env = Env {
            t: self.t,
            clock: "09:41:00".into(),
            today: "2026-10-28".into(),
            date: "10月28日 周三".into(),
            cursor: self.cursor,
            bt: self.bt.clone(),
            bt_ready: true,
            media:self.media.clone(),
            battery:self.power.0,
            charging:self.power.1,
            network:self.network,
            idle_seconds:self.system_idle,
            screen_locked:self.screen_locked,
            fullscreen_hidden:self.fullscreen && self.app.settings.hide_fullscreen,
            browser_at_top:self.browser_at_top,
            ..Default::default()
        };
        let cmds=&mut self.cmds;
        let region=&mut self.region;
        let hit=&mut self.hit;
        let animating=&mut self.animating;
        let page_transition=&mut self.page_transition;
        let app = &mut self.app;
        let file_drop_zone=&mut self.file_drop_zone;
        let output = self.ctx.run(raw, |ctx| {
            egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx, |ui| {
                let win = Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));
                let frame=app.frame(ui, win, &env);*region=frame.region;*hit=frame.hit;*animating=frame.animating;*page_transition=frame.page_transition;*file_drop_zone=frame.file_drop_zone;cmds.extend(frame.cmds);
            });
        });
        self.shapes=output.shapes;
    }
    fn hover(&mut self, p: Pos2, secs: f64) {
        self.cursor = Some(p);
        let n = (secs * 60.0) as usize;
        for _ in 0..n {
            self.frame(vec![egui::Event::PointerMoved(p)]);
        }
    }
    fn click(&mut self, p: Pos2) {
        self.cursor = Some(p);
        self.frame(vec![egui::Event::PointerMoved(p)]);
        self.frame(vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }]);
        self.frame(vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }]);
    }
    fn click_text(&mut self,label:&str){
        if !self.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==label))&&self.app.notice.is_some(){self.click_notice_badge();}
        let point=self.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label&&t.opacity_factor>0.9&&s.clip_rect.contains(t.visual_bounding_rect().center())=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap_or_else(||panic!("未找到可点击文字：{label}"));
        self.click(point);
    }
    fn click_notice_badge(&mut self){if self.app.compact_notices(){self.click(Pos2::new(WIN_W/2.0+(self.app.current_width()-44.0)/2.0+26.0,15.0));}}
    fn wheel(&mut self,delta:f32) {
        self.frame(vec![egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:Vec2::new(0.0,delta),modifiers:Default::default()}]);
    }
    fn picker_frame(&mut self,events:Vec<egui::Event>){
        self.frame(events);
    }
    fn picker_click(&mut self,label:&str){
        for _ in 0..60 {self.picker_frame(vec![]);if (self.app.progress()-1.0).abs()<0.01 {break;}}
        self.picker_frame(vec![]);
        let find=|h:&H|h.shapes.iter().find_map(|shape|match &shape.shape{
            egui::Shape::Text(text) if text.galley.text()==label && shape.clip_rect.contains(text.visual_bounding_rect().center())=>Some(text.visual_bounding_rect().center()),_=>None,
        });
        let point=find(self).unwrap_or_else(||panic!("未找到可见选项：{label}"));
        assert!(self.hit.contains(point),"选项 {label} 超出岛体命中区域");
        self.picker_frame(vec![egui::Event::PointerMoved(point)]);
        for pressed in [true,false]{self.picker_frame(vec![egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}]);}
    }
    /// 页面内容的本地坐标：x 相对面板左边，y 相对状态行下方的页面内容顶部。
    fn local(x: f32, y: f32) -> Pos2 {
        Pos2::new((WIN_W - PW) / 2.0 + x, crate::island::STATUS_H - crate::island::PAGE_RAISE + y)
    }
    fn quota_accounts(&mut self,count:usize) {
        use crate::quota::{Account,Provider,Snapshot,Window};
        for i in 0..count {
            let id=format!("preview-{i}");
            self.app.quota.config.accounts.push(Account{id:id.clone(),provider:Provider::ALL[i%3],alias:format!("账号 {}",i+1),source:format!("preview-{i}.json").into(),slot:None,enabled:true,pinned:true});
            self.app.quota_event(crate::quota::Event{id,result:Ok(Snapshot{windows:vec![Window{label:"本周".into(),seconds:Some(604800),remaining:100.0,reset:None,amount:None}],..Default::default()})});
        }
    }
    fn quota_centers(&self)->Vec<f32> {
        fn collect(shape:&egui::Shape,out:&mut Vec<f32>) {
            match shape {
                egui::Shape::Vec(shapes)=>for shape in shapes {collect(shape,out);},
                egui::Shape::Path(path) if path.points.len()>50=>{
                    let min=path.points.iter().map(|p|p.x).fold(f32::INFINITY,f32::min);
                    let max=path.points.iter().map(|p|p.x).fold(f32::NEG_INFINITY,f32::max);
                    if (20.0..28.0).contains(&(max-min)) {out.push((min+max)/2.0);}
                },
                egui::Shape::Circle(circle) if (10.0..14.0).contains(&circle.radius) && circle.stroke.width>0.0 && (circle.center.y-15.0).abs()<0.5=>out.push(circle.center.x),
                _=>{},
            }
        }
        let mut out=Vec::new();for shape in &self.shapes {collect(&shape.shape,&mut out);}
        out.sort_by(f32::total_cmp);out
    }
}

fn rand_id() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64
}

fn island_drag_frame(h:&mut H,bounds:crate::os::display_layout::Bounds,ppp:f32,pointer:Option<Pos2>,pressed:Option<bool>)->Rect {
    use crate::{os::display_layout,island::side_dock};
    let dock=h.app.settings.dock;
    let (width,height)=if dock.is_side(){(side_dock::SIDE_W,side_dock::SIDE_H)}else{(WIN_W,WIN_H)};
    let offset=h.app.placement_offset();
    let (x,y)=if dock.is_side(){display_layout::side_origin(bounds,dock,(width,height),side_dock::SIDE_HALF_EXTENT,ppp,offset)}
        else{display_layout::island_origin_offset(bounds,width,ppp,offset)}.unwrap();
    let screen=Rect::from_min_max(Pos2::new((bounds.left-x) as f32/ppp,(bounds.top-y) as f32/ppp),Pos2::new((bounds.right-x) as f32/ppp,(bounds.bottom-y) as f32/ppp));
    let cursor=pointer.map(|p|p+screen.min.to_vec2());
    let mut events=cursor.map(|p|vec![egui::Event::PointerMoved(p)]).unwrap_or_default();
    if let (Some(pos),Some(pressed))=(cursor,pressed){events.push(egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()});}
    h.t+=1.0/60.0;
    let env=Env{t:h.t,clock:"09:41:00".into(),today:"2026-10-28".into(),date:"10月28日 周三".into(),cursor,screen:Some(screen),bt_ready:true,..Default::default()};
    let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::island::CANVAS_W.max(width),height))),time:Some(h.t),events,focused:true,
        viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
    let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{
        let out=h.app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(width,height)),&env);h.region=out.region;h.hit=out.hit;h.cmds.extend(out.cmds);
    });});
    h.shapes=output.shapes;
    screen
}

fn island_drag_header(h:&mut H,bounds:crate::os::display_layout::Bounds,ppp:f32)->Pos2 {
    h.app.show_page(1);
    let mut screen=Rect::NOTHING;
    for _ in 0..90{screen=island_drag_frame(h,bounds,ppp,None,None);}
    let (width,height)=if h.app.side(){(crate::island::side_dock::SIDE_W,crate::island::side_dock::SIDE_H)}else{(WIN_W,WIN_H)};
    let win=Rect::from_min_size(Pos2::ZERO,Vec2::new(width,height));
    h.app.content_origin(win)+Vec2::new(210.0,15.0)-screen.min.to_vec2()
}

/// 展开态状态行最右侧的设置按钮；位置随组件宽度变化，按响应取中心。
fn settings_button(h:&H)->Pos2 {
    h.ctx.read_response(egui::Id::new(("card",true,"page-settings"))).expect("展开后应有设置按钮").rect.center()
}

fn kiro_snapshot(remaining:f64,limit:f64)->crate::quota::Snapshot{
    crate::quota::parse(crate::quota::Provider::Kiro,&serde_json::json!({"userInfo":{"userId":"ui-kiro-user"},"subscriptionInfo":{"subscriptionTitle":"KIRO PRO"},
        "usageBreakdownList":[{"resourceType":"CREDIT","usageLimitWithPrecision":limit,"currentUsageWithPrecision":limit-remaining,"nextDateReset":4102444800_i64}]})).unwrap()
}

/// 去掉 egui 在浮层淡入首帧留下的不可见占位，只比较真正绘制的内容。
fn visible_shapes(shapes:&[egui::epaint::ClippedShape])->Vec<egui::epaint::ClippedShape>{
    shapes.iter().filter(|s|!matches!(s.shape,egui::Shape::Noop)).cloned().collect()
}

fn post_agent_hook(port:u16,token:&str,session:&str){
    use std::io::{Read,Write};
    let body=serde_json::json!({
        "agent":"claude","event":"PreToolUse","blocking":false,
        "payload":{"session_id":session,"cwd":"E:\\demo\\project","tool_name":"AskUserQuestion",
            "tool_input":{"questions":[{"question":"选哪个？","header":"选择","multiSelect":false,
                "options":[{"label":"甲","description":"第一个"},{"label":"乙","description":"第二个"}]}]}}
    }).to_string();
    let mut stream=std::net::TcpStream::connect(("127.0.0.1",port)).unwrap();
    write!(stream,"POST /hook HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Daozai-Token: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
    let mut response=Vec::new();
    let _=stream.read_to_end(&mut response);
}

fn post_agent_permission_hook(port:u16,token:&str,session:&str){
    use std::io::{Read,Write};
    let body=serde_json::json!({
        "agent":"claude","event":"PermissionRequest","blocking":false,
        "payload":{"session_id":session,"cwd":"E:\\demo\\project","tool_name":"Bash",
            "tool_input":{"command":"dir"}}
    }).to_string();
    let mut stream=std::net::TcpStream::connect(("127.0.0.1",port)).unwrap();
    write!(stream,"POST /hook HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Daozai-Token: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
    let mut response=Vec::new();
    let _=stream.read_to_end(&mut response);
}

fn post_agent_event(port:u16,token:&str,event:&str,payload:serde_json::Value){
    use std::io::{Read,Write};
    let body=serde_json::json!({"agent":"claude","event":event,"blocking":false,"payload":payload}).to_string();
    let mut stream=std::net::TcpStream::connect(("127.0.0.1",port)).unwrap();
    write!(stream,"POST /hook HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Daozai-Token: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
    let mut response=Vec::new();
    let _=stream.read_to_end(&mut response);
}

/// 新功能画面：按需导出，检查排版。
fn feature_raster()->crate::preview_raster::Raster{crate::preview_raster::Raster::new((crate::island::CANVAS_W*2.0) as usize,(WIN_H*2.0) as usize)}

fn shoot_feature(h:&mut H,raster:&mut crate::preview_raster::Raster,dest:&std::path::Path,name:&str,frames:usize,cursor:Option<Pos2>,today:&str){
    let ppp=2.0;let (w,hgt)=(crate::island::CANVAS_W,WIN_H);
    for f in 0..frames {
        h.t+=1.0/60.0;
        let env=Env{t:h.t,clock:"09:41:00".into(),today:today.into(),date:"11月4日 周三".into(),cursor,bt_ready:true,..Default::default()};
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,
            events:cursor.map(|p|vec![egui::Event::PointerMoved(p)]).unwrap_or_default(),
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
        let app=&mut h.app;
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));app.frame(ui,win,&env);});});
        raster.update_textures(&output.textures_delta);
        let primitives=h.ctx.tessellate(output.shapes,output.pixels_per_point);
        if f+1==frames {
            raster.buf.fill([0.20,0.24,0.32,1.0]);
            raster.draw(&primitives,output.pixels_per_point);
            let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            let image=image::RgbaImage::from_raw((w*ppp) as u32,(hgt*ppp) as u32,rgba).unwrap();
            let crop_h=(h.app.expanded_height()*ppp+24.0).max(330.0).min(hgt*ppp) as u32;
            image::imageops::crop_imm(&image,200,0,880,crop_h).to_image().save(dest.join(format!("{name}.png"))).unwrap();
        }
        raster.free_textures(&output.textures_delta);
    }
}

static SIDE_MUSIC:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);

/// 侧边停靠的画面：画布为 SIDE_W × SIDE_H。
fn shoot_side(h:&mut H,raster:&mut crate::preview_raster::Raster,dest:&std::path::Path,name:&str,frames:usize,cursor:Option<Pos2>){
    let ppp=2.0;let (w,hgt)=(crate::island::side_dock::SIDE_W,crate::island::side_dock::SIDE_H);
    for f in 0..frames {
        h.t+=1.0/60.0;
        let mut env=Env{t:h.t,clock:"09:41:00".into(),today:"2026-11-04".into(),date:"11月4日 周三".into(),cursor,bt_ready:true,battery:Some(76),..Default::default()};
        if SIDE_MUSIC.load(std::sync::atomic::Ordering::Relaxed) {env.media.title="晴天".into();env.media.artist="周杰伦".into();env.media.playing=true;env.media.source="cloudmusic.exe".into();env.media.duration=269.0;}
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,
            events:cursor.map(|p|vec![egui::Event::PointerMoved(p)]).unwrap_or_default(),
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
        let app=&mut h.app;let mut out=None;
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(w,hgt));out=Some(app.frame(ui,win,&env));});});
        raster.update_textures(&output.textures_delta);
        if f+1==frames {
            raster.buf.fill([0.20,0.24,0.32,1.0]);
            raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
            let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            let mut image=image::RgbaImage::from_raw((w*ppp) as u32,(hgt*ppp) as u32,rgba).unwrap();
            // 画出窗口区域（红）与命中区域（绿）的边框，方便检查。
            if let Some(out)=out {
                for (r,color) in [(out.region,image::Rgba([255,80,80,255])),(out.hit,image::Rgba([80,255,120,255]))] {
                    let (x0,y0,x1,y1)=((r.left()*ppp).max(0.0) as u32,(r.top()*ppp).max(0.0) as u32,((r.right()*ppp) as u32).min(image.width()-1),((r.bottom()*ppp) as u32).min(image.height()-1));
                    for x in x0..=x1 {image.put_pixel(x,y0,color);image.put_pixel(x,y1,color);}
                    for y in y0..=y1 {image.put_pixel(x0,y,color);image.put_pixel(x1,y,color);}
                }
            }
            image.save(dest.join(format!("{name}.png"))).unwrap();
        }
        raster.free_textures(&output.textures_delta);
    }
}

fn feature_raster_sized(w:f32,h:f32)->crate::preview_raster::Raster{crate::preview_raster::Raster::new((w*2.0) as usize,(h*2.0) as usize)}

/// 三个环形额度账号，随后显示 WorkBuddy 与 DeepSeek 余额。
fn side_quota_fixture(h:&mut H)->(String,String){
    use crate::quota::{Provider,Event};
    h.quota_accounts(3);
    let wb=h.app.quota.config.connect_workbuddy("fixture".into(),"WorkBuddy".into());
    let ds=h.app.quota.config.connect_deepseek("fixture".into(),"DeepSeek".into());
    h.app.quota_event(Event{id:wb.clone(),result:crate::quota::parse(Provider::WorkBuddy,&serde_json::json!({"code":0,"data":{"Packages":[{"CycleRemainCapacity":"28560"}]}}))});
    h.app.quota_event(Event{id:ds.clone(),result:crate::quota::parse(Provider::DeepSeek,&serde_json::json!({"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"4.18","granted_balance":"0","topped_up_balance":"4.18"}]}))});
    (wb,ds)
}

fn side_frame_with(h:&mut H,cursor:Option<Pos2>)->crate::island::FrameOut{
    let at_top=h.browser_at_top;
    let (w,hgt)=(crate::island::side_dock::SIDE_W,crate::island::side_dock::SIDE_H);
    h.t+=1.0/60.0;
    let env=Env{t:h.t,clock:"09:41:00".into(),cursor,battery:Some(76),browser_at_top:at_top,..Default::default()};
    let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,..Default::default()};
    let app=&mut h.app;let mut out=None;
    let _=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{out=Some(app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(w,hgt)),&env));});});
    out.unwrap()
}

fn side_frame(h:&mut H,cursor:Option<Pos2>)->crate::island::FrameOut{
    side_frame_events(h,cursor,cursor.map(|p|vec![egui::Event::PointerMoved(p)]).unwrap_or_default())
}

fn side_frame_events(h:&mut H,cursor:Option<Pos2>,events:Vec<egui::Event>)->crate::island::FrameOut{
    let (w,hgt)=(crate::island::side_dock::SIDE_W,crate::island::side_dock::SIDE_H);
    h.t+=1.0/60.0;
    let env=Env{t:h.t,clock:"09:41:00".into(),today:"2026-11-04".into(),date:"11月4日 周三".into(),cursor,bt_ready:true,battery:Some(76),..Default::default()};
    let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,
        events,..Default::default()};
    let app=&mut h.app;let mut out=None;
    let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{out=Some(app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(w,hgt)),&env));});});
    h.shapes=output.shapes;
    out.unwrap()
}

fn shoot_unread(h:&mut H,raster:&mut crate::preview_raster::Raster,dest:&std::path::Path,name:&str,frames:usize,cursor:Option<Pos2>){
    let ppp=2.0;
    let side=h.app.settings.dock.is_side();
    let (w,hgt)=if side {(crate::island::side_dock::SIDE_W,crate::island::side_dock::SIDE_H)} else {(crate::island::CANVAS_W,WIN_H)};
    let (ww,wh)=if side {(w,hgt)} else {(WIN_W,WIN_H)};
    for f in 0..frames {
        h.t+=1.0/60.0;
        let env=Env{t:h.t,clock:"09:41:00".into(),today:"2026-11-05".into(),date:"11月5日 周四".into(),cursor,bt_ready:true,battery:Some(76),..Default::default()};
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,
            events:cursor.map(|p|vec![egui::Event::PointerMoved(p)]).unwrap_or_default(),
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
        let app=&mut h.app;
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(ww,wh));app.frame(ui,win,&env);});});
        raster.update_textures(&output.textures_delta);
        if f+1==frames {
            raster.buf.fill([0.20,0.24,0.32,1.0]);
            raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
            let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            let image=image::RgbaImage::from_raw((w*ppp) as u32,(hgt*ppp) as u32,rgba).unwrap();
            let image=if side {image::imageops::crop_imm(&image,0,240,(w*ppp) as u32,720).to_image()} else {image::imageops::crop_imm(&image,0,0,(WIN_W*ppp) as u32,(WIN_H*ppp) as u32).to_image()};
            image.save(dest.join(format!("{name}.png"))).unwrap();
        }
        raster.free_textures(&output.textures_delta);
    }
}

fn managed_mail_fixture()->H{
    use crate::mail::manage::{Folder,Snapshot};
    let mut h=H::new();h.app.settings.mail_sync_operations=true;let account=crate::mail::Account{custom:None,email:"manage@example.com".into(),provider:crate::mail::Provider::Qq,enabled:true};
    h.app.mail.accounts=vec![account.clone()];
    h.app.mail.recent=(1..=3).map(|uid|crate::mail::Item{html:String::new().into(),received_unix:None,account:account.email.clone(),from:format!("发件人 {uid}"),subject:format!("测试邮件 {uid}"),time:format!("09:4{uid}"),code:None,summary:"完整的测试邮件正文。".repeat(10).into(),links:vec![],meeting:None,uid}).collect();
    h.app.mail.manage.identities.insert(account.email.clone(),account.clone());
    h.app.mail.manage.data.insert((account.email,Folder::Inbox),Snapshot{validity:7,entries:vec![(1,false),(2,true),(3,true)],items:vec![]});
    h.app.show_page(5);h.hover(H::local(220.0,80.0),1.0);h
}

fn raised(h:&H,title:&str)->bool{
    h.app.alerts.iter().chain(h.app.notice.as_ref()).any(|alert|alert.title==title)
}

/// 设置页的 AI 额度账号默认收起成一行；需要检查明细的测试先把所有账号展开。
fn open_quota_details(h:&H){
    for account in &h.app.quota.config.accounts {h.ctx.data_mut(|d|d.insert_temp(egui::Id::new(("quota-account-open",account.id.clone())),true));}
}
