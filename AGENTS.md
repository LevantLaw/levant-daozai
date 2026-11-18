# 本机修复版交付

修复验证通过后，统一覆盖桌面「岛仔－当前测试版」快捷方式指向的 `target/x86_64-pc-windows-msvc/release/daozai.exe`。交付前核对快捷方式目标和文件哈希。如果岛仔正在运行，切换到更新后的程序并核对进程路径。后续修复沿用同一入口，不另建临时交付目录。

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
