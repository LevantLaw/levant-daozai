<div align="center">

<img src="assets/daozai-logo.png" width="112" alt="岛仔" />

# 岛仔 · Daozai

**一座小岛，装下桌面日常。**

Windows 桌面伙伴：消息、音乐、日程、天气、AI 额度与 AI 会话，都在屏幕边缘。

[![最新版本](https://img.shields.io/github/v/release/Renly1994/xiaotang-daozai?color=7955b8&label=最新版本)](https://github.com/Renly1994/xiaotang-daozai/releases/latest)
[![Windows](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D4)](#下载安装)
[![开源许可](https://img.shields.io/badge/License-MIT-3cbf91)](LICENSE)

**[下载 Windows 版](https://github.com/Renly1994/xiaotang-daozai/releases/latest)** · [使用指南](docs/USAGE.md) · [更新记录](CHANGELOG.md) · [反馈问题](https://github.com/Renly1994/xiaotang-daozai/issues)

</div>

岛仔平时收起在屏幕边缘，鼠标靠近就展开；切歌、看日程、复制验证码、处理 AI 会话，都能顺手完成。支持刘海与胶囊两种形态，顶部、左侧、右侧三边停靠，五款皮肤。使用 Rust 与 egui 构建，不依赖 Electron。

## 功能

| 功能 | 说明 |
| --- | --- |
| AI 会话 | 集中查看本机 Claude Code、Codex、ZCode、VS Code Copilot、DeepSeek Harness 会话；等待审批或回答时直接在岛上处理，完成、中断或失败时提醒 |
| AI 额度 | Codex、Claude、Kimi、Grok Bot、Gemini、ZCode、WorkBuddy、DeepSeek、Kiro、OpenCode Go、MiniMax、GLM 及中转站的额度、积分与余额 |
| 音乐 | 播放、暂停、切歌、音量与进度；同步歌词，收起时保留单行歌词；兼容网易云、QQ 音乐、酷狗、汽水 |
| 日程 | 月历与当日时间线，合并多个日历与邮件邀请并提前提醒；支持飞书、钉钉、企业微信、iCloud、CalDAV、Exchange 与 .ics |
| 待办 / 专注 / 健康 | 单次或每周重复待办；番茄钟与休息计时；喝水、久坐提醒 |
| 邮件 | 多邮箱 IMAP，验证码识别与一键复制，正文阅读，已读与删除管理 |
| Windows 通知 | 汇总即时通讯以外的系统通知，按来源应用显示图标 |
| 微信 / QQ | 未读提醒与窗口切换，多开账号分开显示，不读取聊天正文 |
| 置物架 / 剪贴板 | 拖入文件临时收集；找回最近 30 条文字并重新复制 |
| 天气 | 当前实况与七日预报，Open-Meteo 或和风天气（自备 Key） |
| 系统状态 | 电量、网络、音量、大写锁定、输入语言与蓝牙设备提示 |

功能页可在 **设置 → 岛体 → 功能页面** 中开关和排序；关闭的功能不在后台运行。各服务的连接方式与可读字段不同，以设置页为准。

## 下载安装

前往 **[最新正式版](https://github.com/Renly1994/xiaotang-daozai/releases/latest)** 下载：

| 文件 | 使用方式 |
| --- | --- |
| `Daozai-Setup-v版本号-x64.exe` | **推荐**。双击安装，可选择安装位置、桌面快捷方式和开机自启 |
| `Daozai-v版本号-windows-x64.zip` | 免安装。完整解压后运行 `daozai.exe`，用户数据仍保存在应用数据目录 |
| `SHA256SUMS.txt` | 校验文件，可用 `Get-FileHash -Algorithm SHA256` 比对 |

首次安装默认创建桌面快捷方式、不开机自启。更新在 **设置 → 关于 → 检查更新**。

系统要求 Windows 10 / 11 x64。默认使用 OpenGL，初始化失败时自动尝试 Direct3D 12；暂不提供 macOS、Linux 或 ARM64 成品包。程序未做商业代码签名，Windows 可能提示未知发布者，请核对下载来源与校验值。

## 数据与隐私

- 设置、待办、本机日程、饮水记录、剪贴板历史与置物路径保存在 `%APPDATA%\Daozai`。
- 邮箱和日历密码、API Key 保存在 Windows 凭据管理器；独立会话使用 DPAPI 加密。
- 微信提醒根据任务栏闪动判断新消息，不读取联系人或聊天正文；邮箱只读取收件箱。
- 岛仔没有自建的邮件或日程上传后台；联网功能只访问对应服务商与歌词、天气服务。
- 完整说明见 [隐私与安全](docs/PRIVACY.md)。

## 从源码构建

需要 Rust stable、MSVC C++ 构建工具和 Windows SDK。

```powershell
rustup default stable-x86_64-pc-windows-msvc
cargo build --locked --release --bin daozai
.\target\release\daozai.exe
```

测试与打包：

```powershell
cargo test --locked --all-targets --features record -- --test-threads=1
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\package.ps1
```

测试按单线程运行，以隔离使用进程环境变量的数据目录测试。打包需要 NSIS 3。项目结构与贡献方式见 [CONTRIBUTING.md](CONTRIBUTING.md)。
