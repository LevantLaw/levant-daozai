//! Windows 通知：读取、发送方核对、历史、岛内页面与设置。

pub(crate) mod history;
#[cfg(windows)]
pub(crate) mod listener;
pub(crate) mod page;
pub(crate) mod settings;
#[cfg(windows)]
mod toast_source;

pub use history::WindowsNotice;
#[cfg(windows)]
pub use listener::{connect, connect_with_settings, Incoming, Notifications};
#[cfg(windows)]
pub(crate) use listener::{
    notification_apps_enabled, notification_enabled, set_system_notifications,
    system_notifications_enabled,
};
