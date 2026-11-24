//! 喝水与久坐提醒：待处理状态、提醒窗口与每帧推进。

use crate::health::Water;
use crate::island::extras::PetState;
use crate::island::{App, Env, Phase, SIT_TITLE, WATER_PAGE};
use crate::visuals::anim;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum HealthReminder {Water,Sit}
impl HealthReminder {
    pub(crate) const ALL:[Self;2]=[Self::Water,Self::Sit];
    pub(crate) fn title(self)->&'static str {match self {Self::Water=>"喝水时间到了",Self::Sit=>SIT_TITLE}}
    pub(crate) fn label(self)->&'static str {match self {Self::Water=>"待喝水",Self::Sit=>"待活动"}}
    pub(crate) fn id(self)->&'static str {match self {Self::Water=>"health-water",Self::Sit=>"health-sit"}}
    pub(crate) fn icon(self,c:&crate::visuals::paint::Card,x:f32,y:f32,scale:f32) {
        use crate::visuals::theme;
        use eframe::egui::Color32;
        let c=&c.scaled(x,y,scale);
        match self {
            Self::Water=>{
                c.poly(&[(x-7.0,y-1.0),(x+5.0,y-1.0),(x+3.5,y+8.0),(x-5.5,y+8.0)],theme::BLUE);
                c.path(&[(x-9.0,y-10.0),(x-6.5,y+9.0),(x+4.5,y+9.0),(x+7.0,y-10.0)],1.6,theme::CYAN);
                c.seg((x-9.0,y-10.0),(x+7.0,y-10.0),1.6,theme::CYAN);
                c.path(&[(x+6.5,y-6.0),(x+10.5,y-6.0),(x+11.5,y-3.0),(x+10.0,y+2.0),(x+5.5,y+3.0)],1.6,theme::CYAN);
                c.seg((x-5.0,y-6.5),(x-4.2,y-2.5),1.0,Color32::from_white_alpha(170));
            },
            Self::Sit=>crate::visuals::paint::chair(c,x,y,0.6,theme::SOFT_GREEN),
        }
    }
}

impl App {
    pub(crate) fn health_badges(&self)->Vec<HealthReminder> {
        if self.fullscreen_hidden || !self.module_on(WATER_PAGE){return Vec::new();}
        HealthReminder::ALL.into_iter().filter(|kind|self.health_pending[*kind as usize]
            && (match kind {HealthReminder::Water=>self.water.remind,HealthReminder::Sit=>self.settings.sit_remind})
            && (self.open || !self.notice.as_ref().is_some_and(|a|a.health()==Some(*kind)))).collect()
    }
    pub(crate) fn confirm_health_reminder(&mut self,kind:HealthReminder) {
        self.health_pending[kind as usize]=false;
        crate::model::save_health_pending(self.health_pending);
        match kind {HealthReminder::Water=>self.last_reminder=self.t,HealthReminder::Sit=>self.sit_start=self.t}
        self.alerts.retain(|a|a.health()!=Some(kind));
        if self.notice.as_ref().is_some_and(|a|a.health()==Some(kind)){self.notice=None;}
        if self.hud_notice.as_ref().is_some_and(|a|a.health()==Some(kind)){self.hud_notice=None;}
    }
    /// 已连续久坐的分钟数。
    pub(crate) fn sat_minutes(&self) -> u32 {
        ((self.t - self.sit_start).max(0.0) / 60.0) as u32
    }
    /// 下一次久坐提醒的时刻：从上次确认或起身起算满 sit_minutes。
    pub(crate) fn sit_due(&self) -> f64 {
        self.sit_start + self.settings.sit_minutes as f64 * 60.0
    }
    /// 「起身了」：久坐计时归零。
    pub(crate) fn stood_up(&mut self) {
        self.confirm_health_reminder(HealthReminder::Sit);
        self.pet.react(PetState::Happy);
        self.say("久坐计时已重新开始");
    }
    /// 现在是否在午休免打扰时段内（支持跨午夜；起止相同视为未设置）。
    pub(crate) fn lunch_now(&self, env: &Env) -> bool {
        let (a, b) = (self.settings.lunch_start, self.settings.lunch_end);
        let m = env.hour() as u32 * 60 + env.minute();
        self.settings.lunch_rest && a != b && if a < b { m >= a && m < b } else { m >= a || m < b }
    }
    /// 喝水和久坐提醒此刻能否弹出：在提醒时段内，且不在午休。
    pub(crate) fn remind_window(&self, env: &Env) -> bool {
        self.water_hours(env.hour() as u32) && !self.lunch_now(env)
    }
    pub(crate) fn water_hours(&self, hour: u32) -> bool {
        let (a, b) = (self.settings.water_start, self.settings.water_end);
        if a == b {
            true
        } else if a < b {
            hour >= a && hour < b
        } else {
            hour >= a || hour < b
        }
    }
    pub(crate) fn drink_water(&mut self) {        let hour = self.current_hour;
        let amount = self.settings.serving_ml;
        let was = self.water.percent();
        let before = self.water.drunk_ml;
        self.water.drink(amount, hour);
        let amount = self.water.drunk_ml.saturating_sub(before);
        self.last_drink = Some((amount, hour));
        self.confirm_health_reminder(HealthReminder::Water);
        self.pet.react(PetState::Satisfied);
        self.say(format!("已记录 {amount} ml"));
        if was < 100.0 && self.water.percent() >= 100.0 {
            self.alert("今日饮水目标达成", "做得不错，继续保持", 2, true);
        }
    }

    /// 跨天：重新读饮水进度并复位动画。
    pub(crate) fn reload_water(&mut self, today:&str) {
        self.water = Water::load(today);
        self.last_drink=None;
        self.water_ml = self.water.drunk_ml as f32;
        self.water_ring.snap(self.water.percent() / 100.0);
    }

    /// 喝水与久坐提醒、进度环与开关弹簧的每帧推进。
    pub(crate) fn tick_health(&mut self, env: &Env, dt: f32) {
        if self.module_on(WATER_PAGE) && self.water.remind && !self.health_pending[HealthReminder::Water as usize] && self.water.percent() < 100.0 && self.remind_window(env) && self.t - self.last_reminder >= self.settings.water_minutes as f64 * 60.0 {
            self.alert("喝水时间到了", format!("休息一下，喝 {} ml 水吧", self.settings.serving_ml), 2, true);
        }
        // 专注阶段不打断，等专注结束（通常正好进入休息）再提醒。
        let focusing = self.pomo.running && self.pomo.phase == Phase::Focus;
        if self.module_on(WATER_PAGE) && self.settings.sit_remind && !self.health_pending[HealthReminder::Sit as usize] && !focusing && self.remind_window(env) && self.t >= self.sit_due() {
            self.alert(SIT_TITLE, "起来走走，休息一下", 2, true);
        }
        self.water_ring.target = self.water.percent() / 100.0;
        self.water_ring.step(dt);
        self.water_ml = anim::approach(self.water_ml, self.water.drunk_ml as f32, 9.0, dt);
        self.water_view_t.target = self.water_view as f32;
        self.water_view_t.step(dt);
        self.sit_view_t.target = if self.sit_mode { 1.0 } else { 0.0 };
        self.sit_view_t.step(dt);
        self.remind_t.target = if self.water.remind { 1.0 } else { 0.0 };
        self.remind_t.step(dt);
        self.sit_t.target = if self.settings.sit_remind { 1.0 } else { 0.0 };
        self.sit_t.step(dt);
    }
}
