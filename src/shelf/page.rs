//! 置物架页：文件卡片、拖入遮罩、清空确认与右键菜单。

use crate::island::{App, Cmd, Env};
use crate::visuals::anim::{lerp, smoothstep, Spring};
use crate::visuals::paint::{self, capsule, Card, H};
use crate::visuals::theme::{self, size, W};
use eframe::egui::{self, Color32, Pos2, Rect};

impl App {
    pub(crate) fn page_shelf(&mut self, c: &Card, _env: &Env, cmds: &mut Vec<Cmd>) {
        let n = self.shelf.items.len();
        let drag = self.drag_a;
        c.text(320.0,23.0,"添加",size::SMALL,W::Regular,theme::GRAY,H::Right);
        if c.hit(287.0,8.0,38.0,22.0,"shelf-add").on_hover_text("从电脑选择文件").clicked(){cmds.push(Cmd::AddFiles);}
        if n>5 {c.text(209.5,121.0,&format!("{}–{} / {n} · 滚轮查看",self.shelf_first+1,(self.shelf_first+5).min(n)),10.5,W::Regular,theme::DIM,H::Center);}

        if n > 0 {
            // 「添加」左边的「清空」：第一次点变红，3 秒内再点才真正清空。
            // 「确认」与「清空」同宽，不会挤到顶栏时间。
            let confirming = self.confirm_clear.is_some();
            let label = if confirming { "确认" } else { "清空" };
            let lw = c.text_w("清空", size::SMALL, W::Regular) + 16.0;
            let base = if confirming { Color32::from_rgb(120, 28, 36) } else { theme::BTN };
            let hover = if confirming { Color32::from_rgb(150, 34, 44) } else { theme::BTN_HOVER };
            // 放在右下角：顶栏中间是日期时间，宽度随 12 小时制等设置变化。
            let (x, y) = (377.0 - lw, 109.0);
            let (r, bc) = capsule(&c.faded(1.0 - drag), x, y, lw, 17.0, "shelf-clear", base, hover);
            let r = if confirming { r.on_hover_text("再点一次清空置物架（文件本身不动）") } else { r.on_hover_text("清空置物架") };
            bc.text(x + lw / 2.0, y + 12.0, label, size::SMALL, W::Regular, theme::WHITE, H::Center);
            if r.clicked() {
                if confirming {
                    self.shelf.clear();
                    self.icons.clear();
                    self.shelf_born.clear();
                    self.shelf_x.clear();
                    self.shelf_first = 0;
                    self.confirm_clear = None;
                    self.say("置物架已清空（文件本身没动）");
                } else {
                    self.confirm_clear = Some(self.t);
                }
            }
        }

        let body = c.faded(lerp(1.0, 0.14, drag));
        if n == 0 {
            let a = 1.0 - drag;
            if a > 0.0 {
                let ec = body.faded(a);
                ec.dashed(42.0, 32.0, 335.0, 91.0, 12.0, 1.2, Color32::from_gray(70), 0.0);
                paint::tray(&ec, 209.5, 60.0, 28.0, 0.0, Color32::from_gray(200), Color32::from_gray(80));
                ec.text(209.5, 93.0, "把文件拖到这儿", 12.0, W::Semibold, theme::WHITE, H::Center);
                ec.text(209.5, 108.0, "只记位置，不复制文件 · 点一下打开", size::TINY, W::Regular, theme::GRAY, H::Center);
            }
        } else {
            self.shelf_row(&body, cmds);
        }
        if drag > 0.002 {
            self.drag_overlay(&c.faded(drag));
        }
    }

    fn shelf_row(&mut self, c: &Card, cmds: &mut Vec<Cmd>) {
        let n = self.shelf.items.len();
        let slot = 68.0;
        let center = 209.5;
        let visible = n.min(5) as f32;
        let scroll = self.shelf_scroll.pos;
        let row = c.clipped(36.0, 28.0, 347.0, 100.0);
        let dt = self.dt;
        let items = self.shelf.items.clone();
        self.icons.retain(&items);
        self.shelf_x.retain(|p, _| items.contains(p));
        for (i, p) in items.iter().enumerate() {
            let target = center + (i as f32 - scroll - (visible - 1.0) / 2.0) * slot;
            let born = self.shelf_born.get(p).copied();
            let s = self.shelf_x.entry(p.clone()).or_insert_with(|| {
                // 新放入的从右侧滑进来；启动时已有的直接就位
                let start = if born.is_some() { target + slot * 0.9 } else { target };
                Spring::new(start, 230.0, 0.9)
            });
            s.target = target;
            s.step(dt);
            let x = s.pos;
            if x < 0.0 || x > 420.0 {
                continue;
            }
            let appear = born.map(|b| smoothstep(0.0, 0.35, (self.t - b) as f32)).unwrap_or(1.0);
            let edge = smoothstep(26.0, 60.0, x) * smoothstep(392.0, 358.0, x);
            let a = appear * edge;
            if a <= 0.01 {
                continue;
            }
            let r = row.hit_drag(x - 31.0, 30.0, 62.0, 78.0, ("shelf", p));
            let show_hint=self.shelf_menu_rect.is_none() && !r.context_menu_opened() && !r.secondary_clicked();
            if show_hint {r.clone().on_hover_text(p.display().to_string());}
            let (h, pr) = row.hover_t(&r);
            let cc = row.faded(a).scaled(x, 66.0, 1.0 - 0.04 * pr);
            if h > 0.01 {
                cc.fill(x - 31.0, 30.0, 62.0, 78.0, 10.0, Color32::from_white_alpha((16.0 * h) as u8));
            }
            let lift = -2.0 * h + (1.0 - appear) * 8.0;
            self.file_icon_at(&cc, p, x, 52.0 + lift, 36.0);
            let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let name = cc.fit(&name, size::SMALL, W::Semibold, 60.0);
            cc.text(x, 86.0, &name, size::SMALL, W::Semibold, theme::WHITE, H::Center);
            let meta=self.icons.get(c.ui.ctx(),p).map(|preview|preview.metadata).unwrap_or_default();
            cc.text(x, 99.0, &meta, size::TINY, W::Regular, theme::GRAY, H::Center);
            if r.drag_started() && p.exists() {cmds.push(Cmd::DragFile(p.clone()));}
            let remove=row.hit(x+15.0,30.0,17.0,18.0,("shelf-remove",p));
            let remove_card=row.faded(a);
            remove_card.circle(x+23.5,39.0,7.5,Color32::from_rgb(55,40,46));
            paint::close(&remove_card,x+23.5,39.0,2.4,theme::WHITE);
            let mut delete=remove.clicked();
            if show_hint {remove.on_hover_text("从置物架移除，保留原文件");}
            r.context_menu(|ui|{
                if ui.button("打开").clicked(){if p.exists(){cmds.push(Cmd::Open(p.clone()));}ui.close_menu();}
                if ui.button("在文件夹中显示").clicked(){cmds.push(Cmd::Reveal(p.clone()));ui.close_menu();}
                if ui.button("复制完整路径").clicked(){cmds.push(Cmd::Copy(p.display().to_string()));ui.close_menu();}
                if ui.button("从置物架移除").clicked(){delete=true;ui.close_menu();}
                self.shelf_menu_rect=Some(egui::Frame::menu(ui.style()).widget_rect(ui.min_rect()));
            });
            if delete { self.shelf.remove(p); self.icons.remove(p); self.shelf_x.remove(p); self.shelf_born.remove(p); self.shelf_first=self.shelf_first.min(self.shelf.items.len().saturating_sub(5)); }
            else if r.clicked() { if p.exists(){cmds.push(Cmd::Open(p.clone()));} else {self.say("文件已移动或删除，可从置物架移除");} }
        }
    }

    fn file_icon_at(&mut self, c: &Card, p: &std::path::Path, cx: f32, cy: f32, sz: f32) {
        let tex=self.icons.get(c.ui.ctx(),p).and_then(|preview|preview.image);
        if let Some(tex) = tex {
            let s = tex.size_vec2();
            let k = sz / s.x.max(s.y);
            let r = Rect::from_center_size(c.pos(cx, cy), s * k * c.s);
            c.painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        } else {
            paint::file_glyph(c, paint::file_kind(p), cx, cy, sz);
        }
    }

    /// 拖入遮罩：流动虚线框 + 跟随光标的柔光 + 上下轻跳的托盘图标。
    fn drag_overlay(&self, c: &Card) {
        let (x, y, w, h) = (42.0, 31.0, 335.0, 93.0);
        c.fill(x, y, w, h, 12.0, Color32::from_black_alpha(110));
        if let Some(p) = self.cursor_local {
            let px = p.x.clamp(x + 20.0, x + w - 20.0);
            let py = p.y.clamp(y + 10.0, y + h - 10.0);
            c.clipped(x, y, w, h).glow(px, py, 90.0, Color32::from_rgba_unmultiplied(120, 170, 255, 34));
        }
        c.dashed(x, y, w, h, 12.0, 1.4, Color32::from_white_alpha(230), self.t as f32 * 14.0);
        let bob = ((self.t * 5.0).sin() as f32 * 0.5 + 0.5) * 3.0;
        paint::tray(c, 209.5, 64.0, 30.0, bob, theme::WHITE, Color32::from_gray(90));
        c.text(209.5, 98.0, "松手放进置物架", 12.0, W::Semibold, theme::WHITE, H::Center);
    }
}
