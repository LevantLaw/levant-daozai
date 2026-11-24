//! 邮件页：验证码大卡、邮件列表与详情。
//! 坐标全部是面板局部逻辑点（419×140），文字 y 是基线。

use crate::island::{App, Cmd, Env};
use crate::mail;
use crate::visuals::paint::{self, capsule, Card, H};
use crate::visuals::theme::{self, size, W};
use eframe::egui::{self, Color32, Pos2, Rect};

impl App {

    // ---------------------------------------------------------------
    // 邮件：有验证码大卡时分栏，否则显示紧凑单行列表。
    // ---------------------------------------------------------------
    fn visible_mail_code(&self)->Option<&mail::Item> {
        self.mail.code_item().filter(|item|!self.mail.manage.trash_open && self.settings.mail_code && !self.mail_code_hidden(item))
    }
    pub(crate) fn mail_list_bounds(&self)->Rect {
        let left=if self.visible_mail_code().is_some(){182.0}else{36.0};
        Rect::from_min_max(Pos2::new(left,54.0),Pos2::new(380.0,122.0))
    }
    pub(crate) fn page_mail(&mut self, c: &Card, _env: &Env, cmds: &mut Vec<Cmd>) {
        if self.mail.accounts.is_empty() {
            c.text(42.0, 60.0, "还没有邮箱", 13.0, W::Bold, theme::WHITE, H::Left);
            let hint = c.fit("在设置 → 邮箱里添加；授权码只存进系统凭据管理器", 10.5, W::Regular, 344.0);
            c.text(42.0, 77.0, &hint, 10.5, W::Regular, theme::GRAY, H::Left);
            let (go, gc) = capsule(c, 42.0, 88.0, 84.0, 26.0, "mail-setup", theme::BTN, theme::BTN_HOVER);
            gc.text(58.0, 105.0, "去添加", size::BODY, W::Regular, theme::WHITE, H::Left);
            if go.clicked() {
                self.open_mail_settings();
            }
            return;
        }

        self.mail_manage_tick();
        // v0.5.5：点开某封邮件 → 整块换成正文摘要，左上「返回」回到列表。
        if let Some((account, uid)) = self.mail.open_uid.clone() {
            match self.mail_rows()
                .iter()
                .find(|m| m.uid == uid && m.account.eq_ignore_ascii_case(&account))
                .cloned()
            {
                Some(item) => {
                    self.mail_detail(c, &item, cmds);
                    return;
                }
                None => self.mail.open_uid = None,
            }
        }
        let code=self.visible_mail_code().cloned();
        if let Some(raw)=&code {
            c.fill(36.0,36.0,140.0,86.0,11.0,theme::CARD);
            let hidden=self.mail_code_hidden(raw);
            let item=self.display_mail(raw);
            let value=item.code.as_deref().unwrap_or("••••••");
            c.text(46.0,51.0,"验证码",10.5,W::Regular,theme::GRAY,H::Left);
            c.text(166.0,51.0,&c.fit(&item.from,10.5,W::Semibold,78.0),10.5,W::Semibold,theme::WHITE,H::Right);
            let code_size=23.0*(120.0/c.text_w(value,23.0,W::Bold)).min(1.0);
            c.text(46.0,75.0,value,code_size,W::Bold,theme::GREEN,H::Left);
            c.text(46.0,91.0,&c.fit(&item.subject,10.5,W::Regular,120.0),10.5,W::Regular,Color32::from_gray(190),H::Left);
            c.text(46.0,108.0,if hidden{"已隐藏 · 点击查看"}else{"点击复制"},10.5,W::Regular,theme::DIM,H::Left);
            c.text(166.0,108.0,item.time.rsplit(' ').next().unwrap_or(&item.time),10.0,W::Regular,theme::GRAY,H::Right);
            c.hit(42.0,38.0,128.0,16.0,"mail-code-source").on_hover_text(format!("来自：{}\n{}\n收件邮箱：{}\n{}",item.from,item.subject,item.account,item.time));
            if c.hit(40.0,56.0,132.0,38.0,("mail-code",item.uid)).on_hover_text(format!("复制 {} 的验证码",item.from)).clicked(){if hidden{self.reveal_mail_code(raw);}else{cmds.push(Cmd::Copy(value.to_string()));self.code_copied(value);}}
        }
        use crate::mail::manage::{Action,Folder};
        let folder=self.mail.manage.folder();let trash=folder==Folder::Trash;
        let list_bounds=self.mail_list_bounds();let list_x=list_bounds.left();let row_w=list_bounds.width()-12.0;
        let compact=code.is_none();let row_h=if compact{21.5}else{34.0};
        let all_count=self.mail.recent.len();
        let unread=self.mail.recent.iter().filter(|m|self.mail_seen(m,Folder::Inbox)==Some(false)).count();
        if trash{
            c.text(list_x+6.0,47.0,&format!("{} {}",if self.settings.mail_sync_operations{"回收站"}else{"本机回收站"},self.mail.manage.trash.len()),10.5,W::Semibold,theme::WHITE,H::Left);
            c.text(375.0,47.0,"返回邮件",10.0,W::Regular,theme::BLUE,H::Right);
            if c.hit(322.0,33.0,56.0,19.0,"mail-trash-back").clicked(){self.mail_toggle_trash();return;}
        }else{
            for (x,width,label,selected,id) in [
                (list_x,52.0,format!("全部 {all_count}"),!self.mail.manage.unread_only,"mail-all"),
                (list_x+56.0,52.0,format!("未读 {unread}"),self.mail.manage.unread_only,"mail-unread"),
            ]{
                if selected{c.fill(x,33.0,width,19.0,6.0,theme::BTN);}
                c.text(x+width/2.0,47.0,&label,10.0,W::Semibold,if selected{theme::WHITE}else{theme::GRAY},H::Center);
                if c.hit(x,33.0,width,19.0,id).on_hover_text("当前加载的最近邮件").clicked(){self.mail.manage.unread_only=id=="mail-unread";}
            }
            c.text(307.0,46.0,if self.settings.mail_sync_operations{"同步"}else{"仅本机"},8.0,W::Regular,theme::DIM,H::Center);
            let ready=self.mail.manage.pending.is_empty()&&self.mail.accounts.iter().any(|a|a.enabled&&self.mail.manage.data.contains_key(&(crate::mail::key(a),Folder::Inbox))&&self.mail.manage.identities.get(&crate::mail::key(a))==Some(a));
            c.text(375.0,47.0,"全部已读",10.0,W::Regular,if ready{theme::BLUE}else{theme::DIM},H::Right);
            if c.hit(325.0,33.0,53.0,19.0,"mail-read-all").on_hover_text(if self.settings.mail_sync_operations{"将已连接邮箱的未读邮件全部标为已读"}else{"仅将岛仔当前加载的邮件标为已读，不更改邮箱"}).clicked()&&ready{
                self.mail_manage_request(None,Folder::Inbox,Action::ReadAll);
            }
        }
        if !trash{
            let hit=c.hit(359.0,122.0,22.0,17.0,"mail-trash").on_hover_text("回收站");
            crate::mail::manage_ui::trash_icon(c,370.0,130.0,if hit.hovered(){theme::WHITE}else{theme::GRAY});
            if hit.clicked(){self.mail_toggle_trash();return;}
        }
        if !self.mail.manage.message.is_empty(){
            c.text(42.0,134.0,&c.fit(&self.mail.manage.message,8.0,W::Regular,125.0),8.0,W::Regular,theme::GRAY,H::Left);
            if c.hit(40.0,123.0,130.0,16.0,"mail-manage-status").on_hover_text(format!("{}\n点击刷新邮件状态",self.mail.manage.message)).clicked(){self.mail_manage_request(None,folder,Action::Load);}
        }
        let indices:Vec<_>=self.mail_rows().iter().enumerate().filter(|(_,m)|trash||!self.mail.manage.unread_only||self.mail_seen(m,folder)==Some(false)).map(|(i,_)|i).collect();
        if indices.is_empty(){
            let message=if !self.mail.manage.pending.is_empty(){"正在同步邮件…"}else if trash{"回收站为空"}else if self.mail.manage.unread_only{"没有未读邮件"}else{"还没有收到邮件"};
            c.text(list_x+6.0,80.0,message,11.0,W::Regular,theme::GRAY,H::Left);return;
        }
        let mut content=egui::Ui::new(c.ui.ctx().clone(),egui::Id::new(("mail-list",c.live)),
            egui::UiBuilder::new().max_rect(c.rect(list_bounds.left(),list_bounds.top(),list_bounds.width(),list_bounds.height())).layer_id(c.painter.layer_id()).style(c.ui.style().clone()));
        content.set_clip_rect(c.rect(list_bounds.left(),list_bounds.top(),list_bounds.width(),list_bounds.height()).intersect(c.painter.clip_rect()));
        content.set_opacity(c.painter.opacity());if !c.live{content.disable();}
        content.spacing_mut().item_spacing.y=0.0;content.spacing_mut().scroll.floating=false;content.spacing_mut().scroll.bar_width=4.0*c.s;
        content.visuals_mut().widgets.inactive.bg_fill=Color32::from_gray(80);
        egui::ScrollArea::vertical().id_salt(("mail-list-scroll",trash,self.mail.manage.unread_only)).max_height(list_bounds.height()*c.s).auto_shrink([false,false]).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible).show_rows(&mut content,row_h*c.s,indices.len(),|ui,range|{
            let rows:Vec<_>=range.map(|i|self.mail_rows()[indices[i]].clone()).collect();
            for raw in rows{
                let item=self.display_mail(&raw);let hidden=self.mail_code_hidden(&raw);let seen=self.mail_seen(&raw,folder);
                let(rect,_)=ui.allocate_exact_size(egui::vec2(row_w*c.s,row_h*c.s),egui::Sense::hover());
                let mut row=Card::new(ui,rect.min,ui.clip_rect());row.s=c.s;row.live=c.live;
                let hit=row.hit(0.0,0.0,row_w,row_h-1.0,("mail-row",item.uid,item.account.as_str()));
                let hovered=ui.ctx().pointer_hover_pos().is_some_and(|p|rect.contains(p)&&ui.clip_rect().contains(p));
                let ready=self.mail_ready(&item.account,folder);let actions=trash||hovered&&ready;
                if hovered{row.fill(0.0,0.0,row_w,row_h-1.0,5.0,Color32::from_gray(29));}
                if seen==Some(false)&&!trash{row.circle(5.0,if compact{10.5}else{8.0},2.1,theme::BLUE);}
                let ink=if seen==Some(true){theme::GRAY}else{theme::WHITE};
                if compact{
                    row.text(12.0,14.0,&row.fit(&item.from,11.0,W::Semibold,57.0),11.0,W::Semibold,ink,H::Left);
                    let subject_w=(row_w-128.0-if hidden{32.0}else{0.0}).max(10.0);
                    let width=row.text(76.0,14.0,&row.fit(&item.subject,10.5,W::Regular,subject_w),10.5,W::Regular,if seen==Some(true){theme::GRAY}else{Color32::from_gray(210)},H::Left);
                    if hidden{row.fill(80.0+width,5.0,29.0,12.0,3.0,theme::BTN);row.text(94.5+width,14.0,"已隐藏",8.0,W::Regular,theme::GRAY,H::Center);}
                }else{
                    row.text(12.0,12.0,&row.fit(&item.from,11.0,W::Semibold,row_w-65.0),11.0,W::Semibold,ink,H::Left);
                    row.text(12.0,27.0,&row.fit(&item.subject,10.0,W::Regular,row_w-20.0),10.0,W::Regular,theme::GRAY,H::Left);
                }
                let cy=if compact{10.0}else{8.0};let mut acted=false;
                if actions{
                    if trash{
                        row.text(row_w-5.0,cy+4.0,"恢复",10.0,W::Regular,if ready{theme::BLUE}else{theme::DIM},H::Right);
                        if row.hit(row_w-39.0,0.0,36.0,20.0,("mail-restore",item.uid,item.account.as_str())).clicked()&&ready{self.mail_manage_request(Some(&item.account),folder,Action::Move{uid:item.uid});acted=true;}
                    }else{
                        row.ring(row_w-35.0,cy,4.7,1.1,theme::GRAY);paint::check(&row,row_w-35.0,cy,6.0,if seen==Some(true){1.0}else{0.0},1.1,theme::GRAY);
                        if row.hit(row_w-45.0,0.0,20.0,20.0,("mail-seen",item.uid,item.account.as_str())).on_hover_text(if seen==Some(true){"标为未读"}else{"标为已读"}).clicked(){self.mail_manage_request(Some(&item.account),folder,Action::Read{uid:item.uid,seen:seen!=Some(true)});acted=true;}
                        let remove=row.hit(row_w-24.0,0.0,21.0,20.0,("mail-delete",item.uid,item.account.as_str())).on_hover_text(if self.settings.mail_sync_operations{"移到邮箱回收站"}else{"移到本机回收站，不更改邮箱"});
                        crate::mail::manage_ui::trash_icon(&row,row_w-14.0,cy,if remove.hovered(){theme::RED}else{theme::GRAY});
                        if remove.clicked(){self.mail_manage_request(Some(&item.account),folder,Action::Move{uid:item.uid});acted=true;}
                    }
                }else{row.text(row_w-6.0,cy+4.0,item.time.rsplit(' ').next().unwrap_or(&item.time),10.0,W::Regular,theme::GRAY,H::Right);}
                row.seg((6.0,row_h-0.5),(row_w-6.0,row_h-0.5),0.5,Color32::from_gray(40));
                hit.clone().on_hover_text(format!("{}\n{}\n收件邮箱：{}\n{}",item.from,item.subject,item.account,item.time));
                if hit.clicked()&&!acted{self.mail_open(&raw);}
            }
        });
    }

    /// 岛内邮件详情可滚动，也可以进入独立窗口阅读全文。
    pub(crate) fn mail_detail(&mut self, c: &Card, raw: &mail::Item, cmds: &mut Vec<Cmd>) {
        let hidden=self.mail_code_hidden(raw);
        let display=self.display_mail(raw);let item=&display;
        let body=self.mail_body_lines(item);
        c.fill(36.0, 36.0, 347.0, 188.0, 11.0, theme::CARD);
        let back = c.hit(330.0, 42.0, 44.0, 14.0, ("mail-back", item.uid));
        let bh = c.hover_t(&back).0;
        c.text(374.0, 50.0, "返回", 10.5, W::Regular, if bh > 0.5 { theme::WHITE } else { theme::GRAY }, H::Right);
        if back.clicked() {
            self.mail.open_uid = None;
            return;
        }
        let folder=self.mail.manage.folder();let ready=self.mail_ready(&item.account,folder);
        let seen=self.mail_seen(raw,folder)==Some(true);
        let status=if folder==crate::mail::manage::Folder::Trash{"恢复"}else if seen{"标为未读"}else{"标为已读"};
        c.text(215.0,50.0,status,9.0,W::Regular,if ready{theme::BLUE}else{theme::DIM},H::Right);
        if c.hit(166.0,39.0,50.0,18.0,("mail-detail-seen",item.uid)).clicked()&&ready{
            let action=if folder==crate::mail::manage::Folder::Trash{crate::mail::manage::Action::Move{uid:item.uid}}else{crate::mail::manage::Action::Read{uid:item.uid,seen:!seen}};
            self.mail_manage_request(Some(&item.account),folder,action);
        }
        if folder==crate::mail::manage::Folder::Inbox{
            c.text(252.0,50.0,"删除",9.0,W::Regular,if ready{theme::RED}else{theme::DIM},H::Right);
            if c.hit(220.0,39.0,35.0,18.0,("mail-detail-delete",item.uid)).on_hover_text(if self.settings.mail_sync_operations{"移到邮箱回收站"}else{"移到本机回收站，不更改邮箱"}).clicked()&&ready{self.mail_manage_request(Some(&item.account),folder,crate::mail::manage::Action::Move{uid:item.uid});}
        }
        let from = c.fit(&item.from, 11.5, W::Semibold, 112.0);
        c.text(46.0, 50.0, &from, 11.5, W::Semibold, theme::WHITE, H::Left);
        let read=c.hit(266.0,42.0,60.0,14.0,("mail-read",&item.account,item.uid)).on_hover_text("在独立窗口阅读完整正文");
        c.text(324.0,50.0,"阅读全文",10.5,W::Regular,if read.hovered(){theme::BLUE}else{theme::GRAY},H::Right);
        if read.clicked(){self.mail_open_reader(raw);}
        let subject = c.fit(&item.subject, 13.5, W::Semibold, 328.0);
        c.text(46.0, 73.0, &subject, 13.5, W::Semibold, theme::WHITE, H::Left);
        c.hit(46.0,58.0,328.0,20.0,("mail-subject",&item.account,item.uid)).on_hover_text(&item.subject);
        c.text(374.0, 92.0, &item.time, 10.0, W::Regular, Color32::from_gray(170), H::Right);
        let account=format!("收件邮箱：{}",item.account);
        c.text(46.0,92.0,&c.fit(&account,10.0,W::Regular,230.0),10.0,W::Regular,Color32::from_gray(175),H::Left);
        c.hit(46.0,80.0,230.0,16.0,("mail-account",&item.account,item.uid)).on_hover_text(&account);

        // 正文和链接共用滚动区域，会议与验证码操作保留在底部。
        let mut content=egui::Ui::new(c.ui.ctx().clone(),egui::Id::new(("mail-content",&item.account,item.uid,c.live)),
            egui::UiBuilder::new().max_rect(c.rect(46.0,104.0,328.0,90.0)).layer_id(c.painter.layer_id()).style(c.ui.style().clone()));
        content.set_clip_rect(c.rect(46.0,104.0,328.0,90.0).intersect(c.painter.clip_rect()));
        content.set_opacity(c.painter.opacity());
        if !c.live {content.disable();}
        content.spacing_mut().item_spacing.y=6.0*c.s;
        content.style_mut().spacing.scroll.floating=false;
        content.spacing_mut().scroll.bar_width=4.0*c.s;
        egui::ScrollArea::vertical().id_salt(("mail-body",&item.account,item.uid)).max_height(90.0*c.s).auto_shrink([false,false]).show(&mut content,|ui|{
            if !body.is_empty() {
                crate::mail::reader::body_ui(ui,body.as_ref(),11.5*c.s);
            }else if item.links.is_empty(){
                ui.label(egui::RichText::new("（这封邮件没有正文）").font(c.font(12.0,W::Regular)).color(theme::DIM));
            }
            for link in &item.links {
                let text=format!("{} ↗",c.fit(&link.label,12.0,W::Regular,295.0));
                if ui.add(egui::Link::new(egui::RichText::new(text).font(c.font(12.0,W::Regular)).color(theme::BLUE))).on_hover_text(&link.url).clicked(){
                    cmds.push(Cmd::OpenUrl(link.url.clone()));
                }
            }
        });

        if !self.mail.manage.message.is_empty(){
            c.text(46.0,229.0,&c.fit(&self.mail.manage.message,8.0,W::Regular,130.0),8.0,W::Regular,theme::GRAY,H::Left);
            c.hit(44.0,219.0,132.0,15.0,"mail-detail-status").on_hover_text(&self.mail.manage.message);
        }
        // 底部一行：会议时间（绿）优先，其次验证码（点一下复制）。
        c.seg((46.0,199.0),(374.0,199.0),0.6,Color32::from_gray(48));
        let y = 214.0;
        if hidden {
            c.text(46.0,y,"验证码已隐藏（不代表失效）",11.0,W::Regular,theme::GRAY,H::Left);
            c.text(374.0,y,"查看 15 秒",10.5,W::Regular,theme::BLUE,H::Right);
            if c.hit(290.0,y-14.0,86.0,24.0,("mail-reveal",item.uid)).clicked(){self.reveal_mail_code(raw);}
        } else if let Some(m) = &item.meeting {
            let ignored=self.mail_event_ignored(m);
            c.text(46.0,y,&format!("{} · {}",if ignored{"已忽略"}else{"日程"},m.at),10.5,W::Regular,if ignored{theme::DIM}else{theme::GREEN},H::Left);
            let label=if ignored{"恢复日程"}else{"忽略日程"};
            c.text(374.0,y,label,10.5,W::Regular,theme::GRAY,H::Right);
            if c.hit(316.0,y-14.0,60.0,24.0,("mail-ignore-event",item.uid)).on_hover_text(&m.title).clicked(){self.toggle_mail_event(m);}
        } else if let Some(code) = &item.code {
            let label = format!("验证码 {code}");
            let w = c.text(46.0, y, &label, 13.0, W::Semibold, theme::GREEN, H::Left);
            c.text(46.0 + w + 6.0, y, "点击复制", 10.5, W::Regular, theme::DIM, H::Left);
            let hit = c.hit(42.0, y - 14.0, w + 60.0, 24.0, ("mail-detail-code", item.uid));
            if hit.clicked() {
                cmds.push(Cmd::Copy(code.clone()));
                self.code_copied(code);
            }
        }
    }
}
