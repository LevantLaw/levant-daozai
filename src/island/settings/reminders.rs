//! 提醒设置页：系统状态、即时通讯、AI 会话与 Windows 通知。
use super::*;

impl App {
    pub(crate) fn reminders_settings(&mut self, ui:&mut egui::Ui, ctx:&egui::Context) {
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
                for client in crate::instant_messaging::Messenger::ALL {
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
            let current=crate::windows_toasts::system_notifications_enabled();
            let mut system_on=current;
            let listener_on=current && self.settings.read_notifications && listening(&self.listener_status);
            setting_row_chip(ui,"系统通知",if listener_on{"监听中"}else{"未监听"},listener_on,&status_tip(&self.listener_status),&mut system_on);
            if system_on!=current {
                match crate::windows_toasts::set_system_notifications(system_on) {
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
                    || crate::instant_messaging::Messenger::from_notification_id(id).is_some()
                    || crate::instant_messaging::taskbar_client(name,id).is_some()
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
}
