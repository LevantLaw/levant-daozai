//! 岛仔：Windows 顶部「灵动岛」。
//!
//! - `app`：与平台无关的界面与动效（外壳与录制器共用）
//! - `render`：弹簧、矢量绘制、面板轮廓、配色字体
//! - `model` / `lyrics` / `smtc`：数据、歌词、系统媒体
//! - `platform`：仅 Windows 的窗口区域、命中穿透、托盘与系统数据
//! - `support`：跨功能的进程、时钟、资源与测试辅助

pub mod platform;
pub mod render;
pub mod support;
pub mod app;
pub mod agent;
pub mod messenger;
pub mod calendar;
pub mod lyrics;
pub mod mail;
pub mod model;
pub mod quota;
pub mod weather;
pub mod smtc;
#[cfg(windows)]
pub use app::updates::record_install_size;

#[cfg(test)]
mod ui_tests;
#[cfg(test)]
#[path = "bin/daozai-record/raster.rs"]
mod preview_raster;
