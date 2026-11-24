//! 音乐页：封面、进度、播放控制、音量与歌词。

use crate::island::{App, Cmd, Env};
use crate::music::Transport;
use crate::visuals::anim::lerp;
use crate::visuals::paint::{self, capsule, Card, H};
use crate::visuals::theme::{self, size, W};
use eframe::egui::{self, Color32, Pos2, Rect};

pub(crate) const MUSIC_VOLUME_RECT: Rect = Rect::from_min_max(Pos2::new(345.0,6.0),Pos2::new(389.0,104.0));

fn mmss(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// 由标题生成稳定的封面渐变色。
fn cover_colors(title: &str) -> (Color32, Color32) {
    let mut h: u64 = 1469598103934665603;
    for b in title.bytes() {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    let palette = [
        (Color32::from_rgb(255, 94, 98), Color32::from_rgb(255, 153, 102)),
        (Color32::from_rgb(86, 204, 242), Color32::from_rgb(47, 128, 237)),
        (Color32::from_rgb(168, 85, 247), Color32::from_rgb(236, 72, 153)),
        (Color32::from_rgb(52, 211, 153), Color32::from_rgb(16, 120, 200)),
        (Color32::from_rgb(251, 191, 36), Color32::from_rgb(239, 68, 68)),
    ];
    palette[(h % palette.len() as u64) as usize]
}

impl App {
    // ================= 音乐 =================
    pub(crate) fn page_music(&mut self, c: &Card, _env: &Env, cmds: &mut Vec<Cmd>) {
        let np=self.media.clone();
        let has=!np.title.is_empty();
        let with_lyrics=has && self.show_lyrics;
        let ctx=c.ui.ctx().clone();
        let playing_t=ctx.animate_bool_with_time(egui::Id::new("np-playing"),np.playing,0.22);
        let art_size=if with_lyrics {44.0}else{62.0};
        let art_y=if with_lyrics {23.0}else{13.0};
        if let Some((image,texture))=&self.music_artwork {
            let [w,h]=image.size;
            let (uw,vh)=if w>h {(h as f32/w as f32,1.0)}else{(1.0,w as f32/h as f32)};
            let uv=Rect::from_center_size(Pos2::new(0.5,0.5),egui::vec2(uw,vh));
            c.painter.add(egui::epaint::RectShape::filled(c.rect(44.0,art_y,art_size,art_size),10.0*c.s,Color32::WHITE).with_texture(texture.id(),uv));
        } else if has {
            let (a,b)=cover_colors(&np.title);
            let cc=c.clipped(44.0,art_y,art_size,art_size);
            cc.vgrad(44.0,art_y,art_size,art_size,10.0,a,b);
            paint::note(&cc,44.0+art_size/2.0,art_y+art_size/2.0,art_size*0.39,Color32::from_white_alpha(235));
        } else {
            c.fill(44.0,13.0,62.0,62.0,13.0,Color32::from_rgb(25,25,27));
            paint::note(c,75.0,44.0,26.0,Color32::from_rgb(110,110,114));
        }

        if !has {
            c.text(121.0,47.0,"暂无音乐",14.0,W::Bold,theme::WHITE,H::Left);
        } else {
            let text_x=if with_lyrics {99.0}else{121.0};
            let text_width=if with_lyrics {123.0}else{220.0};
            let title_size=if with_lyrics {12.0}else{14.0};
            c.text(text_x,if with_lyrics {41.0}else{53.0},&c.fit(&np.title,title_size,W::Bold,text_width),title_size,W::Bold,theme::WHITE,H::Left);
            c.text(text_x,if with_lyrics {57.0}else{67.0},&c.fit(&np.artist,size::BODY,W::Regular,text_width),size::BODY,W::Regular,theme::GRAY,H::Left);
            if with_lyrics {self.draw_music_lyrics(c);}
        }

        let tc=if has {theme::WHITE}else{Color32::from_rgb(78,78,82)};
        let (bx,bw,by)=if with_lyrics {(44.0,176.0,82.0)}else{(96.0,221.0,92.6)};
        let bar=c.hit_drag(bx-4.0,by-7.0,bw+8.0,14.0,"m-seek");
        let seek_position=if has && np.duration>0.0 && (bar.is_pointer_button_down_on() || bar.clicked() || bar.drag_stopped()) {
            bar.interact_pointer_pos().or_else(||ctx.input(|i|i.pointer.interact_pos()))
                .map(|pointer|((c.to_local(pointer).x-bx)/bw).clamp(0.0,1.0) as f64*np.duration)
        }else{None};
        // 松手后播放器的新位置要等下一轮采样才回来（QQ 音乐尤其慢），先用目标值顶着，否则滑块会先弹回旧位置再跳过去。
        let pending_position=self.music_seek_ui.as_ref().filter(|(_,title)|*title==np.title).map(|(target,_)|*target);
        // 切到上一首 / 下一首后先本地归零：新歌位置要等下一轮采样，否则会拿旧歌进度配新歌名停一下。
        let restart=self.music_restart_at.filter(|at|self.t<at+1.5).map(|at|(self.t-at).max(0.0));
        let position=seek_position.or(pending_position).unwrap_or_else(||match restart {
            Some(fresh) if np.position>fresh+1.0=>fresh,
            _=>np.position,
        });
        let frac=if np.duration>0.0 {(position/np.duration).clamp(0.0,1.0) as f32}else{0.0};
        let (time_y,time_size,right)=if with_lyrics {(100.0,10.0,220.0)}else{(97.0,15.0,375.0)};
        c.text(44.0,time_y,&mmss(position),time_size,W::Regular,tc,H::Left);
        let remaining=if has && np.duration<=0.0{"--:--".into()}else{format!("-{}",mmss((np.duration-position).max(0.0)))};
        c.text(right,time_y,&remaining,time_size,W::Regular,tc,H::Right);
        let (bh,_)=c.hover_t(&bar);
        let bh=if has {bh}else{0.0};
        let th=lerp(3.5,5.0,bh);
        c.fill(bx,by-th/2.0,bw,th,th/2.0,if has {Color32::from_gray(90)}else{theme::TRACK});
        if self.settings.skin==crate::model::Skin::Fluid {
            let head=bx+bw*frac;
            let hot=if bar.is_pointer_button_down_on(){1.0}else if np.playing{0.62}else{0.45};
            if has && frac>0.0 {
                c.fill(bx,by-th/2.0,bw*frac,th,th/2.0,self.fluid.tint_color(0.78));
                paint::chase(c,bx,head,by,th,self.fluid.tint_color(0.30),hot);
            }
            if c.live {
                let pos=c.pos(head,by)-self.fluid_origin;
                self.fluid.set_seek((has && frac>0.0).then_some((pos.x,pos.y)),hot);
            }
        }else{
        if frac>0.0 {c.fill(bx,by-th/2.0,(bw*frac).max(th),th,th/2.0,theme::WHITE);}
        if bh>0.01 {c.faded(bh).circle(bx+bw*frac,by,4.5,theme::WHITE);}
        }
        if bar.clicked() || bar.drag_stopped() {
            if let Some(position)=seek_position {
                self.music_seek_ui=Some((position,np.title.clone()));
                self.music_seek_quiet_until=self.t+2.5;
                cmds.push(Cmd::Media(Transport::Seek(position)));
            }
        }

        let y=118.0;
        let ic=if has {theme::WHITE}else{Color32::from_rgb(88,88,92)};
        let btn=|x:f32,w:f32,id:&str| {
            let response=c.hit(x-w/2.0,y-12.0,w,24.0,id);
            let (hover,pressed)=c.hover_t(&response);
            if has && hover>0.01 {c.faded(hover).circle(x,y,12.5,Color32::from_white_alpha(20));}
            (response,c.scaled(x,y,1.0-0.1*pressed))
        };
        let (lyrics_button,lyrics_card)=btn(74.0,26.0,"m-lyr");
        lyrics_card.text(74.0,y+5.0,"词",14.0,W::Semibold,if !has {ic}else if self.show_lyrics {theme::WHITE}else{theme::GRAY},H::Center);
        let (previous,previous_card)=btn(162.0,30.0,"m-prev");
        paint::skip(&previous_card,162.0,y,-1.0,ic);
        let (play,play_card)=btn(210.5,30.0,"m-play");
        paint::play_pause(&play_card,211.0,y,15.0,playing_t,ic);
        let (next,next_card)=btn(258.0,30.0,"m-next");
        paint::skip(&next_card,258.0,y,1.0,ic);
        let (volume_button,volume_card)=btn(367.0,26.0,"m-volume");
        let (volume,muted)=self.music_volume_ui.unwrap_or((0,false));
        paint::speaker(&volume_card,367.0,y,muted,if self.music_volume_ui.is_some(){ic}else{theme::GRAY});
        let hint=if self.music_volume_ui.is_some(){format!("音乐应用音量 {volume}%{} · 点击展开 · 滚轮调节",if muted{"（已静音）"}else{""})}else{"暂无可调节的音乐音频会话".into()};
        let volume_button=volume_button.on_hover_text(hint);
        if c.live && has && self.music_volume_ui.is_some() {
            if volume_button.clicked(){self.music_volume_open=!self.music_volume_open;}
            if volume_button.hovered(){
                    let delta=c.ui.ctx().input(|i|i.raw_scroll_delta.y);
                    if delta.abs()>0.5 {
                        self.change_music_volume(Transport::SetVolume((volume as f32+delta/120.0*4.0).round().clamp(0.0,100.0) as u32),cmds);
                    }
            }
        }
        if has {
            if previous.clicked() {self.music_restart_at=Some(self.t);self.music_seek_ui=None;cmds.push(Cmd::Media(Transport::Prev));}
            if play.clicked() {cmds.push(Cmd::Media(Transport::Toggle));}
            if next.clicked() {self.music_restart_at=Some(self.t);self.music_seek_ui=None;cmds.push(Cmd::Media(Transport::Next));}
        }
        if lyrics_button.on_hover_text(if self.show_lyrics {"隐藏歌词"}else{"显示歌词"}).clicked() {
            if !has {self.say("先放一首歌吧");}
            else {
                self.show_lyrics=!self.show_lyrics;
                self.settings.lyrics_enabled=self.show_lyrics;
                self.settings.save();
            }
        }
        // 岛上歌词是横向的，只在顶部停靠时提供。
        if has && !self.side() {
            let (button,button_card)=capsule(c,279.0,108.0,68.0,20.0,"m-island-lyrics",if self.settings.island_lyrics {Color32::from_gray(39)}else{Color32::from_gray(22)},Color32::from_gray(47));
            button_card.text(313.0,122.0,"岛上歌词",10.0,W::Semibold,if self.settings.island_lyrics {theme::WHITE}else{theme::GRAY},H::Center);
            if button.on_hover_text(if self.settings.island_lyrics {"关闭收起时的歌词"}else{"收起时显示当前歌词"}).clicked() {
                self.settings.island_lyrics=!self.settings.island_lyrics;
                self.settings.save();
            }
        }
    }

    fn change_music_volume(&mut self,command:Transport,cmds:&mut Vec<Cmd>){
        let Some((volume,_))=self.music_volume_ui else{return;};
        let next=match command{Transport::SetVolume(v)=>(v,false),Transport::SetMute(muted)=>(volume,muted),_=>return};
        if self.music_volume_ui==Some(next){return;}
        self.music_volume_ui=Some(next);
        self.music_volume_quiet_until=self.t+0.9;
        cmds.push(Cmd::MusicVolume(command,self.media.source.clone()));
    }

    pub(crate) fn draw_music_volume(&mut self,c:&Card,cmds:&mut Vec<Cmd>){
        let ctx=c.ui.ctx();
        let bounds=MUSIC_VOLUME_RECT;
        let panel=c.rect(bounds.min.x,bounds.min.y,bounds.width(),bounds.height());
        let trigger=c.rect(354.0,106.0,26.0,24.0);
        if self.music_volume_open && ctx.input(|i|i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p|!panel.contains(p) && !trigger.contains(p))){
            self.music_volume_open=false;
        }
        let a=ctx.animate_bool_with_time(egui::Id::new("music-volume-open"),self.music_volume_open,0.12);
        let Some((volume,muted))=self.music_volume_ui else{return;};
        if a<0.01{return;}
        let popup=c.faded(a);
        let popup=if self.music_volume_open{popup}else{popup.ghost()};
        // 浮条最后绘制并占据整块命中区域，防止点击穿到后面的翻页按钮。
        popup.hit(bounds.min.x,bounds.min.y,bounds.width(),bounds.height(),"m-volume-surface");
        popup.fill(bounds.min.x,bounds.min.y,bounds.width(),bounds.height(),12.0,Color32::from_gray(36));
        popup.stroke(bounds.min.x,bounds.min.y,bounds.width(),bounds.height(),12.0,1.0,Color32::from_gray(60));
        let track=popup.hit_drag(351.0,27.0,32.0,54.0,"m-volume-track");
        let mut command=None;
        if popup.live && (track.clicked() || track.dragged() || track.is_pointer_button_down_on()) {
            if let Some(pointer)=track.interact_pointer_pos(){
                command=Some(Transport::SetVolume(((75.0-popup.to_local(pointer).y)/43.0*100.0).round().clamp(0.0,100.0) as u32));
            }
        }else if popup.live && panel.contains(ctx.pointer_hover_pos().unwrap_or(Pos2::ZERO)) {
            let delta=ctx.input(|i|i.raw_scroll_delta.y);
            if delta.abs()>0.5{command=Some(Transport::SetVolume((volume as f32+delta/120.0*4.0).round().clamp(0.0,100.0) as u32));}
        }
        let mute=popup.hit(353.0,82.0,28.0,20.0,"m-volume-mute").on_hover_text(if muted{"恢复音乐音量"}else{"静音音乐，保留当前音量"});
        let (hover,press)=popup.hover_t(&mute);
        if popup.live && mute.clicked(){command=Some(Transport::SetMute(!muted));}
        if let Some(command)=command {self.change_music_volume(command,cmds);}
        let (volume,muted)=self.music_volume_ui.unwrap_or((volume,muted));
        let ink=if muted{theme::GRAY}else{theme::WHITE};
        popup.centered_text(367.0,20.0,&format!("{volume}%"),10.5,W::Semibold,ink,H::Center);
        popup.fill(364.5,32.0,5.0,43.0,2.5,Color32::from_gray(68));
        let head=75.0-43.0*volume as f32/100.0;
        if volume>0 {popup.fill(364.5,head,5.0,(75.0-head).max(2.5),2.5,ink);}
        popup.circle(367.0,head,if track.dragged(){5.5}else{4.5},ink);
        popup.fill(353.0,82.0,28.0,20.0,6.0,paint::mix(theme::BTN,theme::BTN_HOVER,if muted{1.0}else{hover}));
        popup.scaled(367.0,92.0,1.0-0.06*press).centered_text(367.0,92.0,if muted{"恢复"}else{"静音"},9.5,W::Semibold,theme::WHITE,H::Center);
    }

    fn draw_music_lyrics(&mut self,c:&Card) {
        let col=c.clipped(243.0,28.0,136.0,74.0);
        c.seg((233.0,32.0),(233.0,98.0),1.0,Color32::from_gray(35));
        if let Some(lyrics)=&self.ly.lyrics {
            let current=if lyrics.synced {lyrics.index_at(self.lyric_position())}else{Some(0)};
            let target=current.unwrap_or(0);
            let id=egui::Id::new("music-lyrics-scroll");
            if !self.ly.scroll_animation_started {
                c.ui.ctx().animate_value_with_time(id,target as f32,0.0);
                self.ly.scroll_animation_started=true;
            }
            let position=c.ui.ctx().animate_value_with_time(id,target as f32,0.3);
            let first=(position.floor() as usize).saturating_sub(2);
            for (index,line) in lyrics.lines.iter().enumerate().skip(first).take(6) {
                let distance=index as f32-position;
                if distance.abs()>2.2 {continue;}
                let active=current==Some(index);
                let ink=if active {theme::WHITE}else{theme::GRAY};
                let row=col.faded((1.0-distance.abs()*0.35).clamp(0.0,1.0));
                let font=row.font(if active {12.0}else{10.0},if active {W::Semibold}else{W::Regular});
                // 维语等从右往左的歌词：先按阅读顺序分行再逐行转换（#200）。
                let text=crate::visuals::rtl_text::display_wrapped(&line.text,132.0*c.s,|s|row.painter.layout_no_wrap(s.to_owned(),font.clone(),ink).size().x).into_owned();
                let mut job=egui::text::LayoutJob::simple(text,font,ink,132.0*c.s);
                job.wrap.max_rows=if active {3}else{1};
                job.wrap.break_anywhere=true;
                let galley=row.painter.layout_job(job);
                let y=64.0+distance*29.0-galley.size().y/(2.0*c.s);
                row.painter.galley(row.pos(246.0,y),galley,ink);
            }
            if !lyrics.synced {col.text(246.0,39.0,"文本歌词",8.0,W::Regular,theme::GRAY,H::Left);}
        } else {
            let status=if self.ly.loading {"正在查找歌词…"}else if let Some(error)=&self.ly.error {error.label()}else{"暂无歌词"};
            let galley=col.painter.layout(status.into(),col.font(10.0,W::Regular),theme::GRAY,130.0*c.s);
            col.painter.galley(col.pos(246.0,61.0-galley.size().y/(2.0*c.s)),galley,theme::GRAY);
            if self.ly.error.is_some() {
                col.text(246.0,87.0,"点击重试",9.0,W::Regular,theme::GRAY,H::Left);
                if c.hit(243.0,28.0,136.0,74.0,"lyrics-retry").on_hover_text("重新查询当前歌曲的歌词").clicked(){self.ly.key.clear();self.ly.manual_retry=true;}
            }
        }
    }
}
