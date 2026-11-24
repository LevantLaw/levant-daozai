//! 会话跳转：记录 Hook 进程的祖先链，点击时激活对应的顶层窗口。

/// Hook 进程的祖先链，顺序为「自己 → 父进程 → … → 顶层宿主」。
pub fn ancestors() -> Vec<(u32, String)> {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return Vec::new(); };
        let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut map = std::collections::HashMap::new();
        let mut ok = Process32FirstW(snapshot, &mut entry).is_ok();
        while ok {
            let name = String::from_utf16_lossy(&entry.szExeFile);
            map.insert(entry.th32ProcessID, (entry.th32ParentProcessID, name.trim_end_matches('\0').to_string()));
            ok = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
        let mut chain = Vec::new();
        let mut pid = std::process::id();
        for _ in 0..10 {
            let Some((parent, name)) = map.get(&pid).cloned() else { break; };
            chain.push((pid, name));
            if parent == 0 || parent == pid { break; }
            pid = parent;
        }
        chain
    }
    #[cfg(not(windows))]
    { Vec::new() }
}

/// 激活祖先链中最外层宿主的可见窗口；找不到返回 false。
pub fn focus(hosts: &[(u32, String)]) -> bool {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
        use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows::Win32::UI::WindowsAndMessaging::*;

        struct Windows(Vec<(HWND, u32, bool)>);
        unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> BOOL {
            let list = &mut *(data.0 as *mut Windows);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid != 0 && IsWindowVisible(hwnd).as_bool() {
                list.0.push((hwnd, pid, GetWindowTextLengthW(hwnd) > 0));
            }
            true.into()
        }
        let mut windows = Windows(Vec::new());
        let _ = EnumWindows(Some(visit), LPARAM((&mut windows as *mut Windows) as isize));
        for (pid, name) in hosts.iter().rev() {
            // 桌面进程的窗口是任务栏，不是会话宿主。
            if name.eq_ignore_ascii_case("explorer.exe") { continue; }
            let candidate = windows.0.iter().find(|(_, wpid, titled)| wpid == pid && *titled)
                .or_else(|| windows.0.iter().find(|(_, wpid, _)| wpid == pid));
            let Some(&(hwnd, _, _)) = candidate else { continue; };
            if IsIconic(hwnd).as_bool() { let _ = ShowWindow(hwnd, SW_RESTORE); }
            // 前台锁：临时附加到当前前台线程，切换成功率更高。
            let foreground_thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
            let current = GetCurrentThreadId();
            let attached = foreground_thread != 0 && foreground_thread != current && AttachThreadInput(current, foreground_thread, true).as_bool();
            let _ = SetForegroundWindow(hwnd);
            if attached { let _ = AttachThreadInput(current, foreground_thread, false); }
            return true;
        }
        false
    }
    #[cfg(not(windows))]
    { let _ = hosts; false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ancestor_chain_starts_with_the_current_process() {
        let chain = ancestors();
        if cfg!(windows) {
            assert_eq!(chain[0].0, std::process::id());
            assert!(!chain[0].1.is_empty());
        }
    }
}
