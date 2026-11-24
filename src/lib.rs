//! 岛仔：Windows 顶部「灵动岛」。
//!
//! - `app`：与平台无关的界面与动效（外壳与录制器共用）
//! - `render`：弹簧、矢量绘制、面板轮廓、配色字体
//! - `model` / `lyrics` / `smtc`：数据、歌词、系统媒体
//! - `platform`：仅 Windows 的窗口区域、命中穿透、托盘与系统数据

pub mod platform;
pub mod render;
pub mod poll_clock;
pub mod resources;
pub mod app;
pub mod agent;
#[cfg(test)]
mod test_support;
pub mod messenger;
pub mod child_process;
pub mod bt;
pub mod calendar;
pub mod lyrics;
pub mod mail;
pub mod clipboard;
pub mod model;
pub mod quota;
pub mod weather;
mod pet;
mod pet_expressions;
mod bar_layout;
pub mod smtc;
pub mod settings_theme;
mod updates;
#[cfg(windows)]
pub use updates::record_install_size;

#[cfg(test)]
mod ui_tests;
#[cfg(test)]
#[path = "bin/daozai-record/raster.rs"]
mod preview_raster;
