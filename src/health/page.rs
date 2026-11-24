//! 健康页：喝水与久坐两种模式、进度环、分时柱状图与提醒时段入口。

use crate::health::HealthReminder;
use crate::island::{App, Env, Phase};
use crate::visuals::anim::{self, lerp, smoothstep};
use crate::visuals::paint::{self, capsule, mix, Card, H};
use crate::visuals::theme::{self, size, W};
use eframe::egui::Color32;

impl App {
    fn next_hint(&self, env: &Env) -> String {
        Self::clock_after(env, self.settings.water_minutes as f64 * 60.0 - (self.t - self.last_reminder))
    }
    /// 从现在起 secs 秒后的时钟读数（HH:MM）。
    fn clock_after(env: &Env, secs: f64) -> String {
        let total = env.hour() as u32 * 60 + env.minute() + secs.max(0.0) as u32 / 60;
        format!("{:02}:{:02}", (total / 60) % 24, total % 60)
    }

    /// 健康页：标题「健康」后接「喝水 | 久坐」分段切换。两种模式是同一套版式——
    /// 大数字、右上开关、状态行、进度条、主按钮 + 设置、右侧进度环都在同一位置，
    /// 只换内容和主色（蓝 / 绿）。设置按钮、分隔线和开关本体不随切换闪动。
    pub(crate) fn page_water(&mut self, c: &Card, env: &Env) {
        let sv = self.sit_view_t.pos.clamp(0.0, 1.0);
        self.health_tabs(c, 15.0, sv);

        // 共用的开关：位置不变，颜色和旋钮图标随模式过渡。
        let sit = self.sit_mode;
        let sw = c.hit(160.0, 35.0, 38.0, 22.0, "w-remind")
            .on_hover_text(match (sit, if sit { self.settings.sit_remind } else { self.water.remind }) {
                (false, true) => "关闭喝水提醒", (false, false) => "打开喝水提醒",
                (true, true) => "关闭久坐提醒", (true, false) => "打开久坐提醒",
            });
        let (sh, _) = c.hover_t(&sw);
        let on = lerp(self.remind_t.pos, self.sit_t.pos, sv);
        let accent = mix(theme::BLUE, theme::SIT_GREEN, sv);
        let track = mix(Color32::from_rgb(58, 58, 62), accent, on.clamp(0.0, 1.0));
        c.fill(162.0, 36.0, 34.0, 20.0, 10.0, mix(track, theme::WHITE, sh * 0.08));
        let kx = lerp(172.0, 186.0, on);
        c.circle(kx, 46.0, 8.0, theme::WHITE);
        let ink = mix(Color32::from_gray(140), accent, on.clamp(0.0, 1.0));
        paint::bell(&c.scaled(kx, 46.0, 0.9), kx, 46.0, ink, 0.0);
        if sw.clicked() {
            if sit {
                self.settings.sit_remind = !self.settings.sit_remind;
                self.settings.save();
                self.confirm_health_reminder(HealthReminder::Sit);
                self.say(if self.settings.sit_remind { "久坐提醒已打开" } else { "久坐提醒已关闭" });
            } else {
                self.water.remind = !self.water.remind;
                self.water.save();
                self.confirm_health_reminder(HealthReminder::Water);
                self.say(if self.water.remind { "喝水提醒已打开" } else { "喝水提醒已关闭" });
            }
        }

        let (s, sc) = capsule(c, 111.0, 98.0, 66.0, 25.0, "w-set", theme::BTN, theme::BTN_HOVER);
        paint::sliders(&sc, 126.0, 110.5, theme::WHITE);
        sc.text(149.0, 114.0, "设置", size::BODY, W::Regular, theme::WHITE, H::Center);
        if s.clicked() { self.open_settings_category(1); }
        c.fill(208.0, 36.0, 1.0, 86.0, 0.5, Color32::from_rgb(30, 30, 33));

        // 内容原地交叉淡化，不滑动；不可见的一侧不接收点击。
        if sv < 0.99 {
            let wc = c.faded(1.0 - sv);
            self.water_body(&if sit { wc.ghost() } else { wc }, env);
        }
        if sv > 0.01 {
            let sc = c.faded(sv);
            self.sit_body(&if sit { sc } else { sc.ghost() }, env);
        }
    }

    /// 标题「健康提醒」（「提醒」随模式变蓝 / 绿）与紧跟其后的分段切换：水滴 = 喝水，椅子 = 久坐。
    /// 胶囊外框与标题字等高（11..27），读起来像标题的一部分。设置子页的标题也用它。
    pub(crate) fn health_tabs(&mut self, c: &Card, x: f32, sv: f32) {
        let accent_now = mix(theme::BLUE, theme::SIT_GREEN, sv);
        let wa = c.text(x, 25.0, "健康", size::TITLE, W::Bold, theme::WHITE, H::Left);
        let wb = c.text(x + wa + 1.5, 25.0, "提醒", size::TITLE, W::Bold, accent_now, H::Left);
        let x0 = x + wa + 1.5 + wb + 7.0;
        let (w, y, h) = (22.0, 11.0, 16.0);
        c.fill(x0, y, w * 2.0 + 3.0, h, h / 2.0, Color32::from_rgb(30, 30, 33));
        let px = lerp(x0 + 1.5, x0 + 1.5 + w, sv);
        let pill = mix(Color32::from_rgb(22, 62, 108), Color32::from_rgb(26, 84, 54), sv);
        c.fill(px, y + 1.5, w, h - 3.0, (h - 3.0) / 2.0, pill);
        for i in 0..2 {
            let ix = x0 + 1.5 + i as f32 * w;
            let r = c.hit(ix, y - 3.0, w, h + 6.0, ("health-tab", i))
                .on_hover_text(if i == 0 { "喝水提醒" } else { "久坐提醒" });
            let (hv, _) = c.hover_t(&r);
            let act = if i == 0 { 1.0 - sv } else { sv };
            let accent = if i == 0 { theme::BLUE } else { theme::SIT_GREEN };
            let icon = mix(mix(Color32::from_gray(110), Color32::from_gray(170), hv), mix(accent, theme::WHITE, 0.25), act);
            if i == 0 {
                paint::droplet(c, ix + w / 2.0, y + h / 2.0, 8.5, icon);
            } else {
                paint::chair(c, ix + w / 2.0, y + h / 2.0, 0.34, icon);
            }
            if r.clicked() { self.sit_mode = i == 1; }
        }
    }

    fn water_body(&mut self, c: &Card, env: &Env) {
        let pct = self.water.percent();
        let liters = self.water_ml / 1000.0;
        let pending=self.health_pending[HealthReminder::Water as usize];
        if pending {
            c.text(44.0,59.0,"待喝水",24.0,W::Bold,theme::BLUE,H::Left);
        }else{
            let ww = c.text(44.0, 61.0, &format!("{liters:.1}"), size::HERO, W::Bold, theme::WHITE, H::Left);
            c.text(44.0 + ww + 5.0, 60.0, &format!("/ {:.1} L", self.water.goal_ml as f32 / 1000.0), 11.0, W::Semibold, theme::GRAY, H::Left);
        }

        // 今天 · 3% · 下次 08:07
        let mut x = 44.0;
        x += c.text(x, 77.0, "今天 · ", size::SMALL, W::Regular, theme::GRAY, H::Left);
        x += c.text(x, 77.0, &format!("{pct:.0}%"), size::SMALL, W::Bold, theme::BLUE, H::Left);
        if pending {
            c.text(x,77.0," · 待确认",size::SMALL,W::Regular,theme::BLUE,H::Left);
        } else if self.water.percent()>=100.0 {
            c.text(x,77.0," · 已达标",size::SMALL,W::Regular,theme::GRAY,H::Left);
        } else if self.water.remind && self.lunch_now(env) {
            c.text(x, 77.0, " · 午休中", size::SMALL, W::Regular, theme::GRAY, H::Left);
        } else if self.water.remind && self.water_hours(env.hour() as u32) {
            c.text(x, 77.0, &format!(" · 下次 {}", self.next_hint(env)), size::SMALL, W::Regular, theme::GRAY, H::Left);
        }
        c.fill(44.0, 84.0, 151.0, 3.5, 1.75, Color32::from_rgb(34, 34, 37));
        let f = self.water_ring.pos.clamp(0.0, 1.0);
        if f > 0.0 {
            c.fill(44.0, 84.0, (151.0 * f).max(3.5), 3.5, 1.75, theme::BLUE);
        }

        let (d, dc) = capsule(c, 43.0, 98.0, 62.0, 25.0, "w-drink", theme::BLUE_DEEP, Color32::from_rgb(22, 92, 165));
        paint::droplet(&dc, 56.0, 110.5, 11.0, Color32::from_rgb(120, 200, 255));
        dc.text(65.0, 114.0, "喝一口", size::BODY, W::Semibold, Color32::from_rgb(220, 236, 255), H::Left);
        if d.clicked() { self.drink_water(); }
        if self.last_drink.is_some() {
            c.text(191.0,114.0,"撤销",10.5,W::Regular,theme::GRAY,H::Center);
            if c.hit(177.0,98.0,28.0,25.0,"water-undo").on_hover_text("撤销最近一次饮水记录").clicked() {
                if let Some((ml,hour))=self.last_drink.take() {
                    self.water.drunk_ml=self.water.drunk_ml.saturating_sub(ml);
                    self.water.hours[hour]=self.water.hours[hour].saturating_sub(ml);
                    self.water.save(); self.say("已撤销这次记录");
                }
            }
        }

        // 右侧：环形 ↔ 柱状图（点右侧区域或右上角两颗豆切换）
        let vt = self.water_view_t.pos;
        let area = c.hit(214.0, 30.0, 166.0, 96.0, "w-view");
        if area.clicked() {
            self.water_view = 1 - self.water_view;
        }
        for i in 0..2 {
            let on = if i == 0 { 1.0 - vt.clamp(0.0, 1.0) } else { vt.clamp(0.0, 1.0) };
            let w = lerp(3.0, 7.0, on);
            let x = 362.0 + i as f32 * 8.0 - w / 2.0 + if i == 0 { -1.0 * on } else { 1.0 * on };
            c.fill(x, 33.5, w, 3.0, 1.5, mix(Color32::from_gray(80), theme::WHITE, on));
        }

        let ring_a = 1.0 - smoothstep(0.0, 0.6, vt);
        if ring_a > 0.0 {
            let k = 1.0 + 0.22 * vt;
            let rc = c.faded(ring_a).scaled(292.0, 78.0, k);
            Self::progress_ring(&rc, 292.0, 78.0, 33.0, 6.0, f, theme::WATER_TRACK, theme::BLUE);
            rc.text(292.0, 86.0, &format!("{pct:.0}"), 26.0, W::Bold, theme::WHITE, H::Center);
            paint::droplet(&rc, 287.0, 97.0, 7.0, theme::CYAN);
            rc.text(292.5, 100.0, "%", 10.5, W::Semibold, Color32::from_gray(217), H::Left);
        }
        let chart_a = smoothstep(0.25, 1.0, vt);
        if chart_a > 0.0 {
            self.water_chart(&c.faded(chart_a), env, vt);
        }
    }

    /// 久坐：与喝水同一版式，但只讲一件事——「还有多久该起来」。
    /// 左边大数字是倒计时，状态行补充已坐多久和几点提醒；右侧环是到下次提醒的进度，环心是椅子，
    /// 提醒后保持待处理，确认或起身后才重新计时。
    fn sit_body(&mut self, c: &Card, env: &Env) {
        let on = self.settings.sit_remind;
        let sat = self.sat_minutes();
        let goal = self.settings.sit_minutes.max(1);
        let focusing = self.pomo.running && self.pomo.phase == Phase::Focus;
        let lunch = self.lunch_now(env);
        let in_hours = self.remind_window(env);
        let left = ((self.sit_due() - self.t).max(0.0) / 60.0).ceil() as u32;
        let pending = on && self.health_pending[HealthReminder::Sit as usize];
        let counting = on && !pending && left > 0;
        // 待处理时进度停在满环，确认后从零开始。
        let frac = if on { (1.0 - (self.sit_due() - self.t) / (goal as f64 * 60.0)).clamp(0.0, 1.0) as f32 }
            else { (sat as f32 / goal as f32).clamp(0.0, 1.0) };
        let fill = theme::SIT_GREEN;

        if pending {
            c.text(44.0, 59.0, "待活动", 24.0, W::Bold, fill, H::Left);
        } else if counting {
            let ww = c.text(44.0, 61.0, &left.to_string(), size::HERO, W::Bold, theme::WHITE, H::Left);
            c.text(44.0 + ww + 5.0, 60.0, "分钟后提醒", 11.0, W::Semibold, theme::GRAY, H::Left);
        } else {
            c.text(44.0,59.0,if on{"等待提醒"}else{"已关闭"},24.0,W::Bold,theme::GRAY,H::Left);
        }

        // 已坐 25 分钟 · 10:45 提醒
        let tail = if pending { "确认提醒或起身后重新计时".to_string() }
            else if !on { "久坐提醒已关闭".to_string() }
            else if lunch { "午休中不提醒".to_string() }
            else if !in_hours { format!("{:02}:00–{:02}:00 外不提醒", self.settings.water_start, self.settings.water_end) }
            else if !counting && focusing { "专注结束后再提醒".to_string() }
            else { format!("{} 提醒", Self::clock_after(env, self.sit_due() - self.t)) };
        let mut x = 44.0;
        if !pending && counting {
            x += c.text(x, 77.0, "已坐 ", size::SMALL, W::Regular, theme::GRAY, H::Left);
            x += c.text(x, 77.0, &format!("{sat} 分钟"), size::SMALL, W::Bold, fill, H::Left);
            x += c.text(x,77.0," · ",size::SMALL,W::Regular,theme::GRAY,H::Left);
        }
        c.text(x, 77.0, &tail, size::SMALL, W::Regular, theme::GRAY, H::Left);
        c.fill(44.0, 84.0, 151.0, 3.5, 1.75, Color32::from_rgb(34, 34, 37));
        if frac > 0.0 {
            c.fill(44.0, 84.0, (151.0 * frac).max(3.5), 3.5, 1.75, fill);
        }

        let (b, bc) = capsule(c, 43.0, 98.0, 62.0, 25.0, "sit-stand", Color32::from_rgb(24, 84, 52), Color32::from_rgb(30, 104, 64));
        paint::chair(&bc, 55.5, 110.5, 0.36, Color32::from_rgb(150, 230, 180));
        bc.text(64.0, 114.0, "起身了", size::BODY, W::Semibold, Color32::from_rgb(220, 244, 228), H::Left);
        if b.on_hover_text("确认已起身，重新计时").clicked() { self.stood_up(); }

        // 右侧：与喝水同一个环，环心是椅子。
        Self::progress_ring(c, 292.0, 78.0, 33.0, 6.0, frac, Color32::from_rgb(16, 48, 32), if on { fill } else { Color32::from_gray(90) });
        let (label, ink) = if !on { ("已关", theme::GRAY) }
            else if pending { ("动一动", fill) }
            else { ("坐着", Color32::from_gray(217)) };
        paint::chair(c, 292.0, 76.0, 0.75, if !on { theme::GRAY } else if pending { fill } else { theme::WHITE });
        c.text(292.0, 101.0, label, 10.5, W::Semibold, ink, H::Center);
    }

    /// 底部留缺口的进度环（约 276°），起点有圆头亮点。
    fn progress_ring(c: &Card, cx: f32, cy: f32, r: f32, thick: f32, frac: f32, track: Color32, color: Color32) {
        let start = 132.0f32.to_radians();
        let sweep = 276.0f32.to_radians();
        c.arc(cx, cy, r, start, start + sweep, thick, track);
        c.circle(cx + r * start.cos(), cy + r * start.sin(), thick * 0.5, track);
        c.circle(cx + r * (start + sweep).cos(), cy + r * (start + sweep).sin(), thick * 0.5, track);
        let end = start + sweep * frac;
        if frac > 0.002 {
            c.arc(cx, cy, r, start, end, thick, color);
        }
        c.circle(cx + r * start.cos(), cy + r * start.sin(), thick * 0.55, color);
        if frac > 0.002 {
            c.circle(cx + r * end.cos(), cy + r * end.sin(), thick * 0.5, color);
        }
    }

    /// 今日分小时柱状图：12 根柱，从当前时间往前；柱子依次升起。
    fn water_chart(&self, c: &Card, env: &Env, vt: f32) {
        let hour = env.hour();
        let first = hour.saturating_sub(8).min(12);
        let (x0, bottom, top) = (224.0, 103.0, 52.0);
        let max = self.water.hours.iter().copied().max().unwrap_or(0).max(500) as f32;
        let scale = if max > 500.0 { (max / 250.0).ceil() * 250.0 } else { 500.0 };
        for (i, label) in [(0.0, format!("{}", scale as u32)), (0.5, format!("{}", (scale / 2.0) as u32)), (1.0, "0 ml".to_string())] {
            let y = lerp(top, bottom, i);
            c.text(343.0, y + 3.0, &label, size::TINY, W::Regular, theme::DIM, H::Left);
            if i == 0.5 {
                for k in 0..28 {
                    c.fill(x0 + k as f32 * 4.0, y, 2.0, 0.8, 0.0, Color32::from_gray(48));
                }
            }
        }
        c.fill(x0 - 2.0, bottom + 1.5, 114.0, 0.8, 0.0, Color32::from_gray(48));
        for i in 0..12 {
            let hr = first + i;
            let x = x0 + i as f32 * 9.4;
            // 依次升起：每根晚 18 ms
            let rise = anim::ease_out_cubic(((vt - 0.3 - i as f32 * 0.03) / 0.5).clamp(0.0, 1.0));
            let full = bottom - top;
            c.fill(x, bottom - full * rise, 5.6, full * rise, 2.8, Color32::from_rgb(26, 26, 29));
            let v = self.water.hours.get(hr).copied().unwrap_or(0) as f32;
            if v > 0.0 {
                let hgt = (full * (v / scale)).max(5.6) * rise;
                let col = if hr == hour { theme::BLUE } else { Color32::from_rgb(40, 120, 190) };
                c.fill(x, bottom - hgt, 5.6, hgt, 2.8, col);
            } else if hr == hour {
                c.circle(x + 2.8, bottom - 2.8, 2.8 * rise, theme::BLUE);
            }
            if hr % 3 == 0 {
                c.text(x + 2.8, 117.0, &format!("{hr:02}"), size::TINY, W::Regular, theme::GRAY, H::Center);
            }
        }
    }
}
