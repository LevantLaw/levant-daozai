//! 播放诊断：读取间隙与播放器状态日志。

#[cfg(windows)]
pub(crate) mod gaps;
#[cfg(windows)]
pub(crate) mod log;
