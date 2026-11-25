# 参与开发

欢迎反馈兼容性问题、修复 bug 或完善使用说明。提交较大的功能前，先开 Issue 说明具体场景与预期行为。

## 本地开发

安装 Rust stable、Visual Studio C++ 构建工具和 Windows SDK，克隆仓库后运行：

```powershell
cargo build --locked --bin daozai
cargo test --locked --all-targets --features record -- --test-threads=1
```

为开发实例指定独立目录，避免覆盖日常使用数据：

```powershell
$env:DAOZAI_DATA_DIR = Join-Path $env:TEMP 'daozai-development'
cargo run --locked --bin daozai
```

常规测试使用合成数据。带 `#[ignore]` 的真实账号测试或截图导出测试须按需要显式运行；不要在公共 CI 中连接私人账号。

## 模块布局

`src/foo.rs` 是模块入口，声明子模块并导出 API；实现放在 `src/foo/`。不新建 `foo/mod.rs`。

- `src/` 顶层只放 `lib.rs`、`main.rs`、功能根目录、技术层根目录（`island`、`visuals`、`os`、`support`）、`model.rs` 和测试入口 `ui_tests.rs`；`startup/` 只由二进制声明。
- 新代码按归属放进对应功能根或技术层；跨功能复用的进程、时钟与资源工具放 `support/`。
- `os` 与 `visuals` 不依赖 `island`。

## 代码导航

| 目录／文件 | 内容 |
| --- | --- |
| `src/lib.rs`、`src/main.rs` | crate 入口；Windows 外壳、系统状态和后台服务衔接 |
| `src/island/` | 与平台无关的岛体界面、设置页与动效 |
| `src/visuals/` | 弹簧、矢量绘制、面板轮廓、配色字体 |
| `src/os/` | 仅 Windows 的窗口、输入、托盘与系统数据 |
| `src/support/` | 跨功能的进程、时钟、资源与测试辅助 |
| `src/startup/` | 启动诊断、单实例与图形后端（仅二进制使用） |
| `src/ai_session/`、`src/calendar/`、`src/clipboard/`、`src/focus/`、`src/health/`、`src/instant_messaging/`、`src/mail/`、`src/music/`、`src/quota/`、`src/shelf/`、`src/todo/`、`src/weather/`、`src/windows_toasts/` | 各功能模块 |
| `src/model.rs` | 共享数据模型与常量 |
| `src/ui_tests/` | 无窗口交互与布局回归测试，按功能分文件 |
| `tests/` | 以独立 crate 运行的集成测试 |
| `src/bin/daozai-record` | 合成数据演示与软件渲染器 |
| `vendor/` | 少量必要的上游补丁，见依赖说明 |
| `installer/`、`scripts/` | 安装、卸载与打包 |

## 提交要求

修 bug 时提供能复现的测试；界面变更附不含私人信息的截图。保持改动聚焦，不把个人配置、授权码、Cookie、编译产物或真实账号数据提交到仓库。

源码贡献按仓库 MIT 许可提交；第三方素材须同时提交准确的来源与许可声明。不要修改或移除 vendored 依赖的原始版权声明。
