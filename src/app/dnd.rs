//! 勿扰模式：模拟打开控制中心，切换系统「请勿打扰」。
//!
//! 系统没有给普通应用直接开关勿扰的接口（`FocusSessionManager` 是受限功能，
//! 没有微软下发的解锁令牌会返回 `E_ACCESSDENIED`），只能通过 UI 自动化点控制中心里的开关。
//! 开关状态也只在操作时读得到，所以月亮是否常驻以本应用最后一次切换的结果为准。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

const OFF: u8 = 0;
const ON: u8 = 1;
const UNKNOWN: u8 = 2;

pub(crate) struct Dnd {
    pub active: bool,
    state: Arc<AtomicU8>,
    busy: Arc<AtomicBool>,
}

impl Default for Dnd {
    fn default() -> Self {
        Self { active: false, state: Arc::new(AtomicU8::new(UNKNOWN)), busy: Arc::new(AtomicBool::new(false)) }
    }
}

impl Dnd {
    #[cfg(test)]
    pub(crate) fn fake(active: bool) -> Self {
        Self { active, state: Arc::new(AtomicU8::new(if active { ON } else { OFF })), busy: Arc::new(AtomicBool::new(false)) }
    }

    /// 把后台切换线程读到的结果同步到界面。
    pub fn tick(&mut self, _t: f64) {
        self.active = self.state.load(Ordering::Relaxed) == ON;
    }

    #[cfg(not(test))]
    pub fn toggle(&mut self, _t: f64) {
        if self.busy.swap(true, Ordering::SeqCst) { return; }
        let state = self.state.clone();
        let busy = self.busy.clone();
        std::thread::spawn(move || {
            state.store(automation::toggle(), Ordering::Relaxed);
            busy.store(false, Ordering::Relaxed);
        });
    }

    /// 测试不碰系统界面，点击只翻转本地值。
    #[cfg(test)]
    pub fn toggle(&mut self, _t: f64) {
        self.active = !self.active;
        self.state.store(if self.active { ON } else { OFF }, Ordering::Relaxed);
    }
}

#[cfg(all(windows, not(test)))]
mod automation {
    use super::{OFF, ON, UNKNOWN};
    use std::time::{Duration, Instant};
    use windows::core::{w, IUnknown};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationInvokePattern, IUIAutomationTogglePattern,
        IUIAutomationTreeWalker, UIA_InvokePatternId, UIA_TogglePatternId,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_ESCAPE,
    };
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, SW_SHOWNORMAL};

    /// 控制中心里「请勿打扰」开关的 AutomationId，与系统语言无关。
    const TOGGLE_ID: &str = "DoNotDisturbButton";

    fn send_escape() {
        let input = |up| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_ESCAPE,
                    wScan: 0,
                    dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe { SendInput(&[input(false), input(true)], std::mem::size_of::<INPUT>() as i32); }
    }

    fn foreground() -> (HWND, String) {
        unsafe {
            let hwnd = GetForegroundWindow();
            let mut class = [0u16; 256];
            let n = GetClassNameW(hwnd, &mut class).max(0) as usize;
            (hwnd, String::from_utf16_lossy(&class[..n]))
        }
    }

    fn open_settings() {
        unsafe { ShellExecuteW(None, w!("open"), w!("ms-settings:quiethours"), None, None, SW_SHOWNORMAL); }
    }

    unsafe fn find_by(
        walker: &IUIAutomationTreeWalker,
        element: &IUIAutomationElement,
        depth: usize,
        hit: &dyn Fn(&IUIAutomationElement) -> bool,
    ) -> Option<IUIAutomationElement> {
        if depth > 18 { return None; }
        if hit(element) { return Some(element.clone()); }
        if let Ok(child) = walker.GetFirstChildElement(element) {
            let mut current = Some(child);
            while let Some(node) = current {
                if let Some(found) = find_by(walker, &node, depth + 1, hit) { return Some(found); }
                current = walker.GetNextSiblingElement(&node).ok();
            }
        }
        None
    }

    fn is_toggle(element: &IUIAutomationElement) -> bool {
        unsafe { element.CurrentAutomationId().is_ok_and(|id| id.to_string() == TOGGLE_ID) }
    }

    fn is_clock(element: &IUIAutomationElement) -> bool {
        unsafe { element.CurrentClassName().is_ok_and(|class| class.to_string() == "SystemTray.OmniButton") }
    }

    /// 通知中心窗口是当前前台窗口时只搜它，避免每轮都遍历整个桌面。
    unsafe fn find_toggle(automation: &IUIAutomation, walker: &IUIAutomationTreeWalker) -> Option<IUIAutomationElement> {
        let (hwnd, class) = foreground();
        if class == "Windows.UI.Core.CoreWindow" {
            if let Ok(panel) = automation.ElementFromHandle(hwnd) {
                if let Some(element) = find_by(walker, &panel, 0, &is_toggle) { return Some(element); }
            }
        }
        let root = automation.GetRootElement().ok()?;
        find_by(walker, &root, 0, &is_toggle)
    }

    /// 点任务栏时钟打开通知中心；返回是否成功点开。
    unsafe fn open_panel(automation: &IUIAutomation, walker: &IUIAutomationTreeWalker) -> bool {
        let Some(root) = automation.GetRootElement().ok() else { return false; };
        let Some(clock) = find_by(walker, &root, 0, &is_clock) else { return false; };
        let Ok(invoke) = clock.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId) else { return false; };
        invoke.Invoke().is_ok()
    }

    /// 返回切换后的状态；自动化失败时退回系统勿扰设置页，返回未知。
    pub(super) fn toggle() -> u8 {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let automation = CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None::<&IUnknown>, CLSCTX_INPROC_SERVER);
            let Ok(automation) = automation else { open_settings(); return UNKNOWN; };
            let Ok(walker) = automation.ControlViewWalker() else { open_settings(); return UNKNOWN; };
            if !open_panel(&automation, &walker) {
                open_settings();
                return UNKNOWN;
            }
            let start = Instant::now();
            let mut target = None;
            while start.elapsed() < Duration::from_millis(2500) {
                std::thread::sleep(Duration::from_millis(120));
                if let Some(element) = find_toggle(&automation, &walker) {
                    target = Some(element);
                    break;
                }
            }
            let Some(element) = target else {
                send_escape();
                open_settings();
                return UNKNOWN;
            };
            let Ok(pattern) = element.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId) else {
                send_escape();
                open_settings();
                return UNKNOWN;
            };
            let before = pattern.CurrentToggleState().map(|state| state.0).unwrap_or(-1);
            let _ = pattern.Toggle();
            // 等开关动画把 ToggleState 改过来，读不到变化就用旧值。
            let mut state = before;
            for _ in 0..8 {
                std::thread::sleep(Duration::from_millis(80));
                if let Ok(current) = pattern.CurrentToggleState() {
                    state = current.0;
                    if state != before { break; }
                }
            }
            send_escape();
            match state {
                1 => ON,
                0 => OFF,
                _ => if before == 1 { OFF } else { ON },
            }
        }
    }
}

#[cfg(all(not(windows), not(test)))]
mod automation {
    use super::UNKNOWN;
    pub(super) fn toggle() -> u8 { UNKNOWN }
}
