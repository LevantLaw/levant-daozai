//! 待办编辑器：内容、每周重复与提醒时间。

use crate::island::extras::setting_button;
use crate::island::{App, Cmd, TODO_PAGE};
use crate::todo::picker::{PickerAction, ReminderPicker};
use crate::todo::{repeat_label, REPEAT_ALL, WEEKDAY_NAMES};
use crate::visuals::paint::{Card, H};
use crate::visuals::theme::{self, size, W};
use eframe::egui::{self, Color32, Rect};

/// 将本地提醒时间规范化，拒绝不存在的日期与超出范围的时分。
fn parse_due(value: &str, today: &str) -> Option<String> {
    let value = value.trim();
    let full = if value.len() == 5 {
        format!("{today} {value}")
    } else {
        value.to_string()
    };
    if !full.is_ascii() || full.len() != 16 {
        return None;
    }
    let b = full.as_bytes();
    if b[4] != b'-' || b[7] != b'-' || b[10] != b' ' || b[13] != b':' {
        return None;
    }
    let year = full[0..4].parse::<u32>().ok()?;
    let month = full[5..7].parse::<usize>().ok()?;
    let day = full[8..10].parse::<u32>().ok()?;
    let hour = full[11..13].parse::<u32>().ok()?;
    let minute = full[14..16].parse::<u32>().ok()?;
    if !(2000..=9999).contains(&year) || !(1..=12).contains(&month) || hour > 23 || minute > 59 {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day == 0 || day > days[month - 1] {
        return None;
    }
    Some(full)
}
pub(crate) struct TaskDraft {index:usize,original:String,text:String,due:String,error:String,focused:bool,reminder_only:bool,pub(crate) repeat:u8,pub(crate) picker:ReminderPicker}
impl App {
    pub(crate) fn open_task_dialog(&mut self,index:usize){
        self.todo_choosing=false;
        if let Some(task)=self.todos.items.get(index){
            let draft=TaskDraft{index,original:task.text.clone(),text:task.text.clone(),due:task.due.clone().unwrap_or_default(),error:String::new(),focused:false,reminder_only:false,repeat:task.repeat,picker:ReminderPicker::new(task.due.as_deref())};
            self.show_page(TODO_PAGE);self.task_dialog=Some(draft);
        }
    }
    pub(crate) fn open_task_reminder(&mut self,index:usize){
        self.open_task_dialog(index);
        if let Some(d)=self.task_dialog.as_mut(){d.reminder_only=true;d.picker.show();}
    }
    pub(crate) fn draw_task_editor(&mut self,c:&Card){
        if self.task_dialog.as_ref().is_some_and(|draft|draft.picker.open){self.draw_task_reminder(c);return;}
        let reminder_only=self.task_dialog.as_ref().is_some_and(|d|d.reminder_only);
        c.text(15.0,25.0,if reminder_only{"设置提醒"}else{"编辑待办"},size::TITLE,W::Bold,theme::WHITE,H::Left);
        c.text(43.0,48.0,if reminder_only{"待办"}else{"内容"},10.5,W::Regular,theme::GRAY,H::Left);
        c.fill(91.0,33.0,275.0,20.0,6.0,theme::CARD);
        if let Some(d)=self.task_dialog.as_ref().filter(|d|d.reminder_only){
            c.text(97.0,47.0,&c.fit(&d.text,11.5,W::Regular,260.0),11.5,W::Regular,theme::WHITE,H::Left);
        }
        c.text(43.0,72.0,"提醒",10.5,W::Regular,theme::GRAY,H::Left);
        let time=self.task_dialog.as_ref().map(|d|if d.due.is_empty(){"点击选择时间".into()}
            else if d.repeat!=0{format!("{} {}",repeat_label(d.repeat),d.due.get(11..16).unwrap_or(&d.due))}else{d.due.clone()}).unwrap_or_default();
        if setting_button(c,91.0,58.0,275.0,&time,false,"task-select-reminder"){if let Some(d)=self.task_dialog.as_mut(){d.picker.show();}}
        // 每周重复：七个星期按钮，全不选即一次性待办。
        c.text(43.0,95.0,"重复",10.5,W::Regular,theme::GRAY,H::Left);
        self.task_repeat_chips(c,91.0,81.0,25.0,19.0,28.0);
        let repeat=self.task_dialog.as_ref().map(|d|d.repeat).unwrap_or(0);
        let quick=if repeat==REPEAT_ALL{"不重复"}else{"每天"};
        if setting_button(c,289.0,81.0,77.0,quick,false,"task-repeat-all"){
            if let Some(d)=self.task_dialog.as_mut(){d.repeat=if d.repeat==REPEAT_ALL{0}else{REPEAT_ALL};}
        }
        if let Some(d)=&self.task_dialog {
            let summary=if d.repeat!=0{repeat_label(d.repeat)}else{"只做一次".into()};
            let hint=if d.error.is_empty(){summary.as_str()}else{&d.error};
            c.text(43.0,117.0,&c.fit(hint,10.5,W::Regular,if reminder_only{140.0}else{154.0}),10.5,if d.error.is_empty(){W::Regular}else{W::Semibold},if d.error.is_empty(){theme::GRAY}else{theme::RED},H::Left);
        }
        if reminder_only {
            let has_reminder=self.task_dialog.as_ref().and_then(|d|self.todos.items.get(d.index)).is_some_and(|task|task.due.is_some());
            let skip=if has_reminder{"取消提醒"}else if repeat!=0{"只设重复"}else{"暂不提醒"};
            if setting_button(c,188.0,104.0,88.0,skip,false,"task-reminder-skip"){
                if let Some(d)=self.task_dialog.as_mut(){d.due.clear();}
                self.save_task_editor();
            }
        }else if setting_button(c,202.0,104.0,74.0,"取消",false,"task-cancel"){self.task_dialog=None;}
        if setting_button(c,286.0,104.0,80.0,if reminder_only{"选择时间"}else{"保存"},true,"task-save"){
            if reminder_only{if let Some(d)=self.task_dialog.as_mut(){d.picker.show();}}else{self.save_task_editor();}
        }
    }
    pub(crate) fn task_editor_inputs(&mut self,ui:&mut egui::Ui,origin:egui::Pos2,cmds:&mut Vec<Cmd>){
        let Some(d)=self.task_dialog.as_mut() else{return};
        if d.reminder_only || d.picker.open{return;}
        let name=ui.put(Rect::from_min_size(origin+egui::vec2(97.0,35.0),egui::vec2(260.0,17.0)),egui::TextEdit::singleline(&mut d.text).vertical_align(egui::Align::Center).id(egui::Id::new("task-name-edit")).frame(false).font(theme::font(11.5,W::Regular)));
        let first_focus=!d.focused;
        if first_focus{name.request_focus();d.focused=true;cmds.push(Cmd::Focus);}
        // 新建内容与下一步共用这一帧，避免同一次回车又提交了空提醒。
        if !first_focus && ui.input(|i|i.key_pressed(egui::Key::Enter)){self.save_task_editor();}
    }
    /// 七个星期按钮（一…日），点一下切换；全不选即一次性待办。
    fn task_repeat_chips(&mut self,c:&Card,x:f32,y:f32,w:f32,h:f32,step:f32){
        let repeat=self.task_dialog.as_ref().map(|d|d.repeat).unwrap_or(0);
        for (day,name) in WEEKDAY_NAMES.iter().enumerate() {
            let on=repeat&(1<<day)!=0;
            let cx=x+day as f32*step;
            let (r,b)=crate::visuals::paint::capsule(c,cx,y,w,h,&format!("task-repeat-{day}"),if on{Color32::from_rgb(39,94,150)}else{theme::BTN},theme::BTN_HOVER);
            b.centered_text(cx+w/2.0,y+h/2.0,name,if h<18.0{9.5}else{10.5},W::Semibold,if on{theme::WHITE}else{theme::GRAY},H::Center);
            if r.on_hover_text(format!("每周{name}")).clicked(){
                if let Some(d)=self.task_dialog.as_mut(){d.repeat^=1<<day;d.error.clear();}
            }
        }
    }
    fn draw_task_reminder(&mut self,c:&Card){
        use PickerAction;
        let Some(d)=self.task_dialog.as_mut().filter(|d|d.picker.open) else{return};
        let has_reminder=self.todos.items.get(d.index).is_some_and(|task|task.due.is_some());
        c.text(15.0,25.0,"待办",size::TITLE,W::Bold,theme::WHITE,H::Left);
        c.text(44.0,25.0,"清单",size::TITLE,W::Bold,theme::GREEN,H::Left);
        let (done,total)=self.todos.progress(&self.today);
        c.text(42.0,61.0,&format!("{done}/{total}"),29.0,W::Bold,theme::WHITE,H::Left);
        // 左侧：每周重复。选了星期几后，提醒只按时间、在这些日子到点提醒。
        d.picker.repeat_note=Some(if d.repeat!=0{format!("{} · 到点提醒",repeat_label(d.repeat))}else{String::new()});
        let action=d.picker.draw(c,&d.text,has_reminder);
        let focus=std::mem::take(&mut d.picker.focus);
        self.task_repeat_chips(c,41.0,84.0,17.0,16.0,19.5);
        let Some(d)=self.task_dialog.as_mut() else{return};
        if focus{self.pending.push(Cmd::Focus);}
        match action{
            Some(PickerAction::Save)=>{d.due=d.picker.due();d.picker.open=false;if d.reminder_only{self.save_task_editor();}},
            Some(PickerAction::Skip)=>{d.due.clear();d.picker.open=false;if d.reminder_only{self.save_task_editor();}},
            Some(PickerAction::Cancel)=>{d.picker.open=false;if d.reminder_only{self.task_dialog=None;}},
            None=>{},
        }
    }
    fn save_task_editor(&mut self){
        let Some(mut d)=self.task_dialog.take() else{return};
        let due=if d.due.trim().is_empty(){Some(None)}else{parse_due(&d.due,&self.today).map(Some)};
        if d.text.trim().is_empty(){d.error="请填写任务内容".into();}
        else if due.is_none(){d.error="时间格式应为 18:30 或完整日期时间".into();}
        else if self.todos.items.get(d.index).is_none_or(|t|t.text!=d.original){d.error="任务已变化，请取消后重新打开".into();}
        else{
            let item=&mut self.todos.items[d.index];
            if item.repeat!=d.repeat{item.done_on=None;}
            item.text=d.text.trim().into();item.due=due.unwrap();item.reminded=false;item.reminded_on=None;item.repeat=d.repeat;
            self.todos.save();self.say("待办已保存");return;
        }
        self.task_dialog=Some(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn due_time_checks_calendar_and_format() {
        assert_eq!(
            parse_due("09:30", "2026-10-29").as_deref(),
            Some("2026-10-29 09:30")
        );
        assert!(parse_due("2026-02-29 12:00", "").is_none());
        assert!(parse_due("2028-02-29 12:00", "").is_some());
        for value in [
            "24:00",
            "12:60",
            "明天中午",
            "2026-13-01 10:00",
            "2026-00-01 10:00",
        ] {
            assert!(parse_due(value, "2026-10-29").is_none());
        }
    }
}
