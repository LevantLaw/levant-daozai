//! 置物架条目：只记文件位置，不复制文件本体。

use crate::model::{atomic_write, config_dir};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub(crate) struct Shelf {
    pub items: Vec<PathBuf>,
}

impl Shelf {
    pub fn load() -> Self {
        let mut s = Self::default();
        if let Ok(text) = fs::read_to_string(config_dir().join("shelf.txt")) {
            for line in text.lines() {
                let p = PathBuf::from(line.trim());
                if !line.trim().is_empty() && !s.items.contains(&p) {
                    s.items.push(p);
                }
            }
        }
        s
    }

    pub fn save(&self) {
        let body: String = self
            .items
            .iter()
            .map(|p| format!("{}\n", p.display()))
            .collect();
        let path = config_dir().join("shelf.txt");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = atomic_write(&path, body.as_bytes());
    }

    /// 原工具的做法：记录文件所在位置，不复制文件本体（官网说明
    /// "the shelf remembers where your files live"）。
    pub fn add(&mut self, src: &Path) -> std::io::Result<()> {
        if !src.exists() { return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "文件不存在")); }
        if !self.items.iter().any(|p| p == src) {
            self.items.push(src.to_path_buf());
            self.save();
        }
        Ok(())
    }

    pub fn remove(&mut self, p: &Path) {
        self.items.retain(|item| item != p);
        self.save();
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.save();
    }
}
