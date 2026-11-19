//! AI 会话提示音：直接播放内嵌 WAV，不启动 PowerShell。

use crate::agent::store::{SoundConfig, SoundKind};

pub(crate) fn play(kind: SoundKind, config: &SoundConfig) {
    if !config.enabled { return; }
    let enabled = match kind {
        SoundKind::Start => config.start,
        SoundKind::Submit => config.submit,
        SoundKind::Approval => config.approval,
        SoundKind::Complete => config.complete,
        SoundKind::Error => config.error,
    };
    if !enabled { return; }
    let wav: &'static [u8] = match kind {
        SoundKind::Start => include_bytes!("../assets/agent-sounds/start.wav"),
        SoundKind::Submit => include_bytes!("../assets/agent-sounds/submit.wav"),
        SoundKind::Approval => include_bytes!("../assets/agent-sounds/approval.wav"),
        SoundKind::Complete => include_bytes!("../assets/agent-sounds/complete.wav"),
        SoundKind::Error => include_bytes!("../assets/agent-sounds/error.wav"),
    };
    play_bytes(wav);
}

#[cfg(windows)]
fn play_bytes(wav: &'static [u8]) {
    use windows::core::PCWSTR;
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT, SND_NOSTOP};
    unsafe { let _ = PlaySoundW(PCWSTR(wav.as_ptr().cast()), None, SND_MEMORY | SND_ASYNC | SND_NODEFAULT | SND_NOSTOP); }
}

#[cfg(not(windows))]
fn play_bytes(_wav: &'static [u8]) {}
