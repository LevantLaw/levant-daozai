//! 横幅、HUD 与通知队列：提醒入队、横幅绘制、音量条与未读管理。
use crate::island::extras::HealthReminder;
use crate::island::settings::page_icon;
use crate::island::{App, Cmd, NotificationSound, HUD_TRACK_W, HUD_TRACK_X, HUD_W, PAGE_NAMES};
use crate::visuals::paint::{Card, H};
use crate::visuals::theme::{self, size, W};
use eframe::egui::{self, Color32, Rect};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AlertSource { Internal, Messenger(crate::island::Messenger), External(String) }

#[derive(Clone, Debug)]
pub(crate) struct Alert {
    pub title: String,
    pub source: AlertSource,
    pub body: String,
    pub page: usize,
    pub at: f64,
    pub duration: f64,
    pub bluetooth: Option<(crate::os::bt::DeviceKind, bool)>,
    pub meeting: Option<(crate::mail::Meeting, String)>,
    pub unread: Option<crate::island::Unread>,
    /// 最早可显示横幅的时刻；展开期间到达的通知等岛体收起后再显示。
    pub banner_at: f64,
}
impl Alert {
    /// 只有非即时通讯的 Windows 通知点亮铃铛；邮件、即时通讯和 AI 各有自己的入口。
    pub(crate) fn lights_bell(&self)->bool {
        matches!(&self.source,AlertSource::External(id) if crate::island::Messenger::from_notification_id(id).is_none())
    }
    pub(crate) fn health(&self)->Option<HealthReminder> {
        if self.source!=AlertSource::Internal{return None;}
        HealthReminder::ALL.into_iter().find(|kind|self.title==kind.title())
    }
    /// 固定铃声映射：完成类、提醒类、邮件，其余走默认。
    pub(crate) fn notification_sound(&self)->NotificationSound {
        match self.title.as_str() {
            "专注计时"|"待办到时间了"|"今日饮水目标达成"=>NotificationSound::Complete,
            "喝水时间到了"|crate::island::SIT_TITLE|"会议提醒"|"电量偏低"=>NotificationSound::Reminder,
            "新邮件"|"收到新邮件"|"邮件验证码"=>NotificationSound::Email,
            _=>NotificationSound::Sonar,
        }
    }
    pub(crate) fn messenger(&self)->Option<crate::island::Messenger>{
        match &self.source {
            AlertSource::Messenger(client)=>Some(*client),
            AlertSource::External(id)=>crate::island::Messenger::from_notification_id(id).filter(|client|client.adapter().capture==crate::messenger::Capture::SystemToast),
            _=>None,
        }
    }
    pub(crate) fn is_hud(&self)->bool {self.source==AlertSource::Internal && matches!(self.title.as_str(),"系统音量"|"大写锁定"|"键盘输入语言"|"网络连接"|"电源已连接"|"已切换电池供电"|"蓝牙设备")}
}
fn bluetooth_notice_content(body:&str)->(&str,&str,Option<u8>){
    if let Some(name)=body.strip_suffix(" · 已断开") {return (name,"已断开",None);}
    if body=="已断开连接" {return ("蓝牙设备","已断开",None);}
    if let Some((name,level))=body.rsplit_once(" · 电量 ") {
        if let Some(level)=level.strip_suffix('%').and_then(|s|s.parse::<u8>().ok()).filter(|v|*v<=100){
            return (name,"已连接",Some(level));
        }
    }
    (body,"已连接",None)
}

pub(crate) fn device_icon(c: &Card, kind: crate::os::bt::DeviceKind, x: f32, y: f32, size: f32) {
    let icon=match kind {
        crate::os::bt::DeviceKind::Bone | crate::os::bt::DeviceKind::Earbuds | crate::os::bt::DeviceKind::Headphones=>egui::include_image!("../../assets/device-icons/headphones-round-bold.svg"),
        crate::os::bt::DeviceKind::Speaker=>egui::include_image!("../../assets/device-icons/speaker-bold.svg"),
        crate::os::bt::DeviceKind::Keyboard=>egui::include_image!("../../assets/device-icons/keyboard-bold.svg"),
        crate::os::bt::DeviceKind::Mouse=>egui::include_image!("../../assets/device-icons/mouse-bold.svg"),
        crate::os::bt::DeviceKind::Gamepad=>egui::include_image!("../../assets/device-icons/gamepad-bold.svg"),
        crate::os::bt::DeviceKind::Phone=>egui::include_image!("../../assets/device-icons/smartphone-bold.svg"),
        crate::os::bt::DeviceKind::Other=>egui::include_image!("../../assets/device-icons/bluetooth-linear.svg"),
    };
    let rect=c.rect(x,y,size,size);
    let egui::ImageSource::Bytes{uri,bytes}=icon else{return};
    let pixels=(rect.width()*c.ui.ctx().pixels_per_point()).ceil() as u32;
    let icon=egui::Image::from_bytes(format!("{uri}/{pixels}.svg"),bytes);
    if let Ok(egui::load::TexturePoll::Ready { texture })=icon.fit_to_exact_size(rect.size()).load_for_size(c.ui.ctx(),rect.size()) {
        c.painter.image(texture.id,rect,Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),Color32::WHITE);
    }
}

pub(crate) fn notice_badge(c:&Card,x:f32,y:f32,a:&Alert){
    if a.source==AlertSource::Internal && a.title=="蓝牙设备"{
        c.circle(x,y,11.0,if a.body.contains("已断开"){Color32::from_gray(95)}else{theme::BLUE});
        device_icon(c,a.bluetooth.map_or(crate::os::bt::DeviceKind::Other,|v|v.0),x-7.5,y-7.5,15.0);
    }else{notice_icon(&c.scaled(x,y,0.85),x,y,a);}
}
pub(crate) fn messenger_icon(c:&Card,x:f32,y:f32,source:crate::island::Messenger){(source.adapter().paint_icon)(c,x,y);}

/// 应用图标：有纹理就画纹理，否则用名称首字占位，不用 Windows 铃铛。
pub(crate) fn app_icon_badge(c:&Card,x:f32,y:f32,size:f32,name:&str,texture:Option<&egui::TextureHandle>){
    if let Some(texture)=texture {
        let rect=c.rect(x-size/2.0,y-size/2.0,size,size);
        c.painter.image(texture.id(),rect,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),Color32::WHITE);
        return;
    }
    c.fill(x-size/2.0,y-size/2.0,size,size,size*0.25,Color32::from_gray(58));
    c.centered_text(x,y,&app_initial(name),size*0.55,W::Semibold,theme::WHITE,H::Center);
}
fn app_initial(name:&str)->String{name.chars().next().map(|ch|ch.to_uppercase().to_string()).unwrap_or_default()}

fn notice_icon(c:&Card,x:f32,y:f32,a:&Alert) {
    if a.source!=AlertSource::Internal{crate::visuals::paint::bell(c,x,y,theme::WHITE,0.0);return;}
    match a.title.as_str() {
        "系统音量"=>{
            c.poly(&[(x-9.0,y-3.0),(x-5.0,y-3.0),(x,y-7.0),(x,y+7.0),(x-5.0,y+3.0),(x-9.0,y+3.0)],theme::WHITE);
            c.arc(x,y,5.0,-0.8,0.8,1.5,theme::WHITE);c.arc(x,y,8.0,-0.7,0.7,1.3,theme::WHITE);
        },
        "大写锁定"=>{
            let ink=if a.body=="已开启"{theme::GREEN}else{theme::GRAY};
            c.poly(&[(x-8.0,y-1.0),(x,y-9.0),(x+8.0,y-1.0)],ink);
            c.fill(x-3.0,y-1.5,6.0,6.5,0.5,ink);
            c.fill(x-4.0,y+7.0,8.0,2.0,0.8,ink);
        },
        "键盘输入语言"=>{
            c.stroke(x-10.0,y-8.5,20.0,17.0,4.0,1.2,theme::GRAY);
            let label=match a.body.as_str(){"中文"=>"中","英语"=>"EN",_=>"Aa"};
            c.centered_text(x,y,label,if label=="EN"{9.0}else{11.0},W::Semibold,theme::WHITE,H::Center);
        },
        "网络连接"=>{
            let connected=a.body.contains("已连接");
            let ink=if connected{theme::GREEN}else{theme::GRAY};
            if a.body.starts_with("Wi-Fi") {
                for r in [4.0,8.0,12.0]{c.arc(x,y+6.0,r,-2.35,-0.79,1.7,ink);}
                c.circle(x,y+6.0,1.6,ink);
            }else if a.body.starts_with("以太网") {
                c.stroke(x-8.0,y-8.0,16.0,14.0,2.5,1.6,ink);
                for dx in [-4.0,-1.3,1.3,4.0]{c.seg((x+dx,y-5.0),(x+dx,y-1.0),1.2,ink);}
                c.path(&[(x-4.0,y+6.0),(x-4.0,y+9.0),(x+4.0,y+9.0),(x+4.0,y+6.0)],1.6,ink);
            }else{
                c.ring(x,y,8.0,1.4,ink);c.seg((x-8.0,y),(x+8.0,y),1.0,ink);
                c.stroke(x-3.5,y-8.0,7.0,16.0,3.5,1.0,ink);
            }
            if !connected{c.seg((x-8.0,y-8.0),(x+9.0,y+9.0),2.0,theme::WHITE);}
        },
        "电源已连接"|"已切换电池供电"|"电量偏低"=>{
            let low=a.title=="电量偏低";let ink=if low{theme::RED_TOP}else if a.title=="电源已连接"{theme::GREEN}else{theme::WHITE};
            c.stroke(x-11.0,y-6.0,20.0,12.0,3.0,1.5,ink);c.fill(x+10.0,y-2.5,2.0,5.0,0.8,ink);
            if low{c.fill(x-8.0,y-3.5,3.5,7.0,1.0,ink);}
            else if a.title=="电源已连接"{
                c.poly(&[(x,y-4.5),(x-4.0,y+0.8),(x+0.5,y+0.8)],ink);
                c.poly(&[(x+0.5,y-0.8),(x+4.0,y-0.8),(x-1.0,y+4.5)],ink);
            }else{c.fill(x-8.0,y-3.5,12.0,7.0,1.0,ink);}
        },
        "新邮件"|"收到新邮件"|"邮件验证码"=>crate::calendar::ui::source_icon(c,crate::calendar::ui::Source::Mail(""),x-14.0,y-14.0,28.0),
        "喝水时间到了"|"今日饮水目标达成"=>HealthReminder::Water.icon(c,x,y,1.0),
        crate::island::SIT_TITLE=>HealthReminder::Sit.icon(c,x,y,1.0),
        "待办到时间了"=>crate::visuals::paint::checklist(c,x,y,23.0,Color32::from_rgb(190,180,255),Color32::from_rgb(43,35,66)),
        "会议提醒"=>{
            let label=a.meeting.as_ref().map(|(_,source)|source.as_str()).unwrap_or("");
            let source=if label=="本机日程"{crate::calendar::ui::Source::Local}else if label.starts_with("邮件"){crate::calendar::ui::Source::Mail("")}
                else{crate::calendar::ui::Source::Calendar(crate::calendar::Provider::ALL.into_iter().find(|provider|provider.label()==label))};
            crate::calendar::ui::source_icon(c,source,x-14.0,y-14.0,28.0);
        },
        "专注计时"=>crate::visuals::paint::stopwatch(&c.scaled(x,y,0.65),x,y,8.0,0.0,theme::RED_TOP),
        "岛仔已暂停音乐功能"=>page_icon(c,crate::island::MUSIC_PAGE,x,y,24.0),
        _=>page_icon(c,a.page,x,y,24.0),
    }
}

impl App {
    pub fn alert(
        &mut self,
        title: impl Into<String>,
        body: impl Into<String>,
        page: usize,
        system: bool,
    ) {
        let title = title.into();
        if let Some(kind)=HealthReminder::ALL.into_iter().find(|kind|title==kind.title()) {
            if self.health_pending[kind as usize]{return;}
            self.health_pending[kind as usize]=true;
            crate::model::save_health_pending(self.health_pending);
        }
        let transient = matches!(
            title.as_str(),
            "系统音量" | "大写锁定" | "键盘输入语言" | "网络连接" | "电源已连接" | "已切换电池供电" | "蓝牙设备"
        );
        let a = Alert {
            source: AlertSource::Internal,
            title,
            body: body.into(),
            page,
            at: self.t,
            duration: if transient { 2.5 } else { 8.0 },
            bluetooth: None,
            meeting: None,
            unread: None,
            banner_at: self.t,
        };
        self.enqueue_alert(a, system);
    }
    pub(crate) fn meeting_alert(&mut self, meeting: crate::mail::Meeting, source: String, page: usize) {
        let left=meeting.start-chrono::Local::now().timestamp();
        self.calendar_badge_until=self.t+if left>0 {left as f64}else{crate::island::page_badges::HOLD};
        self.enqueue_alert(Alert {
            source: AlertSource::Internal,
            title: "会议提醒".into(), body: format!("{} · {} · {}", meeting.title, meeting.at, source),
            page, at: self.t, duration: 12.0, bluetooth: None, meeting: Some((meeting, source)), unread: None,
            banner_at: self.t,
        }, true);
    }
    pub(crate) fn bluetooth_alert(&mut self, device: &crate::os::bt::Device, connected: bool) {
        if !self.settings.bluetooth_notifications { return; }
        if connected && matches!(device.kind,crate::os::bt::DeviceKind::Bone|crate::os::bt::DeviceKind::Earbuds|crate::os::bt::DeviceKind::Headphones|crate::os::bt::DeviceKind::Speaker) {
            self.alerts.retain(|a|a.title!="系统音量");
            if self.notice.as_ref().is_some_and(|a|a.title=="系统音量") {self.notice=None;}
        }
        let a = Alert {
            source: AlertSource::Internal,
            title: "蓝牙设备".into(),
            body: if connected { device.summary() } else { format!("{} · 已断开", device.name) },
            page: 5, at: self.t, duration: 2.5,
            bluetooth: Some((device.kind, connected)),
            meeting: None,
            unread: None,
            banner_at: self.t,
        };
        self.enqueue_alert(a, false);
    }
    fn enqueue_alert(&mut self, a: Alert, system: bool) {
        // 通知仅保存在本次运行的内存中。
        if a.source==AlertSource::Internal {self.history.push_front(a.clone());}
        self.history.truncate(50);
        if self.fullscreen_hidden && a.health().is_some(){return;}
        if self.settings.notifications {
            if a.source==AlertSource::Internal {
                let sound = if self.settings.sound { a.notification_sound() } else { NotificationSound::Silent };
                if system {
                    if self.settings.system_notify {
                        self.pending.push(Cmd::Notify(a.title.clone(), a.body.clone(), sound));
                    }
                } else if sound==NotificationSound::Email {
                    // 新邮件不发 Windows Toast，只播放岛内铃声。
                    self.pending.push(Cmd::PlayNotificationSound(sound));
                }
            }
            let same_event=|old:&Alert| match (&a.meeting,&old.meeting) {
                (Some((next,_)),Some((previous,_)))=>crate::calendar::dedupe_key(next)==crate::calendar::dedupe_key(previous),
                _=>old.source==a.source && old.title==a.title && old.unread==a.unread && (a.is_hud() || old.body==a.body),
            };
            if a.source!=AlertSource::Internal || a.title != "蓝牙设备" { self.alerts.retain(|old| !same_event(old)); }
            if a.banner_at <= self.t && (a.source!=AlertSource::Internal || a.title != "蓝牙设备") && self.notice.as_ref().is_some_and(same_event) {
                if a.lights_bell() {
                    self.notification_bell_at=self.t;
                    self.notification_bell_until=self.t+a.duration;
                }
                self.notice = Some(a);
                return;
            }
            if a.banner_at > self.t && self.notice.as_ref().is_some_and(same_event) {
                self.notice = None;
            }
            self.alerts.push_back(a);
            while self.alerts.len() > 8 {
                self.alerts.pop_front();
            }
        }
    }
    /// 该来源的所有账号都已查看。
    pub fn viewed_messenger(&mut self,source:crate::island::Messenger){
        self.pending_messengers.retain(|u|u.source!=source);
        self.clear_messenger_notices(source);
    }
    /// 只清已查看的账号；无法确认账号的 QQ 提醒由用户明确关闭。
    pub fn viewed_client(&mut self,source:crate::island::Messenger,pid:u32){
        self.pending_messengers.retain(|u|!u.viewed_by(source,pid));
        if pid!=0{self.clear_account_notices(source,pid);}
        if !source.adapter().keep_unidentified{self.clear_account_notices(source,0);}
        if !self.pending_messengers.iter().any(|u|u.source==source){self.clear_messenger_notices(source);}
    }
    pub fn confirm_messenger_open(&mut self,unread:crate::island::Unread,foreground:Option<(crate::island::Messenger,u32)>)->bool{
        if unread.pid==0 || foreground!=Some((unread.source,unread.pid)){return false;}
        self.viewed_client(unread.source,unread.pid);
        true
    }
    pub fn discard_unread(&mut self,unread:crate::island::Unread){
        if !self.pending_messengers.contains(&unread){return;}
        self.pending_messengers.retain(|current|*current!=unread);
        self.clear_account_notices(unread.source,unread.pid);
        if !self.pending_messengers.iter().any(|current|current.source==unread.source){self.clear_messenger_notices(unread.source);}
    }
    fn clear_account_notices(&mut self,source:crate::island::Messenger,pid:u32){
        let matches=|notice:&Alert|notice.unread.is_some_and(|u|u.source==source && u.pid==pid);
        self.alerts.retain(|notice|!matches(notice));
        if self.notice.as_ref().is_some_and(matches){self.notice=None;}
    }
    fn clear_messenger_notices(&mut self,source:crate::island::Messenger){
        self.alerts.retain(|a|a.messenger()!=Some(source));
        if self.notice.as_ref().is_some_and(|a|a.messenger()==Some(source)){self.notice=None;}
    }
    fn mark_messenger(&mut self,source:crate::island::Messenger,pid:u32,window:isize){
        if let Some(index)=self.pending_messengers.iter().position(|u|u.source==source && u.pid==pid){
            if window!=0 && self.pending_messengers[index].window!=window {
                self.pending_messengers[index].window=window;
                if source.adapter().capture==crate::messenger::Capture::Taskbar{
                    let unread=self.pending_messengers[index];
                    for alert in self.alerts.iter_mut().chain(self.notice.iter_mut()){
                        if alert.unread.is_some_and(|old|old.source==source && old.pid==pid){alert.unread=Some(unread);}
                    }
                }else{self.clear_account_notices(source,pid);}
            }
            return;
        }
        self.pending_messengers.push(crate::island::Unread{source,pid,window});
    }
    /// 横幅「查看」：跳到该来源最近一条未读的窗口。
    pub(crate) fn latest_unread(&self,source:crate::island::Messenger)->crate::island::Unread{
        self.pending_messengers.iter().rev().find(|u|u.source==source).copied().unwrap_or(crate::island::Unread::of(source))
    }
    pub fn receive_wechat_reminder(&mut self){self.receive_wechat_reminder_from(0,0);}
    /// 微信窗口闪动：`pid` 是该账号的进程，`window` 是闪动的窗口。
    pub fn receive_wechat_reminder_from(&mut self,pid:u32,window:isize){
        self.receive_messenger_reminder(crate::island::Unread{source:crate::island::Messenger::Wechat,pid,window});
    }
    pub fn receive_messenger_reminder(&mut self,unread:crate::island::Unread){
        let client=unread.source;
        if client.adapter().capture!=crate::messenger::Capture::Taskbar || !client.enabled(&self.settings){return;}
        let pending=self.pending_messengers.iter().any(|old|old.source==client && old.pid==unread.pid);
        self.mark_messenger(client,unread.pid,unread.window);
        if pending{return;}
        let hold=self.settings.notice_hold_secs;
        self.enqueue_alert(Alert{source:AlertSource::Messenger(client),title:client.name().into(),body:format!("有新消息 · 请到{}查看",client.name()),page:usize::MAX,at:self.t,duration:hold,bluetooth:None,meeting:None,unread:Some(unread),banner_at:self.t},false);
    }
    pub fn set_messenger_status(&mut self,client:crate::island::Messenger,status:String){self.messenger_statuses.insert(client,status);}
    pub fn messenger_status(&self,client:crate::island::Messenger)->&str{self.messenger_statuses.get(&client).map(String::as_str).unwrap_or("")}
    pub fn receive_notification(&mut self, app:String, id:String, body:String){
        self.receive_notification_from(app,id,body,None);
    }
    pub fn receive_notification_from(&mut self,app:String,id:String,body:String,unread:Option<crate::island::Unread>){
        if !self.settings.read_notifications || id.is_empty(){return;}
        if crate::messenger::suppresses_toast(&app,&id,crate::messenger::toast_suppression(&self.settings)){return;}
        let known=crate::island::Messenger::from_notification_id(&id);
        let client=known.filter(|client|client.adapter().capture==crate::messenger::Capture::SystemToast);
        if known==Some(crate::island::Messenger::Qq) && !self.settings.qq_notifications {return;}
        // QQ 等即时通讯有独立的驻岛图标；只有非即时通讯的 Windows 通知点亮铃铛。
        if known.is_none() && !self.module_on(crate::island::WINDOWS_PAGE) {return;}
        let id=client.map(|client|client.adapter().notification_id.to_string()).unwrap_or(id);
        // 岛仔自己发出的系统提醒会被通知中心回传；已在历史里的同一内容不再入队，避免重复显示。
        if crate::messenger::Messenger::from_notification_id(&id).is_none() && self.history.iter().any(|a|format!("{} · {}",a.title,a.body)==body){return;}
        if !self.notification_apps.observe(&id,&app) || !self.settings.notifications{return;}
        let unread=client.map(|client|unread.filter(|u|u.source==client && u.pid!=0).unwrap_or(crate::island::Unread::of(client)));
        if let Some(unread)=unread{self.mark_messenger(unread.source,unread.pid,unread.window);}
        let hold=self.settings.notice_hold_secs;
        let title=app.chars().filter(|c|!c.is_control()).take(80).collect::<String>();
        let body=if self.settings.notification_preview {
            let mut text=body.chars().filter(|c|!c.is_control()).take(300).collect::<String>();
            if let Some(code)=crate::mail::otp::extract("",&text,&[]) {text=crate::mail::privacy::mask(&text,&code.text);}
            text
        }else{"收到新通知 · 内容已隐藏".into()};
        if known.is_none() {
            self.notification_bell_at=self.t;
            self.notification_bell_until=self.t+hold;
        }
        self.enqueue_alert(Alert{source:AlertSource::External(id.clone()),title:title.clone(),body:body.clone(),page:usize::MAX,at:self.t,duration:hold,bluetooth:None,meeting:None,unread,banner_at:self.t},false);
        if known.is_none() {
            self.windows_notices.push(crate::island::WindowsNotice{app:title,id,body,time:chrono::Local::now().format("%H:%M").to_string()});
            if self.windows_notices.len()>30 {self.windows_notices.remove(0);}
            if !(self.open && self.page==crate::island::WINDOWS_PAGE) {self.windows_unread=true;}
        }
    }
    pub fn reveal_messenger_notice(&mut self,unread:crate::island::Unread){
        // 只有会保留未确认账号的客户端需要这条「无法打开」说明。
        if !unread.source.adapter().keep_unidentified || !self.pending_messengers.contains(&unread){return;}
        let body=if unread.pid==0{"消息账号暂无法确认"}else{"暂时无法打开对应窗口"};
        let source=if unread.source.adapter().capture==crate::messenger::Capture::Taskbar{AlertSource::Messenger(unread.source)}else{AlertSource::External(unread.source.adapter().notification_id.into())};
        let hold=self.settings.notice_hold_secs;
        self.notice=Some(Alert{source,title:unread.source.name().into(),body:body.into(),page:usize::MAX,at:self.t,duration:hold,bluetooth:None,meeting:None,unread:Some(unread),banner_at:self.t});
    }
    pub(crate) fn tick_alerts(&mut self) {
        self.pending_messengers.retain(|u|u.source.enabled(&self.settings)
            && (u.source!=crate::island::Messenger::Qq || self.settings.qq_notifications)
            && (u.source.adapter().capture!=crate::messenger::Capture::SystemToast
            || self.notification_apps.items.iter().any(|(key,_,on)|key==u.source.adapter().notification_id && *on)));
        let allowed=|a:&Alert|match &a.source {
            AlertSource::Internal=>!self.fullscreen_hidden || a.health().is_none(),
            AlertSource::Messenger(client)=>client.enabled(&self.settings),
            AlertSource::External(id)=>self.settings.read_notifications && self.settings.notifications
                && (crate::messenger::Messenger::from_notification_id(id)!=Some(crate::island::Messenger::Qq) || self.settings.qq_notifications)
                && !crate::messenger::suppresses_toast(&a.title,id,crate::messenger::toast_suppression(&self.settings)) && self.notification_apps.items.iter().any(|(key,_,on)|key==id && *on),
        };
        self.alerts.retain(&allowed);
        if self.notice.as_ref().is_some_and(|a|!allowed(a)){self.notice=None;}
        if self.hud_notice.as_ref().is_some_and(|a|!allowed(a)){self.hud_notice=None;}
        if self.fullscreen_hidden {
            self.pending.retain(|cmd|!matches!(cmd,Cmd::Notify(title,..)if HealthReminder::ALL.into_iter().any(|kind|title==kind.title())));
        }
        if !self.settings.notification_preview {
            for a in self.alerts.iter_mut().chain(self.notice.iter_mut()).chain(self.hud_notice.iter_mut()) {
                if matches!(&a.source,AlertSource::External(_)){a.body="收到新通知 · 内容已隐藏".into();}
            }
        }
        let held=self.notice.as_ref().is_some_and(|a|self.open && !a.is_hud());
        if held {
            if let Some(a)=self.notice.as_mut(){a.at=self.t;}
        } else if self.notice.as_ref().is_some_and(|a|self.t - a.at > a.duration) {
            self.notice = None;
        }
        if self.notice.is_none() {
            let front_hud=self.alerts.front().is_some_and(|a|a.is_hud());
            let ready=(!self.open || front_hud) && self.alerts.front().is_some_and(|a|a.banner_at<=self.t);
            if ready {
                if let Some(mut a) = self.alerts.pop_front() {
                    a.at = self.t;
                    if a.lights_bell() {
                        self.notification_bell_at=self.t;
                        self.notification_bell_until=self.t+a.duration;
                    }
                    self.notice = Some(a);
                }
            }
        }
    }
    pub(crate) fn draw_notice(&mut self, c:&Card,panel:Rect,a:&Alert,_cmds:&mut Vec<Cmd>){
        let nc=c.clipped(0.0,0.0,panel.width(),panel.height());
        if let Some(source)=a.messenger(){
            messenger_icon(&nc,22.0,22.0,source);
            nc.circle(31.0,14.0,2.6,Color32::from_rgb(255,72,72));
            nc.centered_text(44.0,14.0,&a.title,11.0,W::Semibold,theme::WHITE,H::Left);
            nc.centered_text(44.0,30.0,&nc.fit(&a.body,10.5,W::Regular,154.0),10.5,W::Regular,theme::GRAY,H::Left);
            if notice_button(&nc,206.0,12.0,52.0,"查看","messenger-view"){_cmds.push(Cmd::ViewMessenger(a.unread.unwrap_or_else(||self.latest_unread(source))));}
            if notice_button(&nc,265.0,12.0,52.0,"关闭","messenger-dismiss"){
                // 已确认账号的未读留到打开客户端后清除；保留未确认账号的客户端只能在这里明确忽略。
                if let Some(unread)=a.unread.filter(|unread|unread.pid==0 && unread.source.adapter().keep_unidentified){self.discard_unread(unread);}
                self.notice=None;
            }
            return;
        }
        if a.source!=AlertSource::Internal {
            crate::visuals::paint::bell(&nc,322.0,22.0,theme::GRAY,crate::visuals::paint::bell_swing(self.t-a.at));
            nc.circle(326.0,17.5,2.0,Color32::from_rgb(255,72,72));
            nc.centered_text(16.0,14.0,&nc.fit(&a.title,11.0,W::Semibold,198.0),11.0,W::Semibold,theme::WHITE,H::Left);
            nc.centered_text(16.0,30.0,&nc.fit(&a.body,10.5,W::Regular,198.0),10.5,W::Regular,theme::GRAY,H::Left);
            if notice_button(&nc,220.0,12.0,44.0,"查看","external-view"){
                self.show_page(crate::island::WINDOWS_PAGE);
            }
            if notice_button(&nc,270.0,12.0,44.0,"关闭","external-dismiss"){
                self.notice=None;
                if a.lights_bell() {self.notification_bell_until=0.0;}
            }
            return;
        }
        if let Some((meeting,_))=&a.meeting {
            let secondary=Color32::from_gray(174);
            notice_icon(&nc.scaled(34.0,46.0,36.0/28.0),34.0,46.0,a);
            let now=chrono::Local::now();
            let start=chrono::DateTime::from_timestamp(meeting.start,0).unwrap().with_timezone(&chrono::Local);
            let left=meeting.start-now.timestamp();
            let countdown=if left>0 {format!("{} 分钟后",(left+59)/60)}else{"正在开始".into()};
            let countdown_width=nc.text_w(&countdown,10.5,W::Semibold)+16.0;
            let countdown_x=344.0-countdown_width;
            nc.centered_text(64.0,33.0,&start.format("%H:%M").to_string(),30.0,W::Semibold,theme::WHITE,H::Left);
            nc.fill(countdown_x,23.0,countdown_width,20.0,10.0,Color32::from_gray(28));
            nc.centered_text(countdown_x+countdown_width/2.0,33.0,&countdown,10.5,W::Semibold,theme::WHITE,H::Center);
            let overflow=(nc.text_w(&meeting.title,size::TITLE,W::Semibold)-280.0).max(0.0);
            let offset=(((self.t-a.at-1.2).max(0.0) as f32)*24.0).min(overflow);
            nc.clipped(64.0,52.0,280.0,24.0).centered_text(64.0-offset,64.0,&meeting.title,size::TITLE,W::Semibold,theme::WHITE,H::Left);
            nc.seg((16.0,84.0),(344.0,84.0),0.6,Color32::from_gray(36));
            let date=format!("{}{}",if start.date_naive()==now.date_naive(){"今天 · "}else{""},start.format("%-m月%-d日"));
            nc.centered_text(16.0,110.0,&date,11.0,W::Regular,secondary,H::Left);
            let (dismiss,dc)=crate::visuals::paint::capsule(&nc,188.0,98.0,72.0,24.0,"meeting-dismiss",Color32::from_gray(32),Color32::from_gray(48));
            dc.centered_text(224.0,110.0,"关闭",10.5,W::Semibold,theme::WHITE,H::Center);
            if dismiss.on_hover_text("关闭本次提醒，日程仍保留").clicked(){
                self.notice=None;
                return;
            }
            let (button,bc)=crate::visuals::paint::capsule(&nc,272.0,98.0,72.0,24.0,"meeting-action",Color32::from_gray(42),Color32::from_gray(56));
            bc.centered_text(308.0,110.0,"查看",10.5,W::Semibold,theme::WHITE,H::Center);
            if button.clicked() {
                self.notice=None;
                self.open_calendar(Some(meeting.clone()));
            }
            return;
        }
        if a.is_hud(){
            if a.title=="蓝牙设备" {
                let (name,status,battery)=bluetooth_notice_content(&a.body);
                nc.circle(26.0,24.0,14.0,if status=="已连接"{Color32::from_rgb(10,132,255)}else{Color32::from_gray(95)});
                let kind=a.bluetooth.map(|(kind,_)|kind).unwrap_or(crate::os::bt::DeviceKind::Other);
                device_icon(&nc,kind,17.0,15.0,18.0);
                let right=HUD_W-14.0;
                let name_end=if let Some(level)=battery {
                    let pct=format!("{level}%");
                    let pct_width=nc.text_w(&pct,10.5,W::Semibold);
                    let x=right-pct_width-22.0;
                    nc.stroke(x,20.0,15.0,8.0,2.0,1.0,Color32::from_gray(175));
                    nc.fill(x+15.5,22.5,2.0,3.0,0.8,Color32::from_gray(175));
                    let fill=11.0*level as f32/100.0;
                    if fill>0.0 {nc.fill(x+2.0,22.0,fill,4.0,1.0,if level<=20{theme::RED}else{theme::GREEN});}
                    nc.centered_text(right,24.0,&pct,10.5,W::Semibold,theme::WHITE,H::Right);
                    x-8.0
                } else {right};
                nc.centered_text(49.0,16.0,&nc.fit(name,11.0,W::Semibold,(name_end-49.0).max(20.0)),11.0,W::Semibold,theme::WHITE,H::Left);
                nc.centered_text(49.0,32.0,status,10.5,W::Regular,theme::GRAY,H::Left);
                return;
            }
            match a.title.as_str(){
                "系统音量"=>{
                    nc.poly(&[(17.0,16.0),(21.0,16.0),(26.0,12.0),(26.0,26.0),(21.0,22.0),(17.0,22.0)],theme::WHITE);
                    if self.volume_ui.is_some_and(|(_,muted)|muted){nc.seg((29.0,16.0),(34.0,22.0),1.4,theme::WHITE);nc.seg((34.0,16.0),(29.0,22.0),1.4,theme::WHITE);}
                    else{nc.arc(26.0,19.0,5.0,-0.8,0.8,1.5,theme::WHITE);nc.arc(26.0,19.0,8.0,-0.7,0.7,1.3,theme::WHITE);}
                },
                _=>notice_icon(&nc,24.0,19.0,a),
            }
            if a.title=="系统音量" {
                self.draw_volume_hud(&nc);
            } else {
                let (title,body)=if a.title=="键盘输入语言"{("输入语言",a.body.as_str())}
                    else if a.title=="已切换电池供电"{("电池供电",a.body.as_str())}
                    else if a.title=="网络连接"{a.body.split_once(" · ").unwrap_or(("网络",a.body.as_str()))}
                    else{(a.title.as_str(),a.body.as_str())};
                nc.centered_text(48.0,19.0,&nc.fit(title,11.0,W::Semibold,90.0),11.0,W::Semibold,theme::WHITE,H::Left);
                nc.centered_text(HUD_W-16.0,19.0,&nc.fit(body,10.5,W::Regular,92.0),10.5,W::Regular,if a.title=="大写锁定"&&a.body=="已开启"||a.title=="网络连接"&&a.body.contains("已连接"){theme::GREEN}else{theme::GRAY},H::Right);
            }
        } else {
            let is_sit=a.health()==Some(HealthReminder::Sit);let is_water=a.health()==Some(HealthReminder::Water);let is_focus=a.page==1;let is_power=a.title=="电量偏低";
            notice_icon(&nc,22.0,22.0,a);
            let max=if is_water||is_sit{125.0}else{195.0};
            nc.centered_text(43.0,14.0,&nc.fit(&a.title,11.0,W::Semibold,max),11.0,W::Semibold,theme::WHITE,H::Left);
            nc.centered_text(43.0,30.0,&nc.fit(&a.body,10.5,W::Regular,max),10.5,W::Regular,theme::GRAY,H::Left);
            if is_sit {
                if notice_button(&nc,181.0,12.0,62.0,"知道了","sit-confirm-read"){self.confirm_health_reminder(HealthReminder::Sit);}
                if notice_button(&nc,250.0,12.0,68.0,"起身了","sit-confirm"){self.notice=None;self.stood_up();}
            }else if is_water {
                if notice_button(&nc,181.0,12.0,62.0,"知道了","water-confirm-read"){self.confirm_health_reminder(HealthReminder::Water);}
                if notice_button(&nc,250.0,12.0,68.0,"喝一口","water-confirm"){self.notice=None;self.drink_water();}
            }else{
                let label=if is_focus{"开始"}else if is_power{"知道了"}else{"查看"};
                if notice_button(&nc,257.0,12.0,60.0,label,"notice-action"){
                    self.notice=None;if is_power{return;}if is_focus{self.pomo.running=true;}else if a.page<PAGE_NAMES.len(){
                        self.show_page(a.page);
                        if a.page==4 {
                            let order=self.todos.display_order(&self.today);
                            if let Some(index)=order.iter().position(|&i|self.todos.items[i].text==a.body && !self.todos.items[i].done_for(&self.today)){
                                self.todo_first=index.min(order.len().saturating_sub(3));
                            }
                        }
                    }else{self.settings_tab=6;self.open_settings();}
                }
            }
            if let Some(kind)=a.health() {
                if nc.hit(0.0,0.0,175.0,44.0,"health-notice-confirm").clicked(){self.confirm_health_reminder(kind);}
            }
        }
    }
    /// v0.5.4：可交互音量条。样式参考 oyenza-studio（MIT）的 HUD 音量条：
    /// 圆角胶囊、填充缓动、拖动时加粗并出现手柄；拖动/滚轮改音量，点喇叭静音。
    pub(crate) fn draw_volume_hud(&mut self, nc: &Card) {
        let (v, muted) = self.volume_ui.unwrap_or((0, false));
        let v = v.min(100);
        // 喇叭：点击静音 / 恢复
        let icon = nc.hit(12.0, 9.0, 28.0, 22.0, "volume-mute");
        if icon.clicked() && nc.live {
            self.volume_ui = Some((v, !muted));
            self.volume_quiet_until = self.t + 0.9;
            self.pending.push(Cmd::ToggleMute);
        }
        // 滑条：拖动 / 点击定位；滚轮微调
        let track = nc.hit_drag(HUD_TRACK_X, 8.0, HUD_TRACK_W, 24.0, "volume-track");
        let mut want: Option<u32> = None;
        if track.dragged() || track.clicked() {
            if let Some(p) = track.interact_pointer_pos() {
                let x = nc.to_local(p).x;
                want = Some((((x - HUD_TRACK_X) / HUD_TRACK_W) * 100.0).clamp(0.0, 100.0).round() as u32);
            }
        } else if nc.live && track.hovered() {
            let dy = nc.ui.ctx().input(|i| i.raw_scroll_delta.y);
            if dy.abs() > 0.5 {
                want = Some(((v as f32 + (dy / 120.0) * 4.0).clamp(0.0, 100.0)).round() as u32);
            }
        }
        if let Some(nv) = want {
            if self.volume_ui.map(|(o, _)| o) != Some(nv) || muted {
                self.volume_ui = Some((nv, false));
                self.volume_quiet_until = self.t + 0.9;
                self.pending.push(Cmd::SetVolume(nv));
            }
        }
        let (v, muted) = self.volume_ui.unwrap_or((v, muted));
        let pressed = track.dragged() || track.is_pointer_button_down_on() || icon.is_pointer_button_down_on();
        let target = if muted { 0.0 } else { v as f32 / 100.0 };
        self.volume_a = crate::visuals::anim::approach(self.volume_a, target, if pressed { 34.0 } else { 13.0 }, self.dt);
        let h = if pressed { 7.0 } else { 5.0 };
        let y = 19.0 - h / 2.0;
        nc.fill(HUD_TRACK_X, y, HUD_TRACK_W, h, h / 2.0, Color32::from_white_alpha(if pressed { 46 } else { 34 }));
        if self.volume_a > 0.0 {
            let w = (HUD_TRACK_W * self.volume_a).max(h);
            let col = if muted { Color32::from_gray(150) } else { theme::WHITE };
            nc.fill(HUD_TRACK_X, y, w, h, h / 2.0, col);
            nc.fill(HUD_TRACK_X, y + 0.8, (w - 1.2).max(0.0), h * 0.4, h * 0.2, Color32::from_white_alpha(64));
            if pressed {
                nc.circle(HUD_TRACK_X + w - h / 2.0, y + h / 2.0, h * 0.95, col);
            }
        }
        let label = if muted { "静音".to_string() } else { format!("{v}%") };
        nc.centered_text(HUD_W - 26.0, 19.0, &label, 10.5, if pressed { W::Semibold } else { W::Regular }, if muted { theme::GRAY } else { theme::WHITE }, H::Center);
    }
}

fn notice_button(c:&Card,x:f32,y:f32,w:f32,text:&str,id:&str)->bool {
    let (r,b)=crate::visuals::paint::capsule(c,x,y,w,20.0,id,Color32::from_gray(35),theme::BTN_HOVER);
    b.centered_text(x+w/2.0,y+10.0,text,10.5,W::Semibold,theme::WHITE,H::Center);
    r.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bluetooth_notice_uses_real_connection_and_battery_data(){
        assert_eq!(bluetooth_notice_content("OpenMove by Shokz · 电量 80%"),("OpenMove by Shokz","已连接",Some(80)));
        assert_eq!(bluetooth_notice_content("耳机"),("耳机","已连接",None));
        assert_eq!(bluetooth_notice_content("已断开连接"),("蓝牙设备","已断开",None));
        assert_eq!(bluetooth_notice_content("青龙键盘 · 已断开"),("青龙键盘","已断开",None));
        assert_eq!(bluetooth_notice_content("耳机 · 电量 101%"),("耳机 · 电量 101%","已连接",None));
    }
    #[test]
    fn app_icon_falls_back_to_the_name_initial(){
        assert_eq!(app_initial("Telegram"),"T");
        assert_eq!(app_initial("应用0"),"应");
        assert_eq!(app_initial(""),"");
    }
}
