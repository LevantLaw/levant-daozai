//! 文件选择与原生拖出。

use windows::core::{Result, HSTRING};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

pub fn pick_files() -> Result<Vec<std::path::PathBuf>> {
    pick_files_for("添加文件到置物架",true)
}

pub fn pick_files_for(title:&str,multiple:bool) -> Result<Vec<std::path::PathBuf>> {
    unsafe {
        use windows::Win32::UI::Shell::*;
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL)?;
        let mut options=FOS_FORCEFILESYSTEM | FOS_FILEMUSTEXIST;
        if multiple{options|=FOS_ALLOWMULTISELECT;}
        dialog.SetOptions(options)?;
        dialog.SetTitle(&HSTRING::from(title))?;
        if let Err(e) = dialog.Show(None) {
            if e.code().0 as u32 == 0x800704c7 {
                return Ok(vec![]);
            }
            return Err(e);
        }
        let items = dialog.GetResults()?;
        let mut out = vec![];
        for i in 0..items.GetCount()? {
            let ptr = items.GetItemAt(i)?.GetDisplayName(SIGDN_FILESYSPATH)?;
            let text = ptr.to_string()?;
            windows::Win32::System::Com::CoTaskMemFree(Some(ptr.0 as _));
            out.push(text.into());
        }
        Ok(out)
    }
}
/// 构建资源管理器可识别的文件数据对象。
fn file_data_object(path: &std::path::Path) -> Result<windows::Win32::System::Com::IDataObject> {
    unsafe {
        use windows::Win32::UI::Shell::*;
        let wide: Vec<u16> = path
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let pidl = ILCreateFromPathW(windows::core::PCWSTR(wide.as_ptr()));
        if pidl.is_null() {
            return Err(windows::core::Error::from_win32());
        }
        let result = (|| {
            let mut child = std::ptr::null_mut();
            let folder: IShellFolder = SHBindToParent(pidl, Some(&mut child))?;
            folder.GetUIObjectOf(None, &[child as *const _], None)
        })();
        ILFree(Some(pidl));
        result
    }
}
/// 原生拖出只允许复制，取消拖放不会改变置物架与原文件。
pub fn drag_file(path: &std::path::Path) -> Result<()> {
    unsafe {
        use windows::Win32::System::Ole::{OleInitialize, OleUninitialize, DROPEFFECT_COPY};
        use windows::Win32::UI::Shell::SHDoDragDrop;
        let initialized = OleInitialize(None).is_ok();
        let result = file_data_object(path)
            .and_then(|data| SHDoDragDrop(None, &data, None, DROPEFFECT_COPY).map(|_| ()));
        if initialized {
            OleUninitialize();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dragged_file_exposes_native_file_drop_format() {
        unsafe {
            use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
            use windows::Win32::System::Ole::{OleInitialize, OleUninitialize, CF_HDROP};
            let initialized = OleInitialize(None).is_ok();
            let path =
                std::env::temp_dir().join(format!("daozai-drag-test-{}.txt", std::process::id()));
            std::fs::write(&path, "test").unwrap();
            {
                let data = file_data_object(&path).unwrap();
                let format = FORMATETC {
                    cfFormat: CF_HDROP.0,
                    dwAspect: DVASPECT_CONTENT.0,
                    lindex: -1,
                    tymed: TYMED_HGLOBAL.0 as u32,
                    ..Default::default()
                };
                assert!(data.QueryGetData(&format).is_ok());
            }
            std::fs::remove_file(path).unwrap();
            if initialized {
                OleUninitialize();
            }
        }
    }
}
