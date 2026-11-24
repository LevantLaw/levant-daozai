//! 待办：清单状态、提醒滚轮、编辑器、岛内页与设置。

pub(crate) mod editor;
pub(crate) mod list;
pub(crate) mod page;
pub(crate) mod picker;
pub(crate) mod settings;

pub(crate) use editor::TaskDraft;
pub(crate) use list::{
    repeat_label, repeat_tag, TodoItem, TodoList, REPEAT_ALL, WEEKDAY_NAMES,
};

use crate::island::{App, Env, TODO_PAGE};

impl App {
    /// 到期待办的提醒：按模块开关过滤，逐条标记已提醒并弹横幅。
    pub(crate) fn todo_reminders(&mut self, env: &Env) {
        let now = format!("{} {}", env.today, env.clock.get(..5).unwrap_or("00:00"));
        let todo_on=self.module_on(TODO_PAGE);
        let due: Vec<_> = self.todos.items.iter().enumerate().filter(|(_, item)| todo_on && item.reminder_due(&now)).map(|(i,item)| (i,item.text.clone())).collect();
        for (i,text) in due { self.todos.items[i].mark_reminded(&now); self.todos.save(); self.alert("待办到时间了", text, 4, true); }
    }
}
