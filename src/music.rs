//! 音乐：系统媒体会话、各播放器适配、歌词、播放页与设置。

pub(crate) mod cover;
#[cfg(windows)]
pub(crate) mod diagnostics;
pub mod lyrics;
pub(crate) mod page;
pub(crate) mod playback;
pub(crate) mod players;
pub(crate) mod settings;

pub use playback::{
    shared, spawn_poller, ControlWorker, MediaKind, NowPlaying, Poller, Shared, Transport,
    POLL_BACKGROUND, POLL_FAST, POLL_LYRICS,
};
pub use playback::seek;
#[cfg(test)]
pub(crate) use playback::is_qqmusic;
