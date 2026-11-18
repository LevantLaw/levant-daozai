# 本机测试流程

- 验证二改运行 `target\x86_64-pc-windows-msvc\release\daozai.exe`（快速迭代可用 `target\debug\daozai.exe`），不要用桌面快捷方式或安装目录的旧版。
- 重新编译前先退出正在运行的岛仔；编译后启动新 exe 并核对进程路径。
- 桌面快捷方式「岛仔」指向安装目录 `D:\Program Files\Daozai\daozai.exe`，不是测试构建；二改全部确认前不覆盖安装目录。

# 本机修复版交付

二改全部验证通过后，覆盖安装目录的 `D:\Program Files\Daozai\daozai.exe`（桌面「岛仔」快捷方式指向此处）。交付前核对快捷方式目标、文件哈希和运行中的进程路径。

# Git 分支约定

本仓库是 `Renly1994/xiaotang-daozai` 的 fork：`origin` 指向 `LevantLaw/levant-daozai`，`upstream` 指向作者仓库。

- `main` 只跟随上游，不提交本地修改；所有二改在 `local` 分支
- 同步上游更新：

```
git fetch upstream
git switch main
git merge --ff-only upstream/main
git push origin main
git switch local
git merge main
```

- 二改提交后推送 `local` 到 `origin` 备份
