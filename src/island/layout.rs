//! 岛体尺寸、命中区域、展开收起与拖动动效。

pub(crate) mod bar;
pub mod dock;

use super::*;
use crate::music::lyrics_state::lyric_key_matches;

impl App {
    #[cfg(test)]
    pub(crate) fn forced_open_until_for_tests(&self)->f64{self.forced_open_until}
    pub(crate) fn update_island_lyric_layout(&mut self,ctx:&egui::Context) {
        if !self.island_lyrics_active(){self.island_lyric_layout=None;return;}
        let current=self.island_lyric();
        let lyrics=self.ly.lyrics.as_ref().filter(|lyrics|lyric_key_matches(&self.ly.key,&self.media)
            && lyrics.synced && lyrics.lines.iter().any(|line|!line.text.trim().is_empty()));
        let title=lyrics.is_none();
        let text=if let Some((_,line))=current {line}
            else if let Some(lyrics)=lyrics {
                if lyrics.index_at(self.lyric_position()).is_none() {"前奏"}
                else {"间奏"}
            } else {self.media.title.trim()};
        let ink=if current.is_some()||title{theme::WHITE}else{theme::GRAY};
        let layout=|text:String| {
            // 维语等从右往左的歌词：先按阅读顺序分行再逐行转换，否则折行后第一行会是句子的结尾（#200）。
            let font=theme::font(11.0,W::Semibold);
            let text=crate::visuals::rtl_text::display_wrapped(&text,LYRIC_BAR_W-80.0,|s|ctx.fonts(|fonts|fonts.layout_no_wrap(s.to_owned(),font.clone(),ink).size().x)).into_owned();
            let mut job=egui::text::LayoutJob::simple(text,font,ink,LYRIC_BAR_W-80.0);
            job.halign=egui::Align::Center;
            job.wrap.break_anywhere=true;
            if title {job.wrap.max_rows=1;}
            ctx.fonts(|fonts|fonts.layout_job(job))
        };
        let mut galley=layout(text.to_owned());
        if let Some((index,_))=current.filter(|_|galley.rows.len()>1) {
            let lyrics=self.ly.lyrics.as_ref().unwrap();
            let start=lyrics.lines[index].t;
            let end=lyrics.lines.iter().skip(index+1).find(|line|line.t>start).map_or(self.media.duration as f32,|line|line.t);
            let progress=if end>start {((self.lyric_position()-start)/(end-start)).clamp(0.0,1.0)}else{0.0};
            // 按当前句的时间范围与各段字数切换，暂停和跳转直接跟随播放位置。
            let total:usize=galley.rows.iter().map(|row|row.glyphs.len()).sum();
            let mut count=0;
            let page=galley.rows.iter().position(|row|{count+=row.glyphs.len();progress*(total as f32)<count as f32}).unwrap_or(galley.rows.len()-1);
            let segment=galley.rows[page].glyphs.iter().map(|glyph|glyph.chr).collect::<String>();
            galley=layout(segment.trim().to_owned());
        }
        self.island_lyric_layout=Some(galley);
    }

    pub(crate) fn pet_enabled(&self)->bool {self.settings.pet && !self.browser_minimal_active && !self.side()}
    pub(crate) fn pet_space(&self)->f32 {if self.pet_enabled(){44.0}else{0.0}}
    pub(crate) fn lyric_focus_space(&self)->f32 {if self.pomo.running && self.settings.show_focus{crate::focus::bar::LYRIC_WIDTH+10.0}else{0.0}}
    pub(crate) fn lyric_bar_width(&self)->f32 {LYRIC_BAR_W+if self.pet_enabled(){48.0}else{0.0}+self.lyric_focus_space()}
    pub(crate) fn notice_size(&self,a:&Alert)->(f32,f32) {
        let (width,height)=if a.meeting.is_some(){(360.0,136.0)}else if a.source==crate::island::banner::AlertSource::Internal && a.title=="蓝牙设备"{(HUD_W,48.0)}else if a.is_hud(){(HUD_W,38.0)}else{(334.0,44.0)};
        (width,height)
    }
    pub fn set_pet(&mut self,enabled:bool) {
        self.settings.pet=enabled;self.settings.save();
    }
    /// 直接进入或退出窗口最大化避让的细线状态，不需要模拟浏览器窗口。
    pub fn set_minimal(&mut self,on:bool) {
        self.settings.browser_minimal=on;
        self.browser_at_top=on;
        self.browser_minimal_active=on;
    }
    pub(crate) fn base_size(&self) -> (f32, f32) {
        if !self.compact_notices()&&!self.open {
            if let Some(a)=&self.notice {let (width,height)=self.notice_size(a);return (width+self.pet_space(),height);}
        }
        if self.side() {
            // 左右停靠：竖向胶囊（通知仍是贴边滑出的横向卡片，见上）。
            return self.side_base_size(self.browser_minimal_active&&self.t>=self.notification_bell_until&&!self.windows_unread);
        }
        if self.island_lyrics_active() {
            (self.lyric_bar_width(),BAR_H)
        } else if self.browser_minimal_active&&self.t>=self.notification_bell_until&&!self.windows_unread {
            (BAR_W_MIN, BAR_H_MIN)
        } else {
            (self.resident.target.width.min(WIN_W-32.0), BAR_H)
        }
    }

    /// 0 = 药丸，1 = 完整面板（展开时可略超过 1）。
    #[cfg(test)]
    pub(crate) fn current_width(&self) -> f32 { self.w.pos+self.notice_width.pos }

    pub fn progress(&self) -> f32 {
        let (bw, bh) = self.base_size();
        // 左右停靠时面板沿水平方向从边缘滑出，用宽度衡量展开程度。
        if self.side() {return (self.w.pos - bw) / (PW - bw).max(1.0);}
        (self.h.pos - bh) / (self.expanded_height() - bh)
    }

    pub fn cancel_page_scroll(&mut self) {
        self.page_scroll_area=None;
    }

    pub(crate) fn expanded_height(&self)->f32 {
        let content=if self.page==CALENDAR_PAGE {CALENDAR_H}else if self.page==WEATHER_PAGE {WEATHER_H}else if self.page==6 {176.0}else if self.page==WINDOWS_PAGE {176.0}else if self.page==AI_PAGE {self.agent_page_height()}else if self.page==5&&self.mail.open_uid.is_some(){236.0}else{PH};
        content+STATUS_H
    }

    /// AI 会话页按卡片数增高；详情页按待处理类型增高。
    pub(crate) fn agent_page_height(&self)->f32 {
        let list = crate::ai_session::page::agent_list_height(self.agent.store.visible().len(), self.agent.config.display.rows);
        if self.agent.detail {
            match self.agent.selected_session().and_then(|session|session.pending.as_ref().map(|pending|&pending.kind)) {
                Some(crate::ai_session::store::PendingKind::Question(questions)) => {
                    let index = self.agent.question_index.min(questions.len().saturating_sub(1));
                    questions.get(index).map(|question|crate::ai_session::page::question_height(question.options.len())).unwrap_or(list)
                }
                Some(crate::ai_session::store::PendingKind::Approval(_)) => crate::ai_session::page::approval_height(self.agent.approval_lines),
                None => list,
            }
        } else { list }
    }

    // ---------------------------------------------------------------
    // 每帧入口
    // ---------------------------------------------------------------

    pub(crate) fn hit_rect(&self, cx: f32, top: f32, p: f32) -> Rect {
        let r=self.hit_rect_base(cx,top,p);
        // 胶囊离屏幕顶边有一段空隙：把光标推到屏幕最顶端也要能唤出岛。
        if self.settings.island_shape==model::IslandShape::Pill {Rect::from_min_max(Pos2::new(r.left(),r.top()-PILL_GAP),r.max)} else {r}
    }

    pub(crate) fn hit_rect_base(&self, cx: f32, top: f32, p: f32) -> Rect {
        if self.browser_minimal_active && !self.open {
            let (width,height)=self.base_size();
            let width=self.w.pos.min(width);
            let height=self.h.pos.min(height);
            Rect::from_min_size(Pos2::new(cx-width/2.0,top),Vec2::new(width,height))
        } else if self.notice.is_some()&&!self.compact_notices()&&!self.open {
            let (w,h)=self.base_size();Rect::from_min_size(Pos2::new(cx-(w+8.0)/2.0,top),Vec2::new(w+8.0,h+6.0))
        } else if p > 0.04 || self.open {
            Rect::from_min_size(Pos2::new(cx - PW / 2.0 - 4.0, top), Vec2::new(PW + 8.0, self.expanded_height() + 2.0))
        } else {
            let (bw,bh)=self.base_size();
            let (hit_w,hit_h)=(bw+12.0,bh+4.0);
            Rect::from_min_size(Pos2::new(cx-hit_w/2.0,top),Vec2::new(hit_w,hit_h))
        }
    }

    /// 展开后未读胶囊的出现程度：面板基本展开后从面板旁淡入。
    pub(crate) fn messenger_appear(&self)->f32 {
        if self.calendar.draft.is_some() || self.task_dialog.is_some(){return 0.0;}
        smoothstep(0.0,1.0,anim::clamp01((self.progress()-0.7)/0.3))
    }
    pub(crate) fn messenger_stack_appear(&self)->f32 {1.0-smoothstep(0.04,0.30,self.progress())}
    /// 同一来源有多个账号时给图标编号（1、2…）；只有一个账号不编号。
    pub(crate) fn unread_tag(&self,i:usize)->Option<usize>{
        let unread=self.pending_messengers.get(i)?;let source=unread.source;
        if unread.unidentified(){return None;}
        let known=|u:&&Unread|u.source==source && (!source.adapter().keep_unidentified || u.pid!=0);
        let same=self.pending_messengers.iter().filter(known).count();
        (same>1).then(||self.pending_messengers[..=i].iter().filter(known).count())
    }
    pub(crate) fn unread_id(&self,i:usize)->String{
        let name=self.pending_messengers[i].source.name();
        let n=self.pending_messengers[..=i].iter().filter(|u|u.source==self.pending_messengers[i].source).count();
        if n>1{format!("{name}{n}")}else{name.to_string()}
    }
    /// 展开胶囊高度：一个是圆，两个是短胶囊，三个及以上与面板等高（多出的滚动）。
    pub(crate) fn unread_capsule_height(&self,panel_h:f32)->f32 {
        let n=self.unread_count();
        if n>=3 {panel_h} else {4.0+UNREAD_SLOT*n as f32}
    }
    pub(crate) fn unread_top_inset(&self)->f32 {if self.dock()==model::Dock::Top {UNREAD_GAP}else{0.0}}
    pub(crate) fn unread_slot(&self,height:f32)->f32 {
        if self.unread_count()>=3 {(height-4.0)/3.0} else {UNREAD_SLOT}
    }
    pub(crate) fn unread_count(&self)->usize {self.pending_messengers.len()}
    /// 收起时叠放的来源图标，每个客户端各占一个。
    pub(crate) fn unread_sources(&self)->Vec<Messenger>{
        let mut sources=Vec::new();
        for u in &self.pending_messengers {if !sources.contains(&u.source){sources.push(u.source);}}
        sources
    }
    /// 收起时叠放区沿胶囊方向的长度。
    pub(crate) fn unread_stack_len(&self,horizontal:bool)->f32 {
        let icons=self.unread_sources().len() as f32;
        32.0+unread_step(horizontal)*(icons-1.0)
    }

    pub(crate) fn messenger_badge_rect(&self,win:Rect,panel:Rect)->Option<Rect>{
        if self.unread_count()==0 || self.browser_minimal_active && !self.open {return None;}
        // 完整横幅已包含该来源图标。歌词模式和横幅收起后使用常驻入口。
        if !self.open && !self.compact_notices() && self.pending_messengers.len()==1
            && self.notice.as_ref().and_then(|a|a.messenger())==self.pending_messengers.first().map(|u|u.source){return None;}
        let bounds=self.screen.map_or(win,|screen|screen.intersect(win));
        let appear=self.messenger_appear();
        if appear>0.02 {
            // 展开：贴在面板外侧、顶端对齐的竖向胶囊；顶部停靠和左停靠在右边，右停靠在左边。
            let full=self.island_rect(win,PW,self.expanded_height());
            // 顶部停靠时面板贴着屏幕上沿，胶囊往下让一点，圆顶不会被屏幕边切掉。
            let inset=self.unread_top_inset();
            let height=self.unread_capsule_height(full.height()-inset);
            let left=self.dock()==model::Dock::Right || self.dock()==model::Dock::Top && full.right()+UNREAD_GAP+UNREAD_W>bounds.right();
            let width=UNREAD_W*appear;
            let x=if left {panel.left()-UNREAD_GAP*appear-width}else{panel.right()+UNREAD_GAP*appear};
            return Some(Rect::from_min_size(Pos2::new(x,panel.top()+inset),Vec2::new(width,height*appear)));
        }
        // 窗口最大化避让的细线：横幅照常弹出，收起后只留细线，未读不常挂在线条上；展开岛体时在面板旁出现。
        if self.browser_minimal_active{return None;}
        let scale=self.messenger_stack_appear();
        if scale<0.02 || self.open && (self.calendar.draft.is_some() || self.task_dialog.is_some()){return None;}
        let side_column=self.side() && self.notice.is_none();
        let length=self.unread_stack_len(!side_column);
        let (width,height)=if side_column {(32.0,length)}else{(length,BAR_H)};
        let (width,height)=(width*scale,height*scale);
        let (x,y)=if side_column {
            (panel.center().x-width/2.0,panel.bottom())
        }else{
            let right=panel.right()+self.notice_width.pos;
            let y=if self.notice.is_some() && !self.compact_notices(){panel.center().y-BAR_H/2.0}else{panel.top()};
            (if right+width<=bounds.right(){right}else{panel.left()-width},y)
        };
        Some(Rect::from_min_size(Pos2::new(x,y),Vec2::new(width,height)))
    }

    /// 展开后的未读胶囊：每个账号一个图标，点击跳到对应窗口；超过三个时滚轮翻看。
    pub(crate) fn draw_unread_capsule(&mut self,ctx:&egui::Context,c:&Card,rect:Rect,cursor:Option<Pos2>,cmds:&mut Vec<Cmd>){
        let n=self.unread_count();
        let height=rect.height()/c.s;
        let slot=self.unread_slot(height);
        let max_scroll=(n as f32-3.0).max(0.0)*slot;
        if max_scroll>0.0 && cursor.is_some_and(|p|rect.contains(p)) {
            let delta=ctx.input(|i|i.raw_scroll_delta.y);
            self.messenger_scroll-=delta*0.5;
        }
        self.messenger_scroll=self.messenger_scroll.clamp(0.0,max_scroll);
        let c=c.faded(self.messenger_appear());
        let inner=c.clipped(0.0,2.0,UNREAD_W,height-4.0);
        let x=UNREAD_W/2.0;
        for i in 0..self.pending_messengers.len() {
            let u=self.pending_messengers[i];
            let cy=2.0+slot*(i as f32+0.5)-self.messenger_scroll;
            if cy< -slot || cy>height+slot {continue;}
            let scale=u.source.adapter().icon_scale;
            crate::island::banner::messenger_icon(&inner.scaled(x,cy,scale),x,cy,u.source);
            inner.circle(x+10.0,cy-9.0,2.6,Color32::from_rgb(255,72,72));
            let tag=self.unread_tag(i).map(|tag|tag.to_string()).or_else(||(u.unidentified()).then(||"?".to_string()));
            if let Some(tag)=tag{
                inner.circle(x+10.0,cy+8.5,6.0,Color32::from_gray(238));
                inner.centered_text(x+10.0,cy+8.5,&tag,8.5,W::Bold,Color32::from_gray(28),H::Center);
            }
            let name=if u.unidentified(){format!("{} · 账号未确认",u.source.name())}
                else{match self.unread_tag(i){Some(n)=>format!("{} · 账号 {n}",u.source.name()),None=>u.source.name().to_string()}};
            let button=inner.hit(0.0,cy-slot/2.0,UNREAD_W,slot,self.unread_id(i)).on_hover_text(format!("查看{name}"));
            if button.clicked(){cmds.push(Cmd::ViewMessenger(u));}
        }
        if max_scroll>0.0 {
            // 还有没露出的图标：上下边缘各一条淡淡的指示。
            let ink=paint::alpha(theme::WHITE,0.35);
            if self.messenger_scroll>0.5 {c.fill(x-6.0,4.0,12.0,2.0,1.0,ink);}
            if self.messenger_scroll<max_scroll-0.5 {c.fill(x-6.0,height-6.0,12.0,2.0,1.0,ink);}
        }
    }

    /// 收起时的未读入口：每个来源一个图标加小红点（同一来源多个账号合成一个）。只有一条时点它直接跳转，多条时展开面板挑选。
    pub(crate) fn draw_unread_stack(&mut self,c:&Card,horizontal:bool,cmds:&mut Vec<Cmd>){
        let sources=self.unread_sources();
        let count=self.pending_messengers.len();
        let icons=sources.len();
        let step=unread_step(horizontal);
        let at=|k:usize|if horizontal {(16.0+step*k as f32,14.0)}else{(16.0,14.0+step*k as f32)};
        for k in 0..sources.len() {
            let (x,y)=at(k);
            let scale=sources[k].adapter().collapsed_icon_scale;
            crate::island::banner::messenger_icon(&c.scaled(x,y,scale),x,y,sources[k]);
        }
        // 每个来源图标右上角一个小红点，不显示数字。
        for k in 0..icons {let (x,y)=at(k);c.circle(x+8.0,y-7.0,2.6,Color32::from_rgb(255,59,48));}
        let length=self.unread_stack_len(horizontal);
        let (w,h)=if horizontal {(length,BAR_H)}else{(32.0,length)};
        if count==1 {
            let u=self.pending_messengers[0];
            let button=c.hit(0.0,0.0,w,h,u.source.name()).on_hover_text(format!("查看{}",u.source.name()));
            if button.clicked(){cmds.push(Cmd::ViewMessenger(u));}
        }else if count>1 {
            let button=c.hit(0.0,0.0,w,h,"messenger-stack").on_hover_text(format!("{count} 条未读 · 展开挑选"));
            if button.clicked(){self.reveal_unread();}
        }
    }
    /// 展开面板，让旁边的未读胶囊露出来。
    pub(crate) fn reveal_unread(&mut self){
        self.notice=None;
        self.mark_open(true); self.forced_open_until=self.t+self.settings.page_hold_secs;
        let k=self.settings.motion_speed.factor().powi(2);
        self.w.tune(OPEN_W.0*k,OPEN_W.1);self.h.tune(OPEN_H.0*k,OPEN_H.1);
        self.w.target=PW;self.h.target=self.expanded_height();
    }
    pub(crate) fn notice_rect(&self,cx:f32,top:f32)->Option<Rect>{
        if !self.compact_notices(){return None;}
        let width=self.notice_width.pos-8.0;
        (width>0.5).then(||Rect::from_min_size(Pos2::new(cx+self.w.pos/2.0+8.0,top),Vec2::new(width,BAR_H)))
    }
    pub(crate) fn notice_details_rect(&self,cx:f32,top:f32)->Option<Rect>{
        let a=self.hud_notice.as_ref().filter(|_|self.notice_details&&self.notice.is_some())?;
        let badge=self.notice_rect(cx,top)?;let (width,height)=self.notice_size(a);
        let mut x=badge.left();
        // 岛体靠近屏幕右缘时，详情卡片向左展开，始终完整留在屏幕内。
        if let Some(screen)=self.screen {x=x.min(screen.right()-width-8.0).max(screen.left()+8.0);}
        Some(Rect::from_min_size(Pos2::new(x,badge.bottom()+6.0),Vec2::new(width,height)))
    }

    /// 当前皮肤的换色调色板（经典为 None）。
    pub(crate) fn skin_palette(&self)->Option<crate::visuals::skin_paint::Palette> {
        match self.settings.skin {model::Skin::Cream=>Some(crate::visuals::skin_paint::CREAM),model::Skin::GlassLight=>Some(crate::visuals::skin_paint::GLASS_LIGHT),model::Skin::GlassDark=>Some(crate::visuals::skin_paint::GLASS_DARK),model::Skin::Classic|model::Skin::Fluid=>None}
    }

    /// 设置页预览用的岛体底色：没有真实模糊，毛玻璃按较实的玻璃表现。
    pub(crate) fn preview_fill(&self)->Color32 {
        match self.settings.skin {
            model::Skin::GlassDark=>{let (r,g,b)=glass::GROUND_DARK;Color32::from_rgba_unmultiplied(r,g,b,215)}
            model::Skin::GlassLight=>{let [r,g,b]=glass::LIGHT_GROUND.map(|v|(v*255.0).round() as u8);Color32::from_rgba_unmultiplied(r,g,b,225)}
            _=>self.glass_fill(),
        }
    }

    /// 岛体底色：经典为纯黑；奶油为暖白；毛玻璃按当前着色量叠加深色或浅色玻璃。
    pub(crate) fn glass_fill(&self)->Color32 {
        let alpha=(self.glass_tint.clamp(0.0,1.0)*255.0).round() as u8;
        match self.settings.skin {
            model::Skin::GlassDark=>{let (r,g,b)=glass::GROUND_DARK;Color32::from_rgba_unmultiplied(r,g,b,alpha)}
            model::Skin::GlassLight=>{let [r,g,b]=glass::LIGHT_GROUND.map(|v|(v*255.0).round() as u8);Color32::from_rgba_unmultiplied(r,g,b,alpha)}
            model::Skin::Cream=>{let [r,g,b]=crate::visuals::skin_paint::CREAM.paper;Color32::from_rgb(r,g,b)}
            model::Skin::Classic=>Color32::BLACK,
            model::Skin::Fluid=>self.fluid.compose(0.5,0.5,1.0,1.0,0.0),
        }
    }

    /// 毛玻璃边缘高光：浅色玻璃更亮，像苹果的玻璃边缘。
    pub(crate) fn glass_rim(&self)->Color32 {
        let strength=if self.settings.skin==model::Skin::GlassLight {0.55} else {glass::rim_alpha()*0.7};
        Color32::from_white_alpha((255.0*strength).round() as u8)
    }

    /// 绘制岛体（或通知胶囊）的本体：经典皮肤为纯黑；毛玻璃为着色底 + 顶部柔光 + 边缘高光。
    pub(crate) fn paint_body(&self,painter:&egui::Painter,rect:Rect,flare:f32,flare_h:f32,radius:f32,feather:f32,edge:f32){
        let skin=self.settings.skin;
        if skin==model::Skin::Fluid {
            let pill=self.settings.island_shape==model::IslandShape::Pill;
            let expand=self.progress().clamp(0.0,1.0);
            // 形变期间只铺平色，落定后再把网格淡入，避免每帧重算流体。
            let flat=self.fluid.compose(0.5,0.5,1.0,1.0,expand);
            if pill {
                painter.rect_filled(rect,radius.min(rect.height().min(rect.width())*0.5),flat);
            } else if self.dock().is_side() {
                shape::fill_docked(painter,rect,self.dock(),flare,flare_h,radius,flat,feather);
            } else {
                shape::fill(painter,rect,flare,flare_h,radius,flat,feather);
            }
            if edge<=0.001 {return;}
            let (map,frame)=shape::EdgeMap::of(rect,if pill {model::Dock::Top}else{self.dock()});
            let mut rows=shape::sections(frame,flare,flare_h,radius);
            if pill {
                let mid=frame.center().y;
                rows.retain(|row|row.0>mid && row.0>=frame.bottom()-radius);
                let top=rows.iter().rev().map(|&(y,l,r)|(2.0*mid-y,l,r)).collect::<Vec<_>>();
                rows.splice(0..0,top);
            }
            let mut mesh=shape::shade_mesh(frame,rows,feather,&|x,y|{
                let p=map.apply(Pos2::new(x,y))-rect.min;
                self.fluid.compose(p.x,p.y,rect.width(),rect.height(),expand)
            });
            if self.dock().is_side() && !pill {
                for vertex in &mut mesh.vertices {vertex.pos=map.apply(vertex.pos);}
            }
            for vertex in &mut mesh.vertices {vertex.color=paint::alpha(vertex.color,edge);}
            painter.add(egui::Shape::mesh(mesh));
            return;
        }
        if self.settings.island_shape==model::IslandShape::Pill {
            let radius=radius.min(rect.height().min(rect.width())*0.5);
            painter.rect_filled(rect,radius,self.glass_fill());
            if skin.is_glass() && rect.height()>=10.0 {
                painter.rect_stroke(rect,radius,egui::Stroke::new(1.0_f32,self.glass_rim()),egui::StrokeKind::Inside);
            }
            return;
        }
        let dock=self.dock();
        if dock.is_side() {
            shape::fill_docked(painter,rect,dock,flare,flare_h,radius,self.glass_fill(),feather);
            if !skin.is_glass() {return;}
            if skin==model::Skin::GlassLight {
                let sheen=Color32::from_white_alpha((15.0*edge) as u8);
                if edge>0.001 {shape::fill_gradient_docked(painter,rect,dock,flare,flare_h,radius,sheen,Color32::TRANSPARENT,(rect.width()*0.6).min(64.0),feather);}
            }
            if rect.height()>=20.0 && rect.width()>=10.0 {
                let edge=if dock==model::Dock::Left {Rect::from_min_max(rect.min+Vec2::new(0.0,0.5),rect.max-Vec2::new(0.5,0.5))}
                    else {Rect::from_min_max(rect.min+Vec2::new(0.5,0.5),rect.max-Vec2::new(0.0,0.5))};
                painter.add(egui::Shape::line(shape::outline_docked(edge,dock,flare,flare_h,(radius-0.5).max(0.0)),egui::Stroke::new(1.0_f32,self.glass_rim())));
            }
            return;
        }
        shape::fill(painter,rect,flare,flare_h,radius,self.glass_fill(),feather);
        if !skin.is_glass() {return;}
        // 玻璃厚度带来的上亮下暗：顶部白光在前 60% 高度（最多 64 点）内淡出；浅色玻璃更亮。
        if skin==model::Skin::GlassLight {
            let sheen=Color32::from_white_alpha((15.0*edge) as u8);
            if edge>0.001 {shape::fill_gradient(painter,rect,flare,flare_h,radius,sheen,Color32::TRANSPARENT,(rect.height()*0.6).min(64.0),feather);}
        }
        if rect.height()>=10.0 && rect.width()>=20.0 {
            let edge=Rect::from_min_max(rect.min+Vec2::new(0.5,0.0),rect.max-Vec2::new(0.5,0.5));
            painter.add(egui::Shape::line(shape::outline(edge,flare,flare_h,(radius-0.5).max(0.0)),egui::Stroke::new(1.0_f32,self.glass_rim())));
        }
    }

    /// 拿起时沿岛体轮廓的一圈柔光：外圈宽而淡，内圈细而亮。
    pub(crate) fn paint_lift_halo(&self,painter:&egui::Painter,rect:Rect,flare:f32,flare_h:f32,radius:f32,amount:f32) {
        if rect.width()<10.0 || rect.height()<10.0 {return;}
        let pill=self.settings.island_shape==model::IslandShape::Pill;
        let dock=self.dock();
        for (grow,width,alpha) in [(2.0f32,4.0f32,22.0f32),(0.5,1.2,90.0)] {
            let stroke=egui::Stroke::new(width,Color32::from_white_alpha((alpha*amount) as u8));
            let edge=rect.expand(grow);
            if pill {
                let r=radius.min(rect.height().min(rect.width())*0.5)+grow;
                painter.rect_stroke(edge,r,stroke,egui::StrokeKind::Middle);
            } else if dock.is_side() {
                painter.add(egui::Shape::line(shape::outline_docked(edge,dock,flare,flare_h,radius+grow),stroke));
            } else {
                painter.add(egui::Shape::line(shape::outline(edge,flare,flare_h,radius+grow),stroke));
            }
        }
    }

    pub(crate) fn update_position_drag(&mut self,ctx:&egui::Context,env:&Env,dt:f32) {
        let limit=model::ISLAND_OFFSET_MAX as f32;
        if let Some(mut drag)=self.position_drag {
            let mut speed=0.0;
            if let (Some(cursor),Some(screen))=(env.cursor,env.screen) {
                // 使用屏幕内坐标，窗口移动或横竖画布切换不会累积位移。
                let cursor=cursor-screen.min;
                let edges=[(model::Dock::Top,cursor.y.max(0.0)),(model::Dock::Left,cursor.x.max(0.0)),(model::Dock::Right,(screen.width()-cursor.x).max(0.0))];
                let &(nearest,distance)=edges.iter().min_by(|a,b|a.1.total_cmp(&b.1)).unwrap();
                let current=edges.iter().find(|(dock,_)|*dock==self.dock()).unwrap().1;
                // 靠近新边缘才换边；角落保留缓冲，防止横竖形态来回跳变。
                let mut target=Some(nearest).filter(|dock|*dock!=self.dock() && distance<=64.0 && distance+24.0<current);
                // 左右对侧：过了中线、明显更靠近另一侧就换过去，不必甩到屏幕尽头；中线附近留缓冲不来回跳。
                if target.is_none() && self.side() {
                    let (opposite,far)=if self.dock()==model::Dock::Left {(model::Dock::Right,edges[2].1)}else{(model::Dock::Left,edges[1].1)};
                    if far+screen.width()*0.07<current {target=Some(opposite);}
                }
                let switch=target.is_some();
                if let Some(dock)=target {self.settings.dock=dock;}
                let (position,length,extent,start)=if self.side() {(cursor.y,screen.height(),crate::island::layout::dock::SIDE_HALF_EXTENT,drag.start.y)}
                    else {(cursor.x,screen.width(),crate::os::display_layout::ISLAND_HALF_EXTENT,drag.start.x)};
                let travel=(length/2.0-extent).max(0.0);
                let raw=if travel<=0.0 {0.0}else if switch {(position-length/2.0)/travel*limit}
                    else {drag.offset+(position-start)/travel*limit};
                let raw=raw.clamp(-limit,limit);
                // 居中和拖动前的位置有轻微吸附，窗口弹簧会把这一下「吸过去」。
                // 居中带滞回：进入范围小、松开范围大；刚吸住时岛体轻轻一沉，像卡进了槽位。
                let centered=raw.abs()<=if self.center_locked {CENTER_RELEASE}else{DRAG_MAGNET};
                if centered && !self.center_locked {
                    self.center_lock_at=self.t;
                    self.drag_lift.vel-=2.8;
                }
                self.center_locked=centered;
                let original=Some(drag.original.1 as f32).filter(|_|self.dock()==drag.original.0);
                let target=if centered {0.0}else{original.filter(|p|(raw-p).abs()<=DRAG_MAGNET).unwrap_or(raw)};
                self.settings.island_offset=target.round() as i32;
                let glide=self.drag_glide.get_or_insert_with(||anim::Spring::new(drag.offset,GLIDE_FOLLOW.0,GLIDE_FOLLOW.1));
                glide.target=target;
                if switch {
                    // 换边时画布横竖互换，位置和外形直接到位，避免从旧边缘滑过去。
                    glide.snap(target);
                    drag.start=cursor;drag.offset=raw;
                    let (bw,bh)=self.base_size();
                    self.w.snap(bw);self.h.snap(bh);
                    self.drag_stretch.snap(0.0);
                }
                if travel>0.0 {speed=self.drag_glide.map_or(0.0,|g|g.vel.abs()/limit*travel);}
            }
            self.drag_stretch.target=(speed/STRETCH_SPEED).min(1.0)*STRETCH_MAX;
            if !ctx.input(|i|i.pointer.primary_down()) {
                self.position_drag=None;
                self.hover_lock=true;
                self.drag_stretch.target=0.0;
                if let Some(glide)=&mut self.drag_glide {
                    glide.tune(GLIDE_SETTLE.0,GLIDE_SETTLE.1);
                    glide.target=self.settings.island_offset as f32;
                }
                if (self.dock(),self.settings.island_offset)!=drag.original {self.settings.save();}
            }else{
                self.position_drag=Some(drag);
            }
        }
        self.drag_lift.target=if self.position_drag.is_some() {1.0}else{0.0};
        self.drag_lift.step(dt);
        self.drag_stretch.step(dt);
        if let Some(glide)=&mut self.drag_glide {
            glide.step(dt);
            if self.position_drag.is_none() && glide.settled(limit) {self.drag_glide=None;}
        }
        // 描边：拖动中按离居中的距离渐显；松手后稍作停留再淡出，让人确认落在了中间。
        let dragging=self.position_drag.is_some();
        let distance=self.drag_glide.map_or(f32::INFINITY,|g|g.pos.abs());
        let want=if dragging {1.0-smoothstep(1.5,CENTER_GUIDE_RANGE,distance)}else{0.0};
        self.center_guide=anim::approach(self.center_guide,want,if dragging {18.0}else{5.0},dt);
        let lit=self.center_locked && (dragging || self.center_guide>0.05);
        self.center_lock_a=anim::approach(self.center_lock_a,if lit {1.0}else{0.0},16.0,dt);
        if !dragging && self.center_guide<0.002 {self.center_locked=false;}
    }

    /// 拖动接近居中时由岛体轮廓和一根中轴细线提示，不用文字：
    /// 越靠近居中，轮廓浮出越清晰的淡白描边，屏幕中轴细线与岛体中心刻度逐渐重合；
    /// 吸住瞬间描边和细线一起转为绿色、一道亮光从中心沿轮廓向两端扫开，停在中间期间绿边缓慢呼吸。
    /// 返回需要留在窗口区域内的范围。
    pub(crate) fn paint_center_rim(&self,painter:&egui::Painter,body:Rect,panel:Rect,flare:f32,flare_h:f32,radius:f32,screen:Option<Rect>)->Option<Rect> {
        let sweep=((self.t-self.center_lock_at)/CENTER_SWEEP) as f32;
        let sweeping=(0.0..1.0).contains(&sweep);
        let vis=self.center_guide;
        if (vis<0.004 && !sweeping) || body.width()<10.0 || body.height()<10.0 {return None;}
        let dock=self.dock();
        let side=dock.is_side();
        let along=if side {Vec2::Y}else{Vec2::X};
        let pill=self.settings.island_shape==model::IslandShape::Pill;
        let outline=|grow:f32| if pill {round_rect_points(body.expand(grow),radius+grow)}
            else if side {shape::outline_docked(body.expand(grow),dock,flare,flare_h,radius+grow)}
            else {shape::outline(body.expand(grow),flare,flare_h,radius+grow)};
        // 颜色即状态：接近时中性白，吸住后过渡为绿色。
        let tint=|mix:f32,alpha:f32| {
            let c=|w:f32,a:f32|(lerp(w,a,mix)) as u8;
            Color32::from_rgba_unmultiplied(c(255.0,CENTER_TINT.0),c(255.0,CENTER_TINT.1),c(255.0,CENTER_TINT.2),(alpha.clamp(0.0,1.0)*255.0) as u8)
        };
        let lock=self.center_lock_a;
        let points=outline(0.5);
        if vis>0.004 {
            // 停在中间时绿边以 1.6 秒为周期缓慢呼吸，持续表明「就在中间」。
            let breathe=0.5+0.5*((self.t*std::f64::consts::TAU/1.6).sin() as f32);
            let strength=vis*lerp(0.32,lerp(0.78,1.0,breathe),lock);
            if lock>0.01 {painter.add(egui::Shape::line(outline(2.0),egui::Stroke::new(5.0,tint(1.0,0.22*strength*lock))));}
            painter.add(egui::Shape::line(points.clone(),egui::Stroke::new(lerp(1.0,1.6,lock),tint(lock,0.95*strength))));
        }
        let mut bounds=body.expand(8.0);
        // 中轴细线：屏幕中轴一条向外渐隐的细线，岛体中心一截短刻度，两者重合即居中；吸住后细线同样转绿。
        // 细线画在桌面上，底色未知：彩芯加淡黑描边，深浅壁纸都看得清。
        if let Some(screen)=screen.filter(|_|vis>0.004) {
            let travel=if side {screen.height()/2.0-crate::island::layout::dock::SIDE_HALF_EXTENT}else{screen.width()/2.0-crate::os::display_layout::ISLAND_HALF_EXTENT}.max(0.0);
            // 屏幕中轴相对岛体中心的位置（与窗口跟随同源，不会差一帧）。
            let off=self.placement_offset()*travel;
            let (out,edge)=match dock {
                model::Dock::Top=>(Vec2::Y,Pos2::new(panel.center().x,panel.bottom())),
                model::Dock::Left=>(Vec2::X,Pos2::new(panel.right(),panel.center().y)),
                model::Dock::Right=>(-Vec2::X,Pos2::new(panel.left(),panel.center().y)),
            };
            let start=edge+out*6.0;
            let axis=start-along*off;
            let length=lerp(24.0,32.0,lock);
            let line=|from:Pos2,len:f32,strength:f32,width:f32,mix:f32| {
                const N:usize=12;
                for i in 0..N {
                    let (a,b)=(i as f32/N as f32,(i+1) as f32/N as f32);
                    let fade=(1.0-(a+b)*0.5).powf(1.3)*strength;
                    if fade<0.004 {continue;}
                    let (p,q)=(from+out*len*a,from+out*len*b);
                    painter.line_segment([p,q],egui::Stroke::new(width+2.0,Color32::from_black_alpha((40.0*fade) as u8)));
                    if mix>0.01 {painter.line_segment([p,q],egui::Stroke::new(width+3.0,tint(1.0,0.10*fade*mix)));}
                    painter.line_segment([p,q],egui::Stroke::new(width,tint(mix,0.80*fade)));
                }
            };
            // 细线只做辅助：比描边更短更淡，吸住后也只转成偏浅的绿。
            line(axis,length,vis*lerp(0.40,0.70,lock),lerp(1.0,1.3,lock),lock*0.7);
            if off.abs()>0.5 {line(start,8.0,vis*0.65*(1.0-lock),1.0,0.0);}
            bounds=bounds.union(Rect::from_two_pos(start,axis+out*length).expand(6.0));
        }
        // 扫光：吸住瞬间从岛体中心沿轮廓向两端对称扫开，扫到两端时淡尽。
        if sweeping {
            let half=if side {body.height()}else{body.width()}*0.5;
            let front=anim::ease_out_cubic(sweep)*(half+12.0);
            let fade=(1.0-sweep).powf(1.2);
            let mid=body.center();
            for pair in points.windows(2) {
                let m=pair[0]+(pair[1]-pair[0])*0.5;
                let d=(m-mid).dot(along).abs();
                let g=(-((d-front)/18.0).powi(2)).exp()*fade;
                if g<0.01 {continue;}
                painter.line_segment([pair[0],pair[1]],egui::Stroke::new(6.0,tint(1.0,0.30*g)));
                painter.line_segment([pair[0],pair[1]],egui::Stroke::new(1.8,tint(0.45,0.95*g)));
            }
        }
        Some(bounds)
    }

    /// 拖动中和落位期间窗口使用的位置（-1 … 1，不取整）；其余时候就是保存的位置。
    pub fn placement_offset(&self)->f32 {
        let limit=model::ISLAND_OFFSET_MAX as f32;
        self.drag_glide.map_or(self.settings.island_offset_ratio(),|g|(g.pos/limit).clamp(-1.0,1.0))
    }

    /// 拿起与速度拉伸带来的外形缩放：(宽, 高)。
    pub(crate) fn drag_scale(&self)->(f32,f32) {
        let lift=self.drag_lift.pos.max(0.0);let stretch=self.drag_stretch.pos.max(0.0);
        let along=1.0+LIFT_SCALE.0*lift+stretch;
        let across=1.0+LIFT_SCALE.1*lift-stretch*0.5;
        if self.side() {(across,along)}else{(along,across)}
    }

    pub(crate) fn start_position_drag(&mut self,start:Pos2,screen:Rect) {
        let offset=self.settings.island_offset;
        self.position_drag=Some(PositionDrag{start:start-screen.min,offset:offset as f32,original:(self.dock(),offset)});
        let mut glide=anim::Spring::new(offset as f32,GLIDE_FOLLOW.0,GLIDE_FOLLOW.1);
        if let Some(g)=self.drag_glide {glide.pos=g.pos;glide.vel=g.vel;}
        self.drag_glide=Some(glide);
        // 本来就在中间：直接显示已居中，不重复扫光。
        self.center_locked=offset==0;
        // 拿起时给一点初速度，按下后立刻有反馈。
        self.drag_lift.vel=self.drag_lift.vel.max(4.0);
    }

    /// 记录展开/收起的状态切换时刻；内容淡入按时间而不是尺寸驱动。
    pub(crate) fn mark_open(&mut self, open: bool) {
        if open == self.open { return; }
        if open {
            self.open_at = self.t - (0.12 + 0.24 * self.reveal as f64);
            self.close_at = -10.0;
        } else {
            self.close_at = self.t;
        }
        self.open = open;
    }

    /// 内容淡入：展开后 120 ms 开始、240 ms 完成；收起后 120 ms 内淡出。
    pub(crate) fn content_reveal(&self) -> f32 {
        if self.open {
            smoothstep(0.12, 0.36, (self.t - self.open_at) as f32)
        } else {
            1.0 - smoothstep(0.0, 0.12, (self.t - self.close_at) as f32)
        }
    }

    /// 外壳是否仍在形变：期间内容轮询和重绘可以降级。
    pub(crate) fn shell_moving(&self) -> bool {
        !self.w.settled(PW) || !self.h.settled(self.expanded_height())
    }

    /// 悬停状态机：意图缓冲 → 弹簧展开 → 离开延迟 → 临界阻尼收起；全程可打断。
    pub(crate) fn morph(&mut self, want: bool, inside: bool, t: f64, dt: f32) {
        let (bw, bh) = self.base_size();
        let k=self.settings.motion_speed.factor().powi(2);
        if self.position_drag.is_some() {
            // 拿起后面板快速收成常驻形态；放大效果由 `drag_scale` 叠加在外形上。
            self.mark_open(false);
            self.hover_lock=true;
            self.hover_since=None;
            self.leave_since=None;
            self.forced_open_until=0.0;
            self.music_volume_open=false;
            self.w.tune(CLOSE.0*k, CLOSE.1);
            self.h.tune(CLOSE.0*k, CLOSE.1);
            self.w.target=bw;
            self.h.target=bh;
        } else if want {
            self.leave_since = None;
            let since = *self.hover_since.get_or_insert(t);
            let intent=if self.browser_minimal_active {MINIMAL_HOVER_INTENT}else{HOVER_INTENT};
            if !self.open && (self.drag_hot || t - since >= intent) {
                self.mark_open(true);
                self.w.tune(OPEN_W.0*k, OPEN_W.1);
                self.h.tune(OPEN_H.0*k, OPEN_H.1);
                self.w.target = PW;
                self.h.target = self.expanded_height();
                if self.drag_hot {
                    // 拖入不等缓冲，直接给一点初速度，30 ms 内就能看到形变
                    self.w.vel = self.w.vel.max(600.0);
                    self.h.vel = self.h.vel.max(260.0);
                }
            } else if !self.open {
                self.w.tune(SWELL_K.0*k, SWELL_K.1);
                self.h.tune(SWELL_K.0*k, SWELL_K.1);
                // 悬停时轻微胀大：厚度方向胀得多一点（侧边停靠时厚度是宽度）。
                let (sw,sh)=if self.side(){(SWELL.1,SWELL.0)}else{SWELL};
                self.w.target = bw * sw;
                self.h.target = bh * sh;
            }
        } else {
            self.hover_since = None;
            if self.open {
                let since = *self.leave_since.get_or_insert(t);
                if t - since >= LEAVE_DELAY {
                    self.mark_open(false);
                    self.music_volume_open=false;
                    self.leave_since = None;
                    self.w.tune(CLOSE.0*k, CLOSE.1);
                    self.h.tune(CLOSE.0*k, CLOSE.1);
                    self.w.target = bw;
                    self.h.target = bh;
                    self.cancel_edit();
                    self.confirm_clear = None;
                }
            } else {
                self.w.tune(CLOSE.0*k, CLOSE.1);
                self.h.tune(CLOSE.0*k, CLOSE.1);
                self.w.target = bw;
                self.h.target = bh;
            }
        }
        let _ = inside;
        if self.open {self.h.target=self.expanded_height();}
        let (prev_w,prev_h)=(self.w.pos,self.h.pos);
        self.w.step(dt);
        self.h.step(dt);
        // 仅在从外侧越过目标时收住，扩展提示条时不强制跳到目标宽度。
        if !self.open {
            if prev_w>=bw && self.w.pos<bw {self.w.snap(bw);}
            if prev_h>=bh && self.h.pos<bh {self.h.snap(bh);}
        }
        for d in &mut self.dots {
            d.step(dt);
        }
        self.slide.step(dt);
    }

    // ---------------------------------------------------------------
    // 常驻状态行：电池、时钟、额度、天气、音乐、事件容器、页面徽标、
    // AI 状态灯和铃铛。收起时就是药丸，展开后固定在面板顶部。
    // ---------------------------------------------------------------
}
