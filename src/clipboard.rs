//! 剪贴板记录：本机历史、岛内页面与设置。

mod history;
pub(crate) mod page;
pub(crate) mod settings;

pub use history::{preview, Clip, History, Shared, Watcher, watch, MAX_CHARS, MAX_PINNED, MAX_RECENT};
#[cfg(windows)]
pub use history::copy_history;
