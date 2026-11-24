//! 会话状态机与本地配置。会话只在内存，保留期到点清理。

use super::{AgentId, EventKind, Question};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

fn yes() -> bool { true }
fn default_retention() -> u32 { 30 }
fn default_rows() -> usize { 3 }

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "yes")] pub monitor: bool,
    #[serde(default = "yes")] pub approval: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "yes")] pub bar_indicator: bool,
    #[serde(default = "default_rows")] pub rows: usize,
}
impl Default for DisplayConfig {
    fn default() -> Self { Self { bar_indicator: true, rows: 3 } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotifyConfig {
    /// 旧配置的合并开关，仅用于迁移到 question / approval。
    #[serde(default = "yes", skip_serializing)] pub waiting: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub question: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub approval: Option<bool>,
    #[serde(default = "yes")] pub done: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub interrupted: Option<bool>,
    #[serde(default = "yes")] pub failed: bool,
}
impl NotifyConfig {
    /// 旧配置没有单独的「中断」开关，沿用「完成」的旧值。
    pub fn interrupted(&self) -> bool { self.interrupted.unwrap_or(self.done) }
    pub fn question(&self) -> bool { self.question.unwrap_or(self.waiting) }
    pub fn approval(&self) -> bool { self.approval.unwrap_or(self.waiting) }
    pub fn enabled(&self, kind: NoticeKind) -> bool {
        match kind {
            NoticeKind::Approval => self.approval(),
            NoticeKind::Question => self.question(),
            NoticeKind::Done => self.done,
            NoticeKind::Interrupted => self.interrupted(),
            NoticeKind::Failed => self.failed,
            // 可能中断只更新状态灯，不再提醒。
            NoticeKind::Stale => false,
        }
    }
}
impl Default for NotifyConfig {
    fn default() -> Self { Self { waiting: true, question: Some(true), approval: Some(true), done: true, interrupted: None, failed: true } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoundConfig {
    #[serde(default = "yes")] pub enabled: bool,
    #[serde(default = "yes")] pub start: bool,
    #[serde(default = "yes")] pub submit: bool,
    #[serde(default = "yes")] pub approval: bool,
    #[serde(default = "yes")] pub complete: bool,
    #[serde(default = "yes")] pub error: bool,
}
impl Default for SoundConfig {
    fn default() -> Self { Self { enabled: true, start: true, submit: true, approval: true, complete: true, error: true } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "yes")] pub enabled: bool,
    #[serde(default)] pub agents: HashMap<String, AgentConfig>,
    #[serde(default = "default_retention")] pub retention_minutes: u32,
    #[serde(default)] pub display: DisplayConfig,
    #[serde(default)] pub notify: NotifyConfig,
    #[serde(default)] pub sound: SoundConfig,
}
impl Default for Config {
    fn default() -> Self {
        Self { enabled: true, agents: HashMap::new(), retention_minutes: default_retention(), display: DisplayConfig::default(), notify: NotifyConfig::default(), sound: SoundConfig::default() }
    }
}

impl Config {
    fn path() -> std::path::PathBuf { crate::model::config_dir().join("ai-sessions.json") }

    pub fn load() -> Self {
        if cfg!(test) { return Self::default(); }
        let mut config = std::fs::read(Self::path()).ok()
            .and_then(|raw| serde_json::from_slice::<Self>(&raw).ok())
            .unwrap_or_default();
        config.retention_minutes = snap_retention(config.retention_minutes);
        config.display.rows = config.display.rows.clamp(3, 5);
        // 旧配置只有合并的「等待用户选择」开关，拆分后沿用旧值。
        config.notify.question.get_or_insert(config.notify.waiting);
        config.notify.approval.get_or_insert(config.notify.waiting);
        // 「提示音」总开关已移除：旧配置关闭时把各事件一并关掉，保持原来的静音结果。
        if !config.sound.enabled {
            config.sound.enabled = true;
            config.sound.start = false;
            config.sound.submit = false;
            config.sound.approval = false;
            config.sound.complete = false;
            config.sound.error = false;
        }
        config
    }

    pub fn save(&self) {
        if cfg!(test) { return; }
        if let Ok(raw) = serde_json::to_vec_pretty(self) { let _ = crate::model::atomic_write(&Self::path(), &raw); }
    }

    pub fn agent(&self, agent: AgentId) -> AgentConfig {
        self.agents.get(agent.key()).copied().unwrap_or(AgentConfig {
            monitor: matches!(agent, AgentId::Claude | AgentId::Codex | AgentId::ZCode),
            approval: matches!(agent, AgentId::Claude | AgentId::Codex | AgentId::ZCode),
        })
    }

    pub fn set_agent(&mut self, agent: AgentId, value: AgentConfig) {
        self.agents.insert(agent.key().into(), value);
    }
}

pub fn snap_retention(value: u32) -> u32 {
    let value = value.clamp(1, 60);
    if value == 1 { 1 } else { ((value + 2) / 5 * 5).clamp(5, 60) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState { Running, WaitingApproval, WaitingAnswer, Idle, Completed, Failed, Interrupted }

#[derive(Debug)]
pub enum PendingKind { Approval(super::ApprovalRequest), Question(Vec<Question>) }

/// 回答 AskUserQuestion 后，Claude Code 还会为同一个工具再发一次 PermissionRequest。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QuestionFollowup { Allow, Pass }

#[derive(Debug)]
pub struct Pending {
    pub kind: PendingKind,
    pub at: f64,
    pub reply: Option<mpsc::Sender<String>>,
    pub raw_input: serde_json::Value,
}
impl Pending {
    fn questions(&self) -> &[Question] {
        match &self.kind { PendingKind::Question(questions) => questions, _ => &[] }
    }
}

#[derive(Debug)]
pub struct Session {
    pub key: String,
    pub agent: AgentId,
    pub id: String,
    pub cwd: String,
    pub title: String,
    /// 标题来自 Claude Code 的会话元数据或 transcript，不再被项目名/提问覆盖。
    title_from_meta: bool,
    /// Claude Code 的 transcript 路径；标题从其中的 `custom-title` / `ai-title` 读取。
    transcript: String,
    /// 已读到的 transcript 字节位置，后续只读追加内容。
    transcript_offset: u64,
    /// 已读到用户命名的标题，后续自动标题不再覆盖。
    title_from_custom: bool,
    pub state: SessionState,
    pub since: f64,
    pub updated_at: f64,
    pub tool: Option<String>,
    pub detail: Option<String>,
    pub last_message: Option<String>,
    /// 最近一次提问与最近一条回复，列表卡片展示回复摘要。
    pub prompt: Option<String>,
    pub reply: Option<String>,
    pub pending: Option<Pending>,
    /// 刚回答完问题时记下选择；随后同名工具的 PermissionRequest 直接沿用，不再弹第二次。
    question_followup: Option<(QuestionFollowup, f64)>,
    /// 最近一次 Hook 进程的祖先链（pid, 进程名），跳转时用来找窗口。
    pub hosts: Vec<(u32, String)>,
    /// 本轮任务里出现过的子代理；主会话结束或开始新一轮时清空。
    pub subagents: HashSet<String>,
    pub stale_at: Option<f64>,
    pub stale_notified: bool,
}

impl Session {
    fn new(agent: AgentId, key: String, id: String, cwd: String, transcript: String, now: f64) -> Self {
        let meta_title = super::hooks::session_title(agent, &id);
        let title_from_meta = meta_title.is_some();
        Self {
            key, agent, id, title: meta_title.unwrap_or_else(|| project_title(&cwd, agent)), cwd,
            title_from_meta, transcript, transcript_offset: 0, title_from_custom: false,
            state: SessionState::Running, since: now, updated_at: now,
            tool: None, detail: None, last_message: None, prompt: None, reply: None, pending: None,
            question_followup: None,
            hosts: Vec::new(),
            subagents: HashSet::new(),
            stale_at: None, stale_notified: false,
        }
    }
    #[cfg(test)]
    pub(crate) fn sample(state: SessionState) -> Self {
        let mut session = Self::new(AgentId::Claude, "claude:sample".into(), "sample".into(), "E:\\demo".into(), String::new(), 0.0);
        session.state = state;
        session
    }
    pub fn stale(&self) -> bool { self.stale_at.is_some() }
    pub fn waiting(&self) -> bool { matches!(self.state, SessionState::WaitingApproval | SessionState::WaitingAnswer) }
    /// 仍在进行或等待的会话；闲置、完成、失败只保留卡片，不点亮常驻状态灯。
    pub fn is_active(&self) -> bool { matches!(self.state, SessionState::Running | SessionState::WaitingApproval | SessionState::WaitingAnswer) }
    /// 是否显示在列表里：可能中断的会话提醒后隐藏，其余状态保留到清理。
    pub fn shown(&self) -> bool {
        match self.state {
            SessionState::Running => self.stale_at.is_none() || !self.stale_notified,
            _ => true,
        }
    }
    fn rank(&self) -> u8 {
        if self.waiting() { 3 } else { match self.state { SessionState::Running => 2, SessionState::Idle => 1, _ => 0 } }
    }
}

fn project_title(cwd: &str, agent: AgentId) -> String {
    std::path::Path::new(cwd).file_name().and_then(|name| name.to_str()).filter(|name| !name.is_empty()).unwrap_or(agent.display()).to_string()
}

/// 会话标题以 Claude Code transcript 里的标题为准；拿不到时退回进程名、项目名或首条提问。
fn refresh_title(session: &mut Session) {
    if session.agent == AgentId::Claude && !session.transcript.is_empty() {
        if let Some(name) = super::hooks::transcript_title(
            std::path::Path::new(&session.transcript),
            &mut session.transcript_offset,
            &mut session.title_from_custom,
        ) {
            session.title = name;
            session.title_from_meta = true;
            return;
        }
        if session.title_from_custom { return; }
    }
    if let Some(name) = super::hooks::session_title(session.agent, &session.id) {
        session.title = name;
        session.title_from_meta = true;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind { Approval, Question, Done, Failed, Stale, Interrupted }

#[derive(Clone, Debug)]
pub struct Notice {
    pub kind: NoticeKind,
    pub agent: AgentId,
    pub key: String,
    pub title: String,
    pub body: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundKind { Start, Submit, Approval, Complete, Error }

#[derive(Clone, Debug, Default)]
pub struct Update {
    pub notice: Option<Notice>,
    pub sound: Option<SoundKind>,
}

#[derive(Clone, Debug)]
pub enum Decision { Allow, Deny, Always, Terminal, Answers(Vec<Vec<String>>) }

#[derive(Default)]
pub struct Store {
    pub sessions: Vec<Session>,
    always: HashSet<(String, String)>,
}

impl Store {
    pub fn apply(&mut self, incoming: super::bridge::Incoming, now: f64, approval_enabled: bool) -> Vec<Update> {
        let super::bridge::Incoming { agent, event, payload, reply, hosts, .. } = incoming;
        let Some(kind) = super::normalize(agent, &event, &payload) else { return Vec::new(); };
        let id = super::session_id(&payload);
        let key = format!("{}:{}", agent.key(), id);
        let mut updates = Vec::new();
        // 子代理的事件沿用主会话的 session_id，用 agent_id 区分。
        let subagent = payload.get("agent_id").and_then(serde_json::Value::as_str)
            .filter(|id| !id.is_empty()).map(str::to_string);
        if matches!(kind, EventKind::SessionEnd) {
            if let Some(id) = &subagent {
                if let Some(session) = self.sessions.iter_mut().find(|session| session.key == key) {
                    session.subagents.remove(id);
                }
            } else {
                self.sessions.retain(|session| session.key != key);
            }
            return updates;
        }
        if !approval_enabled && matches!(kind, EventKind::Permission(_) | EventKind::Question(_)) {
            if let Some(reply) = reply { let _ = reply.send("{}".into()); }
            return updates;
        }
        let cwd = super::cwd(&payload);
        let transcript = super::string_field(&payload, &["transcript_path", "transcriptPath"]);
        let index = match self.sessions.iter().position(|session| session.key == key) {
            Some(index) => index,
            None => {
                let mut session = Session::new(agent, key.clone(), id, cwd.clone(), transcript.clone(), now);
                refresh_title(&mut session);
                self.sessions.push(session);
                self.sessions.len() - 1
            }
        };
        let session = &mut self.sessions[index];
        if !hosts.is_empty() { session.hosts = hosts; }
        if let Some(id) = &subagent { session.subagents.insert(id.clone()); }
        if !transcript.is_empty() && session.transcript != transcript {
            session.transcript = transcript;
            session.transcript_offset = 0;
            session.title_from_custom = false;
        }
        session.updated_at = now;
        session.stale_at = None;
        session.stale_notified = false;
        // 第一条事件可能不是 SessionStart；后续事件带上工作目录时补全项目名标题。
        if !cwd.is_empty() && session.cwd != cwd {
            session.cwd = cwd;
            if session.prompt.is_none() && !session.title_from_meta { session.title = project_title(&session.cwd, agent); }
        }
        refresh_title(session);
        match kind {
            EventKind::SessionStart { source, .. } => {
                if source == "compact" {
                    // 压缩不是新会话：非运行中不恢复状态、不发声。
                    if subagent.is_none() && session.state == SessionState::Running {
                        updates.push(Update { notice: None, sound: Some(SoundKind::Start) });
                    }
                } else if subagent.is_none() {
                    // 新会话还没有收到第一条提问，先显示为闲置，避免空会话一直亮运行灯。
                    session.state = SessionState::Idle;
                    session.pending = None;
                    session.question_followup = None;
                    session.subagents.clear();
                    updates.push(Update { notice: None, sound: Some(SoundKind::Start) });
                }
            }
            EventKind::Prompt { text } => {
                if subagent.is_some() { return updates; }
                session.subagents.clear();
                if !text.is_empty() {
                    // 没有自动标题时，用会话的第一条提问作标题，后续追问不再改。
                    if session.prompt.is_none() && !session.title_from_meta { session.title = text.chars().take(28).collect(); }
                    session.prompt = Some(text);
                }
                session.state = SessionState::Running;
                session.pending = None;
                session.question_followup = None;
                session.tool = None;
                session.detail = None;
                // 新一轮任务开始后，卡片只等本轮 Stop 的输出，不沿用上一轮回复。
                session.reply = None;
                updates.push(Update { notice: None, sound: Some(SoundKind::Submit) });
            }
            EventKind::ToolStart { tool, detail } => {
                session.state = SessionState::Running;
                session.tool = Some(tool);
                session.detail = Some(detail);
            }
            EventKind::ToolEnd { ok } => {
                session.state = SessionState::Running;
                session.tool = None;
                session.detail = None;
                session.pending = None;
                if !ok { session.last_message = Some("工具调用失败".into()); }
            }
            EventKind::Permission(request) => {
                // 回答完 AskUserQuestion 后 Claude Code 会再发一次同名 PermissionRequest；
                // 直接沿用刚才的选择，避免同一个问题弹两次。
                if request.tool == "AskUserQuestion" {
                    if let Some((followup, at)) = session.question_followup.take() {
                        if now - at <= 30.0 {
                            if let Some(reply) = reply {
                                let _ = reply.send(match followup {
                                    QuestionFollowup::Allow => allow_json(agent),
                                    QuestionFollowup::Pass => "{}".into(),
                                });
                            }
                            session.state = SessionState::Running;
                            session.tool = None;
                            session.detail = None;
                            return updates;
                        }
                    }
                }
                session.tool = Some(request.tool.clone());
                session.detail = Some(request.detail.clone());
                if self.always.contains(&(session.key.clone(), request.tool.clone())) {
                    if let Some(reply) = reply { let _ = reply.send(allow_json(agent)); }
                    session.state = SessionState::Running;
                } else {
                    let body = if request.detail.is_empty() { format!("请求使用 {}", request.tool) } else { format!("{} · {}", request.tool, request.detail) };
                    session.state = SessionState::WaitingApproval;
                    session.pending = Some(Pending {
                        kind: PendingKind::Approval(request),
                        at: now,
                        reply,
                        raw_input: payload.get("tool_input").cloned().unwrap_or(serde_json::Value::Null),
                    });
                    updates.push(Update {
                        notice: Some(Notice { kind: NoticeKind::Approval, agent, key: session.key.clone(), title: format!("{} · {}", agent.short(), session.title), body }),
                        sound: Some(SoundKind::Approval),
                    });
                }
            }
            EventKind::Question(questions) => {
                let body = questions.first().map(|question| question.question.clone()).unwrap_or_default();
                session.state = SessionState::WaitingAnswer;
                session.pending = Some(Pending {
                    kind: PendingKind::Question(questions),
                    at: now,
                    reply,
                    raw_input: payload.get("tool_input").cloned().unwrap_or(serde_json::Value::Null),
                });
                updates.push(Update {
                    notice: Some(Notice { kind: NoticeKind::Question, agent, key: session.key.clone(), title: format!("{} · {}", agent.short(), session.title), body }),
                    sound: Some(SoundKind::Approval),
                });
            }
            EventKind::Message(text) => {
                if !text.is_empty() { session.last_message = Some(text); }
                if payload.get("notification_type").and_then(serde_json::Value::as_str) == Some("idle_prompt") {
                    session.state = SessionState::Idle;
                }
            }
            EventKind::Stop { message } => {
                if let Some(id) = &subagent {
                    session.subagents.remove(id);
                    return updates;
                }
                session.subagents.clear();
                let was = session.state;
                session.pending = None;
                session.question_followup = None;
                session.tool = None;
                session.detail = None;
                session.reply = message;
                session.state = SessionState::Completed;
                if was != SessionState::Completed {
                    updates.push(Update {
                        notice: Some(Notice { kind: NoticeKind::Done, agent, key: session.key.clone(), title: format!("{} · {}", agent.short(), session.title), body: "已完成".into() }),
                        sound: Some(SoundKind::Complete),
                    });
                }
            }
            EventKind::Failure(message) => {
                if let Some(id) = &subagent {
                    session.subagents.remove(id);
                    return updates;
                }
                session.subagents.clear();
                let was = session.state;
                session.pending = None;
                session.question_followup = None;
                session.tool = None;
                session.detail = None;
                session.state = SessionState::Failed;
                let body = if message.is_empty() { "运行失败".to_string() } else { message };
                session.reply = Some(body.clone());
                if was != SessionState::Failed {
                    updates.push(Update {
                        notice: Some(Notice { kind: NoticeKind::Failed, agent, key: session.key.clone(), title: format!("{} · {}", agent.short(), session.title), body }),
                        sound: Some(SoundKind::Error),
                    });
                }
            }
            EventKind::Interrupt => {
                if let Some(id) = &subagent {
                    session.subagents.remove(id);
                    return updates;
                }
                session.subagents.clear();
                let was = session.state;
                session.pending = None;
                session.question_followup = None;
                session.tool = None;
                session.detail = None;
                session.state = SessionState::Interrupted;
                if was != SessionState::Interrupted {
                    updates.push(Update {
                        notice: Some(Notice { kind: NoticeKind::Interrupted, agent, key: session.key.clone(), title: format!("{} · {}", agent.short(), session.title), body: "已中断".into() }),
                        sound: Some(SoundKind::Error),
                    });
                }
            }
            EventKind::SessionEnd => {}
        }
        updates
    }

    pub fn tick(&mut self, now: f64, retention_minutes: u32) -> Vec<Update> {
        let mut updates = Vec::new();
        for session in &mut self.sessions {
            if session.state == SessionState::Running && session.pending.is_none() && now - session.updated_at > 600.0 {
                match session.stale_at {
                    None => session.stale_at = Some(now),
                    Some(at) if !session.stale_notified && now - at >= 30.0 => {
                        session.stale_notified = true;
                        updates.push(Update {
                            notice: Some(Notice { kind: NoticeKind::Stale, agent: session.agent, key: session.key.clone(), title: format!("{} · {}", session.agent.short(), session.title), body: "可能已中断".into() }),
                            sound: None,
                        });
                    }
                    _ => {}
                }
            }
        }
        let cutoff = now - retention_minutes as f64 * 60.0;
        self.sessions.retain(|session| session.updated_at >= cutoff || session.pending.is_some());
        updates
    }

    /// 列表里的会话：活跃、闲置和已结束的都保留卡片，到保留时间后清理。
    pub fn visible(&self) -> Vec<&Session> {
        let mut list: Vec<_> = self.sessions.iter().filter(|session| session.shown()).collect();
        list.sort_by(|a, b| b.rank().cmp(&a.rank()).then_with(|| b.updated_at.total_cmp(&a.updated_at)));
        list
    }

    /// 常驻状态灯只统计仍在运行或等待的会话。
    pub fn active(&self) -> Vec<&Session> {
        self.visible().into_iter().filter(|session| session.is_active()).collect()
    }

    pub fn has_active(&self) -> bool {
        self.sessions.iter().any(|session| session.shown() && session.is_active())
    }

    pub fn waiting_count(&self) -> usize { self.visible().iter().filter(|session| session.waiting()).count() }

    pub fn respond(&mut self, key: &str, decision: Decision, now: f64) -> bool {
        let Some(session) = self.sessions.iter_mut().find(|session| session.key == key) else { return false; };
        let Some(pending) = session.pending.take() else { return false; };
        if matches!(decision, Decision::Always) {
            if let PendingKind::Approval(request) = &pending.kind {
                self.always.insert((session.key.clone(), request.tool.clone()));
            }
        }
        let body = response_json(session.agent, &pending, &decision);
        if let Some(reply) = pending.reply { let _ = reply.send(body); }
        if matches!(&pending.kind, PendingKind::Question(_)) {
            let followup = if matches!(&decision, Decision::Answers(_)) { QuestionFollowup::Allow } else { QuestionFollowup::Pass };
            session.question_followup = Some((followup, now));
        }
        session.state = SessionState::Running;
        session.updated_at = now;
        true
    }
}

fn allow_json(agent: AgentId) -> String {
    match agent {
        AgentId::Codex => serde_json::json!({"decision":{"behavior":"allow"}}).to_string(),
        _ => serde_json::json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}).to_string(),
    }
}

fn response_json(agent: AgentId, pending: &Pending, decision: &Decision) -> String {
    let value = match (&pending.kind, decision) {
        (PendingKind::Approval(_), Decision::Allow | Decision::Always) => {
            if agent == AgentId::Codex { serde_json::json!({"decision":{"behavior":"allow"}}) }
            else { serde_json::json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}) }
        }
        (PendingKind::Approval(_), Decision::Deny) => {
            if agent == AgentId::Codex { serde_json::json!({"decision":{"behavior":"deny","message":"用户在岛仔中拒绝"}}) }
            else { serde_json::json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"用户在岛仔中拒绝"}}}) }
        }
        (PendingKind::Question(_), Decision::Answers(answers)) => {
            let mut input = pending.raw_input.clone();
            input["answers"] = answers_json(pending.questions(), answers);
            serde_json::json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":input}})
        }
        _ => serde_json::json!({}),
    };
    value.to_string()
}

fn answers_json(questions: &[Question], answers: &[Vec<String>]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (index, question) in questions.iter().enumerate() {
        let answer = answers.get(index).cloned().unwrap_or_default();
        let value = if question.multi { serde_json::json!(answer) } else { serde_json::json!(answer.first().cloned().unwrap_or_default()) };
        map.insert(question.question.clone(), value);
    }
    serde_json::Value::Object(map)
}

pub struct AgentState {
    pub config: Config,
    pub store: Store,
    pub bridge: Option<super::bridge::Bridge>,
    /// 监控是否已启用；关闭时不保留桥接和会话，Hook 调用会立即返回。
    pub active: bool,
    pub status: [super::hooks::Status; super::AGENT_COUNT],
    pub selected: Option<String>,
    pub detail: bool,
    pub question_index: usize,
    pub answers: Vec<Vec<String>>,
    /// 提问自由输入框的草稿与焦点状态。
    pub free: String,
    pub free_focus: bool,
    /// 审批详情实测换行行数（1–2），用于按内容收缩高度。
    pub approval_lines: usize,
    /// 会话列表的内部滚动位置。
    pub scroll: f32,
    pub notice_seq: u64,
    last_tick: f64,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            config: Config::default(),
            store: Store::default(),
            bridge: None,
            active: false,
            status: std::array::from_fn(|index| super::hooks::Status::unknown(AgentId::ALL[index])),
            selected: None,
            detail: false,
            question_index: 0,
            answers: Vec::new(),
            free: String::new(),
            free_focus: false,
            approval_lines: 2,
            scroll: 0.0,
            notice_seq: 0,
            last_tick: 0.0,
        }
    }
}

impl AgentState {
    pub fn load() -> Self {
        if cfg!(test) { return Self::default(); }
        let config = Config::load();
        let status = std::array::from_fn(|index| super::hooks::status(AgentId::ALL[index]));
        Self { config, status, ..Self::default() }
    }

    pub fn start(&mut self, ctx: &eframe::egui::Context) {
        if self.active { return; }
        self.active = true;
        if cfg!(test) { return; }
        if self.bridge.is_none() { self.bridge = super::bridge::Bridge::start(ctx.clone()).ok(); }
        super::hooks::cleanup_legacy();
        super::hooks::refresh_timeouts();
        self.rescan();
    }

    /// 关闭监控：移除桥接文件并丢弃会话，不再轮询或刷新界面。
    pub fn stop(&mut self) {
        self.active = false;
        self.bridge = None;
        self.store = Store::default();
        self.detail = false;
        self.selected = None;
        self.question_index = 0;
        self.answers.clear();
        self.free.clear();
        self.free_focus = false;
        self.scroll = 0.0;
        self.notice_seq = 0;
        self.last_tick = 0.0;
    }

    pub fn set_active(&mut self, active: bool, ctx: &eframe::egui::Context) {
        if active { self.start(ctx); } else { self.stop(); }
    }

    pub fn rescan(&mut self) {
        for (index, agent) in AgentId::ALL.iter().enumerate() {
            self.status[index] = super::hooks::status(*agent);
        }
    }

    pub fn install(&mut self, agent: AgentId) -> Result<String, String> {
        let exe = std::env::current_exe().map_err(|error| format!("找不到岛仔路径：{error}"))?;
        let note = super::hooks::install(agent, &exe)?;
        self.rescan();
        Ok(note)
    }

    pub fn remove(&mut self, agent: AgentId) -> Result<String, String> {
        let note = super::hooks::remove(agent)?;
        self.rescan();
        Ok(note)
    }

    pub fn monitored(&self, agent: AgentId) -> bool { self.config.enabled && self.config.agent(agent).monitor }

    pub fn approval(&self, agent: AgentId) -> bool { self.monitored(agent) && self.config.agent(agent).approval }

    pub fn poll(&mut self, now: f64) -> Vec<Update> {
        if !self.active { return Vec::new(); }
        let mut updates = Vec::new();
        if let Some(bridge) = &self.bridge {
            for incoming in bridge.poll() {
                if !self.monitored(incoming.agent) {
                    if let Some(reply) = incoming.reply { let _ = reply.send("{}".into()); }
                    continue;
                }
                let approval = self.config.agent(incoming.agent).approval;
                updates.extend(self.store.apply(incoming, now, approval));
            }
        }
        if now - self.last_tick >= 1.0 {
            self.last_tick = now;
            updates.extend(self.store.tick(now, self.config.retention_minutes));
        }
        updates
    }

    pub fn selected_session(&self) -> Option<&Session> {
        let key = self.selected.as_ref()?;
        self.store.sessions.iter().find(|session| &session.key == key)
    }

    pub fn respond(&mut self, key: &str, decision: Decision, now: f64) -> bool {
        let ok = self.store.respond(key, decision, now);
        if ok { self.detail = false; self.selected = None; self.free.clear(); self.free_focus = false; }
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_session::bridge::Incoming;

    fn event(agent: AgentId, name: &str, payload: serde_json::Value) -> Incoming {
        Incoming { id: 1, agent, event: name.into(), payload, reply: None, hosts: Vec::new() }
    }

    #[test]
    fn recent_exchange_is_kept_for_the_list_card() {
        let mut store = Store::default();
        store.apply(event(AgentId::Claude, "UserPromptSubmit", serde_json::json!({"session_id":"q1","cwd":"C:\\p","prompt":"修复登录接口"})), 0.0, true);
        store.apply(event(AgentId::Claude, "Stop", serde_json::json!({"session_id":"q1","last_assistant_message":"已修复并通过测试"})), 1.0, true);
        assert_eq!(store.sessions[0].prompt.as_deref(), Some("修复登录接口"));
        assert_eq!(store.sessions[0].reply.as_deref(), Some("已修复并通过测试"));
        assert_eq!(store.visible().len(), 1, "完成后的会话保留为卡片");
        assert!(store.active().is_empty(), "完成后的会话不再点亮常驻状态灯");
        // 下一轮开始后清掉上一轮回复，卡片只等本轮 Stop 的输出。
        store.apply(event(AgentId::Claude, "UserPromptSubmit", serde_json::json!({"session_id":"q1","prompt":"再跑一遍"})), 2.0, true);
        assert_eq!(store.sessions[0].prompt.as_deref(), Some("再跑一遍"));
        assert_eq!(store.sessions[0].reply, None);
    }

    #[test]
    fn answered_question_absorbs_the_followup_permission_request() {
        let mut store = Store::default();
        store.apply(event(AgentId::Claude, "PreToolUse", serde_json::json!({
            "session_id":"q2","cwd":"C:\\p","tool_name":"AskUserQuestion",
            "tool_input":{"questions":[{"question":"选哪个？","header":"选择","multiSelect":false,
                "options":[{"label":"A","description":""},{"label":"B","description":""}]}]}
        })), 0.0, true);
        assert_eq!(store.sessions[0].state, SessionState::WaitingAnswer);
        store.respond("claude:q2", Decision::Answers(vec![vec!["A".into()]]), 1.0);

        let (tx, rx) = mpsc::channel();
        let mut followup = event(AgentId::Claude, "PermissionRequest", serde_json::json!({
            "session_id":"q2","cwd":"C:\\p","tool_name":"AskUserQuestion","tool_input":{}
        }));
        followup.reply = Some(tx);
        let updates = store.apply(followup, 2.0, true);
        assert!(updates.is_empty(), "回答后不再弹出第二次审批");
        assert!(rx.try_recv().unwrap().contains("\"behavior\":\"allow\""));
        assert_eq!(store.sessions[0].state, SessionState::Running);
        assert!(store.sessions[0].pending.is_none());
    }

    #[test]
    fn terminal_question_absorbs_the_followup_permission_request() {
        let mut store = Store::default();
        store.apply(event(AgentId::Claude, "PreToolUse", serde_json::json!({
            "session_id":"q3","cwd":"C:\\p","tool_name":"AskUserQuestion",
            "tool_input":{"questions":[{"question":"选哪个？","header":"选择","multiSelect":false,
                "options":[{"label":"A","description":""}]}]}
        })), 0.0, true);
        store.respond("claude:q3", Decision::Terminal, 1.0);

        let (tx, rx) = mpsc::channel();
        let mut followup = event(AgentId::Claude, "PermissionRequest", serde_json::json!({
            "session_id":"q3","cwd":"C:\\p","tool_name":"AskUserQuestion","tool_input":{}
        }));
        followup.reply = Some(tx);
        store.apply(followup, 2.0, true);
        assert_eq!(rx.try_recv().unwrap(), "{}", "到终端处理后，第二次请求也交给终端");
        assert_eq!(store.sessions[0].state, SessionState::Running);
    }

    #[test]
    fn retention_snaps_to_five_minutes_and_keeps_one() {
        assert_eq!(snap_retention(0), 1);
        assert_eq!(snap_retention(1), 1);
        assert_eq!(snap_retention(2), 5);
        assert_eq!(snap_retention(30), 30);
        assert_eq!(snap_retention(58), 60);
        assert_eq!(snap_retention(99), 60);
    }

    #[test]
    fn stopping_monitoring_drops_sessions_and_pauses_polling() {
        let mut state = AgentState::default();
        state.active = true;
        state.store.apply(event(AgentId::Claude, "SessionStart", serde_json::json!({"session_id":"s1","cwd":"C:\\p"})), 0.0, true);
        state.detail = true;
        state.selected = Some("claude:s1".into());
        state.stop();
        assert!(!state.active);
        assert!(state.store.sessions.is_empty());
        assert!(!state.detail && state.selected.is_none());
        assert!(state.poll(1.0).is_empty(), "关闭后不再产生状态更新");
    }

    #[test]
    fn finished_sessions_keep_a_card_until_retention() {
        let mut store = Store::default();
        let payload = serde_json::json!({"session_id":"s1","cwd":"C:\\work\\proj"});
        store.apply(event(AgentId::Claude, "SessionStart", payload.clone()), 0.0, true);
        store.apply(event(AgentId::Claude, "UserPromptSubmit", serde_json::json!({"session_id":"s1","prompt":"fix login"})), 1.0, true);
        assert_eq!(store.visible().len(), 1);
        assert_eq!(store.visible()[0].title, "fix login");
        let updates = store.apply(event(AgentId::Claude, "Stop", payload.clone()), 2.0, true);
        assert!(updates.iter().any(|update| update.notice.as_ref().is_some_and(|notice| notice.kind == NoticeKind::Done)));
        assert_eq!(store.visible().len(), 1, "完成后卡片还在，打开列表时可见回复");
        assert!(store.active().is_empty());
        store.tick(2.0 + 31.0 * 60.0, 30);
        assert!(store.sessions.is_empty());
    }

    #[test]
    fn waiting_sessions_never_expire() {
        let mut store = Store::default();
        store.apply(event(AgentId::Claude, "PermissionRequest", serde_json::json!({"session_id":"s2","cwd":"C:\\p","tool_name":"Bash","tool_input":{"command":"dir"}})), 0.0, true);
        assert_eq!(store.visible().len(), 1);
        store.tick(10000.0, 1);
        assert_eq!(store.visible().len(), 1);
    }

    #[test]
    fn disabled_approval_returns_empty_decision_without_notice() {
        let mut store = Store::default();
        let (tx, rx) = mpsc::channel();
        let mut incoming = event(AgentId::Claude, "PermissionRequest", serde_json::json!({"session_id":"s3","cwd":"C:\\p","tool_name":"Bash"}));
        incoming.reply = Some(tx);
        let updates = store.apply(incoming, 0.0, false);
        assert!(updates.is_empty());
        assert_eq!(rx.try_recv().unwrap(), "{}");
        assert!(store.visible().is_empty());
    }

    #[test]
    fn waiting_requests_do_not_repeat() {
        let mut store = Store::default();
        store.apply(event(AgentId::Claude, "PermissionRequest", serde_json::json!({"session_id":"s4","cwd":"C:\\p","tool_name":"Bash"})), 0.0, true);
        assert!(store.tick(300.0, 30).iter().all(|update| update.notice.is_none()));
    }

    #[test]
    fn idle_compaction_keeps_the_session_finished_and_silent() {
        let mut store = Store::default();
        let started = store.apply(event(AgentId::Claude, "SessionStart", serde_json::json!({"session_id":"c1","cwd":"C:\\p","source":"startup"})), 0.0, true);
        assert_eq!(started[0].sound, Some(SoundKind::Start));
        store.apply(event(AgentId::Claude, "Stop", serde_json::json!({"session_id":"c1"})), 1.0, true);
        assert_eq!(store.visible().len(), 1);
        assert!(store.active().is_empty());
        let compact = store.apply(event(AgentId::Claude, "SessionStart", serde_json::json!({"session_id":"c1","cwd":"C:\\p","source":"compact"})), 2.0, true);
        assert!(compact.iter().all(|update| update.sound.is_none()));
        assert!(store.active().is_empty());
        assert_eq!(store.sessions[0].state, SessionState::Completed);
    }

    #[test]
    fn idle_sessions_keep_a_card_but_do_not_light_the_bar() {
        let mut store = Store::default();
        store.apply(event(AgentId::Claude, "SessionStart", serde_json::json!({"session_id":"i1","cwd":"C:\\work\\proj"})), 0.0, true);
        store.apply(event(AgentId::Claude, "Notification", serde_json::json!({"session_id":"i1","notification_type":"idle_prompt","message":"Claude is waiting for your input"})), 1.0, true);
        assert_eq!(store.sessions[0].state, SessionState::Idle);
        assert_eq!(store.visible().len(), 1, "已激活的闲置会话保留卡片");
        assert!(store.active().is_empty());
        assert!(!store.has_active());
        assert_eq!(store.visible()[0].last_message.as_deref(), Some("Claude is waiting for your input"));
    }

    #[test]
    fn the_first_prompt_names_the_session() {
        let mut store = Store::default();
        store.apply(event(AgentId::Claude, "SessionStart", serde_json::json!({"session_id":"t1","cwd":"C:\\work\\proj"})), 0.0, true);
        assert_eq!(store.sessions[0].title, "proj");
        store.apply(event(AgentId::Claude, "UserPromptSubmit", serde_json::json!({"session_id":"t1","prompt":"修复登录接口"})), 1.0, true);
        assert_eq!(store.sessions[0].title, "修复登录接口");
        store.apply(event(AgentId::Claude, "UserPromptSubmit", serde_json::json!({"session_id":"t1","prompt":"再跑一遍测试"})), 2.0, true);
        assert_eq!(store.sessions[0].title, "修复登录接口", "标题取第一条提问，不随追问变化");
        store.apply(event(AgentId::Claude, "SessionStart", serde_json::json!({"session_id":"t1","cwd":"C:\\work\\proj","source":"compact"})), 3.0, true);
        assert_eq!(store.sessions[0].title, "修复登录接口", "压缩不重置标题");
    }

    #[test]
    fn interrupt_is_reported_as_a_finished_session() {
        let mut store = Store::default();
        store.apply(event(AgentId::Codex, "SessionStart", serde_json::json!({"session_id":"x1","cwd":"C:\\p"})), 0.0, true);
        store.apply(event(AgentId::Codex, "UserPromptSubmit", serde_json::json!({"session_id":"x1","prompt":"run"})), 1.0, true);
        let updates = store.apply(event(AgentId::Codex, "Interrupt", serde_json::json!({"session_id":"x1"})), 2.0, true);
        assert_eq!(store.sessions[0].state, SessionState::Interrupted);
        assert!(updates.iter().any(|update| update.notice.as_ref().is_some_and(|notice| notice.kind == NoticeKind::Interrupted)));
        assert_eq!(updates.iter().find_map(|update| update.sound), Some(SoundKind::Error));
        assert_eq!(store.visible().len(), 1);
        assert!(store.active().is_empty());
    }

    #[test]
    fn notify_splits_question_and_approval_and_migrates_legacy_waiting() {
        let legacy: NotifyConfig = serde_json::from_value(serde_json::json!({"waiting": false})).unwrap();
        assert!(!legacy.enabled(NoticeKind::Question));
        assert!(!legacy.enabled(NoticeKind::Approval));
        let mut config = NotifyConfig::default();
        config.question = Some(false);
        assert!(!config.enabled(NoticeKind::Question));
        assert!(config.enabled(NoticeKind::Approval));
        config.approval = Some(false);
        assert!(!config.enabled(NoticeKind::Approval));
        let saved = serde_json::to_value(&config).unwrap();
        assert!(saved.get("waiting").is_none(), "旧合并键不再写回");
        assert_eq!(saved["question"], false);
    }

    #[test]
    fn compaction_during_a_turn_still_plays_the_start_sound() {
        let mut store = Store::default();
        store.apply(event(AgentId::Claude, "SessionStart", serde_json::json!({"session_id":"c2","cwd":"C:\\p","source":"startup"})), 0.0, true);
        store.apply(event(AgentId::Claude, "UserPromptSubmit", serde_json::json!({"session_id":"c2","prompt":"fix"})), 1.0, true);
        let compact = store.apply(event(AgentId::Claude, "SessionStart", serde_json::json!({"session_id":"c2","cwd":"C:\\p","source":"compact"})), 2.0, true);
        assert_eq!(compact[0].sound, Some(SoundKind::Start));
        assert_eq!(store.sessions[0].state, SessionState::Running);
    }
}
