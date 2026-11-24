//! 各音乐客户端的专用读取与控制。

pub(crate) mod apple_music;
#[cfg(windows)]
pub(crate) mod bodian;
#[cfg(windows)]
pub(crate) mod kugou_control;
#[cfg(windows)]
pub(crate) mod kugou_metadata;
#[cfg(windows)]
pub(crate) mod kuwo;
#[cfg(windows)]
pub(crate) mod netease_playback;
#[cfg(windows)]
pub(crate) mod netease_timeline;

// 播放器适配文件通过 `super::` 使用播放核心的接口。
#[cfg(windows)]
pub(crate) use crate::music::diagnostics::log as music_log;
#[cfg(windows)]
pub(crate) use crate::music::playback::process as player_process;
#[cfg(windows)]
pub(crate) use crate::music::playback::{decode_artwork, seek_arrived, MediaKind, NowPlaying, Transport};
#[cfg(all(windows, test))]
pub(crate) use crate::music::playback::{seek, shared, spawn_poller};
