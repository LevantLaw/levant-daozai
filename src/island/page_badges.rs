//! 页面徽标：邮箱未读、健康提醒、待办到期、置物架拖入、日程提醒、勿扰模式。
use crate::island::{App, CALENDAR_PAGE, MAIL_PAGE, SHELF_PAGE, TODO_PAGE, WATER_PAGE};
use crate::island::layout::bar::{self, Layout};
use crate::visuals::paint::{self, Card};
use crate::visuals::theme;
use eframe::egui::Color32;

pub(crate) const HOLD: f64 = 10.0;
const MAIL_W: f32 = 30.0;
const TODO_W: f32 = 30.0;
const ICON_W: f32 = 20.0;

/// 徽标在岛上的自然宽度；事件容器按它计算占位宽度。
pub(crate) fn badge_width(slot: usize) -> f32 {
    match slot { bar::MAIL => MAIL_W, bar::TODO => TODO_W, _ => ICON_W }
}

#[derive(Clone, Copy)]
pub(crate) struct Badge {
    pub slot: usize,
    pub page: Option<usize>,
    pub width: f32,
}

impl App {
    pub(crate) fn mail_unread_count(&self) -> usize {
        if !self.settings.mail_enabled { return 0; }
        self.mail.recent.iter().filter(|item| self.mail_seen(item, crate::mail::manage::Folder::Inbox) != Some(true)).count()
    }

    /// 已到期且未完成的待办；重复待办只看当天的时刻。
    pub(crate) fn todo_due_count(&self) -> usize {
        let now = format!("{} {}", self.today, self.current_clock);
        self.todos.items.iter().filter(|item| {
            if !item.scheduled_on(&self.today) || item.done_for(&self.today) { return false; }
            let Some(due) = item.due.as_deref() else { return false; };
            if item.is_repeating() { due.get(11..16).is_some_and(|at| at <= now.get(11..16).unwrap_or("")) } else { due <= now.as_str() }
        }).count()
    }

    /// 当前应该出现在顶部胶囊里的页面徽标；事件容器成员不在行内占位。
    pub(crate) fn page_badges(&self) -> Vec<Badge> {
        self.page_badges_all().into_iter().filter(|badge| !self.badge_in_tray(badge.slot)).collect()
    }

    /// 不看事件容器成员关系的徽标列表；侧栏停靠和命中判断使用。
    pub(crate) fn page_badges_all(&self) -> Vec<Badge> {
        let mut badges = Vec::new();
        if self.settings.show_mail && self.module_on(MAIL_PAGE) && self.mail_unread_count() > 0 {
            badges.push(Badge { slot: bar::MAIL, page: Some(MAIL_PAGE), width: MAIL_W });
        }
        if self.settings.show_health && self.module_on(WATER_PAGE) && !self.health_badges().is_empty() {
            badges.push(Badge { slot: bar::HEALTH, page: Some(WATER_PAGE), width: ICON_W });
        }
        if self.settings.show_todo && self.module_on(TODO_PAGE) && self.todo_due_count() > 0 {
            badges.push(Badge { slot: bar::TODO, page: Some(TODO_PAGE), width: TODO_W });
        }
        if self.settings.show_shelf && self.module_on(SHELF_PAGE) && self.t < self.shelf_badge_until {
            badges.push(Badge { slot: bar::SHELF, page: Some(SHELF_PAGE), width: ICON_W });
        }
        if self.settings.show_calendar && self.module_on(CALENDAR_PAGE) && self.t < self.calendar_badge_until {
            badges.push(Badge { slot: bar::CALENDAR, page: Some(CALENDAR_PAGE), width: ICON_W });
        }
        if self.settings.show_dnd && self.dnd.active {
            badges.push(Badge { slot: bar::DND, page: None, width: ICON_W });
        }
        badges
    }

    fn badge_in_tray(&self, slot: usize) -> bool {
        match slot {
            bar::MAIL => self.settings.tray_mail,
            bar::HEALTH => self.settings.tray_health,
            bar::TODO => self.settings.tray_todo,
            bar::SHELF => self.settings.tray_shelf,
            bar::CALENDAR => self.settings.tray_calendar,
            _ => false,
        }
    }

    /// 岛体预览里的徽标：页面图标已在设置侧栏绘制，不重复画一套；只剩没有页面入口的勿扰月亮。
    pub(crate) fn preview_badges(&self) -> Vec<Badge> {
        if self.settings.show_dnd { vec![Badge { slot: bar::DND, page: None, width: ICON_W }] } else { Vec::new() }
    }

    pub(crate) fn badge_active(&self, slot: usize) -> bool {
        self.page_badges().iter().any(|badge| badge.slot == slot)
    }

    pub(crate) fn badge_label(&self, slot: usize) -> String {
        match slot {
            bar::MAIL => format!("邮箱 · {} 封未读", self.mail_unread_count()),
            bar::HEALTH => format!("健康提醒 · {}", self.health_badges().iter().map(|kind| kind.label()).collect::<Vec<_>>().join("、")),
            bar::TODO => format!("待办 · {} 项已到期", self.todo_due_count()),
            bar::SHELF => "置物架 · 刚收到文件".into(),
            bar::CALENDAR => "日程 · 即将开始".into(),
            bar::DND => "勿扰模式 · 已开启，点击关闭".into(),
            _ => String::new(),
        }
    }

    pub(crate) fn draw_page_badges(&mut self, c: &Card, cx: f32, layout: &Layout) {
        for badge in self.page_badges() {
            let Some(x) = layout.x[badge.slot] else { continue; };
            let x = cx + x;
            let label = self.badge_label(badge.slot);
            let response = crate::quota::ui::bar_tooltip(c.hit(x - badge.width / 2.0, 1.0, badge.width, 28.0, ("page-badge", badge.slot)), label);
            self.paint_badge(c, badge.slot, x, theme::WHITE);
            if response.clicked() {
                match badge.page {
                    Some(page) => self.show_page(page),
                    None => self.dnd.toggle(self.t),
                }
            }
        }
    }

    /// 徽标图标；邮箱和待办在图标右侧带数量。
    pub(crate) fn paint_badge(&self, c: &Card, slot: usize, x: f32, color: Color32) {
        match slot {
            bar::MAIL => {
                paint::envelope(c, x - 5.0, 15.0, bar::GLYPH, color);
                c.centered_text(x + 7.0, 15.0, &self.mail_unread_count().to_string(), 9.5, crate::visuals::theme::W::Semibold, color, crate::visuals::paint::H::Center);
            }
            bar::HEALTH => {
                if let Some(kind) = self.health_badges().first() { kind.icon(c, x, 15.0, 0.68); }
                else { crate::visuals::paint::droplet(c, x, 15.0, bar::GLYPH, color); }
            }
            bar::TODO => {
                paint::checklist(c, x - 5.0, 15.0, bar::GLYPH, color, Color32::TRANSPARENT);
                c.centered_text(x + 7.0, 15.0, &self.todo_due_count().to_string(), 9.5, crate::visuals::theme::W::Semibold, color, crate::visuals::paint::H::Center);
            }
            // 托盘前唇和日历顶部挂钩让墨迹上下不对称，按视觉中线微调回 15 点。
            bar::SHELF => paint::tray(c, x, 14.2, bar::GLYPH, 0.0, color, theme::GRAY),
            bar::CALENDAR => paint::calendar(c, x, 15.5, bar::GLYPH, color),
            // 月亮外圆比同尺寸图标大，按统一图标尺寸折算。
            bar::DND => paint::moon(c, x, 15.0, bar::MOON_SIZE, color),
            _ => {}
        }
    }
}
