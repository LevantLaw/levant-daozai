//! 本地中继：`daozai.exe --agent-hook <agent> --event <event>` 把 Hook 送进岛仔，
//! 阻塞事件等待岛上决定，其余事件立即返回。

use super::AgentId;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MAX_BODY: usize = 256 * 1024;
const WAIT_SECONDS: u64 = 90;

#[derive(Debug)]
pub struct Incoming {
    pub id: u64,
    pub agent: AgentId,
    pub event: String,
    pub payload: serde_json::Value,
    /// 中继进程的祖先链，用于跳转时定位会话窗口。
    pub hosts: Vec<(u32, String)>,
    pub reply: Option<Sender<String>>,
}

pub struct Bridge {
    pub port: u16,
    pub token: String,
    rx: Receiver<Incoming>,
    path: std::path::PathBuf,
}

impl Bridge {
    pub fn start(ctx: eframe::egui::Context) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        let token = new_token();
        let path = crate::model::config_dir().join("agent-bridge.json");
        let body = serde_json::json!({ "port": port, "token": token, "pid": std::process::id() });
        let _ = crate::model::atomic_write(&path, &serde_json::to_vec(&body).unwrap_or_default());
        let (tx, rx) = mpsc::channel();
        let pending: Arc<Mutex<HashMap<u64, Sender<String>>>> = Arc::default();
        let thread_token = token.clone();
        std::thread::Builder::new().name("daozai-agent-bridge".into()).spawn(move || {
            for stream in listener.incoming().flatten() {
                let tx = tx.clone();
                let pending = pending.clone();
                let token = thread_token.clone();
                let ctx = ctx.clone();
                std::thread::spawn(move || handle(stream, tx, pending, token, ctx));
            }
        })?;
        Ok(Self { port, token, rx, path })
    }

    pub fn poll(&self) -> Vec<Incoming> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() { events.push(event); }
        events
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn new_token() -> String {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut bytes = [0u8; 16];
    let _ = SystemRandom::new().fill(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(serde::Deserialize)]
struct Request {
    agent: String,
    event: String,
    #[serde(default)]
    blocking: bool,
    #[serde(default)]
    payload: serde_json::Value,
    #[serde(default)]
    hosts: Vec<(u32, String)>,
}

fn handle(mut stream: TcpStream, tx: Sender<Incoming>, pending: Arc<Mutex<HashMap<u64, Sender<String>>>>, token: String, ctx: eframe::egui::Context) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let Some((headers, body)) = read_http(&mut stream) else { return; };
    let authorized = headers.iter().any(|(name, value)| name.eq_ignore_ascii_case("x-daozai-token") && value == &token);
    if !authorized { write_response(&mut stream, 403, "{}"); return; }
    let Ok(request) = serde_json::from_slice::<Request>(&body) else { write_response(&mut stream, 400, "{}"); return; };
    let Some(agent) = AgentId::from_key(&request.agent) else { write_response(&mut stream, 400, "{}"); return; };
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let (reply_tx, reply_rx) = if request.blocking {
        let (sender, receiver) = mpsc::channel();
        (Some(sender), Some(receiver))
    } else {
        (None, None)
    };
    if let Some(sender) = &reply_tx { pending.lock().unwrap().insert(id, sender.clone()); }
    let _ = tx.send(Incoming { id, agent, event: request.event, payload: request.payload, hosts: request.hosts, reply: reply_tx });
    ctx.request_repaint();
    let body = match reply_rx {
        Some(receiver) => receiver.recv_timeout(Duration::from_secs(WAIT_SECONDS)).unwrap_or_else(|_| "{}".into()),
        None => "{}".into(),
    };
    pending.lock().unwrap().remove(&id);
    write_response(&mut stream, 200, &body);
}

fn read_http(stream: &mut TcpStream) -> Option<(Vec<(String, String)>, Vec<u8>)> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        if let Some(position) = find(&buffer, b"\r\n\r\n") { break position; }
        if buffer.len() > MAX_BODY { return None; }
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 { return None; }
        buffer.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let mut headers = Vec::new();
    let mut lines = head.split("\r\n");
    lines.next();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }
    let length = headers.iter().find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok()).unwrap_or(0);
    if length > MAX_BODY { return None; }
    let mut body = buffer[header_end + 4..].to_vec();
    while body.len() < length {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 { break; }
        body.extend_from_slice(&chunk[..read]);
        if body.len() > MAX_BODY { return None; }
    }
    body.truncate(length);
    Some((headers, body))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn write_response(stream: &mut TcpStream, code: u16, body: &str) {
    let _ = write!(stream, "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    let _ = stream.flush();
    // Windows 回环在写完响应后立即关闭偶发 RST，对端可能收不到决定；短暂保留连接。
    std::thread::sleep(Duration::from_millis(20));
}

/// 从命令行识别中继调用；必须在单实例检查之前处理。
pub fn requested<I: Iterator<Item = String>>(mut args: I) -> Option<(AgentId, String)> {
    let mut agent = None;
    let mut event = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--agent-hook" => agent = args.next().and_then(|value| AgentId::from_key(&value)),
            "--event" => event = args.next(),
            _ => {}
        }
    }
    Some((agent?, event?))
}

#[derive(serde::Deserialize)]
struct BridgeInfo { port: u16, token: String }

/// 中继进程：读 stdin，POST 给岛仔；失败或超时都输出空决定，绝不阻塞 Agent。
pub fn run_relay(agent: AgentId, event: &str) -> i32 {
    let mut raw = String::new();
    let _ = std::io::stdin().take((MAX_BODY + 1) as u64).read_to_string(&mut raw);
    let payload: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
    let blocking = super::blocking_event(event, &payload);
    let output = relay(agent, event, payload, blocking).unwrap_or_else(|| "{}".into());
    print!("{output}");
    let _ = std::io::stdout().flush();
    0
}

fn relay(agent: AgentId, event: &str, payload: serde_json::Value, blocking: bool) -> Option<String> {
    let bytes = std::fs::read(crate::model::config_dir().join("agent-bridge.json")).ok()?;
    let info: BridgeInfo = serde_json::from_slice(&bytes).ok()?;
    let address = SocketAddr::from(([127, 0, 0, 1], info.port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(300)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(if blocking { WAIT_SECONDS + 5 } else { 2 })));
    let body = serde_json::json!({ "agent": agent.key(), "event": event, "blocking": blocking, "payload": payload, "hosts": super::focus::ancestors() }).to_string();
    let request = format!(
        "POST /hook HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Daozai-Token: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        info.token, body.len(), body
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    let text = String::from_utf8_lossy(&response);
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("").trim();
    Some(if body.is_empty() { "{}".into() } else { body.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_flags_are_parsed_before_single_instance() {
        let args = vec!["daozai.exe".to_string(), "--agent-hook".into(), "claude".into(), "--event".into(), "PermissionRequest".into()];
        assert_eq!(requested(args.into_iter()), Some((AgentId::Claude, "PermissionRequest".into())));
        assert_eq!(requested(vec!["daozai.exe".to_string()].into_iter()), None);
    }

    #[test]
    fn only_permission_and_questions_block() {
        assert!(super::super::blocking_event("PermissionRequest", &serde_json::json!({})));
        assert!(super::super::blocking_event("PreToolUse", &serde_json::json!({"tool_name":"AskUserQuestion"})));
        assert!(!super::super::blocking_event("PreToolUse", &serde_json::json!({"tool_name":"Bash"})));
    }
}
