//! 系统状态采样：音量、静音、网络、大小写锁定与锁屏、空闲时间。

use std::sync::mpsc::{self, Receiver};
use windows::core::Result;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, GetLastInputInfo, LASTINPUTINFO};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub volume: Option<(u32, bool)>,
    pub audio_endpoint: Option<String>,
    pub caps: bool,
    pub network: bool,
    pub network_kind: NetworkKind,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Observation {
    pub system: Snapshot,
    pub input: Option<crate::os::input::State>,
    pub fullscreen: Option<crate::os::display_layout::Bounds>,
}

/// 跨进程输入法查询和 COM 状态采样留在后台，只在状态改变时唤醒界面。
pub fn observe(ctx:eframe::egui::Context,initial:Snapshot,input:Option<crate::os::input::State>)->Receiver<Observation> {
    let(tx,rx)=mpsc::sync_channel(1);
    std::thread::Builder::new().name("daozai-system".into()).spawn(move||{
        let mut current=Observation{system:initial,input,fullscreen:None};
        let mut delivered=None;
        let mut sampled=crate::support::poll_clock::PollClock::default();
        let mut input_at=sampled;
        loop {
            let identity=crate::os::input::State::identity();
            let changed=identity.map(|state|(state.context,state.layout))!=current.input.map(|state|(state.context,state.layout));
            if changed || crate::os::input::switch_recent() || input_at.ready(std::time::Duration::from_secs(1)){
                current.input=crate::os::input::State::read();input_at=crate::support::poll_clock::PollClock::now();
            }
            if sampled.ready(std::time::Duration::from_millis(500)){
                current.system=Snapshot::read();current.fullscreen=crate::os::foreground::fullscreen();sampled=crate::support::poll_clock::PollClock::now();
            }
            if delivered.as_ref()!=Some(&current) {
                match tx.try_send(current.clone()) {
                    Ok(())=>{delivered=Some(current.clone());ctx.request_repaint();},
                    Err(mpsc::TrySendError::Disconnected(_))=>return,
                    Err(mpsc::TrySendError::Full(_))=>{},
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }).ok();
    rx
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NetworkKind { Wifi, Ethernet, #[default] Other }
impl Snapshot {
    pub fn read() -> Self {
        unsafe {
            let device=audio_device().ok();
            let audio_endpoint=device.as_ref().and_then(|d|d.GetId().ok()).and_then(|id|{
                let text=id.to_string().ok();
                windows::Win32::System::Com::CoTaskMemFree(Some(id.0 as _));
                text
            });
            let volume = device.and_then(|d|d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL,None).ok()).and_then(|a| {
                Some((
                    (a.GetMasterVolumeLevelScalar().ok()? * 100.0).round() as u32,
                    a.GetMute().ok()?.as_bool(),
                ))
            });
            let profile=windows::Networking::Connectivity::NetworkInformation::GetInternetConnectionProfile().ok();
            let network=profile.as_ref().and_then(|p|p.GetNetworkConnectivityLevel().ok()).is_some_and(|v|v==windows::Networking::Connectivity::NetworkConnectivityLevel::InternetAccess);
            let network_kind=profile.as_ref().map(|p| {
                if p.IsWlanConnectionProfile().unwrap_or(false){NetworkKind::Wifi}
                else if p.NetworkAdapter().ok().and_then(|a|a.IanaInterfaceType().ok())==Some(6){NetworkKind::Ethernet}
                else{NetworkKind::Other}
            }).unwrap_or_default();
            Self{volume,audio_endpoint,network,network_kind,caps:GetKeyState(0x14)&1!=0}
        }
    }
    pub fn changes(&self, old: &Self) -> Vec<(String, String)> {
        let mut out = vec![];
        if self.audio_endpoint == old.audio_endpoint && self.volume != old.volume {
            if let Some((v, mute)) = self.volume {
                out.push((
                    "系统音量".into(),
                    if mute {
                        "已静音".into()
                    } else {
                        format!("音量 {v}%")
                    },
                ));
            }
        }
        if self.caps != old.caps {
            out.push((
                "大写锁定".into(),
                if self.caps { "已开启" } else { "已关闭" }.into(),
            ));
        }
        if self.network != old.network || self.network && self.network_kind != old.network_kind {
            let kind=if self.network {self.network_kind}else{old.network_kind};
            let status=if self.network {"网络已连接"}else{"网络已断开"};
            out.push((
                "网络连接".into(),
                match kind {NetworkKind::Wifi=>format!("Wi-Fi · {status}"),NetworkKind::Ethernet=>format!("以太网 · {status}"),NetworkKind::Other=>status.into()},
            ));
        }
        out
    }
}
unsafe fn audio_device() -> Result<IMMDevice> {    thread_local! {static INITIALIZED:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
    INITIALIZED.with(|v| {
        if !v.replace(true) {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
    });
    // 每个采样线程复用枚举器，但每次仍查询当前默认设备，保留热插拔响应。
    thread_local! {static ENUMERATOR:std::cell::RefCell<Option<IMMDeviceEnumerator>>=const{std::cell::RefCell::new(None)};}
    ENUMERATOR.with(|slot| {
        let mut slot=slot.borrow_mut();
        if slot.is_none(){*slot=Some(CoCreateInstance(&MMDeviceEnumerator,None,CLSCTX_ALL)?);}
        let result=slot.as_ref().unwrap().GetDefaultAudioEndpoint(eRender,eConsole);
        if result.is_err(){*slot=None;}
        result
    })
}
unsafe fn audio() -> Result<IAudioEndpointVolume> {audio_device()?.Activate(CLSCTX_ALL,None)}
/// 设置主音量（0..100）；同时解除静音，拖动音量条时立刻能听到。
#[cfg(windows)]
pub fn set_volume(percent: u32) {
    unsafe {
        if let Ok(a) = audio() {
            let _ = a.SetMasterVolumeLevelScalar((percent.min(100) as f32) / 100.0, std::ptr::null());
            let _ = a.SetMute(false, std::ptr::null());
        }
    }
}

/// 切换静音，返回切换后的静音状态（拿不到设备时返回 None）。
#[cfg(windows)]
pub fn toggle_mute() -> Option<bool> {
    unsafe {
        let a = audio().ok()?;
        let muted = a.GetMute().ok()?.as_bool();
        a.SetMute(!muted, std::ptr::null()).ok()?;
        Some(!muted)
    }
}

/// 当前会话是否锁屏。查询走 WTS（一次本机 RPC），按 2 秒缓存，避免每帧都查。
pub fn screen_locked() -> bool {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use windows::Win32::System::RemoteDesktop::{
        WTSFreeMemory, WTSQuerySessionInformationW, WTSSessionInfoEx, WTSINFOEXW, WTS_CURRENT_SERVER_HANDLE,
        WTS_CURRENT_SESSION, WTS_SESSIONSTATE_LOCK,
    };
    static CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, locked)) = *cache {
        if at.elapsed() < Duration::from_secs(2) { return locked; }
    }
    let locked = unsafe {
        let mut buf = windows::core::PWSTR::null();
        let mut len = 0u32;
        if WTSQuerySessionInformationW(WTS_CURRENT_SERVER_HANDLE, WTS_CURRENT_SESSION, WTSSessionInfoEx, &mut buf, &mut len).is_ok() && !buf.is_null() {
            let info = &*(buf.0 as *const WTSINFOEXW);
            let locked = info.Level == 1 && info.Data.WTSInfoExLevel1.SessionFlags == WTS_SESSIONSTATE_LOCK as i32;
            WTSFreeMemory(buf.0 as *mut core::ffi::c_void);
            locked
        } else {
            false
        }
    };
    *cache = Some((Instant::now(), locked));
    locked
}

pub fn idle_seconds() -> f64 {
    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            ..Default::default()
        };
        if GetLastInputInfo(&mut info).as_bool() {
            windows::Win32::System::SystemInformation::GetTickCount().wrapping_sub(info.dwTime)
                as f64
                / 1000.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_connection_uses_connected_wording() {
        let old=Snapshot{volume:None,audio_endpoint:None,caps:false,network:false,network_kind:NetworkKind::Other};
        let mut connected=old.clone();connected.network=true;
        assert_eq!(connected.changes(&old),vec![("网络连接".into(),"网络已连接".into())]);
    }
    #[test]
    fn network_switches_transport_and_keeps_the_disconnected_type() {
        let wifi=Snapshot{volume:None,audio_endpoint:None,caps:false,network:true,network_kind:NetworkKind::Wifi};
        let mut cable=wifi.clone();cable.network_kind=NetworkKind::Ethernet;
        assert_eq!(cable.changes(&wifi),vec![("网络连接".into(),"以太网 · 网络已连接".into())]);
        let mut off=cable.clone();off.network=false;off.network_kind=NetworkKind::Other;
        assert_eq!(off.changes(&cable),vec![("网络连接".into(),"以太网 · 网络已断开".into())]);
        assert_eq!(wifi.changes(&off),vec![("网络连接".into(),"Wi-Fi · 网络已连接".into())]);
    }
    #[test]
    fn switching_audio_endpoints_is_not_a_volume_adjustment() {
        let old=Snapshot{volume:Some((20,false)),audio_endpoint:Some("speakers".into()),caps:false,network:true,network_kind:NetworkKind::Other};
        let mut new=old.clone();
        new.audio_endpoint=Some("headphones".into());new.volume=Some((80,false));new.caps=true;
        assert_eq!(new.changes(&old),vec![("大写锁定".into(),"已开启".into())]);
        let mut adjusted=new.clone();adjusted.volume=Some((60,false));
        assert_eq!(adjusted.changes(&new),vec![("系统音量".into(),"音量 60%".into())]);
        adjusted.volume=Some((60,true));
        assert_eq!(adjusted.changes(&new),vec![("系统音量".into(),"已静音".into())]);
        assert!(!old.changes(&new).iter().any(|(title,_)|title=="系统音量"));
    }

}
