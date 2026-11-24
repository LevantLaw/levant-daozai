//! 测试辅助：本机回环 mock 服务器的收尾与串行化。

/// 本机 Windows 回环在写完响应后立即关闭连接时会偶发 RST，对端可能读不到响应；
/// 请求体未读完就关闭也会触发 RST。读到对端关闭再返回，确认响应已被取走。
pub fn settle(stream: &mut std::net::TcpStream) {
    use std::io::{Read, Write};
    let _ = stream.flush();
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
    let mut buf = [0u8; 1024];
    while matches!(stream.read(&mut buf), Ok(n) if n > 0) {}
}

/// TLS 等无法设置读超时的流用固定等待，关闭前给对端留出读取响应的时间。
pub fn settle_sleep(stream: &mut impl std::io::Write) {
    let _ = stream.flush();
    std::thread::sleep(std::time::Duration::from_millis(50));
}

/// 窗口、托盘和真实计时类用例会互相干扰，用同一把锁串行执行。
pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 全局工作位可能被并行测试的临时任务占着；等到两个空闲位再返回。
pub fn acquire_pair(lane: &'static crate::support::resources::Lane) -> [crate::support::resources::Lease; 2] {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let first = lane.acquire();
        let second = if first.is_some() { lane.acquire() } else { None };
        if let (Some(first), Some(second)) = (first, second) { return [first, second]; }
        assert!(std::time::Instant::now() < deadline, "等待两个空闲工作位超时");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
