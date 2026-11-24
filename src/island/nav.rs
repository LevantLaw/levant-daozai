//! 页面导航：切页、模块开关与设置入口。

use super::*;

impl App {
    /// 设置窗口第 8 个标签页：邮件。
    pub fn open_mail_settings(&mut self) {
        self.settings_tab = 7;
        // 从岛上「去添加」进来时直接打开添加邮箱弹窗。
        self.mail.form.open = true;
        self.open_settings();
    }

    pub fn show_page(&mut self,page:usize) {
        self.agent_open_after_notice=None;
        if !self.settings.visible_pages[page] {
            self.settings_tab=PAGE_SETTINGS_TABS[page];
            self.open_settings();
            return;
        }
        self.notice=None;
        if page==WINDOWS_PAGE {self.windows_unread=false;}
        self.go(page,1.0);
        self.mark_open(true); self.forced_open_until=self.t+self.settings.page_hold_secs;
        let k=self.settings.motion_speed.factor().powi(2);
        self.w.tune(OPEN_W.0*k,OPEN_W.1);self.h.tune(OPEN_H.0*k,OPEN_H.1);
        self.w.target=PW;self.h.target=self.expanded_height();
    }
    pub fn open_settings(&mut self){
        self.settings_open=true;
        self.settings_focus_requested=true;
    }
    pub fn open_settings_category(&mut self,category:usize){
        self.settings_category=category.min(3);
        self.health_time=false;
        if category==0 || category>=3 {self.settings_tab=if category==0{0}else{6};self.open_settings();return;}
        let page=if category==1{2}else{1};
        self.show_page(page);
        if self.settings.visible_pages[page] && (category==1 || category==2) {self.subpage=Some(page);}
    }
    pub fn page(&self) -> usize {
        self.page
    }
    /// 打开健康提醒设置里的「提醒时段」二级页（演示录制用）。
    pub fn show_health_time(&mut self) { self.health_time = true; }
    /// 切到饮水页的久坐屏，并假定已连续久坐 minutes 分钟（演示录制用）。
    pub fn show_sit_view(&mut self, minutes: f64) {
        self.show_page(WATER_PAGE);
        self.sit_mode = true;
        self.sit_start = self.t - minutes * 60.0;
    }

    // ---------- 功能模块：页面开关即模块开关，关闭后页面隐藏、后台同步和提醒一并停止 ----------

    /// 模块是否开启（与「功能页面」里的页面开关一致）。
    pub fn module_on(&self,page:usize)->bool {self.settings.visible_pages.get(page).copied().unwrap_or(false)}
    /// 收信线程是否应运行：邮箱模块开启且打开了「新邮件提醒」。
    pub fn mail_active(&self)->bool {self.module_on(MAIL_PAGE) && self.settings.mail_enabled}
    /// 剪贴板监听是否应运行：剪贴板模块开启且打开了「记录剪贴板」。
    pub fn clipboard_active(&self)->bool {self.module_on(CLIPBOARD_PAGE) && self.settings.clipboard_history}
    /// 额度查询是否应运行：AI 额度模块开启且打开了「开启 AI 额度」。
    pub fn quota_active(&self)->bool {self.module_on(QUOTA_PAGE) && self.quota.config.enabled}
    /// 旧版设置页里「新邮件提醒」「开启 AI 额度」与功能页面的模块开关是两层开关。
    /// 合并后只保留模块开关：旧的内部开关关着时，迁移为模块关闭，内部开关恢复为开。
    /// 只剩这一个页面、模块关不掉时保持原样，不改变已有行为。
    pub fn migrate_merged_switches(&mut self){
        let mut settings_changed=false;
        if !self.settings.mail_enabled {
            self.set_page_visible(MAIL_PAGE,false);
            if !self.settings.visible_pages[MAIL_PAGE] {self.settings.mail_enabled=true;settings_changed=true;}
        }
        if !self.quota.config.enabled {
            self.set_page_visible(QUOTA_PAGE,false);
            if !self.settings.visible_pages[QUOTA_PAGE] {self.quota.config.enabled=true;self.quota.config.save();settings_changed=true;}
        }
        if settings_changed {self.settings.save();}
    }

    pub(crate) fn set_page_visible(&mut self,page:usize,visible:bool) {
        if !visible && self.settings.visible_pages.iter().filter(|enabled|**enabled).count()==1
            && self.settings.visible_pages[page] { return; }
        self.settings.visible_pages[page]=visible;
        // 关掉专注模块时一并停止正在进行的计时，避免页面不可见却到点弹提醒。
        if !visible && page==FOCUS_PAGE {self.pomo.running=false;}
        if !visible && page==WINDOWS_PAGE {self.windows_unread=false;}
        if !self.settings.visible_pages[self.page] {
            if let Some(first)=self.settings.page_order.iter().copied().find(|page|self.settings.visible_pages[*page]) { self.go(first,1.0); }
        }
    }

    pub(crate) fn navigate_page(&mut self,next:bool) {
        let position=self.settings.page_order.iter().position(|page|*page==self.page).unwrap();
        for offset in 1..=PAGE_NAMES.len() {
            let page=self.settings.page_order[(position+if next {offset}else{PAGE_NAMES.len()-offset})%PAGE_NAMES.len()];
            if self.settings.visible_pages[page] {
                self.go(page,if next {1.0}else{-1.0});
                break;
            }
        }
    }

    /// 带过渡地切到某页。`dir` > 0 表示内容从右侧进入。
    pub(crate) fn go(&mut self, page: usize, dir: f32) {
        if page == self.page || !self.settings.visible_pages[page] { return; }
        self.music_volume_open=false;
        self.subpage=None;
        // 连点：上一段过渡还没结束时，以当前页为起点重新开始，位移速度保留。
        self.page_from = self.page;
        self.page = page;
        if page==CALENDAR_PAGE {self.request_calendar_month(false);}
        if page==WINDOWS_PAGE {self.windows_unread=false;}
        self.page_dir = dir;
        self.page_t0 = self.t;
        self.slide.pos = dir * PAGE_SLIDE + if self.page_e() < 0.4 { self.slide.pos * 0.3 } else { 0.0 };
        self.slide.target = 0.0;
        self.dots_target();
        self.cancel_edit();
    }

    pub(crate) fn page_e(&self) -> f32 {
        (self.t - self.page_t0) as f32
    }

    pub(crate) fn dots_target(&mut self) {
        for (i, d) in self.dots.iter_mut().enumerate() {
            d.target = if i == self.page { 16.0 } else { 4.0 };
        }
    }

}
