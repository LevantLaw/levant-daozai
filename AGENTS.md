# 本机测试流程

- 验证二改运行 `target\x86_64-pc-windows-msvc\release\daozai.exe`（快速迭代可用 `target\debug\daozai.exe`），不要用桌面快捷方式或安装目录的旧版。
- 重新编译前先退出正在运行的岛仔；编译后启动新 exe 并核对进程路径。
- 桌面快捷方式「岛仔」指向安装目录 `D:\Program Files\Daozai\daozai.exe`，不是测试构建；二改全部确认前不覆盖安装目录。

# 本机修复版交付

二改全部验证通过后，覆盖安装目录的 `D:\Program Files\Daozai\daozai.exe`（桌面「岛仔」快捷方式指向此处）。交付前核对快捷方式目标、文件哈希和运行中的进程路径。

# 设置页布局规范

- 层级固定为：左侧栏分组 → 具体页 → 页同名标题 → 分组 → 子项。
- 页同名标题下不得直接放说明文字；说明只放在具体控件旁（`setting_row` 描述、`settings_hint`）或作为状态提示（断开、未检测到、功能关闭等）。
- 分组标题下通常不放说明文字；只有子项无法表达整组语义时（如「岛体 → 功能页面」）才允许一句，且只写一行。
- 功能页顶部的模块开关卡片可保留一句功能说明，它属于开关卡片，不是页标题说明。
- 说明文字至多一行，简洁有效，不写空话、重复话和操作教程。
- 「应用 → 岛体 → 岛上显示」是岛体组件开关的唯一入口：新增可开关的岛体组件时，开关必须加在这里，不得放到功能页或其他分组。
- 侧栏分组之间画一条横线；组间距统一，且大于组内行距。
- 「功能」组的顺序跟随岛上功能页顺序（`page_order`）；在「岛体 → 功能页面」拖动后点「保存」同步侧栏，不需要重启。
- 页面或分组底部的操作按钮放在右下角（`settings_action_row`）；主操作蓝色（`settings_primary_button`）、危险操作红色（`settings_danger_button`）。行内按钮（开关旁的连接/移除、账号或 Agent 行内操作）随所在行，不强制右下角。

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
