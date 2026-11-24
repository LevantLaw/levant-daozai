use super::*;

impl App {
    pub fn frame(&mut self, ui: &mut egui::Ui, win: Rect, env: &Env) -> FrameOut {
        let ctx = ui.ctx().clone();
        let dt = match self.last_t {
            Some(t0) => (env.t - t0).clamp(0.0, 0.1) as f32,
            None => 0.0,
        };
        let elapsed = self.last_t.map(|at| (env.t - at).max(0.0) as f32).unwrap_or(0.0);
        self.last_t = Some(env.t);
        self.t = env.t;
        self.fullscreen_hidden=env.fullscreen_hidden;
        self.quota_tick();
        self.agent_tick(&ctx);
        self.weather_tick();
        self.current_hour=env.hour();
        self.current_clock=env.clock.get(..5).unwrap_or_default().to_string();
        self.system_idle=env.idle_seconds;
        self.dt = dt;
        if let Some(at_top)=env.browser_at_top {self.browser_at_top=at_top;}
        self.screen=env.screen;
        if self.settings.skin.is_glass() {
            let target=if self.settings.skin==model::Skin::GlassLight {glass::light_tint(env.backdrop,env.glass_blur)}else{glass::glass_tint(env.backdrop,env.glass_blur)};
            self.glass_tint=if self.glass_tint<=0.0 {target}else{anim::approach(self.glass_tint,target,3.0,dt)};
        }
        let browser_minimal=self.settings.browser_minimal && self.browser_at_top;
        if browser_minimal && !self.browser_minimal_active {
            self.mark_open(false);
            self.hover_lock=true;
            self.hover_since=None;
            self.leave_since=None;
            self.forced_open_until=0.0;
        }
        self.browser_minimal_active=browser_minimal;
        if !self.started {
            self.started = true;
            self.last_input = env.t;
            self.last_reminder = env.t;
            self.sit_start = env.t;
            self.page_t0 = env.t - 10.0;
            if self.page==CALENDAR_PAGE {self.request_calendar_month(false);}
        }
        let mut cmds = Vec::new();

        self.tick(env, dt, elapsed);
        self.dnd.tick(self.t);
        self.sync_device(env);
        self.tick_alerts();
        self.agent_after_notice();
        self.tray_tick();
        let hud_active=self.notice.clone();
        if hud_active.as_ref().map(|a|&a.title)!=self.hud_notice.as_ref().map(|a|&a.title)||hud_active.is_none(){self.notice_details=false;}
        if let Some(a)=&hud_active {self.hud_notice=Some(a.clone());}
        self.hud_alpha=anim::approach(self.hud_alpha,if hud_active.is_some(){1.0}else{0.0},if hud_active.is_some(){16.0}else{8.0},dt);
        if hud_active.is_none() && self.hud_alpha<0.003 {self.hud_notice=None;}
        let notice_target=if !self.notice.as_ref().is_some_and(|a|a.messenger().is_some()) && self.compact_notices()&&(self.notice.is_some()||self.t<self.notification_bell_until||self.windows_unread) {44.0}else{0.0};
        self.notice_width.target=notice_target;
        self.notice_width.tune(NOTICE_CLOSE.0*self.settings.motion_speed.factor().powi(2),NOTICE_CLOSE.1);
        if self.compact_notices(){self.notice_width.step(dt);}else{self.notice_width.snap(0.0);self.notice_details=false;}
        self.update_media(env,&ctx);
        let bell=(self.t<self.notification_bell_until||self.windows_unread)&&!self.compact_notices();
        self.resident_power=(env.battery,env.charging);
        let badges=self.page_badges();
        let build_layout=|gap:f32|{
            let mut layout=Layout::new_with_gap(gap,self.settings.show_battery&&env.battery.is_some()&&!self.settings.tray_battery,self.bar_quota_indices().len(),self.settings.show_music&&self.media.is_music()&&!self.settings.tray_music,self.settings.show_focus&&self.pomo.running&&!self.settings.tray_focus,bell,self.pet_enabled(),self.weather_pill().is_some()&&!self.settings.tray_weather,self.agent_bar_visible());
            layout.reorder(&self.settings.resident_order);
            for (position,extra) in self.quota_balance_layout(&ctx){layout.balance_tail(position,extra);}
            if self.settings.clock_period!=0 && !self.open { layout.widen_clock(40.0); }
            if self.settings.clock_seconds && !self.open { layout.widen_clock(24.0); }
            if let Some(extra)=self.weather_pill_extra(&ctx){ layout.widen(bar::WEATHER,extra); }
            if !self.settings.show_clock && !self.open {layout.hide_clock();}
            // 展开后勿扰固定在状态行最右侧，页面徽标里不再重复占位。
            layout.add_extras(&badges.iter().filter(|badge| !(self.open && badge.slot==bar::DND)).map(|badge|(badge.slot,badge.width)).collect::<Vec<_>>());
            if self.tray_visible() {
                if let Some(width)=self.tray_width(&ctx) {layout.add_tray(width);}
            }
            if self.pet_enabled() {
                if let Some(a)=self.notice.as_ref().filter(|_|!self.compact_notices()&&!self.open){layout.x[bar::PET]=Some(22.0-(self.notice_size(a).0+self.pet_space())/2.0);}
                else if self.island_lyrics_active() {layout.x[bar::PET]=Some(60.0-self.lyric_bar_width()/2.0);}
            }
            layout
        };
        let mut layout=build_layout(bar::GAP);
        // 展开态优先压缩间距保住图标尺寸；间距压到下限仍放不下时再由绘制层等比缩小。
        if self.open {
            let available=PW-16.0-bar::TAIL_W;
            let slots=layout.x.iter().filter(|x|x.is_some()).count();
            if slots>1 && layout.width>available {
                let content=layout.width-bar::GAP*(slots-1) as f32;
                let gap=((available-content)/(slots-1) as f32).clamp(bar::MIN_GAP,bar::GAP);
                if gap<bar::GAP {layout=build_layout(gap);}
            }
        }
        self.resident.step(layout,dt,ctx.style().animation_time>0.0&&bar::animations_enabled());
        if self.opts.fetch_lyrics {
            self.poll_lyrics();
        }
        self.update_island_lyric_layout(&ctx);

        let cx = win.center().x;
        // 胶囊皮肤悬浮在屏幕顶边下方。
        let pill = self.settings.island_shape==model::IslandShape::Pill;
        let top = win.top() + if pill { PILL_GAP } else { 0.0 };
        let side = self.side();
        let dock = self.dock();

        // ---------- 输入：拖放 ----------
        let (hovering_files, dropped) = ctx.input(|i| {
            (
                self.settings.visible_pages[3] && !i.raw.hovered_files.is_empty(),
                i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect::<Vec<_>>(),
            )
        });
        if !dropped.is_empty() && self.settings.visible_pages[3] {
            self.on_drop(dropped);
            self.show_page(3);
        } else if hovering_files && !self.drag_hot {
            self.drag_hot = true;
            if self.page != 3 {
                self.drag_saved = Some(self.page);
                self.go(3, if self.page < 3 { 1.0 } else { -1.0 });
            }
        } else if !hovering_files && self.drag_hot {
            // 拖到外面松手 / 取消：安静地退回原页
            self.drag_hot = false;
            if let Some(p) = self.drag_saved.take() {
                self.go(p, if p < 3 { -1.0 } else { 1.0 });
            }
        }
        self.drag_a = anim::approach(self.drag_a, if self.drag_hot { 1.0 } else { 0.0 }, 16.0, dt);

        // ---------- 输入：悬停 ----------
        let p_prev = self.progress();
        let hit = if side {self.side_hit_rect(win,p_prev)} else {self.hit_rect(cx, top, p_prev)};
        let page_before=self.page;
        let previous_menu=self.shelf_menu_rect.take();
        let inside = env.cursor.is_some_and(|c| hit.contains(c) || previous_menu.is_some_and(|r|r.contains(c)));
        self.page_scroll_area=self.page_scroll_area.filter(|area|self.open && !self.hover_lock
            && !env.fullscreen_hidden && !env.screen_locked && env.cursor.is_some_and(|c|area.contains(c))
            && !ctx.input(|i|i.pointer.any_pressed() || i.key_pressed(egui::Key::Escape)));
        let page_hover=inside || self.page_scroll_area.is_some();
        let on_notice=env.cursor.is_some_and(|c|self.messenger_badge_rect(win,self.island_rect(win,self.w.pos,self.h.pos)).is_some_and(|r|r.contains(c)) || self.notice_rect(cx,top).is_some_and(|r|r.contains(c))||self.notice_details_rect(cx,top).is_some_and(|r|r.contains(c)));
        if !inside {
            self.hover_lock = false;
        }
        if env.cursor != self.last_cursor || ctx.input(|i| !i.events.is_empty()) {
            if inside || hovering_files || ctx.input(|i| !i.events.is_empty()) {
                self.last_input = env.t;
            }
            self.last_cursor = env.cursor;
        }
        if self.music_volume_open && ctx.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::Escape)) {
            self.music_volume_open=false;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.is_editing() {
            self.cancel_edit();
        }
        if (!self.is_editing() || self.todo_choosing) && page_hover && self.open && self.reveal > 0.95 {
            let (scroll, left, right, esc) = ctx.input(|i| (i.raw_scroll_delta.y, i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight), i.key_pressed(egui::Key::Escape)));
            if esc { self.hover_lock = true; }
            if (left || right) && !self.todo_choosing {
                self.wheel_delta=0.0;
                self.navigate_page(right);
            } else if scroll!=0.0 {
                let cursor=env.cursor.unwrap()-self.content_origin(win).to_vec2();
                let mail_reading=inside && self.page==5 && self.mail.open_uid.is_some();
                let mail_list=self.page==5 && !self.mail.accounts.is_empty() && self.mail.open_uid.is_none() && self.mail_list_bounds().contains(cursor);
                let music_volume=self.page==0 && (Rect::from_min_max(Pos2::new(354.0,106.0),Pos2::new(380.0,130.0)).contains(cursor)
                    || self.music_volume_open && crate::music::page::MUSIC_VOLUME_RECT.contains(cursor));
                if mail_reading || mail_list || music_volume {self.wheel_delta=0.0;}
                else {
                    // 累积高精度滚轮的小幅输入；停顿或反向时丢弃上一段余量。
                    if self.t-self.wheel_at>0.18 || self.wheel_delta*scroll<0.0 {self.wheel_delta=0.0;}
                    self.wheel_at=self.t;
                    self.wheel_delta+=scroll;
                    if self.wheel_delta.abs()>=8.0 {
                        let next=self.wheel_delta<0.0;
                        self.wheel_delta=0.0;
                        let calendar_max=if self.page==CALENDAR_PAGE {self.calendar_scroll_max(env)}else{0};
                        let calendar_body=self.page==CALENDAR_PAGE && Rect::from_min_max(Pos2::new(163.0,52.0),Pos2::new(378.0,208.0)).contains(cursor);
                        let clipboard_body=self.page==CLIPBOARD_PAGE && Rect::from_min_max(Pos2::new(42.0,31.0),Pos2::new(377.0,124.0)).contains(cursor);
                        let (agent_max,agent_step,agent_body)=if self.page==AI_PAGE && !self.agent.detail {
                            let count=self.agent.store.visible().len();
                            let rows=self.agent.config.display.rows;
                            let viewport=crate::ai_session::page::agent_card_area(count,rows);
                            (crate::ai_session::page::agent_scroll_max(count,rows),
                                crate::ai_session::page::agent_card_height(crate::ai_session::page::agent_visible_rows(count,rows))+crate::ai_session::page::AGENT_CARD_GAP,
                                Rect::from_min_max(Pos2::new(40.0,crate::ai_session::page::AGENT_LIST_TOP),Pos2::new(360.0,crate::ai_session::page::AGENT_LIST_TOP+viewport)).contains(cursor))
                        } else {(0.0,0.0,false)};
                        let list=match self.page {
                            CALENDAR_PAGE if calendar_body=>Some((&mut self.calendar.view.first,calendar_max)),
                            4 if Rect::from_min_max(Pos2::new(196.0,31.0),Pos2::new(377.0,124.0)).contains(cursor)
                                =>Some((&mut self.todo_first,self.todos.items.len().saturating_sub(3))),
                            CLIPBOARD_PAGE if clipboard_body
                                =>Some((&mut self.clip_first,self.clipboard.lock().items.len().saturating_sub(crate::clipboard::page::CLIP_ROWS))),
                            SHELF_PAGE if Rect::from_min_max(Pos2::new(36.0,28.0),Pos2::new(383.0,128.0)).contains(cursor)
                                =>Some((&mut self.shelf_first,self.shelf.items.len().saturating_sub(5))),
                            WINDOWS_PAGE if Rect::from_min_max(Pos2::new(40.0,52.0),Pos2::new(378.0,160.0)).contains(cursor)
                                =>Some((&mut self.windows_first,self.windows_notices.len().saturating_sub(WINDOWS_ROWS))),
                            _=>None,
                        };
                        let agent_scrolled=agent_body && agent_max>0.0 && {
                            let target=if next {(self.agent.scroll+agent_step).min(agent_max)}else{(self.agent.scroll-agent_step).max(0.0)};
                            let changed=(target-self.agent.scroll).abs()>0.01;
                            self.agent.scroll=target;changed
                        };
                        let content_scrolls=list.as_ref().is_some_and(|(_,max)|*max>0) || agent_body && agent_max>0.0;
                        let scrolled=list.is_some_and(|(first,max)|{
                            let target=if next {(*first+1).min(max)}else{first.saturating_sub(1)};
                            let changed=target!=*first;*first=target;changed
                        }) || agent_scrolled;
                        // 内容溢出时滚动列表并停在边界；未溢出时滚轮翻页。
                        if !scrolled && !content_scrolls && !self.todo_choosing && self.t-self.wheel_page_at>=0.08 {
                            self.navigate_page(next);
                            self.wheel_page_at=self.t;
                        }
                        ctx.input_mut(|i|{i.raw_scroll_delta=Vec2::ZERO;i.smooth_scroll_delta=Vec2::ZERO;});
                    }
                }
            }
        } else {self.wheel_delta=0.0;}
        let volume_dragging=self.music_volume_open && ctx.is_being_dragged(egui::Id::new(("card-drag",true,"m-volume-track")));
        let want = (page_hover && !self.hover_lock && (self.compact_notices()||self.notice.is_none()||self.open)) || self.open && on_notice || (self.open && self.t<self.forced_open_until) || self.drag_hot || volume_dragging || (self.is_editing() && self.open) || self.agent_waiting_pinned();
        self.morph(want, inside, env.t, dt);
        let shell_moving = self.shell_moving();
        if shell_moving { self.shell_settled_at = self.t; }
        let shell_edge = if shell_moving { 0.0 } else { smoothstep(0.0, 0.20, (self.t - self.shell_settled_at) as f32) };
        let p = self.progress();
        // 展开后状态行整体左移给勿扰和设置让位，吉祥物的绘制和悬停也跟随行中心。
        let tail_w=bar::TAIL_W*smoothstep(0.70,0.98,p);
        self.update_pet(env, cx-tail_w/2.0);

        // ---------- 绘制 ----------
        let (sx, sy) = self.drag_scale();
        // 拿起放大不能超出透明画布，否则两端会被切掉。
        let (max_w, max_h) = if side {(crate::island::layout::dock::SIDE_W, crate::island::layout::dock::SIDE_H)} else {(WIN_W, WIN_H)};
        let (w, h) = ((self.w.pos * sx).min((max_w - 24.0).max(self.w.pos)), (self.h.pos * sy).min((max_h - 24.0).max(self.h.pos)));
        let panel = self.island_rect(win, w, h);
        let messenger_rect=self.messenger_badge_rect(win,panel);
        let appear=self.messenger_appear();
        let outside_badge=messenger_rect.filter(|_|appear>0.02);
        let pc = anim::clamp01(p);
        let painter = ui.painter_at(ui.max_rect().expand(40.0));
        let feather = 1.0 / ctx.pixels_per_point();
        let notice_rect=self.notice_rect(cx,top);
        let notice_shape=notice_rect.map(|r|Rect::from_min_max(Pos2::new(r.left()-36.0,r.top()),r.max));
        let body=messenger_rect.filter(|_|outside_badge.is_none()).map_or(panel,|badge|panel.union(badge));
        let body=notice_shape.map_or(body,|notice|body.union(notice));
        let minimal_strip = self.browser_minimal_active && !self.island_lyrics_active() && self.notice.is_none() && self.t>=self.notification_bell_until && !self.windows_unread;
        let minimal_pill = minimal_strip && pc < 0.02;
        let flare = if minimal_pill || pill { 0.0 } else { lerp(5.0, FLARE, pc) };
        let k = (w / PW).clamp(0.3, 1.1);
        let thickness = if side { crate::island::layout::dock::SIDE_BAR_W } else { BAR_H };
        let radius = if minimal_pill { BAR_H_MIN / 2.0 } else if pill { (w.min(h) * 0.5).min(lerp(thickness * 0.5, 26.0, pc)) } else { lerp(12.5, RAD * k.min(1.0), pc) };
        let flare_h = lerp(8.0, FLARE_H, pc);
        let thick = glass::thickness(PH);
        let glass_skin = self.settings.skin.is_glass();
        let layer = painter.layer_id();
        let palette = self.skin_palette();
        // 换色时跳过的图形范围（岛体底色、阴影、吉祥物）。
        let mut keep_colors:Vec<(usize,usize)>=Vec::new();
        // 毛玻璃：真实模糊下保留柔和投影增加层次；无模糊时阴影再减淡一些，保持轻盈。
        let skin_shadow = if !glass_skin {1.0} else if env.glass_blur {0.75} else {0.7};
        let shadow_k = glass::shadow_alpha(0.0, thick) / 0.12 * skin_shadow;
        let island_shadow=|r:Rect,flare:f32,flare_h:f32,radius:f32,strength:f32| if pill {pill_shadow(&painter,r,radius,strength)} else if side {if shell_moving {shape::shadow_docked_fast(&painter,r,dock,flare,flare_h,radius,strength)} else {shape::shadow_docked(&painter,r,dock,flare,flare_h,radius,strength)}} else if shell_moving {shape::shadow_fast(&painter,r,flare,flare_h,radius,strength)} else {shape::shadow(&painter,r,flare,flare_h,radius,strength)};
        // 拿起时投影加深，像被从屏幕边缘提起来。
        let lifted = self.drag_lift.pos.clamp(0.0, 1.2);
        island_shadow(body, flare, flare_h, radius, (smoothstep(0.1, 0.9, pc) + lifted * 0.9) * shadow_k);
        if let Some(r)=outside_badge{
            let appear=self.messenger_appear();
            painter.rect_filled(r,r.width()/2.0,paint::alpha(self.glass_fill(),appear));
            if glass_skin {painter.rect_stroke(r,r.width()/2.0,egui::Stroke::new(1.0_f32,paint::alpha(self.glass_rim(),appear)),egui::StrokeKind::Inside);}
        }
        if self.settings.skin==model::Skin::Fluid {
            self.fluid_origin=body.min;
            let art=self.music_artwork.as_ref().map(|(_,texture)|texture.id());
            if art!=self.fluid_art || self.media.title!=self.fluid_title {
                let tint=if self.media.title.is_empty(){crate::visuals::fluid::Fluid::neutral()}
                    else if let Some((image,_))=&self.music_artwork{crate::visuals::fluid::Fluid::tint_from_image(image)}
                    else{crate::visuals::fluid::Fluid::tint_from_title(&self.media.title)};
                self.fluid.set_tint(tint);
                self.fluid_art=art;
                self.fluid_title.clone_from(&self.media.title);
            }
            self.fluid.set_cursor(env.cursor.filter(|p|body.contains(*p)).map(|p|(p.x-body.left(),p.y-body.top())));
            if !self.open || self.page!=0 || self.media.title.is_empty(){self.fluid.set_seek(None,0.0);}
            if !shell_moving {self.fluid.advance(env.t,dt);}
        }
        self.paint_body(&painter, body, flare, flare_h, radius, feather, shell_edge);
        if lifted > 0.01 {self.paint_lift_halo(&painter, body, flare, flare_h, radius, lifted.min(1.0));}
        let center_guide=self.paint_center_rim(&painter, body, panel, flare, flare_h, radius, env.screen);
        let content_from=crate::visuals::skin_paint::shape_count(&ctx,layer);

        // 内容只随形体「揭开」：不缩放，按面板裁剪；淡入由时间驱动，不跟尺寸走。
        let reveal = self.content_reveal();
        self.reveal = reveal;
        let origin = self.content_origin(win);
        let clip = panel.intersect(win);
        let base = Card::new(ui, origin, clip);
        let status_base = Card::new(ui, panel.min, clip);
        self.cursor_local = env.cursor.map(|p| base.to_local(p));

        let banner=!self.compact_notices()&&self.notice.is_some()&&!self.open;
        let restoring_lyrics=!self.open&&self.notice.is_none()&&self.island_lyrics_active()&&self.hud_notice.as_ref().is_some_and(|a|a.meeting.is_some()||a.source!=crate::island::banner::AlertSource::Internal);
        let fading_hud=!self.compact_notices()&&!self.open&&self.hud_notice.as_ref().is_some_and(|a|a.is_hud())&&(banner||self.hud_alpha>0.003);
        if fading_hud {
            let from=self.resident.target.width;let to=HUD_W+self.pet_space();
            let width_progress=if (to-from).abs()<1.0 {if banner{1.0}else{0.0}}else{((self.w.pos-from)/(to-from)).clamp(0.0,1.0)};
            let (opacity,header_opacity)=if banner {(self.hud_alpha*smoothstep(0.45,0.85,width_progress),1.0-smoothstep(0.05,0.40,width_progress))}
                else {(self.hud_alpha*smoothstep(0.20,0.60,width_progress),1.0-smoothstep(0.05,0.25,width_progress))};
            if !minimal_pill||pc>0.0 {self.resident_header(&status_base.faded(header_opacity),panel,cx,pc,reveal,env);}
            if let Some(a)=self.hud_notice.clone(){
                let overlay=Card::new(ui,panel.min+Vec2::new(self.pet_space(),0.0),panel.intersect(ui.max_rect())).faded(opacity);
                let overlay=if opacity<0.85||!banner{overlay.ghost()}else{overlay};
                self.draw_notice(&overlay,panel,&a,&mut cmds);
            }
        }else if banner {
            if let Some(a)=self.notice.clone(){self.draw_notice(&Card::new(ui,panel.min+Vec2::new(self.pet_space(),0.0),panel.intersect(ui.max_rect())),panel,&a,&mut cmds);}
        }else if !minimal_pill || pc > 0.0 {
            self.resident_header(&status_base, panel, cx, if restoring_lyrics{0.0}else{pc}, reveal, env);
        }
        if let Some(rect)=notice_rect {
            let c=Card::new(ui,rect.min,rect.intersect(ui.max_rect()));
            if let Some(a)=self.hud_notice.clone() {
                let opacity=self.hud_alpha*smoothstep(0.45,0.95,self.notice_width.pos/44.0);
                let c=c.faded(opacity);let c=if self.notice.is_none()||opacity<0.85 {c.ghost()}else{c};
                if a.source!=crate::island::banner::AlertSource::Internal {
                    paint::bell(&c.scaled(18.0,15.0,0.85),18.0,15.0,theme::WHITE,paint::bell_swing(self.t-a.at));
                    c.circle(22.0,10.5,2.0,Color32::from_rgb(255,72,72));
                }else{crate::island::banner::notice_badge(&c,18.0,15.0,&a);}
                let button=crate::quota::ui::bar_tooltip(c.hit(0.0,0.0,36.0,BAR_H,"notice-badge"),format!("{}\n{}",a.title,a.body));
                if button.clicked(){if let Some(kind)=a.health(){self.confirm_health_reminder(kind);}else{self.notice_details=!self.notice_details;}}
            }else if self.t<self.notification_bell_until || self.windows_unread {
                paint::bell(&c.scaled(20.0,14.0,1.2),20.0,14.0,theme::WHITE,paint::bell_swing(self.t-self.notification_bell_at));
                c.circle(24.0,9.5,2.0,Color32::from_rgb(255,72,72));
                if c.hit(6.0,0.0,28.0,30.0,"bell-mail").clicked(){
                    self.show_page(WINDOWS_PAGE);
                }
            }
        }
        if let Some(rect)=messenger_rect {
            let from=crate::visuals::skin_paint::shape_count(&ctx,layer);
            let c=Card::new(ui,rect.min,rect.intersect(ui.max_rect()));
            if outside_badge.is_some() {
                let c=c.scaled(0.0,0.0,appear);
                self.draw_unread_capsule(&ctx,&c,rect,env.cursor,&mut cmds);
            }else{
                let horizontal=!(self.side() && self.notice.is_none());
                let scale=self.messenger_stack_appear();
                self.draw_unread_stack(&c.scaled(0.0,0.0,scale).faded(scale),horizontal,&mut cmds);
            }
            keep_colors.push((from,crate::visuals::skin_paint::shape_count(&ctx,layer)));
        }
        let notice_details=self.notice_details_rect(cx,top);
        if let Some(rect)=notice_details {
            let radius=if self.hud_notice.as_ref().is_some_and(|a|a.meeting.is_some()){14.0}else{10.0};
            let from=crate::visuals::skin_paint::shape_count(&ctx,layer);
            shape::shadow(&painter,rect,0.0,0.0,radius,0.5*shadow_k);
            painter.rect_filled(rect,radius,self.glass_fill());
            if glass_skin {
                painter.rect_stroke(rect,radius,egui::Stroke::new(1.0_f32,self.glass_rim()),egui::StrokeKind::Inside);
            }
            keep_colors.push((from,crate::visuals::skin_paint::shape_count(&ctx,layer)));
            self.draw_notice(&Card::new(ui,rect.min,rect.intersect(ui.max_rect())),rect,&self.hud_notice.clone().unwrap(),&mut cmds);
            if ctx.input(|i|i.pointer.any_pressed()&&i.pointer.interact_pos().is_some_and(|p|!rect.contains(p)&&!notice_rect.is_some_and(|r|r.contains(p)))){self.notice_details=false;}
        }
        let pet_alpha=self.pet_a;
        if pet_alpha>0.004 && !minimal_pill {
            let y=(h.min(self.base_size().1)-BAR_H).max(0.0)/2.0;
            // 吉祥物跟随皮肤换色：浅色底上变成深色版本，保持清晰。
            let available=lerp(WIN_W-32.0,PW-16.0,p)-tail_w;
            let fit=if self.island_lyrics_active()||self.notice.is_some(){1.0}else{(available/self.resident.target.width).min(1.0)};
            let local_cx=cx-panel.min.x-tail_w/2.0;
            self.pet.draw(&status_base.scaled(local_cx,15.0,fit).shifted(0.0,y).faded(pet_alpha),local_cx+self.resident.position(bar::PET),self.pet.clock,self.skin_palette());
        }
        let mut page_transition=false;
        if reveal > 0.004 && !banner && !restoring_lyrics {
            let c = base.shifted(0.0, (1.0 - reveal) * -6.0).faded(reveal);
            let c = if reveal < 0.6 { c.ghost() } else { c };
            if self.calendar.draft.is_some(){self.draw_calendar_editor(&c);}else if self.task_dialog.is_some(){self.draw_task_editor(&c);}else{page_transition=self.panel_body(&c, env, &mut cmds);}
            if !self.toast_below {self.toast_layer(&c,106.0);}
        }
        // 岛下的确认提示使用独立裁剪区域，收起时跟随岛体下沿。
        let floating_toast=if self.toast_below && self.toast.is_some() {
            // 侧边停靠：提示出现在胶囊正下方，靠边时向内挪，保证完整显示。
            let toast_origin=if side {
                let center=if dock==model::Dock::Left {panel.center().x.max(win.left()+90.0)} else {panel.center().x.min(win.right()-90.0)};
                Pos2::new(center-PW/2.0,panel.top())
            } else {panel.min};
            self.toast_layer(&Card::new(ui,toast_origin,win),h+8.0)
        }else{None};
        if (self.adding||self.editing.is_some()) && reveal > 0.9 && self.page == 4 && self.page_e() > 0.25 {
            self.input_field(ui, origin, &mut cmds);
        }
        if reveal > 0.9 && self.page == AI_PAGE && self.agent_free_row().is_some() {
            self.agent_free_input(ui, origin, &mut cmds);
        }

        if self.task_dialog.is_some() && reveal>0.9 {self.task_editor_inputs(ui,origin,&mut cmds);}
        if self.calendar.draft.is_some() && reveal>0.9 {self.calendar_editor_inputs(ui,origin,&mut cmds);}

        // ---------- 输出 ----------
        let rel = |r: Rect| r.translate(-win.min.to_vec2());
        let mut region = if side {
            // 侧边：面板从边缘滑出，区域包含肩部（沿边缘上下）与外侧阴影，并一直延伸到屏幕边缘。
            let r=if p > 0.02 || self.open {
                let height=self.expanded_height().max(self.h.pos)*1.08+FLARE*2.0+24.0;
                self.island_rect(win,PW+20.0,height).union(panel.expand(20.0))
            } else {
                let (bw,bh)=self.base_size();
                self.island_rect(win,bw*SWELL.1+14.0,bh*SWELL.0+FLARE+16.0).union(panel.expand(12.0))
            };
            if dock==model::Dock::Left {Rect::from_min_max(Pos2::new(win.left(),r.top()),r.max)} else {Rect::from_min_max(r.min,Pos2::new(win.right(),r.bottom()))}
        } else if p > 0.02 || self.open {
            let height=self.expanded_height().max(self.h.pos)*1.08+16.0;
            Rect::from_center_size(Pos2::new(cx, top + height/2.0), Vec2::new(PW + FLARE * 2.0 + 56.0, height))
        } else {
            let (bw, bh) = self.base_size();
            Rect::from_center_size(Pos2::new(cx, top + (bh * SWELL.1 + 12.0) / 2.0), Vec2::new(bw * SWELL.0 + 24.0, bh * SWELL.1 + 12.0))
        };
        // 拿起放大、甩动拉伸以及加深的投影都要留在窗口区域内，否则两端会被系统裁掉。
        if self.drag_lift.pos.abs()>0.005 || self.drag_stretch.pos.abs()>0.001 {region=region.union(panel.expand2(Vec2::new(18.0,22.0)));}
        if let Some(r)=center_guide {region=region.union(r);}
        if let Some(r)=messenger_rect{region=region.union(r.expand(5.0));}
        if let Some(rect)=notice_shape {region=region.union(rect.expand2(Vec2::new(12.0,8.0)));}
        if let Some(rect)=notice_details {region=region.union(rect.expand(12.0));}
        // 胶囊上方留空的那一小条也要在窗口区域内，阴影才不会被截断。
        if pill && !side {region.min.y=region.min.y.min(win.top());}
        // 浅色皮肤：把这一帧岛体内容统一换成皮肤配色。
        if let Some(palette)=palette {
            let end=crate::visuals::skin_paint::shape_count(&ctx,layer);
            let mut ranges=Vec::new();let mut at=content_from;
            keep_colors.sort();
            for (from,to) in keep_colors {if from>at {ranges.push((at,from));} at=at.max(to);}
            if end>at {ranges.push((at,end));}
            crate::visuals::skin_paint::recolor_layer(&ctx,layer,&ranges,&palette);
        }
        // 原生裁剪还需包含悬停提示；只扩大绘制区域，保持桌面点击穿透。
        let mut tooltips=Vec::new();
        ctx.memory(|memory|{
            for layer in memory.areas().visible_layer_ids() {
                if layer.order==egui::Order::Tooltip {
                    if let Some(rect)=memory.area_rect(layer.id){region=region.union(rect.expand(12.0));tooltips.push(rect);}
                }
            }
        });
        // 毛玻璃：窗口区域只取岛体与浮层的真实轮廓，系统模糊不会溢出到透明画布上。
        let glass_region=(glass_skin && self.settings.glass_blur || env.opaque_surface).then(||{
            let outline=|r:Rect,flare:f32,flare_h:f32,radius:f32| if pill {GlassShape::Round(rel(r),radius.min(r.height().min(r.width())*0.5))}
                else if side {GlassShape::Docked{rect:rel(r),dock,flare,flare_h,radius}} else {GlassShape::Notch{rect:rel(r),flare,flare_h,radius}};
            let mut shapes=vec![outline(body,flare,flare_h,radius)];
            if let Some(r)=outside_badge{shapes.push(GlassShape::Round(rel(r),r.width()/2.0));}
            if let Some(r)=notice_details {
                let radius=if self.hud_notice.as_ref().is_some_and(|a|a.meeting.is_some()){14.0}else{10.0};
                shapes.push(GlassShape::Round(rel(r),radius));
            }
            if let Some(r)=floating_toast {shapes.push(GlassShape::Round(rel(r),11.0));}
            shapes.extend(tooltips.iter().map(|r|GlassShape::Round(rel(*r),6.0)));
            shapes
        });
        self.update_position_drag(&ctx,env,dt);
        let transitioning = self.position_drag.is_some() || self.drag_glide.is_some() || !self.drag_lift.settled(1.0) || !self.drag_stretch.settled(1.0) || !self.w.settled(PW)
            || self.center_guide>0.002 || self.center_lock_a>0.002 || self.t-self.center_lock_at<CENTER_SWEEP
            || !self.h.settled(self.expanded_height())
            || self.page_e() < 0.7
            || self.dots.iter().any(|d| !d.settled(16.0))
            || self.hover_since.is_some() && !self.open
            || self.leave_since.is_some()
            || (self.drag_a > 0.001 && self.drag_a < 0.999)
            || self.drag_hot
            || (self.pet_a > 0.001 && self.pet_a < 0.999)
            || !self.resident.settled()
            || self.open && self.page==3 && self.shelf_x.values().any(|s| !s.settled(68.0))
            || self.open && self.page==2 && (!self.water_ring.settled(1.0) || !self.sit_view_t.settled(1.0) || !self.sit_t.settled(1.0))
            || self.toast.is_some()
            || self.t < self.notification_bell_until && self.t-self.notification_bell_at < 0.95
            || self.notice.as_ref().is_some_and(|a|a.source!=crate::island::banner::AlertSource::Internal&&self.t-a.at<0.95)
            || self.hud_alpha > 0.003 && self.hud_alpha < 0.997
            || self.t-self.open_at < 0.40
            || self.t-self.close_at < 0.14
            || self.t-self.shell_settled_at < 0.22
            || !self.notice_width.settled(1.0);
        let pet_visible=self.pet_a>0.01 && !self.open && !minimal_pill;
        let pet_moving=pet_visible && (self.pet.hovered || self.pet.reaction.is_some() || self.pet.waking_until > self.t
            || self.pet.expressions.current().is_some() || self.t < self.pet.anim_until || self.pet.state==PetState::Music);
        let ambient=self.settings.skin==model::Skin::Fluid && self.fluid.animating() || self.pomo.running && (self.open && self.page==1 || !self.open && !minimal_pill && self.notice.is_none() && bar::animations_enabled())
            || self.media.playing && (self.open && self.page==0 || !self.open && !minimal_pill && self.notice.is_none() && self.media.is_music())
            || self.notice.as_ref().is_some_and(|notice|notice.meeting.is_some());
        let animating=transitioning || ambient || pet_moving;
        // 交互形变保持 60 帧，媒体 30 帧；吉祥物只在悬停/事件时动，空闲停在静态姿势。
        let repaint_after=std::time::Duration::from_secs_f64(if transitioning{1.0/60.0}else if ambient{1.0/30.0}else if pet_moving{0.1}else{1.0});
        cmds.append(&mut self.pending);
        let next_hit=if side {self.side_hit_rect(win,self.progress())} else {self.hit_rect(cx,top,self.progress())};
        if page_before!=self.page && self.open {
            let previous=self.page_scroll_area.map_or(hit,|area|hit.union(area));
            // 保留缩短一侧原有的整段宽度。和新页重叠一点，避免在边缘来回挪动时误收起。
            self.page_scroll_area=env.cursor.filter(|&c|previous.contains(c)).and_then(|c|{
                let mut band=previous;
                if previous.bottom()>next_hit.bottom() && c.y>=next_hit.bottom()-PAGE_EDGE_OVERLAP {
                    band.min.y=next_hit.bottom()-PAGE_EDGE_OVERLAP;
                } else if previous.top()<next_hit.top() && c.y<=next_hit.top()+PAGE_EDGE_OVERLAP {
                    band.max.y=next_hit.top()+PAGE_EDGE_OVERLAP;
                } else {
                    return None;
                }
                Some(band)
            });
        }
        FrameOut {
            region: rel(floating_toast.into_iter().chain(self.shelf_menu_rect.map(|r|r.union(r + ctx.style().visuals.popup_shadow.margin()).expand(1.0 / ctx.pixels_per_point()))).fold(region,Rect::union).intersect(ui.max_rect())),
            glass: glass_region,
            hit: rel(next_hit),
            page_scroll_area:self.page_scroll_area.map(rel),
            click_through: minimal_strip && !self.open && self.position_drag.is_none(),
            notice_hit:notice_shape.into_iter().chain(notice_details).chain(messenger_rect).chain(self.shelf_menu_rect).reduce(|a,b|a.union(b)).map(rel),
            file_drop_zone:self.settings.visible_pages[3].then(||rel(if side {self.island_rect(win,PW,self.expanded_height())} else {Rect::from_min_size(Pos2::new(cx-PW/2.0,top),Vec2::new(PW,96.0))})),
            expanded: self.open,
            editing: self.is_editing(),
            animating,
            page_transition,
            repaint_after,
            cmds,
        }
    }
}
