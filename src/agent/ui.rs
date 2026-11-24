//! AI 会话页面、详情子页与设置。

use crate::agent::hooks::Health;
use crate::agent::store::{Decision, Notice, NoticeKind, NotifyConfig, PendingKind, SessionState, Store};
use crate::agent::{AgentId, Session};
use crate::app::{App, AI_PAGE};
use crate::app::extras::{setting_row, setting_row_nested, settings_card, settings_control_row_right, settings_group_fold, settings_hint, settings_label, settings_segmented};
use crate::render::paint::{Card, H};
use crate::render::theme::{self, W};
use eframe::egui::{self, Color32};

/// 横幅对应的 AI 事件身份：会话键 + 事件类型，用于去重和失效判断。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AgentAlert {
    pub key: String,
    pub kind: NoticeKind,
}

impl AgentAlert {
    pub(crate) fn waiting(&self) -> bool {
        matches!(self.kind, NoticeKind::Approval | NoticeKind::Question)
    }
}

/// 会话卡片：3–5 行共用内容区，卡片越高信息行越多。
pub(crate) const AGENT_CARD_GAP: f32 = 6.0;
pub(crate) const AGENT_CARD_AREA: f32 = 190.0;
/// 列表首张卡片与页头的间距，以及列表底部给翻页圆点留出的空间。
pub(crate) const AGENT_LIST_TOP: f32 = 54.0;
const AGENT_LIST_BOTTOM: f32 = 18.0;
/// 详情页选项行：序号在左，胶囊从固定位置开始并随文字长度伸展。
const OPTION_TOP: f32 = 102.0;
const OPTION_STEP: f32 = 25.0;
const OPTION_X: f32 = 27.0;
const OPTION_W: f32 = 374.0;
const OPTION_H: f32 = 22.0;

fn option_row_y(index: usize) -> f32 { OPTION_TOP + index as f32 * OPTION_STEP }
fn free_row_y(option_count: usize) -> f32 { option_row_y(option_count.min(4)) }
/// 详情页按钮行紧跟在自由输入行之后。
fn question_actions_y(option_count: usize) -> f32 { free_row_y(option_count) + OPTION_H + 8.0 }
/// 提问详情高度随选项数变化，最多 4 个选项加自由输入行。
pub(crate) fn question_height(option_count: usize) -> f32 { question_actions_y(option_count) + 32.0 }

/// 审批详情：卡片随实际换行行数（1–2 行）收缩。
const APPROVAL_TOP: f32 = 58.0;
const APPROVAL_DETAIL_TOP: f32 = 84.0;
const APPROVAL_DETAIL_H: f32 = 38.0;
const APPROVAL_LINE_STEP: f32 = 14.0;

fn approval_detail_height(lines: usize) -> f32 { APPROVAL_DETAIL_H + (lines.clamp(1, 2) as f32 - 1.0) * APPROVAL_LINE_STEP }
fn approval_actions_y(lines: usize) -> f32 { APPROVAL_DETAIL_TOP + approval_detail_height(lines) + 32.0 }
pub(crate) fn approval_height(lines: usize) -> f32 { approval_actions_y(lines) + 32.0 }

/// 提醒事件的二级分组：与「岛体 → 岛上显示」的常驻 / 事件分组一致，子项缩进一级。
fn notify_group(ui: &mut egui::Ui, id: &str, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    settings_group_fold(ui, id, title, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                add(ui);
            });
        });
    });
}

pub(crate) fn agent_card_height(rows: usize) -> f32 {
    let rows = rows.clamp(1, 5) as f32;
    ((AGENT_CARD_AREA - (rows - 1.0) * AGENT_CARD_GAP) / rows).min(44.0)
}

/// 列表实际显示的卡片数：受会话数和「会话数」设置共同限制。
pub(crate) fn agent_visible_rows(count: usize, rows: usize) -> usize {
    if count == 0 { 0 } else { count.min(rows.clamp(3, 5)) }
}

/// 卡片区域高度，不含页头和底部留白。
pub(crate) fn agent_card_area(count: usize, rows: usize) -> f32 {
    let visible = agent_visible_rows(count, rows);
    if visible == 0 { return 0.0; }
    visible as f32 * agent_card_height(visible) + visible.saturating_sub(1) as f32 * AGENT_CARD_GAP
}

/// 列表最多能滚动的距离；卡片数不超过显示行数时为 0。
pub(crate) fn agent_scroll_max(count: usize, rows: usize) -> f32 {
    let visible = agent_visible_rows(count, rows);
    if visible == 0 { return 0.0; }
    let card = agent_card_height(visible);
    let content = count as f32 * card + count.saturating_sub(1) as f32 * AGENT_CARD_GAP;
    (content - agent_card_area(count, rows)).max(0.0)
}

pub(crate) fn agent_list_height(count: usize, rows: usize) -> f32 {
    if count == 0 { return 150.0; }
    AGENT_LIST_TOP + agent_card_area(count, rows) + AGENT_LIST_BOTTOM
}

impl App {
    pub(crate) fn agent_tick(&mut self, ctx: &egui::Context) {
        if !self.module_on(AI_PAGE) || !self.agent.config.enabled { return; }
        let updates = self.agent.poll(self.t);
        let sound_allowed = self.agent_sound_allowed();
        let mut notices = Vec::new();
        for update in updates {
            if let Some(sound) = update.sound {
                if sound_allowed { crate::agent::sound::play(sound, &self.agent.config.sound); }
            }
            if let Some(notice) = update.notice { notices.push(notice); }
        }
        if self.settings.notifications { self.agent_notices(notices); }
        let waiting = self.agent.store.waiting_count();
        if self.agent.detail || waiting > 0 || (self.open && self.page == AI_PAGE && !self.agent.store.visible().is_empty()) {
            // 等待状态点有呼吸效果；其余计时每秒刷新。
            let millis = if waiting > 0 { 120 } else { 1000 };
            ctx.request_repaint_after(std::time::Duration::from_millis(millis));
        }
    }

    /// 处理本轮 AI 通知：等待类进详情，完成/失败/中断进列表；闲置只更新状态灯。
    fn agent_notices(&mut self, notices: Vec<Notice>) {
        // 等待类优先，避免同批完成事件覆盖待处理详情。
        let (waiting, finished): (Vec<_>, Vec<_>) = notices.into_iter()
            .partition(|notice| matches!(notice.kind, NoticeKind::Approval | NoticeKind::Question));
        for notice in waiting.into_iter().chain(finished) {
            if !self.agent.config.notify.enabled(notice.kind) { continue; }
            let alert = AgentAlert { key: notice.key, kind: notice.kind };
            let pending = self.agent.detail
                && self.agent.selected_session().is_some_and(|session| session.pending.is_some());
            // 输入、其他通知或已有待处理详情时排队。
            let busy = self.is_editing() || self.notice.is_some() || pending;
            if busy {
                // 排队时等待类优先，完成类不覆盖排队中的等待事件。
                let keep_waiting = self.agent_open_after_notice.as_ref()
                    .is_some_and(|queued| queued.waiting() && !alert.waiting());
                if !keep_waiting { self.agent_open_after_notice = Some(alert); }
            } else {
                self.agent_open_event(alert);
            }
        }
    }

    /// 输入、其他通知或待处理详情结束后，打开排队中的 AI 事件。
    pub(crate) fn agent_after_notice(&mut self) {
        if !self.settings.notifications || !self.module_on(AI_PAGE) {
            self.agent_open_after_notice = None;
            return;
        }
        if self.notice.is_some() || self.is_editing() { return; }
        let Some(alert) = self.agent_open_after_notice.clone() else { return; };
        let pending = self.agent.detail
            && self.agent.selected_session().is_some_and(|session| session.pending.is_some());
        // 待处理详情不抢占，等用户答完或返回后再展开下一条。
        if pending { return; }
        self.agent_open_after_notice = None;
        if !crate::agent::ui::agent_alert_valid(&self.agent.store, &self.agent.config.notify, &alert) { return; }
        self.agent_open_event(alert);
    }

    /// 进入 AI 页：等待类打开处理详情，完成类打开会话列表。
    pub(crate) fn agent_open_event(&mut self, alert: AgentAlert) {
        if alert.waiting() {
            self.agent.answers = self.agent.selected_session_questions(&alert.key).into_iter().map(|_| Vec::new()).collect();
            self.agent.question_index = 0;
            self.agent.selected = Some(alert.key);
            self.agent.detail = true;
            // 新详情先按两行留足高度，绘制后再按实际行数收缩。
            self.agent.approval_lines = 2;
        } else {
            self.agent.detail = false;
            self.agent.selected = None;
        }
        self.show_page(AI_PAGE);
    }

    /// AI 提示音跟随提醒总开关和声音开关；状态灯隐藏时不再单独出声。
    pub(crate) fn agent_sound_allowed(&self) -> bool {
        self.settings.notifications && self.settings.sound && self.agent_bar_visible()
    }

    /// 等待回答或审批的详情保持展开，等用户在岛上直接处理。
    pub(crate) fn agent_waiting_pinned(&self) -> bool {
        self.agent.detail
            && self.agent.selected_session().is_some_and(|session| {
                session.pending.as_ref().is_some_and(|pending| matches!(pending.kind, PendingKind::Question(_) | PendingKind::Approval(_)))
            })
    }

    /// 模块和开关开启时显示槽位；没有活跃会话时画暗色圆环，让开关有可见反馈。
    pub(crate) fn agent_bar_visible(&self) -> bool {
        self.agent.config.enabled && self.agent.config.display.bar_indicator && self.module_on(AI_PAGE)
    }

    /// 常驻槽：一个状态灯，运行=蓝、等待=橙（呼吸）、可能中断=黄。
    pub(crate) fn draw_agent_light(&self, c: &Card, x: f32, y: f32) {
        let active = self.agent.store.active();
        if active.is_empty() {
            c.ring(x, y, 4.3, 1.5, crate::render::paint::alpha(theme::GRAY, 0.55));
            return;
        }
        let waiting = active.iter().filter(|session| session.waiting()).count();
        let stale = active.iter().any(|session| session.stale());
        let breath = 0.55 + 0.45 * ((self.t * 4.0).sin().abs() as f32);
        let (color, alpha) = if stale {
            (theme::YELLOW, 1.0)
        } else if waiting > 0 {
            (Color32::from_rgb(255, 166, 77), breath)
        } else {
            (theme::BLUE, 1.0)
        };
        if waiting > 0 && !stale {
            c.circle(x, y, 8.0, Color32::from_rgba_unmultiplied(255, 166, 77, (breath * 44.0) as u8));
        }
        c.circle(x, y, 4.5, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (alpha * 255.0) as u8));
    }

    /// 详情页自由输入框的位置：每个待回答的问题都留一行。
    pub(crate) fn agent_free_row(&self) -> Option<(usize, f32)> {
        if !self.agent.detail { return None; }
        let pending = self.agent.selected_session()?.pending.as_ref()?;
        let PendingKind::Question(questions) = &pending.kind else { return None; };
        let index = self.agent.question_index.min(questions.len().checked_sub(1)?);
        Some((index, free_row_y(questions[index].options.len())))
    }

    /// 岛内自由输入：真正的 egui 文本框，叠在详情页固定的输入行上。
    pub(crate) fn agent_free_input(&mut self, ui: &mut egui::Ui, origin: egui::Pos2, cmds: &mut Vec<crate::app::Cmd>) {
        let Some((index, y)) = self.agent_free_row() else { self.agent.free_focus = false; return; };
        let rect = egui::Rect::from_min_size(origin + egui::Vec2::new(OPTION_X + 8.0, y + 2.0), egui::Vec2::new(OPTION_W - 16.0, OPTION_H - 4.0));
        let mut buf = std::mem::take(&mut self.agent.free);
        let editor = egui::TextEdit::singleline(&mut buf)
            .id(egui::Id::new(("agent-free", index)))
            .frame(false)
            .margin(egui::Margin::ZERO)
            .text_color(theme::WHITE)
            .font(theme::font(10.0, W::Regular))
            .hint_text(egui::RichText::new("输入其他答案，回车完成").color(theme::DIM).font(theme::font(10.0, W::Regular)))
            .desired_width(rect.width());
        let response = ui.put(rect, editor);
        if response.gained_focus() { cmds.push(crate::app::Cmd::Focus); }
        self.agent.free_focus = response.has_focus();
        // 回车只结束输入，文字留在行内；真正提交仍由「提交」按钮完成。
        self.agent.free = buf;
    }

    /// 把自由输入并入当前问题的答案；单选取代已选项，多选追加。
    fn agent_commit_free(&mut self, index: usize, multi: bool) {
        let free = std::mem::take(&mut self.agent.free);
        let free = free.trim().to_string();
        if free.is_empty() { return; }
        let entry = &mut self.agent.answers[index];
        if multi {
            if !entry.contains(&free) { entry.push(free); }
        } else {
            entry.clear(); entry.push(free);
        }
    }

    pub(crate) fn page_agent(&mut self, c: &Card) {
        if self.agent.detail {
            if self.agent.selected_session().is_some() {
                self.agent_detail(c);
                return;
            }
            self.agent.detail = false;
            self.agent.selected = None;
        }
        let sessions: Vec<String> = self.agent.store.visible().into_iter().map(|session| session.key.clone()).collect();
        let waiting = self.agent.store.waiting_count();
        let header = if sessions.is_empty() { "暂无会话".to_string() } else { format!("{} 个会话 · {} 个等待你", sessions.len(), waiting) };
        c.text(15.0, 45.0, &header, 10.5, W::Regular, theme::GRAY, H::Left);
        if sessions.is_empty() {
            c.text(209.5, 84.0, "还没有活跃会话", 13.0, W::Bold, theme::WHITE, H::Center);
            c.text(209.5, 102.0, "勾选 Agent 并安装 Hook 后，会话会出现在这里", 10.0, W::Regular, theme::GRAY, H::Center);
            let (response, body) = crate::render::paint::capsule(c, 162.0, 114.0, 94.0, 24.0, "agent-setup", theme::BTN, theme::BTN_HOVER);
            body.text(209.0, 130.0, "去接入 Agent", 10.0, W::Semibold, theme::WHITE, H::Center);
            if response.clicked() { self.settings_tab = 15; self.open_settings(); }
            return;
        }
        let rows = self.agent.config.display.rows;
        let visible = agent_visible_rows(sessions.len(), rows);
        let height = agent_card_height(visible);
        let viewport = agent_card_area(sessions.len(), rows);
        let max_scroll = agent_scroll_max(sessions.len(), rows);
        self.agent.scroll = self.agent.scroll.clamp(0.0, max_scroll);
        let scroll = self.agent.scroll;
        let list = c.clipped(40.0, AGENT_LIST_TOP, 320.0, viewport);
        let mut handled = None;
        let mut jumped = None;
        for (index, key) in sessions.iter().enumerate() {
            let Some(session) = self.agent.store.sessions.iter().find(|session| &session.key == key) else { continue; };
            // 卡片主体直接跳转到会话应用；等待中的卡片右侧留「处理」进入详情。
            let top = AGENT_LIST_TOP + index as f32 * (height + AGENT_CARD_GAP) - scroll;
            if top + height < AGENT_LIST_TOP || top > AGENT_LIST_TOP + viewport { continue; }
            let row = list.hit(40.0, top, 320.0, height, ("agent-row", key));
            let (hover, _) = list.hover_t(&row);
            let waiting = session.waiting();
            let fill = if waiting {
                Color32::from_rgba_unmultiplied(255, 166, 77, 16)
            } else {
                Color32::from_white_alpha((8.0 + 8.0 * hover) as u8)
            };
            list.fill(40.0, top, 320.0, height, 10.0, fill);
            let accent = state_color(session);
            let alpha = if waiting { 0.55 + 0.45 * ((self.t * 4.0).sin() * 0.5 + 0.5) as f32 } else { 0.9 };
            list.fill(47.0, top + (height - 16.0) / 2.0, 3.0, 16.0, 1.5, crate::render::paint::alpha(accent, alpha));
            if waiting || session.stale() || matches!(session.state, SessionState::Failed | SessionState::Interrupted) {
                list.stroke(40.0, top, 320.0, height, 10.0, 1.0, crate::render::paint::alpha(accent, 0.65));
            } else if hover > 0.01 {
                list.stroke(40.0, top, 320.0, height, 10.0, 1.0, Color32::from_white_alpha((20.0 * hover) as u8));
            }
            let title = session.title.as_str();
            let preview = card_preview(session);
            // 运行状态由左侧状态灯表达；时长和子代理数用小图标加数字放在标题行右侧。
            let duration_text = duration(self.t - session.since);
            let subagents = (session.is_active() && !session.stale() && !session.subagents.is_empty())
                .then(|| session.subagents.len().to_string());
            let meta_color = if session.stale() { theme::YELLOW } else { theme::GRAY };
            let stat_size = if height >= 40.0 { 9.5 } else { 9.0 };
            let stat_gap = 8.0;
            let icon_w = 7.0;
            let duration_w = list.text_w(&duration_text, stat_size, W::Regular);
            let subagents_w = subagents.as_ref().map_or(0.0, |count| stat_gap + icon_w + 2.0 + list.text_w(count, stat_size, W::Regular));
            let stats_x = 310.0 - (icon_w + 2.0 + duration_w + subagents_w);
            let base = if height >= 40.0 { top + height * 0.34 } else { top + 15.0 };
            if !waiting {
                let stat_cy = base - 3.3;
                clock_glyph(&list, stats_x + icon_w / 2.0, stat_cy, meta_color);
                list.text(stats_x + icon_w + 2.0, base, &duration_text, stat_size, W::Regular, meta_color, H::Left);
                if let Some(count) = &subagents {
                    let x = stats_x + icon_w + 2.0 + duration_w + stat_gap;
                    subagent_glyph(&list, x + icon_w / 2.0, stat_cy, meta_color);
                    list.text(x + icon_w + 2.0, base, count, stat_size, W::Regular, meta_color, H::Left);
                }
            }
            let title_w = if waiting { 231.0 } else { (stats_x - 6.0 - 56.0).max(60.0) };
            let preview_w = if waiting { 230.0 } else { 250.0 };
            if height >= 40.0 {
                list.text(56.0, base, &list.fit(title, 11.5, W::Semibold, title_w), 11.5, W::Semibold, theme::WHITE, H::Left);
                if !preview.is_empty() {
                    list.text(56.0, top + height * 0.66, &list.fit(&preview, 9.5, W::Regular, preview_w), 9.5, W::Regular, theme::GRAY, H::Left);
                }
            } else {
                list.text(56.0, base, &list.fit(title, 11.0, W::Semibold, title_w), 11.0, W::Semibold, theme::WHITE, H::Left);
                if !preview.is_empty() {
                    list.text(56.0, top + 28.0, &list.fit(&preview, 9.0, W::Regular, preview_w), 9.0, W::Regular, theme::GRAY, H::Left);
                }
            }
            if waiting {
                agent_icon(&list, session.agent, 300.0, top + height / 2.0, 14.0);
                let handle_id = format!("agent-handle-{key}");
                let (handle, body) = crate::render::paint::capsule(&list, 312.0, top + (height - 18.0) / 2.0, 44.0, 18.0, &handle_id, theme::BTN, theme::BTN_HOVER);
                body.centered_text(334.0, top + height / 2.0, "处理", 9.0, W::Semibold, theme::WHITE, H::Center);
                if handle.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("在岛上处理").clicked() {
                    handled = Some(key.clone());
                } else if row.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    jumped = Some(session.hosts.clone());
                }
            } else {
                agent_icon(&list, session.agent, 340.0, top + height / 2.0, 14.0);
                if row.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("跳转到会话应用").clicked() {
                    jumped = Some(session.hosts.clone());
                }
            }
        }
        if max_scroll > 0.0 {
            // 右侧 3px 滚动条；拖动或滚轮都能翻看超出的会话。
            let track_x = 363.0;
            let thumb_h = (viewport / (viewport + max_scroll) * viewport).max(18.0);
            let thumb_y = AGENT_LIST_TOP + (scroll / max_scroll) * (viewport - thumb_h);
            c.fill(track_x, AGENT_LIST_TOP, 3.0, viewport, 1.5, Color32::from_white_alpha(14));
            c.fill(track_x, thumb_y, 3.0, thumb_h, 1.5, Color32::from_white_alpha(64));
            let drag = c.hit_drag(track_x - 3.0, AGENT_LIST_TOP, 9.0, viewport, "agent-scrollbar");
            if drag.dragged() {
                if let Some(p) = drag.interact_pointer_pos() {
                    let local = c.to_local(p);
                    let ratio = ((local.y - AGENT_LIST_TOP) / viewport).clamp(0.0, 1.0);
                    self.agent.scroll = ratio * max_scroll;
                }
            }
        }
        if let Some(key) = handled {
            self.agent.answers = self.agent.selected_session_questions(&key).into_iter().map(|_| Vec::new()).collect();
            self.agent.question_index = 0;
            self.agent.selected = Some(key);
            self.agent.detail = true;
            self.agent.approval_lines = 2;
            return;
        }
        if let Some(hosts) = jumped {
            if !crate::agent::focus::focus(&hosts) { self.say("没找到窗口"); }
        }
    }

    fn agent_detail(&mut self, c: &Card) {
        let Some(session) = self.agent.selected_session() else { self.agent.detail = false; return; };
        let key = session.key.clone();
        let agent = session.agent;
        let title = session.title.clone();
        let hosts = session.hosts.clone();
        let approval = match session.pending.as_ref().map(|pending| &pending.kind) {
            Some(crate::agent::store::PendingKind::Approval(request)) => Some((request.tool.clone(), request.detail.clone())),
            _ => None,
        };
        let questions = session.pending.as_ref().and_then(|pending| match &pending.kind {
            crate::agent::store::PendingKind::Question(questions) => Some(questions.clone()),
            _ => None,
        }).unwrap_or_default();
        // 待处理事件已消失（例如在终端处理完），详情没有可展示的内容，退回列表。
        if approval.is_none() && questions.is_empty() {
            self.agent.detail = false;
            self.agent.selected = None;
            return;
        }
        if self.agent.answers.len() < questions.len() { self.agent.answers.resize(questions.len(), Vec::new()); }

        agent_icon(c, agent, 22.0, 45.0, 13.0);
        c.text(36.0, 45.0, &c.fit(&title, 11.0, W::Semibold, 250.0), 11.0, W::Semibold, theme::WHITE, H::Left);
        let jump = c.hit(300.0, 34.0, 64.0, 18.0, "agent-jump");
        c.text(332.0, 47.0, "跳转", 10.0, W::Regular, theme::GRAY, H::Center);
        if jump.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            if !crate::agent::focus::focus(&hosts) { self.say("没找到窗口"); }
        }
        let back = c.hit(372.0, 34.0, 34.0, 18.0, "agent-back");
        c.text(389.0, 47.0, "返回", 10.0, W::Regular, theme::GRAY, H::Center);
        if back.clicked() { self.agent.detail = false; self.agent.selected = None; return; }

        if let Some((tool, detail)) = approval {
            let lines = c.wrap(&detail, 10.0, W::Regular, 354.0, 2);
            self.agent.approval_lines = lines.len().clamp(1, 2);
            let detail_h = approval_detail_height(lines.len());
            let actions_y = approval_actions_y(lines.len());
            c.fill(15.0, APPROVAL_TOP, 386.0, detail_h + 48.0, 10.0, Color32::from_white_alpha(15));
            c.stroke(15.0, APPROVAL_TOP, 386.0, detail_h + 48.0, 10.0, 1.0, Color32::from_white_alpha(31));
            c.text(23.0, 76.0, &c.fit(&format!("请求使用 {tool}"), 11.5, W::Bold, 370.0), 11.5, W::Bold, Color32::from_rgb(255, 176, 86), H::Left);
            c.fill(23.0, APPROVAL_DETAIL_TOP, 370.0, detail_h, 7.0, Color32::from_white_alpha(18));
            for (index, line) in lines.iter().enumerate() { c.text(30.0, APPROVAL_DETAIL_TOP + 16.0 + index as f32 * APPROVAL_LINE_STEP, line, 10.0, W::Regular, theme::GRAY, H::Left); }
            c.hit(23.0, APPROVAL_DETAIL_TOP, 370.0, detail_h, "agent-approval-detail").on_hover_text(&detail);
            if button(c, 15.0, actions_y, 94.0, "到终端处理", theme::BTN).clicked() {
                self.agent.respond(&key, Decision::Terminal, self.t);
                if !crate::agent::focus::focus(&hosts) { self.say("没找到窗口"); }
                return;
            }
            if button(c, 195.0, actions_y, 58.0, "拒绝", theme::RED).clicked() { self.agent.respond(&key, Decision::Deny, self.t); return; }
            if button(c, 259.0, actions_y, 72.0, "总是允许", theme::BLUE).clicked() { self.agent.respond(&key, Decision::Always, self.t); return; }
            if button(c, 337.0, actions_y, 64.0, "允许一次", theme::GREEN).clicked() { self.agent.respond(&key, Decision::Allow, self.t); return; }
            return;
        }

        let index = self.agent.question_index.min(questions.len() - 1);
        let question = &questions[index];
        c.text(15.0, 62.0, &format!("问题 {}/{}", index + 1, questions.len()), 10.0, W::Semibold, theme::BLUE, H::Left);
        if !question.header.is_empty() {
            c.text(72.0, 62.0, &c.fit(&question.header, 10.0, W::Regular, 230.0), 10.0, W::Regular, theme::GRAY, H::Left);
        }
        if question.multi { c.centered_text(401.0, 62.0, "多选", 9.0, W::Regular, theme::BLUE, H::Right); }
        let lines = c.wrap(&question.question, 11.5, W::Semibold, 386.0, 2);
        for (line_index, line) in lines.iter().enumerate() { c.text(15.0, 80.0 + line_index as f32 * 14.0, line, 11.5, W::Semibold, theme::WHITE, H::Left); }
        let selected = self.agent.answers.get(index).cloned().unwrap_or_default();
        let mut clicked_option = None;
        // 选项一行一个：序号在左，胶囊与序号保持固定间距并左对齐；说明放悬停提示。
        for (option_index, option) in question.options.iter().take(4).enumerate() {
            let y = option_row_y(option_index);
            let on = selected.iter().any(|label| label == &option.label);
            let width = (c.text_w(&option.label, 10.0, W::Semibold) + 20.0).clamp(44.0, OPTION_W);
            let response = c.hit(15.0, y, 386.0, OPTION_H, ("agent-option", index, option_index));
            let (hover, _) = c.hover_t(&response);
            let fill = if on { Color32::from_rgba_unmultiplied(62, 172, 248, 36) } else { Color32::from_white_alpha((hover * 12.0) as u8) };
            c.fill(OPTION_X, y, width, OPTION_H, OPTION_H / 2.0, fill);
            c.text(15.0, y + 15.0, &(option_index + 1).to_string(), 10.0, W::Semibold, if on { theme::BLUE } else { theme::GRAY }, H::Left);
            if !option.description.is_empty() { response.clone().on_hover_text(&option.description); }
            c.text(OPTION_X + 10.0, y + 15.0, &c.fit(&option.label, 10.0, W::Semibold, width - 20.0), 10.0, W::Semibold, if on { theme::BLUE } else { theme::WHITE }, H::Left);
            if response.clicked() { clicked_option = Some(option.label.clone()); }
        }
        if let Some(label) = clicked_option {
            let entry = &mut self.agent.answers[index];
            if question.multi {
                if let Some(position) = entry.iter().position(|value| value == &label) { entry.remove(position); } else { entry.push(label); }
            } else {
                entry.clear(); entry.push(label);
                // 单选时点选项即取代刚输入的自由答案，避免提交时旧文字又覆盖选择。
                self.agent.free.clear();
            }
        }
        // 自由输入和选项同列，直接在这一行输入。
        let free_y = free_row_y(question.options.len());
        let free_number = question.options.len().min(4) + 1;
        let free_on = self.agent.free_focus || !self.agent.free.is_empty();
        c.fill(OPTION_X, free_y, OPTION_W, OPTION_H, OPTION_H / 2.0, if free_on { Color32::from_rgba_unmultiplied(62, 172, 248, 36) } else { Color32::from_white_alpha(10) });
        c.text(15.0, free_y + 15.0, &free_number.to_string(), 10.0, W::Semibold, if free_on { theme::BLUE } else { theme::GRAY }, H::Left);
        let last = index + 1 >= questions.len();
        let actions_y = question_actions_y(question.options.len());
        if button(c, 15.0, actions_y, 94.0, "到终端处理", theme::BTN).clicked() {
            self.agent.respond(&key, Decision::Terminal, self.t);
            if !crate::agent::focus::focus(&hosts) { self.say("没找到窗口"); }
            return;
        }
        if index > 0 && button(c, 263.0, actions_y, 60.0, "上一题", theme::BTN).clicked() {
            self.agent_commit_free(index, question.multi);
            self.agent.question_index = index - 1;
            self.agent.free_focus = false;
        }
        if button(c, 331.0, actions_y, 70.0, if last { "提交" } else { "下一题" }, if last { theme::GREEN } else { theme::BTN }).clicked() {
            self.agent_commit_free(index, question.multi);
            if last {
                self.agent.respond(&key, Decision::Answers(self.agent.answers.clone()), self.t);
                return;
            }
            self.agent.question_index = index + 1;
            self.agent.free_focus = false;
        }
    }

    pub(crate) fn agent_settings(&mut self, ui: &mut egui::Ui) {
        let statuses = self.agent.status.clone();
        settings_label(ui, "Hooks");
        settings_card(ui, |ui| {
            for (index, agent) in AgentId::ALL.iter().enumerate() {
                if index > 0 { ui.separator(); }
                let status = &statuses[index];
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(agent.display()).strong());
                        let (color, text) = health_text(status.health, &status.note);
                        ui.label(egui::RichText::new(text).size(11.0).color(color));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let supported = !matches!(status.health, Health::Unsupported);
                        if supported {
                            let label = if matches!(status.health, Health::Installed | Health::NeedsRepair) { "移除" } else { "安装" };
                            if ui.button(label).clicked() {
                                let result = if matches!(status.health, Health::Installed | Health::NeedsRepair) { self.agent.remove(*agent) } else { self.agent.install(*agent) };
                                match result { Ok(note) | Err(note) => self.say(note) }
                            }
                            if matches!(status.health, Health::NeedsRepair) {
                                if ui.button("修复").clicked() { match self.agent.install(*agent) { Ok(note) | Err(note) => self.say(note) } }
                            }
                        }
                        let mut config = self.agent.config.agent(*agent);
                        let before = config;
                        ui.add_enabled_ui(supported, |ui| {
                            ui.checkbox(&mut config.approval, "审批");
                            ui.checkbox(&mut config.monitor, "监控");
                        });
                        if config.monitor != before.monitor || config.approval != before.approval {
                            self.agent.config.set_agent(*agent, config);
                            self.agent.config.save();
                        }
                    });
                });
            }
            ui.separator();
            crate::app::extras::settings_action_row(ui,|ui|{
                if ui.add(crate::app::extras::settings_primary_button("重新扫描")).clicked() { self.agent.rescan(); }
                if ui.add(crate::app::extras::settings_danger_button("清理旧 Hook")).clicked() {
                    let changed = crate::agent::hooks::cleanup_legacy();
                    self.agent.rescan();
                    self.say(if changed { "已清理 Fly Island / 提示音 Hook，备份已保留" } else { "没有发现需要清理的旧 Hook" });
                }
            });
            settings_hint(ui, "关闭监控不会删除 Hook；需要干净配置时点「移除」。");
        });

        settings_label(ui, "提醒事件");
        settings_card(ui, |ui| {
            notify_group(ui, "expand", "展开提醒", |ui| {
                let mut question = self.agent.config.notify.question();
                setting_row_nested(ui, "用户选择", "等待回答时直接展开选择页", &mut question);
                if question != self.agent.config.notify.question() { self.agent.config.notify.question = Some(question); self.agent.config.save(); }
                ui.separator();
                let mut approval = self.agent.config.notify.approval();
                setting_row_nested(ui, "审批", "等待审批时直接展开审批页", &mut approval);
                if approval != self.agent.config.notify.approval() { self.agent.config.notify.approval = Some(approval); self.agent.config.save(); }
                ui.separator();
                let mut done = self.agent.config.notify.done;
                setting_row_nested(ui, "完成", "会话完成时直接打开 AI 会话页", &mut done);
                if done != self.agent.config.notify.done { self.agent.config.notify.done = done; self.agent.config.save(); }
                ui.separator();
                let mut interrupted = self.agent.config.notify.interrupted();
                let before = interrupted;
                setting_row_nested(ui, "中断", "会话中断时直接打开 AI 会话页", &mut interrupted);
                if interrupted != before {
                    self.agent.config.notify.interrupted = Some(interrupted);
                    self.agent.config.save();
                }
                ui.separator();
                let mut failed = self.agent.config.notify.failed;
                setting_row_nested(ui, "失败", "会话失败时直接打开 AI 会话页", &mut failed);
                if failed != self.agent.config.notify.failed { self.agent.config.notify.failed = failed; self.agent.config.save(); }
            });
        });

        settings_label(ui, "提示音");
        settings_card(ui, |ui| {
            let mut changed = false;
            {
                let sound = &mut self.agent.config.sound;
                let rows = [
                    ("开始", "Agent 会话开始"),
                    ("提交", "发送新任务"),
                    ("审批", "等待确认或回答"),
                    ("完成", "会话完成"),
                    ("错误", "会话失败"),
                ];
                let mut values = [sound.start, sound.submit, sound.approval, sound.complete, sound.error];
                for (index, (label, description)) in rows.iter().enumerate() {
                    if index > 0 { ui.separator(); }
                    setting_row(ui, label, description, &mut values[index]);
                }
                if values != [sound.start, sound.submit, sound.approval, sound.complete, sound.error] {
                    sound.start = values[0];
                    sound.submit = values[1];
                    sound.approval = values[2];
                    sound.complete = values[3];
                    sound.error = values[4];
                    changed = true;
                }
            }
            if changed { self.agent.config.save(); }
            settings_hint(ui, "按事件选择提示音；关闭的事件不会播放声音。");
        });
    }

    pub(crate) fn agent_reminder_settings(&mut self, ui: &mut egui::Ui) {
        settings_card(ui, |ui| {
            settings_control_row_right(ui, "会话数", |ui| {
                let mut rows = self.agent.config.display.rows;
                if settings_segmented(ui, &mut rows, &[(3, "3 个"), (4, "4 个"), (5, "5 个")], 52.0) {
                    self.agent.config.display.rows = rows;
                    self.agent.config.save();
                }
            });
            ui.separator();
            settings_control_row_right(ui, "会话记录保留", |ui| {
                ui.horizontal(|ui| {
                    let mut value = self.agent.config.retention_minutes;
                    let before = value;
                    if ui.button("−").clicked() { value = if value <= 5 { 1 } else { value - 5 }; }
                    ui.label(format!("{value} 分钟"));
                    if ui.button("+").clicked() { value = if value < 5 { 5 } else { (value + 5).min(60) }; }
                    if value != before { self.agent.config.retention_minutes = value; self.agent.config.save(); }
                });
            });
            settings_hint(ui, "会话到保留时间后清理；事件与提示音在「AI 会话」页设置。");
        });
    }
}

/// 横幅对应的 AI 事件是否仍然成立：事件开关开着，且会话仍处于该状态。
pub(crate) fn agent_alert_valid(store: &Store, notify: &NotifyConfig, alert: &AgentAlert) -> bool {
    if !notify.enabled(alert.kind) { return false; }
    let Some(session) = store.sessions.iter().find(|session| session.key == alert.key) else { return false; };
    match alert.kind {
        NoticeKind::Approval => matches!(session.pending.as_ref().map(|pending| &pending.kind), Some(PendingKind::Approval(_))),
        NoticeKind::Question => matches!(session.pending.as_ref().map(|pending| &pending.kind), Some(PendingKind::Question(_))),
        NoticeKind::Done => session.state == SessionState::Completed,
        NoticeKind::Failed => session.state == SessionState::Failed,
        NoticeKind::Interrupted => session.state == SessionState::Interrupted,
        NoticeKind::Stale => session.stale(),
    }
}

impl crate::agent::AgentState {
    fn selected_session_questions(&self, key: &str) -> Vec<crate::agent::Question> {
        self.store.sessions.iter().find(|session| session.key == key)
            .and_then(|session| session.pending.as_ref())
            .and_then(|pending| match &pending.kind {
                crate::agent::store::PendingKind::Question(questions) => Some(questions.clone()),
                _ => None,
            }).unwrap_or_default()
    }
}

fn button<'a>(c: &Card<'a>, x: f32, y: f32, w: f32, label: &str, color: Color32) -> egui::Response {
    let (response, body) = crate::render::paint::capsule(c, x, y, w, 24.0, label, color, crate::render::paint::mix(color, Color32::WHITE, 0.12));
    body.text(x + w / 2.0, y + 16.0, label, 10.0, W::Semibold, theme::WHITE, H::Center);
    response
}

fn state_color(session: &Session) -> Color32 {
    if session.stale() { return theme::YELLOW; }
    match session.state {
        SessionState::WaitingApproval | SessionState::WaitingAnswer => Color32::from_rgb(255, 166, 77),
        SessionState::Running => theme::BLUE,
        SessionState::Completed => theme::GREEN,
        SessionState::Failed => theme::RED,
        SessionState::Interrupted => theme::YELLOW,
        SessionState::Idle => theme::DIM,
    }
}

/// 卡片副标题：只预览本轮 Stop 后的输出；回答过程中的工具和消息不往卡片上带。
fn card_preview(session: &Session) -> String {
    if session.stale() { return "可能已中断".into(); }
    let text = match session.state {
        SessionState::Completed => session.reply.clone().unwrap_or_default(),
        SessionState::WaitingApproval => "等待你的确认".into(),
        SessionState::WaitingAnswer => "等你回答".into(),
        SessionState::Idle => "等待新任务…".into(),
        SessionState::Failed => session.reply.clone().unwrap_or_else(|| "运行失败".into()),
        SessionState::Interrupted => "已中断".into(),
        SessionState::Running => String::new(),
    };
    text.lines().next().unwrap_or("").to_owned()
}

fn duration(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    if seconds < 60 { format!("{seconds} 秒") }
    else if seconds < 3600 { format!("{} 分钟", seconds / 60) }
    else { format!("{} 小时", seconds / 3600) }
}

/// 标题行右侧的时长图标：外圈加时针分针。
fn clock_glyph(c: &Card, x: f32, y: f32, color: Color32) {
    c.ring(x, y, 3.2, 0.9, color);
    c.seg((x, y), (x, y - 1.8), 0.9, color);
    c.seg((x, y), (x + 1.4, y + 0.6), 0.9, color);
}

/// 子代理图标：主干带两个分支。
fn subagent_glyph(c: &Card, x: f32, y: f32, color: Color32) {
    c.seg((x - 2.4, y - 2.8), (x - 2.4, y + 2.8), 0.9, color);
    c.seg((x - 2.4, y - 0.8), (x + 1.2, y - 2.2), 0.9, color);
    c.seg((x - 2.4, y + 1.2), (x + 1.2, y + 2.4), 0.9, color);
    c.circle(x - 2.4, y - 2.8, 1.0, color);
    c.circle(x + 1.6, y - 2.4, 1.0, color);
    c.circle(x + 1.6, y + 2.6, 1.0, color);
}

fn health_text(health: Health, note: &str) -> (Color32, String) {
    let color = match health {
        Health::Installed => theme::GREEN,
        Health::NeedsRepair => theme::YELLOW,
        Health::NotInstalled => theme::GRAY,
        Health::NotDetected => theme::DIM,
        Health::Unsupported => theme::DIM,
    };
    (color, note.to_string())
}

pub(crate) fn agent_icon(c: &Card, agent: AgentId, x: f32, y: f32, size: f32) {
    let ctx = c.ui.ctx();
    let ppp = ctx.pixels_per_point();
    let pixels = (size * c.s * ppp).round().max(1.0) as u32;
    let key = egui::Id::new(("agent-icon", agent.key(), pixels));
    let texture = ctx.data_mut(|data| data.get_temp::<egui::TextureHandle>(key)).or_else(|| {
        let bytes: &[u8] = match agent {
            AgentId::Claude => include_bytes!("../../assets/ai-icons/claude.svg"),
            AgentId::Codex => include_bytes!("../../assets/ai-icons/codex.svg"),
            AgentId::ZCode => include_bytes!("../../assets/ai-icons/zcode.svg"),
            AgentId::Copilot => include_bytes!("../../assets/ai-icons/copilot.svg"),
            AgentId::Dsh => include_bytes!("../../assets/ai-icons/deepseek.svg"),
        };
        let image = egui_extras::image::load_svg_bytes_with_size(bytes, Some(egui::load::SizeHint::Size(pixels, pixels))).ok()?;
        let texture = ctx.load_texture(format!("agent-icon-{}-{pixels}", agent.key()), image, egui::TextureOptions::LINEAR);
        ctx.data_mut(|data| data.insert_temp(key, texture.clone()));
        Some(texture)
    });
    let Some(texture) = texture else { return; };
    let center = c.pos(x, y);
    let half = pixels as f32 / 2.0;
    let min = egui::pos2((center.x * ppp - half).round() / ppp, (center.y * ppp - half).round() / ppp);
    let rect = egui::Rect::from_min_size(min, egui::Vec2::splat(pixels as f32 / ppp));
    c.painter.image(texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_preview_waits_for_the_completed_hook() {
        let mut session = Session::sample(SessionState::Running);
        session.tool = Some("Bash".into());
        session.detail = Some("cargo test".into());
        assert_eq!(card_preview(&session), "", "运行中不显示过程输出");
        session.state = SessionState::Completed;
        session.reply = Some("已修复\n更多细节".into());
        assert_eq!(card_preview(&session), "已修复", "完成后只取输出首行");
        session.reply = None;
        assert_eq!(card_preview(&session), "", "没有输出时保持空");

        let mut waiting = Session::sample(SessionState::WaitingAnswer);
        assert_eq!(card_preview(&waiting), "等你回答");
        waiting.state = SessionState::WaitingApproval;
        assert_eq!(card_preview(&waiting), "等待你的确认");
    }

    #[test]
    fn card_heights_fit_three_to_five_rows() {
        assert!((agent_card_height(3) - 44.0).abs() < 0.01);
        assert!((agent_card_height(4) - 43.0).abs() < 0.01);
        assert!((agent_card_height(5) - 33.2).abs() < 0.01);
        for rows in 3..=5 {
            let height = agent_list_height(rows, rows);
            assert!(height <= 268.0, "{rows} 行列表不应超过 268px：{height}");
            let cards = rows as f32 * agent_card_height(rows) + (rows as f32 - 1.0) * AGENT_CARD_GAP;
            assert!((AGENT_LIST_TOP + cards + AGENT_LIST_BOTTOM - height).abs() < 0.01);
        }
    }

    #[test]
    fn question_height_follows_option_count() {
        for (options, expected) in [(0, 164.0), (1, 189.0), (2, 214.0), (3, 239.0), (4, 264.0), (6, 264.0)] {
            assert!((question_height(options) - expected).abs() < 0.01, "{options} 个选项的详情高度应为 {expected}");
        }
    }

    #[test]
    fn approval_height_follows_detail_lines() {
        assert!((approval_height(1) - 186.0).abs() < 0.01);
        assert!((approval_height(2) - 200.0).abs() < 0.01);
        assert!((approval_height(5) - 200.0).abs() < 0.01, "超过两行的详情按两行封顶");
    }

    #[test]
    fn list_height_follows_actual_session_count_and_scrolls_after_the_limit() {
        for (count, expected) in [(0, 150.0), (1, 116.0), (2, 166.0), (3, 216.0), (4, 262.0), (5, 262.0)] {
            assert!((agent_list_height(count, 5) - expected).abs() < 0.01, "{count} 个会话的高度应为 {expected}");
        }
        assert!(agent_list_height(4, 3) <= 268.0);
        assert!((agent_scroll_max(3, 5) - 0.0).abs() < 0.01);
        assert!((agent_scroll_max(6, 3) - 150.0).abs() < 0.01);
    }
}
