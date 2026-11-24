//! 专注页：番茄钟圆环、阶段切换与计时控制。

use crate::island::{App, Env, Phase};
use crate::visuals::paint::{self, capsule, mix, Card, H};
use crate::visuals::theme::{self, size, W};
use eframe::egui::{self, Color32};
use std::f32::consts::{FRAC_PI_2, TAU};

impl App {
    // ================= 专注（番茄钟） =================
    pub(crate) fn page_focus(&mut self, c: &Card, _env: &Env) {
        if c.hit(90.0,8.0,42.0,21.0,"focus-settings").on_hover_text("专注设置").clicked(){self.open_settings_category(2);}
        c.text(111.0,23.0,"设置",10.5,W::Regular,theme::GRAY,H::Center);
        let (rx, ry, rr) = (79.0, 74.0, 32.0);
        let frac = self.ring_shown.pos.clamp(0.0, 1.0);
        let run = c.ui.ctx().animate_bool_with_time(egui::Id::new("pomo-run"), self.pomo.running, 0.3);
        let resting=self.pomo.phase!=Phase::Focus;
        let (top,bottom)=if resting{(theme::SOFT_GREEN,mix(theme::SOFT_GREEN,Color32::BLACK,0.2))}else{(theme::RED_TOP,theme::RED)};
        // 外圈柔光；计时中缓慢呼吸
        let breathe = 0.5 + 0.5 * ((self.t * 2.2).sin() as f32);
        let glow = 0.30 + 0.25 * run * breathe;
        c.glow(rx, ry, 48.0, paint::alpha(bottom, glow * 0.55));
        c.ring(rx, ry, rr, 7.0, if resting{mix(theme::CARD,top,0.18)}else{Color32::from_rgb(58,22,26)});
        // 渐变进度环随当前阶段配色。
        if frac > 0.001 {
            let steps = (120.0 * frac).ceil().max(2.0) as usize;
            let a0 = -FRAC_PI_2;
            let mut prev = (rx + rr * a0.cos(), ry + rr * a0.sin());
            for i in 1..=steps {
                let a = a0 + TAU * frac * i as f32 / steps as f32;
                let next = (rx + rr * a.cos(), ry + rr * a.sin());
                let t = ((prev.1 + next.1) * 0.5 - (ry - rr)) / (2.0 * rr);
                c.seg(prev, next, 7.0, mix(top, bottom, t));
                prev = next;
            }
            // 圆头
            c.circle(rx, ry - rr, 3.5, top);
            c.circle(prev.0, prev.1, 3.5, mix(top, bottom, (prev.1 - (ry - rr)) / (2.0 * rr)));
        }
        let hand = if self.pomo.running { (self.t as f32 * 1.2) % TAU } else { 0.0 };
        if resting {
            let t=if self.pomo.running && crate::island::bar_layout::animations_enabled(){self.t as f32}else{0.0};
            crate::focus::bar::tea(&c.scaled(rx,ry,2.2),rx,ry,frac,t);
        }else{paint::stopwatch(c, rx, ry + 0.5, 15.5, hand * run, theme::WHITE);}

        let secs = self.pomo.remaining.max(0.0).ceil() as u32;
        c.text(264.0, 61.0, &format!("{:02}:{:02}", secs / 60, secs % 60), size::HERO + 1.0, W::Bold, if resting{theme::SOFT_GREEN}else{theme::WHITE}, H::Center);
        c.text(264.0, 79.0, self.pomo.phase.label(), size::SMALL, W::Regular, theme::GRAY, H::Center);

        // 重置：点一下图标转一圈
        let r = c.hit(317.0, 38.0, 24.0, 24.0, "f-reset");
        let (h, p) = c.hover_t(&r);
        let rc = c.scaled(329.0, 50.0, 1.0 - 0.1 * p);
        rc.circle(329.0, 50.0, 11.0, mix(Color32::from_rgb(28, 28, 30), Color32::from_rgb(50, 50, 54), h));
        paint::reset(&rc, 329.0, 50.0, -self.reset_spin.pos, Color32::from_gray(230));
        if r.clicked() {
            self.pomo.remaining = self.phase_secs(self.pomo.phase);
            self.pomo.running = false;
            self.reset_spin.target += TAU;
        }

        for (phase,label,x,width,id) in [(Phase::Focus,"专注",148.0,88.0,"f-focus"),(Phase::Rest,"休息",292.0,88.0,"f-rest")] {
            let selected=self.pomo.phase==phase;
            let accent=if phase==Phase::Focus{theme::RED_TOP}else{theme::SOFT_GREEN};
            let background=if selected{mix(theme::BTN,accent,0.22)}else{theme::BTN};
            let (response,button)=capsule(c,x,94.0,width,25.0,id,background,if selected{mix(theme::BTN_HOVER,accent,0.25)}else{theme::BTN_HOVER});
            button.text(x+width/2.0,110.0,label,size::BODY,W::Semibold,if selected{mix(accent,theme::WHITE,0.5)}else{theme::WHITE},H::Center);
            if response.clicked(){self.set_phase(phase);}
        }
        let (b2, c2) = capsule(c, 246.0, 94.0, 36.0, 25.0, "f-start", theme::RED, Color32::from_rgb(255, 82, 98));
        paint::play_pause(&c2, 264.0, 106.5, 11.0, run, theme::WHITE);
        let b2=b2.on_hover_text(if self.pomo.running{"暂停计时"}else{"开始计时"});
        if b2.clicked() {
            if self.pomo.remaining <= 0.0 {
                self.pomo.remaining = self.phase_secs(self.pomo.phase);
            }
            self.pomo.running = !self.pomo.running;
        }
    }

    fn set_phase(&mut self, ph: Phase) {
        if self.pomo.phase==ph {return;}
        self.pomo.phase = ph;
        self.pomo.remaining = self.phase_secs(ph);
        self.pomo.running = false;
    }
}
