# 岛仔 PRD：提醒铃声固定映射与外部通知接管

- 编号：PRD-004
- 状态：v3，固定映射与外部通知接管（方案 A）已实现，待手工验收
- 日期：2026-11-24
- 适用版本：岛仔 v0.6.66 之后（基于 `local` 分支 2026-11-24 工作区）
- 变更类型：提醒铃声从「总开关」改为固定分类映射；外部 Windows 通知纳入接管
- 硬约束：
  - 不新增设置键，不新增侧栏页，不提供逐类下拉或自定义 WAV。
  - AI 会话提示音保持现有 5 事件配置，不并入本映射。
  - 所有岛仔铃声仍受「提醒总开关」和「声音」开关控制。
  - 内置铃声只使用 `assets/notification-sounds/` 下的 WAV。
  - 外部通知只临时静音系统默认通知音，不替换来源 Toast、不按应用改写设置；退出必须恢复。

---

## 0. 调研结论

- 当前播放路径：`src/agent_sound.rs`（AI 5 音）、`src/tray.rs`（健康音）、`src/main.rs`（`Cmd::Notify`）。
- 当前只有健康两类有专属 WAV；饮水达标、专注、待办、会议、电量走 Windows Toast 默认音；新邮件静音。
- 外部 Windows 通知通过 `UserNotificationListener` 读取并展示；当前岛仔不发声，来源 Toast 声音照常。
- `UserNotificationListener` 只能读取通知，不能阻止来源 Toast 或按应用静音其声音；Windows 没有公开的按应用通知音 API。
- 唯一可行路径是临时改写系统默认通知音，退出恢复；方案 A 已确认。

## 1. 目标

1. 岛仔自己的提醒按固定映射播放 4 个内置 WAV 之一。
2. 升级后行为变化明确、可预期；不提供逐类配置。
3. 岛仔运行时接管外部 Windows 通知的展示，并临时静音系统默认通知音、统一播放 `sonar.wav`（见第 5 节）。
4. 避免 Toast 默认音与 WAV 叠加：所有岛仔 Toast 静音。
5. 即时通讯、AI、HUD 保持现有边界。

## 2. 非目标

- 不做逐类下拉、自定义 WAV、音量调节、铃声市场。
- 不改写用户的系统铃声方案：仅在接管期间临时静音默认通知音，退出恢复。
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
| 外部 | 非即时通讯的外部 Windows 通知 | sonar | `sonar.wav` |
| 静音 | 系统 HUD、蓝牙、AI 会话、即时通讯 | — | — |

- `doorbell.wav` 保留在素材目录，不参与映射，不删除。
- 健康提醒不再使用 `health-reminder.wav`；该文件已删除。
- 新邮件不发 Windows Toast，只播放 `email.wav`。
- 外部即时通讯通知不播放岛仔铃声；但接管期间系统默认通知音被静音，来源 Toast 也会变安静（见第 5 节）。

## 4. 播放与门控

- 开关层级：`notifications` → `system_notify`（仅 Windows Toast）→ `sound`。
- `notifications=false`：所有提醒与声音关闭。
- `system_notify=false`：不发 Windows Toast，也不播放系统提醒声音；新邮件仍可播放岛内 `email.wav`（它不走 Toast）。
- `sound=false`：Toast 静音，不播放 WAV。
- `sound=true`：按第 3 节映射播放 WAV；Toast 一律静音，避免双重发声。
- AI 会话提示音不受 `system_notify` 影响，仍由 `notifications && sound && 状态灯可见` 控制。
- 外部通知 sonar：`notifications && sound && read_notifications && Windows 功能页开启`，且通知监听已连接；不受 `system_notify` 影响。
- 系统默认通知音的临时静音与外部通知 sonar 条件同步；任一条件关闭、监听断开或退出岛仔时立即恢复。
- 同一事件在去重窗口内只播放一次。

## 5. 外部 Windows 通知接管

目标：岛仔运行时，外部应用的通知由岛仔展示并播放 `sonar.wav`。

技术限制：

- Windows 的 `UserNotificationListener` 只能读取通知，不能阻止来源 Toast 或来源声音。
- 没有公开的按应用静音通知音 API；`HKCU\...\Notifications\Settings\<AUMID>` 只有 `Enabled`、`ShowBanner` 等，没有 `Sound`。
- 因此「岛仔播放 sonar + 来源应用不响」只能通过临时改写系统默认通知音实现。

已确认方案 A：运行时临时静音 Windows 默认通知音。

- 目标键：`HKCU\AppEvents\Schemes\Apps\.Default\Notification.Default\.Current` 的默认值（REG_SZ，空串表示无声）。
- 静音前把原值写入 `%APPDATA%\Daozai\notification-sound.backup`，再写入空串。
- 正常退出（托盘退出、重启）时恢复原值并删除备份。
- 启动时自检备份：当前值仍为空串才恢复，避免覆盖用户期间改过的铃声；若当前值已被用户改成非空，丢弃旧备份。
- 静音条件：`notifications && sound && read_notifications && Windows 功能页开启` 且监听已连接。条件不满足时不静音；条件变化时下一帧同步。
- 外部非即时通讯通知被接收后播放 `sonar.wav`；即时通讯不播放岛仔铃声。

已知影响：

- 静音是全局的，接管期间所有使用默认通知音的应用（包括即时通讯的 Toast）都会变安静；岛仔只为非即时通讯的外部通知补播 sonar。
- 用户自定义的默认通知音在接管期间被临时覆盖，退出后恢复。
- 异常退出（崩溃、任务管理器结束进程）可能残留静音；下次启动按备份自检恢复。
- 只能覆盖使用 `Notification.Default` 事件的应用；自带独立提示音的应用不受影响。

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
  - 外部通知：来源默认音 → 来源静音 + 岛仔 `sonar.wav`（即时通讯只静音来源，不补播）。
- 旧配置 `sound=false` 用户升级后仍不发声，系统默认通知音也不会被静音。
- 外部接管在监听未连接或 Windows 功能页关闭时不启用，系统通知音保持原样。

## 9. 验收标准

1. 固定映射逐项正确，Toast 与 WAV 不叠加。
2. 新邮件只播放 `email.wav`，不新增 Windows Toast。
3. `notifications=false`、`system_notify=false`、`sound=false` 的门控符合第 4 节。
4. AI 提示音、HUD、即时通讯边界不变。
5. 外部通知按方案 A 验收：接管期间系统默认通知音静音、外部通知播放 sonar、即时通讯不补播；关闭开关、断开监听或退出后系统铃声恢复。
6. 异常退出后重新启动，系统铃声能按备份恢复；用户期间改过铃声时不覆盖新值。
7. `doorbell.wav` 保留且不被引用。
8. 现有测试通过，新增映射、门控与外部通知测试通过。

## 10. 测试

- 分类映射：完成 / 提醒 / 邮件 / 默认四类。
- 门控：总开关、系统提醒、声音。
- 新邮件：只发 `PlayNotificationSound(Email)`，不发 `Notify`。
- 外部通知：非即时通讯发 `PlayNotificationSound(Sonar)` 且不发 `Notify`；`sound=false` 不发声；即时通讯不发声。
- 系统静音：手工验证真实 Toast 声音被静音、退出后恢复、异常退出后下次启动恢复。
- AI / HUD / IM：不产生新铃声。

## 11. 待确认

1. 素材来源与再分发授权。
2. `doorbell.wav` 长期保留还是后续删除（当前按用户要求保留）。
3. 接管期间即时通讯 Toast 也被静音是否符合预期；如需保留其声音，只能放弃全局静音。

## 12. 代码位置

- 枚举与命令：`src/app.rs`（`NotificationSound`、`Cmd::Notify`、`Cmd::PlayNotificationSound`）
- 映射与入队：`src/extras.rs`（`Alert::notification_sound`、`enqueue_alert`）
- 播放与系统静音：`src/tray.rs`（`play_notification_sound`、`set_system_notification_muted`）
- 命令处理与静音状态同步：`src/main.rs`（`Cmd::Notify`、`Cmd::PlayNotificationSound`、`sync_notification_mute`）
- 退出恢复：`src/app.rs`（托盘退出回调）、`src/tray.rs`（重启退出）
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
| 外部通知 | 不发声，来源默认音 | 来源静音，岛仔 sonar（方案 A） |
| 设置入口 | 无分类设置 | 无新增设置 |
