//! 饮水进度：每日目标、每次记录、分时饮水量与持久化。

use crate::model::{config_dir, read_kv, write_kv};

#[derive(Clone, Debug)]
pub(crate) struct Water {
    pub bottle_ml: u32,
    pub goal_ml: u32,
    pub start_day: String,
    pub drunk_ml: u32,
    pub refills: u32,
    /// 每小时饮水量（ml），给饮水页的柱状图用。
    pub hours: [u32; 24],
    /// 定时提醒开关。
    pub remind: bool,
}

impl Default for Water {
    fn default() -> Self {
        Self { bottle_ml: 2000, goal_ml: 2000, start_day: String::new(), drunk_ml: 0, refills: 0, hours: [0; 24], remind: true }
    }
}

impl Water {
    /// `water.txt` 只写原工具已知的三个键；复刻版新增的进度键写到 `water-extra.txt`，
    /// 避免给原工具的文件引入未知键。
    pub fn load(today: &str) -> Self {
        let mut w = Self::default();
        for (k, v) in read_kv(&config_dir().join("water.txt")) {
            match k.as_str() {
                "bottle" => w.bottle_ml = v.parse::<f32>().map(|x| x as u32).unwrap_or(2000),
                "goal" => w.goal_ml = v.parse().unwrap_or(2000),
                "start" => w.start_day = v,
                _ => {}
            }
        }
        for (k, v) in read_kv(&config_dir().join("water-extra.txt")) {
            match k.as_str() {
                "drunk" => w.drunk_ml = v.parse().unwrap_or(0),
                "refills" => w.refills = v.parse().unwrap_or(0),
                "remind" => w.remind = v != "0",
                "hours" => {
                    for (i, x) in v.split(',').take(24).enumerate() {
                        w.hours[i] = x.trim().parse().unwrap_or(0);
                    }
                }
                _ => {}
            }
        }
        w.goal_ml=w.goal_ml.clamp(500,6000);
        w.bottle_ml=w.bottle_ml.clamp(50,6000);
        if w.start_day != today {
            // 跨天重置，与原工具 start= 字段语义一致
            w.start_day = today.to_string();
            w.drunk_ml = 0;
            w.refills = 0;
            w.hours = [0; 24];
            w.save();
        }
        w
    }

    pub fn save(&self) {
        let _ = write_kv(
            &config_dir().join("water.txt"),
            &[
                ("bottle".into(), self.bottle_ml.to_string()),
                ("goal".into(), self.goal_ml.to_string()),
                ("start".into(), self.start_day.clone()),
            ],
        );
        let _ = write_kv(
            &config_dir().join("water-extra.txt"),
            &[
                ("drunk".into(), self.drunk_ml.to_string()),
                ("refills".into(), self.refills.to_string()),
                ("remind".into(), if self.remind { "1".into() } else { "0".into() }),
                (
                    "hours".into(),
                    self.hours.iter().map(|h| h.to_string()).collect::<Vec<_>>().join(","),
                ),
            ],
        );
    }

    /// 喝一口：计入总量与当前小时。
    pub fn drink(&mut self, ml: u32, hour: usize) {
        let total=self.drunk_ml.saturating_add(ml).min(self.goal_ml.max(ml).saturating_mul(4));
        let added=total.saturating_sub(self.drunk_ml);
        self.drunk_ml=total;
        self.hours[hour.min(23)]=self.hours[hour.min(23)].saturating_add(added);
        self.save();
    }

    pub fn percent(&self) -> f32 {
        if self.goal_ml == 0 {
            0.0
        } else {
            (self.drunk_ml as f32 / self.goal_ml as f32 * 100.0).min(100.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn water_percent_clamps_at_100() {
        let w = Water { drunk_ml: 1000, ..Water::default() };
        assert!((w.percent() - 50.0).abs() < 0.01);
        let w = Water { drunk_ml: 5000, ..w };
        assert!((w.percent() - 100.0).abs() < 0.01);
    }
}
