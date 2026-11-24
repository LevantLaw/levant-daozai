//! 通知队列、岛仔表情与设置页的公共入口。
use crate::app::{App, Cmd, Env, NotificationSound, Phase, HUD_TRACK_W, HUD_TRACK_X, HUD_W, PAGE_NAMES};

pub(crate) use crate::app::settings::{
    page_icon, setting_row, setting_row_nested, settings_action_row, settings_apply_style,
    settings_card, settings_control_row, settings_control_row_right, settings_danger_button,
    settings_group_fold, settings_hint, settings_label, settings_modal_frame, settings_modal_title,
    settings_primary_button, settings_segmented,
};
#[cfg(test)]
pub(crate) use crate::app::settings::{
    island_size_slider, number_stepper, setting_number, settings_feature_rows,
    settings_tab_group_for_tests, settings_tab_module, SETTINGS_GROUPS, SETTINGS_SECTIONS,
    SETTINGS_TAB_APPEARANCE, SETTINGS_TAB_BEHAVIOR, SETTINGS_TAB_BODY, SETTINGS_TAB_CLIPBOARD,
    SETTINGS_TAB_PAGES, SETTINGS_TAB_REMINDERS, SETTINGS_TAB_WINDOWS,
};
use crate::paint::{Card, H};
use crate::theme::{self, size, W};
use eframe::egui::{self, Color32, Rect};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AlertSource { Internal, Messenger(crate::app::Messenger), External(String) }

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum HealthReminder {Water,Sit}
impl HealthReminder {
    pub(crate) const ALL:[Self;2]=[Self::Water,Self::Sit];
    pub(crate) fn title(self)->&'static str {match self {Self::Water=>"喝水时间到了",Self::Sit=>crate::app::SIT_TITLE}}
    pub(crate) fn label(self)->&'static str {match self {Self::Water=>"待喝水",Self::Sit=>"待活动"}}
    pub(crate) fn id(self)->&'static str {match self {Self::Water=>"health-water",Self::Sit=>"health-sit"}}
    pub(crate) fn icon(self,c:&Card,x:f32,y:f32,scale:f32) {
        let c=&c.scaled(x,y,scale);
        match self {
            Self::Water=>{
                c.poly(&[(x-7.0,y-1.0),(x+5.0,y-1.0),(x+3.5,y+8.0),(x-5.5,y+8.0)],theme::BLUE);
                c.path(&[(x-9.0,y-10.0),(x-6.5,y+9.0),(x+4.5,y+9.0),(x+7.0,y-10.0)],1.6,theme::CYAN);
                c.seg((x-9.0,y-10.0),(x+7.0,y-10.0),1.6,theme::CYAN);
                c.path(&[(x+6.5,y-6.0),(x+10.5,y-6.0),(x+11.5,y-3.0),(x+10.0,y+2.0),(x+5.5,y+3.0)],1.6,theme::CYAN);
                c.seg((x-5.0,y-6.5),(x-4.2,y-2.5),1.0,Color32::from_white_alpha(170));
            },
            Self::Sit=>crate::paint::chair(c,x,y,0.6,theme::SOFT_GREEN),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Alert {
    pub title: String,
    pub source: AlertSource,
    pub body: String,
    pub page: usize,
    pub at: f64,
    pub duration: f64,
    pub bluetooth: Option<(crate::bt::DeviceKind, bool)>,
    pub meeting: Option<(crate::mail::Meeting, String)>,
    pub unread: Option<crate::app::Unread>,
    /// 最早可显示横幅的时刻；展开期间到达的通知等岛体收起后再显示。
    pub banner_at: f64,
}
impl Alert {
    /// 只有非即时通讯的 Windows 通知点亮铃铛；邮件、即时通讯和 AI 各有自己的入口。
    pub(crate) fn lights_bell(&self)->bool {
        matches!(&self.source,AlertSource::External(id) if crate::app::Messenger::from_notification_id(id).is_none())
    }
    pub(crate) fn health(&self)->Option<HealthReminder> {
        if self.source!=AlertSource::Internal{return None;}
        HealthReminder::ALL.into_iter().find(|kind|self.title==kind.title())
    }
    /// 固定铃声映射：完成类、提醒类、邮件，其余走默认。
    pub(crate) fn notification_sound(&self)->NotificationSound {
        match self.title.as_str() {
            "专注计时"|"待办到时间了"|"今日饮水目标达成"=>NotificationSound::Complete,
            "喝水时间到了"|crate::app::SIT_TITLE|"会议提醒"|"电量偏低"=>NotificationSound::Reminder,
            "新邮件"|"收到新邮件"|"邮件验证码"=>NotificationSound::Email,
            _=>NotificationSound::Sonar,
        }
    }
    pub(crate) fn messenger(&self)->Option<crate::app::Messenger>{
        match &self.source {
            AlertSource::Messenger(client)=>Some(*client),
            AlertSource::External(id)=>crate::app::Messenger::from_notification_id(id).filter(|client|client.adapter().capture==crate::messenger::Capture::SystemToast),
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

pub(crate) fn device_icon(c: &Card, kind: crate::bt::DeviceKind, x: f32, y: f32, size: f32) {
    let icon=match kind {
        crate::bt::DeviceKind::Bone | crate::bt::DeviceKind::Earbuds | crate::bt::DeviceKind::Headphones=>egui::include_image!("../assets/device-icons/headphones-round-bold.svg"),
        crate::bt::DeviceKind::Speaker=>egui::include_image!("../assets/device-icons/speaker-bold.svg"),
        crate::bt::DeviceKind::Keyboard=>egui::include_image!("../assets/device-icons/keyboard-bold.svg"),
        crate::bt::DeviceKind::Mouse=>egui::include_image!("../assets/device-icons/mouse-bold.svg"),
        crate::bt::DeviceKind::Gamepad=>egui::include_image!("../assets/device-icons/gamepad-bold.svg"),
        crate::bt::DeviceKind::Phone=>egui::include_image!("../assets/device-icons/smartphone-bold.svg"),
        crate::bt::DeviceKind::Other=>egui::include_image!("../assets/device-icons/bluetooth-linear.svg"),
    };
    let rect=c.rect(x,y,size,size);
    let egui::ImageSource::Bytes{uri,bytes}=icon else{return};
    let pixels=(rect.width()*c.ui.ctx().pixels_per_point()).ceil() as u32;
    let icon=egui::Image::from_bytes(format!("{uri}/{pixels}.svg"),bytes);
    if let Ok(egui::load::TexturePoll::Ready { texture })=icon.fit_to_exact_size(rect.size()).load_for_size(c.ui.ctx(),rect.size()) {
        c.painter.image(texture.id,rect,Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),Color32::WHITE);
    }
}

pub(crate) use crate::pet::PetState;

pub(crate) fn notice_badge(c:&Card,x:f32,y:f32,a:&Alert){
    if a.source==AlertSource::Internal && a.title=="蓝牙设备"{
        c.circle(x,y,11.0,if a.body.contains("已断开"){Color32::from_gray(95)}else{theme::BLUE});
        device_icon(c,a.bluetooth.map_or(crate::bt::DeviceKind::Other,|v|v.0),x-7.5,y-7.5,15.0);
    }else{notice_icon(&c.scaled(x,y,0.85),x,y,a);}
}
pub(crate) fn messenger_icon(c:&Card,x:f32,y:f32,source:crate::app::Messenger){(source.adapter().paint_icon)(c,x,y);}

/// 应用图标：有纹理就画纹理，否则用名称首字占位，不用 Windows 铃铛。
fn app_icon_badge(c:&Card,x:f32,y:f32,size:f32,name:&str,texture:Option<&egui::TextureHandle>){
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
    if a.source!=AlertSource::Internal{crate::paint::bell(c,x,y,theme::WHITE,0.0);return;}
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
        crate::app::SIT_TITLE=>HealthReminder::Sit.icon(c,x,y,1.0),
        "待办到时间了"=>crate::paint::checklist(c,x,y,23.0,Color32::from_rgb(190,180,255),Color32::from_rgb(43,35,66)),
        "会议提醒"=>{
            let label=a.meeting.as_ref().map(|(_,source)|source.as_str()).unwrap_or("");
            let source=if label=="本机日程"{crate::calendar::ui::Source::Local}else if label.starts_with("邮件"){crate::calendar::ui::Source::Mail("")}
                else{crate::calendar::ui::Source::Calendar(crate::calendar::Provider::ALL.into_iter().find(|provider|provider.label()==label))};
            crate::calendar::ui::source_icon(c,source,x-14.0,y-14.0,28.0);
        },
        "专注计时"=>crate::paint::stopwatch(&c.scaled(x,y,0.65),x,y,8.0,0.0,theme::RED_TOP),
        "岛仔已暂停音乐功能"=>page_icon(c,crate::app::MUSIC_PAGE,x,y,24.0),
        _=>page_icon(c,a.page,x,y,24.0),
    }
}

impl App {
    pub(crate) fn health_badges(&self)->Vec<HealthReminder> {
        if self.fullscreen_hidden || !self.module_on(crate::app::WATER_PAGE){return Vec::new();}
        HealthReminder::ALL.into_iter().filter(|kind|self.health_pending[*kind as usize]
            && (match kind {HealthReminder::Water=>self.water.remind,HealthReminder::Sit=>self.settings.sit_remind})
            && (self.open || !self.notice.as_ref().is_some_and(|a|a.health()==Some(*kind)))).collect()
    }
    pub(crate) fn confirm_health_reminder(&mut self,kind:HealthReminder) {
        self.health_pending[kind as usize]=false;
        crate::model::save_health_pending(self.health_pending);
        match kind {HealthReminder::Water=>self.last_reminder=self.t,HealthReminder::Sit=>self.sit_start=self.t}
        self.alerts.retain(|a|a.health()!=Some(kind));
        if self.notice.as_ref().is_some_and(|a|a.health()==Some(kind)){self.notice=None;}
        if self.hud_notice.as_ref().is_some_and(|a|a.health()==Some(kind)){self.hud_notice=None;}
    }
    pub(crate) fn phase_secs(&self, phase: Phase) -> f32 {
        (match phase {
            Phase::Focus => self.settings.focus_minutes,
            Phase::Rest => self.settings.rest_minutes,
        }) as f32
            * 60.0
    }
    /// 已连续久坐的分钟数。
    pub(crate) fn sat_minutes(&self) -> u32 {
        ((self.t - self.sit_start).max(0.0) / 60.0) as u32
    }
    /// 下一次久坐提醒的时刻：从上次确认或起身起算满 sit_minutes。
    pub(crate) fn sit_due(&self) -> f64 {
        self.sit_start + self.settings.sit_minutes as f64 * 60.0
    }
    /// 「起身了」：久坐计时归零。
    pub(crate) fn stood_up(&mut self) {
        self.confirm_health_reminder(HealthReminder::Sit);
        self.pet.react(PetState::Happy);
        self.say("久坐计时已重新开始");
    }
    /// 现在是否在午休免打扰时段内（支持跨午夜；起止相同视为未设置）。
    pub(crate) fn lunch_now(&self, env: &Env) -> bool {
        let (a, b) = (self.settings.lunch_start, self.settings.lunch_end);
        let m = env.hour() as u32 * 60 + env.minute();
        self.settings.lunch_rest && a != b && if a < b { m >= a && m < b } else { m >= a || m < b }
    }
    /// 喝水和久坐提醒此刻能否弹出：在提醒时段内，且不在午休。
    pub(crate) fn remind_window(&self, env: &Env) -> bool {
        self.water_hours(env.hour() as u32) && !self.lunch_now(env)
    }
    pub(crate) fn water_hours(&self, hour: u32) -> bool {
        let (a, b) = (self.settings.water_start, self.settings.water_end);
        if a == b {
            true
        } else if a < b {
            hour >= a && hour < b
        } else {
            hour >= a || hour < b
        }
    }
    pub(crate) fn pet_state(&self) -> PetState {
        if let Some((state, _)) = self.pet.reaction {
            state
        } else if self.pet.waking_until > self.t {
            PetState::Waking
        } else if self.pomo.running && self.pomo.phase == Phase::Focus {
            PetState::Focused
        } else if self.pet.worried {
            PetState::Worried
        } else if self.media.playing && self.media.is_music() {
            if self.notice.is_some() || self.hud_notice.is_some() {PetState::Awake}else{PetState::Music}
        } else if self.pet.hovered {
            PetState::Curious
        } else if self.idle_for() >= 180.0 {
            PetState::Sleeping
        } else if self.idle_for() >= self.opts.pet_idle {
            PetState::Dozing
        } else {
            PetState::Awake
        }
    }
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
        self.calendar_badge_until=self.t+if left>0 {left as f64}else{crate::page_badges::HOLD};
        self.enqueue_alert(Alert {
            source: AlertSource::Internal,
            title: "会议提醒".into(), body: format!("{} · {} · {}", meeting.title, meeting.at, source),
            page, at: self.t, duration: 12.0, bluetooth: None, meeting: Some((meeting, source)), unread: None,
            banner_at: self.t,
        }, true);
    }
    pub(crate) fn bluetooth_alert(&mut self, device: &crate::bt::Device, connected: bool) {
        if !self.settings.bluetooth_notifications { return; }
        if connected && matches!(device.kind,crate::bt::DeviceKind::Bone|crate::bt::DeviceKind::Earbuds|crate::bt::DeviceKind::Headphones|crate::bt::DeviceKind::Speaker) {
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
    pub fn viewed_messenger(&mut self,source:crate::app::Messenger){
        self.pending_messengers.retain(|u|u.source!=source);
        self.clear_messenger_notices(source);
    }
    /// 只清已查看的账号；无法确认账号的 QQ 提醒由用户明确关闭。
    pub fn viewed_client(&mut self,source:crate::app::Messenger,pid:u32){
        self.pending_messengers.retain(|u|!u.viewed_by(source,pid));
        if pid!=0{self.clear_account_notices(source,pid);}
        if !source.adapter().keep_unidentified{self.clear_account_notices(source,0);}
        if !self.pending_messengers.iter().any(|u|u.source==source){self.clear_messenger_notices(source);}
    }
    pub fn confirm_messenger_open(&mut self,unread:crate::app::Unread,foreground:Option<(crate::app::Messenger,u32)>)->bool{
        if unread.pid==0 || foreground!=Some((unread.source,unread.pid)){return false;}
        self.viewed_client(unread.source,unread.pid);
        true
    }
    pub fn discard_unread(&mut self,unread:crate::app::Unread){
        if !self.pending_messengers.contains(&unread){return;}
        self.pending_messengers.retain(|current|*current!=unread);
        self.clear_account_notices(unread.source,unread.pid);
        if !self.pending_messengers.iter().any(|current|current.source==unread.source){self.clear_messenger_notices(unread.source);}
    }
    fn clear_account_notices(&mut self,source:crate::app::Messenger,pid:u32){
        let matches=|notice:&Alert|notice.unread.is_some_and(|u|u.source==source && u.pid==pid);
        self.alerts.retain(|notice|!matches(notice));
        if self.notice.as_ref().is_some_and(matches){self.notice=None;}
    }
    fn clear_messenger_notices(&mut self,source:crate::app::Messenger){
        self.alerts.retain(|a|a.messenger()!=Some(source));
        if self.notice.as_ref().is_some_and(|a|a.messenger()==Some(source)){self.notice=None;}
    }
    fn mark_messenger(&mut self,source:crate::app::Messenger,pid:u32,window:isize){
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
        self.pending_messengers.push(crate::app::Unread{source,pid,window});
    }
    /// 横幅「查看」：跳到该来源最近一条未读的窗口。
    pub(crate) fn latest_unread(&self,source:crate::app::Messenger)->crate::app::Unread{
        self.pending_messengers.iter().rev().find(|u|u.source==source).copied().unwrap_or(crate::app::Unread::of(source))
    }
    pub fn receive_wechat_reminder(&mut self){self.receive_wechat_reminder_from(0,0);}
    /// 微信窗口闪动：`pid` 是该账号的进程，`window` 是闪动的窗口。
    pub fn receive_wechat_reminder_from(&mut self,pid:u32,window:isize){
        self.receive_messenger_reminder(crate::app::Unread{source:crate::app::Messenger::Wechat,pid,window});
    }
    pub fn receive_messenger_reminder(&mut self,unread:crate::app::Unread){
        let client=unread.source;
        if client.adapter().capture!=crate::messenger::Capture::Taskbar || !client.enabled(&self.settings){return;}
        let pending=self.pending_messengers.iter().any(|old|old.source==client && old.pid==unread.pid);
        self.mark_messenger(client,unread.pid,unread.window);
        if pending{return;}
        let hold=self.settings.notice_hold_secs;
        self.enqueue_alert(Alert{source:AlertSource::Messenger(client),title:client.name().into(),body:format!("有新消息 · 请到{}查看",client.name()),page:usize::MAX,at:self.t,duration:hold,bluetooth:None,meeting:None,unread:Some(unread),banner_at:self.t},false);
    }
    pub fn set_messenger_status(&mut self,client:crate::app::Messenger,status:String){self.messenger_statuses.insert(client,status);}
    pub fn messenger_status(&self,client:crate::app::Messenger)->&str{self.messenger_statuses.get(&client).map(String::as_str).unwrap_or("")}
    pub fn receive_notification(&mut self, app:String, id:String, body:String){
        self.receive_notification_from(app,id,body,None);
    }
    pub fn receive_notification_from(&mut self,app:String,id:String,body:String,unread:Option<crate::app::Unread>){
        if !self.settings.read_notifications || id.is_empty(){return;}
        if crate::messenger::suppresses_toast(&app,&id,crate::messenger::toast_suppression(&self.settings)){return;}
        let known=crate::app::Messenger::from_notification_id(&id);
        let client=known.filter(|client|client.adapter().capture==crate::messenger::Capture::SystemToast);
        if known==Some(crate::app::Messenger::Qq) && !self.settings.qq_notifications {return;}
        // QQ 等即时通讯有独立的驻岛图标；只有非即时通讯的 Windows 通知点亮铃铛。
        if known.is_none() && !self.module_on(crate::app::WINDOWS_PAGE) {return;}
        let id=client.map(|client|client.adapter().notification_id.to_string()).unwrap_or(id);
        // 岛仔自己发出的系统提醒会被通知中心回传；已在历史里的同一内容不再入队，避免重复显示。
        if crate::messenger::Messenger::from_notification_id(&id).is_none() && self.history.iter().any(|a|format!("{} · {}",a.title,a.body)==body){return;}
        if !self.notification_apps.observe(&id,&app) || !self.settings.notifications{return;}
        let unread=client.map(|client|unread.filter(|u|u.source==client && u.pid!=0).unwrap_or(crate::app::Unread::of(client)));
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
            self.windows_notices.push(crate::app::WindowsNotice{app:title,id,body,time:chrono::Local::now().format("%H:%M").to_string()});
            if self.windows_notices.len()>30 {self.windows_notices.remove(0);}
            if !(self.open && self.page==crate::app::WINDOWS_PAGE) {self.windows_unread=true;}
        }
    }
    pub fn reveal_messenger_notice(&mut self,unread:crate::app::Unread){
        // 只有会保留未确认账号的客户端需要这条「无法打开」说明。
        if !unread.source.adapter().keep_unidentified || !self.pending_messengers.contains(&unread){return;}
        let body=if unread.pid==0{"消息账号暂无法确认"}else{"暂时无法打开对应窗口"};
        let source=if unread.source.adapter().capture==crate::messenger::Capture::Taskbar{AlertSource::Messenger(unread.source)}else{AlertSource::External(unread.source.adapter().notification_id.into())};
        let hold=self.settings.notice_hold_secs;
        self.notice=Some(Alert{source,title:unread.source.name().into(),body:body.into(),page:usize::MAX,at:self.t,duration:hold,bluetooth:None,meeting:None,unread:Some(unread),banner_at:self.t});
    }
    /// 应用图标纹理：首次用到时从开始菜单取，失败也缓存，避免每帧重试。
    fn app_icon(&mut self,ctx:&egui::Context,id:&str)->Option<egui::TextureHandle>{
        if let Some(icon)=self.app_icons.get(id){return icon.clone();}
        if self.app_icons.len()>=32{self.app_icons.clear();}
        let icon=crate::icons::app_icon(id).map(|image|ctx.load_texture(format!("app-icon:{id}"),image,egui::TextureOptions::LINEAR));
        self.app_icons.insert(id.to_string(),icon.clone());
        icon
    }
    /// Windows 通知页：即时通讯以外的系统通知汇总在这里，每行带应用图标。
    pub(crate) fn page_windows(&mut self,c:&Card) {
        const ROW_H:f32=33.0;
        const ROW_GAP:f32=5.0;
        const LIST_TOP:f32=56.0;
        let count=self.windows_notices.len();
        let header=if count==0 {"暂无通知".to_string()} else {format!("{count} 条通知")};
        c.text(15.0,45.0,&header,10.5,W::Regular,theme::GRAY,H::Left);
        if count==0 {
            c.text(209.5,84.0,"还没有收到 Windows 通知",13.0,W::Bold,theme::WHITE,H::Center);
            c.text(209.5,102.0,"连接系统通知后，即时通讯以外的通知会汇总在这里",10.0,W::Regular,theme::GRAY,H::Center);
            return;
        }
        let (clear,body)=crate::paint::capsule(c,337.0,33.0,52.0,20.0,"windows-clear",theme::BTN,theme::BTN_HOVER);
        body.text(363.0,47.0,"清空",10.0,W::Semibold,theme::WHITE,H::Center);
        if clear.clicked(){self.windows_notices.clear();self.windows_first=0;return;}
        let max=count.saturating_sub(crate::app::WINDOWS_ROWS);
        self.windows_first=self.windows_first.min(max);
        let first=self.windows_first;
        let rows:Vec<_>=self.windows_notices.iter().skip(first).take(crate::app::WINDOWS_ROWS)
            .map(|notice|(notice.app.clone(),notice.id.clone(),notice.body.clone(),notice.time.clone())).collect();
        let ctx=c.ui.ctx();
        let icons:Vec<_>=rows.iter().map(|(_,id,_,_)|self.app_icon(ctx,id)).collect();
        for (slot,(app,_,body,time)) in rows.iter().enumerate() {
            let top=LIST_TOP+slot as f32*(ROW_H+ROW_GAP);
            c.fill(40.0,top,320.0,ROW_H,10.0,Color32::from_white_alpha(8));
            app_icon_badge(c,52.0,top+ROW_H/2.0,20.0,app,icons[slot].as_ref());
            c.text(68.0,top+14.0,&c.fit(app,11.5,W::Semibold,230.0),11.5,W::Semibold,theme::WHITE,H::Left);
            c.text(352.0,top+14.0,time,9.5,W::Regular,theme::GRAY,H::Right);
            c.text(68.0,top+27.0,&c.fit(body,9.5,W::Regular,280.0),9.5,W::Regular,theme::GRAY,H::Left);
        }
        if max>0 {
            let track_x=367.0;
            let viewport=crate::app::WINDOWS_ROWS as f32*ROW_H+(crate::app::WINDOWS_ROWS-1) as f32*ROW_GAP;
            let thumb_h=((crate::app::WINDOWS_ROWS as f32/count as f32)*viewport).max(18.0);
            let thumb_y=LIST_TOP+first as f32/max as f32*(viewport-thumb_h);
            c.fill(track_x,LIST_TOP,3.0,viewport,1.5,Color32::from_white_alpha(14));
            c.fill(track_x,thumb_y,3.0,thumb_h,1.5,Color32::from_white_alpha(64));
        }
    }
    pub(crate) fn tick_alerts(&mut self) {
        self.pending_messengers.retain(|u|u.source.enabled(&self.settings)
            && (u.source!=crate::app::Messenger::Qq || self.settings.qq_notifications)
            && (u.source.adapter().capture!=crate::messenger::Capture::SystemToast
            || self.notification_apps.items.iter().any(|(key,_,on)|key==u.source.adapter().notification_id && *on)));
        let allowed=|a:&Alert|match &a.source {
            AlertSource::Internal=>!self.fullscreen_hidden || a.health().is_none(),
            AlertSource::Messenger(client)=>client.enabled(&self.settings),
            AlertSource::External(id)=>self.settings.read_notifications && self.settings.notifications
                && (crate::messenger::Messenger::from_notification_id(id)!=Some(crate::app::Messenger::Qq) || self.settings.qq_notifications)
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
            crate::paint::bell(&nc,322.0,22.0,theme::GRAY,crate::paint::bell_swing(self.t-a.at));
            nc.circle(326.0,17.5,2.0,Color32::from_rgb(255,72,72));
            nc.centered_text(16.0,14.0,&nc.fit(&a.title,11.0,W::Semibold,198.0),11.0,W::Semibold,theme::WHITE,H::Left);
            nc.centered_text(16.0,30.0,&nc.fit(&a.body,10.5,W::Regular,198.0),10.5,W::Regular,theme::GRAY,H::Left);
            if notice_button(&nc,220.0,12.0,44.0,"查看","external-view"){
                self.show_page(crate::app::WINDOWS_PAGE);
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
            let (dismiss,dc)=crate::paint::capsule(&nc,188.0,98.0,72.0,24.0,"meeting-dismiss",Color32::from_gray(32),Color32::from_gray(48));
            dc.centered_text(224.0,110.0,"关闭",10.5,W::Semibold,theme::WHITE,H::Center);
            if dismiss.on_hover_text("关闭本次提醒，日程仍保留").clicked(){
                self.notice=None;
                return;
            }
            let (button,bc)=crate::paint::capsule(&nc,272.0,98.0,72.0,24.0,"meeting-action",Color32::from_gray(42),Color32::from_gray(56));
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
                let kind=a.bluetooth.map(|(kind,_)|kind).unwrap_or(crate::bt::DeviceKind::Other);
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
        self.volume_a = crate::anim::approach(self.volume_a, target, if pressed { 34.0 } else { 13.0 }, self.dt);
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

    pub(crate) fn drink_water(&mut self) {        let hour = self.current_hour;
        let amount = self.settings.serving_ml;
        let was = self.water.percent();
        let before = self.water.drunk_ml;
        self.water.drink(amount, hour);
        let amount = self.water.drunk_ml.saturating_sub(before);
        self.last_drink = Some((amount, hour));
        self.confirm_health_reminder(HealthReminder::Water);
        self.pet.react(PetState::Satisfied);
        self.say(format!("已记录 {amount} ml"));
        if was < 100.0 && self.water.percent() >= 100.0 {
            self.alert("今日饮水目标达成", "做得不错，继续保持", 2, true);
        }
    }


}

/// 将本地提醒时间规范化，拒绝不存在的日期与超出范围的时分。
pub(crate) fn parse_due(value: &str, today: &str) -> Option<String> {
    let value = value.trim();
    let full = if value.len() == 5 {
        format!("{today} {value}")
    } else {
        value.to_string()
    };
    if !full.is_ascii() || full.len() != 16 {
        return None;
    }
    let b = full.as_bytes();
    if b[4] != b'-' || b[7] != b'-' || b[10] != b' ' || b[13] != b':' {
        return None;
    }
    let year = full[0..4].parse::<u32>().ok()?;
    let month = full[5..7].parse::<usize>().ok()?;
    let day = full[8..10].parse::<u32>().ok()?;
    let hour = full[11..13].parse::<u32>().ok()?;
    let minute = full[14..16].parse::<u32>().ok()?;
    if !(2000..=9999).contains(&year) || !(1..=12).contains(&month) || hour > 23 || minute > 59 {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day == 0 || day > days[month - 1] {
        return None;
    }
    Some(full)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn island_size_drag_commits_only_on_release(){
        let ctx=egui::Context::default();let mut value=100;
        let pos=egui::pos2(30.0,30.0);let end=egui::pos2(110.0,30.0);
        let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
        let frames=[vec![],vec![egui::Event::PointerMoved(pos)],vec![button(pos,true)],vec![egui::Event::PointerMoved(end)],vec![button(end,false)]];
        for (i,events) in frames.into_iter().enumerate(){
            let _=ctx.run(egui::RawInput{events,screen_rect:Some(Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(320.0,80.0))),..Default::default()},|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(20)).show(ctx,|ui|{ui.spacing_mut().slider_width=180.0;island_size_slider(ui,&mut value);});
            });
            if i<4{assert_eq!(value,100);}else{assert_ne!(value,100);}
        }
    }
    #[test]
    fn app_icon_falls_back_to_the_name_initial(){
        assert_eq!(app_initial("Telegram"),"T");
        assert_eq!(app_initial("应用0"),"应");
        assert_eq!(app_initial(""),"");
    }
    #[test]
    fn number_stepper_clicks_follow_steps_and_limits() {
        for (initial,step,x,expected) in [(5u32,1,127.0,6),(5,1,13.0,4),(0,1,13.0,0),(10,1,127.0,10),(9,3,127.0,10),(1,3,13.0,0)] {
            let ctx=egui::Context::default();
            let mut value=initial;
            let pos=egui::pos2(x+20.0,34.0);
            let events=[vec![],vec![egui::Event::PointerMoved(pos)],vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}],vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]];
            for events in events {
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(200.0,80.0))),events,..Default::default()};
                let _=ctx.run(raw,|ctx|{
                    egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(20)).show(ctx,|ui|{
                        ui.horizontal(|ui|{number_stepper(ui,&mut value,0,10,"",step,76.0);});
                    });
                });
            }
            assert_eq!(value,expected,"初值 {initial}，步长 {step}，点击位置 {x}");
        }
    }

    #[test]
    fn number_stepper_fits_with_settings_button_padding() {
        let ctx=egui::Context::default();
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(340.0,240.0))),..Default::default()};
        let _=ctx.run(raw,|ctx|{
            egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{
                ui.spacing_mut().button_padding=egui::vec2(10.0,6.0);
                for mut value in [0u32,25,180] {
                    let row=ui.horizontal(|ui|{number_stepper(ui,&mut value,0,180," 分钟",1,76.0);});
                    assert!(row.response.rect.width()<=140.1,"数值控件超出预留宽度：{}",row.response.rect.width());
                }
                for mut value in [25u32,5,15] {setting_number(ui,"计时时长",&mut value,1,180," 分钟",1.0);}
                assert!(ui.min_rect().right()<=340.1,"连续数值行不应撑宽卡片");
            });
        });
    }

    #[test]
    fn settings_groups_keep_every_page_reachable() {
        use crate::extras::settings_tab_group_for_tests as group;
        let names:Vec<_>=SETTINGS_GROUPS.iter().map(|group|group.0).collect();
        assert_eq!(names,["通用","岛体","行为","提醒","音乐","专注","健康","待办","置物架","剪贴板","邮箱","日程","AI 额度","AI 会话","天气","Windows 通知","关于"]);
        let sections:Vec<_>=SETTINGS_SECTIONS.iter().map(|section|section.0).collect();
        assert_eq!(sections,["应用","功能","其他"]);
        // 分组必须连续覆盖所有侧栏项，侧栏才能按切片直接渲染。
        let mut covered=0;
        for (_,from,to) in SETTINGS_SECTIONS {assert_eq!(from,covered);covered=to;}
        assert_eq!(covered,SETTINGS_GROUPS.len());
        assert_eq!(group(0),"通用");
        assert_eq!(group(2),"专注");assert_eq!(group(3),"健康");
        assert_eq!(group(4),"待办");assert_eq!(group(5),"置物架");
        assert_eq!(group(6),"提醒");assert_eq!(group(7),"邮箱");assert_eq!(group(8),"日程");
        assert_eq!(group(9),"关于");assert_eq!(group(15),"AI 会话");
        assert_eq!(group(16),"岛体");assert_eq!(group(SETTINGS_TAB_CLIPBOARD),"剪贴板");
        assert_eq!(group(SETTINGS_TAB_WINDOWS),"Windows 通知");
        assert_eq!(group(SETTINGS_TAB_APPEARANCE),"岛体","外观并入岛体");
        assert_eq!(group(SETTINGS_TAB_PAGES),"岛体","功能页面并入岛体");
        assert_eq!(group(SETTINGS_TAB_BEHAVIOR),"行为");
        let body=names.iter().position(|name|*name=="岛体").unwrap();
        assert_eq!((names[body-1],names[body+1]),("通用","行为"));
        let reminders=names.iter().position(|name|*name=="提醒").unwrap();
        assert_eq!((names[reminders-1],names[reminders+1]),("行为","音乐"));
        let feature:Vec<_>=settings_feature_rows([0,1,2,3,11,4,5,6,7,8,9,10]).into_iter().map(|row|row.0).collect();
        assert_eq!(feature,["音乐","专注","健康","置物架","剪贴板","待办","邮箱","AI 额度","日程","天气","AI 会话","Windows 通知"]);
        let reordered:Vec<_>=settings_feature_rows([1,0,2,3,11,4,5,6,7,8,9,10]).into_iter().map(|row|row.0).collect();
        assert_eq!(&reordered[..2],["专注","音乐"],"功能组应跟随岛上的页面顺序");
    }

    #[test]
    fn settings_body_icon_renders() {
        let image=egui_extras::image::load_svg_bytes(include_bytes!("../assets/settings-icons/system/body.svg")).unwrap();
        assert!(image.pixels.iter().any(|pixel|pixel.a()>0));
    }

    #[test]
    fn bluetooth_notice_uses_real_connection_and_battery_data(){
        assert_eq!(bluetooth_notice_content("OpenMove by Shokz · 电量 80%"),("OpenMove by Shokz","已连接",Some(80)));
        assert_eq!(bluetooth_notice_content("耳机"),("耳机","已连接",None));
        assert_eq!(bluetooth_notice_content("已断开连接"),("蓝牙设备","已断开",None));
        assert_eq!(bluetooth_notice_content("青龙键盘 · 已断开"),("青龙键盘","已断开",None));
        assert_eq!(bluetooth_notice_content("耳机 · 电量 101%"),("耳机 · 电量 101%","已连接",None));
    }

    #[test]
    fn device_icons_render_as_visible_svg() {
        let icons: &[&[u8]] = &[
            include_bytes!("../assets/device-icons/headphones-round-bold.svg"),
            include_bytes!("../assets/device-icons/speaker-bold.svg"),
            include_bytes!("../assets/device-icons/keyboard-bold.svg"),
            include_bytes!("../assets/device-icons/mouse-bold.svg"),
            include_bytes!("../assets/device-icons/gamepad-bold.svg"),
            include_bytes!("../assets/device-icons/smartphone-bold.svg"),
            include_bytes!("../assets/device-icons/bluetooth-linear.svg"),
        ];
        for bytes in icons {
            let image=egui_extras::image::load_svg_bytes(bytes).unwrap();
            assert!(image.pixels.iter().any(|p|p.a()>0));
        }
    }

    #[test]
    fn due_time_checks_calendar_and_format() {
        assert_eq!(
            parse_due("09:30", "2026-10-29").as_deref(),
            Some("2026-10-29 09:30")
        );
        assert!(parse_due("2026-02-29 12:00", "").is_none());
        assert!(parse_due("2028-02-29 12:00", "").is_some());
        for value in [
            "24:00",
            "12:60",
            "明天中午",
            "2026-13-01 10:00",
            "2026-00-01 10:00",
        ] {
            assert!(parse_due(value, "2026-10-29").is_none());
        }
    }
}

pub(crate) struct TaskDraft {index:usize,original:String,text:String,due:String,error:String,focused:bool,reminder_only:bool,pub(crate) repeat:u8,pub(crate) picker:crate::task_picker::ReminderPicker}
impl App {
    pub(crate) fn open_task_dialog(&mut self,index:usize){
        self.todo_choosing=false;
        if let Some(task)=self.todos.items.get(index){
            let draft=TaskDraft{index,original:task.text.clone(),text:task.text.clone(),due:task.due.clone().unwrap_or_default(),error:String::new(),focused:false,reminder_only:false,repeat:task.repeat,picker:crate::task_picker::ReminderPicker::new(task.due.as_deref())};
            self.show_page(4);self.task_dialog=Some(draft);
        }
    }
    pub(crate) fn open_task_reminder(&mut self,index:usize){
        self.open_task_dialog(index);
        if let Some(d)=self.task_dialog.as_mut(){d.reminder_only=true;d.picker.show();}
    }
    pub(crate) fn draw_task_editor(&mut self,c:&Card){
        if self.task_dialog.as_ref().is_some_and(|draft|draft.picker.open){self.draw_task_reminder(c);return;}
        let reminder_only=self.task_dialog.as_ref().is_some_and(|d|d.reminder_only);
        c.text(15.0,25.0,if reminder_only{"设置提醒"}else{"编辑待办"},size::TITLE,W::Bold,theme::WHITE,H::Left);
        c.text(43.0,48.0,if reminder_only{"待办"}else{"内容"},10.5,W::Regular,theme::GRAY,H::Left);
        c.fill(91.0,33.0,275.0,20.0,6.0,theme::CARD);
        if let Some(d)=self.task_dialog.as_ref().filter(|d|d.reminder_only){
            c.text(97.0,47.0,&c.fit(&d.text,11.5,W::Regular,260.0),11.5,W::Regular,theme::WHITE,H::Left);
        }
        c.text(43.0,72.0,"提醒",10.5,W::Regular,theme::GRAY,H::Left);
        let time=self.task_dialog.as_ref().map(|d|if d.due.is_empty(){"点击选择时间".into()}
            else if d.repeat!=0{format!("{} {}",crate::model::repeat_label(d.repeat),d.due.get(11..16).unwrap_or(&d.due))}else{d.due.clone()}).unwrap_or_default();
        if setting_button(c,91.0,58.0,275.0,&time,false,"task-select-reminder"){if let Some(d)=self.task_dialog.as_mut(){d.picker.show();}}
        // 每周重复：七个星期按钮，全不选即一次性待办。
        c.text(43.0,95.0,"重复",10.5,W::Regular,theme::GRAY,H::Left);
        self.task_repeat_chips(c,91.0,81.0,25.0,19.0,28.0);
        let repeat=self.task_dialog.as_ref().map(|d|d.repeat).unwrap_or(0);
        let quick=if repeat==crate::model::REPEAT_ALL{"不重复"}else{"每天"};
        if setting_button(c,289.0,81.0,77.0,quick,false,"task-repeat-all"){
            if let Some(d)=self.task_dialog.as_mut(){d.repeat=if d.repeat==crate::model::REPEAT_ALL{0}else{crate::model::REPEAT_ALL};}
        }
        if let Some(d)=&self.task_dialog {
            let summary=if d.repeat!=0{crate::model::repeat_label(d.repeat)}else{"只做一次".into()};
            let hint=if d.error.is_empty(){summary.as_str()}else{&d.error};
            c.text(43.0,117.0,&c.fit(hint,10.5,W::Regular,if reminder_only{140.0}else{154.0}),10.5,if d.error.is_empty(){W::Regular}else{W::Semibold},if d.error.is_empty(){theme::GRAY}else{theme::RED},H::Left);
        }
        if reminder_only {
            let has_reminder=self.task_dialog.as_ref().and_then(|d|self.todos.items.get(d.index)).is_some_and(|task|task.due.is_some());
            let skip=if has_reminder{"取消提醒"}else if repeat!=0{"只设重复"}else{"暂不提醒"};
            if setting_button(c,188.0,104.0,88.0,skip,false,"task-reminder-skip"){
                if let Some(d)=self.task_dialog.as_mut(){d.due.clear();}
                self.save_task_editor();
            }
        }else if setting_button(c,202.0,104.0,74.0,"取消",false,"task-cancel"){self.task_dialog=None;}
        if setting_button(c,286.0,104.0,80.0,if reminder_only{"选择时间"}else{"保存"},true,"task-save"){
            if reminder_only{if let Some(d)=self.task_dialog.as_mut(){d.picker.show();}}else{self.save_task_editor();}
        }
    }
    pub(crate) fn task_editor_inputs(&mut self,ui:&mut egui::Ui,origin:egui::Pos2,cmds:&mut Vec<Cmd>){
        let Some(d)=self.task_dialog.as_mut() else{return};
        if d.reminder_only || d.picker.open{return;}
        let name=ui.put(Rect::from_min_size(origin+egui::vec2(97.0,35.0),egui::vec2(260.0,17.0)),egui::TextEdit::singleline(&mut d.text).vertical_align(egui::Align::Center).id(egui::Id::new("task-name-edit")).frame(false).font(theme::font(11.5,W::Regular)));
        let first_focus=!d.focused;
        if first_focus{name.request_focus();d.focused=true;cmds.push(Cmd::Focus);}
        // 新建内容与下一步共用这一帧，避免同一次回车又提交了空提醒。
        if !first_focus && ui.input(|i|i.key_pressed(egui::Key::Enter)){self.save_task_editor();}
    }
    /// 七个星期按钮（一…日），点一下切换；全不选即一次性待办。
    fn task_repeat_chips(&mut self,c:&Card,x:f32,y:f32,w:f32,h:f32,step:f32){
        let repeat=self.task_dialog.as_ref().map(|d|d.repeat).unwrap_or(0);
        for (day,name) in crate::model::WEEKDAY_NAMES.iter().enumerate() {
            let on=repeat&(1<<day)!=0;
            let cx=x+day as f32*step;
            let (r,b)=crate::paint::capsule(c,cx,y,w,h,&format!("task-repeat-{day}"),if on{Color32::from_rgb(39,94,150)}else{theme::BTN},theme::BTN_HOVER);
            b.centered_text(cx+w/2.0,y+h/2.0,name,if h<18.0{9.5}else{10.5},W::Semibold,if on{theme::WHITE}else{theme::GRAY},H::Center);
            if r.on_hover_text(format!("每周{name}")).clicked(){
                if let Some(d)=self.task_dialog.as_mut(){d.repeat^=1<<day;d.error.clear();}
            }
        }
    }
    fn draw_task_reminder(&mut self,c:&Card){
        use crate::task_picker::PickerAction;
        let Some(d)=self.task_dialog.as_mut().filter(|d|d.picker.open) else{return};
        let has_reminder=self.todos.items.get(d.index).is_some_and(|task|task.due.is_some());
        c.text(15.0,25.0,"待办",size::TITLE,W::Bold,theme::WHITE,H::Left);
        c.text(44.0,25.0,"清单",size::TITLE,W::Bold,theme::GREEN,H::Left);
        let (done,total)=self.todos.progress(&self.today);
        c.text(42.0,61.0,&format!("{done}/{total}"),29.0,W::Bold,theme::WHITE,H::Left);
        // 左侧：每周重复。选了星期几后，提醒只按时间、在这些日子到点提醒。
        d.picker.repeat_note=Some(if d.repeat!=0{format!("{} · 到点提醒",crate::model::repeat_label(d.repeat))}else{String::new()});
        let action=d.picker.draw(c,&d.text,has_reminder);
        let focus=std::mem::take(&mut d.picker.focus);
        self.task_repeat_chips(c,41.0,84.0,17.0,16.0,19.5);
        let Some(d)=self.task_dialog.as_mut() else{return};
        if focus{self.pending.push(Cmd::Focus);}
        match action{
            Some(PickerAction::Save)=>{d.due=d.picker.due();d.picker.open=false;if d.reminder_only{self.save_task_editor();}},
            Some(PickerAction::Skip)=>{d.due.clear();d.picker.open=false;if d.reminder_only{self.save_task_editor();}},
            Some(PickerAction::Cancel)=>{d.picker.open=false;if d.reminder_only{self.task_dialog=None;}},
            None=>{},
        }
    }
    fn save_task_editor(&mut self){
        let Some(mut d)=self.task_dialog.take() else{return};
        let due=if d.due.trim().is_empty(){Some(None)}else{parse_due(&d.due,&self.today).map(Some)};
        if d.text.trim().is_empty(){d.error="请填写任务内容".into();}
        else if due.is_none(){d.error="时间格式应为 18:30 或完整日期时间".into();}
        else if self.todos.items.get(d.index).is_none_or(|t|t.text!=d.original){d.error="任务已变化，请取消后重新打开".into();}
        else{
            let item=&mut self.todos.items[d.index];
            if item.repeat!=d.repeat{item.done_on=None;}
            item.text=d.text.trim().into();item.due=due.unwrap();item.reminded=false;item.reminded_on=None;item.repeat=d.repeat;
            self.todos.save();self.say("待办已保存");return;
        }
        self.task_dialog=Some(d);
    }
}

fn notice_button(c:&Card,x:f32,y:f32,w:f32,text:&str,id:&str)->bool {
    let (r,b)=crate::paint::capsule(c,x,y,w,20.0,id,Color32::from_gray(35),theme::BTN_HOVER);
    b.centered_text(x+w/2.0,y+10.0,text,10.5,W::Semibold,theme::WHITE,H::Center);
    r.clicked()
}
fn setting_button(c:&Card,x:f32,y:f32,w:f32,text:&str,on:bool,id:&str)->bool {
    let (r,b)=crate::paint::capsule(c,x,y,w,19.0,id,if on{Color32::from_rgb(39,94,150)}else{theme::BTN},theme::BTN_HOVER);
    b.text(x+w/2.0,y+13.3,text,10.5,W::Semibold,theme::WHITE,H::Center);
    r.clicked()
}
fn hhmm(m:u32)->String{format!("{:02}:{:02}",m/60%24,m%60)}
/// 岛内小开关（与健康页右上开关同款）。
fn island_switch(c:&Card,x:f32,y:f32,on:bool,id:&str)->bool{
    let r=c.hit(x-2.0,y-2.0,38.0,24.0,id);
    let (hv,_)=c.hover_t(&r);
    let track=if on{theme::SIT_GREEN}else{Color32::from_rgb(58,58,62)};
    c.fill(x,y,34.0,20.0,10.0,crate::paint::mix(track,theme::WHITE,hv*0.08));
    c.circle(if on{x+24.0}else{x+10.0},y+10.0,8.0,theme::WHITE);
    r.clicked()
}
impl App {
    fn health_time_summary(&self)->String{
        let s=&self.settings;
        let hours=format!("{:02}:00–{:02}:00",s.water_start,s.water_end);
        if s.lunch_rest{format!("{hours} · 午休 {}–{}",hhmm(s.lunch_start),hhmm(s.lunch_end))}else{hours}
    }
    /// 「提醒时段」二级页：提醒时段起止、午休开关、午休起止（15 分钟一档），喝水和久坐共用。
    fn health_time_page(&mut self,c:&Card){
        let row_y=|i:usize|38.0+i as f32*22.0;
        for i in 1..4{c.fill(44.0,row_y(i)-2.0,326.0,1.0,0.5,Color32::from_rgb(28,28,31));}
        let y=row_y(0);
        c.text(44.0,y+13.0,"提醒时段",10.5,W::Regular,theme::GRAY,H::Left);
        c.text(250.0,y+13.0,"至",10.5,W::Regular,theme::GRAY,H::Center);
        for i in 0..2{
            let hour=if i==0{&mut self.settings.water_start}else{&mut self.settings.water_end};
            let value=egui::DragValue::new(hour).range(0..=if i==0{23}else{24})
                .custom_formatter(|v,_|format!("{:02}:00",v as u32))
                .custom_parser(|t|{let t=t.trim();t.strip_suffix(":00").or_else(||t.strip_suffix("：00")).unwrap_or(t).parse::<u32>().ok().map(|v|v as f64)});
            let (d,field)=stepper(c,if i==0{238.0}else{370.0},y,value,&format!("hour-{i}"));
            if field.gained_focus(){self.pending.push(Cmd::Focus);}self.settings_editing|=field.has_focus();
            if d!=0{let hour=if i==0{&mut self.settings.water_start}else{&mut self.settings.water_end};*hour=(*hour as i32+d).rem_euclid(24) as u32;}
        }
        let y=row_y(1);
        c.text(44.0,y+13.0,"午休不提醒",10.5,W::Regular,theme::GRAY,H::Left);
        if island_switch(c,336.0,y-0.5,self.settings.lunch_rest,"lunch-rest"){self.settings.lunch_rest=!self.settings.lunch_rest;}
        let y=row_y(2);
        let on=self.settings.lunch_rest;
        let lc=c.faded(if on{1.0}else{0.35});let lc=if on{lc}else{lc.ghost()};
        lc.text(44.0,y+13.0,"午休时间",10.5,W::Regular,theme::GRAY,H::Left);
        lc.text(250.0,y+13.0,"至",10.5,W::Regular,theme::GRAY,H::Center);
        for i in 0..2{
            let minute=if i==0{&mut self.settings.lunch_start}else{&mut self.settings.lunch_end};
            let value=egui::DragValue::new(minute).range(0..=1439).speed(1.0).custom_formatter(|v,_|hhmm(v as u32))
                .custom_parser(|t|{let (h,m)=t.trim().split_once([':','：'])?;Some((h.parse::<u32>().ok()?.min(23)*60+m.parse::<u32>().ok()?.min(59)) as f64)});
            let (d,field)=stepper(&lc,if i==0{238.0}else{370.0},y,value,&format!("lunch-{i}"));
            if field.gained_focus(){self.pending.push(Cmd::Focus);}self.settings_editing|=field.has_focus();
            if d!=0{let minute=if i==0{&mut self.settings.lunch_start}else{&mut self.settings.lunch_end};*minute=(*minute as i32+d*15).rem_euclid(1440) as u32;}
        }
        c.text(44.0,row_y(3)+13.0,"喝水和久坐提醒都只在提醒时段内、午休以外弹出",10.5,W::Regular,theme::GRAY,H::Left);
    }
}
fn island_number_field(c:&Card,rect:Rect,value:egui::DragValue<'_>,id:impl std::hash::Hash)->egui::Response {
    let mut ui=egui::Ui::new(c.ui.ctx().clone(),egui::Id::new(id),egui::UiBuilder::new().layer_id(c.ui.layer_id()).max_rect(rect).style(c.ui.style().clone()));
    ui.set_clip_rect(c.painter.clip_rect());ui.set_opacity(c.painter.opacity());if !c.live{ui.disable();}
    ui.spacing_mut().interact_size=rect.size();ui.spacing_mut().button_padding=egui::vec2(2.0,1.0);
    ui.style_mut().drag_value_text_style=egui::TextStyle::Body;
    ui.style_mut().text_styles.insert(egui::TextStyle::Body,c.font(11.0,W::Bold));
    ui.style_mut().explanation_tooltips=false;
    ui.visuals_mut().widgets.inactive.weak_bg_fill=Color32::TRANSPARENT;
    crate::quota::ui::bar_tooltip(ui.put(rect,value.update_while_editing(false)),"点击输入数值".into())
}
/// 加减器：[−] 可输入数值 [+]，右缘对齐到 right。
fn stepper(c:&Card,right:f32,y:f32,value:egui::DragValue<'_>,id:&str)->(i32,egui::Response) {
    let (w,vw)=(25.0,58.0);
    let x=right-w*2.0-vw;
    let field=island_number_field(c,c.rect(x+w,y,vw,19.0),value,("health-number",id));
    let delta=if setting_button(c,x,y,w,"−",false,&format!("{id}-minus")){-1}
        else if setting_button(c,right-w,y,w,"+",false,&format!("{id}-plus")){1}else{0};
    (delta,field)
}
impl App {
    pub(crate) fn page_settings(&mut self,c:&Card) {
        let help=match self.settings_category{0=>"外观与系统",1=>"调整喝水、久坐与提醒时段",2=>"点击分钟数可直接输入，也可用加减微调",_=>"通知"};
        if self.settings_category!=1{c.text(44.0,44.0,help,10.5,W::Regular,theme::GRAY,H::Left);}
        let before=format!("{:?}",self.settings);
        self.settings_editing=false;
        match self.settings_category {
            0=>{
                let system_hud=self.settings.hud_volume||self.settings.hud_power||self.settings.hud_network||self.settings.hud_keyboard;
                let choices=[("吉祥物",self.settings.pet),("系统状态",system_hud),("全屏隐藏",self.settings.hide_fullscreen)];
                for (i,(name,on)) in choices.iter().enumerate(){let x=42.0+(i%2) as f32*169.0;let y=57.0+(i/2) as f32*27.0;let text=format!("{}  {}",name,if *on{"开"}else{"关"});if setting_button(c,x,y,158.0,&text,*on,&format!("general-{i}")){match i{0=>self.set_pet(!self.settings.pet),1=>{self.settings.hud_volume=!system_hud;self.settings.hud_power=!system_hud;self.settings.hud_network=!system_hud;self.settings.hud_keyboard=!system_hud;},_=>self.settings.hide_fullscreen=!self.settings.hide_fullscreen}}}
                c.text(42.0,116.0,"岛仔收起时常驻",10.5,W::Regular,theme::GRAY,H::Left);
            }
            1 if self.health_time=>self.health_time_page(c),
            1=>{
                // 与健康页同一套切换（标题旁的水滴 / 椅子）；四行一列，标签靠左、加减器靠右对齐。
                // 喝水三行、久坐一行加两句说明；最后一行「提醒时段」两边共用，位置不变。
                let sv=self.sit_view_t.pos.clamp(0.0,1.0);
                let row_y=|i:usize|38.0+i as f32*22.0;
                for i in 1..4{c.fill(44.0,row_y(i)-2.0,326.0,1.0,0.5,Color32::from_rgb(28,28,31));}
                if sv<0.99{
                    let wc=c.faded(1.0-sv);let wc=if self.sit_mode{wc.ghost()}else{wc};
                    for (i,label) in ["每日目标","每次记录","提醒间隔"].iter().enumerate(){
                        wc.text(44.0,row_y(i)+13.0,label,10.5,W::Regular,theme::GRAY,H::Left);
                        let goal=self.water.goal_ml;let mut liters=goal as f32/1000.0;
                        let value=match i {
                            0=>egui::DragValue::new(&mut liters).range(0.5..=6.0).speed(0.25).suffix(" L").min_decimals(1).max_decimals(2),
                            1=>egui::DragValue::new(&mut self.settings.serving_ml).range(50..=1000).speed(50.0).suffix(" ml"),
                            _=>egui::DragValue::new(&mut self.settings.water_minutes).range(5..=180).speed(5.0).suffix(" 分钟"),
                        };
                        let (d,field)=stepper(&wc,370.0,row_y(i),value,&format!("water-{i}"));
                        if field.gained_focus(){self.pending.push(Cmd::Focus);}self.settings_editing|=field.has_focus();
                        match i {
                            0=>{self.water.goal_ml=((liters*1000.0).round() as i32+d*250).clamp(500,6000) as u32;if self.water.goal_ml!=goal{self.water.save();}}
                            1=>self.settings.serving_ml=(self.settings.serving_ml as i32+d*50).clamp(50,1000) as u32,
                            _=>self.settings.water_minutes=(self.settings.water_minutes as i32+d*5).clamp(5,180) as u32,
                        }
                    }
                }
                if sv>0.01{
                    let sc=c.faded(sv);let sc=if self.sit_mode{sc}else{sc.ghost()};
                    sc.text(44.0,row_y(0)+13.0,"坐满多久提醒",10.5,W::Regular,theme::GRAY,H::Left);
                    let value=egui::DragValue::new(&mut self.settings.sit_minutes).range(15..=180).speed(5.0).suffix(" 分钟");
                    let (d,field)=stepper(&sc,370.0,row_y(0),value,"sit");
                    if field.gained_focus(){self.pending.push(Cmd::Focus);}self.settings_editing|=field.has_focus();
                    if d!=0{self.settings.sit_minutes=(self.settings.sit_minutes as i32+d*5).clamp(15,180) as u32;}
                    sc.text(44.0,row_y(1)+13.0,"专注计时进行中不打扰，专注结束后再提醒",10.5,W::Regular,theme::GRAY,H::Left);
                    sc.text(44.0,row_y(2)+13.0,"提醒后常驻，点击确认或起身后重新计时",10.5,W::Regular,theme::GRAY,H::Left);
                }
                // 最后一行两边共用：提醒时段 + 午休的摘要，点进二级页修改。
                let y=row_y(3);
                let r=c.hit(40.0,y-2.0,334.0,22.0,"health-time").on_hover_text("设置提醒时段和午休免打扰");
                let (hv,_)=c.hover_t(&r);
                if hv>0.0{c.fill(40.0,y-2.0,334.0,22.0,8.0,Color32::from_white_alpha((hv*14.0) as u8));}
                c.text(44.0,y+13.0,"提醒时段",10.5,W::Regular,theme::GRAY,H::Left);
                c.text(356.0,y+13.0,&self.health_time_summary(),10.5,W::Semibold,theme::WHITE,H::Right);
                c.text(366.0,y+13.5,"›",14.0,W::Bold,theme::GRAY,H::Center);
                if r.clicked(){self.health_time=true;}
            }
            2=>{
                for (i,name) in ["专注","休息"].iter().enumerate(){
                    let y=53.0+i as f32*23.0;
                    c.text(48.0,y+13.0,name,10.5,W::Regular,theme::GRAY,H::Left);
                    let value=if i==0{&mut self.settings.focus_minutes}else{&mut self.settings.rest_minutes};
                    let max=if i==0{180}else{60};let rect=c.rect(168.0,y,76.0,19.0);
                    let field=island_number_field(c,rect,egui::DragValue::new(value).range(1..=max).suffix(" 分钟"),("focus-duration",i));
                    if field.gained_focus(){self.pending.push(Cmd::Focus);}
                    self.settings_editing|=field.has_focus();
                    if setting_button(c,131.0,y,27.0,"−",false,&format!("focus-minus-{i}")){*value=value.saturating_sub(1).max(1);}
                    if setting_button(c,255.0,y,27.0,"+",false,&format!("focus-plus-{i}")){*value=(*value+1).min(max);}
                }
                c.text(315.0,93.0,"下次生效",10.5,W::Regular,theme::GRAY,H::Center);
            }
            _=>{
                if setting_button(c,43.0,56.0,152.0,if self.settings.notifications{"提醒 · 开"}else{"提醒 · 关"},self.settings.notifications,"set-notice"){self.settings.notifications=!self.settings.notifications;}
                if setting_button(c,212.0,56.0,153.0,if self.settings.system_notify && self.settings.sound{"声音 · 开"}else{"声音 · 关"},self.settings.system_notify && self.settings.sound,"set-sound"){
                    self.settings.sound=!(self.settings.system_notify && self.settings.sound);
                    if self.settings.sound {self.settings.system_notify=true;}
                }
                if setting_button(c,43.0,84.0,152.0,if self.settings.read_notifications{"断开系统通知"}else{"连接系统通知"},self.settings.read_notifications,"set-listener"){self.pending.push(if self.settings.read_notifications{Cmd::DisconnectNotifications}else{Cmd::RequestNotifications});}
                let status=c.fit(&self.listener_status,10.5,W::Regular,170.0);c.text(210.0,97.0,&status,10.5,W::Regular,theme::GRAY,H::Left);
            }
        }
        if before!=format!("{:?}",self.settings){self.settings.save();}
    }
}

