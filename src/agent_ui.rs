//! AI 会话页面、详情子页与设置。

use crate::agent::hooks::Health;
use crate::agent::store::{Decision, NoticeKind, PendingKind, SessionState};
use crate::agent::{AgentId, Session};
use crate::app::{App, AI_PAGE};
use crate::extras::{setting_row, settings_card, settings_control_row_right, settings_hint, settings_label, settings_segmented};
use crate::paint::{Card, H};
use crate::theme::{self, W};
use eframe::egui::{self, Color32};

impl App {
    pub(crate) fn agent_tick(&mut self, ctx: &egui::Context) {
        if !self.module_on(AI_PAGE) || !self.agent.config.enabled { return; }
        let updates = self.agent.poll(self.t);
        let mut expand = false;
        for update in updates {
            if let Some(sound) = update.sound {
                crate::agent_sound::play(sound, &self.agent.config.sound);
            }
            let Some(notice) = update.notice else { continue; };
            let waiting = matches!(notice.kind, NoticeKind::Approval | NoticeKind::Question);
            let finished = matches!(notice.kind, NoticeKind::Done | NoticeKind::Failed | NoticeKind::Interrupted);
            let enabled = match notice.kind {
                NoticeKind::Approval | NoticeKind::Question => self.agent.config.notify.waiting,
                NoticeKind::Done => self.agent.config.notify.done,
                NoticeKind::Interrupted => self.agent.config.notify.interrupted(),
                NoticeKind::Failed => self.agent.config.notify.failed,
                NoticeKind::Stale => self.agent.config.notify.stale,
            };
            if !enabled { continue; }
            let title = match notice.kind {
                NoticeKind::Approval => "需要确认",
                NoticeKind::Question => "等你回答",
                NoticeKind::Done => "会话完成",
                NoticeKind::Failed => "会话失败",
                NoticeKind::Stale => "可能已中断",
                NoticeKind::Interrupted => "会话中断",
            };
            if finished && self.agent.config.notify.auto_expand && !self.open && self.module_on(AI_PAGE) {
                // 收起时直接展开列表，让刚结束的卡片和消息预览露出来；之后按岛体全局的自动收起行为折叠。
                self.agent.detail = false;
                self.agent.selected = None;
                self.show_page(AI_PAGE);
            } else {
                self.agent_alert(title, format!("{} · {}", notice.title, notice.body), if waiting { 6.0 } else { 3.0 });
            }
            if waiting && self.agent.config.notify.auto_expand {
                self.agent.selected = Some(notice.key);
                self.agent.detail = true;
                self.agent.question_index = 0;
                self.agent.answers.clear();
                expand = true;
            }
        }
        if expand && self.module_on(AI_PAGE) { self.show_page(AI_PAGE); }
        let waiting = self.agent.store.waiting_count();
        if self.agent.detail || waiting > 0 || (self.open && self.page == AI_PAGE && !self.agent.store.visible().is_empty()) {
            // 等待状态点有呼吸效果；其余计时每秒刷新。
            let millis = if waiting > 0 { 120 } else { 1000 };
            ctx.request_repaint_after(std::time::Duration::from_millis(millis));
        }
    }

    /// 常驻状态灯是否显示：模块关闭、开关关闭或没有进行中的会话时隐藏。
    pub(crate) fn agent_bar_visible(&self) -> bool {
        self.agent.config.enabled && self.agent.config.display.bar_indicator && self.module_on(AI_PAGE)
            && self.agent.store.has_active()
    }

    /// 常驻槽：一个状态灯，运行=蓝、等待=橙（呼吸）、可能中断=黄。
    pub(crate) fn draw_agent_light(&self, c: &Card, x: f32, y: f32) {
        let active = self.agent.store.active();
        if active.is_empty() {
            c.ring(x, y, 4.0, 1.2, theme::DIM);
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
        c.circle(x, y, 4.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (alpha * 255.0) as u8));
    }

    /// 详情页自由输入框的位置：当前问题选项少于 3 个时占用空出的那一行。
    pub(crate) fn agent_free_row(&self) -> Option<(usize, f32)> {
        if !self.agent.detail { return None; }
        let pending = self.agent.selected_session()?.pending.as_ref()?;
        let PendingKind::Question(questions) = &pending.kind else { return None; };
        let index = self.agent.question_index.min(questions.len().checked_sub(1)?);
        let count = questions[index].options.len();
        (count < 3).then(|| (index, 96.0 + count as f32 * 22.0))
    }

    /// 岛内自由输入：真正的 egui 文本框，叠在详情页空出的那一行上。
    pub(crate) fn agent_free_input(&mut self, ui: &mut egui::Ui, origin: egui::Pos2, cmds: &mut Vec<crate::app::Cmd>) {
        let Some((index, y)) = self.agent_free_row() else { self.agent.free_focus = false; return; };
        let rect = egui::Rect::from_min_size(origin + egui::Vec2::new(22.0, y - 11.0), egui::Vec2::new(372.0, 18.0));
        let mut buf = std::mem::take(&mut self.agent.free);
        let editor = egui::TextEdit::singleline(&mut buf)
            .id(egui::Id::new(("agent-free", index)))
            .frame(false)
            .margin(egui::Margin::ZERO)
            .text_color(theme::WHITE)
            .font(theme::font(10.0, W::Regular))
            .hint_text(egui::RichText::new("输入其他答案，回车加入").color(theme::DIM).font(theme::font(10.0, W::Regular)))
            .desired_width(rect.width());
        let response = ui.put(rect, editor);
        if response.gained_focus() { cmds.push(crate::app::Cmd::Focus); }
        self.agent.free_focus = response.has_focus();
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let text = buf.trim().to_string();
            if !text.is_empty() {
                let multi = self.agent_question_multi(index);
                if let Some(entry) = self.agent.answers.get_mut(index) {
                    if multi { if !entry.contains(&text) { entry.push(text); } } else { entry.clear(); entry.push(text); }
                }
            }
            buf.clear();
        }
        self.agent.free = buf;
    }

    fn agent_question_multi(&self, index: usize) -> bool {
        self.agent.selected_session()
            .and_then(|session| session.pending.as_ref())
            .and_then(|pending| match &pending.kind {
                PendingKind::Question(questions) => questions.get(index).map(|question| question.multi),
                _ => None,
            })
            .unwrap_or(false)
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
            let (response, body) = crate::paint::capsule(c, 162.0, 114.0, 94.0, 24.0, "agent-setup", theme::BTN, theme::BTN_HOVER);
            body.text(209.0, 130.0, "去接入 Agent", 10.0, W::Semibold, theme::WHITE, H::Center);
            if response.clicked() { self.settings_tab = 15; self.open_settings(); }
            return;
        }
        let rows = self.agent.config.display.rows.clamp(3, 5);
        let mut clicked = None;
        let mut jumped = None;
        for (index, key) in sessions.iter().take(rows).enumerate() {
            let Some(session) = self.agent.store.sessions.iter().find(|session| &session.key == key) else { continue; };
            // 卡片左右留出翻页箭头的空间；整张卡片进详情，右侧箭头单独跳转。
            let top = 52.0 + index as f32 * 42.0;
            let row = c.hit(40.0, top, 320.0, 38.0, ("agent-row", key));
            let fill = if session.waiting() {
                Color32::from_rgba_unmultiplied(255, 166, 77, 16)
            } else if row.hovered() {
                Color32::from_white_alpha(16)
            } else {
                Color32::from_white_alpha(8)
            };
            c.fill(40.0, top, 320.0, 38.0, 9.0, fill);
            agent_icon(c, session.agent, 58.0, top + 19.0, 17.0);
            let title = session.title.as_str();
            c.text(72.0, top + 12.0, &c.fit(title, 11.5, W::Semibold, 248.0), 11.5, W::Semibold, theme::WHITE, H::Left);
            let state = state_text(session);
            let preview = card_preview(session);
            let subtitle = if preview.is_empty() { format!("{} · {}", state, duration(self.t - session.since)) } else { format!("{} · {} · {}", state, duration(self.t - session.since), preview) };
            let color = if session.stale() { theme::YELLOW } else { theme::GRAY };
            c.text(72.0, top + 26.0, &c.fit(&subtitle, 9.5, W::Regular, 250.0), 9.5, W::Regular, color, H::Left);
            c.circle(328.0, top + 12.0, 3.5, state_color(session));
            let jump = c.hit(334.0, top, 26.0, 38.0, ("agent-jump", key));
            if jump.hovered() { c.fill(336.0, top + 8.0, 22.0, 22.0, 6.0, Color32::from_white_alpha(18)); }
            c.text(347.0, top + 27.0, "›", 15.0, W::Regular, if jump.hovered() { theme::WHITE } else { theme::GRAY }, H::Center);
            if jump.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("跳转到终端").clicked() {
                jumped = Some(session.hosts.clone());
            } else if row.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                clicked = Some(key.clone());
            }
        }
        if let Some(hosts) = jumped {
            if !crate::agent::focus::focus(&hosts) { self.say("没找到窗口"); }
            return;
        }
        if let Some(key) = clicked {
            self.agent.answers = self.agent.selected_session_questions(&key).into_iter().map(|_| Vec::new()).collect();
            self.agent.question_index = 0;
            self.agent.selected = Some(key);
            self.agent.detail = true;
        }
    }

    fn agent_detail(&mut self, c: &Card) {
        let Some(session) = self.agent.selected_session() else { self.agent.detail = false; return; };
        let key = session.key.clone();
        let agent = session.agent;
        let title = session.title.clone();
        let hosts = session.hosts.clone();
        let state = state_text(session);
        let reply = session.reply.clone();
        let since = session.since;
        let approval = match session.pending.as_ref().map(|pending| &pending.kind) {
            Some(crate::agent::store::PendingKind::Approval(request)) => Some((request.tool.clone(), request.detail.clone())),
            _ => None,
        };
        let questions = session.pending.as_ref().and_then(|pending| match &pending.kind {
            crate::agent::store::PendingKind::Question(questions) => Some(questions.clone()),
            _ => None,
        }).unwrap_or_default();
        if self.agent.answers.len() < questions.len() { self.agent.answers.resize(questions.len(), Vec::new()); }

        c.text(15.0, 45.0, &c.fit(&format!("{} · {}", agent.short(), title), 11.0, W::Semibold, 270.0), 11.0, W::Semibold, theme::WHITE, H::Left);
        let jump = c.hit(300.0, 34.0, 64.0, 18.0, "agent-jump");
        c.text(332.0, 47.0, "跳转", 10.0, W::Regular, theme::GRAY, H::Center);
        if jump.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            if !crate::agent::focus::focus(&hosts) { self.say("没找到窗口"); }
        }
        let back = c.hit(372.0, 34.0, 34.0, 18.0, "agent-back");
        c.text(389.0, 47.0, "返回", 10.0, W::Regular, theme::GRAY, H::Center);
        if back.clicked() { self.agent.detail = false; self.agent.selected = None; return; }

        if let Some((tool, detail)) = approval {
            c.text(15.0, 66.0, &format!("请求使用 {tool}"), 12.0, W::Bold, Color32::from_rgb(255, 176, 86), H::Left);
            let lines = c.wrap(&detail, 10.0, W::Regular, 386.0, 2);
            for (index, line) in lines.iter().enumerate() { c.text(15.0, 82.0 + index as f32 * 14.0, line, 10.0, W::Regular, theme::GRAY, H::Left); }
            let y = 130.0;
            if button(c, 15.0, y, 88.0, "允许一次", theme::GREEN).clicked() { self.agent.respond(&key, Decision::Allow, self.t); return; }
            if button(c, 111.0, y, 66.0, "拒绝", theme::RED).clicked() { self.agent.respond(&key, Decision::Deny, self.t); return; }
            if button(c, 185.0, y, 80.0, "总是允许", theme::BLUE).clicked() { self.agent.respond(&key, Decision::Always, self.t); return; }
            if button(c, 273.0, y, 94.0, "到终端处理", theme::BTN).clicked() {
                self.agent.respond(&key, Decision::Terminal, self.t);
                if !crate::agent::focus::focus(&hosts) { self.say("没找到窗口"); }
                return;
            }
            return;
        }

        if questions.is_empty() {
            // 只展示最近回复，行数在设置里调整。
            let lines = self.agent.config.display.recent_lines.clamp(1, 3);
            let mut y = 64.0;
            c.text(15.0, y, "最近回复", 9.0, W::Regular, theme::GRAY, H::Left);
            y += 14.0;
            let reply = reply.unwrap_or_default();
            let reply = if reply.is_empty() { "等待回复…".to_owned() } else { reply };
            for line in c.wrap(&reply, 10.0, W::Regular, 386.0, lines) {
                c.text(15.0, y, &line, 10.0, W::Regular, theme::WHITE, H::Left);
                y += 14.0;
            }
            c.text(15.0, self.expanded_height() - 30.0, &format!("{state} · {}", duration(self.t - since)), 9.5, W::Regular, theme::GRAY, H::Left);
            return;
        }
        let index = self.agent.question_index.min(questions.len() - 1);
        let question = &questions[index];
        let prompt = if question.header.is_empty() { question.question.clone() } else { format!("{} · {}", question.header, question.question) };
        let lines = c.wrap(&prompt, 11.0, W::Semibold, 386.0, 2);
        for (line_index, line) in lines.iter().enumerate() { c.text(15.0, 66.0 + line_index as f32 * 14.0, line, 11.0, W::Semibold, theme::WHITE, H::Left); }
        let selected = self.agent.answers.get(index).cloned().unwrap_or_default();
        let mut clicked_option = None;
        for (option_index, option) in question.options.iter().take(3).enumerate() {
            let y = 96.0 + option_index as f32 * 22.0;
            let on = selected.iter().any(|label| label == &option.label);
            let response = c.hit(15.0, y - 13.0, 386.0, 20.0, ("agent-option", index, option_index));
            if on { c.fill(15.0, y - 13.0, 386.0, 20.0, 6.0, Color32::from_rgba_unmultiplied(62, 172, 248, 30)); }
            c.text(24.0, y, &c.fit(&option.label, 10.5, W::Semibold, 150.0), 10.5, W::Semibold, if on { theme::BLUE } else { theme::WHITE }, H::Left);
            if !option.description.is_empty() { c.text(180.0, y, &c.fit(&option.description, 9.5, W::Regular, 215.0), 9.5, W::Regular, theme::GRAY, H::Left); }
            if response.clicked() { clicked_option = Some(option.label.clone()); }
        }
        if let Some(label) = clicked_option {
            let entry = &mut self.agent.answers[index];
            if question.multi {
                if let Some(position) = entry.iter().position(|value| value == &label) { entry.remove(position); } else { entry.push(label); }
            } else {
                entry.clear(); entry.push(label);
            }
        }
        // 选项少于三行时，空出的那一行用于自由输入（文本框在页面渲染后叠加）。
        if question.options.len() < 3 {
            let input_y = 96.0 + question.options.len() as f32 * 22.0;
            c.fill(15.0, input_y - 11.0, 386.0, 18.0, 6.0, Color32::from_white_alpha(10));
        }
        let last = index + 1 >= questions.len();
        let y = 148.0;        if !last {
            if button(c, 15.0, y, 76.0, "下一题", theme::BTN).clicked() { self.agent.question_index = index + 1; }
        } else if button(c, 15.0, y, 76.0, "提交", theme::GREEN).clicked() {
            self.agent.respond(&key, Decision::Answers(self.agent.answers.clone()), self.t);
            return;
        }
        if index > 0 && button(c, 99.0, y, 76.0, "上一题", theme::BTN).clicked() { self.agent.question_index = index - 1; }
        if button(c, 183.0, y, 94.0, "到终端处理", theme::BTN).clicked() {
            self.agent.respond(&key, Decision::Terminal, self.t);
            if !crate::agent::focus::focus(&hosts) { self.say("没找到窗口"); }
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
            crate::extras::settings_action_row(ui,|ui|{
                if ui.add(crate::extras::settings_primary_button("重新扫描")).clicked() { self.agent.rescan(); }
                if ui.add(crate::extras::settings_danger_button("清理旧 Hook")).clicked() {
                    let changed = crate::agent::hooks::cleanup_legacy();
                    self.agent.rescan();
                    self.say(if changed { "已清理 Fly Island / 提示音 Hook，备份已保留" } else { "没有发现需要清理的旧 Hook" });
                }
            });
            settings_hint(ui, "关闭监控不会删除 Hook；需要干净配置时点「移除」。");
        });

        settings_label(ui, "提醒事件");
        settings_card(ui, |ui| {
            let mut waiting = self.agent.config.notify.waiting;
            setting_row(ui, "等待用户选择", "等待审批或回答时在岛上提醒", &mut waiting);
            if waiting != self.agent.config.notify.waiting { self.agent.config.notify.waiting = waiting; self.agent.config.save(); }
            ui.separator();
            let mut done = self.agent.config.notify.done;
            setting_row(ui, "完成", "会话完成时在岛上提醒", &mut done);
            if done != self.agent.config.notify.done { self.agent.config.notify.done = done; self.agent.config.save(); }
            ui.separator();
            let mut interrupted = self.agent.config.notify.interrupted();
            let before = interrupted;
            setting_row(ui, "中断", "会话中断时在岛上提醒", &mut interrupted);
            if interrupted != before {
                self.agent.config.notify.interrupted = Some(interrupted);
                self.agent.config.save();
            }
            ui.separator();
            let mut failed = self.agent.config.notify.failed;
            setting_row(ui, "失败", "会话失败时在岛上提醒", &mut failed);
            if failed != self.agent.config.notify.failed { self.agent.config.notify.failed = failed; self.agent.config.save(); }
            ui.separator();
            let mut stale = self.agent.config.notify.stale;
            setting_row(ui, "闲置", "运行中超过 10 分钟没有事件时提醒", &mut stale);
            if stale != self.agent.config.notify.stale { self.agent.config.notify.stale = stale; self.agent.config.save(); }
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
            let mut auto_expand = self.agent.config.notify.auto_expand;
            setting_row(ui, "自动展开", "等待确认或回答时打开详情；完成或中断时打开列表", &mut auto_expand);
            if auto_expand != self.agent.config.notify.auto_expand { self.agent.config.notify.auto_expand = auto_expand; self.agent.config.save(); }
            ui.separator();
            settings_control_row_right(ui, "重复提醒", |ui| {
                let mut repeat = match self.agent.config.notify.repeat.as_str() { "30s" => 1, "1m" => 2, "5m" => 3, _ => 0 };
                if settings_segmented(ui, &mut repeat, &[(0, "不提醒"), (1, "30 秒"), (2, "1 分钟"), (3, "5 分钟")], 56.0) {
                    self.agent.config.notify.repeat = match repeat { 1 => "30s", 2 => "1m", 3 => "5m", _ => "never" }.into();
                    self.agent.config.save();
                }
            });
            ui.separator();
            settings_control_row_right(ui, "会话数", |ui| {
                let mut rows = self.agent.config.display.rows;
                if settings_segmented(ui, &mut rows, &[(3, "3 个"), (4, "4 个"), (5, "5 个")], 52.0) {
                    self.agent.config.display.rows = rows;
                    self.agent.config.save();
                }
            });
            ui.separator();
            settings_control_row_right(ui, "最近消息行数", |ui| {
                let mut lines = self.agent.config.display.recent_lines;
                if settings_segmented(ui, &mut lines, &[(1, "1 行"), (2, "2 行"), (3, "3 行")], 52.0) {
                    self.agent.config.display.recent_lines = lines;
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
    let (response, body) = crate::paint::capsule(c, x, y, w, 24.0, label, color, crate::paint::mix(color, Color32::WHITE, 0.12));
    body.text(x + w / 2.0, y + 16.0, label, 10.0, W::Semibold, theme::WHITE, H::Center);
    response
}

fn state_text(session: &Session) -> &'static str {
    if session.stale() { return "可能已中断"; }
    match session.state {
        SessionState::WaitingApproval => "等待确认",
        SessionState::WaitingAnswer => "等你回答",
        SessionState::Running => "运行中",
        SessionState::Idle => "闲置",
        SessionState::Completed => "已完成",
        SessionState::Failed => "失败",
        SessionState::Interrupted => "已中断",
    }
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

/// 卡片副标题里的消息预览：当前工具优先，运行中还没输出时给占位；
/// 结束/闲置后显示最近回复或最近消息。
fn card_preview(session: &Session) -> String {
    let text = if let Some(tool) = &session.tool {
        match session.detail.as_deref() {
            Some(detail) if !detail.is_empty() => format!("{tool} · {detail}"),
            _ => tool.clone(),
        }
    } else if session.state == SessionState::Running {
        "等待输出…".into()
    } else if let Some(reply) = &session.reply {
        reply.clone()
    } else if let Some(message) = &session.last_message {
        message.clone()
    } else {
        match session.state {
            SessionState::WaitingApproval | SessionState::WaitingAnswer => "等待你的决定".into(),
            SessionState::Idle => "等待新任务…".into(),
            SessionState::Completed => "已完成".into(),
            SessionState::Failed => "运行失败".into(),
            SessionState::Interrupted => "已中断".into(),
            _ => String::new(),
        }
    };
    text.lines().next().unwrap_or("").to_owned()
}

fn duration(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    if seconds < 60 { format!("{seconds} 秒") }
    else if seconds < 3600 { format!("{} 分钟", seconds / 60) }
    else { format!("{} 小时", seconds / 3600) }
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

fn agent_icon(c: &Card, agent: AgentId, x: f32, y: f32, size: f32) {
    let ctx = c.ui.ctx();
    let ppp = ctx.pixels_per_point();
    let pixels = (size * c.s * ppp).round().max(1.0) as u32;
    let key = egui::Id::new(("agent-icon", agent.key(), pixels));
    let texture = ctx.data_mut(|data| data.get_temp::<egui::TextureHandle>(key)).or_else(|| {
        let bytes: &[u8] = match agent {
            AgentId::Claude => include_bytes!("../assets/ai-icons/claude.svg"),
            AgentId::Codex => include_bytes!("../assets/ai-icons/codex.svg"),
            AgentId::ZCode => include_bytes!("../assets/ai-icons/zcode.svg"),
            AgentId::Copilot => include_bytes!("../assets/ai-icons/copilot.svg"),
            AgentId::Dsh => include_bytes!("../assets/ai-icons/deepseek.svg"),
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
