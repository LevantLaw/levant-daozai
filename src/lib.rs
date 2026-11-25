//! 岛仔：Windows 顶部「灵动岛」。
//!
//! - `island`：与平台无关的界面与动效（外壳与录制器共用）
//! - `visuals`：弹簧、矢量绘制、面板轮廓、配色字体
//! - `music`：系统媒体会话、播放器适配、歌词、播放页与设置
//! - `model`：数据与设置
//! - `os`：仅 Windows 的窗口区域、命中穿透、托盘与系统数据
//! - `support`：跨功能的进程、时钟、资源与测试辅助
//! - 其余顶层模块按功能划分：`ai_session`、`calendar`、`clipboard`、`focus`、
//!   `health`、`instant_messaging`、`mail`、`quota`、`shelf`、`todo`、`weather`、
//!   `windows_toasts`

pub mod os;
pub mod visuals;
pub mod support;
pub mod island;
pub mod ai_session;
pub mod instant_messaging;
pub mod windows_toasts;
pub mod calendar;
pub mod mail;
pub mod clipboard;
pub mod focus;
pub mod health;
pub mod model;
pub mod music;
pub mod shelf;
pub mod todo;
pub mod quota;
pub mod weather;
#[cfg(windows)]
pub use island::updates::record_install_size;

#[cfg(test)]
mod ui_tests;
#[cfg(test)]
#[path = "bin/daozai-record/raster.rs"]
mod preview_raster;
