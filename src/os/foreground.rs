//! 前台窗口、全屏判定与名单应用避让。

use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Threading::{GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::WindowsAndMessaging::*;

/// 前台窗口在全屏判定里需要的信息。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Foreground {
    hwnd: isize,
    pid: u32,
    /// 最终所有者窗口（没有所有者时是它自己）。
    root_owner: isize,
    /// 会在任务栏单独显示按钮的普通程序窗口（不是工具窗口、对话框或浮动面板）。
    app_window: bool,
}

/// 前台从全屏窗口切到别的窗口时，是否仍按原来的全屏处理（岛仔保持隐藏）。
///
/// 3ds Max、Photoshop 这类软件全屏时，插件面板、浮动工具条、
/// 属性对话框都是同一个程序的附属窗口；每点一下前台就在它们和主窗口之间来回切，
/// 过去岛仔会跟着「出现 → 消失」闪一次。只要原来的全屏窗口还盖着屏幕，
/// 而新的前台是它的附属窗口（被它拥有，或同一程序里不单独占任务栏的窗口），就继续隐藏。
/// 切到另一个程序、或同一程序里另开的普通窗口（如浏览器的新窗口）时照常显示。
fn keeps_fullscreen(last: Foreground, still_covers: bool, now: Option<Foreground>) -> bool {
    if !still_covers { return false; }
    match now {
        // 激活切换的瞬间前台可能是空的，不当成离开全屏。
        None => true,
        Some(now) => now.hwnd == last.hwnd || now.root_owner == last.hwnd || (now.pid == last.pid && !now.app_window),
    }
}

unsafe fn foreground_info(hwnd: HWND) -> Foreground {
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let root = GetAncestor(hwnd, GA_ROOTOWNER);
    let root_owner = if root.0.is_null() { hwnd.0 as isize } else { root.0 as isize };
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    let owned = !GetWindow(hwnd, GW_OWNER).unwrap_or_default().0.is_null();
    let app_window = ex & WS_EX_APPWINDOW.0 as isize != 0 || (!owned && ex & WS_EX_TOOLWINDOW.0 as isize == 0);
    Foreground { hwnd: hwnd.0 as isize, pid, root_owner, app_window }
}

/// 全屏判定允许的边框 / 舍入误差，物理像素。
const FULLSCREEN_TOLERANCE:i32=8;

/// 系统是否报告前台处于全屏或演示状态。
fn system_fullscreen()->bool {
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState,QUNS_BUSY,QUNS_PRESENTATION_MODE,QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    unsafe {SHQueryUserNotificationState()}.is_ok_and(|state|state==QUNS_BUSY||state==QUNS_RUNNING_D3D_FULL_SCREEN||state==QUNS_PRESENTATION_MODE)
}

/// 窗口盖满它所在的整块屏幕时返回该屏幕范围。
unsafe fn covering_monitor(hwnd: HWND) -> Option<crate::os::display_layout::Bounds> {
    covering_monitor_with(hwnd,system_fullscreen)
}

/// `system_fullscreen` 由调用方注入，测试可以固定系统全屏状态。
unsafe fn covering_monitor_with(hwnd:HWND,system_fullscreen:impl Fn()->bool) -> Option<crate::os::display_layout::Bounds> {
    if !IsWindow(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
        return None;
    }
    let mut r = RECT::default();
    if GetWindowRect(hwnd, &mut r).is_err() {
        return None;
    }
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info).as_bool()
    {
        return None;
    }
    let monitor=info.rcMonitor;
    let covers=r.left <= monitor.left
        && r.top <= monitor.top
        && r.right >= monitor.right
        && r.bottom >= monitor.bottom;
    // 边框和 DPI 舍入可能让无边框全屏差 1–2 像素；系统报告全屏时允许少量容差。
    let near=r.left <= monitor.left+FULLSCREEN_TOLERANCE
        && r.top <= monitor.top+FULLSCREEN_TOLERANCE
        && r.right >= monitor.right-FULLSCREEN_TOLERANCE
        && r.bottom >= monitor.bottom-FULLSCREEN_TOLERANCE;
    if !covers && !near {
        return None;
    }
    let zoomed=IsZoomed(hwnd).as_bool();
    // 任务栏自动隐藏时，普通最大化窗口的外框也可能覆盖整屏，但标题栏占掉了客户区顶部。
    // 客户区没有盖满、或系统没有报告全屏状态时，按普通最大化处理。
    // 有的游戏全屏时保留标题栏样式和最大化标志、实际并不画标题栏，此时客户区盖满整屏，
    // 且系统报告全屏状态，仍应隐藏岛仔（0.6.65 的无畏契约回归）。
    if covers && zoomed && (!client_covers(hwnd,&monitor) || !system_fullscreen()) {
        #[cfg(not(test))] log_maximized_exclusion(hwnd, r, monitor);
        return None;
    }
    if !covers && !system_fullscreen() {
        return None;
    }
    Some(crate::os::display_layout::Bounds {left:monitor.left,top:monitor.top,right:monitor.right,bottom:monitor.bottom})
}

/// 客户区（不含标题栏和边框）是否盖满整块屏幕。
unsafe fn client_covers(hwnd: HWND, monitor: &RECT) -> bool {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    let mut c = RECT::default();
    if GetClientRect(hwnd, &mut c).is_err() { return false; }
    let (mut tl, mut br) = (POINT { x: c.left, y: c.top }, POINT { x: c.right, y: c.bottom });
    if !ClientToScreen(hwnd, &mut tl).as_bool() || !ClientToScreen(hwnd, &mut br).as_bool() { return false; }
    tl.x <= monitor.left && tl.y <= monitor.top && br.x >= monitor.right && br.y >= monitor.bottom
}

/// 诊断：盖满屏幕却按普通最大化处理（不隐藏岛仔）的窗口，每个窗口只记一次，便于核对用户反馈。
#[cfg(not(test))]
unsafe fn log_maximized_exclusion(hwnd: HWND, window: RECT, monitor: RECT) {
    use std::sync::Mutex;
    static LOGGED: Mutex<isize> = Mutex::new(0);
    let mut logged = LOGGED.lock().unwrap_or_else(|e| e.into_inner());
    if *logged == hwnd.0 as isize { return; }
    *logged = hwnd.0 as isize;
    let mut cls = [0u16; 128];
    let n = GetClassNameW(hwnd, &mut cls);
    let mut c = RECT::default();
    let _ = GetClientRect(hwnd, &mut c);
    let line = format!("{} 普通最大化不隐藏：类名={} 样式=0x{:08x} 扩展=0x{:08x} 窗口={:?} 客户区={}×{} 屏幕={:?}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), String::from_utf16_lossy(&cls[..n.max(0) as usize]),
        GetWindowLongPtrW(hwnd, GWL_STYLE) as u32, GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32,
        (window.left, window.top, window.right, window.bottom), c.right - c.left, c.bottom - c.top,
        (monitor.left, monitor.top, monitor.right, monitor.bottom));
    crate::support::resources::append_log(&crate::model::config_dir().join("diagnostics"), "fullscreen.log", &line);
}

pub fn fullscreen() -> Option<crate::os::display_layout::Bounds> {
    use std::sync::Mutex;
    // 上一次判定为全屏的窗口：前台切到它的附属窗口时沿用（见 `keeps_fullscreen`）。
    static LAST: Mutex<Option<Foreground>> = Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let hwnd = GetForegroundWindow();
        let now = (!hwnd.0.is_null()).then(|| foreground_info(hwnd));
        if let Some(info) = now {
            // 岛仔自己的窗口（设置窗口等）不算全屏，也不沿用。
            if info.pid == windows::Win32::System::Threading::GetCurrentProcessId() {
                *last = None;
                return None;
            }
            // 桌面与资源管理器桌面窗口不能当成全屏游戏。
            let mut cls = [0u16; 64];
            let n = GetClassNameW(hwnd, &mut cls);
            let cls = String::from_utf16_lossy(&cls[..n.max(0) as usize]);
            if matches!(cls.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
                *last = None;
                return None;
            }
            if let Some(bounds) = covering_monitor(hwnd) {
                *last = Some(info);
                return Some(bounds);
            }
        }
        if let Some(previous) = *last {
            let bounds = covering_monitor(HWND(previous.hwnd as *mut _));
            if keeps_fullscreen(previous, bounds.is_some(), now) {
                return bounds;
            }
        }
        *last = None;
        None
    }
}

#[cfg(test)]
mod fullscreen_follow_tests {
    use super::*;

    const MAX: Foreground = Foreground { hwnd: 100, pid: 7, root_owner: 100, app_window: true };

    #[test]
    fn maximized_browser_and_borderless_fullscreen_at_the_same_bounds_are_distinct() {
        unsafe {
            let hwnd = CreateWindowExW(WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW, windows::core::w!("STATIC"),
                windows::core::w!(""), WS_OVERLAPPEDWINDOW | WS_MAXIMIZE, 0, 0, 320, 240, None, None, None, None).unwrap();
            let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            assert!(GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info).as_bool());
            let r = info.rcMonitor;
            let cover = || SetWindowPos(hwnd, HWND_BOTTOM, r.left, r.top, r.right-r.left, r.bottom-r.top,
                SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED).unwrap();
            cover();
            assert!(IsZoomed(hwnd).as_bool());
            let maximized = covering_monitor_with(hwnd, || false);
            // Chromium 进入 F11 时移除标题栏和可调整大小的边框，仍可能保留最大化标志。
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            SetWindowLongPtrW(hwnd, GWL_STYLE, style & !((WS_CAPTION | WS_THICKFRAME).0 as isize));
            cover();
            let fullscreen = covering_monitor_with(hwnd, || true);
            DestroyWindow(hwnd).unwrap();
            assert!(fullscreen.is_some(), "无边框全屏仍应隐藏岛仔");
            assert!(maximized.is_none(), "普通最大化即使盖满屏幕也应显示岛仔");
        }
    }

    #[test]
    fn game_that_keeps_caption_style_but_draws_no_caption_still_hides_the_island() {
        use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
        unsafe extern "system" fn proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
            // 不留非客户区：样式里有标题栏，但客户区就是整个窗口（自绘或无边框全屏的游戏）。
            if m == WM_NCCALCSIZE && w.0 != 0 { return LRESULT(0); }
            DefWindowProcW(h, m, w, l)
        }
        unsafe {
            let class = windows::core::w!("DaozaiCaptionlessGame");
            let wc = WNDCLASSW { lpfnWndProc: Some(proc), lpszClassName: class, ..Default::default() };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW, class, windows::core::w!(""),
                WS_OVERLAPPEDWINDOW | WS_MAXIMIZE, 0, 0, 320, 240, None, None, None, None).unwrap();
            let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            assert!(GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info).as_bool());
            let r = info.rcMonitor;
            SetWindowPos(hwnd, HWND_BOTTOM, r.left, r.top, r.right-r.left, r.bottom-r.top,
                SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED).unwrap();
            let zoomed = IsZoomed(hwnd).as_bool();
            let captioned = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_CAPTION.0 == WS_CAPTION.0;
            let result = covering_monitor_with(hwnd, || true);
            let maximized = covering_monitor_with(hwnd, || false);
            DestroyWindow(hwnd).unwrap();
            assert!(zoomed && captioned, "复现条件：最大化且保留标题栏样式");
            assert!(result.is_some(), "客户区盖满整屏的全屏游戏应隐藏岛仔");
            assert!(maximized.is_none(), "系统没有报告全屏时，无边框最大化不能隐藏岛仔");
        }
    }

    #[test]
    fn plugin_panels_and_dialogs_of_the_fullscreen_app_keep_the_island_hidden() {
        // 被主窗口拥有的插件面板 / 对话框
        let owned = Foreground { hwnd: 200, pid: 7, root_owner: 100, app_window: false };
        assert!(keeps_fullscreen(MAX, true, Some(owned)));
        // 同一程序里不占任务栏的浮动工具窗口（没有设所有者）
        let tool = Foreground { hwnd: 201, pid: 7, root_owner: 201, app_window: false };
        assert!(keeps_fullscreen(MAX, true, Some(tool)));
        // 插件放在独立进程里、但窗口挂在主窗口下面
        let other_process = Foreground { hwnd: 202, pid: 9, root_owner: 100, app_window: false };
        assert!(keeps_fullscreen(MAX, true, Some(other_process)));
        // 激活切换瞬间前台为空
        assert!(keeps_fullscreen(MAX, true, None));
    }

    #[test]
    fn switching_to_another_app_or_leaving_fullscreen_shows_the_island() {
        let other_app = Foreground { hwnd: 300, pid: 9, root_owner: 300, app_window: true };
        assert!(!keeps_fullscreen(MAX, true, Some(other_app)), "切到别的程序时照常显示");
        // 同一程序新开的普通窗口（如浏览器的第二个窗口）也照常显示
        let second_window = Foreground { hwnd: 301, pid: 7, root_owner: 301, app_window: true };
        assert!(!keeps_fullscreen(MAX, true, Some(second_window)));
        // 原来的全屏窗口已经还原、最小化或关闭
        let owned = Foreground { hwnd: 200, pid: 7, root_owner: 100, app_window: false };
        assert!(!keeps_fullscreen(MAX, false, Some(owned)));
        assert!(!keeps_fullscreen(MAX, false, None));
    }
}

pub(crate) fn is_browser_process(name:&str)->bool {
    ["chrome.exe","msedge.exe","firefox.exe","brave.exe","vivaldi.exe","opera.exe",
        "arc.exe","zen.exe","chromium.exe","librewolf.exe","floorp.exe","waterfox.exe",
        "duckduckgo.exe","360se.exe","360chrome.exe","qqbrowser.exe","sogouexplorer.exe","quark.exe"]
        .iter().any(|browser|name.eq_ignore_ascii_case(browser))
}

fn browser_overlaps_island(browser:RECT,island:RECT,dock:crate::model::Dock)->bool {
    if dock.is_side() {return window_covers_side(browser,island,dock);}
    // 窗口是 CANVAS_W 宽的透明画布，岛体中心固定在画布左侧 WIN_W/2 处（与水平位置、大小无关）。
    let scale=(island.right-island.left) as f32/crate::model::CANVAS_W;
    let center=island.left as f32+crate::model::WIN_W*scale/2.0;
    let half=crate::model::PW*scale/2.0;
    (browser.left as f32)<center+half && (browser.right as f32)>center-half
        && (browser.top as f32)<island.top as f32+crate::model::BAR_H*scale && browser.bottom>island.top
}

/// 左右停靠：画布是 SIDE_W × SIDE_H，岛体贴在画布的左 / 右边、竖直居中。
/// 前台窗口盖住岛所在的那一段屏幕边缘（例如最大化、全屏或贴边摆放）即视为遮挡。
fn window_covers_side(window:RECT,island:RECT,dock:crate::model::Dock)->bool {
    let scale=(island.right-island.left) as f32/crate::model::SIDE_W;
    let cy=island.top as f32+crate::model::SIDE_H*scale/2.0;
    let half=crate::model::PH*scale/2.0;
    let strip=crate::model::SIDE_BAR_W*scale;
    let vertical=(window.top as f32)<cy+half && (window.bottom as f32)>cy-half;
    let horizontal=if dock==crate::model::Dock::Left {(window.left as f32)<island.left as f32+strip && window.right>island.left}
        else {(window.right as f32)>island.right as f32-strip && window.left<island.right};
    vertical && horizontal
}

/// 窗口是否盖住整块工作区（最大化或全屏）。
fn covers_work_area(window:RECT,work:RECT)->bool {
    window.left<=work.left && window.top<=work.top && window.right>=work.right && window.bottom>=work.bottom
}

pub fn browser_at_top(island:HWND,settings:&crate::model::Settings)->Option<bool> {
    unsafe {
        let foreground=GetForegroundWindow();
        if foreground.0.is_null(){return Some(false);}
        let mut pid=0;
        GetWindowThreadProcessId(foreground,Some(&mut pid));
        // 设置、托盘和岛内输入沿用此前的避让状态，避免自身窗口引起反复切换。
        if pid==GetCurrentProcessId(){return None;}
        if pid==0{return Some(false);}
        let mut info=MONITORINFO{cbSize:std::mem::size_of::<MONITORINFO>() as u32,..Default::default()};
        if !GetMonitorInfoW(MonitorFromWindow(island,MONITOR_DEFAULTTONEAREST),&mut info).as_bool(){return Some(false);}
        thread_local! { static LAST_MAIN: std::cell::Cell<Option<(HWND,u32)>> = const { std::cell::Cell::new(None) }; }
        LAST_MAIN.with(|last| avoidance_for_window(foreground,island,pid,settings,info.rcWork,last))
    }
}

unsafe fn visible_avoidance_rect(hwnd:HWND)->Option<RECT> {
    if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool(){return None;}
    let mut cloaked=0u32;
    let _=windows::Win32::Graphics::Dwm::DwmGetWindowAttribute(hwnd,windows::Win32::Graphics::Dwm::DWMWA_CLOAKED,(&mut cloaked as *mut u32).cast(),4);
    if cloaked!=0{return None;}
    let mut rect=RECT::default();
    GetWindowRect(hwnd,&mut rect).ok()?;
    Some(rect)
}

unsafe fn process_image_path(pid:u32)->Option<String> {
    let process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,false,pid).ok()?;
    let mut path=vec![0u16;32768];let mut length=path.len() as u32;
    let result=QueryFullProcessImageNameW(process,PROCESS_NAME_WIN32,PWSTR(path.as_mut_ptr()),&mut length);
    let _=CloseHandle(process);
    result.ok()?;
    Some(String::from_utf16_lossy(&path[..length as usize]))
}

// 无所有者的工具窗可能在岛仔启动时就已打开。只有同进程的主窗口唯一时才补认宿主。
unsafe fn unique_main_window(pid:u32)->Option<HWND> {
    use windows::Win32::Foundation::{BOOL,LPARAM};
    struct Search { pid:u32, main:Option<HWND> }
    unsafe extern "system" fn visit(hwnd:HWND,param:LPARAM)->BOOL {
        let search=&mut *(param.0 as *mut Search);
        let mut pid=0;GetWindowThreadProcessId(hwnd,Some(&mut pid));
        if pid==search.pid && foreground_info(hwnd).app_window && visible_avoidance_rect(hwnd).is_some() {
            if search.main.is_some(){search.main=None;return BOOL(0);}
            search.main=Some(hwnd);
        }
        BOOL(1)
    }
    let mut search=Search{pid,main:None};
    EnumWindows(Some(visit),LPARAM((&mut search as *mut Search) as isize)).ok()?;
    search.main
}

unsafe fn avoidance_for_window(foreground:HWND,island:HWND,pid:u32,settings:&crate::model::Settings,work:RECT,last:&std::cell::Cell<Option<(HWND,u32)>>)->Option<bool> {
    if !settings.browser_minimal || IsIconic(GetAncestor(foreground,GA_ROOTOWNER)).as_bool(){return Some(false);}
    let Some(rect)=visible_avoidance_rect(foreground) else{return Some(false)};
    let mut island_rect=RECT::default();GetWindowRect(island,&mut island_rect).ok()?;
    let overlaps=|rect|browser_overlaps_island(rect,island_rect,settings.dock);
    // 名单应用只有最大化（盖住整块工作区）才避让；普通窗口只是压到岛仔时不收缩。
    let covers=|rect|overlaps(rect) && covers_work_area(rect,work);
    let owner=GetWindow(foreground,GW_OWNER).ok();
    let info=foreground_info(foreground);
    if owner.is_none() && info.app_window {last.set(Some((foreground,pid)));}
    else if last.get().is_some_and(|(_,prev)|prev!=pid){last.set(None);}
    let path=std::cell::LazyCell::new(||process_image_path(pid));
    if covers(rect) && path.as_deref().is_some_and(|p|avoidance_matches(p,settings)){return Some(true);}
    if settings.avoid_apps.is_empty(){return Some(false);}

    // 所有者关系可以跨进程，嵌套对话框也沿这条链找到实际遮挡岛的名单窗口。
    let listed=|p:&str|crate::model::contains_app(&settings.avoid_apps,p);
    let mut parent=owner;
    while let Some(hwnd)=parent {
        if IsIconic(hwnd).as_bool(){return Some(false);}
        if visible_avoidance_rect(hwnd).is_some_and(covers) {
            let mut owner_pid=0;GetWindowThreadProcessId(hwnd,Some(&mut owner_pid));
            let registered=if owner_pid==pid {path.as_deref().is_some_and(listed)}
                else {process_image_path(owner_pid).as_deref().is_some_and(listed)};
            if registered{
                if foreground_info(hwnd).app_window {last.set(Some((hwnd,owner_pid)));}
                return Some(true);
            }
        }
        parent=GetWindow(hwnd,GW_OWNER).ok();
    }
    if owner.is_some() || info.app_window || !path.as_deref().is_some_and(listed){return Some(false);}
    // 部分插件工具窗未登记 owner。优先跟随刚才操作的同进程主窗口，每次重新检查其位置。
    let host=match last.get() {
        Some((hwnd,prev)) if prev==pid && foreground_info(hwnd).pid==pid=>Some(hwnd),
        _=>unique_main_window(pid),
    };
    Some(host.and_then(|hwnd|visible_avoidance_rect(hwnd)).is_some_and(covers))
}



#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_minimal_recognizes_browsers_without_matching_electron_apps(){
        for name in ["chrome.exe","MSEDGE.EXE","firefox.exe","brave.exe","zen.exe","QQBrowser.exe","quark.exe","Quark.exe","QUARK.EXE"] {
            assert!(is_browser_process(name),"应识别浏览器 {name}");
        }
        for name in ["Codex.exe","Code.exe","msedgewebview2.exe","explorer.exe","chrome-helper.exe","quark_proxy.exe","quark_pwa_launcher.exe","quark_host_client.exe","quark_swap_util.exe","QuarkUpdaterSetup.exe",""] {
            assert!(!is_browser_process(name),"不能将 {name} 当作浏览器");
        }
    }
    #[test]
    fn browser_minimal_checks_the_actual_top_overlap_on_the_island_monitor(){
        let island=RECT{left:560,top:0,right:1360,bottom:238};
        for browser in [RECT{left:0,top:-8,right:1920,bottom:1200},RECT{left:0,top:0,right:960,bottom:1150},RECT{left:700,top:10,right:1500,bottom:800}] {
            assert!(browser_overlaps_island(browser,island,crate::model::Dock::Top));
        }
        for browser in [RECT{left:100,top:150,right:1700,bottom:1100},RECT{left:1920,top:0,right:3840,bottom:1200},RECT{left:0,top:0,right:600,bottom:1000},RECT{left:0,top:-1200,right:1920,bottom:0}] {
            assert!(!browser_overlaps_island(browser,island,crate::model::Dock::Top));
        }
    }
    #[test]
    fn side_dock_avoidance_checks_the_covered_screen_edge(){
        use crate::model::Dock;
        // 1920×1080 屏幕，100% 缩放：左侧画布 0..500，竖直居中（240..840），岛体中心 y=540。
        let left=RECT{left:0,top:240,right:500,bottom:840};
        let right=RECT{left:1420,top:240,right:1920,bottom:840};
        let maximized=RECT{left:-8,top:-8,right:1928,bottom:1048};
        assert!(browser_overlaps_island(maximized,left,Dock::Left) && browser_overlaps_island(maximized,right,Dock::Right));
        let left_half=RECT{left:0,top:0,right:960,bottom:1040};
        assert!(browser_overlaps_island(left_half,left,Dock::Left) && !browser_overlaps_island(left_half,right,Dock::Right),"贴左半屏只遮住左侧");
        let floating=RECT{left:300,top:100,right:1600,bottom:900};
        assert!(!browser_overlaps_island(floating,left,Dock::Left) && !browser_overlaps_island(floating,right,Dock::Right),"没碰到边缘的窗口不避让");
        let top_strip=RECT{left:0,top:0,right:1920,bottom:300};
        assert!(!browser_overlaps_island(top_strip,left,Dock::Left),"没盖到岛所在的那一段边缘");
    }
}


fn avoidance_matches(path:&str,settings:&crate::model::Settings)->bool {
    settings.browser_minimal && (crate::model::contains_app(&settings.avoid_apps,path)
        || settings.avoid_browsers && std::path::Path::new(path).file_name().and_then(|name|name.to_str()).is_some_and(is_browser_process))
}

pub(crate) fn avoidance_windows()->std::result::Result<Vec<crate::model::Candidate>,String> {
    use windows::Win32::Foundation::{BOOL,LPARAM};
    unsafe extern "system" fn visit(hwnd:HWND,param:LPARAM)->BOOL {
        if !IsWindowVisible(hwnd).as_bool() || GetWindowLongPtrW(hwnd,GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0!=0 {return BOOL(1);}
        let mut cloaked=0u32;
        let _=windows::Win32::Graphics::Dwm::DwmGetWindowAttribute(hwnd,windows::Win32::Graphics::Dwm::DWMWA_CLOAKED,(&mut cloaked as *mut u32).cast(),4);
        if cloaked!=0{return BOOL(1);}
        let mut title=[0u16;1024];let len=GetWindowTextW(hwnd,&mut title);
        if len==0{return BOOL(1);}
        let mut pid=0;GetWindowThreadProcessId(hwnd,Some(&mut pid));
        if pid==GetCurrentProcessId(){return BOOL(1);}
        let Some(path)=process_image_path(pid) else{return BOOL(1)};
        let items=&mut *(param.0 as *mut Vec<crate::model::Candidate>);
        if items.iter().any(|a|crate::model::same_app(&a.path,&path)){return BOOL(1);}
        let filename=std::path::Path::new(&path).file_name().and_then(|s|s.to_str()).unwrap_or("");
        let name=executable_description(&path).unwrap_or_else(||filename.trim_end_matches(".exe").to_string());
        items.push(crate::model::Candidate{browser:is_browser_process(filename),path,name,title:String::from_utf16_lossy(&title[..len as usize])});
        BOOL(1)
    }
    let mut items:Vec<crate::model::Candidate>=Vec::new();
    unsafe{EnumWindows(Some(visit),LPARAM(&mut items as *mut _ as isize))}.map_err(|_|"无法读取应用列表，请刷新重试".to_string())?;
    items.sort_by_key(|a|a.name.to_lowercase());
    Ok(items)
}

fn executable_description(path:&str)->Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW,GetFileVersionInfoW,VerQueryValueW};
    let path:Vec<u16>=path.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let size=GetFileVersionInfoSizeW(PCWSTR(path.as_ptr()),None);if size==0{return None;}
        let mut bytes=vec![0u8;size as usize];GetFileVersionInfoW(PCWSTR(path.as_ptr()),0,size,bytes.as_mut_ptr().cast()).ok()?;
        let mut ptr=std::ptr::null_mut();let mut len=0;
        if !VerQueryValueW(bytes.as_ptr().cast(),windows::core::w!("\\VarFileInfo\\Translation"),&mut ptr,&mut len).as_bool() || len<4{return None;}
        let translation=std::slice::from_raw_parts(ptr as *const u16,2);
        let key:Vec<u16>=format!("\\StringFileInfo\\{:04x}{:04x}\\FileDescription",translation[0],translation[1]).encode_utf16().chain(Some(0)).collect();
        if !VerQueryValueW(bytes.as_ptr().cast(),PCWSTR(key.as_ptr()),&mut ptr,&mut len).as_bool() || len<=1{return None;}
        let name=String::from_utf16_lossy(std::slice::from_raw_parts(ptr as *const u16,len as usize)).trim_end_matches('\0').trim().to_string();
        (!name.is_empty()).then_some(name)
    }
}

#[cfg(test)]
mod avoidance_tests {
    use super::*;
    #[test]
    fn work_area_coverage_requires_the_whole_work_area() {
        let work=RECT{left:0,top:0,right:1920,bottom:1040};
        assert!(covers_work_area(RECT{left:-8,top:-8,right:1928,bottom:1048},work));
        assert!(!covers_work_area(RECT{left:0,top:0,right:1920,bottom:1000},work),"贴顶的普通窗口不算最大化");
        assert!(!covers_work_area(RECT{left:0,top:0,right:960,bottom:1040},work),"半屏不算最大化");
    }
    #[test]
    fn registered_app_keeps_avoidance_when_its_plugin_panel_takes_focus() {
        unsafe {
            let create=|x,y,width,height,owner,ex| CreateWindowExW(WS_EX_NOACTIVATE|ex,windows::core::w!("STATIC"),
                windows::core::w!(""),WS_POPUP|WS_VISIBLE,x-10000,y-10000,width,height,owner,None,None,None).unwrap();
            let island=create(0,0,960,100,None,WS_EX_TOOLWINDOW);
            let host=create(0,0,1920,1080,None,WS_EX_APPWINDOW);
            let panel=create(600,300,300,200,Some(&host),WS_EX_TOOLWINDOW);
            let path=std::env::current_exe().unwrap();
            let mut settings=crate::model::Settings::default();
            settings.browser_minimal=true;
            settings.avoid_apps.push((path.to_string_lossy().into_owned(),"测试宿主".into()));
            let last=std::cell::Cell::new(None);
            let work=RECT{left:-10000,top:-10000,right:-8080,bottom:-8920};
            let poll=|window,settings:&crate::model::Settings| avoidance_for_window(window,island,GetCurrentProcessId(),settings,work,&last);
            let results=[poll(host,&settings),poll(panel,&settings),poll(host,&settings)];
            let detached=create(600,300,300,200,None,WS_EX_TOOLWINDOW);
            let nested=create(700,400,100,100,Some(&panel),WS_EX_TOOLWINDOW);
            assert_eq!(poll(detached,&settings),Some(true),"无 owner 的同进程工具窗沿用刚获得焦点的主窗口");
            assert_eq!(poll(nested,&settings),Some(true),"嵌套插件对话框应找到宿主");
            assert_eq!(avoidance_for_window(panel,island,0,&settings,work,&last),Some(true),"面板进程不可识别时仍可通过所有者识别宿主");
            let independent=create(600,300,300,200,None,WS_EX_APPWINDOW);
            last.set(None);
            assert_eq!(poll(detached,&settings),Some(false),"两个可见主窗且无法确定宿主时，不借用后台窗口");
            let small=poll(independent,&settings);
            assert_eq!(poll(detached,&settings),Some(false),"当前主窗不遮岛时不能借用同进程另一个大窗");
            SetWindowPos(independent,HWND_BOTTOM,-9700,-9995,96,80,SWP_NOACTIVATE).unwrap();
            assert_eq!(poll(independent,&settings),Some(false),"小窗只是遮住岛，不是最大化，不避让");
            assert_eq!(poll(host,&settings),Some(true));

            SetWindowPos(host,HWND_BOTTOM,-8000,-10000,1920,1080,SWP_NOACTIVATE).unwrap();
            let other_screen=[poll(host,&settings),poll(panel,&settings)];
            SetWindowPos(panel,HWND_TOP,-9700,-9995,96,80,SWP_NOACTIVATE).unwrap();
            assert_eq!(poll(panel,&settings),Some(false),"宿主不在工作区时，小浮窗自身不触发最大化避让");
            SetWindowPos(panel,HWND_TOP,-9400,-9700,300,200,SWP_NOACTIVATE).unwrap();
            SetWindowPos(host,HWND_BOTTOM,-10000,-10000,1920,1080,SWP_NOACTIVATE).unwrap();
            let _=ShowWindow(host,SW_SHOWMINNOACTIVE);
            assert!(IsIconic(host).as_bool());
            let minimized=poll(host,&settings);
            let minimized_owner=poll(panel,&settings);
            let _=ShowWindow(host,SW_SHOWNOACTIVATE);
            let _=ShowWindow(panel,SW_HIDE);
            let hidden=poll(panel,&settings);
            let _=ShowWindow(panel,SW_SHOWNOACTIVATE);
            settings.browser_minimal=false;
            let disabled=poll(panel,&settings);
            settings.browser_minimal=true;
            settings.avoid_apps.clear();
            let removed=poll(panel,&settings);
            DestroyWindow(detached).unwrap();
            DestroyWindow(independent).unwrap();
            DestroyWindow(host).unwrap();
            DestroyWindow(island).unwrap();
            assert_eq!(results,[Some(true);3],"焦点从宿主切到中央插件浮窗再切回时应持续避让");
            assert_eq!(small,Some(false),"同进程的独立小窗只检查自己的位置");
            assert_eq!(other_screen,[Some(false);2],"主窗口移到副屏后，浮窗也应停止避让");
            assert_eq!([minimized,minimized_owner,hidden],[Some(false);3],"收起或隐藏的应用窗口应结束避让");
            assert_eq!((disabled,removed),(Some(false),Some(false)));
        }
    }
    #[test]
    fn avoidance_respects_master_browser_switch_and_custom_app_identity() {
        let mut settings=crate::model::Settings::default();
        settings.avoid_apps=vec![(r"D:\Apps\Editor.exe".into(),"编辑器".into())];
        assert!(!avoidance_matches(r"D:\Apps\Editor.exe",&settings));
        settings.browser_minimal=true;
        assert!(avoidance_matches(r"D:\Quark\quark.exe",&settings));
        assert!(avoidance_matches(r"d:\apps\EDITOR.EXE",&settings));
        assert!(!avoidance_matches(r"C:\Other\Editor.exe",&settings));
        settings.avoid_browsers=false;
        assert!(!avoidance_matches(r"D:\Quark\quark.exe",&settings));
        assert!(avoidance_matches(r"D:\Apps\Editor.exe",&settings));
        settings.avoid_apps.clear();
        assert!(!avoidance_matches(r"D:\Apps\Editor.exe",&settings));
    }
    #[test]
    fn avoidance_window_list_excludes_this_process_and_deduplicates_apps() {
        let entries=avoidance_windows().unwrap();
        let current=std::env::current_exe().unwrap().to_string_lossy().into_owned();
        for (index,app) in entries.iter().enumerate() {
            assert!(!crate::model::same_app(&app.path,&current));
            assert!(!app.name.is_empty() && !app.title.is_empty());
            assert!(!entries[..index].iter().any(|previous|crate::model::same_app(&previous.path,&app.path)));
        }
        let notepad=std::path::Path::new(r"C:\Windows\System32\notepad.exe");
        if notepad.exists(){assert!(executable_description(notepad.to_str().unwrap()).is_some());}
    }
}
