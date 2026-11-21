//! AI 会话：Agent 描述表、统一事件、Hook 安装与会话状态。
//!
//! 新增 Agent 只需在这里补一条描述和事件表，安装器、状态机和页面都复用。

pub mod bridge;
pub mod focus;
pub mod hooks;
pub mod store;

pub use store::{AgentState, Config, Session, SessionState};

pub const AGENT_COUNT: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentId {
    Claude,
    Codex,
    ZCode,
    Copilot,
    Dsh,
}

impl AgentId {
    pub const ALL: [AgentId; AGENT_COUNT] = [Self::Claude, Self::Codex, Self::ZCode, Self::Copilot, Self::Dsh];

    pub fn index(self) -> usize {
        match self {
            Self::Claude => 0,
            Self::Codex => 1,
            Self::ZCode => 2,
            Self::Copilot => 3,
            Self::Dsh => 4,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::ZCode => "zcode",
            Self::Copilot => "copilot",
            Self::Dsh => "dsh",
        }
    }

    pub fn display(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
            Self::ZCode => "ZCode",
            Self::Copilot => "VS Code Copilot",
            Self::Dsh => "DeepSeek Harness",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
            Self::ZCode => "ZCode",
            Self::Copilot => "Copilot",
            Self::Dsh => "DSH",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|agent| agent.key() == key)
    }

    pub fn capabilities(self) -> Capabilities {
        match self {
            Self::Claude => Capabilities { approve: true, question: true, focus: true },
            Self::Codex => Capabilities { approve: true, question: false, focus: true },
            Self::ZCode => Capabilities { approve: true, question: false, focus: true },
            Self::Copilot => Capabilities { approve: false, question: false, focus: true },
            Self::Dsh => Capabilities { approve: false, question: false, focus: false },
        }
    }

    pub fn hook_events(self) -> &'static [HookEvent] {
        match self {
            Self::Claude => CLAUDE_EVENTS,
            Self::Codex => CODEX_EVENTS,
            Self::ZCode => ZCODE_EVENTS,
            Self::Copilot => COPILOT_EVENTS,
            Self::Dsh => &[],
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Capabilities {
    pub approve: bool,
    pub question: bool,
    pub focus: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct HookEvent {
    pub name: &'static str,
    /// Hook 进程自身的最长等待时间（秒）。
    pub timeout: u64,
}

const CLAUDE_EVENTS: &[HookEvent] = &[
    HookEvent { name: "SessionStart", timeout: 10 },
    HookEvent { name: "UserPromptSubmit", timeout: 10 },
    HookEvent { name: "PreToolUse", timeout: 86400 },
    HookEvent { name: "PermissionRequest", timeout: 86400 },
    HookEvent { name: "PostToolUse", timeout: 10 },
    HookEvent { name: "PostToolUseFailure", timeout: 10 },
    HookEvent { name: "Notification", timeout: 10 },
    HookEvent { name: "Stop", timeout: 10 },
    HookEvent { name: "StopFailure", timeout: 10 },
    HookEvent { name: "SessionEnd", timeout: 10 },
];

const CODEX_EVENTS: &[HookEvent] = &[
    HookEvent { name: "SessionStart", timeout: 10 },
    HookEvent { name: "UserPromptSubmit", timeout: 10 },
    HookEvent { name: "PreToolUse", timeout: 10 },
    HookEvent { name: "PermissionRequest", timeout: 86400 },
    HookEvent { name: "PostToolUse", timeout: 10 },
    HookEvent { name: "Stop", timeout: 10 },
    HookEvent { name: "Interrupt", timeout: 10 },
    HookEvent { name: "SessionEnd", timeout: 10 },
];

const ZCODE_EVENTS: &[HookEvent] = &[
    HookEvent { name: "SessionStart", timeout: 10 },
    HookEvent { name: "UserPromptSubmit", timeout: 10 },
    HookEvent { name: "PreToolUse", timeout: 10 },
    HookEvent { name: "PermissionRequest", timeout: 86400 },
    HookEvent { name: "PostToolUse", timeout: 10 },
    HookEvent { name: "PostToolUseFailure", timeout: 10 },
    HookEvent { name: "Stop", timeout: 10 },
];

/// VS Code Copilot Chat 的 Hook 事件（`chat.useHooks` 预览功能）。
const COPILOT_EVENTS: &[HookEvent] = &[
    HookEvent { name: "SessionStart", timeout: 10 },
    HookEvent { name: "SessionEnd", timeout: 10 },
    HookEvent { name: "UserPromptSubmit", timeout: 10 },
    HookEvent { name: "PreToolUse", timeout: 10 },
    HookEvent { name: "PostToolUse", timeout: 10 },
    HookEvent { name: "Stop", timeout: 10 },
    HookEvent { name: "ErrorOccurred", timeout: 10 },
];

#[derive(Clone, Debug)]
pub struct ApprovalRequest {
    pub tool: String,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct Question {
    pub question: String,
    pub header: String,
    pub options: Vec<QuestionOption>,
    pub multi: bool,
}

#[derive(Clone, Debug)]
pub struct QuestionOption {
    pub label: String,
    pub description: String,
}

#[derive(Clone, Debug)]
pub enum EventKind {
    SessionStart { cwd: String, source: String },
    SessionEnd,
    Prompt { text: String },
    ToolStart { tool: String, detail: String },
    ToolEnd { ok: bool },
    Permission(ApprovalRequest),
    Question(Vec<Question>),
    Message(String),
    Stop { message: Option<String> },
    Failure(String),
    Interrupt,
}

/// 把各 Agent 的原始 Hook payload 归一化为统一事件。
pub fn normalize(agent: AgentId, event: &str, payload: &serde_json::Value) -> Option<EventKind> {
    let tool = payload.get("tool_name").and_then(serde_json::Value::as_str).unwrap_or("").to_string();
    match event {
        "SessionStart" => Some(EventKind::SessionStart {
            cwd: string_field(payload, &["cwd", "workspace", "project_dir", "working_directory"]),
            source: string_field(payload, &["source"]),
        }),
        "SessionEnd" => Some(EventKind::SessionEnd),
        "UserPromptSubmit" => Some(EventKind::Prompt { text: prompt_text(payload) }),
        "PreToolUse" => {
            if tool == "AskUserQuestion" {
                let questions = parse_questions(payload.get("tool_input"));
                if questions.is_empty() { None } else { Some(EventKind::Question(questions)) }
            } else {
                Some(EventKind::ToolStart { detail: tool_detail(payload), tool })
            }
        }
        "PermissionRequest" => Some(EventKind::Permission(ApprovalRequest { tool, detail: tool_detail(payload) })),
        "PostToolUse" => Some(EventKind::ToolEnd { ok: true }),
        "PostToolUseFailure" => Some(EventKind::ToolEnd { ok: false }),
        "Notification" => Some(EventKind::Message(notification_text(payload))),
        "Stop" => Some(EventKind::Stop { message: stop_message(payload) }),
        "StopFailure" => Some(EventKind::Failure(notification_text(payload))),
        "ErrorOccurred" => Some(EventKind::Failure(notification_text(payload))),
        "Interrupt" => Some(EventKind::Interrupt),
        // 子代理、压缩等事件不进入主会话状态。
        _ => {
            let _ = agent;
            None
        }
    }
}

pub fn session_id(payload: &serde_json::Value) -> String {
    let id = string_field(payload, &["session_id", "sessionId", "thread_id", "conversation_id", "id"]);
    if !id.is_empty() { id } else {
        let cwd = string_field(payload, &["cwd", "workspace", "workspaceFolder", "project_dir"]);
        if cwd.is_empty() { "default".into() } else { cwd }
    }
}

pub fn cwd(payload: &serde_json::Value) -> String {
    string_field(payload, &["cwd", "workspace", "workspaceFolder", "project_dir", "working_directory"])
}

pub fn prompt_text(payload: &serde_json::Value) -> String {
    let text = string_field(payload, &["prompt", "message", "text"]);
    text.lines().next().unwrap_or("").chars().take(240).collect()
}

/// `Stop` 可带最后一轮回复（Claude Code 的 `last_assistant_message`），详情页只展示摘要。
fn stop_message(payload: &serde_json::Value) -> Option<String> {
    let raw = string_field(payload, &["last_assistant_message", "lastMessage", "last_message"]);
    let text = markdown_text(&raw);
    let text = if text.is_empty() { raw } else { text };
    (!text.is_empty()).then(|| text.chars().take(400).collect())
}

/// 把常见 Markdown 标记转成适合岛上窄行显示的纯文本。
fn markdown_text(text: &str) -> String {
    let mut lines = Vec::new();
    for raw in text.lines() {
        let mut line = raw.trim();
        if line.is_empty() || line.starts_with("```") || line.starts_with("~~~") { continue; }
        // 表格分隔线和水平线。
        if line.starts_with('|') && line.ends_with('|') && line.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')) { continue; }
        if line.chars().all(|c| matches!(c, '-' | '*' | '_' | ' ')) { continue; }
        while let Some(rest) = line.strip_prefix('>') { line = rest.trim_start(); }
        if line.starts_with('#') { line = line.trim_start_matches('#').trim_start(); }
        if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")).or_else(|| line.strip_prefix("+ ")) {
            line = rest;
        } else if let Some((number, rest)) = line.split_once(". ") {
            if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) { line = rest; }
        }
        let line = if line.starts_with('|') && line.ends_with('|') {
            line.trim_matches('|').split('|').map(str::trim).filter(|cell| !cell.is_empty()).collect::<Vec<_>>().join(" · ")
        } else {
            line.to_owned()
        };
        let line = strip_links(&line).replace("**", "").replace("__", "").replace("~~", "").replace('`', "");
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if !line.is_empty() { lines.push(line); }
    }
    lines.join("\n")
}

/// `[文字](地址)` 只保留文字；图片同样只保留替代文字。
fn strip_links(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '[' {
            if let Some(close) = chars[index + 1..].iter().position(|c| *c == ']').map(|offset| index + 1 + offset) {
                if close + 1 < chars.len() && chars[close + 1] == '(' {
                    if let Some(paren) = chars[close + 2..].iter().position(|c| *c == ')').map(|offset| close + 2 + offset) {
                        out.extend(&chars[index + 1..close]);
                        index = paren + 1;
                        continue;
                    }
                }
            }
        }
        out.push(chars[index]);
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::markdown_text;

    #[test]
    fn markdown_reply_is_plain_text() {
        let text = markdown_text("## 标题\n\n**完成**：`cargo test` 通过\n- [链接](https://example.com)\n| 项 | 值 |\n|---|---|\n| 结果 | 成功 |\n```\ncode\n```");
        assert_eq!(text, "标题\n完成：cargo test 通过\n链接\n项 · 值\n结果 · 成功\ncode");
    }
}

fn string_field(payload: &serde_json::Value, keys: &[&str]) -> String {
    keys.iter().find_map(|key| payload.get(*key).and_then(serde_json::Value::as_str)).unwrap_or("").to_string()
}

fn tool_detail(payload: &serde_json::Value) -> String {
    let input = payload.get("tool_input").unwrap_or(payload);
    for key in ["command", "file_path", "path", "url", "pattern", "query"] {
        if let Some(value) = input.get(key).and_then(serde_json::Value::as_str) {
            return value.chars().take(240).collect();
        }
    }
    let text = input.to_string();
    text.chars().take(240).collect()
}

fn notification_text(payload: &serde_json::Value) -> String {
    let text = string_field(payload, &["message", "reason", "error"]);
    if text.is_empty() {
        string_field(payload, &["notification_type"])
    } else {
        text.lines().next().unwrap_or("").chars().take(120).collect()
    }
}

fn parse_questions(input: Option<&serde_json::Value>) -> Vec<Question> {
    let Some(questions) = input.and_then(|value| value.get("questions")).and_then(serde_json::Value::as_array) else { return Vec::new(); };
    questions.iter().filter_map(|question| {
        let text = question.get("question").and_then(serde_json::Value::as_str)?;
        let options = question.get("options").and_then(serde_json::Value::as_array).map(|options| options.iter().filter_map(|option| {
            let label = option.get("label").and_then(serde_json::Value::as_str)?;
            Some(QuestionOption {
                label: label.to_string(),
                description: option.get("description").and_then(serde_json::Value::as_str).unwrap_or("").to_string(),
            })
        }).collect()).unwrap_or_default();
        Some(Question {
            question: text.to_string(),
            header: question.get("header").and_then(serde_json::Value::as_str).unwrap_or("").to_string(),
            options,
            multi: question.get("multiSelect").and_then(serde_json::Value::as_bool).unwrap_or(false),
        })
    }).collect()
}

/// 判断这次 Hook 调用是否需要等待岛上决定。
pub fn blocking_event(event: &str, payload: &serde_json::Value) -> bool {
    event == "PermissionRequest"
        || (event == "PreToolUse" && payload.get("tool_name").and_then(serde_json::Value::as_str) == Some("AskUserQuestion"))
}
