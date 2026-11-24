//! Windows 通知读取：权限、变更检测与发送方核对。

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{self, Receiver};
use windows::core::{Result, HSTRING};
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus,
};
use windows::UI::Notifications::{NotificationKinds, UserNotification};

use super::toast_source;

pub enum Incoming {
    Revoked,
    Status(String),
    Message(String, String, String, Option<crate::instant_messaging::Target>),
}

pub(crate) fn notification_identity(n:&UserNotification,number:u32,created:i64,known:Option<crate::instant_messaging::Messenger>)->(String,String){
    use crate::instant_messaging::Messenger;
    let info=n.AppInfo().ok();
    let id=info.as_ref().and_then(|app|app.AppUserModelId().ok()).map(|id|id.to_string()).unwrap_or_default();
    let client=if !id.is_empty(){Messenger::from_notification_id(&id)}
        else{known.or_else(||toast_source::sender(number,created).and_then(|id|Messenger::from_notification_id(&id)))};
    if let Some(client)=client{return (client.name().into(),client.adapter().notification_id.into());}
    let name=info.and_then(|app|app.DisplayInfo().ok()).and_then(|display|display.DisplayName().ok())
        .map(|name|name.to_string()).filter(|name|!name.trim().is_empty()).unwrap_or_else(||"Windows 通知".into());
    let id=if id.is_empty(){name.clone()}else{id};
    (name,id)
}
fn notification_body(n: &UserNotification) -> Result<String> {
    let binding = n
        .Notification()?
        .Visual()?
        .GetBinding(&HSTRING::from("ToastGeneric"))?;
    let texts = binding.GetTextElements()?;
    let mut parts = vec![];
    for i in 0..texts.Size()?.min(8) {
        parts.push(texts.GetAt(i)?.Text()?.to_string().chars().take(300).collect::<String>());
    }
    Ok(parts.join(" · "))
}
/// 停止信号独立于消息队列，断开后结束等待，不再读取通知。
pub struct Notifications {
    incoming: Receiver<Incoming>,
    stop: mpsc::Sender<()>,
    taskbar_mask:std::sync::Arc<std::sync::atomic::AtomicU8>,
}
impl Notifications {
    pub fn try_recv(&self)->std::result::Result<Incoming,mpsc::TryRecvError>{self.incoming.try_recv()}
    pub fn taskbar_settings(&self,settings:&crate::model::Settings){self.taskbar_mask.store(crate::instant_messaging::toast_suppression(settings),std::sync::atomic::Ordering::Relaxed);}
}
impl Drop for Notifications {fn drop(&mut self){let _=self.stop.send(());}}

#[derive(Clone,Copy,PartialEq,Eq)]
enum ToastBody {Skipped,Unavailable,Read(u64)}
#[derive(Clone,Copy,PartialEq,Eq)]
struct ToastVersion { created:i64, client:Option<crate::instant_messaging::Messenger>, body:ToastBody }
impl ToastVersion {
    fn new(created:i64,client:Option<crate::instant_messaging::Messenger>,body:Option<&str>)->Self{
        Self{created,client,body:body.map(|body|{let mut hash=std::collections::hash_map::DefaultHasher::new();body.hash(&mut hash);ToastBody::Read(hash.finish())}).unwrap_or(ToastBody::Skipped)}
    }
}
struct NotificationBaseline { previous:Option<HashMap<u32,ToastVersion>>,started:i64,primed:bool }
impl Default for NotificationBaseline {
    fn default()->Self{
        let started=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()
            .and_then(|time|i64::try_from(time.as_nanos()/100+116444736000000000).ok()).unwrap_or(i64::MAX);
        Self{previous:None,started,primed:false}
    }
}
impl NotificationBaseline {
    fn unchanged_other(&self,id:u32,created:i64)->Option<ToastVersion>{
        self.previous.as_ref()?.get(&id).copied().filter(|old|old.created==created && old.client.is_none() && old.body==ToastBody::Skipped)
    }
    fn changed(&self,id:u32,version:ToastVersion)->bool{
        let Some(previous)=&self.previous else{return version.created>self.started;};
        let Some(old)=previous.get(&id) else{return self.primed || version.created>self.started;};
        if old.created!=version.created{return true;}
        // 未读取、读取失败后的首次成功只建立指纹，避免通道切换和恢复读取时重放旧通知。
        matches!((old.body,version.body),(ToastBody::Read(before),ToastBody::Read(after)) if before!=after)
    }
    fn finish(&mut self,mut current:HashMap<u32,ToastVersion>,complete:bool){
        if !complete{if let Some(previous)=&self.previous{for (&id,&old) in previous{current.entry(id).or_insert(old);}}}
        self.previous=Some(current);self.primed|=complete;
    }
    fn observe(&self,id:u32,created:i64,client:Option<crate::instant_messaging::Messenger>,tracked:bool,body:Result<String>)->(ToastVersion,Option<String>){
        let body=match body{
            Ok(body)=>body,
            Err(_)=>{
                if let Some(old)=self.previous.as_ref().and_then(|items|items.get(&id)).filter(|old|old.created==created && old.client==client){return (*old,None);}
                let version=ToastVersion{created,client,body:ToastBody::Unavailable};
                // 已确认是新通知时仍给通用提醒；恢复正文后只补指纹，不重复弹出。
                let message=self.changed(id,version).then(||if tracked{"有新消息".into()}else{"收到新通知".into()});
                return (version,message);
            }
        };
        let version=ToastVersion::new(created,client,tracked.then_some(body.as_str()));
        let message=self.changed(id,version).then(||if tracked && body.trim().is_empty(){"有新消息".into()}else{body});
        (version,message)
    }
}

fn channel_status(client:crate::instant_messaging::Messenger)->String{
    use windows::UI::Notifications::{NotificationSetting,ToastNotificationManager};
    let name=client.name();
    match ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(client.adapter().notification_id)).and_then(|notifier|notifier.Setting()){
        Ok(NotificationSetting::Enabled)=>format!("{name} 系统通知通道可用"),
        Ok(NotificationSetting::DisabledForApplication)=>format!("{name} 系统通知已关闭，请在 Windows 通知设置中开启后重新连接"),
        Ok(NotificationSetting::DisabledForUser)=>"Windows 系统通知已关闭，请开启后重新连接".into(),
        Ok(NotificationSetting::DisabledByGroupPolicy)=>"系统策略禁止通知".into(),
        _=>format!("{name} 系统通知通道未就绪，请检查安装和通知设置后重新连接"),
    }
}

/// Windows 通知设置中已登记且开启、能解析出名称的应用。
#[cfg(windows)]
pub fn notification_apps_enabled()->Vec<(String,String)>{
    use windows::core::{w,GUID,Interface};
    use windows::Win32::System::Com::{CoInitializeEx,CoTaskMemFree,COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{IEnumShellItems,IShellItem2,SHCreateItemFromParsingName,BHID_EnumItems,SIGDN_NORMALDISPLAY};
    use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;
    const PKEY_APP_USER_MODEL_ID:PROPERTYKEY=PROPERTYKEY{fmtid:GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),pid:5};
    let mut apps=Vec::new();
    unsafe {
        let _=CoInitializeEx(None,COINIT_APARTMENTTHREADED);
        let Ok(folder)=SHCreateItemFromParsingName::<_,_,IShellItem2>(w!("shell:AppsFolder"),None) else{return apps};
        let Ok(items)=folder.BindToHandler::<_,IEnumShellItems>(None,&BHID_EnumItems) else{return apps};
        let mut batch=[const { None };32];
        loop {
            if items.Next(&mut batch,None).is_err(){break;}
            let mut any=false;
            for item in batch.iter_mut().filter_map(Option::take) {
                any=true;
                let Ok(item)=item.cast::<IShellItem2>() else{continue};
                let Ok(raw_id)=item.GetString(&PKEY_APP_USER_MODEL_ID) else{continue};
                let id=raw_id.to_string().unwrap_or_default();
                CoTaskMemFree(Some(raw_id.0 as _));
                if id.is_empty()||notification_enabled(&id)!=Some(true){continue;}
                let Ok(raw_name)=item.GetDisplayName(SIGDN_NORMALDISPLAY) else{continue};
                let name=raw_name.to_string().unwrap_or_default().trim().to_string();
                CoTaskMemFree(Some(raw_name.0 as _));
                if name.is_empty()||crate::instant_messaging::Messenger::from_notification_id(&id).is_some(){continue;}
                if crate::instant_messaging::taskbar_client(&name,&id).is_some(){continue;}
                apps.push((id,name));
            }
            if !any{break;}
        }
    }
    apps.sort_by(|a,b|a.1.cmp(&b.1));
    apps.dedup_by(|a,b|a.1==b.1);
    apps
}

/// 应用在 Windows 通知设置里的开关：`None` 表示没有登记，不会出现在通知设置列表里。
#[cfg(windows)]
pub(crate) fn notification_enabled(id:&str)->Option<bool>{
    use windows::core::{w,HSTRING};
    use windows::Win32::System::Registry::{RegCloseKey,RegOpenKeyExW,RegQueryValueExW,HKEY,HKEY_CURRENT_USER,KEY_QUERY_VALUE};
    let path=HSTRING::from(format!("Software\\Microsoft\\Windows\\CurrentVersion\\Notifications\\Settings\\{id}"));
    unsafe {
        let mut key=HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER,&path,0,KEY_QUERY_VALUE,&mut key).0!=0{return None;}
        let mut value=1u32;let mut size=4u32;
        let rc=RegQueryValueExW(key,w!("Enabled"),None,None,Some(&mut value as *mut u32 as *mut u8),Some(&mut size));
        let _=RegCloseKey(key);
        Some(rc.0!=0||value!=0)
    }
}

const NOTIFICATIONS_KEY:&str="Software\\Microsoft\\Windows\\CurrentVersion\\PushNotifications";

/// Windows 全局通知开关；注册表没有这个值时系统按开启处理。
pub(crate) fn system_notifications_enabled()->bool{
    use windows::core::{w,HSTRING};
    use windows::Win32::System::Registry::{RegCloseKey,RegOpenKeyExW,RegQueryValueExW,HKEY,HKEY_CURRENT_USER,KEY_QUERY_VALUE};
    let path=HSTRING::from(NOTIFICATIONS_KEY);
    unsafe {
        let mut key=HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER,&path,0,KEY_QUERY_VALUE,&mut key).0!=0{return true;}
        let mut value=1u32;let mut size=4u32;
        let rc=RegQueryValueExW(key,w!("ToastEnabled"),None,None,Some(&mut value as *mut u32 as *mut u8),Some(&mut size));
        let _=RegCloseKey(key);
        rc.0!=0||value!=0
    }
}

/// 写入 Windows 全局通知开关（设置 → 系统 → 通知）。
pub(crate) fn set_system_notifications(on:bool)->std::result::Result<(),String>{
    use windows::core::{w,HSTRING};
    use windows::Win32::System::Registry::{RegCloseKey,RegCreateKeyExW,RegSetValueExW,HKEY,HKEY_CURRENT_USER,KEY_SET_VALUE,REG_DWORD,REG_OPTION_NON_VOLATILE};
    let path=HSTRING::from(NOTIFICATIONS_KEY);
    let value=u32::from(on);
    unsafe {
        let mut key=HKEY::default();
        let rc=RegCreateKeyExW(HKEY_CURRENT_USER,&path,0,None,REG_OPTION_NON_VOLATILE,KEY_SET_VALUE,None,&mut key,None);
        if rc.0!=0{return Err(format!("无法打开通知设置：{}",rc.0));}
        let rc=RegSetValueExW(key,w!("ToastEnabled"),0,REG_DWORD,Some(&value.to_ne_bytes()));
        let _=RegCloseKey(key);
        if rc.0!=0{return Err(format!("无法写入通知开关：{}",rc.0));}
        Ok(())
    }
}

/// 必须由窗口线程发起权限请求，后台仅读取新增或更新的通知。
pub fn connect(ctx:eframe::egui::Context,filter:crate::instant_messaging::NotificationFilter) -> Result<Notifications> {
    connect_inner(ctx,filter,crate::instant_messaging::toast_suppression(&crate::model::Settings::default()))
}
pub fn connect_with_settings(ctx:eframe::egui::Context,settings:&crate::model::Settings)->Result<Notifications>{
    connect_inner(ctx,crate::instant_messaging::NotificationFilter::All,crate::instant_messaging::toast_suppression(settings))
}
fn connect_inner(ctx:eframe::egui::Context,filter:crate::instant_messaging::NotificationFilter,mask:u8)->Result<Notifications>{
    let baseline=NotificationBaseline::default();
    let listener = UserNotificationListener::Current()?;
    let request = listener.RequestAccessAsync()?;
    let (tx, rx) = mpsc::sync_channel(32);
    let (stop, stopped)=mpsc::channel();
    let taskbar_mask=std::sync::Arc::new(std::sync::atomic::AtomicU8::new(mask));let worker_mask=taskbar_mask.clone();
    std::thread::Builder::new().name("daozai-notifications".into()).spawn(move || {
        let send=|event|{
            let result=tx.send(event);
            if result.is_ok(){ctx.request_repaint();}
            result.is_ok()
        };
        let run = || -> Result<()> {
            unsafe {windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED)}?;
            struct Apartment;
            impl Drop for Apartment {fn drop(&mut self){unsafe{windows::Win32::System::WinRT::RoUninitialize();}}}
            let _apartment=Apartment;
            if request.get()? != UserNotificationListenerAccessStatus::Allowed {
                send(Incoming::Revoked);
                send(Incoming::Status("Windows 未授予通知读取权限，可在系统设置中调整。".into()));
                return Ok(());
            }
            let listener = UserNotificationListener::Current()?;
            let channels=crate::instant_messaging::Messenger::ALL.into_iter().filter(|client|client.adapter().capture==crate::instant_messaging::Capture::SystemToast)
                .filter(|client|filter.allows(client.adapter().notification_id)).map(channel_status).collect::<Vec<_>>().join("\n");
            let mut baseline=baseline;
            let mut accounts=crate::instant_messaging::ToastAccounts::default();accounts.sample();
            let mut last_status=String::new();
            loop {
                if stopped.try_recv()!=Err(mpsc::TryRecvError::Empty){return Ok(());}
                let (items,count)=match listener.GetAccessStatus().and_then(|access|{
                    if access!=UserNotificationListenerAccessStatus::Allowed{return Ok(None);}
                    let items=listener.GetNotificationsAsync(NotificationKinds::Toast)?.get()?;let count=items.Size()?;
                    Ok(Some((items,count)))
                }){
                    Ok(Some(items))=>items,
                    Ok(None)=>{send(Incoming::Revoked);send(Incoming::Status("通知权限已被关闭".into()));return Ok(());}
                    Err(error)=>{
                        let status=format!("Windows 通知暂时读取失败：{} · 将自动重试",error.code());
                        if status!=last_status{send(Incoming::Status(status.clone()));last_status=status;}
                        if stopped.recv_timeout(std::time::Duration::from_secs(2))!=Err(mpsc::RecvTimeoutError::Timeout){return Ok(());}
                        continue;
                    }
                };
                accounts.sample();
                let mut current=HashMap::new();
                let mut complete=true;
                let mut messages=Vec::new();
                for i in 0..count {
                    let Ok(n)=items.GetAt(i) else{complete=false;continue};
                    let (Ok(number),Ok(created))=(n.Id(),n.CreationTime()) else{complete=false;continue};
                    let created=created.UniversalTime;
                    if let Some(old)=baseline.unchanged_other(number,created){current.insert(number,old);continue;}
                    let known=baseline.previous.as_ref().and_then(|old|old.get(&number)).filter(|old|old.created==created).and_then(|old|old.client);
                    let (app,id)=notification_identity(&n,number,created,known);
                    // 微信只走任务栏事件，即使客户端同时发送系统通知也不读取其正文。
                    let client=crate::instant_messaging::Messenger::from_notification_id(&id);
                    if !filter.allows(&id) || crate::instant_messaging::suppresses_toast(&app,&id,worker_mask.load(std::sync::atomic::Ordering::Relaxed)){current.insert(number,ToastVersion::new(created,client,None));continue;}
                    let tracked=client.is_some();
                    // 已适配客户端同时检查正文变化；其他应用的启动历史不需要读取正文。
                    let body=if tracked || baseline.previous.is_some() || created>baseline.started{notification_body(&n)}else{Ok(String::new())};
                    let (version,body)=baseline.observe(number,created,client,tracked,body);
                    current.insert(number,version);
                    if let Some(body)=body{
                        let target=client.filter(|client|client.adapter().capture==crate::instant_messaging::Capture::SystemToast).map(|client|(client,accounts.target(client,number,created)))
                            .and_then(|(client,target)|{
                                if target.is_none(){crate::instant_messaging::client::trace(format!("通知 {id}·创建 {created} 未能绑定 {} 账号",client.name()));}
                                target
                            });
                        if !body.trim().is_empty(){messages.push((created,app,id,body,target));}
                    }
                }
                baseline.finish(current,complete);
                accounts.retain(|client,id,created|baseline.previous.as_ref().and_then(|items|items.get(&id)).is_some_and(|item|item.created==created && item.client==Some(client)));
                messages.sort_by_key(|item|item.0);
                for (_,app,id,body,target) in messages {
                    if !send(Incoming::Message(app,id,body,target)){return Ok(());}
                }
                let status=match filter{crate::instant_messaging::NotificationFilter::Only(client)=>format!("仅监听 {} · {channels}",client.name()),crate::instant_messaging::NotificationFilter::All=>format!("正在监听 Windows 通知 · 当前可见 {count} 条\n{channels}")};
                if status!=last_status {
                    if !send(Incoming::Status(status.clone())){return Ok(());}
                    last_status=status;
                }
                if stopped.recv_timeout(std::time::Duration::from_secs(1))!=Err(mpsc::RecvTimeoutError::Timeout){return Ok(());}
            }
        };
        if let Err(e) = run() {
            send(Incoming::Status(format!("Windows 暂不允许读取通知：{}。请检查通知访问权限与应用身份。",e.code())));
        }
    }).map_err(|_|windows::core::Error::from_win32())?;
    Ok(Notifications{incoming:rx,stop,taskbar_mask})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notifications_skip_history_but_detect_reused_ids_and_updated_content(){
        let mut baseline=NotificationBaseline{primed:true,..Default::default()};
        let first=ToastVersion::new(10,Some(crate::instant_messaging::Messenger::Qq),Some("第一条"));
        assert!(!baseline.changed(1,first));
        baseline.previous=Some(HashMap::from([(1,first),(2,ToastVersion::new(10,None,None))]));
        assert!(!baseline.changed(1,first));
        assert!(baseline.changed(1,ToastVersion::new(10,Some(crate::instant_messaging::Messenger::Qq),Some("第二条"))));
        assert!(baseline.changed(1,ToastVersion::new(11,Some(crate::instant_messaging::Messenger::Qq),Some("第一条"))));
        assert!(baseline.unchanged_other(2,10).is_some());
        assert!(baseline.unchanged_other(1,10).is_none());
        assert!(baseline.unchanged_other(2,11).is_none());
        baseline.previous=Some(HashMap::new());assert!(baseline.changed(1,first));
    }
    #[test]
    fn notifications_arriving_during_connection_are_not_treated_as_old_history(){
        let baseline=NotificationBaseline{previous:None,started:100,primed:false};
        let (_,message)=baseline.observe(1,101,Some(crate::instant_messaging::Messenger::Qq),true,Ok("连接期间的新消息".into()));
        assert!(message.is_some(),"连接期间才到达的新通知不能混入启动历史而漏报");
    }
    #[test]
    fn notifications_read_failures_do_not_replay_an_unchanged_qq_message(){
        let client=Some(crate::instant_messaging::Messenger::Qq);
        let original=ToastVersion::new(10,client,Some("同一条消息"));
        let baseline=NotificationBaseline{previous:Some(HashMap::from([(1,original)])),..Default::default()};
        let (version,message)=baseline.observe(1,10,client,true,Err(windows::core::Error::from_win32()));
        assert!(message.is_none(),"临时读取失败不能变成空正文更新而重复提醒");
        assert!(version==original,"读取失败时保留上一轮指纹");
    }
    #[test]
    fn notifications_unreadable_new_toasts_remind_once_and_historical_toasts_stay_silent(){
        let client=Some(crate::instant_messaging::Messenger::Qq);
        for (created,expected) in [(90,false),(110,true)]{
            let mut baseline=NotificationBaseline{previous:None,started:100,primed:false};
            let (version,message)=baseline.observe(1,created,client,true,Err(windows::core::Error::from_win32()));
            assert_eq!(message.is_some(),expected,"历史与连接期间的新通知必须区分");
            baseline.previous=Some(HashMap::from([(1,version)]));
            let (version,message)=baseline.observe(1,created,client,true,Ok("恢复后的正文".into()));
            assert!(message.is_none(),"正文恢复不能重弹同一条通知");
            baseline.previous=Some(HashMap::from([(1,version)]));
            assert!(baseline.observe(1,created,client,true,Ok("下一条消息".into())).1.is_some());
        }
    }
    #[test]
    fn notifications_office_channel_switch_does_not_replay_but_later_body_updates_remind(){
        for client in crate::instant_messaging::Messenger::OFFICE{
            let mut baseline=NotificationBaseline{previous:Some(HashMap::from([(1,ToastVersion::new(10,Some(client),None))])),started:0,primed:true};
            assert!(baseline.unchanged_other(1,10).is_none(),"办公软件的捕获通道会切换，不能永久跳过");
            let (version,message)=baseline.observe(1,10,Some(client),true,Ok("切换前的旧消息".into()));
            assert!(message.is_none());baseline.previous=Some(HashMap::from([(1,version)]));
            assert!(baseline.observe(1,10,Some(client),true,Ok("切换后的新消息".into())).1.is_some());
        }
    }
    #[test]
    fn notifications_partial_snapshots_do_not_replay_old_items_or_drop_new_items(){
        let client=Some(crate::instant_messaging::Messenger::Qq);
        let old=ToastVersion::new(90,client,Some("启动前的旧通知"));let fresh=ToastVersion::new(110,client,Some("连接后的新通知"));
        let mut baseline=NotificationBaseline{previous:None,started:100,primed:false};
        assert!(baseline.changed(2,fresh));baseline.finish(HashMap::from([(2,fresh)]),false);
        assert!(!baseline.changed(1,old),"首次只读到部分条目时，补读到的启动历史不能重放");
        assert!(!baseline.changed(2,fresh));baseline.finish(HashMap::from([(1,old),(2,fresh)]),true);
        baseline.finish(HashMap::from([(2,fresh)]),false);
        assert!(!baseline.changed(1,old),"临时漏读条目不能被当成移除后重新新增");
        let next=ToastVersion::new(120,client,Some("下一条"));assert!(baseline.changed(3,next));
        baseline.finish(HashMap::new(),true);
        assert!(baseline.previous.as_ref().unwrap().is_empty(),"成功读完整空列表后应清理历史，不能永久保留");
    }
    #[test]
    fn notifications_disconnect_signals_stop_without_waiting_for_new_messages(){
        let (_tx,incoming)=mpsc::sync_channel(32);let (stop,stopped)=mpsc::channel();
        drop(Notifications{incoming,stop,taskbar_mask:std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0))});
        assert_eq!(stopped.try_recv(),Ok(()));
    }
}
