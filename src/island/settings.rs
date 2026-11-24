//! 设置页与设置控件。
use super::*;
use crate::island::settings_theme::color as settings_color;
use crate::island::extras::HealthReminder;

fn switch(ui:&mut egui::Ui,value:&mut bool)->bool {
    let (rect,response)=ui.allocate_exact_size(egui::vec2(34.0,20.0),egui::Sense::click());
    if response.clicked(){*value=!*value;}
    let fill=if *value{Color32::from_rgb(48,184,92)}else{settings_color(&ui.ctx().clone(),Color32::from_rgb(82,82,87),Color32::from_rgb(203,207,214))};
    ui.painter().rect_filled(rect,10.0,fill);
    let x=if *value{rect.right()-10.0}else{rect.left()+10.0};
    ui.painter().circle_filled(egui::pos2(x,rect.center().y+0.5),7.5,Color32::from_black_alpha(35));
    ui.painter().circle_filled(egui::pos2(x,rect.center().y),7.0,Color32::WHITE);
    response.clicked()
}
/// 设置页版式：标签列宽、开关尺寸、控件与卡片右边缘之间的固定留白。
pub(crate) const SETTINGS_LABEL_W: f32 = 80.0;
const SETTINGS_SWITCH_W: f32 = 34.0;
const SETTINGS_ROW_GAP: f32 = 16.0;

/// 开关行：左侧标题 + 说明（宽度固定、自动换行），右侧开关，长说明也不会把开关挤出卡片。
pub(crate) fn setting_row(ui:&mut egui::Ui,title:&str,description:&str,value:&mut bool){
    setting_row_sized(ui,title,description,value,13.0,11.5);
}

/// 分组内的子项：字号更小，配合缩进表达层级。
pub(crate) fn setting_row_nested(ui:&mut egui::Ui,title:&str,description:&str,value:&mut bool){
    setting_row_sized(ui,title,description,value,12.0,10.5);
}

fn setting_row_sized(ui:&mut egui::Ui,title:&str,description:&str,value:&mut bool,title_size:f32,desc_size:f32){
    let ctx=&ui.ctx().clone();
    ui.add_space(2.0);
    ui.horizontal(|ui|{
        let text_w=(ui.available_width()-SETTINGS_SWITCH_W-SETTINGS_ROW_GAP).max(120.0);
        ui.vertical(|ui|{
            ui.set_width(text_w);
            ui.spacing_mut().item_spacing.y=2.0;
            ui.add(egui::Label::new(egui::RichText::new(title).size(title_size).color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41)))).wrap());
            if !description.is_empty(){ui.add(egui::Label::new(egui::RichText::new(description).size(desc_size).color(settings_color(ctx,Color32::from_gray(185),Color32::from_gray(82)))).wrap());}
        });
        // add_space 之后还会再插入一次 item_spacing.x，这里一并扣掉，开关才不会每行往外挤一点。
        ui.add_space((ui.available_width()-SETTINGS_SWITCH_W-ui.spacing().item_spacing.x).max(0.0));
        switch(ui,value);
    });
    ui.add_space(2.0);
}

/// 设置页统一的分段选择器（灰色底框 + 选中块）。返回值为是否发生了切换。
pub(crate) fn settings_segmented<T:PartialEq+Copy>(ui:&mut egui::Ui,value:&mut T,options:&[(T,&str)],item_w:f32)->bool{
    let ctx=&ui.ctx().clone();
    let mut changed=false;
    egui::Frame::new().fill(settings_color(ctx,Color32::from_rgb(28,28,30),Color32::from_rgb(230,233,238)))
        .corner_radius(8.0).inner_margin(3).show(ui,|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.spacing_mut().item_spacing.x=2.0;
                for (option,label) in options {
                    let selected=*value==*option;
                    let text=egui::RichText::new(*label).size(12.0).color(if selected{settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))}else{settings_color(ctx,Color32::from_gray(200),Color32::from_gray(70))});
                    let button=egui::Button::new(text)
                        .fill(if selected{settings_color(ctx,Color32::from_rgb(72,72,76),Color32::WHITE)}else{Color32::TRANSPARENT})
                        .stroke(egui::Stroke::NONE).corner_radius(6.0);
                    let width=item_w.max(ctx.fonts(|f|f.layout_no_wrap(label.to_string(),egui::FontId::proportional(12.0),Color32::WHITE).size().x)+20.0);
                    // 选中项浮起一点：浅色模式白底加淡阴影，深色模式更亮的底色加阴影。
                    let clicked=if selected {
                        egui::Frame::new().corner_radius(6.0).shadow(egui::Shadow{offset:[0,1],blur:4,spread:0,color:Color32::from_black_alpha(if ui.visuals().dark_mode{90}else{38})})
                            .show(ui,|ui|ui.add_sized([width,26.0],button)).inner.clicked()
                    } else {ui.add_sized([width,26.0],button).clicked()};
                    if clicked && !selected {*value=*option;changed=true;}
                }
            });
        });
    changed
}

/// 选项行：标签列之后紧跟控件，同一卡片里所有控件左缘对齐成一条竖线。
pub(crate) fn settings_control_row_right(ui:&mut egui::Ui,title:&str,controls:impl FnOnce(&mut egui::Ui))->egui::Response{
    settings_control_row(ui,title,SETTINGS_LABEL_W,|ui|ui.horizontal(|ui|controls(ui)).response)
}

/// 设置项下方的说明文字。
pub(crate) fn island_size_slider(ui:&mut egui::Ui,value:&mut u32)->egui::Response {
    let id=ui.make_persistent_id("island-size-draft");
    let mut draft=ui.data(|d|d.get_temp::<u32>(id)).unwrap_or(*value);
    let response=ui.add(egui::Slider::new(&mut draft,crate::model::ISLAND_SIZE_MIN..=crate::model::ISLAND_SIZE_MAX)
        .step_by(crate::model::ISLAND_SIZE_STEP as f64).suffix("%"));
    if ui.input(|i|i.pointer.primary_down()) {
        if response.changed(){ui.data_mut(|d|d.insert_temp(id,draft));}
    }else{
        *value=crate::model::island_size_value(draft);
        ui.data_mut(|d|d.remove::<u32>(id));
    }
    response
}

pub(crate) fn settings_hint(ui:&mut egui::Ui,text:&str){
    let ctx=&ui.ctx().clone();
    ui.add(egui::Label::new(egui::RichText::new(text).size(11.5).color(settings_color(ctx,Color32::from_gray(170),Color32::from_gray(92)))).wrap());
}
/// 页面或分组底部的操作按钮行：靠右排列，行高只占按钮本身。
pub(crate) fn settings_action_row(ui:&mut egui::Ui,add:impl FnOnce(&mut egui::Ui)){
    ui.horizontal(|ui|ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),add));
}
pub(crate) fn settings_card(ui:&mut egui::Ui,add:impl FnOnce(&mut egui::Ui)) {
    let ctx=&ui.ctx().clone();
    egui::Frame::new().fill(settings_color(ctx,Color32::from_rgb(35,35,38),Color32::from_rgb(255,255,255)))
        .corner_radius(10.0).inner_margin(egui::Margin::symmetric(16,12))
        .show(ui,|ui|{ui.set_min_width(ui.available_width());ui.spacing_mut().item_spacing.y=8.0;add(ui);});
}

/// 卡片内的折叠区：整条标题行可点，右侧箭头表示展开状态，默认收起。
fn settings_fold(ui:&mut egui::Ui,id:&str,title:&str,muted:bool,add:impl FnOnce(&mut egui::Ui)){
    let ctx=&ui.ctx().clone();
    let state=egui::Id::new(("settings-fold",id));
    let mut open=ctx.data(|d|d.get_temp::<bool>(state)).unwrap_or(false);
    ui.add_space(2.0);
    let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),34.0),egui::Sense::click());
    if response.hovered(){ui.painter().rect_filled(rect.expand2(egui::vec2(6.0,0.0)),7.0,settings_color(ctx,Color32::from_rgb(44,44,48),Color32::from_rgb(242,244,247)));}
    let fg=if muted{settings_color(ctx,Color32::from_gray(205),Color32::from_gray(60))}else{settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))};
    ui.painter().text(egui::pos2(rect.left(),rect.center().y),egui::Align2::LEFT_CENTER,title,egui::FontId::proportional(13.0),fg);
    // 右侧箭头：收起朝右，展开朝下。
    let c=egui::pos2(rect.right()-10.0,rect.center().y);
    let pts=if open{vec![c+egui::vec2(-4.5,-2.0),c+egui::vec2(0.0,2.5),c+egui::vec2(4.5,-2.0)]}else{vec![c+egui::vec2(-2.0,-4.5),c+egui::vec2(2.5,0.0),c+egui::vec2(-2.0,4.5)]};
    ui.painter().add(egui::Shape::line(pts,egui::Stroke::new(1.6,fg)));
    if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){open=!open;ctx.data_mut(|d|d.insert_temp(state,open));}
    if open {
        ui.spacing_mut().item_spacing.y=8.0;
        add(ui);
    }
}
/// 卡片内的二级分组折叠区（如「常驻显示」「事件显示」）：默认展开，点标题收起。
pub(crate) fn settings_group_fold(ui:&mut egui::Ui,id:&str,title:&str,add:impl FnOnce(&mut egui::Ui)){
    let ctx=ui.ctx().clone();
    let state=egui::Id::new(("settings-group-fold",id));
    let mut open=ctx.data(|d|d.get_temp::<bool>(state)).unwrap_or(true);
    ui.add_space(2.0);
    let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),26.0),egui::Sense::click());
    if response.hovered(){ui.painter().rect_filled(rect.expand2(egui::vec2(6.0,0.0)),7.0,settings_color(&ctx,Color32::from_rgb(44,44,48),Color32::from_rgb(242,244,247)));}
    let fg=settings_color(&ctx,Color32::WHITE,Color32::from_rgb(30,34,41));
    ui.painter().text(egui::pos2(rect.left(),rect.center().y),egui::Align2::LEFT_CENTER,title,egui::FontId::proportional(13.0),fg);
    let c=egui::pos2(rect.right()-10.0,rect.center().y);
    let pts=if open{vec![c+egui::vec2(-4.5,-2.0),c+egui::vec2(0.0,2.5),c+egui::vec2(4.5,-2.0)]}else{vec![c+egui::vec2(-2.0,-4.5),c+egui::vec2(2.5,0.0),c+egui::vec2(-2.0,4.5)]};
    ui.painter().add(egui::Shape::line(pts,egui::Stroke::new(1.6,fg)));
    if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){open=!open;ctx.data_mut(|d|d.insert_temp(state,open));}
    if open {
        ui.spacing_mut().item_spacing.y=8.0;
        add(ui);
    }
}
/// 卡片底部的「高级」折叠区：只放内容多的页面里不常改的项，默认收起。
pub(crate) fn settings_advanced(ui:&mut egui::Ui,id:&str,add:impl FnOnce(&mut egui::Ui)){
    settings_fold(ui,id,"高级",true,add);
}
/// 状态小标签：开启绿色、关闭灰色；固定高度的胶囊，不随行高拉伸。
fn settings_chip(ui:&mut egui::Ui,text:&str,on:bool)->egui::Response{
    let ctx=&ui.ctx().clone();
    let (fg,bg)=if on{(settings_color(ctx,Color32::from_rgb(127,214,155),Color32::from_rgb(29,122,60)),settings_color(ctx,Color32::from_rgb(28,51,36),Color32::from_rgb(227,244,232)))}
        else{(settings_color(ctx,Color32::from_gray(160),Color32::from_gray(105)),settings_color(ctx,Color32::from_rgb(44,44,48),Color32::from_rgb(232,235,240)))};
    let galley=ui.painter().layout_no_wrap(text.into(),egui::FontId::proportional(11.5),fg);
    let (rect,response)=ui.allocate_exact_size(egui::vec2(galley.size().x+16.0,20.0),egui::Sense::hover());
    ui.painter().rect_filled(rect,10.0,bg);
    ui.painter().galley(rect.center()-galley.size()/2.0,galley,fg);
    response
}
fn settings_state_chip(ui:&mut egui::Ui,on:bool){
    settings_chip(ui,if on{"已开启"}else{"已关闭"},on);
}
/// 蓝色主操作按钮。
pub(crate) fn settings_primary_button(text:&str)->egui::Button<'static>{
    egui::Button::new(egui::RichText::new(text).color(Color32::WHITE)).fill(Color32::from_rgb(58,120,240)).corner_radius(6.0)
}
/// 红色危险操作按钮。
pub(crate) fn settings_danger_button(text:&str)->egui::Button<'static>{
    egui::Button::new(egui::RichText::new(text).color(Color32::WHITE)).fill(Color32::from_rgb(200,60,60)).corner_radius(6.0)
}
/// Windows 通知设置里已开启的应用；其他平台只显示已经收到过通知的应用。
#[cfg(windows)]
fn enabled_notification_apps()->Vec<(String,String)>{crate::os::system::notification_apps_enabled()}
#[cfg(not(windows))]
fn enabled_notification_apps()->Vec<(String,String)>{Vec::new()}
#[cfg(windows)]
fn notification_app_enabled(id:&str)->bool{crate::os::system::notification_enabled(id)!=Some(false)}
#[cfg(not(windows))]
fn notification_app_enabled(_id:&str)->bool{true}
/// 带状态胶囊的开关行：胶囊在开关左侧，说明胶囊不可点。
fn setting_row_chip(ui:&mut egui::Ui,title:&str,chip:&str,chip_on:bool,chip_tip:&str,value:&mut bool){
    let ctx=&ui.ctx().clone();
    ui.add_space(2.0);
    ui.horizontal(|ui|{
        let chip_w=ctx.fonts(|fonts|fonts.layout_no_wrap(chip.to_owned(),egui::FontId::proportional(11.5),Color32::WHITE).size().x)+16.0;
        let text_w=(ui.available_width()-SETTINGS_SWITCH_W-SETTINGS_ROW_GAP-chip_w-8.0).max(120.0);
        ui.vertical(|ui|{
            ui.set_width(text_w);
            ui.add(egui::Label::new(egui::RichText::new(title).size(13.0).color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41)))).wrap());
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
            switch(ui,value);
            let response=settings_chip(ui,chip,chip_on);
            if !chip_tip.is_empty(){response.on_hover_text(chip_tip);}
        });
    });
    ui.add_space(2.0);
}
/// 设置页统一的控件样式（按钮、输入框、选中色）；设置页正文和弹窗共用。
pub(crate) fn settings_apply_style(ui:&mut egui::Ui){
    let ctx=&ui.ctx().clone();
    // 分割线：浅色模式用很淡的灰，深色模式略亮于卡片，不再是系统默认的粗灰线。
    ui.visuals_mut().widgets.noninteractive.bg_stroke=egui::Stroke::new(1.0,settings_color(ctx,Color32::from_rgb(52,52,57),Color32::from_rgb(233,235,240)));
    ui.spacing_mut().item_spacing.y=ui.spacing().item_spacing.y.max(5.0);
    ui.spacing_mut().item_spacing=egui::vec2(8.0,5.0);
    ui.spacing_mut().button_padding=egui::vec2(10.0,6.0);
    ui.style_mut().text_styles.insert(egui::TextStyle::Small,egui::FontId::proportional(12.0));
    ui.visuals_mut().override_text_color=Some(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41)));
    ui.visuals_mut().extreme_bg_color=settings_color(ctx,Color32::from_rgb(25,25,28),Color32::from_rgb(244,246,249));
    ui.visuals_mut().widgets.inactive.bg_fill=settings_color(ctx,Color32::from_rgb(50,50,54),Color32::from_rgb(228,231,236));
    ui.visuals_mut().widgets.hovered.bg_fill=settings_color(ctx,Color32::from_rgb(65,65,71),Color32::from_rgb(216,222,231));
    // 统一按钮：高度 28、圆角 7、内边距 12×5，无描边；默认按钮也套用这套样式。
    ui.spacing_mut().button_padding=egui::vec2(12.0,5.0);
    ui.spacing_mut().interact_size.y=28.0;
    {
        let fill=settings_color(ctx,Color32::from_rgb(50,50,54),Color32::from_rgb(228,231,236));
        let hover=settings_color(ctx,Color32::from_rgb(65,65,71),Color32::from_rgb(216,222,231));
        let widgets=&mut ui.visuals_mut().widgets;
        for (w,f) in [(&mut widgets.inactive,fill),(&mut widgets.hovered,hover),(&mut widgets.active,hover)] {
            w.corner_radius=egui::CornerRadius::same(7);
            w.weak_bg_fill=f;
            w.bg_stroke=egui::Stroke::NONE;
        }
    }
    ui.visuals_mut().selection.bg_fill=settings_color(ctx,Color32::from_rgb(45,105,165),Color32::from_rgb(209,226,248));
}
/// 设置页弹窗的外框：卡片底色、大圆角、细描边和阴影。
pub(crate) fn settings_modal_frame(ctx:&egui::Context)->egui::Frame{
    egui::Frame::new().fill(settings_color(ctx,Color32::from_rgb(36,36,40),Color32::WHITE)).corner_radius(14.0).inner_margin(egui::Margin::same(20))
        .stroke(egui::Stroke::new(1.0,settings_color(ctx,Color32::from_rgb(58,58,64),Color32::from_rgb(222,225,231))))
        .shadow(egui::Shadow{offset:[0,8],blur:28,spread:0,color:Color32::from_black_alpha(90)})
}
/// 弹窗标题栏：标题在左、关闭在右，下面一条分隔线。返回是否点了关闭。
pub(crate) fn settings_modal_title(ui:&mut egui::Ui,title:&str)->bool{
    let ctx=&ui.ctx().clone();
    let mut close=false;
    ui.horizontal(|ui|{
        ui.label(egui::RichText::new(title).size(16.0).strong().color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
            close=ui.add(egui::Button::new(egui::RichText::new("×").size(18.0)).frame(false)).on_hover_text("关闭").clicked();
        });
    });
    ui.add_space(4.0);settings_separator(ui);ui.add_space(6.0);
    close
}
/// 设置页分割线（样式由 settings_apply_style 统一设定）。
fn settings_separator(ui:&mut egui::Ui){ui.separator();}
/// 功能页面对应的图标，与侧栏同一套。
/// 纹理缓存只按地址区分、不分尺寸：侧栏 23px 先加载后，36px 的卡片图标会被放大发糊，所以这里用独立地址。
fn settings_page_icon(page:usize)->egui::ImageSource<'static>{
    use crate::island::*;
    let (name,bytes):(&str,&'static [u8])=match page {
        MUSIC_PAGE=>("music",include_bytes!("../../assets/settings-icons/system/music.svg")),
        FOCUS_PAGE=>("focus",include_bytes!("../../assets/settings-icons/system/focus.svg")),
        WATER_PAGE=>("health",include_bytes!("../../assets/settings-icons/system/health.svg")),
        SHELF_PAGE=>("shelf",include_bytes!("../../assets/settings-icons/system/shelf.svg")),
        TODO_PAGE=>("tasks",include_bytes!("../../assets/settings-icons/system/tasks.svg")),
        MAIL_PAGE=>("mail",include_bytes!("../../assets/settings-icons/system/mail.svg")),
        QUOTA_PAGE=>("quota",include_bytes!("../../assets/settings-icons/system/quota.svg")),
        AI_PAGE=>("sessions",include_bytes!("../../assets/settings-icons/system/sessions.svg")),
        WINDOWS_PAGE=>("windows",include_bytes!("../../assets/settings-icons/system/notifications.svg")),
        CALENDAR_PAGE=>("calendar",include_bytes!("../../assets/settings-icons/system/calendar.svg")),
        CLIPBOARD_PAGE=>("clipboard",include_bytes!("../../assets/settings-icons/system/clipboard.svg")),
        _=>("weather",include_bytes!("../../assets/settings-icons/system/weather.svg")),
    };
    egui::ImageSource::Bytes{uri:format!("bytes://settings-header-{name}.svg").into(),bytes:bytes.into()}
}

/// 把功能页图标画到横幅图标位；按实际像素加载，避免与设置页纹理互相拉伸。
pub(crate) fn page_icon(c:&Card,page:usize,x:f32,y:f32,size:f32){
    let egui::ImageSource::Bytes{uri,bytes}=settings_page_icon(page) else{return;};
    let rect=c.rect(x-size/2.0,y-size/2.0,size,size);
    let pixels=(rect.width()*c.ui.ctx().pixels_per_point()).ceil() as u32;
    let icon=egui::Image::from_bytes(format!("{uri}/{pixels}.svg"),bytes);
    if let Ok(egui::load::TexturePoll::Ready{texture})=icon.fit_to_exact_size(rect.size()).load_for_size(c.ui.ctx(),rect.size()) {
        c.painter.image(texture.id,rect,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),Color32::WHITE);
    }
}

/// 设置分页对应的功能模块（页面序号）；通用、岛体、行为、提醒、关于不属于任何模块。
#[cfg_attr(not(test),allow(dead_code))]
pub(crate) fn settings_tab_module(tab:usize)->Option<usize>{
    use crate::island::*;
    match tab {1=>Some(MUSIC_PAGE),2=>Some(FOCUS_PAGE),3=>Some(WATER_PAGE),4=>Some(TODO_PAGE),5=>Some(SHELF_PAGE),7=>Some(MAIL_PAGE),8=>Some(CALENDAR_PAGE),11=>Some(QUOTA_PAGE),12=>Some(WEATHER_PAGE),15=>Some(AI_PAGE),18=>Some(WINDOWS_PAGE),SETTINGS_TAB_CLIPBOARD=>Some(CLIPBOARD_PAGE),_=>None}
}

pub(crate) fn settings_label(ui:&mut egui::Ui,text:&str){
    let ctx=&ui.ctx().clone();
    ui.add_space(9.0);
    ui.label(egui::RichText::new(text).size(11.5).color(settings_color(ctx,Color32::from_gray(185),Color32::from_gray(70))));
    ui.add_space(2.0);
}
fn support_author(ui:&mut egui::Ui){
    let id=egui::Id::new("support-author-open");
    let open=ui.ctx().data(|data|data.get_temp::<bool>(id).unwrap_or(false));
    if !open {release_support_codes(ui.ctx());}
    let dark=ui.visuals().dark_mode;
    let accent=if dark{Color32::from_rgb(182,163,245)}else{Color32::from_rgb(105,77,163)};
    ui.scope(|ui|{
    ui.set_min_height(225.0);
    if !open {
    egui::Frame::new().fill(if dark{Color32::from_rgb(42,37,53)}else{Color32::from_rgb(239,233,250)})
        .corner_radius(10.0).show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.vertical_centered(|ui|{
                ui.add(egui::Image::new(egui::include_image!("../../assets/support/daozai-coffee.svg")).fit_to_exact_size(egui::vec2(211.0,110.0)));
            });
        });
    ui.add_space(9.0);
    ui.label(egui::RichText::new("小唐维护 · 岛仔陪伴").size(10.5).color(accent));
    ui.label(egui::RichText::new("请小唐喝杯咖啡").size(19.0).strong());
    ui.label("岛仔陪你认真生活，\n你的支持，陪小唐把它慢慢做好。");
    } else {
        let ctx=ui.ctx().clone();
        // 只解码一次并裁出二维码区域，按实际显示像素缩小：原图解码后两张合计约 17MB，缩小后不到 1MB。
        let pixels=((180.0*ctx.pixels_per_point()).ceil() as u32).clamp(180,600);
        ui.columns(2,|columns|{
            for (i,(name,bytes,crop,color)) in [
                ("微信支付",include_bytes!("../../assets/support/support-wechat-original.jpg").as_slice(),[330.0,370.0,600.0],Color32::from_rgb(5,160,80)),
                ("支付宝",include_bytes!("../../assets/support/support-alipay-original.jpg").as_slice(),[200.0,660.0,860.0],Color32::from_rgb(22,120,255)),
            ].into_iter().enumerate(){
                let texture_id=egui::Id::new(("support-author-qr",i));
                let texture=ctx.data_mut(|data|data.get_temp::<egui::TextureHandle>(texture_id)).filter(|texture|texture.size()==[pixels as usize;2]).unwrap_or_else(||{
                    let image=support_code_image(bytes,crop,pixels);
                    let texture=ctx.load_texture(name,image,egui::TextureOptions::LINEAR);
                    ctx.data_mut(|data|data.insert_temp(texture_id,texture.clone()));texture
                });
                let uv=egui::Rect::from_min_max(egui::pos2(0.0,0.0),egui::pos2(1.0,1.0));
                columns[i].vertical_centered(|ui|{
                    ui.label(egui::RichText::new(name).color(color).strong());
                    let side=ui.available_width().min(180.0);
                    ui.add(egui::Image::new((texture.id(),egui::vec2(side,side))).uv(uv));
                    ui.label(egui::RichText::new(if i==0{"打开微信扫一扫"}else{"打开支付宝扫一扫"}).size(11.0));
                });
            }
        });
    }
    });
    let button=egui::Button::new(egui::RichText::new(if open{"返回"}else{"请喝咖啡 · 支持作者"}).color(Color32::from_rgb(33,27,51)))
        .fill(Color32::from_rgb(182,163,245)).corner_radius(8.0);
    if ui.add_sized([ui.available_width(),32.0],button).clicked(){
        ui.ctx().data_mut(|data|data.insert_temp(id,!open));
    }
    ui.add_space(7.0);
    ui.label(egui::RichText::new("完全自愿 · 感谢每一份支持 · 不影响功能使用").size(11.0).color(ui.visuals().weak_text_color()));
}
pub(crate) fn settings_control_row(ui:&mut egui::Ui,title:&str,label_width:f32,controls:impl FnOnce(&mut egui::Ui)->egui::Response)->egui::Response{
    ui.horizontal(|ui|{
        // 同一张卡片里的控件左缘对齐：标签列至少 SETTINGS_LABEL_W。
        let (label_rect,_)=ui.allocate_exact_size(egui::vec2(label_width.max(SETTINGS_LABEL_W),30.0),egui::Sense::hover());
        let content=controls(ui);
        ui.painter().text(egui::pos2(label_rect.left(),content.rect.center().y),egui::Align2::LEFT_CENTER,title,egui::TextStyle::Body.resolve(ui.style()),ui.visuals().text_color());
        content
    }).inner
}
pub(crate) fn setting_number<N:egui::emath::Numeric>(ui:&mut egui::Ui,title:&str,value:&mut N,min:N,max:N,suffix:&str,speed:f64)->bool{
    let mut changed=false;
    settings_control_row(ui,title,80.0,|ui|{
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
            ui.allocate_ui_with_layout(egui::vec2(140.0,30.0),egui::Layout::left_to_right(egui::Align::Center),|ui|{
                changed=number_stepper(ui,value,min,max,suffix,N::from_f64(speed),76.0);
            });
        }).response
    });
    changed
}
/// 0.5 秒一档的时长设置：1–15 秒，显示时去掉多余的零。
fn setting_seconds(ui:&mut egui::Ui,title:&str,value:&mut f64)->bool{
    let ctx=&ui.ctx().clone();
    let mut changed=false;
    settings_control_row_right(ui,title,|ui|{
        ui.horizontal(|ui|{
            ui.spacing_mut().item_spacing.x=6.0;
            let button=|text|egui::Button::new(egui::RichText::new(text).size(16.0).color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))))
                .fill(settings_color(ctx,Color32::from_rgb(48,48,51),Color32::from_rgb(229,232,238))).stroke(egui::Stroke::NONE).corner_radius(13.0);
            if ui.add_enabled(*value>1.0,button("−").min_size(egui::vec2(26.0,26.0))).on_hover_text("减少 0.5 秒").clicked(){
                *value=(((*value-0.5)*2.0).round()/2.0).max(1.0);changed=true;
            }
            let field=ui.add_sized([62.0,28.0],egui::DragValue::new(value).range(1.0..=15.0).speed(0.5).suffix(" 秒").custom_formatter(|value,_|format!("{value}")));
            if field.changed(){*value=((*value*2.0).round()/2.0).clamp(1.0,15.0);changed=true;}
            if ui.add_enabled(*value<15.0,button("+").min_size(egui::vec2(26.0,26.0))).on_hover_text("增加 0.5 秒").clicked(){
                *value=(((*value+0.5)*2.0).round()/2.0).min(15.0);changed=true;
            }
        });
    });
    changed
}
pub(crate) fn number_stepper<N:egui::emath::Numeric>(ui:&mut egui::Ui,value:&mut N,min:N,max:N,suffix:&str,step:N,field_width:f32)->bool{
    let ctx=&ui.ctx().clone();
    let mut changed=false;
    ui.scope(|ui|{
        ui.spacing_mut().item_spacing.x=6.0;
        ui.spacing_mut().button_padding=egui::vec2(4.0,2.0);
        let button=|text|egui::Button::new(egui::RichText::new(text).size(16.0).color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))))
            .fill(settings_color(ctx,Color32::from_rgb(48,48,51),Color32::from_rgb(229,232,238))).stroke(egui::Stroke::NONE).corner_radius(13.0);
        let decrease=ui.add_enabled(*value>min,button("−").min_size(egui::vec2(26.0,26.0))).on_hover_text("减少");
        if decrease.clicked(){
            *value=N::from_f64((value.to_f64()-step.to_f64()).max(min.to_f64()));changed=true;
        }
        let field=ui.add_sized([field_width,28.0],egui::DragValue::new(value).range(min..=max).speed(step.to_f64()).suffix(suffix).fixed_decimals(if N::INTEGRAL{0}else{2}));
        changed|=field.changed();
        if ui.add_enabled(*value<max,button("+").min_size(egui::vec2(26.0,26.0))).on_hover_text("增加").clicked(){
            *value=N::from_f64((value.to_f64()+step.to_f64()).min(max.to_f64()));changed=true;
        }
    });
    changed
}
/// 时段行的宽度：两个加减器（−、输入框、+ 各间隔 6）加中间的「至」和两侧间距。
/// 区域刚好装下内容，靠右放置时右边缘就和单个加减器对齐。
const TIME_RANGE_HOURS_W:f32=(26.0+6.0+48.0+6.0+26.0)*2.0+8.0+13.0+8.0;
const TIME_RANGE_CLOCK_W:f32=(26.0+6.0+56.0+6.0+26.0)*2.0+8.0+13.0+8.0;
/// 时刻加减器（一天中的分钟数，15 分钟一档，跨午夜循环），中间可直接输入 12:30。
fn time_stepper(ui:&mut egui::Ui,value:&mut u32,id:&str)->bool{
    let ctx=&ui.ctx().clone();
    let mut changed=false;
    ui.push_id(id,|ui|{
        ui.spacing_mut().item_spacing.x=6.0;
        ui.spacing_mut().button_padding=egui::vec2(4.0,2.0);
        let button=|text|egui::Button::new(egui::RichText::new(text).size(16.0).color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))))
            .fill(settings_color(ctx,Color32::from_rgb(48,48,51),Color32::from_rgb(229,232,238))).stroke(egui::Stroke::NONE).corner_radius(13.0);
        if ui.add(button("−").min_size(egui::vec2(26.0,26.0))).on_hover_text("提前 15 分钟").clicked(){*value=(*value+1440-15)%1440;changed=true;}
        let field=ui.add_sized([56.0,28.0],egui::DragValue::new(value).range(0..=1439).speed(1.0)
            .custom_formatter(|v,_|{let m=v as u32;format!("{:02}:{:02}",m/60,m%60)})
            .custom_parser(|t|{let (h,m)=t.trim().split_once([':','：'])?;Some((h.parse::<u32>().ok()?.min(23)*60+m.parse::<u32>().ok()?.min(59)) as f64)}));
        changed|=field.changed();
        if ui.add(button("+").min_size(egui::vec2(26.0,26.0))).on_hover_text("推后 15 分钟").clicked(){*value=(*value+15)%1440;changed=true;}
    });
    changed
}
fn setting_choices(ui:&mut egui::Ui,title:&str,value:&mut u32,values:[u32;3],unit:&str){
    let labels=values.map(|v|format!("{v} {unit}"));
    let options:Vec<(u32,&str)>=values.iter().zip(labels.iter()).map(|(v,l)|(*v,l.as_str())).collect();
    settings_control_row_right(ui,title,|ui|{settings_segmented(ui,value,&options,62.0);});
}
/// 设置页分页序号。旧序号保持不变（岛上入口、测试都按序号跳转）。
pub(crate) const SETTINGS_TAB_BODY: usize = 16;
pub(crate) const SETTINGS_TAB_BEHAVIOR: usize = 14;
pub(crate) const SETTINGS_TAB_REMINDERS: usize = 6;
pub(crate) const SETTINGS_TAB_CLIPBOARD: usize = 17;
pub(crate) const SETTINGS_TAB_WINDOWS: usize = 18;
/// 外观、功能页面已并入岛体；旧序号仍可跳转。
pub(crate) const SETTINGS_TAB_APPEARANCE: usize = 13;
pub(crate) const SETTINGS_TAB_PAGES: usize = 10;
/// 侧栏分组：标题与该组在 `SETTINGS_GROUPS` 中的起止下标（左闭右开）。
pub(crate) const SETTINGS_SECTIONS: [(&str,usize,usize);3] = [("应用",0,4),("功能",4,16),("其他",16,17)];
/// 侧栏：一项就是一页。外观、功能页面并入岛体，旧序号由 `settings_tab_normalized` 转过去。
pub(crate) const SETTINGS_GROUPS: [(&str,usize,usize);17] = [
    ("通用",0,0), ("岛体",SETTINGS_TAB_BODY,SETTINGS_TAB_BODY), ("行为",SETTINGS_TAB_BEHAVIOR,SETTINGS_TAB_BEHAVIOR), ("提醒",SETTINGS_TAB_REMINDERS,SETTINGS_TAB_REMINDERS),
    ("音乐",1,1), ("专注",2,2), ("健康",3,3), ("待办",4,4), ("置物架",5,5), ("剪贴板",SETTINGS_TAB_CLIPBOARD,SETTINGS_TAB_CLIPBOARD), ("邮箱",7,7), ("日程",8,8), ("AI 额度",11,11), ("AI 会话",15,15), ("天气",12,12), ("Windows 通知",SETTINGS_TAB_WINDOWS,SETTINGS_TAB_WINDOWS),
    ("关于",9,9),
];
/// 「功能」组跟随岛上的页面顺序。
pub(crate) fn settings_feature_rows(page_order:[usize;12])->Vec<(&'static str,usize,usize)>{
    page_order.into_iter().map(|page|{
        let tab=crate::island::PAGE_SETTINGS_TABS[page];
        (crate::island::PAGE_NAMES[page],tab,tab)
    }).collect()
}
/// 外观（13）与功能页面（10）并入岛体；旧序号仍能打开合并后的页面。
fn settings_tab_normalized(tab:usize)->usize{
    match tab {SETTINGS_TAB_APPEARANCE|SETTINGS_TAB_PAGES=>SETTINGS_TAB_BODY,t=>t}
}
/// 分页标题；旧序号显示合并后的页面名。
fn settings_tab_title(tab:usize)->&'static str{
    let tab=settings_tab_normalized(tab);
    SETTINGS_GROUPS.iter().find(|(_,start,end)|(*start..=*end).contains(&tab)).map_or("通用",|g|g.0)
}
#[cfg(test)]
pub(crate) fn settings_tab_group_for_tests(tab:usize)->&'static str{settings_tab_title(tab)}

impl App {
    /// 功能页顶部卡片：名称、说明和开关。开关按钮与「功能页面」是同一个状态，点一下即切换。
    /// 返回功能是否开启，调用方据此把下面的设置置灰。
    pub(crate) fn settings_feature_header(&mut self,ui:&mut egui::Ui,page:usize,description:&str)->bool{
        let ctx=&ui.ctx().clone();
        let on=self.module_on(page);
        // 和「功能页面」一致：只剩最后一个功能页时不允许关闭。
        let can_change=!on || self.settings.visible_pages.iter().filter(|enabled|**enabled).count()>1;
        let mut toggle=None;
        settings_card(ui,|ui|{
            ui.horizontal(|ui|{
                ui.add(egui::Image::new(settings_page_icon(page)).fit_to_exact_size(egui::vec2(36.0,36.0)));
                ui.add_space(4.0);
                ui.vertical(|ui|{
                    ui.spacing_mut().item_spacing.y=2.0;
                    ui.label(egui::RichText::new(PAGE_NAMES[page]).size(15.0).strong().color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))));
                    ui.label(egui::RichText::new(description).size(11.5).color(settings_color(ctx,Color32::from_gray(175),Color32::from_gray(90))));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                    // 开着时「关闭」，关着时「开启」，和功能页面里的开关同步；开启蓝色，关闭红色。
                    let action=if on{settings_danger_button("关闭")}else{settings_primary_button("开启")};
                    if ui.add_enabled(can_change,action).clicked(){toggle=Some(!on);}
                    settings_state_chip(ui,on);
                });
            });
            if !on {
                ui.label(egui::RichText::new(format!("「{}」已关闭，不会在后台同步或提醒",PAGE_NAMES[page])).size(12.0).color(settings_color(ctx,Color32::from_rgb(238,181,91),Color32::from_rgb(139,88,12))));
            }
        });
        if let Some(enabled)=toggle {self.set_page_visible(page,enabled);}
        self.module_on(page)
    }
    pub fn settings_ui(&mut self,ctx:&egui::Context,autostart:bool){
        if !self.settings_open{self.pending_page_order=None;self.window_layout().close();release_support_codes(ctx);self.weather_source.close();}
        self.updates.poll(ctx);
        self.updates.dialog_ui(ctx);
        if !self.settings_open{return;}
        let id=egui::ViewportId::from_hash_of("daozai-settings");
        let focus_requested=std::mem::take(&mut self.settings_focus_requested);
        let opening=self.window_layout().open();
        ctx.show_viewport_immediate(id,egui::ViewportBuilder::default().with_title("岛仔设置").with_icon(theme::app_icon()).with_inner_size([690.0,560.0]).with_min_inner_size([600.0,450.0]).with_visible(false).with_active(false).with_window_level(egui::WindowLevel::Normal),|ctx,_|{
            let restored=opening && self.window_layout().restore(ctx);
            if !restored{self.window_layout().observe(ctx);}
            let dark=self.settings.settings_theme.dark(crate::island::settings_theme::system_dark(ctx));
            let original_style=ctx.style();
            let mut style=(*original_style).clone();
            style.visuals=if dark{egui::Visuals::dark()}else{egui::Visuals::light()};
            // 侧栏与正文之间的竖线、弹窗里的分隔线都用这条淡线；浅色模式不再是一条深灰粗线。
            // 侧栏边框已去掉，这里是唯一的一条：浅色模式一条淡线，深色模式不画线，靠两块底色区分。
            style.visuals.widgets.noninteractive.bg_stroke=if dark{egui::Stroke::NONE}else{egui::Stroke::new(1.0,Color32::from_rgb(226,229,234))};
            ctx.set_style(style);
            ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(if dark{egui::SystemTheme::Dark}else{egui::SystemTheme::Light}));
            if self.settings.settings_theme==crate::island::settings_theme::SettingsTheme::System {
                ctx.request_repaint_after(std::time::Duration::from_secs(1));
            }
            if focus_requested {
                // Glow 在首帧交换缓冲后执行窗口命令，避免先显示白色空窗口。
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            if ctx.input(|i|i.viewport().close_requested()){self.settings_open=false;self.quota_close_connection();}
            let before=format!("{:?}",self.settings);
            egui::SidePanel::left("settings-sidebar").resizable(false).exact_width(180.0).frame(egui::Frame::new().fill(settings_color(ctx,Color32::from_rgb(29,29,32),Color32::from_rgb(237,239,243))).stroke(egui::Stroke::NONE).inner_margin(egui::Margin::symmetric(10,12))).show(ctx,|ui|{
                ui.add_space(2.0);
                // 17 项加三个分组标题和两条组间分隔线：窗口拉到最矮时按可用高度压缩行高。
                // 每项占 行高 + 控件间距(3) + 行后空隙；三个分组标题和两条分隔线预留约 96。
                let per=(ui.available_height()-96.0)/SETTINGS_GROUPS.len() as f32;
                let row_h=(per-6.0).clamp(18.0,32.0);
                let row_gap=(per-3.0-row_h).clamp(0.0,3.0);
                // 最小高度时行间距一并收紧，三个分组标题才放得下。
                if per<24.0 {ui.spacing_mut().item_spacing.y=2.0;}
                // 侧栏自己的分隔线：深色主题全局把非交互描边设成了 NONE。
                ui.visuals_mut().widgets.noninteractive.bg_stroke=egui::Stroke::new(1.0,settings_color(ctx,Color32::from_rgb(52,52,57),Color32::from_rgb(226,229,234)));
                let selected_tab=settings_tab_normalized(self.settings_tab);
                let feature_rows=settings_feature_rows(self.settings.page_order);
                for (section_index,(section,from,to)) in SETTINGS_SECTIONS.iter().enumerate() {
                    let rows:&[(&str,usize,usize)]=if section_index==1 {&feature_rows}else{&SETTINGS_GROUPS[*from..*to]};
                    // 组间一条横线，间距统一；组内只靠行距区分。
                    if section_index==0 {ui.add_space(2.0);}
                    else {ui.add_space(3.0);ui.add(egui::Separator::default().spacing(1.0));ui.add_space(3.0);}
                    ui.label(egui::RichText::new(*section).size(11.0).color(settings_color(ctx,Color32::from_gray(150),Color32::from_gray(105))));
                    ui.add_space(3.0);
                    for &(name,start,end) in rows {
                        let selected=(start..=end).contains(&selected_tab);
                        let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),row_h),egui::Sense::click());
                        if response.clicked() && !selected{self.settings_tab=start;}
                        if selected || response.hovered(){
                            ui.painter().rect_filled(rect,7.0,if selected{settings_color(ctx,Color32::from_rgb(43,43,46),Color32::from_rgb(214,221,232))}else{settings_color(ctx,Color32::from_rgb(30,30,33),Color32::from_rgb(225,230,238))});
                        }
                        let icon=match start {
                            0=>egui::include_image!("../../assets/settings-icons/system/general.svg"),
                            SETTINGS_TAB_BODY=>egui::include_image!("../../assets/settings-icons/system/body.svg"),
                            11=>egui::include_image!("../../assets/settings-icons/system/quota.svg"),
                            1=>egui::include_image!("../../assets/settings-icons/system/music.svg"),
                            2=>egui::include_image!("../../assets/settings-icons/system/focus.svg"),
                            3=>egui::include_image!("../../assets/settings-icons/system/health.svg"),
                            4=>egui::include_image!("../../assets/settings-icons/system/tasks.svg"),
                            5=>egui::include_image!("../../assets/settings-icons/system/shelf.svg"),
                            SETTINGS_TAB_CLIPBOARD=>egui::include_image!("../../assets/settings-icons/system/clipboard.svg"),
                            6=>egui::include_image!("../../assets/settings-icons/system/notifications.svg"),
                            12=>egui::include_image!("../../assets/settings-icons/system/weather.svg"),
                            7=>egui::include_image!("../../assets/settings-icons/system/mail.svg"),
                            8=>egui::include_image!("../../assets/settings-icons/system/calendar.svg"),
                            15=>egui::include_image!("../../assets/settings-icons/system/sessions.svg"),
                            SETTINGS_TAB_WINDOWS=>egui::include_image!("../../assets/settings-icons/system/notifications.svg"),
                            SETTINGS_TAB_BEHAVIOR=>egui::include_image!("../../assets/settings-icons/system/position.svg"),
                            _=>egui::include_image!("../../assets/settings-icons/system/about.svg"),
                        };
                        let icon_bg=egui::Rect::from_center_size(egui::pos2(rect.left()+21.0,rect.center().y),egui::vec2(23.0,23.0));
                        egui::Image::new(icon).paint_at(ui,icon_bg);
                        ui.painter().text(egui::pos2(rect.left()+40.0,rect.center().y),egui::Align2::LEFT_CENTER,name,egui::FontId::proportional(13.0),if selected{settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))}else{settings_color(ctx,Color32::from_gray(225),Color32::from_gray(55))});
                        ui.add_space(row_gap);
                    }
                }
            });
            egui::CentralPanel::default().frame(egui::Frame::new().fill(settings_color(ctx,Color32::from_rgb(25,25,27),Color32::from_rgb(246,247,249))).inner_margin(egui::Margin::symmetric(22,16))).show(ctx,|ui|{
                settings_apply_style(ui);
                ui.label(egui::RichText::new(settings_tab_title(self.settings_tab)).size(18.0).strong().color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))));
                ui.add_space(6.0);
                // 外观（13）、功能页面（10）已并入岛体，旧序号仍能打开对应页面。
                let tab=settings_tab_normalized(self.settings_tab);
                egui::ScrollArea::vertical().id_salt(("settings-scroll",tab)).auto_shrink([false,false]).show(ui,|ui|{
                    // 右侧给滚动条留出空隙，卡片和开关不会被它压住。
                    ui.set_width((ui.available_width()-12.0).max(240.0));
                    match tab {
                    15=>{
                        let on=self.settings_feature_header(ui,crate::island::AI_PAGE,"监控本机 Agent 会话，在岛上审批与回答");
                        ui.add_space(4.0);
                        ui.add_enabled_ui(on,|ui|self.agent_settings(ui));
                    }
                    SETTINGS_TAB_WINDOWS=>{
                        let on=self.settings_feature_header(ui,crate::island::WINDOWS_PAGE,"即时通讯以外的系统通知汇总，横幅结束后保留铃铛");
                        ui.add_space(4.0);
                        ui.add_enabled_ui(on,|ui|{
                            settings_card(ui,|ui|{
                                ui.label(egui::RichText::new(format!("本次运行已记录 {} 条通知",self.windows_notices.len())).size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                                settings_action_row(ui,|ui|{
                                    if ui.add(settings_danger_button("清空记录")).clicked(){self.windows_notices.clear();self.windows_first=0;}
                                });
                            });
                        });
                    }
                    12=>{
                        let on=self.settings_feature_header(ui,crate::island::WEATHER_PAGE,"当地天气与七日预报");
                        ui.add_space(4.0);
                        ui.add_enabled_ui(on,|ui|self.weather_settings(ui));
                    }
                    11=>{
                        let on=self.settings_feature_header(ui,crate::island::QUOTA_PAGE,"每 3 分钟自动更新剩余额度");
                        ui.add_space(4.0);
                        ui.add_enabled_ui(on,|ui|self.quota_settings(ui));
                    }
                    0=>{
                        settings_label(ui,"设置页外观");
                        settings_card(ui,|ui|{
                            use crate::island::settings_theme::SettingsTheme;
                            settings_control_row_right(ui,"主题",|ui|{
                                if settings_segmented(ui,&mut self.settings.settings_theme,&[(SettingsTheme::System,"跟随系统"),(SettingsTheme::Light,"浅色"),(SettingsTheme::Dark,"深色")],52.0) {ctx.request_repaint();}
                            });
                            settings_hint(ui,"仅影响设置页，岛的外观保持不变。");
                        });
                        settings_label(ui,"时间格式");
                        settings_card(ui,|ui|{
                            settings_control_row_right(ui,"时间制式",|ui|{settings_segmented(ui,&mut self.settings.clock_12h,&[(false,"24 小时"),(true,"12 小时")],72.0);});
                            settings_control_row_right(ui,"时段文字",|ui|{settings_segmented(ui,&mut self.settings.clock_period,&[(0,"不显示"),(1,"上午 / 下午"),(2,"详细时段")],72.0);});
                            if self.settings.clock_period==2 {settings_hint(ui,"详细时段：凌晨、上午、中午、下午、傍晚、晚上");}
                            ui.separator();
                            setting_row(ui,"显示秒数","时间显示为 14:32:08",&mut self.settings.clock_seconds);
                        });
                        settings_label(ui,"启动");
                        settings_card(ui,|ui|{
                            let mut a=autostart;
                            setting_row(ui,"开机启动","",&mut a);
                            if a!=autostart{self.pending.push(Cmd::ToggleAutostart);}
                        });
                        settings_label(ui,"高级");
                        settings_card(ui,|ui|{
                            let before=self.settings.render_compat;
                            setting_row(ui,"兼容渲染（Direct3D 12）","黑框、闪烁或设置崩溃时开启；重启生效",&mut self.settings.render_compat);
                            // 手动关闭后不再显示自动切换的原因；再次开启视为手动开启。
                            if before!=self.settings.render_compat {self.settings.render_compat_reason.clear();}
                            if !self.settings.render_compat_reason.is_empty() {settings_hint(ui,&self.settings.render_compat_reason);}
                            if self.settings.render_compat!=self.render_compat_running {
                                ui.horizontal(|ui|{
                                    ui.label(egui::RichText::new("兼容渲染重启岛仔后生效").size(12.0).color(Color32::from_rgb(230,150,40)));
                                    if ui.button("立即重启").clicked(){self.pending.push(Cmd::RestartApp);}
                                });
                            }
                            ui.separator();
                            setting_row(ui,"OLED 像素偏移","防止 OLED 烧屏：收起静止时缓慢移动几个像素",&mut self.settings.oled_shift);
                        });
                    }
                    SETTINGS_TAB_BODY=>{
                        settings_label(ui,"岛体外观");
                        settings_card(ui,|ui|{
                            use crate::model::{self as model,Skin};
                            let skins:Vec<(Skin,&str)>=Skin::ALL.iter().map(|s|(*s,s.label())).collect();
                            settings_control_row_right(ui,"皮肤",|ui|{settings_segmented(ui,&mut self.settings.skin,&skins,64.0);});
                            settings_hint(ui,self.settings.skin.description());
                            if self.settings.skin.is_glass() {
                                let mut blur=self.settings.glass_blur;
                                setting_row(ui,"背景模糊","需开启系统“透明效果”；关闭后改为烟熏半透明",&mut blur);
                                self.settings.glass_blur=blur;
                            }
                            ui.separator();
                            let shapes:Vec<(model::IslandShape,&str)>=model::IslandShape::ALL.iter().map(|s|(*s,s.label())).collect();
                            settings_control_row_right(ui,"形状",|ui|{settings_segmented(ui,&mut self.settings.island_shape,&shapes,64.0);});
                            settings_hint(ui,self.settings.island_shape.description());
                            ui.separator();
                            settings_control_row(ui,"大小",56.0,|ui|{
                                ui.horizontal_wrapped(|ui|{
                                    // 给百分比和恢复按钮留出空间，宽窗口仍使用原来的轨道长度。
                                    ui.spacing_mut().slider_width=(ui.available_width()-150.0).clamp(64.0,180.0);
                                    island_size_slider(ui,&mut self.settings.island_size);
                                    if ui.add_enabled(self.settings.island_size!=100,egui::Button::new("恢复默认")).clicked(){self.settings.island_size=100;}
                                }).response
                            });
                            settings_hint(ui,"只影响岛体，放大后文字和图标依旧清晰。");
                            ui.separator();
                            let selected=self.displays.iter().find(|d|d.id==self.settings.display_id);
                            let label=if self.settings.display_id.is_empty(){"跟随主屏"}else{selected.map_or("所选屏幕未连接",|d|d.label.as_str())};
                            settings_control_row(ui,"显示屏",56.0,|ui|{
                                egui::ComboBox::from_id_salt("island-display").selected_text(label).width(ui.available_width().min(300.0)).truncate().show_ui(ui,|ui|{
                                    ui.selectable_value(&mut self.settings.display_id,String::new(),"跟随主屏");
                                    for display in &self.displays {ui.selectable_value(&mut self.settings.display_id,display.id.clone(),&display.label);}
                                }).response
                            });
                            // 只在所选屏幕断开时说明，平时不占位置。
                            if !self.settings.display_id.is_empty()&&selected.is_none(){settings_hint(ui,"暂时显示在主屏，重新连接所选屏幕后自动恢复。");}
                            ui.separator();
                            // 停靠边缘：顶部为横向；左右为竖向排列，展开面板从边缘滑出。
                            let docks:Vec<(crate::model::Dock,&str)>=crate::model::Dock::ALL.iter().map(|d|(*d,d.label())).collect();
                            settings_control_row_right(ui,"停靠",|ui|{
                                // 换边后偏移的含义变了（水平 ↔ 上下），回到居中。
                                if settings_segmented(ui,&mut self.settings.dock,&docks,64.0) {self.settings.island_offset=0;}
                            });
                            if self.settings.dock.is_side() {settings_hint(ui,"左右停靠时竖向排列；吉祥物和岛上歌词只在顶部停靠时显示。");}
                            let side=self.settings.dock.is_side();
                            settings_control_row(ui,if side{"上下位置"}else{"水平位置"},56.0,|ui|{
                                ui.horizontal_wrapped(|ui|{
                                    let limit=crate::model::ISLAND_OFFSET_MAX;
                                    let offset=&mut self.settings.island_offset;
                                    let (less,more,less_tip,more_tip)=if side{("↑","↓","向上微调","向下微调")}else{("←","→","向左微调","向右微调")};
                                    if ui.add_enabled(*offset>-limit,egui::Button::new(less).min_size(egui::vec2(26.0,22.0))).on_hover_text(less_tip).clicked(){*offset=(*offset-5).max(-limit);}
                                    // 左侧微调按钮已占位，剩余宽度还要容纳右侧微调和恢复居中。
                                    ui.spacing_mut().slider_width=(ui.available_size_before_wrap().x-134.0).clamp(64.0,150.0);
                                    ui.add(egui::Slider::new(&mut *offset,-limit..=limit).show_value(false));
                                    if ui.add_enabled(*offset<limit,egui::Button::new(more).min_size(egui::vec2(26.0,22.0))).on_hover_text(more_tip).clicked(){*offset=(*offset+5).min(limit);}
                                    if ui.add_enabled(*offset!=0,egui::Button::new("恢复居中")).clicked(){*offset=0;}
                                }).response
                            });
                            let offset=self.settings.island_offset;
                            let (less,more)=if side{("偏上","偏下")}else{("偏左","偏右")};
                            let position=match offset {0=>"居中".to_string(),o if o<0=>format!("{less} {}%",-o),o=>format!("{more} {o}%")};
                            settings_hint(ui,&format!("当前：{position}。也可以展开后按住日期时间直接拖动。"));
                        });
                        settings_label(ui,"岛上显示");
                        settings_card(ui,|ui|{
                            self.resident_order_settings(ui);
                            ui.separator();
                            self.resident_groups_settings(ui);
                        });
                        settings_label(ui,"功能页面");
                        settings_card(ui,|ui|{
                            let descriptions=["播放控制与同步歌词","番茄钟与休息计时","喝水与久坐提醒","临时收纳文件位置","任务清单与到期提醒","邮件摘要与验证码","Agent 剩余额度和账号切换","月历、邮件邀请与工作日程","当地天气与七日预报","Agent 会话、审批与提示音","即时通讯以外的系统通知汇总","最近复制的文字"];
                            ui.label(egui::RichText::new("展开后显示哪些功能页、顺序如何。拖动左侧手柄调整顺序；关闭的功能不在后台运行，不同步也不提醒，已有数据保留。").size(11.5).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                            #[derive(Clone)]
                            struct PageDrag(usize);
                            let mut movement=None;
                            let mut jump=None;
                            let row_w=ui.available_width();
                            let mut order=self.pending_page_order.unwrap_or(self.settings.page_order);
                            for (index,page) in order.into_iter().enumerate() {
                                if index>0 {ui.separator();}
                                let mut visible=self.settings.visible_pages[page];
                                let can_change=!visible || self.settings.visible_pages.iter().filter(|enabled|**enabled).count()>1;
                                let row=ui.push_id(("page-order",page),|ui|ui.horizontal(|ui|{
                                    ui.set_max_width(row_w);
                                    let (rect,handle)=ui.allocate_exact_size(egui::vec2(20.0,40.0),egui::Sense::drag());
                                    for x in [-3.0,3.0] {for y in [-6.0,0.0,6.0] {
                                        ui.painter().circle_filled(rect.center()+egui::vec2(x,y),1.4,settings_color(ctx,Color32::from_gray(155),Color32::from_gray(100)));
                                    }}
                                    handle.on_hover_cursor(egui::CursorIcon::Grab).dnd_set_drag_payload(PageDrag(page));
                                    // 每行右侧「设置 ›」直接跳到这个功能的设置页，开关仍在最右。
                                    let jump_w=58.0;
                                    ui.allocate_ui(egui::vec2((ui.available_width()-jump_w-ui.spacing().item_spacing.x).max(160.0),40.0),|ui|{
                                        ui.add_enabled_ui(can_change,|ui|{setting_row(ui,PAGE_NAMES[page],descriptions[page],&mut visible);});
                                    });
                                    if ui.add_sized([jump_w,26.0],egui::Button::new(egui::RichText::new("设置 ›").size(12.0)).frame(false)).on_hover_text(format!("打开「{}」设置",PAGE_NAMES[page])).clicked(){jump=Some(crate::island::PAGE_SETTINGS_TABS[page]);}
                                })).inner.response;
                                if let Some(payload)=row.dnd_hover_payload::<PageDrag>() {
                                    if payload.0!=page {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                        let y=if order.iter().position(|p|*p==payload.0).unwrap()<index {row.rect.bottom()}else{row.rect.top()};
                                        ui.painter().line_segment([egui::pos2(row.rect.left(),y),egui::pos2(row.rect.right(),y)],egui::Stroke::new(2.0,Color32::from_rgb(70,150,255)));
                                    }
                                }
                                if let Some(payload)=row.dnd_release_payload::<PageDrag>() {movement=Some((payload.0,index));}
                                if visible!=self.settings.visible_pages[page] {self.set_page_visible(page,visible);}
                            }
                            if let Some(tab)=jump {self.settings_tab=tab;}
                            if let Some((page,to))=movement {
                                let from=order.iter().position(|p|*p==page).unwrap();
                                if from<to {order[from..=to].rotate_left(1);}
                                else {order[to..=from].rotate_right(1);}
                                self.pending_page_order=Some(order);
                                ui.ctx().request_repaint();
                            }
                            if self.settings.visible_pages.iter().filter(|enabled|**enabled).count()==1 {
                                ui.add_space(5.0);
                                ui.label(egui::RichText::new("至少保留一个页面。").size(11.5).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                            }
                            let changed=self.pending_page_order.is_some_and(|pending|pending!=self.settings.page_order);
                            settings_action_row(ui,|ui|{
                                let button=if changed{settings_primary_button("保存")}else{egui::Button::new("保存")};
                                if ui.add_enabled(changed,button).clicked() {
                                    if let Some(pending)=self.pending_page_order.take() {
                                        self.settings.page_order=pending;
                                        ui.ctx().request_repaint();
                                    }
                                }
                            });
                        });
                    }
                    SETTINGS_TAB_BEHAVIOR=>{
                        settings_label(ui,"岛体组件");
                        settings_card(ui,|ui|{
                            settings_control_row_right(ui,"岛上显示",|ui|{
                                if ui.add(egui::Button::new("打开 ›").frame(false)).clicked(){self.settings_tab=SETTINGS_TAB_BODY;}
                            });
                        });
                        settings_label(ui,"横幅提醒");
                        settings_card(ui,|ui|{
                            setting_row(ui,"提醒总开关","",&mut self.settings.notifications);
                            ui.add_enabled_ui(self.settings.notifications,|ui|{
                                ui.separator();
                                let mut mode:u8=if !self.settings.system_notify{0}else if self.settings.sound{2}else{1};
                                settings_control_row_right(ui,"系统提醒",|ui|{settings_segmented(ui,&mut mode,&[(0,"仅岛上"),(1,"加系统通知"),(2,"系统通知 + 声音")],96.0);});
                                self.settings.system_notify=mode>0;
                                self.settings.sound=mode==2;
                                settings_hint(ui,"只影响岛仔自己的系统提醒；即时通讯和 Windows 通知不跟随，AI 提示音跟随声音开关。");
                                ui.separator();
                                setting_seconds(ui,"横幅停留",&mut self.settings.notice_hold_secs);
                            });
                        });
                        settings_label(ui,"展开与收起");
                        settings_card(ui,|ui|{
                            settings_control_row_right(ui,"动画速度",|ui|{
                                settings_segmented(ui,&mut self.settings.motion_speed,&[
                                    (crate::model::MotionSpeed::Slow,"慢"),
                                    (crate::model::MotionSpeed::Normal,"标准"),
                                    (crate::model::MotionSpeed::Fast,"快"),
                                ],52.0);
                            });
                            settings_hint(ui,"只影响岛体展开、收起和悬停胀大。");
                            ui.separator();
                            setting_seconds(ui,"展开后停留",&mut self.settings.page_hold_secs);
                            settings_hint(ui,"同时作用于打开功能页和展开未读胶囊。");
                        });
                        settings_label(ui,"避让");
                        settings_card(ui,|ui|{
                            setting_row(ui,"全屏时隐藏","全屏隐藏岛仔，健康提醒静默保留",&mut self.settings.hide_fullscreen);
                            ui.separator();
                            setting_row(ui,"顶栏适配","顶部有菜单栏或任务栏时使用",&mut self.settings.avoid_top_bar);
                            if self.settings.avoid_top_bar {
                                settings_control_row_right(ui,"显示方式",|ui|{
                                    settings_segmented(ui,&mut self.settings.top_bar_on_top,&[(false,"避开顶栏"),(true,"岛仔置顶")],90.0);
                                });
                                if self.settings.top_bar_on_top {
                                    settings_hint(ui,"岛体显示在顶栏上方。");
                                } else {
                                    let reserved=crate::os::display_layout::selected(&self.displays,&self.settings.display_id).map_or(0,|d|{
                                        let b=crate::os::display_layout::placement_bounds(d,true);(b.top-d.bounds.top).max(0)
                                    });
                                    settings_hint(ui,&if reserved>0 {format!("已检测到顶栏，岛体下移 {reserved} 像素。")}else{"未检测到顶栏；没有向系统登记的顶栏可尝试“岛仔置顶”。".to_string()});
                                }
                            }
                            ui.separator();
                            self.avoidance.rows(ui,&mut self.settings);
                        });
                    }
                    1=>self.music_settings(ui),
                    2=>{
                        let on=self.settings_feature_header(ui,crate::island::FOCUS_PAGE,"番茄钟与休息计时");
                        ui.add_enabled_ui(on,|ui|{
                            settings_card(ui,|ui|{
                                setting_number(ui,"专注",&mut self.settings.focus_minutes,1,180," 分钟",1.0);
                                ui.separator();
                                setting_number(ui,"休息",&mut self.settings.rest_minutes,1,60," 分钟",1.0);
                                settings_hint(ui,"调整后从下一阶段开始生效。");
                            });
                        });
                    }
                    3=>{
                        let on=self.settings_feature_header(ui,crate::island::WATER_PAGE,"喝水与久坐提醒");
                        ui.add_enabled_ui(on,|ui|{
                            settings_card(ui,|ui|{
                                // 喝水：开关一行，数量与间隔紧随其后。
                                let old_remind=self.water.remind;
                                setting_row(ui,"喝水提醒","确认或喝水后重新计时；达标后停止提醒",&mut self.water.remind);
                                if self.water.remind!=old_remind{self.water.save();self.confirm_health_reminder(HealthReminder::Water);}
                                ui.add_enabled_ui(self.water.remind,|ui|{
                                    setting_number(ui,"提醒间隔",&mut self.settings.water_minutes,5,180," 分钟",1.0);
                                    setting_number(ui,"每次记录",&mut self.settings.serving_ml,50,1000," ml",25.0);
                                    if setting_number(ui,"每日目标",&mut self.water.goal_ml,500,6000," ml",50.0){self.water.save();}
                                });
                                ui.separator();
                                let old_sit=self.settings.sit_remind;
                                setting_row(ui,"久坐提醒","确认提醒或点「起身了」后重新计时",&mut self.settings.sit_remind);
                                if self.settings.sit_remind!=old_sit {self.confirm_health_reminder(HealthReminder::Sit);}
                                ui.add_enabled_ui(self.settings.sit_remind,|ui|{
                                    setting_number(ui,"久坐时长",&mut self.settings.sit_minutes,15,180," 分钟",1.0);
                                });
                                ui.separator();
                                // 提醒时段与午休对喝水和久坐都生效。
                                // 喝水和久坐提醒都关掉时，时段和午休不起作用，一起置灰。
                                let any_reminder=self.water.remind||self.settings.sit_remind;
                                ui.add_enabled_ui(any_reminder,|ui|{
                                    settings_control_row(ui,"提醒时段",80.0,|ui|{
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                                            // 区域宽度正好等于两个加减器加「至」，靠右放置后与上面单个加减器的右边缘对齐。
                                            ui.allocate_ui_with_layout(egui::vec2(TIME_RANGE_HOURS_W,30.0),egui::Layout::left_to_right(egui::Align::Center),|ui|{
                                                number_stepper(ui,&mut self.settings.water_start,0,23," 时",1,48.0);
                                                ui.label("至");
                                                number_stepper(ui,&mut self.settings.water_end,0,24," 时",1,48.0);
                                            });
                                        }).response
                                    });
                                    setting_row(ui,"午休不提醒","午休时间内喝水和久坐都不弹提醒",&mut self.settings.lunch_rest);
                                    if self.settings.lunch_rest {
                                        settings_control_row(ui,"午休时间",80.0,|ui|{
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                                                ui.allocate_ui_with_layout(egui::vec2(TIME_RANGE_CLOCK_W,30.0),egui::Layout::left_to_right(egui::Align::Center),|ui|{
                                                    time_stepper(ui,&mut self.settings.lunch_start,"lunch-start");
                                                    ui.label("至");
                                                    time_stepper(ui,&mut self.settings.lunch_end,"lunch-end");
                                                });
                                            }).response
                                        });
                                    }
                                    settings_hint(ui,"提醒时段和午休对喝水、久坐都生效；专注计时进行中不打扰，结束后再提醒。");
                                });
                            });
                        });
                    }
                    4=>{
                        let on=self.settings_feature_header(ui,crate::island::TODO_PAGE,"任务清单与到期提醒");
                        let _=on;
                    }
                    5=>{
                        let on=self.settings_feature_header(ui,crate::island::SHELF_PAGE,"临时收纳文件位置");
                        ui.add_enabled_ui(on,|ui|{
                            settings_card(ui,|ui|{
                                ui.label(egui::RichText::new(format!("当前 {} 项 · 移除条目不会删除原文件",self.shelf.items.len())).size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                            });
                        });
                    }
                    SETTINGS_TAB_CLIPBOARD=>{
                        let on=self.settings_feature_header(ui,crate::island::CLIPBOARD_PAGE,"记录最近复制的文字，点一下再次复制");
                        ui.add_enabled_ui(on,|ui|{
                            settings_card(ui,|ui|{
                                setting_row(ui,"记录剪贴板","在剪贴板页查看，点一下再次复制",&mut self.settings.clipboard_history);
                                settings_hint(ui,&format!("本机保存最近 {} 条；密码管理器标记为敏感的内容不会记录。",crate::os::clipboard::MAX_RECENT));
                                if ui.button("打开剪贴板").clicked(){self.settings_open=false;self.show_page(crate::island::CLIPBOARD_PAGE);}
                            });
                        });
                    }
                    6=>{
                        if !self.settings.notifications {settings_hint(ui,"提醒总开关已关闭，可在「行为 → 通知」开启。");}
                        settings_label(ui,"系统状态");
                        settings_card(ui,|ui|{
                            setting_row(ui,"音量","",&mut self.settings.hud_volume);
                            ui.separator();
                            setting_row(ui,"电源","",&mut self.settings.hud_power);
                            ui.separator();
                            setting_row(ui,"网络","",&mut self.settings.hud_network);
                            ui.separator();
                            setting_row(ui,"键盘","",&mut self.settings.hud_keyboard);
                            ui.separator();
                            setting_row(ui,"蓝牙设备","",&mut self.settings.bluetooth_notifications);
                        });
                        // 聊天软件：一个软件一行、一个开关，监听状态在开关左侧。
                        let master=self.settings.notifications;
                        settings_label(ui,"即时通讯");
                        let sub=|ui:&mut egui::Ui,title:&str,description:&str,value:&mut bool|{
                            ui.horizontal(|ui|{ui.add_space(22.0);ui.vertical(|ui|setting_row(ui,title,description,value));});
                        };
                        let listening=|status:&str|status.starts_with("正在监听");
                        let status_tip=|status:&str|if status.is_empty(){"未监听".to_string()}else{status.to_string()};
                        settings_card(ui,|ui|{
                            ui.add_enabled_ui(master,|ui|{
                                for client in crate::messenger::Messenger::ALL {
                                    if client==crate::island::Messenger::Qq {continue;}
                                    let status=self.messenger_status(client).to_string();
                                    let on=listening(&status);
                                    let value=match client {
                                        crate::island::Messenger::Wechat=>&mut self.settings.wechat_notifications,
                                        crate::island::Messenger::WeCom=>&mut self.settings.wecom_notifications,
                                        crate::island::Messenger::Feishu=>&mut self.settings.feishu_notifications,
                                        _=>&mut self.settings.dingtalk_notifications,
                                    };
                                    setting_row_chip(ui,client.name(),if on{"监听中"}else{"未监听"},on,&status_tip(&status),value);
                                    if client==crate::island::Messenger::Wechat && self.settings.wechat_notifications {
                                        sub(ui,"托盘一键唤起","缩在托盘时点提醒直接唤起，不读取聊天内容",&mut self.settings.wechat_tray_open);
                                    }
                                    ui.separator();
                                }
                                let listener_on=self.settings.read_notifications && listening(&self.listener_status) && self.settings.qq_notifications;
                                setting_row_chip(ui,"QQ",if listener_on{"监听中"}else{"未监听"},listener_on,&self.listener_status,&mut self.settings.qq_notifications);
                            });
                        });
                        settings_label(ui,"AI 会话消息");
                        let ai_on=self.module_on(crate::island::AI_PAGE);
                        ui.add_enabled_ui(ai_on,|ui|self.agent_reminder_settings(ui));
                        if !ai_on {settings_hint(ui,"「AI 会话」功能已在功能页面中关闭。");}
                        settings_label(ui,"Windows 通知");
                        settings_card(ui,|ui|{
                            // 系统通知就是监听开关：开启后自动监听，关闭时无需监听。
                            let current=crate::os::system::system_notifications_enabled();
                            let mut system_on=current;
                            let listener_on=current && self.settings.read_notifications && listening(&self.listener_status);
                            setting_row_chip(ui,"系统通知",if listener_on{"监听中"}else{"未监听"},listener_on,&status_tip(&self.listener_status),&mut system_on);
                            if system_on!=current {
                                match crate::os::system::set_system_notifications(system_on) {
                                    Ok(())=>self.pending.push(if system_on{Cmd::RequestNotifications}else{Cmd::DisconnectNotifications}),
                                    Err(error)=>self.say(error),
                                }
                            }
                            settings_hint(ui,"关闭后所有应用的横幅都会被 Windows 拦截；开启后岛仔自动监听。");
                            ui.separator();
                            setting_row(ui,"显示通知正文","可能包含私密消息，屏幕共享时建议关闭",&mut self.settings.notification_preview);
                            ui.separator();
                            // 开始菜单应用里核对 Windows 通知开关；已收到过通知的托盘应用补在后面。
                            let cache=egui::Id::new("settings-enabled-notification-apps");
                            let cached=ctx.data(|d|d.get_temp::<(f64,Vec<(String,String)>)>(cache));
                            let cached=if cached.as_ref().map_or(true,|(at,_)|self.t-*at>60.0) {
                                let value=(self.t,enabled_notification_apps());
                                ctx.data_mut(|d|d.insert_temp(cache,value.clone()));
                                Some(value)
                            } else {cached};
                            let mut apps:Vec<String>=cached.map(|(_,list)|list).unwrap_or_default().into_iter().map(|(_,name)|name).collect();
                            for (id,name,enabled) in &self.notification_apps.items {
                                if !*enabled || !notification_app_enabled(id)
                                    || crate::messenger::Messenger::from_notification_id(id).is_some()
                                    || crate::messenger::taskbar_client(name,id).is_some()
                                    || id.starts_with("Windows.") || id.starts_with("Microsoft.Explorer.Notification.") || id.starts_with("Microsoft.AutoGenerated.") {
                                    continue;
                                }
                                apps.push(name.clone());
                            }
                            apps.sort();apps.dedup();
                            let title=settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41));
                            ui.label(egui::RichText::new("已开启的应用").size(13.0).color(title));
                            if apps.is_empty() {
                                ui.label(egui::RichText::new("暂无，收到其他应用的通知后会显示在这里。").size(11.5).color(settings_color(ctx,Color32::from_gray(170),Color32::from_gray(92))));
                            } else {
                                for name in &apps {
                                    ui.separator();
                                    ui.label(egui::RichText::new(name).size(13.0).color(title));
                                }
                            }
                            ui.add_space(4.0);
                            settings_action_row(ui,|ui|{
                                if ui.button("打开 Windows 通知设置").clicked(){
                                    #[cfg(windows)] {let _=crate::support::child_process::spawn(std::process::Command::new("explorer.exe").arg("ms-settings:notifications"));}
                                }
                            });
                        });
                    }
                    7=>{
                        let mut add_mail=false;
                        let on=self.settings_feature_header(ui,crate::island::MAIL_PAGE,"新邮件提醒与验证码；未读邮件在岛上保留邮箱图标");
                        ui.add_enabled_ui(on,|ui|{
                        settings_label(ui,"设置");
                        settings_card(ui,|ui|{
                            setting_row(ui,"验证码高亮","识别到验证码时在岛上放大显示，点击即复制",&mut self.settings.mail_code);
                            ui.separator();
                            let sync_before=self.settings.mail_sync_operations;
                            ui.add_enabled_ui(self.mail.manage.pending.is_empty(),|ui|{
                                setting_row(ui,"同步邮件操作到邮箱","已读、删除、恢复同步到邮箱；关闭时只改岛仔记录",&mut self.settings.mail_sync_operations);
                            });
                            if sync_before!=self.settings.mail_sync_operations{self.mail_manage_mode_changed();}
                            ui.separator();
                            ui.horizontal(|ui|{
                                ui.vertical(|ui|{
                                    ui.spacing_mut().item_spacing.y=2.0;
                                    ui.label(egui::RichText::new("会议提前").size(13.0).color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))));
                                    ui.label(egui::RichText::new(format!("{} 分钟 · 与日程共用",self.settings.mail_ics)).size(11.5).color(settings_color(ctx,Color32::from_gray(175),Color32::from_gray(90))));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{if ui.button("在日程中设置 ›").clicked(){self.settings_tab=8;}});
                            });
                            settings_advanced(ui,"settings-mail-advanced",|ui|{
                                setting_choices(ui,"验证码隐藏",&mut self.settings.mail_code_hide_seconds,[30,60,300],"秒");
                                settings_hint(ui,"明确有效时长更短时提前隐藏；隐藏不代表失效，主动查看后 15 秒再次隐藏。");
                                setting_choices(ui,"检查间隔",&mut self.settings.mail_interval,[60,120,300],"秒");
                                settings_hint(ui,"支持推送的邮箱实时收信；间隔只用于不支持推送的邮箱。");
                            });
                        });
                        ui.horizontal(|ui|{
                            settings_label(ui,"邮箱账号");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM),|ui|{if ui.button("＋ 添加邮箱").clicked(){add_mail=true;}});
                        });
                        settings_card(ui,|ui|{
                            if self.mail.accounts.is_empty(){ui.label(egui::RichText::new("还没有邮箱。点「＋ 添加邮箱」，保存后自动连接。").size(12.0).color(settings_color(ctx,Color32::from_gray(195),Color32::from_gray(60))));}
                            let total=self.mail.accounts.len();
                            let(mut changed,mut remove,mut test)=(false,None,None);
                            for i in 0..total{
                                let(email,provider)=(self.mail.accounts[i].email.clone(),self.mail.accounts[i].provider);
                                let status=self.mail.status_of(&email).label();
                                let mut on=self.mail.accounts[i].enabled;
                                if i>0{ui.separator();}
                                ui.label(egui::RichText::new(&email).size(13.0).color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))));
                                ui.label(egui::RichText::new(format!("{} · {}",provider.label(),status)).size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                                ui.horizontal(|ui|{
                                    if ui.add_sized([66.0,30.0],egui::Button::new("测试")).clicked(){test=Some(i);}
                                    if ui.add_sized([66.0,30.0],egui::Button::new("删除")).clicked(){remove=Some(i);}
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new("启用").size(12.0).color(settings_color(ctx,Color32::from_gray(190),Color32::from_gray(65))));
                                    if switch(ui,&mut on){self.mail.accounts[i].enabled=on;changed=true;}
                                });
                            }
                            if changed{self.mail_accounts_changed();}
                            if let Some(i)=remove{
                                let email=self.mail.accounts[i].email.clone();
                                let credential=self.mail.accounts[i].credential_key();
                                self.mail.accounts.remove(i);
                                crate::mail::secrets::delete(&credential);
                                self.mail.status.remove(&email.to_ascii_lowercase());
                                self.mail.note=format!("{email}：已移除，凭据已从系统凭据管理器删除");
                                self.mail_accounts_changed();
                            }
                            if let Some(i)=test{
                                let account=self.mail.accounts[i].clone();
                                if let Err(error)=account.endpoint(){
                                    self.mail.note=error;
                                }else{
                                match crate::mail::secrets::load(&account.credential_key()){
                                    Ok(pass)=>{self.mail.testing=Some(account.email.clone());self.mail.note=format!("{}：正在测试连接…",account.email);self.pending.push(Cmd::MailTest(account,pass));}
                                    Err(e)=>self.mail.note=format!("{}：{e}",account.email),
                                }
                                }
                            }
                                                    if !self.mail.note.is_empty(){ui.add_space(3.0);ui.add(egui::Label::new(egui::RichText::new(&self.mail.note).size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75)))).wrap());}
                        });
                        });
                        // 添加邮箱：一次性的长表单放进弹窗，页面上只保留账号列表。
                        let form_id=egui::Id::new("settings-mail-add");
                        if add_mail {self.mail.form.open=true;}
                        if self.mail.form.open {
                            let mut close=false;
                            let modal=egui::Modal::new(form_id).frame(settings_modal_frame(ctx)).backdrop_color(Color32::from_black_alpha(110)).show(ctx,|ui|{
                            ui.set_width(440.0);
                            settings_apply_style(ui);if settings_modal_title(ui,"添加邮箱"){close=true;}
                            ui.spacing_mut().item_spacing.y=8.0;
                            settings_control_row(ui,"邮箱服务",56.0,|ui|{
                                egui::ComboBox::from_id_salt("mail-provider").width(160.0).selected_text(self.mail.form.provider.label()).show_ui(ui,|ui|{
                                    for p in crate::mail::Provider::ALL{if ui.selectable_label(self.mail.form.provider==p,p.label()).clicked(){self.mail.form.provider=p;}}
                                }).response
                            });
                            ui.add_space(5.0);
                            let microsoft=self.mail.form.provider==crate::mail::Provider::Microsoft;
                            ui.label(egui::RichText::new(if microsoft{"连接微软邮箱"}else{"获取授权码或专用密码"}).size(12.5).strong());
                            ui.add(egui::Label::new(egui::RichText::new(self.mail.form.provider.hint()).size(12.0).color(settings_color(ctx,Color32::from_gray(200),Color32::from_gray(55)))).wrap());
                            let (label,url)=self.mail.form.provider.setup_link();
                            if !url.is_empty() && ui.link(label).clicked(){self.pending.push(Cmd::OpenUrl(url.into()));}
                            ui.add_space(8.0);
                            if self.mail.form.provider==crate::mail::Provider::Custom {
                                settings_control_row(ui,"IMAP 主机",80.0,|ui|{ui.text_edit_singleline(&mut self.mail.form.custom.host)});
                                settings_control_row(ui,"TLS 端口",80.0,|ui|{ui.add(egui::DragValue::new(&mut self.mail.form.custom.port).range(1..=65535))});
                                settings_control_row(ui,"登录用户名",80.0,|ui|{ui.add(egui::TextEdit::singleline(&mut self.mail.form.custom.username).vertical_align(egui::Align::Center).hint_text("留空使用邮箱地址"))});
                            }
                            if !microsoft{
                            let input_width=(ui.available_width()-56.0-48.0-2.0*ui.spacing().item_spacing.x).max(100.0);
                            settings_control_row(ui,"邮箱",56.0,|ui|{
                                ui.add_sized([input_width,30.0],egui::TextEdit::singleline(&mut self.mail.form.email).vertical_align(egui::Align::Center).hint_text("输入邮箱地址"))
                            });
                            let target=crate::mail::Account{custom:(self.mail.form.provider==crate::mail::Provider::Custom).then(||self.mail.form.custom.clone()),email:self.mail.form.email.clone(),provider:self.mail.form.provider,enabled:true};
                            let unsupported=target.endpoint().err().as_deref()==Some(crate::mail::MICROSOFT_UNSUPPORTED);
                            if unsupported{ui.add(egui::Label::new(crate::mail::MICROSOFT_UNSUPPORTED).wrap());}
                            ui.add_enabled_ui(!unsupported,|ui|{
                            settings_control_row(ui,"授权码",56.0,|ui|{
                                let field=ui.add_sized([input_width,30.0],egui::TextEdit::singleline(&mut self.mail.form.password).vertical_align(egui::Align::Center).password(!self.mail.form.show).hint_text("输入邮箱授权码"));
                                if ui.add_sized([48.0,30.0],egui::Button::new(if self.mail.form.show{"隐藏"}else{"显示"})).clicked(){self.mail.form.show=!self.mail.form.show;}
                                field
                            });
                            ui.horizontal(|ui|{
                                if ui.button("测试连接").clicked(){
                                    let email=self.mail.form.email.trim().to_string();
                                    let target=crate::mail::Account{custom:(self.mail.form.provider==crate::mail::Provider::Custom).then(||self.mail.form.custom.clone()),email:email.clone(),provider:self.mail.form.provider,enabled:true};
                                    if !email.contains('@'){self.mail.note="请先填邮箱地址".into();}
                                    else if let Err(error)=target.endpoint(){self.mail.note=error;}
                                    else if self.mail.form.password.is_empty(){self.mail.note="请先填授权码（不是邮箱登录密码）".into();}
                                    else{
                                        self.mail.testing=Some(email.clone());
                                        self.mail.note=format!("{email}：正在测试连接…");
                                        self.pending.push(Cmd::MailTest(crate::mail::Account {custom: (self.mail.form.provider==crate::mail::Provider::Custom).then(||self.mail.form.custom.clone()), email:email.clone(),provider:self.mail.form.provider,enabled:true},self.mail.form.password.clone()));
                                    }
                                }
                                if ui.button("保存并开始监控").clicked(){
                                    let email=self.mail.form.email.trim().to_string();
                                    let target=crate::mail::Account{custom:(self.mail.form.provider==crate::mail::Provider::Custom).then(||self.mail.form.custom.clone()),email:email.clone(),provider:self.mail.form.provider,enabled:true};
                                    if !email.contains('@'){self.mail.note="请先填邮箱地址".into();}
                                    else if let Err(error)=target.endpoint(){self.mail.note=error;}
                                    else if self.mail.form.password.is_empty(){self.mail.note="请先填授权码（不是邮箱登录密码）".into();}
                                    else{
                                        match crate::mail::secrets::save(&target.credential_key(),&self.mail.form.password){
                                            Ok(())=>{
                                                let account=crate::mail::Account {custom: (self.mail.form.provider==crate::mail::Provider::Custom).then(||self.mail.form.custom.clone()), email:email.clone(),provider:self.mail.form.provider,enabled:true};
                                                if let Some(slot)=self.mail.accounts.iter_mut().find(|a|a.email.eq_ignore_ascii_case(&email)){*slot=account;}else{self.mail.accounts.push(account);}
                                                self.mail.form.password.clear();
                                                self.mail.note=format!("{email}：已保存，正在连接…");
                                                self.mail_accounts_changed();
                                            }
                                            Err(e)=>self.mail.note=e,
                                        }
                                    }
                                }
                            });
                            ui.label(egui::RichText::new("授权码存储在 Windows 凭据管理器中。").size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                            });
                            }
                                                    if !self.mail.note.is_empty(){ui.add_space(3.0);ui.add(egui::Label::new(egui::RichText::new(&self.mail.note).size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75)))).wrap());}
                            });
                            // 保存后弹窗保持打开，结果和「测试连接」都在这里；用户自己关闭。
                            if close || modal.should_close() {self.mail.form.open=false;}
                        }
                    }
                    8=>{
                        ui.spacing_mut().item_spacing.y=4.0;
                        let mut open_add=false;
                        let on=self.settings_feature_header(ui,crate::island::CALENDAR_PAGE,"月历、邮件邀请与工作日程");
                        ui.add_enabled_ui(on,|ui|{
                        settings_label(ui,"设置");
                        settings_card(ui,|ui|{
                            setting_row(ui,"自动同步","每 5 分钟更新日程；关闭后可点「立即同步」",&mut self.settings.calendar_auto_sync);
                            ui.separator();
                            setting_choices(ui,"会议提前",&mut self.settings.mail_ics,[5,15,30],"分钟");
                            settings_hint(ui,"与邮件中的会议邀请共用提醒，同一场会议只提醒一次。");
                        });
                        settings_label(ui,"日历账号");
                        let mut import=false;
                        settings_card(ui,|ui|{
                        if let Some(cfg)=self.calendar.config.clone(){
                                let n=self.calendar.events.len();
                                // 一行：平台图标、名称与状态，右侧立即同步和断开。
                                ui.horizontal(|ui|{
                                    let (rect,_)=ui.allocate_exact_size(egui::vec2(28.0,28.0),egui::Sense::hover());
                                    ui.painter().rect_filled(rect,7.0,Color32::from_rgb(51,112,255));
                                    ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,cfg.provider.label().chars().next().unwrap_or('日'),egui::FontId::proportional(13.0),Color32::WHITE);
                                    ui.vertical(|ui|{
                                        ui.spacing_mut().item_spacing.y=2.0;
                                        ui.label(egui::RichText::new(format!("{} · 最后同步 {}",cfg.provider.label(),if self.calendar.last_sync.is_empty(){"—".to_string()}else{self.calendar.last_sync.clone()})).size(13.0).color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))));
                                        ui.label(egui::RichText::new(if self.calendar.connected{format!("● 已连接 · 未来 24 小时 {n} 个日程")}else if self.calendar.failures==0{"● 正在连接…".into()}else{format!("● 上次连接失败 · 已知 {n} 个日程")}).size(11.5).color(if self.calendar.connected{settings_color(ctx,Color32::from_rgb(120,220,160),Color32::from_rgb(24,121,64))}else{settings_color(ctx,Color32::from_rgb(240,180,110),Color32::from_rgb(145,89,12))}));
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                                        if ui.add_enabled(!self.calendar.testing,egui::Button::new("断开")).clicked(){self.pending.push(Cmd::CalendarForget);}
                                        if ui.button("立即同步").clicked(){self.calendar.note="正在同步…".into();self.calendar.note_error=false;self.pending.push(Cmd::CalendarSync);}
                                    });
                                });
                                if let Some(feed)=self.calendar.feeds.iter().find(|feed|feed.config==cfg) {
                                    let status=if feed.month.loading {"正在同步本月日程…".to_owned()}
                                        else if !feed.month.error.is_empty(){format!("本月同步失败：{}",feed.month.error)}
                                        else if feed.month.loaded==Some(self.calendar.view.month){format!("{} {} {} 个日程",self.calendar.view.month.format("%Y-%m"),if feed.month.warning.is_empty(){"已同步"}else{"已读取"},feed.month.events.len())}
                                        else {String::new()};
                                    if !status.is_empty(){ui.add(egui::Label::new(egui::RichText::new(status).size(12.0)).wrap());}
                                    if !feed.month.warning.is_empty(){ui.add(egui::Label::new(egui::RichText::new(format!("本月部分日程未同步：{}",feed.month.warning)).size(12.0).color(settings_color(ctx,Color32::from_rgb(238,181,91),Color32::from_rgb(139,88,12)))).wrap());}
                                }
                                if !self.calendar.warning.is_empty(){ui.add(egui::Label::new(egui::RichText::new(format!("同步提示：{}",self.calendar.warning)).size(12.0).color(settings_color(ctx,Color32::from_rgb(238,181,91),Color32::from_rgb(139,88,12)))).wrap());}
                                if !self.calendar.error.is_empty(){
                                    ui.add(egui::Label::new(egui::RichText::new(&self.calendar.error).size(12.0).color(settings_color(ctx,Color32::from_rgb(240,96,96),Color32::from_rgb(184,34,34)))).wrap());
                                }
                                ui.separator();
                        }else{
                            ui.label(egui::RichText::new("还没有连接日历。添加飞书、钉钉等日历，或导入 .ics 文件。").size(12.0).color(settings_color(ctx,Color32::from_gray(195),Color32::from_gray(60))));
                        }
                        ui.horizontal(|ui|{
                            if ui.button(if self.calendar.config.is_some(){"更换日历"}else{"＋ 添加日历"}).clicked(){open_add=true;}
                            if ui.button("导入 .ics 文件").clicked(){import=true;}
                        });
                        if !self.calendar.import_note.is_empty(){settings_hint(ui,&self.calendar.import_note);}
                        });
                        if import {self.pending.push(Cmd::CalendarImport);}
                        });
                        // 添加日历：选平台、填账号、获取方法、导入 .ics 都在弹窗里。
                        let add_id=egui::Id::new("settings-calendar-add");
                        if open_add {self.calendar.form.open=true;self.calendar.form.guide_open=true;}
                        if self.calendar.form.open {
                            let mut close=false;
                            let connected_before=self.calendar.config.clone();
                            let modal=egui::Modal::new(add_id.with("modal")).frame(settings_modal_frame(ctx)).backdrop_color(Color32::from_black_alpha(110)).show(ctx,|ui|{
                            ui.set_width(460.0);
                            ui.spacing_mut().item_spacing.y=6.0;
                            settings_apply_style(ui);if settings_modal_title(ui,"添加日历"){close=true;}
                            settings_label(ui,"选择日历平台");
                            egui::Frame::new().fill(settings_color(ctx,Color32::from_rgb(29,29,31),Color32::from_rgb(236,239,244))).corner_radius(8.0).inner_margin(3).show(ui,|ui|{
                                ui.set_min_width(ui.available_width());
                                ui.spacing_mut().item_spacing.x=2.0;
                                // 六个平台分两行：飞书 / 钉钉 / 企业微信，iCloud / CalDAV / Exchange。
                                let width=((ui.available_width()-4.0)/3.0).max(80.0);
                                for row in crate::calendar::Provider::ALL.chunks(3){
                                    ui.horizontal(|ui|{
                                        for &p in row{
                                            let selected=self.calendar.form.provider==p;
                                            let button=egui::Button::new(egui::RichText::new(p.label()).size(13.0).color(if selected{settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))}else{settings_color(ctx,Color32::from_gray(190),Color32::from_gray(65))}))
                                                .fill(if selected{settings_color(ctx,Color32::from_rgb(81,81,85),Color32::from_rgb(208,217,231))}else{Color32::TRANSPARENT})
                                                .stroke(egui::Stroke::NONE).corner_radius(6.0);
                                            let button=ui.add_enabled_ui(!self.calendar.testing,|ui|ui.add_sized([width,30.0],button)).inner;
                                            let button=match p{
                                                crate::calendar::Provider::ICloud=>button.on_hover_text("苹果日历：Apple ID + App 专用密码"),
                                                crate::calendar::Provider::Custom=>button.on_hover_text("任意 CalDAV 日历：Nextcloud、群晖、Fastmail 等"),
                                                crate::calendar::Provider::Exchange=>button.on_hover_text("公司自建或第三方 Exchange 日历（ActiveSync），只读同步"),
                                                _=>button,
                                            };
                                            if button.clicked() && !selected{
                                                self.select_calendar_provider(p);
                                            }
                                        }
                                    });
                                }
                            });
                            ui.add_space(7.0);
                            ui.horizontal_top(|ui|{
                                ui.allocate_ui_with_layout(egui::vec2(50.0,18.0),egui::Layout::left_to_right(egui::Align::Center),|ui|{ui.set_min_size(egui::vec2(50.0,18.0));ui.label(egui::RichText::new("服务器").size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));});
                                if self.calendar.form.provider.custom_server(){
                                    ui.add_sized([ui.available_width(),22.0],egui::TextEdit::singleline(&mut self.calendar.form.server).vertical_align(egui::Align::Center)
                                        .hint_text(egui::RichText::new(self.calendar.form.provider.server_hint()).color(settings_color(ctx,Color32::from_gray(155),Color32::from_gray(100)))));
                                }else{
                                    ui.add(egui::Label::new(egui::RichText::new(self.calendar.form.provider.server()).size(12.0).color(settings_color(ctx,Color32::from_gray(205),Color32::from_gray(55)))).wrap());
                                }
                            });
                            settings_label(ui,"连接账号");
                            ui.scope(|ui|{
                                ui.visuals_mut().extreme_bg_color=settings_color(ctx,Color32::from_rgb(25,25,28),Color32::from_rgb(244,246,249));
                            let input_width=(ui.available_width()-50.0-48.0-2.0*ui.spacing().item_spacing.x).max(80.0);
                                ui.horizontal(|ui|{
                                    ui.allocate_ui_with_layout(egui::vec2(50.0,30.0),egui::Layout::left_to_right(egui::Align::Center),|ui|{ui.set_min_size(egui::vec2(50.0,30.0));ui.label(egui::RichText::new("用户名").size(13.0).color(settings_color(ctx,Color32::from_gray(210),Color32::from_gray(55))));});
                                    let user=ui.add_sized([input_width,30.0],egui::TextEdit::singleline(&mut self.calendar.form.username).vertical_align(egui::Align::Center)
                                        .hint_text(egui::RichText::new("输入用户名或粘贴账号信息").color(settings_color(ctx,Color32::from_gray(155),Color32::from_gray(100)))));
                                    if user.changed(){
                                        let pasted=self.calendar.form.username.clone();
                                        if let Some((u,p))=crate::calendar::parse_paste(&pasted){
                                            self.calendar.form.username=u;
                                            self.calendar.form.password=p;
                                            self.calendar.note="已从粘贴内容中填入用户名和密码".into();
                                            self.calendar.note_error=false;
                                        }
                                    }
                                });
                                ui.add_space(5.0);
                                ui.horizontal(|ui|{
                                    ui.allocate_ui_with_layout(egui::vec2(50.0,30.0),egui::Layout::left_to_right(egui::Align::Center),|ui|{ui.set_min_size(egui::vec2(50.0,30.0));ui.label(egui::RichText::new("密码").size(13.0).color(settings_color(ctx,Color32::from_gray(210),Color32::from_gray(55))));});
                                    ui.add_sized([input_width,30.0],egui::TextEdit::singleline(&mut self.calendar.form.password).vertical_align(egui::Align::Center)
                                        .password(!self.calendar.form.show).hint_text(egui::RichText::new("输入平台生成的密码").color(settings_color(ctx,Color32::from_gray(155),Color32::from_gray(100)))));
                                    if ui.add_sized([48.0,30.0],egui::Button::new(if self.calendar.form.show{"隐藏"}else{"显示"})).clicked(){self.calendar.form.show=!self.calendar.form.show;}
                                });
                            });
                            ui.add_space(7.0);
                            let guide=if self.calendar.form.guide_open{"如何获取用户名和密码？（点击收起）"}else{"如何获取用户名和密码？"};
                            if ui.link(egui::RichText::new(guide).color(settings_color(ctx,Color32::from_rgb(120,185,245),Color32::from_rgb(26,100,177)))).clicked(){self.calendar.form.guide_open=!self.calendar.form.guide_open;}
                            if self.calendar.form.guide_open{
                                ui.add_space(4.0);
                                for(i,step)in self.calendar.form.provider.guide().iter().enumerate(){
                                    ui.label(egui::RichText::new(format!("{}. {}",i+1,step)).size(12.0).color(settings_color(ctx,Color32::from_gray(200),Color32::from_gray(55))));
                                }
                            }
                            ui.add_space(9.0);
                            let busy=self.calendar.testing;
                            let label=if busy{"正在测试连接…"}else{"测试并保存"};
                            let btn=egui::Button::new(egui::RichText::new(label).size(13.0).strong().color(Color32::WHITE))
                                .fill(if busy{settings_color(ctx,Color32::from_rgb(70,70,76),Color32::from_rgb(213,219,227))}else{Color32::from_rgb(45,105,165)})
                                .corner_radius(7.0);
                            if ui.add_enabled_ui(!busy,|ui|ui.add_sized([130.0,32.0],btn)).inner.clicked(){
                                let user=self.calendar.form.username.trim().to_string();
                                let custom=self.calendar.form.provider.custom_server();
                                let server=crate::calendar::normalize_server(&self.calendar.form.server);
                                if custom && server.is_none(){self.calendar.note="请填写以 https:// 开头的 CalDAV 服务器地址".into();self.calendar.note_error=true;}
                                else if user.is_empty(){self.calendar.note="请先填用户名".into();self.calendar.note_error=true;}
                                else if self.calendar.form.password.is_empty(){self.calendar.note="请先填密码".into();self.calendar.note_error=true;}
                                else{
                                    let cfg=crate::calendar::Config{provider:self.calendar.form.provider,username:user,href:None,server:if custom{server}else{None}};
                                    self.calendar.testing=true;
                                    self.calendar.test_password=self.calendar.form.password.clone();
                                    self.calendar.note="正在连接日历…".into();
                                    self.calendar.note_error=false;
                                    self.pending.push(Cmd::CalendarTest(cfg,self.calendar.form.password.clone()));
                                }
                            }
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new("连接成功后保存，密码存储在 Windows 凭据管理器中。").size(11.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                            if !self.calendar.note.is_empty(){
                                ui.add_space(5.0);
                                let color=if self.calendar.note_error{settings_color(ctx,Color32::from_rgb(255,125,125),Color32::from_rgb(184,34,34))}else{settings_color(ctx,Color32::from_gray(210),Color32::from_gray(55))};
                                ui.add(egui::Label::new(egui::RichText::new(&self.calendar.note).size(12.0).color(color)).wrap());
                            }
                            });
                            // 连接成功（日历配置变化）后自动关闭弹窗。
                            if self.calendar.config.is_some() && self.calendar.config!=connected_before && !self.calendar.testing {close=true;}
                            if close || modal.should_close() {self.calendar.form.open=false;}
                        }
                    }
                    _=>{
                        settings_label(ui,"岛仔 Daozai");
                        settings_card(ui,|ui|{
                            ui.horizontal(|ui|{
                                ui.label(egui::RichText::new(format!("版本 {}",env!("CARGO_PKG_VERSION"))).size(13.0));
                                ui.add_space(8.0);
                                ui.vertical(|ui|self.updates.ui(ui));
                            });
                            ui.label(egui::RichText::new("桌面顶部的音乐、专注、提醒与日程小助手。").size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                            ui.horizontal_wrapped(|ui|{
                                if ui.button("打开本地数据目录").clicked(){self.pending.push(Cmd::Open(crate::model::config_dir()));}
                                if ui.button("收集诊断").clicked(){
                                    // 另起 `daozai.exe --collect-diagnostics`：在新窗口里收集，诊断包保存到桌面并自动选中。
                                    if crate::os::diagnostics_tool::launch().is_err(){self.say("无法启动诊断收集，请从开始菜单打开「收集岛仔诊断」");}
                                }
                                ui.label(egui::RichText::new("设备图标：Solar Icons · 480 Design").size(11.0).color(settings_color(ctx,Color32::from_gray(150),Color32::from_gray(105))));
                                ui.hyperlink_to("CC BY 4.0", "https://creativecommons.org/licenses/by/4.0/");
                            });
                        });
                        settings_label(ui,"支持作者");
                        settings_card(ui,support_author);
                    }
                    }
                });
            });
            self.avoidance.dialog(ctx,&mut self.settings);
            if before!=format!("{:?}",self.settings){self.settings.save();}
            ctx.set_style(original_style);
        });
        if !self.settings_open{self.window_layout().close();self.weather_source.close();}else{self.weather_tutorial_ui(ctx);}
    }
}

/// 收款码只在「支持作者」卡片展开时占用贴图；收起卡片或关闭设置窗口即释放。
fn release_support_codes(ctx:&egui::Context){
    ctx.data_mut(|data|for i in 0..2usize {data.remove::<egui::TextureHandle>(egui::Id::new(("support-author-qr",i)));});
}

/// 从原图裁出二维码区域 `[x, y, 边长]` 并缩放到 `pixels` 见方。
fn support_code_image(bytes:&[u8],crop:[f32;3],pixels:u32)->egui::ColorImage {
    let image=image::load_from_memory(bytes).expect("内置收款码图片有效");
    let side=(crop[2] as u32).min(image.width().saturating_sub(crop[0] as u32)).min(image.height().saturating_sub(crop[1] as u32));
    let code=image.crop_imm(crop[0] as u32,crop[1] as u32,side,side).resize_exact(pixels,pixels,image::imageops::FilterType::Lanczos3).to_rgba8();
    egui::ColorImage::from_rgba_unmultiplied([pixels as usize,pixels as usize],code.as_raw())
}

#[test]
fn support_codes_are_cropped_small_and_released_when_hidden(){
    for (bytes,crop) in [(include_bytes!("../../assets/support/support-wechat-original.jpg").as_slice(),[330.0,370.0,600.0]),(include_bytes!("../../assets/support/support-alipay-original.jpg").as_slice(),[200.0,660.0,860.0])] {
        let image=support_code_image(bytes,crop,360);
        assert_eq!(image.size,[360,360]);
        assert!(image.pixels.len()*4<=600*1024,"每张收款码贴图应小于 0.6MB");
        // 裁剪结果应是二维码本身：同时含有大量深色与浅色像素。
        let dark=image.pixels.iter().filter(|p|p.r()<80).count() as f32/image.pixels.len() as f32;
        assert!((0.2..0.8).contains(&dark),"裁剪区域不像二维码：深色占比 {dark}");
    }
    let ctx=egui::Context::default();
    let _=ctx.run(Default::default(),|ctx|{
        let texture=ctx.load_texture("qr",egui::ColorImage::new([4,4],Color32::WHITE),Default::default());
        ctx.data_mut(|data|data.insert_temp(egui::Id::new(("support-author-qr",0usize)),texture));
    });
    assert_eq!(ctx.tex_manager().read().allocated().filter(|(_,meta)|meta.name=="qr").count(),1);
    release_support_codes(&ctx);
    let _=ctx.run(Default::default(),|_|{});
    assert_eq!(ctx.tex_manager().read().allocated().filter(|(_,meta)|meta.name=="qr").count(),0,"收起后应释放收款码贴图");
}
