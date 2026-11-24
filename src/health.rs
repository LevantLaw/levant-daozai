//! 健康：饮水进度、喝水与久坐提醒、岛内页面与设置。

pub(crate) mod page;
pub(crate) mod reminders;
pub(crate) mod settings;
pub(crate) mod water;

pub(crate) use reminders::HealthReminder;
pub(crate) use water::Water;
