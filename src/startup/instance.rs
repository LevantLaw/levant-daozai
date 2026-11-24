//! 单实例等待、异常重启与自动替换。

use std::path::PathBuf;
use super::report::{report, write_report};

static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn diagnostics_dir() -> PathBuf { daozai::model::config_dir().join("diagnostics") }

/// 自动重启的频率限制：本次至少运行 30 秒（避免启动即崩的死循环），且一小时内最多 3 次。
fn restart_allowed(history: &str, now: u64, uptime: std::time::Duration) -> bool {
    if uptime < std::time::Duration::from_secs(30) { return false; }
    let recent = history.lines().filter_map(|line| line.split_whitespace().next()?.parse::<u64>().ok())
        .filter(|at| now.saturating_sub(*at) < 3600).count();
    recent < 3
}

/// 启动一个新的岛仔：它会先等当前进程退出，再走正常的单实例启动流程。
pub fn spawn_replacement() -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    let mut command = std::process::Command::new(exe);
    command.arg("--restart-after").arg(std::process::id().to_string()).env_remove("DAOZAI_TEST_CRASH");
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) { command.current_dir(dir); }
    command.spawn().is_ok()
}

/// 异常退出后自动重新拉起（受频率限制）；返回是否已安排重启。
fn relaunch_after_failure(reason: &str) -> bool {
    let uptime = STARTED.get().map(|at| at.elapsed()).unwrap_or_default();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let path = diagnostics_dir().join("auto-restart.log");
    let history = std::fs::read_to_string(&path).unwrap_or_default();
    if !restart_allowed(&history, now, uptime) { return false; }
    // 只保留最近 20 条，避免常驻程序无限累积。
    let mut lines: Vec<&str> = history.lines().collect();
    if lines.len() > 19 { lines.drain(..lines.len() - 19); }
    let reason: String = reason.chars().filter(|c| !c.is_control()).take(160).collect();
    let entry = format!("{now} 运行 {} 秒后{reason}，已自动重启", uptime.as_secs());
    lines.push(&entry);
    let _ = std::fs::create_dir_all(diagnostics_dir());
    let _ = std::fs::write(&path, lines.join("\n") + "\n");
    spawn_replacement()
}

/// 自动重启的新进程：等上一个岛仔退出（最多 15 秒）再继续，避免单实例检查把自己挡住。
pub fn wait_for_previous_instance() {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg != "--restart-after" { continue; }
        let Some(pid) = args.next().and_then(|v| v.parse::<u32>().ok()) else { return };
        #[cfg(windows)]
        unsafe {
            use windows::Win32::{Foundation::CloseHandle, System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE}};
            if let Ok(process) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
                WaitForSingleObject(process, 15_000);
                let _ = CloseHandle(process);
            }
        }
        #[cfg(not(windows))]
        let _ = pid;
        return;
    }
}

/// 进程级崩溃（例如显卡驱动里访问了无效内存）：Rust 的 panic 记录覆盖不到，这里记下崩在哪个模块；
/// 崩在显卡的 OpenGL 驱动里时，下次启动改用兼容渲染；随后按频率限制自动重启。
#[cfg(windows)]
mod native_crash {
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows::Win32::{Foundation::HMODULE, System::{Diagnostics::Debug::{SetUnhandledExceptionFilter, EXCEPTION_POINTERS},
        LibraryLoader::{GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT}}};
    const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
    static HANDLING: AtomicBool = AtomicBool::new(false);

    /// 异常地址所在的模块文件名与模块内偏移。
    unsafe fn module_at(address: usize) -> (String, usize) {
        let mut module = HMODULE::default();
        let flags = GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
        if GetModuleHandleExW(flags, windows::core::PCWSTR(address as *const u16), &mut module).is_err() { return ("未知模块".into(), address); }
        let mut name = [0u16; 520];
        let length = GetModuleFileNameW(module, &mut name) as usize;
        let path = String::from_utf16_lossy(&name[..length.min(name.len())]);
        let file = path.rsplit('\\').next().unwrap_or(&path).to_string();
        (file, address.wrapping_sub(module.0 as usize))
    }

    unsafe extern "system" fn filter(info: *const EXCEPTION_POINTERS) -> i32 {
        if HANDLING.swap(true, Ordering::SeqCst) || info.is_null() || (*info).ExceptionRecord.is_null() { return EXCEPTION_CONTINUE_SEARCH; }
        let record = &*(*info).ExceptionRecord;
        let code = record.ExceptionCode.0 as u32;
        let (module, offset) = module_at(record.ExceptionAddress as usize);
        let uptime = super::STARTED.get().map(|at| at.elapsed().as_secs()).unwrap_or(0);
        let gl_driver = crate::startup::backend::is_gl_driver(&module);
        let renderer = if crate::startup::backend::compat_active() { "兼容渲染（Direct3D 12）" } else { "OpenGL" };
        let mut details = format!("异常代码：{code:#010x}\n崩溃模块：{module}\n模块内偏移：{offset:#x}\n已运行：{uptime} 秒\n渲染方式：{renderer}\n");
        if gl_driver && !crate::startup::backend::compat_active() {
            crate::startup::backend::request_fallback(&module);
            details.push_str("判断：崩在显卡的 OpenGL 驱动里，下次启动自动改用兼容渲染（Direct3D 12）。\n");
        }
        let restarted = super::relaunch_after_failure(&format!("进程崩溃（{module}）"));
        details.push_str(if restarted { "已自动重新启动岛仔。\n" } else { "未自动重启（运行不足 30 秒或一小时内已重启 3 次）。\n" });
        let _ = super::write_report(&super::diagnostics_dir(), "岛仔意外退出（进程崩溃）", &details);
        // 交给系统继续处理，Windows 事件日志里仍会留下崩溃记录。
        EXCEPTION_CONTINUE_SEARCH
    }

    pub fn install() { unsafe { SetUnhandledExceptionFilter(Some(filter)); } }

    /// 验收用：`DAOZAI_TEST_CRASH=native` 时，运行指定秒数后在岛仔自身触发一次访问违规。
    pub fn test_crash_hook() {
        let Ok(value) = std::env::var("DAOZAI_TEST_CRASH") else { return };
        let Some(seconds) = value.strip_prefix("native:").and_then(|s| s.parse::<u64>().ok()) else { return };
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(seconds));
            unsafe { std::ptr::write_volatile(std::ptr::null_mut::<u32>(), 1); }
        });
    }
}

pub fn install() {
    let _ = STARTED.set(std::time::Instant::now());
    #[cfg(windows)]
    { native_crash::install(); native_crash::test_crash_hook(); }
    std::panic::set_hook(Box::new(|info| {
        // 不写入 panic 的数据载荷，避免把邮件、凭据或登录信息带进日志。
        let location = info.location().map(|p| format!("{}:{}:{}", p.file(), p.line(), p.column()))
            .unwrap_or_else(|| "未知".into());
        let details = format!("代码位置：{location}\n调用栈：\n{}", std::backtrace::Backtrace::force_capture());
        // 主线程出错会让整个岛仔退出：记下原因后直接自动重启，不弹窗打断。后台线程出错不影响主界面，仍按原样提示。
        if std::thread::current().name() == Some("main") && relaunch_after_failure("程序异常") {
            let _ = write_report(&diagnostics_dir(), "岛仔发生异常，已自动重新启动。", &details);
            return;
        }
        report("岛仔发生异常，请重新启动。", &details);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_restart_is_rate_limited() {
        use std::time::Duration;
        let now = 10_000;
        assert!(restart_allowed("", now, Duration::from_secs(60)));
        assert!(!restart_allowed("", now, Duration::from_secs(10)), "启动不到 30 秒就崩溃不自动重启，避免死循环");
        let three = "9000 a\n9500 b\n9900 c\n";
        assert!(!restart_allowed(three, now, Duration::from_secs(600)), "一小时内最多 3 次");
        let old = "1000 a\n2000 b\n9900 c\n";
        assert!(restart_allowed(old, now, Duration::from_secs(600)), "一小时前的重启不计入");
        assert!(restart_allowed("乱码\n\n", now, Duration::from_secs(600)), "记录损坏时仍可重启");
    }
}
