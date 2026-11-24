//! 灵动岛主体：状态、悬停形变、切页、顶栏与公共层。与平台无关——
//! Windows 外壳（`main.rs`）和无窗口录制器（`daozai-record`）共用同一份代码。
//!
//! 时间一律来自 `Env::t`（秒），不直接读系统时钟，所以录屏可逐帧复现。

use crate::island::banner::Alert;
use crate::island::extras::PetState;
use crate::visuals::anim::{self, lerp, smoothstep, Spring};
use crate::island::layout::bar::{self as bar, Layout};
use crate::calendar;
use crate::visuals::glass;
use crate::music::lyrics;
use crate::mail;
use crate::model;
pub use crate::model::{NotificationSound, BAR_H, CANVAS_W, PH, PW, WIN_H, WIN_W};
pub use crate::visuals::glass_shape::{blur_pieces, BlurPiece, GlassShape};
use crate::visuals::paint::{self, Card, H};
use crate::visuals::shape;
use crate::music::{NowPlaying, Transport};
use crate::visuals::theme::{self, size, W};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;

mod frame;
pub(crate) mod app;
pub(crate) mod nav;
pub mod layout;
pub(crate) mod settings;
pub(crate) mod pages;
pub(crate) mod banner;
pub(crate) mod extras;
pub(crate) mod event_tray;
mod resident_settings;
pub(crate) mod avoidance;
pub(crate) mod dnd;
pub(crate) mod page_badges;
pub(crate) mod settings_window;
pub(crate) mod pet;
pub(crate) mod pet_expressions;
pub mod settings_theme;
pub(crate) mod updates;

// ---------- 几何（逻辑点；原版 125% 截图 ÷ 1.25） ----------
/// 展开后保留的常驻状态行高度；收起时它就是整颗药丸。
pub(crate) const STATUS_H: f32 = BAR_H;
/// 页面内容向上压过状态行下缘的量；状态行底部是空白，页头可以更靠近它。
pub(crate) const PAGE_RAISE: f32 = 6.0;
pub const MUSIC_PAGE: usize = 0;
pub const FOCUS_PAGE: usize = 1;
pub const WATER_PAGE: usize = 2;
/// 久坐提醒的标题（通知按钮据此区分喝水提醒）。
pub const SIT_TITLE: &str = "该起来活动了";

pub const SHELF_PAGE: usize = 3;
pub const TODO_PAGE: usize = 4;
pub const MAIL_PAGE: usize = 5;
pub const QUOTA_PAGE: usize = 6;
pub const CALENDAR_PAGE: usize = 7;
pub const WEATHER_PAGE: usize = 8;
pub const AI_PAGE: usize = 9;
/// Windows 通知：横幅结束后保留铃铛，点击打开这一页。
pub const WINDOWS_PAGE: usize = 10;
/// 剪贴板：最近复制的文字，独立于置物架。
pub const CLIPBOARD_PAGE: usize = 11;
/// Windows 通知页一次显示的条数，超出后用滚轮翻看。
pub(crate) const WINDOWS_ROWS: usize = 3;
pub(crate) const WEATHER_H: f32 = 232.0;
pub(crate) const CALENDAR_H: f32 = 216.0;
pub const BAR_W: f32 = 163.0; // 收起药丸宽
pub(crate) const HUD_W: f32 = 260.0;
pub(crate) const LYRIC_BAR_W: f32 = 300.0;
pub(crate) const HUD_TRACK_X: f32 = 48.0;
pub(crate) const HUD_TRACK_W: f32 = 150.0;
pub const BAR_W_MIN: f32 = 44.0; // 极简小条
pub const BAR_H_MIN: f32 = 4.5;
pub const FLARE: f32 = 17.6; // 顶部外翻肩宽
pub const FLARE_H: f32 = 16.0; // 外翻收敛高度
pub const RAD: f32 = 33.0; // 底部圆角
/// 胶囊皮肤与屏幕顶边的间隙。
pub const PILL_GAP: f32 = 6.0;

/// 胶囊的柔和投影：四周都有，多层叠加近似高斯。
/// 圆角矩形一圈轮廓点（顺时针、首尾闭合），用于沿胶囊外形逐段着色。
fn round_rect_points(r:Rect,radius:f32)->Vec<Pos2>{
    let radius=radius.min(r.width().min(r.height())*0.5).max(0.0);
    let corners=[
        (Pos2::new(r.right()-radius,r.top()+radius),-90.0f32),
        (Pos2::new(r.right()-radius,r.bottom()-radius),0.0),
        (Pos2::new(r.left()+radius,r.bottom()-radius),90.0),
        (Pos2::new(r.left()+radius,r.top()+radius),180.0),
    ];
    let mut points=Vec::with_capacity(4*9+1);
    for (c,a0) in corners {
        for i in 0..=8 {let a=(a0+90.0*i as f32/8.0).to_radians();points.push(c+Vec2::new(a.cos(),a.sin())*radius);}
    }
    points.push(points[0]);
    points
}

fn pill_shadow(painter:&egui::Painter,rect:Rect,radius:f32,strength:f32){
    if strength<=0.01 {return;}
    for (grow,a) in [(10.0f32,0.035f32),(6.5,0.05),(3.5,0.07),(1.5,0.09)] {
        let r=Rect::from_min_max(rect.min-Vec2::new(grow*0.6,grow*0.3),rect.max+Vec2::new(grow*0.6,grow));
        painter.rect_filled(r,radius+grow*0.8,Color32::from_black_alpha((255.0*a*strength) as u8));
    }
}
pub(crate) const PAGE_NAMES: [&str; 12] = ["音乐", "专注", "健康", "置物架", "待办", "邮箱", "AI 额度", "日程", "天气", "AI 会话", "Windows 通知", "剪贴板"];
pub(crate) const PAGE_SETTINGS_TABS: [usize; 12] = [1,2,3,5,4,7,11,8,12,15,18,17];

// ---------- 动效参数 ----------
/// 悬停意图缓冲：光标停留这么久才真正展开（期间药丸轻微胀大作为反馈），防误触。
const HOVER_INTENT: f64 = 0.20;
const MINIMAL_HOVER_INTENT: f64 = 0.40;
/// 移开后等待多久开始收起；这段时间内回来不会收。
const LEAVE_DELAY: f64 = 0.12;
const PAGE_EDGE_OVERLAP: f32 = 8.0;
const SWELL: (f32, f32) = (1.075, 1.13);
// 展开：约 0.56 s 到峰值、约 0.8% 过冲；高度略软于宽度，像「垂落」下来。
const OPEN_W: (f32, f32) = (105.0, 0.84);
const OPEN_H: (f32, f32) = (100.0, 0.86);
// 收起：临界阻尼，约 0.38 s 干净收回，无回弹。
const CLOSE: (f32, f32) = (240.0, 1.0);
// 横幅宽度只跟随横幅开合，保持原来的收放速度。
const NOTICE_CLOSE: (f32, f32) = (620.0, 1.0);
const SWELL_K: (f32, f32) = (420.0, 0.8);
const PAGE_SLIDE: f32 = 46.0;

pub use crate::instant_messaging::{Messenger,Unread};

pub use crate::windows_toasts::WindowsNotice;

/// 展开后未读胶囊的宽度、离面板的间距和每个图标占的高度。
pub(crate) const UNREAD_W: f32 = 40.0;
pub(crate) const UNREAD_GAP: f32 = 8.0;
pub(crate) const UNREAD_SLOT: f32 = 36.0;
/// 收起时两个来源图标的间距：并排各占一格，不叠放。
fn unread_step(_horizontal: bool) -> f32 { 22.0 }

#[cfg(test)]
mod blur_piece_tests {
    use super::*;

    fn covered(pieces: &[BlurPiece], x: f32, y: f32) -> bool {
        pieces.iter().any(|p| {
            if !(x >= p.x && x < p.x + p.w && y >= p.y && y < p.y + p.h) { return false; }
            // 圆角：只判断四角的圆弧外侧
            let r = p.radius;
            let cx = x.clamp(p.x + r, p.x + p.w - r);
            let cy = y.clamp(p.y + r, p.y + p.h - r);
            (x - cx).powi(2) + (y - cy).powi(2) <= r * r + 1e-3
        })
    }

    #[test]
    fn blur_follows_the_island_outline_not_its_bounding_box() {
        for ppp in [1.0, 1.25, 1.5, 2.0] {
            let rect = Rect::from_min_size(Pos2::new(110.0, 0.0), Vec2::new(419.0, 140.0));
            let shape = GlassShape::Notch { rect, flare: 17.6, flare_h: 16.0, radius: 33.0 };
            let pieces = blur_pieces(std::slice::from_ref(&shape), ppp);
            let outline = shape.outline();
            // 轮廓内部的点都被模糊覆盖，外部（含底部圆角外、肩部外）都不覆盖；边缘留 1.5 像素容差。
            let inside = |x: f32, y: f32| {
                let mut c = false;
                let mut prev = *outline.last().unwrap();
                for &next in &outline {
                    if (next.y > y) != (prev.y > y) && x < (prev.x - next.x) * (y - next.y) / (prev.y - next.y) + next.x { c = !c; }
                    prev = next;
                }
                c
            };
            let mut checked = 0;
            for yi in 0..(150.0 * ppp) as i32 {
                for xi in ((80.0 * ppp) as i32..(560.0 * ppp) as i32).step_by(3) {
                    let (x, y) = (xi as f32 + 0.5, yi as f32 + 0.5);
                    let (lx, ly) = (x / ppp, y / ppp);
                    let deep_in = inside(lx, ly) && [(-1.5, 0.0), (1.5, 0.0), (0.0, 1.5), (0.0, -1.5)].iter().all(|(dx, dy)| inside(lx + dx / ppp, (ly + dy / ppp).max(0.01)));
                    let far_out = !inside(lx, ly) && [(-1.5, 0.0), (1.5, 0.0), (0.0, 1.5), (0.0, -1.5)].iter().all(|(dx, dy)| !inside(lx + dx / ppp, ly + dy / ppp));
                    if deep_in { assert!(covered(&pieces, x, y), "{ppp} 缩放下岛体内 ({x},{y}) 没有模糊"); checked += 1; }
                    if far_out && y >= 0.0 { assert!(!covered(&pieces, x, y), "{ppp} 缩放下岛体外 ({x},{y}) 不能出现模糊"); }
                }
            }
            assert!(checked > 1000);
            assert!(pieces.len() < 60, "肩部合并后块数应很少：{}", pieces.len());
        }
    }

    #[test]
    fn side_docked_blur_follows_the_rotated_outline() {
        for dock in [model::Dock::Left, model::Dock::Right] {
            for ppp in [1.0, 1.5] {
                let x = if dock == model::Dock::Left { 0.0 } else { 81.0 };
                let shape = GlassShape::Docked { rect: Rect::from_min_size(Pos2::new(x, 200.0), Vec2::new(419.0, 140.0)), dock, flare: 17.6, flare_h: 16.0, radius: 33.0 };
                let pieces = blur_pieces(std::slice::from_ref(&shape), ppp);
                let outline = shape.outline();
                let inside = |x: f32, y: f32| {
                    let mut c = false;
                    let mut prev = *outline.last().unwrap();
                    for &next in &outline {
                        if (next.y > y) != (prev.y > y) && x < (prev.x - next.x) * (y - next.y) / (prev.y - next.y) + next.x { c = !c; }
                        prev = next;
                    }
                    c
                };
                let mut checked = 0;
                for yi in ((170.0 * ppp) as i32..(370.0 * ppp) as i32).step_by(2) {
                    for xi in (0..(500.0 * ppp) as i32).step_by(3) {
                        let (px, py) = (xi as f32 + 0.5, yi as f32 + 0.5);
                        let (lx, ly) = (px / ppp, py / ppp);
                        let near = [(-1.5, 0.0), (1.5, 0.0), (0.0, 1.5), (0.0, -1.5)];
                        let deep_in = inside(lx, ly) && near.iter().all(|(dx, dy)| inside((lx + dx / ppp).clamp(0.01, 499.99), ly + dy / ppp));
                        let far_out = !inside(lx, ly) && near.iter().all(|(dx, dy)| !inside(lx + dx / ppp, ly + dy / ppp));
                        if deep_in { assert!(covered(&pieces, px, py), "{dock:?} {ppp}: 岛体内 ({px},{py}) 没有模糊"); checked += 1; }
                        let on_canvas = lx > 0.5 && lx < 499.5;
                        if far_out && on_canvas { assert!(!covered(&pieces, px, py), "{dock:?} {ppp}: 岛体外 ({px},{py}) 不能有模糊"); }
                    }
                }
                assert!(checked > 1000);
            }
        }
    }

    #[test]
    fn collapsing_pill_blur_radius_fits_the_physical_bounds() {
        for ppp in [1.0,1.25,1.5,2.0] {
            for height in [30.02,30.3,30.49,30.5,30.8,31.0,31.2,32.1,35.3] {
                let rect=Rect::from_min_size(Pos2::new(230.23,6.0),Vec2::new(179.31,height));
                let radius=15.0+(height-30.0)*0.1;
                let pieces=blur_pieces(&[GlassShape::Round(rect,radius)],ppp);
                assert_eq!(pieces.len(),1);
                let p=pieces[0];
                assert!(p.radius*2.0<=p.w.min(p.h),"收起时圆弧不能超过物理边界：ppp={ppp}, height={height}, piece={p:?}");
                assert_eq!(p.x+p.w,(rect.right()*ppp).round(),"模糊与岛体右边界应使用同一像素位置");
                assert_eq!(p.y+p.h,(rect.bottom()*ppp).round(),"模糊与岛体下边界应使用同一像素位置");
            }
        }
    }

    #[test]
    fn rounded_rectangles_and_empty_shapes() {
        let pieces = blur_pieces(&[GlassShape::Round(Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(100.0, 40.0)), 10.0)], 1.5);
        assert_eq!(pieces, vec![BlurPiece { x: 15.0, y: 30.0, w: 150.0, h: 60.0, radius: 15.0 }]);
        assert!(blur_pieces(&[GlassShape::Notch { rect: Rect::NOTHING, flare: 5.0, flare_h: 8.0, radius: 12.0 }], 1.0).is_empty());
        // 极简细条：没有肩部，只有一块
        let tiny = blur_pieces(&[GlassShape::Notch { rect: Rect::from_min_size(Pos2::new(298.0, 0.0), Vec2::new(44.0, 4.5)), flare: 0.0, flare_h: 8.0, radius: 2.25 }], 1.25);
        assert_eq!(tiny.len(), 1);
    }
}

/// 与 `format!("{title}|{artist}|{duration:.0}")` 等价，但不分配内存：界面每帧要比较好几次。
fn lyric_key_matches(key:&str,media:&NowPlaying)->bool {
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

#[derive(Clone, Copy)]
pub(crate) struct PositionDrag {
    start: Vec2,
    /// 本段拖动的起始偏移（未取整，换边后重置），单位与 `island_offset` 相同。
    offset: f32,
    original: (model::Dock, i32),
}

// ---------- 拖动位置动效 ----------
/// 原地按住这么久即「拿起」岛体，不必先移动鼠标。
const LIFT_HOLD: f64 = 0.12;
/// 拿起 / 放下：略欠阻尼，提起时轻轻弹一下，放下时回落。
const LIFT_K: (f32, f32) = (420.0, 0.38);
/// 拿起后放大：沿停靠边方向 / 离开停靠边方向。
const LIFT_SCALE: (f32, f32) = (0.08, 0.14);
/// 窗口跟手：明显拖尾并略微冲过光标；松手后落位回弹一两下。
const GLIDE_FOLLOW: (f32, f32) = (320.0, 0.72);
const GLIDE_SETTLE: (f32, f32) = (380.0, 0.38);
/// 移动越快沿运动方向拉得越长（最多 22%），厚度方向相应收窄。
const STRETCH_MAX: f32 = 0.22;
const STRETCH_SPEED: f32 = 1400.0;
/// 吸附范围（`island_offset` 单位）：居中和拖动前的位置。
const DRAG_MAGNET: f32 = 5.0;
/// 吸住居中后要拖出更远才松开，停在中间时稳稳「咬住」，不会被手抖带走。
const CENTER_RELEASE: f32 = 9.0;
/// 离居中多近开始浮现轮廓描边（`island_offset` 单位）。
const CENTER_GUIDE_RANGE: f32 = 24.0;
/// 吸附瞬间沿轮廓从中心向两端扫开的光，持续时长（秒）。
const CENTER_SWEEP: f64 = 0.55;
/// 已居中的提示色（绿）：岛体描边、外发光和扫光统一用它，颜色本身就是「对准了」。
const CENTER_TINT: (f32, f32, f32) = (88.0, 222.0, 140.0);

pub use app::{App, Cmd, Env, FrameOut, Options};
#[cfg(test)]
pub(crate) use app::{CalendarState, LyricSong, LyricsState};
pub(crate) use app::Phase;
