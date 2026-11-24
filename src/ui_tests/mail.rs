//! 邮件页、收信、验证码与全文阅读。
use super::*;

#[test]
fn v0649_settings_and_mail_accounts_survive_upgrade(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let _h=H::new();
    let directory=crate::model::config_dir();
    std::fs::create_dir_all(&directory).unwrap();
    let legacy=concat!("settings_theme=dark\npet=0\nminimal=1\nbrowser_minimal=1\n",
        "visible_pages=10111111\npage_order=5,2,0,1,3,4,6,7\nshow_battery=0\n",
        "system_hud=0\nread_notifications=1\nnotifications=0\nsound=1\n",
        "lyrics_enabled=0\nisland_lyrics=1\nlyric_delay_ms=-650\nhide_fullscreen=0\n",
        "focus_minutes=45\nrest_minutes=10\nwater_minutes=60\nserving_ml=300\nwater_start=8\nwater_end=23\n",
        "mail_enabled=0\nmail_code=0\nmail_interval=300\nmail_ics=30\ncalendar_prompt=0\ncalendar_auto_sync=0\n");
    std::fs::write(directory.join("settings.txt"),legacy).unwrap();
    let upgraded=crate::model::Settings::load();
    assert!(upgraded.display_id.is_empty() && upgraded.avoid_apps.is_empty());
    assert!(upgraded.avoid_browsers && upgraded.bluetooth_notifications);
    assert!(!upgraded.clock_12h && !upgraded.notification_preview);
    assert!(!upgraded.hud_volume && !upgraded.hud_power && !upgraded.hud_network && !upgraded.hud_keyboard,"旧总开关关闭时四类系统提醒都应关闭");
    assert_eq!(upgraded.clock_period,0);assert_eq!(upgraded.mail_code_hide_seconds,60);
    for _ in 0..2 {
        crate::model::Settings::load().save();
        let saved=std::fs::read_to_string(directory.join("settings.txt")).unwrap();
        for line in legacy.lines().filter(|line|!line.starts_with("visible_pages=")&&!line.starts_with("page_order=")
            &&!line.starts_with("minimal=")&&!line.starts_with("system_hud=")){assert!(saved.lines().any(|value|value==line),"升级保存不能改变旧设置：{line}");}
        for key in ["hud_volume=0","hud_power=0","hud_network=0","hud_keyboard=0"]{assert!(saved.lines().any(|value|value==key),"旧总开关应迁移到四项独立设置：{key}");}
        assert!(saved.lines().any(|value|value=="visible_pages=101111111111"),"旧页面开关应保留，并默认开启天气页、AI 会话页、Windows 通知页与剪贴板页");
        assert!(saved.lines().any(|value|value=="page_order=5,2,0,1,3,11,4,6,7,8,9,10"),"旧顺序应保留，并把天气页、AI 会话页、Windows 通知页接在最后，剪贴板接在置物架后");
    }
    std::fs::write(directory.join("mail-accounts.json"),br#"[{"email":"OldUser@qq.com","provider":"qq","enabled":true},{"email":"old@icloud.com","provider":"icloud","enabled":false}]"#).unwrap();
    let accounts=crate::mail::load_accounts();
    assert_eq!(accounts.len(),2);assert!(accounts.iter().all(|account|account.custom.is_none()));
    assert_eq!(accounts[0].credential_key(),"olduser@qq.com");
    assert_eq!(accounts[0].endpoint().unwrap(),("imap.qq.com".into(),993,"OldUser@qq.com".into()));
    assert!(!accounts[1].enabled);
    crate::mail::save_accounts(&accounts);assert_eq!(crate::mail::load_accounts(),accounts);
}

#[test]
fn quality_removed_mail_accounts_ignore_in_flight_results() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.mail_event(crate::mail::Event::Status{account:"removed@example.com".into(),status:crate::mail::Status::Ready});
    h.app.mail_event(crate::mail::Event::Mail{item:crate::mail::Item {received_unix: None, account:"removed@example.com".into(),from:"发件人".into(),subject:"延迟邮件".into(),time:String::new(),code:None,html: String::new().into(), summary:String::new().into(),links:vec![],meeting:None,uid:1},new:true});
    assert!(h.app.mail.status.is_empty() && h.app.mail.recent.is_empty());
    assert!(h.app.notification_bell_until<=h.t);
}

#[test]
fn alphabetic_mail_codes_reach_the_card_without_changing_case() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (kind,value) in ["text/plain","text/html"].into_iter().flat_map(|kind|["AbCdEf","WWWWWWWW"].map(|value|(kind,value))) {
        let mut h=H::new();
        h.app.mail.accounts=vec![crate::mail::Account{custom:None,email:"test@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
        let body=if kind=="text/html"{format!("<p>Your verification code:</p><p><b>{value}</b></p>")}else{format!("Your verification code:\n{value}")};
        let raw=format!("From: Example <security@example.invalid>\r\nSubject: Your verification code\r\nContent-Type: {kind}; charset=utf-8\r\n\r\n{body}");
        let parsed=crate::mail::parse_message(raw.as_bytes());
        let code=crate::mail::code_of(&parsed);
        assert_eq!(code.as_deref(),Some(value));
        h.app.mail_event(crate::mail::Event::Mail{new:true,item:crate::mail::Item{html:String::new().into(),received_unix:None,account:"test@qq.com".into(),from:"Example".into(),subject:parsed.subject.clone(),time:"09:41".into(),code,summary:crate::mail::body_of(&parsed).into(),links:vec![],meeting:None,uid:43}});
        h.app.show_page(5);h.hover(H::local(300.0,70.0),1.0);
        let text=h.shapes.iter().find_map(|s|match &s.shape {egui::Shape::Text(t) if t.galley.text()==value=>Some(t),_=>None}).expect("字母验证码应出现在大卡上");
        assert!(text.galley.size().x<=120.5,"最长字母码不能挤入邮件列表：{}",text.galley.size().x);
        if value=="AbCdEf"{assert!(text.galley.job.sections.iter().any(|s|s.format.font_id.size>=23.0));}
        h.cmds.clear();h.click(H::local(60.0,70.0));
        assert!(h.cmds.iter().any(|c|matches!(c,crate::app::Cmd::Copy(copied) if copied==value)));
        h.t+=61.0;h.frame(vec![]);
        assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==value)));
    }
}

#[test]
fn regression_microsoft_traditional_mail_creates_code_card() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for kind in ["text/plain","text/html"] {
        let mut h=H::new();
        h.app.mail.accounts=vec![crate::mail::Account{custom:None,email:"test@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
        let body="我們收到您要申請一次性驗證碼來與 Microsoft 帳戶搭配使用。\n您的一次性驗證碼為：431872\n僅在官方網站或應用程式上輸入此代碼。不要與任何人分享。";
        let body=if kind=="text/html" {format!("<p>{}</p>",body.replace('\n',"</p><p>"))}else{body.into()};
        let raw=format!("From: Microsoft <account-security@example.invalid>\r\nSubject: 您的一次性驗證碼\r\nMIME-Version: 1.0\r\nContent-Type: {kind}; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{body}");
        let parsed=crate::mail::parse_message(raw.as_bytes());
        let code=crate::mail::code_of(&parsed);
        assert_eq!(code.as_deref(),Some("431872"),"{kind} 邮件应提取出验证码");
        h.app.mail_event(crate::mail::Event::Mail{new:true,item:crate::mail::Item{html:String::new().into(),received_unix:None,account:"test@qq.com".into(),from:parsed.from.clone(),subject:parsed.subject.clone(),time:"08:59".into(),code,summary:crate::mail::body_of(&parsed).into(),links:vec![],meeting:None,uid:42}});
        h.app.show_page(5);h.hover(H::local(300.0,70.0),1.0);
        assert!(h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(t) if t.galley.text()=="431872"&&t.galley.job.sections.iter().any(|s|s.format.font_id.size>=23.0))),"新收到的繁体验证码应出现在大卡里");
        h.cmds.clear();h.click(H::local(60.0,70.0));
        assert!(h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Copy(value) if value=="431872")),"大卡应支持点击复制");
        h.t+=61.0;h.frame(vec![]);
        assert!(!h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(t) if t.galley.text()=="431872")),"繁体验证码同样应按隐私设置自动隐藏");
    }
}

#[test]
fn mail_page_shows_code_and_copies_on_click() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.app.mail.accounts = vec![crate::mail::Account {
        custom: None, email: "a@qq.com".into(),
        provider: crate::mail::Provider::Qq,
        enabled: true,
    }];
    h.app.mail_event(crate::mail::Event::Mail {
        item: crate::mail::Item {
            received_unix: None, account: "a@qq.com".into(),
            from: "GitHub".into(),
            subject: "verify".into(),
            time: "09:41".into(),
            code: Some("482913".into()),
            html: String::new().into(), summary: "Enter 482913 to finish signing in.".into(),
            links: vec![],
            meeting: None,
            uid: 42,
        },
        new: true,
    });
    assert!(h.app.notification_bell_until <= h.app.t, "新邮件不点亮 Windows 专用铃铛");
    assert!(h.app.alerts.iter().any(|a|a.title=="新邮件"&&a.page==5),"新邮件应进入通知队列");
    assert!(h.app.mail_unread_count() > 0, "未读邮件应由邮箱图标常驻提示");
    h.app.show_page(5);
    h.hover(Pos2::new(WIN_W / 2.0, 12.0), 0.6);
    h.hover(H::local(300.0, 60.0), 0.4);
    h.click(H::local(60.0, 70.0));
    assert!(
        h.cmds.iter().any(|c| matches!(c, crate::app::Cmd::Copy(t) if t == "482913")),
        "点验证码应复制: {:?}",
        h.cmds
    );
}

#[test]
fn mail_row_opens_summary_and_back_returns() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.app.mail.accounts = vec![crate::mail::Account {
        custom: None, email: "a@qq.com".into(),
        provider: crate::mail::Provider::Qq,
        enabled: true,
    }];
    h.app.mail_event(crate::mail::Event::Mail {
        item: crate::mail::Item {
            received_unix: None, account: "a@qq.com".into(),
            from: "GitHub".into(),
            subject: "verify".into(),
            time: "09:41".into(),
            code: Some("482913".into()),
            html: String::new().into(), summary: "Enter 482913 to finish signing in.".into(),
            links: vec![],
            meeting: None,
            uid: 42,
        },
        new: true,
    });
    h.app.show_page(5);
    h.hover(Pos2::new(WIN_W / 2.0, 12.0), 0.6);
    h.hover(H::local(300.0, 50.0), 0.4);
    h.click(H::local(300.0, 50.0));
    assert_eq!(h.app.mail.open_uid, Some(("a@qq.com".to_string(), 42)), "点邮件行应展开摘要");
    h.hover(H::local(350.0, 48.0), 0.4);
    h.click(H::local(350.0, 48.0));
    assert_eq!(h.app.mail.open_uid, None, "点返回应回到列表");
}

#[test]
fn mail_hyperlink_click_requests_the_default_browser() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.mail.accounts=vec![crate::mail::Account {custom: None, email:"a@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
    h.app.mail.recent=vec![crate::mail::Item {received_unix: None, account:"a@qq.com".into(),from:"产品团队".into(),subject:"评审文档".into(),time:"09:41".into(),code:None,html: String::new().into(), summary:"请查看以下文档。".into(),links:vec![crate::mail::Link{label:"查看文档".into(),url:"https://example.com/review?a=1&b=2".into()}],meeting:None,uid:10}];
    h.app.mail.open_uid=Some(("a@qq.com".into(),10));h.app.show_page(5);
    h.hover(H::local(300.0,60.0),1.0);
    let link=h.shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Text(text)if text.galley.text().contains("查看文档")=>Some(text.visual_bounding_rect()),_=>None}).expect("应展示邮件的超链接");
    h.click(link.center());
    assert!(h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::OpenUrl(url)if url=="https://example.com/review?a=1&b=2")),"点击后应通过默认浏览器打开完整地址");
}

#[test]
fn low_power_notice_dismisses_without_opening_the_mail_page() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.frame(vec![]);h.app.alert("电量偏低","剩余 18% · 请连接电源",5,false);
    for _ in 0..60{h.frame(vec![]);}
    h.click_text("知道了");
    assert!(h.app.notice.is_none());
    assert_eq!(h.app.page,0,"低电量提示只需确认关闭");
}

#[test]
fn mail_details_show_the_receiving_account_when_two_accounts_share_a_uid() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.mail.accounts=vec![crate::mail::Account {custom: None, email:"first@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true},crate::mail::Account {custom: None, email:"work@icloud.com".into(),provider:crate::mail::Provider::ICloud,enabled:true}];
    h.app.mail.recent=["first@qq.com","work@icloud.com"].into_iter().map(|account|crate::mail::Item {received_unix: None, account:account.into(),from:"产品团队".into(),subject:"评审安排".into(),time:"09:41".into(),code:None,html: String::new().into(), summary:"请查看会议安排。".into(),links:vec![],meeting:None,uid:42}).collect();
    h.app.show_page(5);h.hover(H::local(300.0,60.0),1.0);
    for (row,account) in ["first@qq.com","work@icloud.com"].into_iter().enumerate(){
        h.hover(H::local(280.0,64.0+row as f32*21.5),0.6);h.click(H::local(280.0,64.0+row as f32*21.5));h.hover(H::local(300.0,90.0),0.1);
        assert_eq!(h.app.mail.open_uid,Some((account.into(),42)));
        assert!(h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text)if text.galley.text()==format!("收件邮箱：{account}"))),"详情必须明确标出这封邮件所属的收件邮箱");
        h.click(H::local(350.0,48.0));
    }
}

#[test]
fn scrolling_mail_details_keeps_the_current_message_open() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.mail.accounts=vec![crate::mail::Account {custom: None, email:"work@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
    h.app.mail.recent=vec![crate::mail::Item {received_unix: None, account:"work@qq.com".into(),from:"产品团队".into(),subject:"项目方案".into(),time:"09:41".into(),code:None,html: String::new().into(), summary:"需要滚动阅读的邮件正文。".repeat(20).into(),links:vec![],meeting:None,uid:42}];
    h.app.mail.open_uid=Some(("work@qq.com".into(),42));h.app.show_page(5);h.hover(H::local(270.0,150.0),1.0);
    let body_clip=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text().starts_with("需要滚动阅读")=>Some(s.clip_rect),_=>None}).expect("正文应显示");
    assert!(body_clip.height()>=80.0,"详情正文应有足够阅读空间，而不是仅能容纳一行");
    h.frame(vec![egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:egui::vec2(0.0,-120.0),modifiers:Default::default()}]);
    assert_eq!(h.app.page(),5,"邮件正文滚动不能切换功能页");
    assert_eq!(h.app.mail.open_uid,Some(("work@qq.com".into(),42)));
}

#[test]
fn full_mail_reader_keeps_the_entire_body_and_scrolls_independently(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let body=format!("开头：这是第一段。\n\n{}\n最后一段：全文结束", "这是一段需要阅读的完整邮件正文。\n\n".repeat(200));
    let html=body.split('\n').map(|line|format!("<p>{line}</p>")).collect::<String>();
    let item=crate::mail::Item {received_unix: None, account:"work@qq.com".into(),from:"产品团队".into(),subject:"完整方案".into(),time:"09:41".into(),code:None,html:html.into(), summary:body.clone().into(),links:vec![],meeting:None,uid:42};
    h.app.mail.accounts=vec![crate::mail::Account {custom: None, email:item.account.clone(),provider:crate::mail::Provider::Qq,enabled:true}];
    h.app.mail.recent=vec![item];h.app.mail.open_uid=Some(("work@qq.com".into(),42));h.app.show_page(5);h.hover(H::local(300.0,90.0),1.0);
    h.click(H::local(293.0,48.0));assert_eq!(h.app.mail.reading.as_ref().unwrap().summary.as_ref(),body);
    h.app.mail.recent.clear();
    let render=|h:&mut H,events|{
        h.ctx.set_embed_viewports(true);h.t+=1.0/60.0;
        let out=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(700.0,620.0))),time:Some(h.t),events,focused:true,..Default::default()},|ctx|h.app.mail_reader_ui(ctx));h.shapes=out.shapes;
    };
    // 正文按块渲染：视口外的块不生成文字形状，所以「末尾可见」只能滚动到底再断言。
    let text_top=|h:&H,needle:&str|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==needle=>Some(t.visual_bounding_rect().top()),_=>None});
    let wheel=|dy:f32|vec![egui::Event::PointerMoved(egui::pos2(350.0,300.0)),egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:egui::vec2(0.0,dy),modifiers:Default::default()}];
    render(&mut h,vec![]);render(&mut h,vec![]);
    let first=text_top(&h,"开头：这是第一段。").expect("首屏应显示正文开头");
    assert!(text_top(&h,"最后一段：全文结束").is_none(),"长邮件末尾应在首屏之外");
    for _ in 0..5{render(&mut h,wheel(-400.0));}
    let moved=text_top(&h,"开头：这是第一段。");
    assert!(moved.is_none_or(|top|top<first-100.0),"阅读窗口的滚轮应向下滚动正文");
    for _ in 0..60{render(&mut h,wheel(-400.0));}
    assert!(text_top(&h,"最后一段：全文结束").is_some(),"滚动到底应能看到邮件末尾");
    assert!(h.app.mail.reading.is_some(),"新邮件列表变化不能打断已经打开的正文");
}

#[test]
#[ignore="按需导出多格式邮件正文的实际画面"]
fn mail_reader_rich_html_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_MAIL_GALLERY_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let mut h=H::new();h.ctx.set_embed_viewports(true);
    let html=r#"<html><head><style>body{font-family:Arial}</style></head><body>
        <div style="display:none;max-height:0;overflow:hidden">&#8203;&#8203; 这是一段只为收件箱预览存在的填充文字 &#8203;</div>
        <table role="presentation" cellpadding="0" cellspacing="0" width="100%"><tr><td><img src="https://example.com/logo.png" alt="logo"> 通知中心</td><td>在浏览器中查看</td></tr></table>
        <h1>订单已确认</h1>
        <p>您好，<strong>订单 A20261107</strong> 已支付成功，我们会在 24 小时内发货。</p>
        <p><img src="https://example.com/order.png" alt="订单物流状态截图" width="600"></p>
        <h3>订单明细</h3>
        <table><tr><th>商品</th><th>数量</th><th>金额</th></tr><tr><td>机械键盘</td><td>1</td><td>¥499</td></tr><tr><td>腕托</td><td>2</td><td>¥78</td></tr></table>
        <h3>接下来会发生什么</h3>
        <ol><li>仓库拣货并出库</li><li>快递揽收后短信通知</li><li>签收后 7 天内可申请退换</li></ol>
        <ul><li>发票为电子发票，发货后发到本邮箱</li><li>如需修改地址，请在发货前回复本邮件</li></ul>
        <blockquote>注意：本邮件由系统自动发送，请勿直接回复。</blockquote>
        <hr>
        <p>感谢支持 · <a href="https://example.com/orders">查看订单详情</a></p>
    </body></html>"#;
    h.app.mail.reading=Some(crate::mail::Item{received_unix:None,account:"reader@example.com".into(),from:"通知中心".into(),
        subject:"订单 A20261107 已确认".into(),time:"11-07 09:41".into(),code:None,
        html:html.into(),
        summary:"订单已确认 您好，订单 A20261107 已支付成功，我们会在 24 小时内发货。订单明细 商品 数量 金额 机械键盘 1 ¥499 腕托 2 ¥78".into(),
        links:vec![crate::mail::Link{label:"查看订单详情".into(),url:"https://example.com/orders".into()}],meeting:None,uid:7});
    let (w,height)=(700,620);let ppp=1.5;
    let mut raster=crate::preview_raster::Raster::new((w as f32*ppp)as usize,(height as f32*ppp)as usize);
    for f in 0..=8{
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(w as f32,height as f32))),time:Some(f as f64/60.0),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
        let out=h.ctx.run(raw,|ctx|h.app.mail_reader_ui(ctx));raster.update_textures(&out.textures_delta);
        if f==8{raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(out.shapes,out.pixels_per_point),out.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join("mail-reader-rich.png"),&rgba,(w as f32*ppp)as u32,(height as f32*ppp)as u32,image::ColorType::Rgba8).unwrap();}
        raster.free_textures(&out.textures_delta);
    }
}

#[test]
fn mail_plain_text_keeps_literal_stars_in_the_reader(){
    let h=H::new();h.ctx.set_embed_viewports(true);
    let lines=crate::mail::format::lines("","保留 ** 原始字符");
    let out=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(700.0,620.0))),..Default::default()},|ctx|{
        egui::CentralPanel::default().show(ctx,|ui|crate::mail::reader::body_ui(ui,&lines,14.0));
    });
    assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="保留 ** 原始字符")));
}

#[test]
fn mail_detail_and_reader_keep_their_own_cached_body(){
    let mut h=H::new();
    let item=|uid,body:&str|crate::mail::Item{html:format!("<p>{body}</p>").into(),summary:body.to_string().into(),uid,
        account:"test@example.com".into(),from:String::new(),subject:String::new(),time:String::new(),code:None,received_unix:None,links:vec![],meeting:None};
    let first=item(1,"第一封邮件");let second=item(2,"第二封邮件");
    let before=h.app.mail_body_lines(&first);h.app.mail_body_lines(&second);
    let after=h.app.mail_body_lines(&first);
    assert!(std::sync::Arc::ptr_eq(&before,&after),"详情与独立阅读窗口不能让另一封邮件的排版缓存每帧失效");
}

#[test]
fn mail_html_headings_scale_with_the_body_font(){
    let h=H::new();let lines=crate::mail::format::lines("<h1>缩放标题</h1>","");
    for (base,expected) in [(14.0,18.0),(28.0,36.0)]{
        let out=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(700.0,620.0))),..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|crate::mail::reader::body_ui(ui,&lines,base));
        });
        let size=out.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()=="缩放标题"=>Some(t.galley.job.sections[0].format.font_id.size),_=>None}).unwrap();
        assert_eq!(size,expected,"正文缩放时标题应保持相同的字号比例");
    }
}

#[test]
fn mail_microsoft_settings_only_explains_unsupported_access(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.open_mail_settings();h.ctx.set_embed_viewports(false);
    h.app.mail.accounts.clear();h.app.mail.form.provider=crate::mail::Provider::Microsoft;
    let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
    egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
        let id=viewport.ids.this;
        let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,1300.0))),viewports:std::iter::once((id,egui::ViewportInfo::default())).collect(),..Default::default()};
        *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
    });
    for _ in 0..3{let _=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));}
    let child=output.lock().unwrap().take().unwrap();
    let texts=child.shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)=>Some(t.galley.text()),_=>None}).collect::<Vec<_>>();
    assert!(texts.iter().any(|t|t.contains("OAuth")&&t.contains("暂不支持")),"应明确说明微软邮箱的支持状态");
    for absent in ["使用 Microsoft 登录","可留空，在浏览器选择账号","输入邮箱授权码","测试连接","保存并开始监控","应用 ID"]{
        assert!(!texts.iter().any(|t|t.contains(absent)),"暂不支持时不应出现误导入口：{absent}");
    }
    assert!(h.app.mail.accounts.is_empty());
    assert!(!h.app.pending.iter().any(|cmd|matches!(cmd,crate::app::Cmd::MailTest(..)|crate::app::Cmd::OpenUrl(_))));
}

#[test]
fn mail_microsoft_custom_host_and_address_disable_password_actions(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.open_mail_settings();h.ctx.set_embed_viewports(false);h.app.mail.accounts.clear();
    let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
    let events=std::sync::Arc::new(Mutex::new(Vec::new()));let input=events.clone();
    egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
        let id=viewport.ids.this;
        let raw=egui::RawInput{viewport_id:id,focused:true,events:std::mem::take(&mut *input.lock().unwrap()),
            screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,1300.0))),
            viewports:std::iter::once((id,egui::ViewportInfo::default())).collect(),..Default::default()};
        *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
    });
    for (provider,email,host) in [(crate::mail::Provider::Custom,"student@example.edu","outlook.office365.com"),(crate::mail::Provider::Qq,"user@hotmail.com","")]{
        h.app.mail.form.provider=provider;h.app.mail.form.email=email.into();h.app.mail.form.custom.host=host.into();
        h.app.mail.form.password.clear();h.app.mail.note.clear();
        for _ in 0..3{let _=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));}
        let child=output.lock().unwrap().take().unwrap();
        assert!(child.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==crate::mail::MICROSOFT_UNSUPPORTED)));
        for label in ["测试连接","保存并开始监控"]{
            let point=child.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap();
            for pressed in [true,false]{
                *events.lock().unwrap()=vec![egui::Event::PointerMoved(point),egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}];
                let _=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));
            }
            assert!(h.app.mail.note.is_empty(),"微软云连接的按钮应停用，点击不应再提示填写授权码");
        }
        assert!(h.app.mail.accounts.is_empty());
        assert!(!h.app.pending.iter().any(|cmd|matches!(cmd,crate::app::Cmd::MailTest(..)|crate::app::Cmd::MailSync)));
    }
}

#[test]
#[ignore="按需导出微软邮箱暂不支持提示的实际画面"]
fn mail_microsoft_settings_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_MAIL_GALLERY_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for custom in [false,true]{
        let mut h=H::new();h.app.open_mail_settings();h.ctx.set_embed_viewports(false);
        h.app.mail.accounts.clear();
        h.app.mail.form.provider=if custom{crate::mail::Provider::Custom}else{crate::mail::Provider::Microsoft};
        h.app.mail.form.email="student@example.edu".into();h.app.mail.form.custom.host="outlook.office365.com".into();
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,1300.0))),
                viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.5),..Default::default()})).collect(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let mut raster=crate::preview_raster::Raster::new(1035,1950);
        for frame in 0..4{
            let root=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));raster.update_textures(&root.textures_delta);
            let child=output.lock().unwrap().take().unwrap();raster.update_textures(&child.textures_delta);
            if frame==3{
                raster.buf.fill([0.08,0.08,0.09,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(if custom{"microsoft-custom-unsupported.png"}else{"microsoft-unsupported.png"}),&rgba,1035,1950,image::ColorType::Rgba8).unwrap();
            }raster.free_textures(&child.textures_delta);
        }
    }
}

#[test]
#[ignore="按需导出提醒选择器与邮件全文的实际画面"]
fn reminder_and_reader_layout_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_READER_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for picker in [true,false]{
        let mut h=H::new();h.ctx.set_embed_viewports(true);
        h.app.todos.items=vec![crate::model::TodoItem{text:"提交产品方案".into(),due:None,done_on:None,reminded:false,..Default::default()}];h.app.open_task_reminder(0);
        let body=(1..=12).map(|n|format!("第 {n} 项 · 工作安排\n请确认需求与交付范围，整理设计方案和本周待办，并在评审前同步进度。相关意见请汇总后反馈给产品团队。\n\n")).collect::<String>();
        h.app.mail.reading=Some(crate::mail::Item {received_unix: None, account:"work@icloud.com".into(),from:"产品团队".into(),subject:"项目方案与下阶段工作安排".into(),time:"10-31 09:41".into(),code:None,html: String::new().into(), summary:format!("各位好，\n\n以下是项目安排与交付要求，请逐项确认。\n\n{body}最后一段\n请在周五前确认，感谢配合。").into(),links:vec![crate::mail::Link{label:"查看项目资料".into(),url:"https://example.com/project".into()}],meeting:None,uid:42});
        let (w,height)=if picker{(WIN_W as u32,WIN_H as u32)}else{(700,620)};let ppp=1.5;
        let mut raster=crate::preview_raster::Raster::new((w as f32*ppp)as usize,(height as f32*ppp)as usize);
        let last=if picker{89}else{7};
        for f in 0..=last{
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(w as f32,height as f32))),time:Some(f as f64/60.0),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
            let out=h.ctx.run(raw,|ctx|if picker{
                let env=Env{t:f as f64/60.0,clock:"09:41:00".into(),today:chrono::Local::now().date_naive().to_string(),date:"周日".into(),..Default::default()};
                egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=ui.max_rect();h.app.frame(ui,win,&env);});
            }else{h.app.mail_reader_ui(ctx)});raster.update_textures(&out.textures_delta);
            if f==last{raster.buf.fill(if picker{[0.07,0.09,0.13,1.0]}else{[0.0,0.0,0.0,1.0]});raster.draw(&h.ctx.tessellate(out.shapes,out.pixels_per_point),out.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(if picker{"reminder-picker.png"}else{"mail-reader.png"}),&rgba,(w as f32*ppp)as u32,(height as f32*ppp)as u32,image::ColorType::Rgba8).unwrap();}
            raster.free_textures(&out.textures_delta);
        }
    }
}

#[test]
fn mailbox_scroll_reaches_older_mail_without_changing_pages() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.mail.accounts=vec![crate::mail::Account {custom: None, email:"test@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
    h.app.mail.recent=(1..=8).map(|uid|crate::mail::Item {received_unix: None, account:"test@qq.com".into(),from:format!("来源 {uid}"),subject:format!("邮件 {uid}"),time:"11-03 09:41".into(),code:Some(format!("{uid:06}")),html: String::new().into(), summary:String::new().into(),links:vec![],meeting:None,uid}).collect();
    for item in h.app.mail.recent.clone(){h.app.mail.privacy.observe(&item,true,h.app.t,chrono::Utc::now().timestamp());}
    h.app.show_page(5);h.hover(H::local(200.0,90.0),1.0);
    h.frame(vec![egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:egui::vec2(0.0,-900.0),modifiers:Default::default()}]);
    h.hover(H::local(200.0,90.0),0.6);
    assert_eq!(h.app.page(),5,"滚动邮箱列表不能切换页面");
    let row=h.ctx.read_response(egui::Id::new(("card",true,("mail-row",8u32,"test@qq.com")))).expect("第八封邮件应可访问");
    h.click(row.rect.center());
    assert_eq!(h.app.mail.open_uid,Some(("test@qq.com".into(),8)));
}

#[test]
#[ignore="按需检查邮箱列表和验证码来源的实际画面"]
fn mailbox_readability_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("DAOZAI_MAIL_PREVIEW").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for variant in 0..5 {
        let multiple=variant==1;let detail=variant==2;
        let mut h=H::new();h.app.settings.pet=false;h.app.settings.mail_sync_operations=true;
        h.app.mail.accounts=vec![crate::mail::Account {custom: None, email:"work@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
        if multiple {h.app.mail.accounts.push(crate::mail::Account {custom: None, email:"personal@gmail.com".into(),provider:crate::mail::Provider::Gmail,enabled:true});}
        h.app.mail.recent=[("GitHub","登录验证码 · 本次登录有效期 10 分钟",Some("274630")),("Figma","产品设计评审 · 查看本轮修改意见",None),("Notion","确认你的邮箱地址",Some("815926")),("产品团队","周五项目安排与交付清单",None),("Microsoft","你的账户安全通知",None)].into_iter().enumerate().map(|(i,(from,subject,code))|crate::mail::Item {html:String::new().into(),received_unix: None, account:if multiple&&i%2==1{"personal@gmail.com"}else{"work@qq.com"}.into(),from:from.into(),subject:subject.into(),time:format!("11-03 09:{:02}",41-i),code:code.map(str::to_string),summary:String::new().into(),links:vec![],meeting:None,uid:i as u32+1}).collect();
        if detail {
            h.app.mail.recent[0].summary="你好，\n\n你的登录验证码是 274630，有效期为 10 分钟。\n\n如果这不是你本人的操作，请忽略这封邮件。请勿将验证码透露给他人。\n\n这封邮件由系统自动发送，无需回复。".into();
            h.app.mail.open_uid=Some(("work@qq.com".into(),1));
        }
        if variant==4{for item in h.app.mail.recent.clone(){h.app.mail.privacy.observe(&item,true,0.0,chrono::Utc::now().timestamp());}}
        for account in h.app.mail.accounts.clone(){
            h.app.mail.manage.identities.insert(account.email.clone(),account.clone());
            let entries=h.app.mail.recent.iter().filter(|m|m.account==account.email).map(|m|(m.uid,m.uid%2==0)).collect();
            h.app.mail.manage.data.insert((account.email,crate::mail::manage::Folder::Inbox),crate::mail::manage::Snapshot{validity:7,entries,items:vec![]});
        }
        if variant==3{
            h.app.mail.manage.trash=h.app.mail.recent.iter().take(2).cloned().map(|mut m|{m.code=None;m}).collect();
            h.app.mail.manage.trash_open=true;
            h.app.mail.manage.data.insert(("work@qq.com".into(),crate::mail::manage::Folder::Trash),crate::mail::manage::Snapshot{validity:9,entries:vec![(1,true),(2,true)],items:vec![]});
        }
        h.app.show_page(5);
        let ppp=1.5;let mut raster=crate::preview_raster::Raster::new((crate::app::CANVAS_W*ppp)as usize,(WIN_H*ppp)as usize);
        for frame in 0..100 {
            let t=frame as f64/60.0;let env=Env{t,clock:"09:41:00".into(),cursor:Some(H::local(if variant==1{350.0}else{200.0},64.0)),..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(crate::app::CANVAS_W,WIN_H))),time:Some(t),events:vec![egui::Event::PointerMoved(H::local(if variant==1{350.0}else{200.0},64.0))],viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
            let out=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,egui::vec2(WIN_W,WIN_H)),&env);});});
            raster.update_textures(&out.textures_delta);
            if frame==99 {raster.buf.fill([0.05,0.07,0.10,1.0]);raster.draw(&h.ctx.tessellate(out.shapes,out.pixels_per_point),out.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);let image=image::RgbaImage::from_raw((crate::app::CANVAS_W*ppp)as u32,(WIN_H*ppp)as u32,rgba).unwrap();image::imageops::crop_imm(&image,((WIN_W-PW)/2.0*ppp-10.0)as u32,0,(PW*ppp+20.0)as u32,if detail{370}else{225}).to_image().save(dest.join(if detail{"邮件详情.png"}else if variant==3{"回收站.png"}else if variant==4{"验证码分栏.png"}else if multiple{"悬停操作.png"}else{"邮箱列表.png"})).unwrap();}
            raster.free_textures(&out.textures_delta);
        }
    }
}

#[test]
fn mailbox_large_code_card_preserves_copy_and_identifies_source(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.mail.accounts=["work@qq.com","personal@gmail.com"].into_iter().map(|email|crate::mail::Account {custom: None, email:email.into(),provider:crate::mail::Provider::Qq,enabled:true}).collect();
    h.app.mail.recent=[("work@qq.com","GitHub","274630"),("personal@gmail.com","Notion","815926")].into_iter().map(|(account,from,code)|crate::mail::Item {received_unix: None, account:account.into(),from:from.into(),subject:"登录验证码".into(),time:"09:41".into(),code:Some(code.into()),html: String::new().into(), summary:String::new().into(),links:vec![],meeting:None,uid:42}).collect();
    for item in h.app.mail.recent.clone(){h.app.mail.privacy.observe(&item,true,h.app.t,chrono::Utc::now().timestamp());}
    h.app.show_page(5);h.hover(H::local(200.0,90.0),1.0);
    assert!(h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(t) if t.galley.text()=="GitHub")));
    h.cmds.clear();h.click(H::local(60.0,70.0));
    assert!(h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Copy(value) if value=="274630")));
    assert!(h.app.mail.open_uid.is_none(),"复制验证码不能误开邮件详情");
    assert_eq!(h.app.expanded_height(),PH+crate::app::STATUS_H,"邮箱沿用录屏里的面板尺寸");
}

#[test]
fn long_mail_lists_only_layout_the_visible_rows(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=false;
    h.app.mail.accounts=vec![crate::mail::Account {custom: None, email:"work@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
    h.app.mail.recent=(0..60).map(|i|crate::mail::Item {received_unix: None, account:"work@qq.com".into(),from:format!("发件人 {i}"),subject:format!("第 {i} 封邮件标题"),time:"09:41".into(),code:None,html: String::new().into(), summary:String::new().into(),links:vec![],meeting:None,uid:i}).collect();
    h.app.show_page(5);h.hover(H::local(300.0,60.0),1.0);
    let count=h.ctx.fonts(|fonts|fonts.num_galleys_in_cache());
    assert!(count<100,"60 封邮件不应全部排版，当前缓存了 {count} 个文字布局");
    for _ in 0..15{h.wheel(-40.0);h.hover(H::local(300.0,60.0),0.1);}
    assert_eq!(h.app.page,5,"列表虚拟化后仍应在邮箱页滚动");
}

#[test]
fn ignored_mail_event_hides_stops_reminders_persists_and_restores() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let now=chrono::Local::now();let day=now.date_naive();
    let meeting=crate::mail::Meeting{all_day:false,title:"测试邀请".into(),start:now.timestamp()+600,end:None,at:String::new(),uid:"ignore-me".into()};
    h.app.mail.recent.push(crate::mail::Item {received_unix: None, account:"test@qq.com".into(),from:"测试".into(),subject:"邀请".into(),time:String::new(),code:None,html: String::new().into(), summary:String::new().into(),links:vec![],meeting:Some(meeting.clone()),uid:99});
    h.app.toggle_mail_event(&meeting);
    assert!(h.app.calendar_entries(day).is_empty());
    assert!(crate::model::Settings::load().ignored_mail_events.contains(&crate::calendar::dedupe_key(&meeting)));
    h.frame(vec![]);assert!(!h.app.history.iter().any(|a|a.title=="会议提醒"));
    assert_eq!(h.app.mail.recent.len(),1,"忽略不删除邮件");
    h.app.calendar.events.push(meeting.clone());
    assert_eq!(h.app.calendar_entries(day)[0].sources.len(),1,"飞书来源不受邮件忽略影响");
    h.app.calendar.events.clear();h.app.toggle_mail_event(&meeting);
    assert_eq!(h.app.calendar_entries(day).len(),1);
    h.frame(vec![]);assert!(h.app.history.iter().any(|a|a.title=="会议提醒"));
}

#[test]
fn mail_report_open_settings_matrix() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.ctx.set_embed_viewports(false);
    for scale in [1.0,1.25,1.5,2.0] {
        for size in [egui::vec2(600.0,450.0),egui::vec2(690.0,560.0)] {
            egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
                let id=viewport.ids.this;
                let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,size)),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()})).collect(),..Default::default()};
                let _=ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx));
            });
            for count in [0,1,6] {
                h.app.mail.accounts=crate::mail::Provider::ALL.into_iter().take(count).enumerate().map(|(i,provider)|crate::mail::Account {custom: None, email:format!("probe{i}@example.invalid"),provider,enabled:false}).collect();
                for provider in crate::mail::Provider::ALL {
                    h.app.mail.form.provider=provider;h.app.open_mail_settings();
                    for _ in 0..3 {let _=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));}
                    assert!(h.app.settings_open);assert_eq!(h.app.settings_tab,7);
                    h.app.settings_open=false;let _=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));
                }
            }
        }
    }
}

#[test]
fn mail_report_initial_fetch_unicode_message() {
    for space in ['\u{00a0}','\u{2002}','\u{2003}','\u{202f}'] {
        for kind in ["text/plain","text/html"] {
            let raw=format!("From: probe@example.invalid\r\nSubject: verification\r\nMIME-Version: 1.0\r\nContent-Type: {kind}; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n验证码：{space}834921");
            let parsed=crate::mail::parse_message(raw.as_bytes());
            assert_eq!(crate::mail::code_of(&parsed),Some("834921".into()));
        }
    }
}

#[test]
fn mail_report_save_and_failure_paths() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.open_mail_settings();h.ctx.set_embed_viewports(false);
    fn frame(h:&mut H,events:Vec<egui::Event>){
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(900.0,1800.0))),events:events.clone(),viewports:std::iter::once((id,egui::ViewportInfo::default())).collect(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let _=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));
        h.shapes=output.lock().unwrap().take().unwrap().shapes;
    }
    fn click(h:&mut H,label:&str){
        frame(h,vec![]);frame(h,vec![]);
        let point=h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap();
        frame(h,vec![egui::Event::PointerMoved(point)]);
        for pressed in [true,false]{frame(h,vec![egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}]);}
    }
    for button in ["测试连接","保存并开始监控"] {
        h.app.mail.form.email.clear();h.app.mail.form.password.clear();click(&mut h,button);
        assert_eq!(h.app.mail.note,"请先填邮箱地址");
        h.app.mail.form.email="probe@example.invalid".into();click(&mut h,button);
        assert!(h.app.mail.note.contains("请先填授权码"));assert!(h.app.mail.accounts.is_empty());
    }
    let email=format!("daozai-crash-probe-{}@example.invalid",std::process::id());
    struct Cleanup(String);impl Drop for Cleanup{fn drop(&mut self){crate::mail::secrets::delete(&self.0);}}
    let _cleanup=Cleanup(email.clone());
    h.app.mail.form.email=email.clone();h.app.mail.form.password="synthetic-test-secret".into();
    click(&mut h,"保存并开始监控");
    assert_eq!(h.app.mail.accounts.len(),1);assert!(h.app.mail.form.password.is_empty());
    assert_eq!(crate::mail::load_accounts()[0].email,email);
    assert_eq!(crate::mail::secrets::load(&email).unwrap(),"synthetic-test-secret");
    h.app.mail_event(crate::mail::Event::Status{account:email.clone(),status:crate::mail::Status::Failed("模拟认证失败".into())});frame(&mut h,vec![]);
    h.app.mail.form.password="synthetic-test-secret".into();click(&mut h,"测试连接");
    assert_eq!(h.app.mail.testing.as_deref(),Some(email.as_str()));
    h.app.mail_event(crate::mail::Event::Test{account:email,ok:false,note:"模拟网络超时".into()});frame(&mut h,vec![]);
    assert!(h.app.mail.testing.is_none());assert!(h.app.mail.note.contains("模拟网络超时"));
}

#[test]
fn hidden_mail_masks_every_display_field_and_reveal_is_temporary(){
 let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
 let i=crate::mail::Item{received_unix:None,account:"a@example.com".into(),from:"123456".into(),subject:"验证码 123456".into(),time:"09:41".into(),code:Some("123456".into()),html:"<h1>验证码</h1><p><strong>123456</strong></p>".into(), summary:"验证码 1 2 3-4 5 6".into(),links:vec![],meeting:None,uid:42};
 h.app.mail.privacy.observe(&i,true,0.0,100);h.app.t=61.0;
 let shown=h.app.display_mail(&i);assert!(shown.code.is_none());for text in [shown.from.as_str(),shown.subject.as_str(),shown.summary.as_ref()]{assert!(!text.contains('1'));}
 assert!(shown.html.is_empty(),"隐藏验证码时 HTML 原文也必须隐藏");
 assert!(!h.app.mail_body_lines(&shown).iter().any(|line|line.text.contains("123456")));
 h.app.reveal_mail_code(&i);let revealed=h.app.display_mail(&i);assert!(revealed.code.is_some());
 assert_eq!(revealed.html,i.html);assert!(h.app.mail_body_lines(&revealed).iter().any(|line|line.text.contains("123456")));
 h.app.t+=15.0;let hidden=h.app.display_mail(&i);assert!(hidden.code.is_none()&&hidden.html.is_empty());
}

#[test]
fn mailbox_wheel_stays_in_list_with_hidden_cards_and_short_lists() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for mode in 0..4 {for count in [0,1,3,8] {
        let mut h=H::new();
        h.app.mail.accounts=vec![crate::mail::Account{custom:None,email:"test@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
        h.app.mail.recent=(1..=count).map(|uid|crate::mail::Item{received_unix:None,account:"test@qq.com".into(),from:format!("来源 {uid}"),subject:format!("邮件 {uid}"),time:"11-04 09:41".into(),code:if mode==0{None}else{Some(format!("{uid:06}"))},html: String::new().into(), summary:String::new().into(),links:vec![],meeting:None,uid}).collect();
        if mode>=2 {for item in h.app.mail.recent.clone(){h.app.mail.privacy.observe(&item,true,h.app.t,chrono::Utc::now().timestamp());}}
        h.app.settings.mail_code=mode!=2;
        let cursor=H::local(if mode==3 && count>0{220.0}else{65.0},90.0);
        h.app.show_page(5);h.hover(cursor,1.0);
        for delta in [300.0,-900.0,-300.0,900.0,300.0] {
            h.wheel(delta);h.hover(cursor,0.3);
            assert_eq!(h.app.page(),5,"模式 {mode}、{count} 封邮件：列表范围内和滚动边界不能触发翻页");
        }
    }}
}

#[test]
fn mailbox_compact_third_row_stays_visible_below_the_toolbar(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.mail.accounts=vec![crate::mail::Account {custom:None,email:"test@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
    h.app.mail.recent=(1..=6).map(|uid|crate::mail::Item {received_unix:None,account:"test@qq.com".into(),from:format!("来源 {uid}"),subject:format!("邮件 {uid}"),time:"09:41".into(),code:None,html: String::new().into(), summary:String::new().into(),links:vec![],meeting:None,uid}).collect();
    h.app.show_page(5);h.hover(H::local(200.0,90.0),1.0);
    let third=h.ctx.read_response(egui::Id::new(("card",true,("mail-row",3u32,"test@qq.com")))).expect("第三封邮件应完整可见");
    assert!(third.rect.bottom()<=H::local(380.0,122.0).y+0.5);
    h.click(third.rect.center());
    assert_eq!(h.app.mail.open_uid,Some(("test@qq.com".into(),3)));
}

#[test]
fn new_mail_notification_masks_codes_in_sender_and_history(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.mail.accounts=vec![crate::mail::Account{email:"a@example.com".into(),provider:crate::mail::Provider::Qq,custom:None,enabled:true}];
    let item=crate::mail::Item{received_unix:None,account:"a@example.com".into(),from:"登录验证码 123456".into(),subject:"验证码 123456".into(),time:"09:41".into(),code:Some("123456".into()),html: String::new().into(), summary:"验证码 123456".into(),links:vec![],meeting:None,uid:42};
    h.app.mail_event(crate::mail::Event::Mail{item,new:true});
    assert!(!h.app.history.is_empty());
    for alert in h.app.alerts.iter().chain(h.app.history.iter()) {
        assert!(!alert.body.contains("123456"),"通知和历史记录不能留下验证码明文");
    }
}

#[test]
fn mail_reading_wheel_over_headers_never_turns_pages(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.mail.accounts=vec![crate::mail::Account{custom:None,email:"work@qq.com".into(),provider:crate::mail::Provider::Qq,enabled:true}];
    h.app.mail.recent=vec![crate::mail::Item{html:String::new().into(),received_unix:None,account:"work@qq.com".into(),from:"产品团队".into(),subject:"阅读中的邮件".into(),time:"09:41".into(),code:None,summary:"正文内容。".repeat(200).into(),links:vec![],meeting:None,uid:42}];
    h.app.show_page(5);h.app.mail.open_uid=Some(("work@qq.com".into(),42));
    for (x,y) in [(210.0,18.0),(200.0,48.0),(200.0,73.0),(200.0,92.0),(270.0,214.0)]{
        for delta in [-120.0,120.0]{
            h.hover(H::local(x,y),0.5);h.wheel(delta);
            assert_eq!(h.app.page(),5,"阅读邮件时在 ({x},{y}) 滚轮不能翻页");
            assert_eq!(h.app.mail.open_uid,Some(("work@qq.com".into(),42)));
        }
    }
    h.click(H::local(350.0,48.0));assert!(h.app.mail.open_uid.is_none());
    h.hover(H::local(210.0,18.0),0.5);h.wheel(-120.0);
    assert_eq!(h.app.page(),6,"返回列表后顶栏滚轮应恢复翻页");
}

#[test]
fn reader_actions_remain_bound_to_the_opened_mailbox() {
    use crate::mail::manage::{Action,Folder,Request,Completed,Snapshot};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=managed_mail_fixture();
    let old=h.app.mail.recent[0].clone();
    h.app.mail_open_reader(&old);assert!(h.app.mail_reader_ready(&old,Folder::Inbox));
    let mut fresh=old.clone();fresh.subject="重建文件夹后的邮件".into();
    h.app.mail_event(crate::mail::Event::Status{account:old.account.clone(),status:crate::mail::Status::Ready});
    assert!(!h.app.mail_reader_ready(&old,Folder::Inbox));
    h.app.mail_managed(Completed{request:Request{account:h.app.mail.accounts[0].clone(),folder:Folder::Inbox,validity:None,known:vec![],extra:vec![],action:Action::Load},result:Ok(Snapshot{validity:8,entries:vec![(old.uid,false)],items:vec![fresh]})});
    assert!(h.app.mail.reading.is_none());assert!(!h.app.mail_reader_ready(&old,Folder::Inbox));
}

#[test]
fn local_reader_moves_and_restores_after_switching_the_main_list() {
    use crate::mail::manage::{Action,Folder};
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=managed_mail_fixture();h.app.settings.mail_sync_operations=false;
    let item=h.app.mail.recent[0].clone();
    for folder in [Folder::Inbox,Folder::Trash]{
        h.app.mail_open_reader(&item);h.app.mail_toggle_trash();
        assert!(h.app.mail_reader_ready(&item,folder));
        h.app.mail_manage_request(Some(&item.account),folder,Action::Move{uid:item.uid});
        assert!(h.app.mail_rows().iter().any(|m|m.uid==item.uid));
    }
    h.app.mail_manage_request(Some(&item.account),Folder::Trash,Action::Move{uid:item.uid});
    assert!(h.app.mail.manage.message.contains("刷新"));
}

#[test]
fn new_mail_preserves_an_explicit_reveal_in_the_trash(){
    let _guard=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=managed_mail_fixture();
    let mut item=h.app.mail.recent[0].clone();item.uid=77;item.code=Some("123456".into());
    h.app.mail.manage.trash.push(item.clone());h.app.reveal_mail_code(&item);
    let incoming=h.app.mail.recent[1].clone();h.app.mail_event(crate::mail::Event::Mail{item:incoming,new:true});
    assert!(!h.app.mail_code_hidden(&item));
    h.app.t+=16.0;assert!(h.app.mail_code_hidden(&item));
}

#[test]
fn managed_mail_unread_filter_open_and_mark_read(){
    use crate::mail::manage::{Action,Folder,Completed};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=managed_mail_fixture();
    h.click(H::local(118.0,42.0));assert!(h.app.mail.manage.unread_only);
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="测试邮件 2")));
    h.cmds.clear();h.click(H::local(180.0,64.0));h.frame(vec![]);
    assert_eq!(h.app.mail.open_uid,Some(("manage@example.com".into(),1)));
    let request=h.cmds.iter().find_map(|c|match c{crate::app::Cmd::MailManage(r)if r.action==(Action::Read{uid:1,seen:true})=>Some(r.clone()),_=>None}).expect("打开未读邮件应提交服务器已读操作");
    let mut snapshot=h.app.mail.manage.data[&("manage@example.com".into(),Folder::Inbox)].clone();snapshot.entries[0].1=true;
    h.app.mail_managed(Completed{request,result:Ok(snapshot)});h.hover(H::local(270.0,150.0),0.5);
    h.click(H::local(350.0,48.0));assert!(h.app.mail.open_uid.is_none());h.hover(H::local(200.0,80.0),0.4);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="没有未读邮件")));
}

#[test]
fn managed_mail_trash_icon_and_restore_keep_inbox_uid_separate(){
    use crate::mail::manage::{Action,Folder,Snapshot,Completed};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=managed_mail_fixture();
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text().starts_with("回收站"))),"回收站入口只显示图标");
    let item=h.app.mail.recent[0].clone();h.app.mail.manage.trash=vec![item];
    h.app.mail.manage.data.insert(("manage@example.com".into(),Folder::Trash),Snapshot{validity:9,entries:vec![(1,true)],items:vec![]});
    h.click(H::local(370.0,130.0));assert!(h.app.mail.manage.trash_open);
    h.hover(H::local(350.0,64.0),0.2);h.cmds.clear();h.click(H::local(350.0,64.0));
    let request=h.cmds.iter().find_map(|c|match c{crate::app::Cmd::MailManage(r)if r.action==(Action::Move{uid:1})=>Some(r.clone()),_=>None}).expect("回收站恢复按钮应提交移动操作");
    assert_eq!(request.folder,Folder::Trash);assert_eq!(request.validity,Some(9));
    h.app.mail_managed(Completed{request,result:Ok(Snapshot{validity:9,entries:vec![],items:vec![]})});
    assert!(h.app.mail.manage.trash.is_empty());assert_eq!(h.app.mail.recent.len(),3,"同 UID 的收件箱邮件不能被回收站恢复操作移除");
    assert!(h.app.mail.manage.data.contains_key(&("manage@example.com".into(),Folder::Inbox)),"恢复时应保留收件箱正文缓存的有效性，切回后只补读新增邮件");
    h.frame(vec![]);h.click(H::local(350.0,43.0));assert!(!h.app.mail.manage.trash_open);
}

#[test]
fn managed_mail_actions_wait_for_the_server_and_batch_read_is_explicit(){
    use crate::mail::manage::{Action,Completed};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=managed_mail_fixture();h.cmds.clear();
    h.hover(H::local(354.0,64.0),0.2);h.click(H::local(354.0,64.0));
    let request=h.cmds.iter().find_map(|c|match c{crate::app::Cmd::MailManage(r)if r.action==(Action::Move{uid:1})=>Some(r.clone()),_=>None}).expect("删除应提交移到回收站");
    assert_eq!(h.app.mail.recent.len(),3,"服务器确认前不能丢掉邮件");
    h.app.mail_managed(Completed{request,result:Err("服务器拒绝操作".into())});assert_eq!(h.app.mail.recent.len(),3);
    for snapshot in h.app.mail.manage.data.values_mut(){for (_,seen) in &mut snapshot.entries{*seen=true;}}
    h.frame(vec![]);h.cmds.clear();h.click(H::local(350.0,43.0));
    assert!(h.cmds.iter().any(|c|matches!(c,crate::app::Cmd::MailManage(r)if r.action==Action::ReadAll)),"即使最近邮件已读，也应能处理邮箱中更早的未读邮件");
}

#[test]
fn mail_local_mode_persists_without_writing_to_the_server(){
    use crate::mail::manage::{Action,Folder,Request,Completed};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=managed_mail_fixture();h.app.settings.mail_sync_operations=false;h.cmds.clear();
    let original=h.app.mail.recent.clone();let account=h.app.mail.accounts[0].clone();
    let snapshot=h.app.mail.manage.data[&("manage@example.com".into(),Folder::Inbox)].clone();
    h.app.mail_manage_request(Some("manage@example.com"),Folder::Inbox,Action::Read{uid:2,seen:false});
    assert_eq!(h.app.mail_seen(&original[1],Folder::Inbox),Some(false));
    assert!(h.app.mail.manage.data[&("manage@example.com".into(),Folder::Inbox)].entries[1].1,"服务器状态不能被本机已读操作改动");
    h.app.mail_manage_request(Some("manage@example.com"),Folder::Inbox,Action::Move{uid:1});
    assert_eq!(h.app.mail.recent.len(),2);assert_eq!(h.app.mail.manage.trash.len(),1);
    h.frame(vec![]);assert!(!h.cmds.iter().any(|c|matches!(c,crate::app::Cmd::MailManage(r)if r.action!=Action::Load)));
    let path=crate::model::config_dir().join("mail-local-state.json");let saved=std::fs::read_to_string(&path).unwrap();
    assert!(!saved.contains("manage@example.com")&&!saved.contains("测试邮件"),"本机状态仅保存标识和操作标记");
    h.app.mail.manage=Default::default();h.app.mail.recent=original.clone();h.app.mail_reload_local();
    h.app.mail_managed(Completed{request:Request{account,folder:Folder::Inbox,validity:Some(7),known:vec![],extra:vec![],action:Action::Load},result:Ok(snapshot)});
    assert_eq!(h.app.mail.recent.len(),2,"重新读取服务器后，本机移除的邮件不能重新出现");assert_eq!(h.app.mail.manage.trash.len(),1);
    assert_eq!(h.app.mail_seen(&original[1],Folder::Inbox),Some(false),"本机已读状态必须跨重启保留");
    h.app.mail.manage.trash_open=true;h.app.mail_manage_request(Some("manage@example.com"),Folder::Trash,Action::Move{uid:1});
    assert!(h.app.mail.manage.trash.is_empty());assert_eq!(h.app.mail.recent.len(),3);
    h.cmds.clear();h.app.settings.mail_sync_operations=true;h.app.mail_manage_mode_changed();h.frame(vec![]);
    assert!(!h.cmds.iter().any(|c|matches!(c,crate::app::Cmd::MailManage(r)if r.action!=Action::Load)),"开启同步不能补交本机操作");
    assert_eq!(h.app.mail_seen(&original[1],Folder::Inbox),Some(true),"开启同步后应显示服务器真实状态");
}

#[test]
fn mail_sync_switch_defaults_off_and_persists(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    assert!(!crate::model::Settings::default().mail_sync_operations);
    h.app.settings.mail_sync_operations=true;h.app.settings.save();assert!(crate::model::Settings::load().mail_sync_operations);
    h.app.settings.mail_sync_operations=false;h.app.settings.save();assert!(!crate::model::Settings::load().mail_sync_operations);
}
