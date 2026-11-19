//! Hook 安装、修复、移除与旧配置清理。只操作岛仔自己的条目，写入前备份。

use super::AgentId;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health { Installed, NeedsRepair, NotInstalled, NotDetected, Unsupported }

#[derive(Clone, Debug)]
pub struct Status {
    pub agent: AgentId,
    pub health: Health,
    pub path: Option<PathBuf>,
    pub note: String,
}

impl Status {
    pub fn unknown(agent: AgentId) -> Self {
        Self { agent, health: Health::NotDetected, path: None, note: "尚未检测".into() }
    }
}

fn home() -> PathBuf {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default()
}

/// Claude Code 把会话的自动标题写在 `~/.claude/sessions/<pid>.json` 的 `name` 里；
/// 按 session_id 找到它。其他 Agent 没有等价的元数据。
pub(crate) fn session_title(agent: AgentId, id: &str) -> Option<String> {
    if cfg!(test) || agent != AgentId::Claude || id.is_empty() { return None; }
    title_in(&home().join(".claude").join("sessions"), agent, id)
}

fn title_in(dir: &Path, agent: AgentId, id: &str) -> Option<String> {
    if agent != AgentId::Claude || id.is_empty() { return None; }
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") { continue; }
        let Ok(raw) = std::fs::read(&path) else { continue; };
        let Ok(value) = serde_json::from_slice::<Value>(&raw) else { continue; };
        if value.get("sessionId").and_then(Value::as_str) == Some(id) {
            let name = value.get("name").and_then(Value::as_str).unwrap_or("").trim();
            if !name.is_empty() { return Some(name.chars().take(40).collect()); }
        }
    }
    None
}

pub fn config_path(agent: AgentId) -> Option<PathBuf> {
    match agent {
        AgentId::Claude => Some(home().join(".claude").join("settings.json")),
        AgentId::Codex => Some(home().join(".codex").join("hooks.json")),
        AgentId::ZCode => Some(home().join(".zcode").join("cli").join("config.json")),
        AgentId::Copilot => Some(home().join(".copilot").join("hooks").join("daozai.json")),
        AgentId::Dsh => None,
    }
}

fn which(exe: &str) -> bool {
    let extensions: &[&str] = if cfg!(windows) { &["", ".exe", ".cmd", ".bat"] } else { &[""] };
    std::env::var_os("PATH").map(|paths| {
        std::env::split_paths(&paths).any(|dir| extensions.iter().any(|extension| dir.join(format!("{exe}{extension}")).is_file()))
    }).unwrap_or(false)
}

pub fn detected(agent: AgentId) -> bool {
    match agent {
        AgentId::Claude => home().join(".claude").is_dir() || which("claude"),
        AgentId::Codex => home().join(".codex").is_dir() || which("codex"),
        AgentId::ZCode => home().join(".zcode").is_dir() || which("zcode"),
        AgentId::Copilot => home().join(".vscode").is_dir() || which("code"),
        AgentId::Dsh => home().join(".dsh").is_dir() || home().join(".dsh-beta").is_dir()
            || std::env::var_os("APPDATA").is_some_and(|dir| PathBuf::from(dir).join("dsh-desktop").join("harness").is_dir()),
    }
}

pub fn status(agent: AgentId) -> Status {
    let Some(path) = config_path(agent) else {
        let note = if agent == AgentId::Dsh { "文件监听，安装后接入" } else { "Hook 注入位置待验证" };
        return Status { agent, health: Health::Unsupported, path: None, note: note.into() };
    };
    if !detected(agent) {
        return Status { agent, health: Health::NotDetected, path: Some(path), note: "未检测到应用".into() };
    }
    let Ok(bytes) = std::fs::read(&path) else {
        return Status { agent, health: Health::NotInstalled, path: Some(path), note: "配置尚未创建".into() };
    };
    let Ok(root) = serde_json::from_slice::<Value>(&bytes) else {
        return Status { agent, health: Health::NeedsRepair, path: Some(path), note: "配置无法解析，未写入".into() };
    };
    let entries = hook_entries(&root, agent);
    let installed = entries.iter().filter(|(_, value)| contains_marker(value, agent)).count();
    if installed == 0 {
        return Status { agent, health: Health::NotInstalled, path: Some(path), note: "尚未安装".into() };
    }
    let expected = agent.hook_events().iter().filter(|event| entries.iter().any(|(name, value)| name == event.name && contains_marker(value, agent))).count();
    if expected < agent.hook_events().len() {
        return Status { agent, health: Health::NeedsRepair, path: Some(path), note: format!("缺少 {} 个事件", agent.hook_events().len() - expected) };
    }
    let exe = std::env::current_exe().ok();
    let current = exe.as_ref().is_some_and(|exe| entries.iter().any(|(_, value)| contains_path(value, exe)));
    if !current {
        return Status { agent, health: Health::NeedsRepair, path: Some(path), note: "Hook 指向旧版本，点修复更新".into() };
    }
    Status { agent, health: Health::Installed, path: Some(path), note: "已接入".into() }
}

fn hook_entries<'a>(root: &'a Value, agent: AgentId) -> Vec<(String, &'a Value)> {
    if agent == AgentId::ZCode {
        root.get("hooks").and_then(|hooks| hooks.get("events")).and_then(Value::as_object)
            .map(|events| events.iter().map(|(name, value)| (name.clone(), value)).collect())
            .unwrap_or_default()
    } else {
        root.get("hooks").and_then(Value::as_object)
            .map(|hooks| hooks.iter().map(|(name, value)| (name.clone(), value)).collect())
            .unwrap_or_default()
    }
}

fn contains_marker(value: &Value, agent: AgentId) -> bool {
    match value {
        Value::String(text) => text.contains(&format!("--agent-hook {}", agent.key())),
        Value::Array(items) => items.windows(2).any(|pair| pair[0].as_str() == Some("--agent-hook") && pair[1].as_str() == Some(agent.key()))
            || items.iter().any(|item| contains_marker(item, agent)),
        Value::Object(map) => map.values().any(|item| contains_marker(item, agent)),
        _ => false,
    }
}

fn contains_path(value: &Value, exe: &Path) -> bool {
    let exe = exe.to_string_lossy();
    match value {
        Value::String(text) => text.contains(exe.as_ref()),
        Value::Array(items) => items.iter().any(|item| contains_path(item, exe.as_ref().as_ref())),
        Value::Object(map) => map.values().any(|item| contains_path(item, exe.as_ref().as_ref())),
        _ => false,
    }
}

fn backup(path: &Path) -> Result<PathBuf, String> {
    let dir = crate::model::config_dir().join("agent-hook-backups");
    std::fs::create_dir_all(&dir).map_err(|error| format!("无法创建备份目录：{error}"))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("hooks.json");
    let backup = dir.join(format!("{name}.{stamp}.bak"));
    std::fs::copy(path, &backup).map_err(|error| format!("备份失败：{error}"))?;
    Ok(backup)
}

fn write_json(path: &Path, root: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(root).map_err(|error| format!("序列化失败：{error}"))?;
    crate::model::atomic_write(path, &bytes).map_err(|error| format!("写入失败：{error}"))
}

fn read_config(path: &Path) -> Result<Value, String> {
    if !path.exists() { return Ok(json!({})); }
    let bytes = std::fs::read(path).map_err(|error| format!("读取配置失败：{error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("配置不是有效 JSON，未写入：{error}"))
}

pub fn install(agent: AgentId, exe: &Path) -> Result<String, String> {
    let Some(path) = config_path(agent) else {
        return Err("DSH 没有 Hook，改用文件监听".into());
    };
    let mut root = read_config(&path)?;
    let changed = {
        match agent {
            AgentId::Claude | AgentId::Codex => {
                let hooks = root.as_object_mut().ok_or("配置根节点不是对象")?
                    .entry("hooks").or_insert_with(|| json!({}));
                let hooks = hooks.as_object_mut().ok_or("hooks 不是对象")?;
                for event in agent.hook_events() {
                    let list = hooks.entry(event.name).or_insert_with(|| json!([]));
                    let list = list.as_array_mut().ok_or_else(|| format!("{} 不是数组", event.name))?;
                    list.retain(|group| !contains_marker(group, agent));
                    // Claude 直接传命令与参数；Codex 在 Windows 上用 PowerShell 风格执行命令串。
                    let entry = if agent == AgentId::Claude {
                        json!({"hooks":[{"type":"command","command":exe.display().to_string(),"args":["--agent-hook","claude","--event",event.name],"timeout":event.timeout}]})
                    } else {
                        let command = format!("& '{}' --agent-hook codex --event {}", exe.display(), event.name);
                        json!({"hooks":[{"type":"command","command":command,"commandWindows":command,"timeout":event.timeout}]})
                    };
                    list.push(entry);
                }
            }
            AgentId::ZCode => {
                let hooks = root.as_object_mut().ok_or("配置根节点不是对象")?
                    .entry("hooks").or_insert_with(|| json!({}));
                let hooks = hooks.as_object_mut().ok_or("hooks 不是对象")?;
                hooks.insert("enabled".into(), json!(true));
                let events = hooks.entry("events").or_insert_with(|| json!({}));
                let events = events.as_object_mut().ok_or("hooks.events 不是对象")?;
                for event in agent.hook_events() {
                    let list = events.entry(event.name).or_insert_with(|| json!([]));
                    let list = list.as_array_mut().ok_or_else(|| format!("{} 不是数组", event.name))?;
                    list.retain(|entry| !contains_marker(entry, agent));
                    list.push(json!({
                        "type":"process",
                        "command":exe.display().to_string(),
                        "args":["--agent-hook","zcode","--event",event.name],
                        "timeoutMs":event.timeout*1000
                    }));
                }
            }
            AgentId::Copilot => {
                let hooks = root.as_object_mut().ok_or("配置根节点不是对象")?
                    .entry("hooks").or_insert_with(|| json!({}));
                let hooks = hooks.as_object_mut().ok_or("hooks 不是对象")?;
                for event in agent.hook_events() {
                    let command = format!("cmd.exe /d /s /c \"\"{}\" --agent-hook copilot --event {}\"", exe.display(), event.name);
                    let list = hooks.entry(event.name).or_insert_with(|| json!([]));
                    let list = list.as_array_mut().ok_or_else(|| format!("{} 不是数组", event.name))?;
                    list.retain(|entry| !contains_marker(entry, agent));
                    list.push(json!({"type":"command","command":command,"timeout":event.timeout}));
                }
            }
            AgentId::Dsh => unreachable!(),
        }
        true
    };
    if changed {
        if path.exists() { let _ = backup(&path)?; }
        write_json(&path, &root)?;
    }
    if agent == AgentId::Copilot {
        Ok(format!("已写入 {} 个事件；还需在 VS Code 设置中开启 chat.useHooks", agent.hook_events().len()))
    } else {
        Ok(format!("已写入 {} 个事件", agent.hook_events().len()))
    }
}

pub fn remove(agent: AgentId) -> Result<String, String> {
    let Some(path) = config_path(agent) else { return Err("该 Agent 不使用 Hook".into()); };
    if !path.exists() { return Ok("没有需要移除的 Hook".into()); }
    let mut root = read_config(&path)?;
    let before = root.clone();
    if agent == AgentId::ZCode {
        if let Some(events) = root.get_mut("hooks").and_then(|hooks| hooks.get_mut("events")).and_then(Value::as_object_mut) {
            let names: Vec<String> = events.keys().cloned().collect();
            for name in names {
                if let Some(list) = events.get_mut(&name).and_then(Value::as_array_mut) {
                    list.retain(|entry| !contains_marker(entry, agent));
                    if list.is_empty() { events.remove(&name); }
                }
            }
        }
    } else if let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) {
        let names: Vec<String> = hooks.keys().cloned().collect();
        for name in names {
            if let Some(list) = hooks.get_mut(&name).and_then(Value::as_array_mut) {
                list.retain(|group| !contains_marker(group, agent));
                if list.is_empty() { hooks.remove(&name); }
            }
        }
    }
    if root != before {
        let _ = backup(&path)?;
        write_json(&path, &root)?;
        Ok("已移除岛仔的 Hook".into())
    } else {
        Ok("没有需要移除的 Hook".into())
    }
}

const LEGACY_MARKERS: &[&str] = &["focusd", "127.0.0.1:47830", "play-sound.ps1"];

fn legacy(value: &Value) -> bool {
    let text = value.to_string();
    LEGACY_MARKERS.iter().any(|marker| text.contains(marker))
}

/// 删除一对标记行之间的文本块；块位于文件末尾时保留单个换行。
fn strip_marked_block(text: &str, begin: &str, end: &str) -> Option<String> {
    let start = text.find(begin)?;
    let stop = text.find(end)? + end.len();
    if stop <= start { return None; }
    let mut head = text[..start].trim_end_matches(['\r', '\n']).to_string();
    let tail = text[stop..].trim_start_matches(['\r', '\n']);
    if tail.is_empty() {
        head.push_str("\r\n");
    } else {
        head.push_str("\r\n\r\n");
        head.push_str(tail);
    }
    Some(head)
}

/// 旧版 Fly Island 把 Codex Hook 写在 `config.toml` 的标记块里，按块删除。
fn cleanup_codex_toml() -> bool {
    let path = home().join(".codex").join("config.toml");
    let Ok(raw) = std::fs::read_to_string(&path) else { return false; };
    let Some(cleaned) = strip_marked_block(&raw, "# BEGIN FocuSD Agent Status Hooks", "# END FocuSD Agent Status Hooks") else { return false; };
    if cleaned == raw { return false; }
    let _ = backup(&path);
    crate::model::atomic_write(&path, cleaned.as_bytes()).is_ok()
}

/// 启动时清理 Fly Island 与独立提示音 Hook；保留 RTK、Pebrel 等其他工具。
pub fn cleanup_legacy() -> bool {
    let mut changed = false;
    for agent in [AgentId::Claude, AgentId::Codex] {
        let Some(path) = config_path(agent) else { continue; };
        let Ok(bytes) = std::fs::read(&path) else { continue; };
        let Ok(mut root) = serde_json::from_slice::<Value>(&bytes) else { continue; };
        let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else { continue; };
        let before = hooks.clone();
        let names: Vec<String> = hooks.keys().cloned().collect();
        for name in names {
            if let Some(list) = hooks.get_mut(&name).and_then(Value::as_array_mut) {
                list.retain(|group| !legacy(group));
                if list.is_empty() { hooks.remove(&name); }
            }
        }
        if *hooks != before {
            let _ = backup(&path);
            if write_json(&path, &root).is_ok() { changed = true; }
        }
    }
    if cleanup_codex_toml() { changed = true; }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_only_match_our_agent() {
        let claude = json!({"hooks":[{"type":"command","command":"\"C:\\d.exe\" --agent-hook claude --event Stop"}]});
        let zcode = json!({"type":"process","command":"C:\\d.exe","args":["--agent-hook","zcode","--event","Stop"]});
        assert!(contains_marker(&claude, AgentId::Claude));
        assert!(!contains_marker(&claude, AgentId::Codex));
        assert!(contains_marker(&zcode, AgentId::ZCode));
        assert!(!contains_marker(&zcode, AgentId::Claude));
    }

    #[test]
    fn legacy_markers_do_not_touch_rtk_or_pebrel() {
        assert!(legacy(&json!({"command":"cmd.exe","args":["com.focusd.island\\focusd-agent-running.cmd"]})));
        assert!(legacy(&json!({"command":"curl.exe","args":["http://127.0.0.1:47830/claude/activity"]})));
        assert!(legacy(&json!({"command":"powershell.exe -File play-sound.ps1 -Sound start"})));
        assert!(!legacy(&json!({"command":"rtk hook claude"})));
        assert!(!legacy(&json!({"command":"D:/Program Files/Pebrel/runtime/pebrel-hook.exe"})));
    }

    #[test]
    fn claude_session_title_comes_from_live_session_metadata() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "daozai-agent-title-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("123.json"), r#"{"sessionId":"abc","name":"灵动岛Agent会话监控PRD"}"#).unwrap();
        assert_eq!(title_in(&dir, AgentId::Claude, "abc").as_deref(), Some("灵动岛Agent会话监控PRD"));
        assert_eq!(title_in(&dir, AgentId::Claude, "missing"), None);
        assert_eq!(title_in(&dir, AgentId::Codex, "abc"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn marked_toml_block_is_removed_without_touching_other_tables() {
        let text = "[a]\r\nvalue = 1\r\n\r\n# BEGIN FocuSD Agent Status Hooks\r\n[[hooks.Stop]]\r\ncommand = \"focusd\"\r\n# END FocuSD Agent Status Hooks\r\n\r\n[b]\r\nvalue = 2\r\n";
        let cleaned = strip_marked_block(text, "# BEGIN FocuSD Agent Status Hooks", "# END FocuSD Agent Status Hooks").unwrap();
        assert_eq!(cleaned, "[a]\r\nvalue = 1\r\n\r\n[b]\r\nvalue = 2\r\n");
        assert!(!cleaned.contains("focusd"));
        let tail = "[a]\r\n\r\n# BEGIN FocuSD Agent Status Hooks\r\nx = 1\r\n# END FocuSD Agent Status Hooks";
        assert_eq!(strip_marked_block(tail, "# BEGIN FocuSD Agent Status Hooks", "# END FocuSD Agent Status Hooks").unwrap(), "[a]\r\n");
        assert!(strip_marked_block("[a]\r\n", "# BEGIN FocuSD Agent Status Hooks", "# END FocuSD Agent Status Hooks").is_none());
    }
}
