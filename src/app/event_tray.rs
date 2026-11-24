//! 事件容器：数据变化时短暂出现；邮件、健康、待办、日程持续到条件解除。
use crate::app::App;
use crate::app::bar_layout;
use crate::render::paint::Card;
use crate::render::theme;
use eframe::egui;
use std::hash::{Hash, Hasher};

/// 事件后显示的秒数；不做轮换，也不需要设置项。
pub(crate) const HOLD: f64 = 10.0;
/// 容器内相邻成员的间距；与状态行的组件间距一致。
pub(crate) const ITEM_GAP: f32 = 8.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Item {
    Weather = 0,
    Quota = 1,
    Focus = 2,
    Mail = 3,
    Health = 4,
    Todo = 5,
    Shelf = 6,
    Music = 7,
    Battery = 8,
    Calendar = 9,
}

impl Item {
    pub(crate) const ALL: [Item; 10] = [
        Item::Weather, Item::Quota, Item::Focus, Item::Mail, Item::Health,
        Item::Todo, Item::Shelf, Item::Calendar, Item::Music, Item::Battery,
    ];

    pub(crate) fn from_slot(slot: usize) -> Option<Item> {
        match slot {
            bar_layout::WEATHER => Some(Item::Weather),
            bar_layout::QUOTA => Some(Item::Quota),
            bar_layout::FOCUS => Some(Item::Focus),
            bar_layout::MAIL => Some(Item::Mail),
            bar_layout::HEALTH => Some(Item::Health),
            bar_layout::TODO => Some(Item::Todo),
            bar_layout::SHELF => Some(Item::Shelf),
            bar_layout::CALENDAR => Some(Item::Calendar),
            bar_layout::MUSIC => Some(Item::Music),
            bar_layout::BATTERY => Some(Item::Battery),
            _ => None,
        }
    }
}

pub(crate) struct EventTray {
    /// 每个成员最近一次事件的时刻。
    last: [f64; 10],
    /// 上一帧的显示值指纹；None 表示当前不可显示。
    seen: [Option<u64>; 10],
    /// 上一帧的容器成员开关，用于拖入时立即触发一次。
    member: [bool; 10],
    /// 横幅期间发生的变化先记下，等横幅结束后再开始计时。
    pending: [bool; 10],
    /// 首帧只记录不触发，避免启动时闪一次。
    primed: bool,
}

impl Default for EventTray {
    fn default() -> Self {
        Self { last: [f64::NEG_INFINITY; 10], seen: [None; 10], member: [false; 10], pending: [false; 10], primed: false }
    }
}

fn hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

impl App {
    /// 天气指纹只取常驻徽标上显示的数值，不含快照时间。
    fn weather_tray_fingerprint(&self) -> Option<u64> {
        let snapshot = self.weather_pill()?;
        Some(hash(&(snapshot.temp.to_bits(), snapshot.code, snapshot.place.clone(), snapshot.day)))
    }

    /// 额度指纹取每个常驻账号的环剩余值与跟随之字，不含 `Snapshot::at`。
    fn quota_tray_fingerprint(&self) -> Option<u64> {
        let indices = self.quota_pinned();
        if indices.is_empty() { return None; }
        let mut parts = Vec::new();
        for index in indices {
            let account = &self.quota.config.accounts[index];
            let result = self.quota.rows.get(&account.id);
            let rings = result.and_then(|r| r.as_ref().ok()).map(|snapshot| {
                snapshot.rings().iter().map(|window| {
                    (window.remaining.to_bits(), window.reset, window.amount.as_ref().map(|amount| (amount.remaining.to_bits(), amount.limit.to_bits())))
                }).collect::<Vec<_>>()
            });
            parts.push((account.id.clone(), rings, result.and_then(|r| r.as_ref().err().cloned()), self.quota.tail_text(&account.id), self.quota.stale.contains_key(&account.id)));
        }
        Some(hash(&parts))
    }

    fn focus_tray_fingerprint(&self) -> Option<u64> {
        (self.settings.show_focus && self.pomo.running).then(|| hash(&(self.pomo.phase == crate::app::Phase::Focus,)))
    }

    fn mail_tray_fingerprint(&self) -> Option<u64> {
        if !self.settings.show_mail || !self.module_on(crate::app::MAIL_PAGE) { return None; }
        let count = self.mail_unread_count();
        (count > 0).then(|| hash(&count))
    }

    fn health_tray_fingerprint(&self) -> Option<u64> {
        if !self.settings.show_health || !self.module_on(crate::app::WATER_PAGE) { return None; }
        let badges = self.health_badges();
        (!badges.is_empty()).then(|| hash(&badges.iter().map(|kind| kind.label()).collect::<Vec<_>>()))
    }

    fn todo_tray_fingerprint(&self) -> Option<u64> {
        if !self.settings.show_todo || !self.module_on(crate::app::TODO_PAGE) { return None; }
        let count = self.todo_due_count();
        (count > 0).then(|| hash(&count))
    }

    fn shelf_tray_fingerprint(&self) -> Option<u64> {
        if !self.settings.show_shelf || !self.module_on(crate::app::SHELF_PAGE) || self.t >= self.shelf_badge_until { return None; }
        Some(hash(&self.shelf_badge_until.to_bits()))
    }

    fn calendar_tray_fingerprint(&self) -> Option<u64> {
        if !self.settings.show_calendar || !self.module_on(crate::app::CALENDAR_PAGE) || self.t >= self.calendar_badge_until { return None; }
        Some(hash(&self.calendar_badge_until.to_bits()))
    }

    fn music_tray_fingerprint(&self) -> Option<u64> {
        (self.settings.show_music && self.media.is_music()).then(|| {
            hash(&(self.media.title.clone(), self.media.artist.clone(), self.media.playing, self.media.source.clone()))
        })
    }

    fn battery_tray_fingerprint(&self) -> Option<u64> {
        (self.settings.show_battery && self.resident_power.0.is_some()).then(|| hash(&(self.resident_power.0, self.resident_power.1)))
    }

    fn tray_fingerprint(&self, item: Item) -> Option<u64> {
        match item {
            Item::Weather => self.weather_tray_fingerprint(),
            Item::Quota => self.quota_tray_fingerprint(),
            Item::Focus => self.focus_tray_fingerprint(),
            Item::Mail => self.mail_tray_fingerprint(),
            Item::Health => self.health_tray_fingerprint(),
            Item::Todo => self.todo_tray_fingerprint(),
            Item::Shelf => self.shelf_tray_fingerprint(),
            Item::Calendar => self.calendar_tray_fingerprint(),
            Item::Music => self.music_tray_fingerprint(),
            Item::Battery => self.battery_tray_fingerprint(),
        }
    }

    fn tray_member(&self, item: Item) -> bool {
        match item {
            Item::Weather => self.settings.tray_weather,
            Item::Quota => self.settings.tray_quota,
            Item::Focus => self.settings.tray_focus,
            Item::Mail => self.settings.tray_mail,
            Item::Health => self.settings.tray_health,
            Item::Todo => self.settings.tray_todo,
            Item::Shelf => self.settings.tray_shelf,
            Item::Calendar => self.settings.tray_calendar,
            Item::Music => self.settings.tray_music,
            Item::Battery => self.settings.tray_battery,
        }
    }

    /// 在通知队列处理之后调用：记录显示值变化，必要时点亮事件容器。
    pub(crate) fn tray_tick(&mut self) {
        for item in Item::ALL {
            let index = item as usize;
            let value = self.tray_fingerprint(item);
            let member = self.tray_member(item);
            let possible = value.is_some();
            let changed = self.event_tray.primed && member && possible
                && (!self.event_tray.member[index] || self.event_tray.seen[index] != value);
            if changed {
                if self.notice.is_none() {
                    self.event_tray.last[index] = self.t;
                    self.event_tray.pending[index] = false;
                } else {
                    self.event_tray.pending[index] = true;
                }
            }
            if self.notice.is_none() && self.event_tray.pending[index] {
                self.event_tray.last[index] = self.t;
                self.event_tray.pending[index] = false;
            }
            self.event_tray.seen[index] = value;
            self.event_tray.member[index] = member;
        }
        self.event_tray.primed = true;
    }

    fn tray_possible(&self, item: Item) -> bool {
        self.tray_member(item) && self.tray_fingerprint(item).is_some()
    }

    /// 容器成员的显示顺序；预览里拖动调整。
    pub(crate) fn tray_order(&self) -> Vec<Item> {
        let order = if bar_layout::valid_tray_order(&self.settings.tray_order) {
            self.settings.tray_order.clone()
        } else {
            bar_layout::TRAY_ORDER.to_vec()
        };
        order.into_iter().filter_map(Item::from_slot).collect()
    }

    /// 当前应显示的成员，按容器顺序排列；多个事件重叠时同时显示。
    pub(crate) fn tray_active_items(&self) -> Vec<Item> {
        self.tray_order().into_iter().filter(|item| {
            if !self.tray_possible(*item) { return false; }
            // 需要用户处理的事件持续到条件解除，不使用 10 秒窗口。
            matches!(item, Item::Mail | Item::Health | Item::Todo | Item::Calendar)
                || self.t - self.event_tray.last[*item as usize] <= HOLD
        }).collect()
    }

    /// 容器里排在最前的活跃成员；仅用于可见性判断和测试。
    pub(crate) fn tray_active(&self) -> Option<Item> {
        self.tray_active_items().first().copied()
    }

    /// 状态行是否给事件容器留槽位；收起和展开共用同一套位置。
    pub(crate) fn tray_visible(&self) -> bool {
        !self.tray_active_items().is_empty()
    }

    /// 成员在容器里的自然宽度。
    pub(crate) fn tray_item_width(&self, item: Item, ctx: &egui::Context) -> f32 {
        match item {
            Item::Weather => bar_layout::WEATHER_WIDTH + self.weather_pill_extra(ctx).unwrap_or(0.0).max(0.0),
            Item::Quota => self.quota_tray_width(ctx),
            Item::Focus => crate::app::focus_bar::WIDTH,
            Item::Mail => crate::app::page_badges::badge_width(bar_layout::MAIL),
            Item::Health => crate::app::page_badges::badge_width(bar_layout::HEALTH),
            Item::Todo => crate::app::page_badges::badge_width(bar_layout::TODO),
            Item::Shelf => crate::app::page_badges::badge_width(bar_layout::SHELF),
            Item::Calendar => crate::app::page_badges::badge_width(bar_layout::CALENDAR),
            Item::Music => bar_layout::slot_width(bar_layout::MUSIC),
            Item::Battery => bar_layout::slot_width(bar_layout::BATTERY),
        }
    }

    /// 容器宽度是所有活跃成员并排所需的宽度。
    pub(crate) fn tray_width(&self, ctx: &egui::Context) -> Option<f32> {
        let items = self.tray_active_items();
        if items.is_empty() { return None; }
        let content: f32 = items.iter().map(|item| self.tray_item_width(*item, ctx)).sum::<f32>()
            + ITEM_GAP * (items.len() - 1) as f32;
        (content > 0.0).then_some(content)
    }

    fn tray_item_alpha(&self, item: Item) -> f32 {
        let age = self.t - self.event_tray.last[item as usize];
        let fade_in = crate::render::anim::smoothstep(0.0, 0.15, age as f32);
        // 需要用户处理的事件持续到条件解除，不受 10 秒窗口的淡出影响。
        if matches!(item, Item::Mail | Item::Health | Item::Todo | Item::Calendar) {
            fade_in
        } else {
            fade_in * crate::render::anim::smoothstep(0.0, 0.3, (HOLD - age) as f32)
        }
    }

    /// 按容器顺序并排画所有活跃成员；命中行为与常驻时一致。
    pub(crate) fn draw_tray(&mut self, c: &Card, x: f32, layout: &bar_layout::Layout) {
        let items = self.tray_active_items();
        if items.is_empty() { return; }
        let clearance = layout.pet_clearance(bar_layout::TRAY);
        let mut left = x - layout.widths[bar_layout::TRAY] / 2.0;
        for item in items {
            let width = self.tray_item_width(item, c.ui.ctx());
            let item_x = left + width / 2.0;
            let alpha = self.tray_item_alpha(item) * clearance;
            if alpha > 0.004 {
                match item {
                    Item::Weather => self.draw_tray_weather(c, item_x, width, alpha),
                    Item::Quota => {
                        let faded = c.faded(alpha);
                        let faded = if alpha < 0.8 { faded.ghost() } else { faded };
                        self.draw_quota_tray(&faded, item_x);
                    }
                    Item::Focus => {
                        let faded = c.faded(alpha);
                        let faded = if alpha < 0.8 { faded.ghost() } else { faded };
                        crate::app::focus_bar::draw(&faded, item_x, 15.0, self.t, &self.pomo, self.phase_secs(self.pomo.phase), false);
                        if crate::quota::ui::bar_tooltip(c.hit(item_x - width / 2.0, 1.0, width, 28.0, "tray-focus"), "专注 · 点击打开".into()).clicked() {
                            self.show_page(crate::app::FOCUS_PAGE);
                        }
                    }
                    Item::Music => {
                        let faded=c.faded(alpha);
                        crate::render::paint::playback_bars_box(&faded, item_x, 15.0, self.t, self.media.playing, theme::WHITE, bar_layout::GLYPH, bar_layout::MUSIC_H);
                        if crate::quota::ui::bar_tooltip(c.hit(item_x - width / 2.0, 1.0, width, 28.0, "tray-music"), "音乐 · 点击打开".into()).clicked() {
                            self.show_page(crate::app::MUSIC_PAGE);
                        }
                    }
                    Item::Battery => {
                        crate::render::paint::battery(&c.faded(alpha), item_x, 15.0, self.resident_power.0, self.resident_power.1, bar_layout::BATTERY_SCALE);
                        let level = self.resident_power.0.map(|pct| format!("{pct}%")).unwrap_or_else(|| "电量不可用".into());
                        crate::quota::ui::bar_tooltip(c.hit(item_x - width / 2.0, 1.0, width, 28.0, "tray-battery"), format!("电量 · {level} · 只读"));
                    }
                    Item::Mail | Item::Health | Item::Todo | Item::Shelf | Item::Calendar => self.draw_tray_badge(c, item_x, item, alpha),
                }
            }
            left += width + ITEM_GAP;
        }
    }

    /// 事件容器里的页面徽标：点击进入对应功能页。
    fn draw_tray_badge(&mut self, c: &Card, x: f32, item: Item, alpha: f32) {
        let slot = match item {
            Item::Mail => bar_layout::MAIL,
            Item::Health => bar_layout::HEALTH,
            Item::Todo => bar_layout::TODO,
            Item::Shelf => bar_layout::SHELF,
            Item::Calendar => bar_layout::CALENDAR,
            _ => return,
        };
        self.paint_badge(&c.faded(alpha), slot, x, theme::WHITE);
        let width = crate::app::page_badges::badge_width(slot);
        let response = crate::quota::ui::bar_tooltip(c.hit(x - width / 2.0, 1.0, width, 28.0, ("page-badge", slot)), self.badge_label(slot));
        if response.clicked() {
            let page = match item {
                Item::Mail => Some(crate::app::MAIL_PAGE),
                Item::Health => Some(crate::app::WATER_PAGE),
                Item::Todo => Some(crate::app::TODO_PAGE),
                Item::Shelf => Some(crate::app::SHELF_PAGE),
                Item::Calendar => Some(crate::app::CALENDAR_PAGE),
                _ => None,
            };
            if let Some(page) = page { self.show_page(page); }
        }
    }
}
