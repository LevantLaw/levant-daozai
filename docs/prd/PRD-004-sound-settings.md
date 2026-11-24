# 岛仔 PRD：提醒铃声固定映射与外部通知接管

- 编号：PRD-004
- 状态：v4，固定映射已实现；外部通知保留来源铃声，全局静音方案已撤销
- 日期：2026-11-24
- 适用版本：岛仔 v0.6.66 之后（基于 `local` 分支 2026-11-24 工作区）
- 变更类型：提醒铃声从「总开关」改为固定分类映射；外部 Windows 通知不再接管声音
- 硬约束：
  - 不新增设置键，不新增侧栏页，不提供逐类下拉或自定义 WAV。
  - AI 会话提示音保持现有 5 事件配置，不并入本映射。
  - 所有岛仔铃声仍受「提醒总开关」和「声音」开关控制。
  - 内置铃声只使用 `assets/notification-sounds/` 下的 WAV。
  - 不改写系统或来源应用的通知音设置；外部通知沿用来源 Toast 铃声。

---

## 0. 调研结论

- 当前播放路径：`src/agent_sound.rs`（AI 5 音）、`src/tray.rs`（健康音）、`src/main.rs`（`Cmd::Notify`）。
- 当前只有健康两类有专属 WAV；饮水达标、专注、待办、会议、电量走 Windows Toast 默认音；新邮件静音。
- 外部 Windows 通知通过 `UserNotificationListener` 读取并展示；岛仔不发声，来源 Toast 声音照常。
- `UserNotificationListener` 只能读取通知，不能阻止来源 Toast 或按应用静音其声音；Windows 没有公开的按应用通知音 API。
- 方案 A（临时改写系统默认通知音）曾实现，但会连带静音即时通讯的原生通知音（微信等直接使用系统默认通知音），已撤销。

## 1. 目标

1. 岛仔自己的提醒按固定映射播放 4 个内置 WAV 之一。
2. 升级后行为变化明确、可预期；不提供逐类配置。
3. 岛仔运行时接管外部 Windows 通知的展示，但声音保持来源应用原样（见第 5 节）。
4. 避免 Toast 默认音与 WAV 叠加：所有岛仔 Toast 静音。
5. 即时通讯、AI、HUD 保持现有边界。

## 2. 非目标

- 不做逐类下拉、自定义 WAV、音量调节、铃声市场。
- 不改写用户的系统铃声方案或来源应用的通知音设置。
- 不替换或阻止来源 Toast 本身，不按应用单独静音。
- 不改提醒触发条件、间隔、展示时长。
- 不改 AI 提示音的 5 事件模型。

## 3. 固定映射

| 分类 | 事件 | 铃声 | 文件 |
|---|---|---|---|
| 完成 | 专注计时、待办到时间了、今日饮水目标达成 | complete | `assets/notification-sounds/complete.wav` |
| 提醒 | 喝水时间到了、该起来活动了、会议提醒、电量偏低 | reminder | `reminder.wav` |
| 邮件 | 新邮件、收到新邮件、邮件验证码 | email | `email.wav`（裁静音、归一化） |
| 默认 | 其他岛仔 Windows Toast | sonar | `sonar.wav` |
| 外部 | 外部 Windows 通知（含即时通讯） | 来源应用铃声 | — |
| 静音 | 系统 HUD、蓝牙、AI 会话 | — | — |

- `doorbell.wav` 保留在素材目录，不参与映射，不删除。
- 健康提醒不再使用 `health-reminder.wav`；该文件已删除。
- 新邮件不发 Windows Toast，只播放 `email.wav`。
- 外部通知（含即时通讯）不播放岛仔铃声，声音由来源应用自己发出。

## 4. 播放与门控

- 开关层级：`notifications` → `system_notify`（仅 Windows Toast）→ `sound`。
- `notifications=false`：所有提醒与声音关闭。
- `system_notify=false`：不发 Windows Toast，也不播放系统提醒声音；新邮件仍可播放岛内 `email.wav`（它不走 Toast）。
- `sound=false`：Toast 静音，不播放 WAV。
- `sound=true`：按第 3 节映射播放 WAV；Toast 一律静音，避免双重发声。
- AI 会话提示音不受 `system_notify` 影响，仍由 `notifications && sound && 状态灯可见` 控制。
- 外部通知不播放岛仔铃声，也不改写系统通知音设置。
- 同一事件在去重窗口内只播放一次。

## 5. 外部 Windows 通知接管

目标：岛仔运行时，外部应用的通知由岛仔展示，声音保持来源应用原样。

技术限制：

- Windows 的 `UserNotificationListener` 只能读取通知，不能阻止来源 Toast 或来源声音。
- 没有公开的按应用静音通知音 API；`HKCU\...\Notifications\Settings\<AUMID>` 只有 `Enabled`、`ShowBanner` 等，没有 `Sound`。
- 曾采用方案 A：临时改写 `HKCU\AppEvents\Schemes\Apps\.Default\Notification.Default\.Current` 全局静音系统默认通知音，再为外部通知补播 `sonar.wav`。
- 方案 A 已撤销：静音是全局的，微信等直接使用系统默认通知音的即时通讯也会一起变安静，违背「保留原生铃声」的要求。
- 由于无法按应用静音，保留来源铃声后外部通知不能再叠加岛仔 `sonar.wav`，否则双重发声。

当前行为：

- 外部通知只展示、不播放岛仔铃声，来源 Toast 及其声音照常。
- 岛仔不改写系统铃声设置，也不保留备份文件。
- 即时通讯（微信、QQ 等）的原生通知音不受影响。

## 6. 素材

| 文件 | 处理 | 时长 | 格式 |
|---|---|---|---|
| `sonar.wav` | 裁静音、归一化 | 2.10s | 22050 Hz mono 16-bit |
| `complete.wav` | 裁静音、归一化 | 1.14s | 同上 |
| `reminder.wav` | 裁静音、归一化 | 1.61s | 同上 |
| `email.wav` | 裁静音、归一化 | 1.45s | 同上 |
| `doorbell.wav` | 保留，不参与映射 | 6.84s | 48 kHz mono 24-bit |

- 所有运行时铃声峰值约 -3 dBFS。
- 原始素材已清理，仓库只保留运行时 WAV。
- 素材来源与授权待确认；公开仓库不能分发未授权铃声。

## 7. 设置页影响

- 不新增侧栏页、不新增分组、不新增设置键。
- 现有「声音」开关继续表示总开关；「系统提醒」三档不变。
- 不提供逐类选择，避免为「选择」而制造选择。

## 8. 迁移与行为变化

- 不新增 `settings.txt` 键。
- 升级后行为变化：
  - 健康提醒：`health-reminder.wav` → `reminder.wav`。
  - 饮水达标、专注、待办、会议、电量：Windows 默认音 → `complete.wav` / `reminder.wav`。
  - 新邮件：静音 → `email.wav`（仍不发 Toast）。
  - 未分类岛仔 Toast：Windows 默认音 → `sonar.wav`。
  - 外部通知：保持来源铃声，岛仔不发声。
- 旧配置 `sound=false` 用户升级后仍不发声。
- 外部通知的展示仍受 `read_notifications` 与 Windows 功能页开关控制，声音不受影响。

## 9. 验收标准

1. 固定映射逐项正确，Toast 与 WAV 不叠加。
2. 新邮件只播放 `email.wav`，不新增 Windows Toast。
3. `notifications=false`、`system_notify=false`、`sound=false` 的门控符合第 4 节。
4. AI 提示音、HUD、即时通讯边界不变。
5. 外部通知不播放岛仔铃声，来源应用（含即时通讯）的原生通知音照常；系统铃声设置不被改写。
6. `doorbell.wav` 保留且不被引用。
7. 现有测试通过，新增映射、门控与外部通知测试通过。

## 10. 测试

- 分类映射：完成 / 提醒 / 邮件 / 默认四类。
- 门控：总开关、系统提醒、声音。
- 新邮件：只发 `PlayNotificationSound(Email)`，不发 `Notify`。
- 外部通知：不发 `Notify`、不发 `PlayNotificationSound`，来源 Toast 声音照常；即时通讯同样不发声。
- 系统铃声：手工验证即时通讯原生通知音在岛仔运行时正常播放。
- AI / HUD / IM：不产生新铃声。

## 11. 待确认

1. 素材来源与再分发授权。
2. `doorbell.wav` 长期保留还是后续删除（当前按用户要求保留）。

## 12. 代码位置

- 枚举与命令：`src/app.rs`（`NotificationSound`、`Cmd::Notify`、`Cmd::PlayNotificationSound`）
- 映射与入队：`src/extras.rs`（`Alert::notification_sound`、`enqueue_alert`）
- 播放：`src/tray.rs`（`play_notification_sound`）
- 命令处理：`src/main.rs`（`Cmd::Notify`、`Cmd::PlayNotificationSound`）
- AI 提示音：`src/agent_sound.rs`、`src/agent_ui.rs`
- 外部通知监听：`src/system.rs`
- 测试：`src/health_reminder_ui_tests.rs`

## 附录 A：现状与目标差异

| 项目 | 现状 | 目标 |
|---|---|---|
| 铃声配置 | 一个总开关 | 固定分类映射 |
| 健康 | 专属 WAV | reminder |
| 饮水达标 / 专注 / 待办 | Windows 默认音 | complete |
| 会议 / 电量 | Windows 默认音 | reminder |
| 新邮件 | 静音 | email（岛内） |
| 默认岛仔 Toast | Windows 默认音 | sonar |
| 外部通知 | 不发声，来源默认音 | 不发声，来源默认音 |
| 设置入口 | 无分类设置 | 无新增设置 |
