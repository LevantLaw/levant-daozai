//! Windows 通知记录：应用、标识、正文与时间。

/// Windows 通知页的一条记录；QQ 有独立未读图标，不进入这里。
#[derive(Clone, Debug)]
pub struct WindowsNotice {
    pub app: String,
    /// 应用标识（AUMID 或名称），用来取应用图标。
    pub id: String,
    pub body: String,
    pub time: String,
}
