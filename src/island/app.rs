//! 岛体应用状态：外部环境、命令、功能状态与生命周期。

use super::*;
use crate::calendar::CalendarState;
use crate::focus::Pomo;
use crate::mail::MailState;
use crate::music::LyricsState;

pub struct Options {
    /// 是否联网查歌词（录制/测试时关掉）。
    pub fetch_lyrics: bool,
    /// 吉祥物在多久无操作后出现（秒）。
    pub pet_idle: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self { fetch_lyrics: true, pet_idle: 45.0 }
    }
}
/// 每帧由外壳提供的外部世界。
#[derive(Clone, Default)]
pub struct Env {
    pub t: f64,
    /// `HH:MM:SS`
    pub clock: String,
    /// `YYYY-MM-DD`
    pub today: String,
    /// 顶栏日期，例如 `10月28日 周三`
    pub date: String,
    pub battery: Option<u8>,
    pub charging: bool,
    /// 互联网连接状态；未知时不触发表情提醒。
    pub network: Option<bool>,
    /// 光标位置（与 `frame` 传入的 `win` 同一坐标系）；不在窗口附近时为 None。
    pub cursor: Option<Pos2>,
    pub idle_seconds: Option<f64>,
    /// 当前会话是否处于锁屏（Win + L、自动锁屏、唤醒后的登录界面）。
    pub screen_locked: bool,
    /// 因「全屏时隐藏」而隐藏岛体，沿用外壳对设置和所在屏幕的判断。
    pub fullscreen_hidden: bool,
    /// 名单应用最大化后是否盖住岛；岛仔自身窗口或暂不可读时为 None。
    pub browser_at_top: Option<bool>,
    pub media: NowPlaying,
    /// 系统主音量（0..100, 是否静音）；v0.5.4 音量条与系统双向同步。
    pub volume: Option<(u32, bool)>,
    /// 已连接的蓝牙音频设备（名称 + 可选电量）。
    pub bt: Vec<crate::os::bt::Device>,
    /// 蓝牙服务是否已经完成首次扫描。
    pub bt_ready: bool,
    /// 岛所在屏幕（与 `win` 同一坐标系，逻辑点）；用于让通知详情留在屏幕内。
    pub screen: Option<Rect>,
    /// 毛玻璃皮肤：岛体四周采样得到的背景色（sRGB 0..1）。
    pub backdrop: Option<[f32; 3]>,
    /// 毛玻璃皮肤：系统背景模糊当前是否生效。
    pub glass_blur: bool,
    /// 不支持逐像素透明的兼容后端需要按内容轮廓裁剪窗口。
    pub opaque_surface: bool,
}

impl Env {
    pub fn hour(&self) -> usize {
        self.clock.get(0..2).and_then(|s| s.parse().ok()).unwrap_or(12)
    }
    pub fn minute(&self) -> u32 {
        self.clock.get(3..5).and_then(|s| s.parse().ok()).unwrap_or(0)
    }
}
/// 交给外壳执行的副作用。
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    Media(Transport),
    Open(PathBuf),
    ViewMessenger(Unread),
    OpenUrl(String),
    /// 需要键盘输入：外壳应让窗口可激活并取得焦点。
    Focus,
    Reveal(PathBuf),
    Copy(String),
    Notify(String, String, NotificationSound),
    /// 只播放岛内铃声，不发 Windows Toast（新邮件）。
    PlayNotificationSound(NotificationSound),
    RequestNotifications,
    DisconnectNotifications,
    ToggleAutostart,
    AddFiles,
    DragFile(PathBuf),
    QuotaRefresh,
    QuotaConnected,
    QuotaChooseFile,
    /// 把系统主音量设为 0..100（v0.5.4 音量条）。
    SetVolume(u32),
    /// 切换系统静音。
    ToggleMute,
    /// 音量命令携带点击时的播放器身份，避免切换歌曲来源后误调其他应用。
    MusicVolume(Transport,String),
    /// v0.5.5：账号清单变了，让外壳重新同步收信线程。
    MailSync,
    /// 测试某个邮箱的连接（授权码只在内存里传给工作线程）。
    MailTest(mail::Account, String),
    MailManage(crate::mail::manage::Request),
    /// v0.5.6：用临时密码试连日历，成功后才允许落盘。
    CalendarTest(calendar::Config, String),
    /// 让外壳立即同步一次日历。
    CalendarSync,
    CalendarMonth(chrono::NaiveDate),
    /// 断开并删除日历配置（凭据由界面删除）。
    CalendarForget,
    /// 选择 .ics 文件导入本机日程。
    CalendarImport,
    /// 重启岛仔（切换渲染方式后生效）。
    RestartApp,
}
pub struct FrameOut {
    /// 原生窗口应保留的可见区域（相对 `win` 左上角）。
    pub region: Rect,
    /// 毛玻璃 + 系统模糊时，替代 `region` 的贴合轮廓；其余情况为 None。
    pub glass: Option<Vec<GlassShape>>,
    /// 鼠标命中区（相对 `win` 左上角），区域外点击穿透到下层窗口。
    pub hit: Rect,
    /// 切到较矮页面时，原页边缘临时保留的续滚范围，不参与绘制或点击命中。
    pub page_scroll_area: Option<Rect>,
    /// 极简细条只感应悬停，鼠标点击交给下面的应用。
    pub click_through: bool,
    pub notice_hit: Option<Rect>,
    /// 文件拖入感应区；独立于普通鼠标命中区。
    pub file_drop_zone: Option<Rect>,
    /// 展开状态立即生效，不等待展开动画结束。
    pub expanded: bool,
    pub editing: bool,
    pub animating: bool,
    /// 当前帧正在绘制翻页过渡。
    pub page_transition: bool,
    pub repaint_after: std::time::Duration,
    pub cmds: Vec<Cmd>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Focus,
    Rest,
}

impl Phase {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Phase::Focus => "专注时间",
            Phase::Rest => "休息",
        }
    }
}

pub struct App {
    pub opts: Options,
    pub settings: model::Settings,
    /// 本次运行实际使用兼容渲染（Direct3D 12）；设置里的开关与它不同时提示重启。
    pub render_compat_running: bool,
    pub(crate) todos: crate::todo::TodoList,
    pub(crate) water: crate::health::Water,
    pub(crate) shelf: crate::shelf::Shelf,

    // 形变
    pub(crate) w: Spring,
    pub(crate) h: Spring,
    pub(crate) open: bool,
    /// 展开/收起的起始时刻，用于内容延迟淡入。
    pub(crate) open_at: f64,
    pub(crate) close_at: f64,
    /// 外壳最近一次落定的时刻，用于玻璃和网格淡入。
    pub(crate) shell_settled_at: f64,
    pub(crate) hover_since: Option<f64>,
    pub(crate) leave_since: Option<f64>,
    /// 前台切换导致的强制收起：光标离开前不再自动展开。
    pub hover_lock: bool,
    pub(crate) reveal: f32,
    pub(crate) position_drag: Option<PositionDrag>,
    /// 拖动中和松手落位期间的窗口位置（未取整），落定后回到 `settings.island_offset`。
    pub(crate) drag_glide: Option<anim::Spring>,
    /// 拿起程度 0..1（可略过冲）与速度拉伸量，只影响岛体外形，不影响展开进度。
    pub(crate) drag_lift: anim::Spring,
    pub(crate) drag_stretch: anim::Spring,
    /// 拖动时是否吸在居中；进入吸附的时刻用于触发扫光和「咔嗒」。
    pub(crate) center_locked: bool,
    pub(crate) center_lock_at: f64,
    /// 居中描边可见度 0..1（越靠近居中越清晰）与「已居中」点亮程度 0..1。
    pub(crate) center_guide: f32,
    pub(crate) center_lock_a: f32,
    /// 光标停在日期时间拖动区的起始时间：停留越久把手提示越明显，划过不打扰。
    pub(crate) grip_hover_since: Option<f64>,

    // 切页
    pub(crate) page: usize,
    pub(crate) page_from: usize,
    pub(crate) page_scroll_area: Option<Rect>,
    pub(crate) page_dir: f32,
    pub(crate) page_t0: f64,
    pub(crate) slide: Spring,
    pub(crate) dots: [Spring; 12],
    pub(crate) wheel_delta: f32,
    pub(crate) wheel_at: f64,
    pub(crate) wheel_page_at: f64,

    // 拖入
    pub(crate) drag_hot: bool,
    pub(crate) drag_a: f32,
    pub(crate) drag_saved: Option<usize>,

    // 置物架
    pub(crate) shelf_scroll: Spring,
    pub(crate) shelf_first: usize,
    pub(crate) shelf_menu_rect: Option<Rect>,
    pub(crate) shelf_x: HashMap<PathBuf, Spring>,
    pub(crate) shelf_born: HashMap<PathBuf, f64>,
    pub(crate) icons: crate::os::icons::Cache,
    pub(crate) avoidance: crate::island::avoidance::Picker,
    pub(crate) confirm_clear: Option<f64>,
    pub clipboard: crate::clipboard::Shared,
    pub(crate) clip_first: usize,
    pub(crate) clip_confirm_clear: Option<f64>,

    // 饮水
    pub(crate) water_ring: Spring,
    pub(crate) water_ml: f32,
    pub(crate) water_view: usize,
    pub(crate) water_view_t: Spring,
    pub(crate) sit_view_t: Spring,
    /// 饮水页当前是否显示「站立提醒」。
    pub(crate) sit_mode: bool,
    /// 健康提醒设置里是否打开了「提醒时段」二级页（时段 + 午休）。
    pub(crate) health_time: bool,
    pub(crate) remind_t: Spring,
    pub(crate) last_reminder: f64,
    pub(crate) health_pending: [bool;2],
    pub(crate) fullscreen_hidden: bool,
    /// 久坐计时起点，确认提醒或点「起身了」后重置。
    pub(crate) sit_start: f64,
    pub(crate) sit_t: Spring,

    // 专注
    pub(crate) pomo: Pomo,
    pub(crate) settings_editing: bool,
    pub(crate) ring_shown: Spring,
    pub(crate) reset_spin: Spring,

    // 待办
    pub(crate) adding: bool,
    pub(crate) editing: Option<usize>,
    pub(crate) buf: String,
    pub(crate) todo_input_focused: bool,
    pub(crate) todo_ime_active: bool,
    pub(crate) todo_ime_frame: bool,
    pub(crate) todo_pop: Spring,

    // 音乐
    pub(crate) ly: LyricsState,
    pub(crate) ly_worker: Option<std::thread::JoinHandle<()>>,
    pub(crate) show_lyrics: bool,
    pub(crate) island_lyric_layout: Option<std::sync::Arc<egui::Galley>>,
    pub(crate) media_anchor: (String, f64, f64, bool),
    pub(crate) media: NowPlaying,
    pub(crate) music_artwork: Option<(std::sync::Arc<egui::ColorImage>,egui::TextureHandle)>,

    pub(crate) forced_open_until:f64,
    pub settings_open:bool,
    pub(crate) settings_window:std::sync::Arc<std::sync::Mutex<crate::island::settings_window::State>>,
    pub displays:Vec<crate::os::display_layout::Display>,
    pub(crate) browser_at_top:bool,
    pub(crate) browser_minimal_active:bool,
    /// 毛玻璃着色量（平滑过渡，避免背景变化时闪烁）；0 表示尚未初始化。
    pub(crate) glass_tint:f32,
    pub(crate) fluid:crate::visuals::fluid::Fluid,
    pub(crate) fluid_origin:Pos2,
    pub(crate) fluid_art:Option<egui::TextureId>,
    pub(crate) fluid_title:String,
    pub(crate) screen:Option<Rect>,
    pub(crate) settings_focus_requested:bool,
    pub(crate) settings_tab:usize,
    pub(crate) settings_category:usize,
    /// 功能页面拖动后的草稿顺序；点「保存」才写入设置。
    pub(crate) pending_page_order:Option<[usize;12]>,
    pub messenger_statuses:std::collections::HashMap<Messenger,String>,
    pub(crate) notification_apps:model::NotificationApps,
    pub(crate) subpage:Option<usize>,
    pub(crate) task_dialog:Option<crate::todo::TaskDraft>,
    pub listener_status: String,
    pub(crate) notification_bell_until: f64,
    pub(crate) notification_bell_at: f64,
    pub(crate) notice: Option<Alert>,
    pub(crate) hud_notice: Option<Alert>,
    pub(crate) hud_alpha: f32,
    pub(crate) notice_width: Spring,
    pub(crate) notice_details: bool,
    pub pending_messengers:Vec<Unread>,
    /// Windows 通知页的记录；QQ 有独立未读图标，不进入这里。
    pub(crate) windows_notices: Vec<WindowsNotice>,
    pub(crate) windows_unread: bool,
    pub(crate) windows_first: usize,
    pub(crate) app_icons: HashMap<String,Option<egui::TextureHandle>>,
    pub(crate) messenger_scroll: f32,
    pub(crate) alerts: std::collections::VecDeque<Alert>,
    pub(crate) history: std::collections::VecDeque<Alert>,
    pub(crate) pending: Vec<Cmd>,
    pub(crate) todo_first: usize,
    pub(crate) todo_choosing: bool,
    pub(crate) pet: crate::island::pet::Pet,
    pub(crate) last_drink: Option<(u32, usize)>,
    // 其它
    pub(crate) toast: Option<(String, f64)>,
    pub(crate) toast_below: bool,
    pub(crate) last_input: f64,
    pub(crate) system_idle: Option<f64>,
    pub(crate) last_cursor: Option<Pos2>,
    pub(crate) pet_a: f32,
    pub(crate) last_t: Option<f64>,
    pub(crate) t: f64,
    pub(crate) dt: f32,
    pub(crate) today: String,
    pub(crate) current_hour: usize,
    /// 当前时刻（HH:MM），与 `today` 一起用于到期判断。
    pub(crate) current_clock: String,
    pub(crate) cursor_local: Option<Pos2>,
    pub(crate) started: bool,
    /// v0.5.4：音量条展示值（拖动时先用本地值，随后跟上系统回报）。
    pub(crate) volume_ui: Option<(u32, bool)>,
    /// 拖动/滚轮刚改过音量，这段时间内系统回声不再单独弹一条。
    pub(crate) volume_quiet_until: f64,
    /// 音量条填充的缓动量（0..1），让数值变化有「量感」。
    pub(crate) volume_a: f32,
    pub(crate) music_volume_ui: Option<(u32,bool)>,
    pub(crate) music_volume_quiet_until: f64,
    pub(crate) music_volume_open: bool,
    /// 拖动进度条松手后先用本地目标值显示：播放器回报新位置有延迟，等追上、换歌或超时再交还（来自 PR #162）。
    pub(crate) music_seek_ui: Option<(f64,String)>,
    pub(crate) music_seek_quiet_until: f64,
    /// 点了上一首 / 下一首的时刻：先本地把进度归零，等播放器报回新歌位置再交还。
    pub(crate) music_restart_at: Option<f64>,
    /// 上一次看到的蓝牙设备，用来只在连接/断开时提示一次。
    pub(crate) bt_shown: Vec<crate::os::bt::Device>,
    pub(crate) bt_initialized: bool,
    /// v0.5.5：邮件账号、最近邮件与设置表单。
    pub(crate) mail: MailState,
    /// 岛内详情与独立阅读窗口各保留一份排版缓存。
    pub(crate) mail_lines_cache: Vec<(std::sync::Arc<str>, std::sync::Arc<str>, u32, std::sync::Arc<Vec<crate::mail::format::Line>>)>,
    /// v0.5.6：日历（CalDAV）配置、未来 24 小时事件与设置表单。
    pub(crate) calendar: CalendarState,
    pub(crate) quota: crate::quota::ui::State,
    /// AI 会话：Agent 状态、Hook 与本地中继。
    pub(crate) agent: crate::ai_session::AgentState,
    /// 输入、其他通知或详情占用时，等它们结束后再展开的 AI 事件。
    pub(crate) agent_open_after_notice: Option<crate::ai_session::page::AgentAlert>,
    /// 天气页的当前数据与后台拉取状态。
    pub(crate) weather: crate::weather::State,
    pub(crate) weather_source: crate::weather::source_ui::State,
    pub(crate) updates: crate::island::updates::State,
    pub(crate) resident: bar::Motion,
    pub(crate) event_tray: crate::island::event_tray::EventTray,
    pub(crate) dnd: crate::island::dnd::Dnd,
    /// 置物架刚拖入文件 / 日程刚提醒时的短暂徽标。
    pub(crate) shelf_badge_until: f64,
    pub(crate) calendar_badge_until: f64,
    pub(crate) resident_power: (Option<u8>,bool),
}
impl App {
    pub fn new(ctx: &egui::Context, opts: Options) -> Self {
        theme::load_fonts(ctx);
        egui_extras::install_image_loaders(ctx);
        ctx.set_theme(egui::ThemePreference::Dark);
        ctx.set_visuals(egui::Visuals::dark());
        ctx.style_mut(|s| {
            s.interaction.selectable_labels = false;
            s.visuals.text_cursor.stroke = egui::Stroke::new(1.5_f32, theme::BLUE);
            s.visuals.selection.bg_fill = theme::alpha_blue(0.45);
        });
        let settings = model::Settings::load();
        let clipboard_on = settings.clipboard_history;
        let (bw, bh) = (BAR_W, BAR_H);
        let focus_seconds=settings.focus_minutes as f32*60.0;
        let initial_lyrics=settings.lyrics_enabled;
        let initial_page=settings.page_order.iter().copied().find(|page|settings.visible_pages[*page]).unwrap_or(0);
        let weather_source=crate::weather::source_ui::State::load();
        let mut weather=crate::weather::State::load(&settings.weather_city,settings.weather_position.as_ref());
        if weather_source.displayed==crate::weather::source_ui::Provider::Qweather {weather.snapshot=weather_source.runtime.snapshot();}
        let mut agent=crate::ai_session::AgentState::load();
        if settings.visible_pages.get(AI_PAGE).copied().unwrap_or(false) && agent.config.enabled { agent.start(ctx); }
        Self {
            opts,
            render_compat_running: settings.render_compat,
            settings,
            todos: crate::todo::TodoList::load(),
            water: crate::health::Water::default(),
            shelf: crate::shelf::Shelf::load(),
            w: Spring::new(bw, CLOSE.0, CLOSE.1),
            h: Spring::new(bh, CLOSE.0, CLOSE.1),
            open: false,
            hover_since: None,
            leave_since: None,
            hover_lock: false,
            reveal: 0.0,
            open_at: -10.0,
            close_at: -10.0,
            shell_settled_at: 0.0,
            position_drag: None,
            drag_glide: None,
            drag_lift: anim::Spring::new(0.0, LIFT_K.0, LIFT_K.1),
            drag_stretch: anim::Spring::new(0.0, 260.0, 0.6),
            center_locked: false,
            center_lock_at: -10.0,
            center_guide: 0.0,
            center_lock_a: 0.0,
            grip_hover_since: None,
            page: initial_page,
            page_from: initial_page,
            page_scroll_area: None,
            page_dir: 1.0,
            page_t0: -10.0,
            slide: Spring::new(0.0, 260.0, 0.92),
            dots: std::array::from_fn(|i| Spring::new(if i == initial_page { 16.0 } else { 4.0 }, 380.0, 0.86)),
            wheel_delta: 0.0,
            wheel_at: -1.0,
            wheel_page_at: -1.0,
            drag_hot: false,
            drag_a: 0.0,
            drag_saved: None,
            shelf_scroll: Spring::new(0.0, 300.0, 0.9),
            shelf_first: 0,
            shelf_menu_rect: None,
            shelf_x: HashMap::new(),
            shelf_born: HashMap::new(),
            icons: Default::default(), avoidance: Default::default(),
            confirm_clear: None,
            clipboard: crate::clipboard::Shared::new(clipboard_on),
            clip_first: 0,
            clip_confirm_clear: None,
            water_ring: Spring::new(0.0, 120.0, 0.9),
            water_ml: 0.0,
            water_view: 0,
            water_view_t: Spring::new(0.0, 200.0, 0.9),
            sit_view_t: Spring::new(0.0, 200.0, 0.9),
            sit_mode: false,
            health_time: false,
            remind_t: Spring::new(1.0, 400.0, 0.8),
            last_reminder: 0.0,
            health_pending: model::load_health_pending(),
            fullscreen_hidden: false,
            sit_start: 0.0,
            sit_t: Spring::new(1.0, 400.0, 0.8),
            pomo: Pomo { phase: Phase::Focus, remaining: focus_seconds, running: false }, settings_editing:false,
            ring_shown: Spring::new(1.0, 160.0, 0.95),
            reset_spin: Spring::new(0.0, 160.0, 0.82),
            adding: false,
            todo_input_focused: false,
            todo_ime_active: false,
            todo_ime_frame: false,
            editing: None,
            buf: String::new(),
            todo_pop: Spring::new(0.0, 500.0, 0.45),
            ly: LyricsState::default(),
            ly_worker: None,
            show_lyrics: initial_lyrics,
            island_lyric_layout: None,
            media_anchor: (String::new(), 0.0, 0.0, false),
            media: NowPlaying::default(),
            music_artwork: None,
            forced_open_until:0.0, settings_open:false, settings_window:std::sync::Arc::new(std::sync::Mutex::new(crate::island::settings_window::State::load())), displays:Vec::new(), browser_at_top:false, browser_minimal_active:false, glass_tint:0.0, fluid:Default::default(), fluid_origin:Pos2::ZERO, fluid_art:None, fluid_title:String::new(), screen:None, settings_focus_requested:false, settings_tab:0, settings_category:0, pending_page_order:None, messenger_statuses:Default::default(), notification_apps:model::NotificationApps::load(), subpage:None, task_dialog:None,
            listener_status: "尚未连接".into(),
            notification_bell_until: 0.0, notification_bell_at: -10.0, notice: None, hud_notice: None, hud_alpha: 0.0, notice_width:Spring::new(0.0,NOTICE_CLOSE.0,NOTICE_CLOSE.1), notice_details:false, pending_messengers:Vec::new(), windows_notices:Vec::new(), windows_unread:false, windows_first:0, app_icons:Default::default(), messenger_scroll:0.0, alerts: Default::default(), history: Default::default(), pending: vec![],
            todo_first: 0, todo_choosing: false, pet: Default::default(), last_drink: None,
            toast: None, toast_below: false,
            last_input: 0.0, system_idle: None,
            last_cursor: None,
            pet_a: 0.0,
            last_t: None,
            t: 0.0,
            dt: 0.0,
            today: String::new(), current_hour: 0, current_clock: String::new(),
            cursor_local: None,
            started: false,
            volume_ui: None,
            volume_quiet_until: 0.0,
            volume_a: 0.0,
            music_volume_ui: None,
            music_volume_quiet_until: 0.0,
            music_volume_open: false,
            music_seek_ui: None,
            music_seek_quiet_until: 0.0,
            music_restart_at: None,
            bt_shown: Vec::new(),
            bt_initialized: false,
            mail: MailState { accounts: mail::load_accounts(), ..Default::default() },
            mail_lines_cache: Vec::new(),
            calendar: CalendarState::load(),
            quota: crate::quota::ui::State::load(),
            agent,
            agent_open_after_notice:None,
            weather,
            weather_source,
            updates: crate::island::updates::State::load(model::config_dir().join("update-check.json")),
            resident: Default::default(), event_tray: Default::default(), dnd: Default::default(), shelf_badge_until:0.0, calendar_badge_until:0.0, resident_power:(None,false),
        }
    }

    pub fn say(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), self.t));
        self.toast_below = false;
    }

    pub fn messenger_unavailable(&mut self, message: impl Into<String>) {
        self.say(message);
        self.toast_below = true;
    }

    /// v0.5.4：把系统音量与蓝牙设备状态同步进界面；只在连接/断开时提示一次。
    pub(crate) fn sync_device(&mut self, env: &Env) {
        if self.t >= self.volume_quiet_until {
            if let Some(v) = env.volume {
                self.volume_ui = Some(v);
            }
        }
        if !self.settings.bluetooth_notifications {
            self.bt_shown.clear();
            self.bt_initialized=false;
            self.alerts.retain(|a|a.source!=crate::island::banner::AlertSource::Internal || a.title!="蓝牙设备");
            if self.notice.as_ref().is_some_and(|a|a.source==crate::island::banner::AlertSource::Internal && a.title=="蓝牙设备") {self.notice=None;}
            return;
        }
        if !env.bt_ready { return; }
        // 只把首次完整扫描作为基线，之后的连接变化都应提示。
        if !self.bt_initialized {
            self.bt_initialized = true;
            self.bt_shown = env.bt.clone();
            return;
        }
        let connected: Vec<_> = env.bt.iter().filter(|d| !self.bt_shown.iter().any(|old| old.id == d.id)).cloned().collect();
        let disconnected: Vec<_> = self.bt_shown.iter().filter(|d| !env.bt.iter().any(|new| new.id == d.id)).cloned().collect();
        for device in &connected { self.bluetooth_alert(device, true); }
        for device in &disconnected { self.bluetooth_alert(device, false); }
        self.bt_shown = env.bt.clone();
    }

    /// 距离上次交互的秒数（小精灵出现的依据）。
    pub fn idle_for(&self) -> f64 {
        self.system_idle.unwrap_or(self.t - self.last_input)
    }

    pub(crate) fn window_layout(&self)->std::sync::MutexGuard<'_,crate::island::settings_window::State>{
        self.settings_window.lock().unwrap_or_else(|e|e.into_inner())
    }

    pub fn save_window_before_exit(&self){self.window_layout().close();}

    #[cfg(windows)]
    pub fn start_tray(&self,state:crate::os::tray::State)->Option<crate::os::tray::Tray>{
        let layout=self.settings_window.clone();
        // 原生文件选择框会占用界面线程，托盘退出时直接保存最近一次观察到的布局。
        crate::os::tray::Tray::start_with_exit(state,move||layout.lock().unwrap_or_else(|e|e.into_inner()).close())
    }

    /// 按 AI 模块与总开关启停会话监控；关闭时移除桥接并清空会话。
    pub fn reconcile_agent_monitor(&mut self,ctx:&egui::Context){
        let on=self.module_on(AI_PAGE)&&self.agent.config.enabled;
        if on!=self.agent.active {self.agent.set_active(on,ctx);}
    }
    /// OLED 像素偏移只在收起动画完成、没有鼠标交互、不在编辑时移动；
    /// 展开、点击、拖拽时保持当前位置，避免面板内容和位移一起跳。
    pub fn oled_shift_idle(&self,ctx:&egui::Context) -> bool {
        crate::os::oled_shift::should_move(self.open, self.position_drag.is_some() || self.drag_glide.is_some(), self.is_editing())
            && self.hover_since.is_none() && !self.drag_hot
            && self.w.settled(PW) && self.h.settled(self.expanded_height())
            && self.drag_lift.settled(1.0) && self.drag_stretch.settled(1.0)
            && ctx.input(|i|!i.pointer.any_down() && !i.pointer.any_pressed() && i.raw.hovered_files.is_empty())
    }

    /// 慢帧日志用：当前页与展开状态。
    pub fn debug_state(&self) -> (usize, bool) {
        (self.page, self.open)
    }

    pub fn is_editing(&self) -> bool {
        self.adding || self.editing.is_some() || self.todo_choosing || self.task_dialog.is_some() || self.calendar.draft.is_some() || self.settings_editing || self.agent.free_focus
    }

    pub fn filter_todo_input(&mut self,input:&mut egui::RawInput) {
        self.todo_ime_frame=false;
        if !self.adding && self.editing.is_none(){self.todo_ime_active=false;return;}
        self.todo_ime_frame=self.todo_ime_active || input.events.iter().any(|event|matches!(event,egui::Event::Ime(egui::ImeEvent::Enabled|egui::ImeEvent::Preedit(_)|egui::ImeEvent::Commit(_))));
        for event in &input.events {
            match event {
                egui::Event::Ime(egui::ImeEvent::Enabled|egui::ImeEvent::Preedit(_))=>self.todo_ime_active=true,
                egui::Event::Ime(egui::ImeEvent::Commit(_)|egui::ImeEvent::Disabled)=>self.todo_ime_active=false,
                _=>{},
            }
        }
        // 首帧和结束帧的按键、失焦仍属于组合输入，不能透传给待办编辑。
        if self.todo_ime_frame {input.events.retain(|event|!matches!(event,egui::Event::Key{key:egui::Key::Backspace|egui::Key::Delete|egui::Key::Enter|egui::Key::Escape|egui::Key::ArrowLeft|egui::Key::ArrowRight|egui::Key::ArrowUp|egui::Key::ArrowDown,..}));}
    }

    pub fn foreground_changed(&mut self) {
        if self.todo_ime_frame {return;}
        self.hover_lock=true;
        if self.adding && self.buf.trim().is_empty(){self.cancel_edit();}
    }

    pub(crate) fn cancel_edit(&mut self) {
        self.settings_editing=false;
        self.adding = false;
        self.editing = None;
        self.todo_choosing = false;
        self.task_dialog=None;
        self.calendar.draft=None;
        self.buf.clear();
        self.todo_input_focused=false;
        self.todo_ime_active=false;
        self.todo_ime_frame=false;
        self.agent.free.clear();
        self.agent.free_focus=false;
    }

    pub(crate) fn shared_header(&mut self, base: &Card, cx: f32, p: f32, env: &Env) {
        let local_cx = cx - base.origin.x;
        // 展开后右侧留给勿扰和设置；状态行在剩余宽度内居中，组件多时整体等比缩小而不是溢出。
        let tail=smoothstep(0.70,0.98,p);
        let tail_w=bar::TAIL_W*tail;
        let available=lerp(WIN_W-32.0,PW-16.0,p)-tail_w;
        let fit=(available/self.resident.target.width).min(1.0);
        let row_cx=local_cx-tail_w/2.0;
        let fitted=base.scaled(row_cx,15.0,fit);
        let base=&fitted;
        let lyric=self.island_lyric().map(|(index,_)|index);
        let lyric_alpha=if self.island_lyrics_active() {1.0-smoothstep(0.0,0.35,p)}else{0.0};
        if let Some(galley)=self.island_lyric_layout.clone().filter(|_|lyric_alpha>0.003) {
            let c=base.faded(lyric_alpha);
            let content_width=self.lyric_bar_width()-self.lyric_focus_space();
            let width=self.w.pos.max(content_width);
            let left=local_cx-width/2.0;
            let center_y=BAR_H/2.0;
            if let Some((image,texture))=&self.music_artwork {
                let [width,height]=image.size;
                let uv_size=if width>height {egui::vec2(height as f32/width as f32,1.0)}else{egui::vec2(1.0,width as f32/height as f32)};
                let uv=Rect::from_center_size(Pos2::new(0.5,0.5),uv_size);
                c.painter.add(egui::epaint::RectShape::filled(c.rect(left+12.0,center_y-10.0,20.0,20.0),5.0*c.s,Color32::WHITE).with_texture(texture.id(),uv));
            } else {
                c.fill(left+12.0,center_y-10.0,20.0,20.0,5.0,theme::CARD);
                paint::note(&c,left+22.0,center_y,13.0,theme::WHITE);
            }
            let index=lyric.unwrap_or(0);
            let id=egui::Id::new("island-lyric-line");
            if !self.ly.island_animation_started {
                c.ui.ctx().animate_value_with_time(id,index as f32,0.0);
                self.ly.island_animation_started=true;
            }
            let shown=c.ui.ctx().animate_value_with_time(id,index as f32,0.18);
            let fade=1.0-(index as f32-shown).abs().min(1.0);
            let position=c.pos(left+content_width-LYRIC_BAR_W/2.0,center_y-galley.size().y/2.0+(1.0-fade)*3.0);
            c.painter.galley(position,galley,if lyric.is_some(){theme::WHITE}else{theme::GRAY});
            paint::playback_bars(&c,left+content_width-21.0,center_y,self.t,self.media.playing,theme::WHITE);
            if self.pomo.running && self.settings.show_focus {
                let alpha=smoothstep(0.88,1.0,(width-content_width)/self.lyric_focus_space());
                crate::focus::bar::draw(&c.faded(alpha),left+width-14.0-crate::focus::bar::LYRIC_WIDTH/2.0,center_y,self.t,&self.pomo,self.phase_secs(self.pomo.phase),true);
            }
        }
        if lyric_alpha>=0.997 {return;}
        let header=base.faded(1.0-lyric_alpha);
        let base=&header;
        // 尾部控件不跟随状态行的等比缩放，固定在面板右侧。
        let status=base;
        let layout=self.resident.current();
        // 展开态只显示时:分，不受 12 小时制、时段文字和秒数设置影响。
        let clock=if self.open {env.clock.get(0..5).unwrap_or("--:--").to_string()}else{self.settings.clock_text(&env.clock)};

        if let Some(x)=layout.x[bar::BATTERY] {
            paint::battery(&base.faded(layout.clearance(bar::BATTERY)),row_cx+x,15.0,env.battery,env.charging,bar::BATTERY_SCALE);
        }
        if let Some(x)=layout.x[bar::CLOCK] {
            let w=paint::clock_width(base,&clock,size::PILL_CLOCK);
            paint::clock(&base.faded(layout.clearance(bar::CLOCK)),row_cx+x-w/2.0,21.0,&clock,size::PILL_CLOCK);
        }
        if let Some(x)=layout.x[bar::MUSIC] {
            let x=row_cx+x;
            paint::playback_bars_box(base,x,15.0,self.t,self.media.playing,theme::WHITE,bar::GLYPH,bar::MUSIC_H);
            if base.hit(x-9.0,3.0,18.0,24.0,"header-music")
                .on_hover_text(if self.settings.visible_pages[0]{"返回音乐播放页"}else{"打开音乐设置"})
                .on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){self.show_page(0);}
        }
        if let Some(x)=layout.x[bar::BELL] {
            let x=row_cx+x;
            paint::bell(&base.scaled(x,14.0,bar::BELL_SCALE),x,14.0,theme::WHITE,paint::bell_swing(self.t-self.notification_bell_at));
            base.circle(x+4.0,9.5,2.0,Color32::from_rgb(255,72,72));
            let bell=base.hit(x-9.0,3.0,18.0,25.0,"bell-mail").on_hover_cursor(egui::CursorIcon::PointingHand);
            if crate::quota::ui::bar_tooltip(bell,"Windows 通知 · 点击打开".into()).clicked(){
                self.show_page(WINDOWS_PAGE);
            }
        }
        let indices=self.bar_quota_indices();
        self.draw_quota_bar(base,row_cx,&layout,&indices);
        if let Some(x)=layout.x[bar::WEATHER] { self.draw_pill_weather(base,row_cx+x,&layout); }
        if let Some(x)=layout.x[bar::TRAY] { self.draw_tray(base,row_cx+x,&layout); }
        self.draw_page_badges(base,row_cx,&layout);
        if let Some(x)=layout.x[bar::AGENT] {
            let x=row_cx+x;
            self.draw_agent_light(base,x,15.0);
            let hint={
                let active=self.agent.store.active();
                let waiting=active.iter().filter(|session|session.waiting()).count();
                active.first().map(|session|format!("{} · {} 个活跃会话 · {} 个等待你",session.agent.display(),active.len(),waiting))
                    .unwrap_or_else(||"AI 会话 · 暂无活跃会话".to_string())
            };
            let pill=base.hit(x-10.0,3.0,20.0,24.0,"agent-pill")
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if crate::quota::ui::bar_tooltip(pill,hint).clicked(){self.show_page(AI_PAGE);}
        }
        if let Some(x)=layout.x[bar::FOCUS] {
            crate::focus::bar::draw(base,row_cx+x,15.0,self.t,&self.pomo,self.phase_secs(self.pomo.phase),false);
        }
        if tail>0.003 {
            let c=status.faded(tail);
            let c=if tail>0.9 {c}else{c.ghost()};
            let gear_x=PW-bar::EDGE_PAD-bar::TAIL_SLOT/2.0;
            let dnd_x=gear_x-bar::TAIL_SLOT-bar::GAP;
            let r=bar::TAIL_SLOT/2.0;
            if self.settings.show_dnd {
                let dnd=c.hit(dnd_x-r,3.0,r*2.0,24.0,"page-dnd").on_hover_text(if self.dnd.active{"关闭勿扰模式"}else{"开启勿扰模式"});
                let (hover,press)=c.hover_t(&dnd);
                c.circle(dnd_x,15.0,r,Color32::from_white_alpha((hover*24.0) as u8));
                paint::moon(&c.scaled(dnd_x,15.0,1.0-press*0.08),dnd_x,15.0,bar::MOON_SIZE,if self.dnd.active{theme::WHITE}else{paint::mix(theme::GRAY,theme::WHITE,hover)});
                if dnd.clicked(){self.dnd.toggle(self.t);}
            }
            let gear=c.hit(gear_x-r,3.0,r*2.0,24.0,"page-settings").on_hover_text(format!("打开{}设置窗口",PAGE_NAMES[self.page]));
            let (hover,press)=c.hover_t(&gear);
            c.circle(gear_x,15.0,r,Color32::from_white_alpha((hover*24.0) as u8));
            paint::gear(&c.scaled(gear_x,15.0,bar::GEAR_SCALE*(1.0-press*0.08)),gear_x,15.0,paint::mix(theme::GRAY,theme::WHITE,hover));
            if gear.clicked(){self.settings_tab=PAGE_SETTINGS_TABS[self.page];self.open_settings();}
        }
    }

    /// 常驻状态行：顶部停靠收起时是药丸，展开后留在面板顶部；
    /// 左右停靠收起时是竖排胶囊，展开后同样换成顶部状态行。
    pub(crate) fn resident_header(&mut self, base: &Card, panel: Rect, cx: f32, p: f32, reveal: f32, env: &Env) {
        if !self.side() {self.shared_header(base,cx,p,env);return;}
        let bar_alpha=1.0-smoothstep(0.0,0.3,p);
        let minimal=self.browser_minimal_active&&self.t>=self.notification_bell_until&&!self.windows_unread;
        if bar_alpha>0.004 && !minimal {
            let c=base.shifted(panel.min.x-base.origin.x,panel.min.y-base.origin.y).faded(bar_alpha);
            let c=if bar_alpha<0.8 {c.ghost()} else {c};
            self.side_bar(&c,panel.width(),panel.height(),env);
        }
        if reveal>0.004 {self.shared_header(&base.faded(reveal),base.origin.x+PW/2.0,1.0,env);}
    }

    pub(crate) fn update_pet(&mut self, env: &Env, cx: f32) {
        let cx=cx+self.resident.position(bar::PET);
        let idle = self.idle_for();
        if self.pet.last_idle >= self.opts.pet_idle && idle < 1.0
            && matches!(self.pet.state, PetState::Dozing | PetState::Sleeping) {
            self.pet.waking_until = self.t + 2.2;
        }
        self.pet.last_idle = idle;
        let was_hovered = self.pet.hovered;
        self.pet.hovered = env.cursor.is_some_and(|p| (p.x-cx).abs() <= BAR_W/2.0+24.0 && (0.0..70.0).contains(&p.y));
        if self.pet.hovered || was_hovered {
            // 悬停时跟随指针；离开后再留半秒把眼神收回，然后停在静态姿势。
            self.pet.anim_until = self.t + 0.4;
        }
        self.pet.worried = self.settings.show_battery && env.battery.is_some_and(|v|v<=20) && !env.charging || env.network == Some(false);
        let allowed = self.pet_enabled();
        if !allowed { self.pet.reaction = None; self.pet.waking_until = 0.0; self.pet.expressions.clear(); self.pet.anim_until = 0.0; }
        let available = allowed && !self.drag_hot;
        self.pet_a = anim::approach(self.pet_a, if available {1.0}else{0.0}, 9.0, self.dt);
        // 吉祥物随状态行常驻；拖动岛体时暂时收起。
        if !available { return; }
        if self.pet_a < 0.7 { self.pet.anim_until = self.t + 1.2; }
        let visible = self.pet_a > 0.7;
        let state = self.pet_state();
        if state != self.pet.state { self.pet.anim_until = self.t + 1.2; }
        // 空闲时不推进吉祥物时钟：没有悬停、表情或状态过渡就保持当前帧。
        // 面板展开时吉祥物保持当前表情，反馈计时留到收起后继续。
        let ticking = !self.open;
        let moving = ticking && self.pet_a > 0.01
            && (self.pet.hovered || self.pet.reaction.is_some() || self.pet.waking_until > self.t
                || self.pet.expressions.current().is_some() || self.t < self.pet.anim_until
                || state == PetState::Music);
        if moving { self.pet.clock += self.dt as f64; }
        let dt = if moving { self.dt } else { 0.0 };
        let gaze = if moving {
            env.cursor.map(|p| [((p.x-cx)/28.0).clamp(-3.0,3.0), ((p.y-15.0)/40.0).clamp(-1.0,1.5)])
                .unwrap_or([0.0, 0.0])
        } else { [0.0, 0.0] };
        self.pet.advance(dt, visible);
        self.pet.step(state, self.pet.clock, dt, gaze, visible);
    }

    // ---------------------------------------------------------------
    // 面板主体：页标题 + 页面（带切页过渡）+ 翻页箭头 + 页码豆
    // ---------------------------------------------------------------
    pub(crate) fn panel_body(&mut self, c: &Card, env: &Env, cmds: &mut Vec<Cmd>) -> bool {
        if self.open && self.reveal>0.95 && self.progress()>0.98 && !self.is_editing() && !self.drag_hot {
            if let Some(screen)=env.screen {
                // 中间的标题区不占用左侧新增、刷新和右侧设置按钮。
                let (left,right)=(150.0,270.0);
                // Windows 没有「抓手」光标，Grab 会变成四向箭头；保持普通箭头，改用岛体内的把手提示。
                let response=c.hit_drag(left,4.0+PAGE_RAISE,right-left,26.0,"island-position").on_hover_text("按住拖动位置");
                let (_,press)=c.hover_t(&response);
                // 停留才显现：划过（< 0.3 s）什么都不出现；先露出把手，再慢慢浮起底色；按下立即到位，移开快速淡出。
                let dwell=(if response.hovered() {self.t-*self.grip_hover_since.get_or_insert(self.t)}else{self.grip_hover_since=None;0.0}) as f32;
                if response.hovered() && dwell<1.2 {c.ui.ctx().request_repaint();}
                let ctx=c.ui.ctx();
                let grip_in=ctx.animate_value_with_time(response.id.with("grip"),smoothstep(0.30,0.65,dwell),0.12);
                let fill_in=ctx.animate_value_with_time(response.id.with("grip-fill"),smoothstep(0.50,1.00,dwell),0.12);
                let (grip_in,fill_in)=(grip_in.max(press),fill_in.max(press));
                if grip_in>0.003 {
                    let pad=6.0;
                    let (x,w)=(left-pad,right-left+pad*2.0);
                    if fill_in>0.003 {c.fill(x,5.0+PAGE_RAISE,w,26.0,13.0,Color32::from_white_alpha((fill_in*7.0+press*12.0) as u8));}
                    let grip=lerp(14.0,22.0,press)*lerp(0.5,1.0,grip_in);
                    c.fill(x+w/2.0-grip/2.0,7.5+PAGE_RAISE,grip,2.6,1.3,Color32::from_white_alpha((grip_in*60.0+press*60.0) as u8));
                }
                // 开始移动立即拿起；原地按住片刻也会拿起，先给反馈再移动。
                let held=response.is_pointer_button_down_on() && c.ui.input(|i|i.pointer.primary_down() && i.pointer.press_start_time().is_some_and(|at|i.time-at>=LIFT_HOLD));
                if response.is_pointer_button_down_on() {c.ui.ctx().request_repaint();}
                if response.drag_started_by(egui::PointerButton::Primary) || held {
                    if let Some(start)=c.ui.input(|i|i.pointer.press_origin()) {self.start_position_drag(start,screen);}
                }
            }
        }
        if self.subpage==Some(self.page) {
            c.text(15.0,25.0,"‹",19.0,W::Bold,theme::WHITE,H::Left);
            // 健康提醒设置沿用主页的标题 + 喝水/久坐切换，切到哪边就设置哪边。
            if self.page==2 && self.health_time {c.text(31.0,25.0,"提醒时段",size::TITLE,W::Bold,theme::WHITE,H::Left);}
            else if self.page==2 {let sv=self.sit_view_t.pos.clamp(0.0,1.0);self.health_tabs(c,31.0,sv);}
            else {c.text(37.0,25.0,"专注设置",size::TITLE,W::Bold,theme::WHITE,H::Left);}
            // 「提醒时段」二级页的返回先回到健康提醒设置，再退出设置。
            if c.hit(9.0,7.0,28.0,23.0,"subpage-back").clicked(){
                if self.page==2 && self.health_time {self.health_time=false;} else {self.subpage=None;self.settings_editing=false;}
            }
            self.page_settings(c);self.nav(c);return false;
        }
        let e = self.page_e();
        let in_tx = e < 0.55 && self.page_from != self.page;
        if in_tx {
            // 旧页：原地向反方向轻移并快速淡出；新页：弹簧滑入 + 稍晚淡入
            let old_a = 1.0 - smoothstep(0.0, 0.16, e);
            let new_a = smoothstep(0.03, 0.28, e);
            let old_dx = -self.page_dir * 18.0 * anim::ease_out_cubic(e / 0.3);
            if old_a > 0.0 {
                let oc = c.faded(old_a).ghost();
                self.page_title(self.page_from, &oc);
                self.render_page(self.page_from, &oc.shifted(old_dx, 0.0), env, &mut Vec::new());
            }
            let nc = c.faded(new_a);
            self.page_title(self.page, &nc);
            let nc = nc.shifted(self.slide.pos, 0.0);
            let nc = if e < 0.12 { nc.ghost() } else { nc };
            self.render_page(self.page, &nc, env, cmds);
        } else {
            self.page_title(self.page, c);
            self.render_page(self.page, c, env, cmds);
        }
        self.nav(c);
        if self.page==0 && c.live {self.draw_music_volume(c,cmds);}
        in_tx
    }

    pub(crate) fn page_title(&self, page: usize, c: &Card) {
        let base = 25.0;
        let (a, b, col) = match page {
            0 | CALENDAR_PAGE => return,
            1 => ("番茄", "专注", theme::RED_TOP),
            // 健康页的标题由页面自己画（「健康提醒」+ 喝水/久坐分段切换）。
            2 => return,
            3 => ("置物", "架", theme::BLUE),
            4 => ("待办", "清单", theme::GREEN),
            5 => ("邮箱", "", theme::CYAN),
            6 => ("AI", "额度", theme::SOFT_GREEN),
            AI_PAGE => ("AI", "会话", theme::BLUE),
            WINDOWS_PAGE => ("Windows", "通知", theme::CYAN),
            CLIPBOARD_PAGE => ("剪贴", "板", theme::BLUE),
            8 if self.weather.solar_open => ("日出", "日落", theme::YELLOW),
            8 => ("当地", "天气", theme::CYAN),
            _ => ("岛仔", "设置", theme::WHITE),
        };
        let wa = c.text(15.0, base, a, size::TITLE, W::Bold, theme::WHITE, H::Left);
        if !b.is_empty() {
            c.text(15.0 + wa + 1.5, base, b, size::TITLE, W::Bold, col, H::Left);
        }
    }

    pub(crate) fn nav(&mut self, c: &Card) {
        let pages: Vec<_>=self.settings.page_order.iter().copied().filter(|page|self.settings.visible_pages[*page]).collect();
        if pages.len()<2 { return; }
        // AI 详情子页的按钮和选项占满整行；先返回列表再翻页，也不画分页圆点。
        if self.page==AI_PAGE && self.agent.detail { return; }
        let arrow_y=if self.page==6||self.page==8||self.page==AI_PAGE||self.page==WINDOWS_PAGE{88.0}else{68.0};
        let l = c.hit(8.0, arrow_y-13.0, 26.0, 26.0, "nav-l");
        let r = c.hit(385.0, arrow_y-13.0, 26.0, 26.0, "nav-r");
        let (lh, lp) = c.hover_t(&l);
        let (rh, rp) = c.hover_t(&r);
        paint::chevron(c, 21.0, arrow_y, -1.0, lh, lp);
        paint::chevron(c, 398.0, arrow_y, 1.0, rh, rp);
        if l.clicked() { self.navigate_page(false); }
        if r.clicked() { self.navigate_page(true); }
        // 页码豆：宽度由弹簧驱动，整行实时居中；当前页是白色长条
        let gap = 4.0;
        // 分页圆点贴着面板下沿；页面内容上移时圆点保持原位。
        let dot_y=self.expanded_height()-STATUS_H-7.2+PAGE_RAISE;
        let total: f32 = pages.iter().map(|page| self.dots[*page].pos).sum::<f32>() + gap * (pages.len()-1) as f32;
        let mut x = PW / 2.0 - total / 2.0;
        let mut clicked = None;
        for i in pages {
            let w = self.dots[i].pos.max(3.0);
            let on = anim::remap(w, 4.0, 16.0);
            let col = paint::mix(Color32::from_rgb(92, 92, 96), theme::WHITE, on);
            c.fill(x, dot_y - 2.0, w, 4.0, 2.0, col);
            let hr = c.hit(x - 2.0, dot_y - 6.0, w + 4.0, 12.0, ("dot", i));
            hr.clone().on_hover_text(PAGE_NAMES[i]);
            if hr.hovered() && i != self.page {
                c.fill(x - 1.0, dot_y - 3.0, w + 2.0, 6.0, 3.0, Color32::from_white_alpha(28));
            }
            if hr.clicked() {
                clicked = Some(i);
            }
            x += w + gap;
        }
        if let Some(i) = clicked {
            let position=|page|self.settings.page_order.iter().position(|p|*p==page).unwrap();
            let dir = if position(i) > position(self.page) { 1.0 } else { -1.0 };
            self.go(i, dir);
        }
    }

    pub(crate) fn render_page(&mut self, page: usize, c: &Card, env: &Env, cmds: &mut Vec<Cmd>) {
        match page {
            0 => self.page_music(c, env, cmds),
            1 => self.page_focus(c, env),
            2 => self.page_water(c, env),
            3 => self.page_shelf(c, env, cmds),
            4 => self.page_todo(c, env, cmds),
            5 => self.page_mail(c, env, cmds),
            6 => self.page_quota(c),
            8 => self.page_weather(c,env,cmds),
            AI_PAGE => self.page_agent(c),
            WINDOWS_PAGE => self.page_windows(c),
            CALENDAR_PAGE => self.page_calendar(c,env),
            CLIPBOARD_PAGE => self.page_clipboard(c,env,cmds),
            _ => self.page_settings(c),
        }
    }

    pub(crate) fn toast_layer(&mut self, c: &Card, base_y:f32)->Option<Rect> {
        let Some((msg, at)) = self.toast.clone() else { return None };
        let e = (self.t - at) as f32;
        if e > 2.4 {
            self.toast = None;
            return None;
        }
        let a = smoothstep(0.0, 0.16, e) * (1.0 - smoothstep(2.1, 2.4, e));
        let rise = (1.0 - anim::ease_out_cubic(e / 0.3)) * 6.0;
        let msg=c.fit(&msg,size::BODY,W::Regular,310.0);
        let w = c.text_w(&msg, size::BODY, W::Regular) + 26.0;
        let tc = c.faded(a);
        let y = base_y + rise;
        tc.fill(PW / 2.0 - w / 2.0, y, w, 22.0, 11.0, Color32::from_rgb(44, 44, 48));
        tc.text(PW / 2.0, y + 15.0, &msg, size::BODY, W::Regular, theme::WHITE, H::Center);
        Some(tc.rect(PW/2.0-w/2.0,y,w,22.0))
    }

    // ---------------------------------------------------------------
    // 每帧推进：番茄钟、饮水提醒、跨天
    // ---------------------------------------------------------------
    pub(crate) fn tick(&mut self, env: &Env, dt: f32, elapsed: f32) {
        if self.toast.as_ref().is_some_and(|(_,at)|self.t-at>2.4){self.toast=None;}
        // 跳转后先用本地目标位置显示；播放器回报追上了、换歌了或超时就交还真实位置。
        let seek_done=self.music_seek_ui.as_ref().is_some_and(|(target,title)|{
            (self.media.duration>0.0 && (self.media.position-*target).abs()<1.5)
                || self.t>=self.music_seek_quiet_until || &self.media.title!=title
        });
        if seek_done {self.music_seek_ui=None;}
        if self.music_restart_at.is_some_and(|at|self.t>=at+1.5){self.music_restart_at=None;}
        if env.today != self.today {
            self.today = env.today.clone();
            self.reload_water(&env.today);
        }
        self.tick_focus(dt, elapsed);
        self.reset_spin.step(dt);
        self.todo_pop.step(dt);
        self.tick_health(env, dt);
        self.todo_reminders(env);
        // v0.5.5 会议提醒：.ics 里的开始时间，默认提前 15 分钟（可设 5/15/30）。
        // v0.5.6 起日历（CalDAV）事件走同一套：同一场会（UID + DTSTART）只提醒一次，
        // 邮件和日历两条来源撞上了也只弹一次。
        let lead = self.settings.mail_ics as i64 * 60;
        let now_ts = chrono::Local::now().timestamp();
        let mut picks = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        // 会议提醒点开后进入日程页；日程模块关闭时不再提醒（含邮件邀请）。
        let calendar_on=self.module_on(CALENDAR_PAGE);
        for entry in self.calendar.local.iter().filter(|_|calendar_on) {
            let Some(at)=entry.reminder_at() else{continue;};
            let key=calendar::dedupe_key(&entry.event);
            let cutoff=if entry.event.all_day{at+60}else{entry.event.start+60};
            if now_ts>=at && now_ts<cutoff && !self.mail.fired.contains(&key) && seen.insert(key.clone()) {
                picks.push((key,entry.event.clone(),"本机日程".into(),CALENDAR_PAGE));
            }
        }
        for (m,provider) in self.calendar_work_events(None).into_iter().filter(|_|calendar_on) {
            let left = m.start - now_ts;
            let key = calendar::dedupe_key(m);
            if !m.all_day && left <= lead && left > -60 && !self.mail.fired.contains(&key) && seen.insert(key.clone()) {
                let source=provider.map(|provider|provider.label()).unwrap_or("日历");
                picks.push((key, m.clone(), source.to_string(), CALENDAR_PAGE));
            }
        }
        if calendar_on && self.mail_active() {
            for m in self.mail.recent.iter().filter_map(|m| m.meeting.as_ref()).filter(|m|!self.mail_event_ignored(m)) {
                let left = m.start - now_ts;
                let key = calendar::dedupe_key(m);
                if !m.all_day && left <= lead && left > -60 && !self.mail.fired.contains(&key) && seen.insert(key.clone()) {
                    picks.push((key, m.clone(), "邮件邀请".to_string(), CALENDAR_PAGE));
                }
            }
        }
        for (key, meeting, source, page) in picks {
            self.mail.fired.insert(key);
            self.meeting_alert(meeting, source, page);
        }
        // 剪贴板记录开关：关闭时同时清掉未固定的记录。
        if self.clipboard.is_enabled() != self.clipboard_active() {
            self.clipboard.set_enabled(self.clipboard_active());
        }
        self.shelf_scroll.target = self.shelf_first as f32;
        self.shelf_scroll.step(dt);
        if let Some(at) = self.confirm_clear {
            if self.t - at > 3.0 {
                self.confirm_clear = None;
            }
        }
    }

    /// 待办输入框：真正的 egui 文本框，放在待办卡片顶部那一行。
    pub(crate) fn input_field(&mut self, ui: &mut egui::Ui, origin: Pos2, cmds: &mut Vec<Cmd>) {
        let rect = Rect::from_min_size(origin + Vec2::new(222.0, 37.0), Vec2::new(142.0, 18.0));
        let mut buf = std::mem::take(&mut self.buf);
        let te = egui::TextEdit::singleline(&mut buf)
            .id(egui::Id::new("todo-input"))
            .frame(false)
            .margin(egui::Margin::ZERO)
            .text_color(theme::WHITE)
            .font(theme::font(size::BODY, W::Regular))
            .hint_text(egui::RichText::new("要做点什么？回车保存").color(theme::DIM).font(theme::font(size::BODY, W::Regular)))
            .desired_width(rect.width());
        let resp = ui.put(rect, te);
        let first_focus=!self.todo_input_focused;
        self.buf=buf;
        let lost_window=ui.input(|i|i.events.iter().any(|event|matches!(event,egui::Event::WindowFocused(false))));
        if !first_focus && !self.todo_ime_frame && self.adding && self.buf.trim().is_empty() && (resp.clicked_elsewhere() || lost_window) {
            resp.surrender_focus();
            self.cancel_edit();
            return;
        }
        if first_focus {
            self.todo_input_focused=true;
            resp.request_focus();
            cmds.push(Cmd::Focus);
        }
        let submit = ui.input(|i| i.key_pressed(egui::Key::Enter));
        if submit {
            let text = self.buf.trim().to_string();
            let mut added=false;
            if let Some(i) = self.editing {
                if !text.is_empty() && i < self.todos.items.len() {
                    self.todos.items[i].text = text;
                    self.todos.save();
                }
            } else if !text.is_empty() {
                self.todo_first = 0;
                self.todos.items.insert(0, crate::todo::TodoItem::new(text));
                self.todos.save();
                self.todo_pop.vel += 6.0;
                added=true;
            }
            self.cancel_edit();
            if added {self.open_task_reminder(0);}
        }
    }
}
