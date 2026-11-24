//! 剪贴板页：记录开关、列表、清空确认与复制/固定/删除。

use crate::island::{App, Cmd, Env};
use crate::visuals::paint::{self, capsule, Card, H};
use crate::visuals::theme::{self, size, W};
use eframe::egui::Color32;

/// 剪贴板分栏一屏显示的条数。
pub(crate) const CLIP_ROWS: usize = 4;

impl App {
    // ================= 剪贴板 =================
    pub(crate) fn page_clipboard(&mut self, c: &Card, _env: &Env, _cmds: &mut Vec<Cmd>) {
        if !self.settings.clipboard_history {
            c.dashed(42.0, 32.0, 335.0, 91.0, 12.0, 1.2, Color32::from_gray(70), 0.0);
            c.text(209.5, 66.0, "剪贴板记录已关闭", 12.0, W::Semibold, theme::WHITE, H::Center);
            c.text(209.5, 82.0, "开启后，最近复制的文字会出现在这里", size::TINY, W::Regular, theme::GRAY, H::Center);
            let (r, bc) = capsule(c, 179.5, 92.0, 60.0, 20.0, "clip-enable", theme::BTN, theme::BTN_HOVER);
            bc.centered_text(209.5, 102.0, "开启", size::SMALL, W::Semibold, theme::WHITE, H::Center);
            if r.clicked() { self.settings.clipboard_history = true; self.clipboard.set_enabled(true); self.settings.save(); }
            return;
        }
        let items = self.clipboard.lock().items.clone();
        let n = items.len();
        let recent = items.iter().filter(|clip| !clip.pinned).count();
        self.clip_first = self.clip_first.min(n.saturating_sub(CLIP_ROWS));

        // 右上：条数与「清空」（只清未固定的）。
        if recent > 0 {
            let confirming = self.clip_confirm_clear.is_some_and(|at| self.t - at < 3.0);
            if !confirming { self.clip_confirm_clear = None; }
            let label = if confirming { "确认" } else { "清空" };
            let lw = c.text_w("清空", size::SMALL, W::Regular) + 16.0;
            let base = if confirming { Color32::from_rgb(120, 28, 36) } else { theme::BTN };
            let hover = if confirming { Color32::from_rgb(150, 34, 44) } else { theme::BTN_HOVER };
            let x = 351.0 - lw;
            let (r, bc) = capsule(c, x, 11.5, lw, 17.0, "clip-clear", base, hover);
            bc.text(x + lw / 2.0, 23.5, label, size::SMALL, W::Regular, theme::WHITE, H::Center);
            let r = r.on_hover_text(if confirming { "再点一次清空未固定的记录" } else { "清空未固定的记录" });
            if r.clicked() {
                if confirming {
                    self.clipboard.lock().clear_recent();
                    self.clip_first = 0;
                    self.clip_confirm_clear = None;
                    self.say("剪贴板记录已清空（固定的保留）");
                } else {
                    self.clip_confirm_clear = Some(self.t);
                }
            }
        }

        if n == 0 {
            c.dashed(42.0, 32.0, 335.0, 91.0, 12.0, 1.2, Color32::from_gray(70), 0.0);
            paint::clipboard_glyph(c, 209.5, 58.0, 22.0, Color32::from_gray(200), Color32::from_gray(80));
            c.text(209.5, 93.0, "复制的文字会出现在这里", 12.0, W::Semibold, theme::WHITE, H::Center);
            c.text(209.5, 108.0, "记录保存在本机，重新打开仍可查看", size::TINY, W::Regular, theme::GRAY, H::Center);
            return;
        }

        c.fill(42.0, 31.0, 335.0, 93.0, 12.0, theme::CARD);
        // 超过一屏时右侧细滚动条，滚轮查看。
        if n > CLIP_ROWS {
            let (top, track) = (38.0, 79.0);
            let thumb = (track * CLIP_ROWS as f32 / n as f32).max(12.0);
            let y = top + (track - thumb) * self.clip_first as f32 / (n - CLIP_ROWS) as f32;
            c.fill(371.0, top, 2.5, track, 1.25, Color32::from_white_alpha(14));
            c.fill(371.0, y, 2.5, thumb, 1.25, Color32::from_white_alpha(70));
        }
        let mut action: Option<(usize, u8)> = None; // 0 复制 1 固定 2 删除
        for (row, (index, clip)) in items.iter().enumerate().skip(self.clip_first).take(CLIP_ROWS).enumerate() {
            let y = 44.0 + row as f32 * 22.5;
            let r = c.hit(48.0, y - 10.0, 323.0, 20.0, ("clip", index, clip.text.len(), clip.at.as_str()));
            let (h, _) = c.hover_t(&r);
            if h > 0.01 {
                c.fill(48.0, y - 10.0, 323.0, 20.0, 7.0, Color32::from_white_alpha((12.0 * h) as u8));
            }
            if clip.pinned {
                paint::pin(c, 58.0, y, 9.0, theme::BLUE);
            } else {
                c.circle(58.0, y, 2.2, Color32::from_gray(110));
            }
            let text = c.fit(&crate::clipboard::preview(&clip.text), size::BODY, W::Regular, 210.0);
            c.text(68.0, y + 3.8, &text, size::BODY, W::Regular, Color32::from_gray(235), H::Left);
            c.text(318.0, y + 3.5, &clip.at, size::TINY, W::Regular, theme::DIM, H::Right);
            let tip: String = clip.text.chars().take(400).collect();
            let r = r.on_hover_text(format!("{tip}{}\n点击复制", if clip.text.chars().count() > 400 { "…" } else { "" }));
            let pin = c.hit(323.0, y - 9.0, 18.0, 18.0, ("clip-pin", index)).on_hover_text(if clip.pinned { "取消固定" } else { "固定（清空时保留）" });
            paint::pin(c, 332.0, y, 9.0, if clip.pinned || pin.hovered() { theme::BLUE } else { theme::GRAY });
            let delete = c.hit(345.0, y - 9.0, 17.0, 18.0, ("clip-delete", index)).on_hover_text("删除这条记录");
            paint::close(c, 353.5, y, 2.4, theme::GRAY);
            if delete.clicked() { action = Some((index, 2)); }
            else if pin.clicked() { action = Some((index, 1)); }
            else if r.clicked() { action = Some((index, 0)); }
        }
        match action {
            Some((index, 0)) => {
                #[cfg(windows)]
                self.say(if crate::clipboard::copy_history(&items[index].text) { "已复制" } else { "复制失败，请重试" });
                #[cfg(not(windows))]
                _cmds.push(Cmd::Copy(items[index].text.clone()));
            }
            Some((index, 1)) => {
                if !self.clipboard.lock().toggle_pin(index) { self.say(format!("最多固定 {} 条", crate::clipboard::MAX_PINNED)); }
            }
            Some((index, _)) => { self.clipboard.lock().remove(index); }
            None => {}
        }
    }
}
