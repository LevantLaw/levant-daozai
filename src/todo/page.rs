//! 待办页：今日进度、清单行、完成勾选与重复标签。

use crate::island::extras::PetState;
use crate::island::{App, Cmd, Env};
use crate::todo::{repeat_label, repeat_tag};
use crate::visuals::anim::{self, smoothstep};
use crate::visuals::paint::{self, capsule, mix, Card, H};
use crate::visuals::theme::{self, size, W};
use eframe::egui::{self, Color32};

impl App {
    // ================= 待办 =================
    pub(crate) fn page_todo(&mut self, c: &Card, env: &Env, _cmds: &mut Vec<Cmd>) {
        // 今天未完成优先，已完成沉底；不在今天的重复待办变暗显示、不能勾选。
        let (done, total) = self.todos.progress(&env.today);
        let order = self.todos.display_order(&env.today);
        let count = order.len();
        let pop = self.todo_pop.pos.abs().min(0.4);
        let hc = c.scaled(60.0, 50.0, 1.0 + pop * 0.25);
        let w = hc.text(42.0, 61.0, &format!("{done}"), size::HERO, W::Bold, if done > 0 && done == total { theme::GREEN } else { theme::WHITE }, H::Left);
        hc.text(42.0 + w, 61.0, &format!("/{total}"), size::HERO, W::Bold, theme::WHITE, H::Left);
        c.text(42.0, 78.0, "已完成", size::SMALL, W::Regular, theme::GRAY, H::Left);

        let (add, ac) = capsule(c, 41.0, 96.0, 74.0, 26.0, "t-add", theme::BTN, theme::BTN_HOVER);
        paint::plus(&ac, 56.0, 109.0, 6.2, theme::WHITE, Color32::BLACK);
        ac.text(67.0, 112.5, "加一个", size::BODY, W::Regular, theme::WHITE, H::Left);
        let can_edit = !self.todos.items.is_empty() && !self.adding && self.editing.is_none();
        let ec = if can_edit { c.clone() } else { c.faded(0.45) };
        let (edit, edc) = capsule(&ec, 121.0, 96.0, 68.0, 26.0, "t-edit", theme::BTN, if can_edit { theme::BTN_HOVER } else { theme::BTN });
        if self.todo_choosing{paint::close(&edc,135.0,109.0,3.5,theme::WHITE);}else{paint::pencil(&edc,135.0,109.0,11.0,theme::WHITE);}
        edc.text(145.0, 112.5, if self.todo_choosing{"取消"}else{"改一下"}, size::BODY, W::Regular, theme::WHITE, H::Left);

        c.fill(196.0, 31.0, 181.0, 93.0, 12.0, theme::CARD);
        let editing = self.adding || self.editing.is_some();
        if count == 0 && !editing {
            paint::checklist(c, 286.5, 64.0, 32.0, Color32::from_gray(150), Color32::from_rgb(52, 54, 58));
            c.text(286.5, 99.0, "今天还没有安排", size::SMALL, W::Regular, theme::GRAY, H::Center);
        } else {
            let mut row = 0;
            let row_y = |r: usize| 46.0 + r as f32 * 22.0;
            if editing {
                // 输入行（文本框本身由 island::input_field 放置）
                let y = row_y(0);
                c.fill(203.0, y - 10.0, 167.0, 20.0, 7.0, Color32::from_rgb(34, 34, 38));
                c.stroke(203.0, y - 10.0, 167.0, 20.0, 7.0, 1.0, paint::alpha(theme::BLUE, 0.7));
                if self.editing.is_some() {
                    paint::pencil(c, 212.0, y, 9.0, theme::BLUE);
                } else {
                    paint::plus(c, 212.0, y, 5.0, theme::BLUE, Color32::BLACK);
                }
                row = 1;
            }
            let ctx = c.ui.ctx().clone();
            let cap = if editing { 2 } else { 3 };
            let mut shown = 0;
            for &i in order.iter().skip(self.todo_first.min(count.saturating_sub(1))) {
                if shown >= cap {
                    break;
                }
                if Some(i) == self.editing {
                    continue;
                }
                let item = self.todos.items[i].clone();
                let today_item = item.scheduled_on(&env.today);
                let y = row_y(row);
                let r = c.hit(202.0, y - 10.0, 169.0, 20.0, ("todo", i, item.text.as_str()));
                let (h, _) = c.hover_t(&r);
                if h > 0.01 {
                    c.fill(202.0, y - 10.0, 169.0, 20.0, 7.0, Color32::from_white_alpha((12.0 * h) as u8));
                }
                let checked = item.done_for(&env.today);
                let ct = ctx.animate_bool_with_time(egui::Id::new(("todo-check", item.text.as_str())), checked, 0.28);
                let rc = if today_item { c.clone() } else { c.faded(0.45) };
                if self.todo_choosing{paint::pencil(c,212.0,y,10.0,theme::BLUE);}else{
                    rc.ring(212.0, y, 5.2, 1.3, paint::alpha(Color32::from_gray(120), 1.0 - ct));
                    if ct > 0.0 {
                        c.circle(212.0, y, 6.0 * anim::ease_out_cubic(ct.min(1.0) / 0.6).min(1.0), theme::GREEN);
                        paint::check(c, 212.0, y, 6.0, smoothstep(0.3, 1.0, ct), 1.4, Color32::BLACK);
                    }
                }
                // 每周重复：文字右侧小标签（周一三五 / 工作日 / 每天）。
                let tag = repeat_tag(item.repeat);
                let tag_w = if tag.is_empty() { 0.0 } else { c.text_w(&tag, size::TINY, W::Regular) + 6.0 };
                let text = c.fit(&item.text, size::BODY, W::Regular, 104.0 - tag_w);
                let repeat_hint = if item.is_repeating() { format!("\n{}{}", repeat_label(item.repeat), if today_item { "" } else { " · 今天不用做" }) } else { String::new() };
                r.clone().on_hover_text(format!("{}{}{}",item.text,repeat_hint,item.reminder_text().map(|d|format!("\n提醒：{d}")).unwrap_or_default()));
                let tcol = mix(Color32::from_gray(235), Color32::from_gray(110), ct);
                let tw = rc.text(223.0, y + 3.8, &text, size::BODY, W::Regular, tcol, H::Left);
                if !tag.is_empty() {
                    rc.text(223.0 + tw + 5.0, y + 3.2, &tag, size::TINY, W::Regular, if today_item { theme::BLUE } else { theme::GRAY }, H::Left);
                }
                if ct > 0.0 {
                    c.fill(223.0, y + 0.2, tw * smoothstep(0.2, 1.0, ct), 1.0, 0.5, Color32::from_gray(110));
                }
                if self.todo_choosing{
                    c.text(366.0,y+3.8,"修改",size::TINY,W::Regular,theme::BLUE,H::Right);
                    if r.clicked(){self.todo_choosing=false;self.editing=Some(i);self.adding=false;self.buf=item.text.clone();self.todo_input_focused=false;break;}
                    row+=1;shown+=1;continue;
                }
                let delete=c.hit(353.0,y-9.0,17.0,18.0,("todo-delete",i)).on_hover_text("删除待办").clicked();
                paint::close(c,361.5,y,2.4,theme::GRAY);
                let reminder=c.hit(333.0,y-9.0,18.0,18.0,("todo-reminder",i)).on_hover_text(item.reminder_text().map(|due|format!("提醒：{due} · 点击修改")).unwrap_or_else(||"设置提醒与每周重复（可选）".into()));
                let reminder_clicked=reminder.clicked();
                let ink=if item.due.is_some()||reminder.hovered(){theme::BLUE}else{theme::GRAY};
                c.ring(342.0,y,4.7,1.1,ink);
                c.path(&[(342.0,y-2.5),(342.0,y),(344.3,y+1.0)],1.1,ink);
                if reminder_clicked {self.open_task_reminder(i);}
                let mut edit_this=false;
                r.context_menu(|ui|{
                    if ui.button("编辑与提醒…").clicked(){edit_this=true;ui.close_menu();}
                    if ui.button("取消提醒").clicked(){self.todos.items[i].due=None;self.todos.save();ui.close_menu();}
                });
                if edit_this {self.open_task_dialog(i);}
                if delete {
                    self.todos.items.remove(i);
                    if let Some(index)=self.editing.as_mut(){if i<*index{*index-=1;}}
                    self.todos.save();self.todo_first=self.todo_first.min(self.todos.items.len().saturating_sub(3));break;
                }
                if r.clicked() && !edit_this && !reminder_clicked && !today_item {
                    self.say(format!("{}的待办，今天不用做", repeat_label(item.repeat)));
                } else if r.clicked() && !r.double_clicked() && !edit_this && !reminder_clicked {
                    let now = self.todos.items[i].done_for(&env.today);
                    self.todos.items[i].toggle(&env.today);
                    self.todos.save();
                    if !now {
                        self.todo_pop.vel += 5.0;
                        self.pet.react(PetState::Happy);
                    }
                }
                row += 1;
                shown += 1;
            }
            let rest = count.saturating_sub(shown + self.editing.map(|_|1).unwrap_or(0));
            if self.todo_choosing{
                c.text(370.0,120.0,if rest>0{"点选修改 · 滚轮查看"}else{"点击要修改的事项"},size::TINY,W::Regular,theme::GRAY,H::Right);
            }else if rest > 0 {
                c.text(370.0, 120.0, &format!("共 {count} 项 · 滚轮查看"), size::TINY, W::Regular, theme::DIM, H::Right);
            }
        }

        if add.clicked() {
            self.todo_choosing=false;
            self.todo_first=0;
            self.adding = true;
            self.editing = None;
            self.buf.clear();
            self.todo_input_focused=false;
        }
        if edit.clicked() && can_edit {
            if count==1 {
                let i=order[0];
                self.editing = Some(i);
                self.adding = false;
                self.buf = self.todos.items[i].text.clone();
                self.todo_input_focused=false;
            }else{self.todo_choosing=!self.todo_choosing;if self.todo_choosing{self.pending.push(Cmd::Focus);}}
        }
    }
}
