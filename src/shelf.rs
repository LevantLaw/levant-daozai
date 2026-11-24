//! 置物架：文件位置清单、岛内置物架页与设置。

mod state;
pub(crate) mod page;
pub(crate) mod settings;

pub(crate) use state::Shelf;

use crate::island::extras::PetState;
use crate::island::{App, SHELF_PAGE};
use std::path::PathBuf;

impl App {
    pub fn on_drop(&mut self, files: Vec<PathBuf>) {
        if !self.settings.visible_pages[SHELF_PAGE] || files.is_empty() { return; }
        self.drag_hot = false;
        self.drag_saved = None;
        if self.page != SHELF_PAGE {
            self.go(SHELF_PAGE, 1.0);
        }
        let (mut added,mut duplicate,mut failed) = (0,0,0);
        for p in files {
            if !p.exists() {failed+=1;continue;}
            if self.shelf.items.contains(&p) {
                duplicate+=1;
                continue;
            }
            if self.shelf.add(&p).is_ok() {
                self.shelf_born.insert(p.clone(), self.t);
                added += 1;
            }else{failed+=1;}
        }
        let message=if added==0 && failed==0 {
            if duplicate==1{"该文件已在置物架中".into()}else{"这些文件已在置物架中".into()}
        }else if added==0 && duplicate==0 {"文件不存在或无法访问".into()}
        else {
            let mut parts=Vec::new();
            if added>0{parts.push(format!("已添加 {added} 个文件"));}
            if duplicate>0{parts.push(format!("{duplicate} 个已存在"));}
            if failed>0{parts.push(format!("{failed} 个无法添加"));}
            parts.join("，")
        };
        self.say(message);
        if added>0 {
            self.shelf_badge_until=self.t+crate::island::page_badges::HOLD;
            self.pet.react(PetState::Happy);
            self.shelf_first = self.shelf.items.len().saturating_sub(5);
        }
    }
}
