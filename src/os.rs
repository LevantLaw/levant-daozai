//! Windows 平台接入：窗口、输入、托盘、系统数据与诊断。
pub mod bt;
pub mod display_layout;
pub mod diagnostics_tool;
pub mod icons;
pub mod input;
pub mod oled_shift;

#[cfg(windows)]
pub mod backdrop;
#[cfg(windows)]
pub mod composition;
#[cfg(windows)]
pub mod layered;
#[cfg(windows)]
pub mod native;
#[cfg(windows)]
pub mod status;
#[cfg(windows)]
pub mod system;
#[cfg(windows)]
pub mod tray;
