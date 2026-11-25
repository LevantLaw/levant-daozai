//! 歌词状态、查询调度与岛内歌词时间轴。

use super::*;
use crate::island::{App, Env, MUSIC_PAGE};
use eframe::egui;

const NATIVE_RETRIES:u32=12;
const NETWORK_RETRIES:u32=10;

/// 发起歌词查询时的歌曲信息，用来判断之后的变化是换歌还是信息补全。
#[derive(Clone,Debug,Default)]
pub(crate) struct LyricSong {pub source:String,pub title:String,pub artist:String,pub duration:f64,pub song_id:Option<u64>}
impl LyricSong {
    fn of(media:&NowPlaying)->Self {
        Self{source:media.source.clone(),title:media.title.clone(),artist:media.artist.clone(),duration:media.duration,song_id:media.song_id}
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum SongChange {
    /// 换了一首歌：立即清空旧歌词并重新查询。
    Different,
    /// 同一首，只是时长、写法等补全：已有同步歌词时不再查询。
    Same,
    /// 同一首，署名有补全：重新查询，但查询期间继续显示已有歌词。
    Refined,
}

fn song_change(previous:&LyricSong,media:&NowPlaying)->SongChange {
    if !previous.source.eq_ignore_ascii_case(&media.source) {return SongChange::Different;}
    match (previous.song_id,media.song_id) {
        (Some(old),Some(new))=>return if old==new {SongChange::Same}else{SongChange::Different},
        _=>{}
    }
    // 同名的不同版本（现场版、伴奏等）时长通常不同：两边都已知且相差超过 3 秒视为不同歌曲。
    if previous.duration>0.0 && media.duration>0.0 && (previous.duration-media.duration).abs()>3.0 {return SongChange::Different;}
    match lyrics::song_relation(&previous.title,&previous.artist,&media.title,&media.artist) {
        Some(true)=>SongChange::Same,
        Some(false)=>SongChange::Refined,
        None=>SongChange::Different,
    }
}

/// 等待旧请求结束后再发起的在线查询（同一时间只保留一个网络线程）。
pub(crate) struct LyricsJob {
    source:String,title:String,artist:String,duration:f64,song_id:Option<u64>,
    cached:Option<lyrics::Lyrics>,native_only:bool,manual:bool,
}

#[derive(Default)]
pub(crate) struct LyricsState {
    pub key: String,
    /// 本次查询对应的歌曲；为空表示还没有按歌曲发起过查询。
    pub song: Option<LyricSong>,
    /// 旧请求未结束时排队的在线查询；本机歌词轮询不等它，换歌后立即开始。
    pub pending: Option<LyricsJob>,
    pub island_animation_started: bool,
    pub scroll_animation_started: bool,
    pub loading: bool,
    pub error: Option<lyrics::FetchError>,
    pub retry_at: Option<f64>,
    /// 发起本次查询时已知的歌曲 ID；之后才拿到 ID 且还没有同步歌词时，按 ID 再查一次。
    pub song_id: Option<u64>,
    /// 同一首歌已自动重试的次数；达到上限后停止，避免整首歌期间反复读盘或联网。
    pub attempts: u32,
    /// 用户点了「重试」：下一次查询跳过近期「没有歌词」的记录。
    pub manual_retry: bool,
    pub lyrics: Option<lyrics::Lyrics>,
    pub rx: Option<std::sync::mpsc::Receiver<Result<(lyrics::Lyrics,bool), lyrics::FetchError>>>,
    pub native_rx: Option<std::sync::mpsc::Receiver<Option<lyrics::Lyrics>>>,
}

impl App {
    /// 外壳据此调整音乐采样频率：看着音乐页时最快，收起显示歌词次之，其余只需跟上歌名和播放状态。
    pub fn media_poll_interval(&self)->std::time::Duration {
        if self.open && self.page==MUSIC_PAGE {crate::music::POLL_FAST}
        else if self.island_lyrics_active() && !self.open {crate::music::POLL_LYRICS}
        else {crate::music::POLL_BACKGROUND}
    }

    pub(crate) fn island_lyrics_active(&self)->bool {
        if self.side() {return false;}
        self.settings.island_lyrics && self.settings.visible_pages[0] && self.media.is_music() && self.media.playing
            && !self.browser_minimal_active
    }
    pub(crate) fn compact_notices(&self)->bool {self.island_lyrics_active()&&!self.open&&!self.notice.as_ref().is_some_and(|a|a.meeting.is_some()||self.settings.notification_preview&&matches!(a.source,crate::island::banner::AlertSource::External(_)))}

    /// 换句、分段和展开高亮共用时间调整；正数延后，负数提前。
    pub(crate) fn lyric_position(&self)->f32 {(self.media.position-self.settings.lyric_delay_ms as f64/1000.0)as f32}

    pub(crate) fn island_lyric(&self) -> Option<(usize,&str)> {
        if !self.island_lyrics_active(){return None;}
        if !lyric_key_matches(&self.ly.key,&self.media) {return None;}
        let lyrics=self.ly.lyrics.as_ref().filter(|lyrics|lyrics.synced)?;
        let index=lyrics.index_at(self.lyric_position())?;
        let text=lyrics.lines.get(index)?.text.trim();
        (!text.is_empty()).then_some((index,text))
    }

    /// 媒体进度在两次 SMTC 刷新之间按时间外推，进度条才会连续走动。
    pub(crate) fn update_media(&mut self, env: &Env, ctx: &egui::Context) {
        let m = &env.media;
        if !m.is_music() {
            self.music_volume_ui=None;
            self.music_volume_quiet_until=0.0;
            self.music_volume_open=false;
            self.media=NowPlaying::default();
            self.media_anchor=(String::new(),0.0,0.0,false);
            self.music_artwork=None;
            self.ly=LyricsState::default();
            return;
        }
        let a = &self.media_anchor;
        if a.0 != m.title || (a.1 - m.position).abs() > 0.01 || a.3 != m.playing {
            self.media_anchor = (m.title.clone(), m.position, env.t, m.playing);
        }
        let mut np = m.clone();
        if self.media.source!=m.source || m.volume.is_none(){self.music_volume_open=false;}
        if self.media.source!=m.source || self.t>=self.music_volume_quiet_until || m.volume.is_none() {
            self.music_volume_ui=m.volume;
        }
        if m.playing {
            let (_, pos, _, _) = &self.media_anchor;
            np.position = if let Some(sampled)=m.sampled_at {m.position+sampled.elapsed().as_secs_f64()}
                else {*pos};
            if m.duration>0.0 {np.position=np.position.min(m.duration);}
        }
        // 网易云的 song_id 只来自本机日志，SMTC 那一路没有；None 与 Some 之间来回不是换歌，不能每次都清空歌词重查。
        let other_song=matches!((self.media.song_id,np.song_id),(Some(old),Some(new)) if old!=new);
        if self.media.source != np.source || other_song { self.ly=LyricsState::default(); }
        self.media = np;
        if let Some(image)=&self.media.artwork {
            if !self.music_artwork.as_ref().is_some_and(|(old,_)|std::sync::Arc::ptr_eq(old,image)) {
                self.music_artwork=Some((image.clone(),ctx.load_texture("music-artwork",image.clone(),egui::TextureOptions::LINEAR)));
            }
        } else {self.music_artwork=None;}
    }

    pub(crate) fn poll_lyrics(&mut self) {
        if !self.media.is_music(){self.ly=LyricsState::default();return;}
        if !self.show_lyrics && !self.settings.island_lyrics {self.ly=LyricsState::default();return;}
        let np = &self.media;
        if np.title.is_empty(){self.ly=LyricsState::default();return;}
        let key = format!("{}|{}|{:.0}", np.title, np.artist, np.duration);
        let id_arrived=key==self.ly.key && !self.ly.loading && self.ly.song_id.is_none() && np.song_id.is_some()
            && !self.ly.lyrics.as_ref().is_some_and(|lyrics|lyrics.synced);
        if key != self.ly.key {
            let change=self.ly.song.as_ref().map_or(SongChange::Different,|song|song_change(song,np));
            let synced=self.ly.lyrics.as_ref().is_some_and(|lyrics|lyrics.synced);
            match change {
                // 时长、写法补全：已有同步歌词就沿用，不清空、不重查。
                SongChange::Same if synced=>{self.ly.key=key;self.ly.song=Some(LyricSong::of(np));}
                // 换歌：立即切换显示状态，旧歌词和迟到的旧结果都不再显示。
                SongChange::Different=>self.start_lyrics(key,None,false),
                // 同一首的补全：重新查询，期间继续显示已有歌词。
                _=>{let keep=self.ly.lyrics.take();self.start_lyrics(key,keep,false);}
            }
        } else if id_arrived || self.ly.retry_at.is_some_and(|at|self.t>=at) {
            self.start_lyrics(key,None,true);
        }
        self.spawn_lyrics_job();
        if let Some(rx)=&self.ly.native_rx {
            let mut found=None;
            while let Ok(result)=rx.try_recv(){if result.is_some(){found=result;}}
            if let Some(lyrics)=found {
                self.ly.lyrics=Some(lyrics);self.ly.loading=false;self.ly.error=None;
                // 本机歌词已到：排队中的在线查询不再需要，进行中的结果也丢弃。
                self.ly.retry_at=None;self.ly.rx=None;self.ly.native_rx=None;self.ly.pending=None;
            }
        }
        if let Some(rx) = &self.ly.rx {
            if let Ok(r) = rx.try_recv() {
                self.ly.loading = false;
                match r {
                    Ok((l,waiting_native)) => {
                        self.ly.error=None;
                        self.ly.retry_at=None;
                        if (waiting_native || lyrics::is_soda(&self.media.source) && !l.synced) && self.ly.attempts<NATIVE_RETRIES {self.ly.retry_at=Some(self.t+5.0);}
                        self.ly.lyrics = Some(l);
                    },
                    Err(error) => {
                        let native=lyrics::is_soda(&self.media.source) || self.media.source.to_ascii_lowercase().contains("kugou") && error==lyrics::FetchError::NotFound;
                        if native {if self.ly.attempts<NATIVE_RETRIES{self.ly.retry_at=Some(self.t+5.0);}}
                        else if matches!(error,lyrics::FetchError::Unavailable(_)|lyrics::FetchError::InvalidResponse(_)) && self.ly.attempts<NETWORK_RETRIES {self.ly.retry_at=Some(self.t+30.0);}
                        self.ly.error = Some(error);
                    },
                }
                self.ly.rx = None;
            }
        }
    }

    /// 为当前歌曲建立新的歌词状态：本机歌词轮询立即开始，在线查询交给 `spawn_lyrics_job`。
    /// `keep`：同一首歌信息补全时继续显示的歌词；`retry`：同一首歌的定时重试或补到歌曲 ID。
    pub(crate) fn start_lyrics(&mut self,key:String,keep:Option<lyrics::Lyrics>,retry:bool) {
        let np=&self.media;
        let kugou=np.source.to_ascii_lowercase().contains("kugou");
        let native_only=retry && kugou && self.ly.error==Some(lyrics::FetchError::NotFound);
        let cached=if retry && kugou {self.ly.lyrics.take()}else{None};
        let attempts=if retry {self.ly.attempts.saturating_add(1)}else{0};
        let manual=self.ly.manual_retry;
        let shown=cached.clone().or(keep);
        self.ly = LyricsState { key, loading: shown.is_none() && !native_only, lyrics:shown,
            error:native_only.then_some(lyrics::FetchError::NotFound), song_id:np.song_id, attempts,
            song:Some(LyricSong::of(np)), ..Default::default() };
        let (source, title, artist, dur) = (np.source.clone(), np.title.clone(), np.artist.clone(), np.duration);
        let song_id=np.song_id;
        if kugou || lyrics::is_soda(&source) {
            let (tx,rx)=std::sync::mpsc::sync_channel(1);self.ly.native_rx=Some(rx);
            let (source,title,artist)=(source.clone(),title.clone(),artist.clone());
            let native=crate::support::resources::NATIVE_LYRICS.spawn(move ||{
                // 客户端通常在开播几秒内写好歌词：先密后疏（0.5s 起，最长 5s 一次），最多等 90 秒。
                // 汽水的缓存文件可达 32MB，修改时间和大小都没变时不重复整份读取，但每 15 秒仍完整核对一次。
                let started=std::time::Instant::now();
                let mut wait=std::time::Duration::from_millis(500);
                let mut last:Option<((std::time::SystemTime,u64),std::time::Instant)>=None;
                let mut miss="未读取";
                loop {
                    let print=lyrics::native_fingerprint(&source);
                    let unchanged=print.is_some_and(|print|last.is_some_and(|(seen,at)|seen==print && at.elapsed()<std::time::Duration::from_secs(15)));
                    let found=if unchanged {None}else{
                        if let Some(print)=print {last=Some((print,std::time::Instant::now()));}
                        lyrics::read_native_detail(&source,&title,&artist,dur).map_err(|reason|miss=reason).ok()
                    };
                    // 找到歌词时等界面取走再结束（换歌或关闭歌词后通道断开会立即返回），不因通道暂满丢掉结果。
                    if found.is_some() {
                        if tx.send(found).is_ok(){lyrics::log_native(&source,&title,&artist,"命中",started.elapsed());}
                        break;
                    }
                    if let Err(std::sync::mpsc::TrySendError::Disconnected(_))=tx.try_send(None){break;}
                    if started.elapsed()>=std::time::Duration::from_secs(90){lyrics::log_native(&source,&title,&artist,&format!("放弃:{miss}"),started.elapsed());break;}
                    std::thread::sleep(wait);
                    wait=(wait*2).min(std::time::Duration::from_secs(5));
                }
            });
            // 快速切歌时本机轮询线程已满：不留空通道，之后的定时重试会再次启动。
            if native.is_none(){self.ly.native_rx=None;}
        }
        self.ly.pending=Some(LyricsJob{source,title,artist,duration:dur,song_id,cached,native_only,manual});
    }

    /// 旧请求结束后再发起排队的在线查询：歌曲信息抖动或快速切歌时不会叠加网络线程，
    /// 也只会查询最新的那首；旧请求的结果因通道已更换而直接丢弃。
    pub(crate) fn spawn_lyrics_job(&mut self) {
        if self.ly.pending.is_none() || self.ly_worker.as_ref().is_some_and(|worker|!worker.is_finished()) {return;}
        let Some(lease)=crate::support::resources::LYRICS.acquire() else {return;};
        let Some(job)=self.ly.pending.take() else {return;};
        let (tx, rx) = std::sync::mpsc::channel();
        self.ly.rx = Some(rx);
        self.ly_worker=lease.spawn(move || {
            let LyricsJob{source,title,artist,duration,song_id,cached,native_only,manual}=job;
            let result=if native_only {
                lyrics::kugou_cache::read(&title,&artist,duration).map(|lyrics|(lyrics,false)).ok_or(lyrics::FetchError::NotFound)
            }else{lyrics::fetch_for_display_with(&source, &title, &artist, duration, song_id, cached, manual)};
            let _ = tx.send(result);
        });
        if self.ly_worker.is_none(){self.ly.rx=None;self.ly.loading=false;self.ly.key.clear();}
    }

    /// 外部直接提供歌词（录制演示用）。
    pub fn set_lyrics(&mut self, title: &str, artist: &str, duration: f64, l: lyrics::Lyrics) {
        self.ly = LyricsState { key: format!("{title}|{artist}|{duration:.0}"), lyrics: Some(l), ..Default::default() };
    }
}

/// 与 `format!("{title}|{artist}|{duration:.0}")` 等价，但不分配内存：界面每帧要比较好几次。
pub(crate) fn lyric_key_matches(key:&str,media:&NowPlaying)->bool {
    use std::io::Write;
    let Some(rest)=key.strip_prefix(media.title.as_str()).and_then(|rest|rest.strip_prefix('|')) else{return false;};
    let Some(rest)=rest.strip_prefix(media.artist.as_str()).and_then(|rest|rest.strip_prefix('|')) else{return false;};
    let mut buffer=[0u8;32];
    let mut cursor=std::io::Cursor::new(&mut buffer[..]);
    if write!(cursor,"{:.0}",media.duration).is_err(){return rest==format!("{:.0}",media.duration);}
    let written=cursor.position() as usize;
    rest.as_bytes()==&buffer[..written]
}

#[test]
fn lyric_key_matches_the_formatted_key_without_allocating(){
    for (title,artist,duration) in [("歌","手",120.4),("A|B","C",0.0),("歌","",239.5),("歌","手",240.5),("","",f64::NAN),("t","a",1e30)] {
        let media=NowPlaying{title:title.into(),artist:artist.into(),duration,..Default::default()};
        let key=format!("{}|{}|{:.0}",title,artist,duration);
        assert!(lyric_key_matches(&key,&media),"{key}");
        assert!(!lyric_key_matches(&format!("{key}0"),&media));
        assert!(!lyric_key_matches(&format!("x{key}"),&media));
    }
}
