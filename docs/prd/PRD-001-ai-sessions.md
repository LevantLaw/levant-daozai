# 岛仔 PRD：AI 会话监控与岛上应答

- 编号：PRD-001
- 状态：草案 v3，决策已锁定（待开工批准）
- 日期：2026-11-19
- 适用版本：岛仔 v0.6.66 之后
- 硬约束：现有「AI 额度」页面、设置、额度环、`ai-quota.json` 全部保持不变；本功能只做加法。

---

## 0. 已确认的决策

1. **入口**：新增「AI 会话」页（第 10 个功能页），不改「AI 额度」页。
2. **支持的 Agent**：Claude Code、Codex、DeepSeek Harness（DSH）、VS Code Copilot、ZCode；其他暂不做。
3. **Hook 清理**：只保留 RTK 与岛仔自己的 Hook；移除 Fly Island 的全部 Hook 和独立的 `play-sound.ps1` 提示音 Hook，提示音改由岛仔播放。
4. **会话显示**：参考 CodeIsland，闲置/关闭的会话不再在岛上活跃或显示；会话记录默认保留 30 分钟，可设置（1–60 分钟，步长 5）；设置控件不用下拉框。
5. **架构要求**：不照搬 CodeIsland；高效、模块化、易维护，后续加新 Agent 只加配置和少量适配代码，不改 UI。
6. **Pebrel Hook**：暂时保留，不在本次清理范围内。
7. **Copilot 变体**：目标是 VS Code 内置 Copilot Chat，不是 Copilot CLI。
8. **审批门控**：只有启用监控的 Agent 才开启审批；关闭监控时 Hook 保留但立即返回空决定，不自动清理；需要干净配置时用显式「移除 Hook」。
9. **完成/失败**：在岛上弹一次横幅，自动收起、不静默，同时播放完成/失败提示音；会话仍从列表消失。
10. **保留步长**：5 分钟（不是秒）；范围 1–60 分钟。
11. **跳转层级**：对齐 fly-island——窗口优先，标签页/终端尽力匹配，不做窗格；桌面端用 deep link 到具体会话。
12. **清理时机**：现在清理，不等「AI 会话」启用；清理前备份。
13. **DSH**：先按文件格式实现并标记「未验证」，等本机安装后补测。

---

## 1. 背景与参考

### 1.1 fly-island 的两个功能

**「AI 会话」**：把本机正在运行的 Agent 会话集中到一个页面。

- 实时状态：运行中、等待确认、等你回答、已完成、失败、可能中断。
- 会话行：项目/标题、模型、状态灯、最近回复、更新时间。
- 就地处理：工具调用允许/拒绝/总是允许；`AskUserQuestion` 选项选择。
- 点击跳转到终端标签页、IDE 或桌面应用。
- 收起时显示 Agent 指示灯、会话数，可显示当前会话标题。

**「接入 AI 工具」**：设置里的 Hook 安装/修复入口。

- 把 Hook 写进 Claude Code / Codex 等工具的配置文件，让事件到达岛。
- 一键安装/修复；Codex 首次需在 `/hooks` 信任。
- 本质是接线，不是日常功能。

### 1.2 CodeIsland 怎么做（只借鉴，不照搬）

CodeIsland 有 macOS 版和独立的 Windows WPF 版。Windows 版的关键实现：

**会话生命周期**

- 状态：`Idle`、`Running`、`WaitingForPermission`、`WaitingForAnswer`、`Completed`、`Failed`，另有保留的 `Cancelled`。
- `SessionEnd` → `Completed`；`Remove(sessionId)` 显式删除。
- `RemoveExpired(cutoff)` 删除 `UpdatedAt < cutoff` 的会话。
- 主程序用 1 分钟一次的 `DispatcherTimer` 调用：
  `Sessions.RemoveExpired(DateTimeOffset.UtcNow.AddMinutes(-_settings.SessionCleanupMinutes))`。
- `SessionCleanupMinutes` 默认 **30**，范围 1–1440；设置界面是 **TextBox**，不是下拉框。
- 显示过滤的 `active` 模式只显示非 `Completed`/`Cancelled`/`Failed` 的会话。
- `MaxVisibleSessions` 默认 5，范围 1–20。
- 状态机本身没有基于时间的 idle 定时器；`Idle` 是状态兜底。

**Hook 安装**

- 用一个 `HookTool` 描述表：Agent、显示名、可执行文件名、配置路径、Hook 标记、事件列表、配置格式、超时。
- 格式策略：`Claude`、`EventMap`、`Cursor`、`OpenCode`。
- Hook 命令统一为 `<bridge> --source <agent> --event <eventName>`。
- 安装是幂等的：先删除包含注册 ID 的旧条目，再追加；保留用户其他 Hook。
- 安装前备份到 `%LOCALAPPDATA%\CodeIsland\hook-backups`。
- 卸载只删自己的注册 ID，事件数组空了就删数组，`hooks` 空了就删 `hooks`。
- 检测靠 PATH 可执行文件 + 配置路径是否存在；健康状态看注册 ID、协议版本、事件列表是否匹配。

**Hook 开关与卸载**

- Hooks 页只有「安装 / 修复 / 移除」，没有逐 Agent 的启用开关；「移除」会真正删掉自己的条目。
- 退出应用不清理 Hook；下次启动检测到「已安装但配置不健康」才自动修复。
- 副作用：应用未运行时 Hook 仍会被调用，只是连接失败后快速退出。

**声音**

- `NotificationSoundManager` 把事件映射到 Windows 系统音：
  - SessionStart → Asterisk
  - PermissionRequest / Question → Exclamation
  - SessionEnd → Beep
  - Error → Hand
- `SoundEnabled` 默认 true；没有音量/静音细节。

**Agent 覆盖**

- Windows 版：Claude、Codex、Gemini、Cursor、OpenCode、Qoder、Factory、CodeBuddy、Copilot CLI。
- macOS 版另有 ZCode 支持；DSH 通过外部 `dsh-island` 插件。
- **Windows 版没有 ZCode 和 DSH**，所以这两者要按我们自己的需求补。

**ZCode 的关键约束（CodeIsland macOS 测试）**

- ZCode 是 Electron 桌面应用，不是普通 CLI。
- 配置：`~/.zcode/cli/config.json` 的 `hooks` 对象，含 `enabled` 与 `events`。
- 事件严格白名单 7 个：`SessionStart`、`UserPromptSubmit`、`PreToolUse`、`PermissionRequest`、`PostToolUse`、`PostToolUseFailure`、`Stop`；写错事件名可能导致整个 hooks 配置被上游丢弃。
- `PermissionRequest` 超时用 86400 秒；其他事件不写 timeout。
- 审批返回只接受 `{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}`；**不要**返回 Claude 的 `updatedPermissions`。
- ZCode 的「总是允许」由岛仔在会话内记住，不写进 ZCode 的数据库。
- 未测试出提问回答流程；`PermissionRequest` 是唯一交互路径。

**Copilot 的关键事实**

- CodeIsland Windows 支持的是 **Copilot CLI**：`~/.copilot/hooks/codeisland.json`，事件为 camelCase（`sessionStart`、`preToolUse`、`postToolUse`、`sessionEnd`）。
- VS Code Copilot Chat 0.69.0（本机 VS Code 1.141 内置）已支持 Agent Hooks（Preview），事件包含 `SessionStart`、`UserPromptSubmit`、`PreToolUse`、`PostToolUse`、`Stop` 等，且兼容 Claude Code 的 Hook 格式；它从 `chat.hookFilesLocations` 和 `.github/hooks` 读取 Hook。
- 两者的全局注入位置不同，**必须在 P0 验证**；适配器要能区分 `copilot-vscode` 与 `copilot-cli`。

**DSH 的关键事实**

- 没有 Claude/Codex 式 Hook；只能读本地会话文件。
- 会话日志：`sessions/<project-shard>/session-<UUID>/session.v<N>.jsonl.zstd`，只解析 `approval/asked`、`approval/decided`、`turn/start`、`turn/end`。
- 投影缓存：`storages/session_projcache/sessions/session-<UUID>.json`，提供标题、目录、模型、状态、上下文 token。
- 目录：`~/.dsh`、`~/.dsh-beta`、`%APPDATA%/dsh-desktop/harness`。
- 只能做到只读状态 + 聚焦主窗口；不能回答提问、不能审批、不能精确跳转会话。

### 1.3 fly-island 的开关与跳转（Windows 参考）

**审批开关**

- 开关默认关闭；Hook 无论如何都安装。
- 关闭时 Hook 仍被调用，但本地端点直接返回空决定（不拦截、不弹窗），因此不需要重装 Hook 或重启 Agent。
- 开关控制「是否介入」，不控制「是否注册」。

**跳转层级**

- 先激活会话所在窗口；窗口内再尽力定位标签页或终端。
- Windows Terminal 仅在标题唯一时用 UI Automation 切标签；Orca 终端直接切；VS Code 用扩展定位。
- 不做 tmux/zellij/Herdr 之类的窗格级定位。
- 精确会话靠 deep link（如 VS Code 的会话 URI）。
- 找不到时提示「没找到对应的窗口」，不静默失败。

**CodeIsland（macOS）对照**

- 能定位到精确终端标签页、IDE 窗口或 tmux/zellij/Herdr 窗格。
- Windows 移植版没有这套窗格能力，因此本项目以 fly-island 为基准。

### 1.4 借鉴与不照搬

| 借鉴 | 不照搬 |
|---|---|
| 描述表驱动 Agent 元数据 | 不为每个 Agent 写一套 UI/状态机 |
| 统一 Hook 命令 + 事件名 | 不把注册 ID 当唯一标记、不用宽泛字符串匹配删除条目 |
| 幂等安装/修复/卸载 | 不用直接 `File.WriteAllText`；岛仔用原子写入 |
| 安装前备份 | 不覆盖格式损坏的容器；解析失败就拒绝写入并提示 |
| `SessionCleanupMinutes` 默认 30 | 不用下拉框；用数字步进器 |
| active 过滤 + 定时清理 | 岛仔更严格：闲置/关闭都不显示，只显示运行中/等待 |
| 事件到提示音的映射 | 岛仔自己播放 WAV，不调用 PowerShell |

### 1.5 本机现状

- Claude Code 2.1.323；`~/.claude/settings.json` 的 Fly Island 条目（`focusd-agent-*`、活动 curl、HTTP Hook）和 `play-sound.ps1` 已于 2026-11-19 清理，保留 Pebrel、rtk。
- Codex 0.169.0；`~/.codex/hooks.json` 的 `play-sound.ps1` 已清理，保留 rtk、Pebrel；`config.toml` 有 `[hooks.state]` 信任记录；`config.toml` 末尾的 FocuSD 标记块已于 2026-11-19 清理（备份 `config.toml.20261119-110634.bak`）。
- ZCode：`~/.zcode/cli/config.json` 目前只有 MCP，没有 hooks。
- VS Code Copilot：VS Code 1.141.0；安装包确认 Hook 从 `~/.copilot/hooks` 与 `~/.claude/settings.json` 加载，事件不含审批/提问；用户设置里 `chat.useClaudeHooks` 已开、`chat.useHooks` 未开。
- DSH：本机未安装。
- 现有提示音素材：`~/Documents/ai-hook-sounds/{start,submit,approval,complete,error}.wav`，来自 CodeIsland（MIT）。

**清理记录**

- 备份：`%APPDATA%\Daozai\agent-hook-backups\settings.json.20261119-094948.bak`、`hooks.json.20261119-094948.bak`、`config.toml.20261119-110634.bak`。
- 清理结果：Claude 删除 23 组、Codex 删除 5 组，Codex `config.toml` 删除 1 个 FocuSD 标记块；JSON 校验通过，TOML 尾部结构人工核对；无 `focusd`/`47830`/`play-sound` 残留。
- 清理后到新功能上线前，Claude/Codex 没有会话提示音；由岛仔新功能接回。
- Fly Island 若重新运行并自动修复 Hook，会再次写回，需要关闭它的 Hook 功能或退出它。

---

## 2. 目标与非目标

### 2.1 目标

1. 在岛仔里看到 5 类 Agent 的活跃会话：项目、状态、持续时间、当前工具/最近回复。
2. 需要人时在岛上提示，支持 Claude Code / Codex / ZCode（以及 P0 验证后的 Copilot）就地审批。
3. 支持 Claude Code 的 `AskUserQuestion` 岛上回答。
4. 点击会话跳到对应窗口；找不到时明确提示。
5. 按 Agent 类型人工勾选是否监控；Hook 安装/修复/移除由岛仔完成。
6. 闲置/关闭的会话不显示；记录保留 30 分钟（可设置），到点清理。
7. 提示音由岛仔播放，移除外部 PowerShell 提示音。
8. 模块化：新增 Agent 只加描述、归一化函数和少量适配，不改页面/设置/状态机。

### 2.2 非目标

- 不改 AI 额度。
- 不做 CodeIsland 式每工具像素吉祥物/粒子动效。
- 不做自动批准、自动切换权限模式。
- 不做 SSH 远程、手机/手表、推送、Webhook。
- 不做会话正文持久化、全文搜索、用量分析。
- DSH 只读；不审批、不回答、不精确跳转。
- Codex 不做提问回答（无 Hook）。
- ZCode 不做提问回答（未发现可用 Hook）。
- 不解决多个审批器同时接管；通过清理 Fly Island Hook + 检测提示规避。

---

## 3. 用户故事

1. 我在多个项目里开了 Claude Code / Codex / ZCode，想知道哪个在跑、哪个在等我。
2. Claude Code 要执行命令，我在岛上看到完整命令，点「允许一次」，终端继续。
3. Claude Code 问我方案，我在岛上点选项，终端直接收到答案。
4. 一个会话跑完或关掉了，岛上弹一下完成/失败提示，然后它从列表消失；30 分钟内我再打开，记录还在，不会重复提醒。
5. 我不想监控 Copilot，只勾选 Claude Code 和 ZCode；没勾选的 Agent 即使 Hook 还在，也不会在岛上弹窗。
6. 我升级岛仔后 Hook 路径变了，设置页显示「需要修复」，点一下恢复。
7. 我关掉岛仔，Agent 仍能在终端正常审批，不会被卡住。
8. 以后新增一个 Agent，我只加一份描述和归一化函数，页面自动出现它。
9. 我取消勾选监控后不想保留配置，可以点「移除 Hook」；不点也不会自动清理。

---

## 4. 支持矩阵与优先级

| Agent | 接入方式 | 可审批 | 可回答 | 会话发现 | 跳转 |
|---|---|---|---|---|---|
| Claude Code | `~/.claude/settings.json` Hook | 是 | 是 | Hook | 终端/IDE/桌面 deep link |
| Codex | `~/.codex/hooks.json` Hook | 是（allow/deny） | 否 | Hook + transcript 兜底 | 终端/Codex 桌面 |
| ZCode | `~/.zcode/cli/config.json` Hook | 是（仅 behavior） | 否 | Hook | ZCode 窗口 |
| VS Code Copilot | Hook（P0 定位置） | P0 验证 | P0 验证 | Hook / 工作区文件 | VS Code 窗口 |
| DSH | 无 Hook，文件监听 | 否 | 否 | zstd 日志 + 投影缓存 | 主窗口 |

**阶段**

| 阶段 | 内容 |
|---|---|
| P0 | 验证 5 个 Agent 的 Hook 路径/事件/输出；验证 VS Code Copilot 注入位置；验证 DSH 文件格式（若安装）；验证阻塞、超时、终端先答、窗口定位 |
| P1 | 模块化核心：Agent 注册表、通用安装器、中继、会话状态机、页面、常驻槽、提醒、跳转（Claude/Codex/ZCode） |
| P2 | 审批：Claude / Codex / ZCode；清理旧 Hook；岛仔提示音 |
| P3 | Claude 提问回答；VS Code Copilot 适配；DSH 只读文件监听；标签页/终端尽力匹配 |
| P4 | 打磨：标题、已读未读、免打扰、测试与文档 |

---

## 5. 功能需求

### 5.1 接入 AI 工具

设置 → 功能 → **AI 会话** 页内的「接入 AI 工具」卡片。

每个 Agent 一行：

- 图标、名称、检测到的版本/变体。
- 状态：`已接入` / `未接入` / `需要修复` / `版本不支持` / `等待信任` / `只读`。
- 操作：`安装/修复`、`移除`。
- 监控勾选框：勾选后才接收事件并显示会话；未勾选时 Hook 可保留，但中继立即返回空决定。
- 审批子开关：仅在勾选监控后可用；关闭时同样返回空决定。

安装行为：

- 合并写入，只增删岛仔条目；写入前备份；原子写入。
- 解析失败/无权限：拒绝写入，显示路径和错误，不破坏原文件。
- 移除只删岛仔条目，保留其他工具。
- 升级/路径变化后显示「需要修复」，一键重写。
- 安装状态以实际配置文件扫描为准，不信任缓存标记。

### 5.2 会话状态与显示

状态：

| 状态 | 含义 | 岛上显示 |
|---|---|---|
| 运行中 | 一轮任务进行中 | 显示 |
| 等待确认 | 工具调用待批准 | 显示 |
| 等待回答 | Agent 提问待回答 | 显示 |
| 闲置 | 没有进行中的任务 | 不显示 |
| 已完成 | `Stop` / `SessionEnd` | 不显示（弹一次完成提醒） |
| 失败 | `StopFailure` / 错误 | 不显示（弹一次失败提醒） |
| 可能中断 | 运行中超过 10 分钟无事件 | 先显示黄色，提醒一次后隐藏 |

规则：

- 岛上列表、常驻状态点、会话数只统计「显示」状态。
- 会话记录在隐藏后继续保留，用于迟到事件合并和恢复；默认保留 **30 分钟**，可设置 1–60 分钟。
- 清理：每 60 秒执行一次，删除 `updated_at < now - 保留分钟` 的记录。
- 隐藏会话收到活跃事件（`UserPromptSubmit`、权限请求、提问）时重新显示。
- 等待中的会话不因超时变成「可能中断」。
- 同一会话同一时间只保留一个待处理请求；新请求覆盖旧的（旧的返回空决定）。
- 排序：等待 > 运行；同组按最近活动倒序。
- 最多显示 12 个；页面一次显示 3/4/5 行（可设置），超出滚动。

### 5.3 常驻胶囊

- 新增槽位「AI 会话」：Agent 图标 + 5px 状态点 + 会话数。
- 颜色：运行=蓝、等待=橙（呼吸）、失败=红（短暂）、可能中断=黄；完成/闲置不显示。
- 无显示会话或模块关闭时隐藏；极简模式隐藏。
- 点击打开 AI 会话页；悬停显示 `Claude Code · 2 个会话 · 1 个等待你`。
- 可与其他常驻槽位一起拖动排序。

### 5.4 AI 会话页

- 标题：`AI` + `会话`。
- 顶部：`2 个会话 · 1 个等待你`。
- 列表只显示运行/等待；每行：
  - `项目名 · 标题`，右侧状态点。
  - 状态 + 持续时间 + 当前工具/最近回复摘要。
  - 等待行有淡色底，点击进详情。
- 详情子页：
  - 审批：工具名、完整命令/路径（可展开）、`允许一次` / `拒绝` / `总是允许`（有建议时）/ `到终端处理`。
  - 提问：问题、选项（单选/多选）、说明、自由输入、`提交` / `到终端处理`。
  - 返回列表。
- 空状态：`还没有活跃会话` + `去接入 Agent`。

### 5.5 提醒

- 等待确认/等待回答：默认提示 + 自动展开到详情（可关）。
- 完成/失败：岛内横幅，约 3 秒自动收起，不需要手动关；同时播放完成/失败提示音；随后会话从列表消失。
- 横幅不拦截操作、不需要点击；受对应提醒开关控制，关掉开关才不弹。
- 可能中断：默认提示一次，黄色，随后隐藏。
- 提示沿用现有 Alert 队列；点击打开 AI 会话页并定位。
- 重复提醒：等待超过 30 秒/1/2/5 分钟可再提醒一次，默认不重复。
- 同一会话等待只提醒一次，回答后重置。
- 会话窗口在前台时不提醒（P4）。
- 用户在岛上输入/编辑时不自动展开，事件排队。

### 5.6 审批

| Agent | 允许 | 拒绝 | 总是允许 | 备注 |
|---|---|---|---|---|
| Claude Code | 是 | 是 | 是（`updatedPermissions`） | 完整命令展示 |
| Codex | 是 | 是 | 否 | 只返回 `behavior` |
| ZCode | 是 | 是 | 岛仔会话内记住 | 返回只含 `behavior`，不写 ZCode 数据库 |
| VS Code Copilot | P0 | P0 | P0 | 以验证结果为准 |

通用：

- 审批只对「已勾选监控」的 Agent 生效；未勾选的 Agent 即使 Hook 仍被调用，岛仔也立即返回空决定，不拦截、不弹窗。
- 关闭监控不自动清理 Hook（与 fly-island 一致，避免反复装卸）；需要干净配置时用设置页的显式「移除 Hook」。
- 0.5 秒内忽略点击。
- 超时（默认 90 秒）返回空决定，终端继续。
- 用户在终端先回答：活动事件撤卡，返回空决定。
- 岛仔关闭/崩溃：所有挂起请求返回空决定。

### 5.7 提问（AskUserQuestion）

仅 Claude Code（Copilot P0 验证后再定）。

- 1–4 个问题；单选/多选；选项说明；自由输入。
- 返回 `permissionDecision: "allow"` + `updatedInput.questions` + `updatedInput.answers`。
- `到终端处理` 返回空决定；随后的第二个 `PermissionRequest` 立即返回空决定。
- 超时返回空决定。

### 5.8 跳转

- 事件到达时记录终端宿主信息（`WT_SESSION`、`TERM_PROGRAM`、进程祖先、cwd、窗口标题）。
- 点击时恢复最小化窗口并切前台。
- Claude Code：终端/VS Code 窗口；Claude 桌面端 deep link 到具体会话。
- Codex：终端或 `codex://threads/<id>`。
- ZCode：按进程/窗口标题聚焦 ZCode 窗口。
- VS Code Copilot：聚焦 VS Code 窗口（按工作区/标题）。
- DSH：只聚焦主窗口。
- 找不到：显示 `没找到窗口`，不报错。
- 层级对齐 fly-island：窗口优先；标签页/终端在 P3 尽力匹配（Windows Terminal 仅在标题唯一时用 UI Automation 切标签；VS Code 用扩展定位）；不做窗格级定位。
- 不做 tmux/zellij/Herdr 窗格跳转。

### 5.9 设置项

**接入 AI 工具**

- 每个 Agent：监控勾选、审批子开关（仅勾选监控后可用）、版本/变体、状态、安装/修复、移除。
- 关闭监控或审批：Hook 保留但岛仔立即返回空决定，不自动清理、不反复装卸。
- 「移除 Hook」是显式操作，只删除岛仔自己的条目；Pebrel Hook 不在清理范围。
- 旧 Hook 清理：岛仔启动时自动执行（不等「AI 会话」启用），显示删除数量；设置页保留手动重新清理按钮；执行前备份。
- 检测到其他审批 Hook 时提示。

**提醒**

- 等待确认时提示（默认开）
- 等待回答时提示（默认开）
- 完成时提示（默认开）
- 失败时提示（默认开）
- 可能中断时提示（默认开）
- 自动展开到详情（默认开）
- 重复提醒：不提醒 / 30 秒 / 1 分钟 / 5 分钟（默认不提醒）

**显示**

- 常驻胶囊显示状态（默认开）在「应用 → 岛体 → 岛上显示」中管理。
- 会话标题始终显示，不提供开关。
- **会话记录保留：数字步进器，默认 30 分钟，范围 1–60，步长 5（1 → 5 → 10 → … → 60）；不使用下拉框**
- 会话数：3 / 4 / 5 个分段按钮（默认 3），即功能页最多显示的会话行数。
- 最近消息行数：1 / 2 / 3 行（默认 1）；岛内只显示最近回复，不显示最近提问，回复中的 Markdown 在写入时转为纯文本。
- 以上展示项与自动展开、重复提醒同在「应用 → 提醒 → AI 会话消息」卡片。

**声音**

- 总开关（默认开）
- 开始 / 提交 / 审批 / 完成 / 错误 五类事件开关（默认开）
- 音量不单独做，跟随系统。

**高级**

- 服务端口（只读，自动选择）
- 重置令牌
- 查看 Hook 安装路径
- 查看诊断日志

### 5.10 提示音

- 把 `start.wav`、`submit.wav`、`approval.wav`、`complete.wav`、`error.wav` 复制到 `assets/agent-sounds/`。
- 播放用 `PlaySoundW`（`SND_MEMORY | SND_ASYNC | SND_NODEFAULT | SND_NOSTOP`），复用 `tray::play_health_sound` 的方式。
- 事件映射：
  - `SessionStart` → start
  - `UserPromptSubmit` → submit
  - `PermissionRequest` / `Question` → approval
  - `Stop` → complete
  - `StopFailure` / 错误 → error
- 删除 `play-sound.ps1` Hook；不再启动 PowerShell。
- 素材来自 CodeIsland（MIT），在 `THIRD_PARTY_NOTICES.txt` 补一行。

---

## 6. 技术架构

### 6.1 总体

```
Agent (Claude/Codex/ZCode/Copilot) ──hook──> daozai.exe --agent-hook <agent> --event <event>
                                                   │ 读取 agent-bridge.json（端口+令牌）
                                                   ▼
                                        岛仔 loopback HTTP 服务
                                                   │ mpsc
                                                   ▼
                        AgentRegistry → Normalizer → SessionStore/StateMachine → UI
                                                   │
                                                   └─ 阻塞请求：保留 Sender，回答后写回

DSH ──无 Hook──> FileWatcher（zstd 日志 + 投影缓存）──> 同一 SessionStore
```

### 6.2 模块化 Agent 注册表（核心）

一个 `AgentDescriptor` 描述表，所有 Agent 共用同一套安装器、状态机、页面：

```rust
struct AgentDescriptor {
    id: &'static str,              // "claude" | "codex" | "zcode" | "copilot" | "dsh"
    display: &'static str,
    icon: Icon,
    detect: DetectSpec,            // exe 名、配置路径、版本命令、变体判断
    hook: Option<HookSpec>,        // None = 文件监听型（DSH）
    capabilities: Capabilities,    // approve / question / focus / discovery
    normalize: fn(&RawHook) -> Option<AgentEvent>,
    discover: Option<fn(&AppCtx) -> Vec<DiscoveredSession>>,
    focus: FocusStrategy,
    cleanup: &'static [CleanupMarker], // 旧 Hook 标记（Fly Island/提示音）
}
```

`HookSpec`：

```rust
struct HookSpec {
    format: HookFormat,            // ClaudeJson | EventMapJson | ZCodeJson | CopilotJson
    config_path: PathBuf,
    events: &'static [HookEvent],  // 每个事件：名称、是否阻塞、超时、匹配器
    command: CommandTemplate,      // <exe> --agent-hook <id> --event <event>
}
```

`HookFormat` 只做三件小事：

- `ClaudeJson`：`hooks.<Event>[].hooks[]`，可带 matcher。
- `EventMapJson`：`hooks.<Event>[]`（Codex/Copilot 的 JSON 变体）。
- `ZCodeJson`：`hooks.enabled` + `hooks.events.<Event>[]`，严格 7 事件白名单。

新增 Agent 的步骤：

1. 在注册表加一个 `AgentDescriptor`。
2. 写一个 `normalize` 函数（把原始 payload 转成统一的 `AgentEvent`）。
3. 如果需要文件发现/特殊跳转，再加 `discover` / `focus`。
4. 加一份 fixture 测试。
5. UI、安装器、状态机不改。

### 6.3 统一事件模型

```rust
enum AgentEventKind {
    SessionStart, SessionEnd,
    TurnStart, TurnEnd,
    PermissionRequest { request: ApprovalRequest },
    Question { questions: Vec<Question> },
    ToolStart { tool: String }, ToolEnd { tool: String, ok: bool },
    Message { text: String },
    Error { message: String },
    Heartbeat,
}
```

会话状态机只消费 `AgentEvent`，不关心来自哪个 Agent。

### 6.4 Hook 安装器

- 读取配置 JSON；解析失败 → 拒绝写入。
- 备份：`%APPDATA%\Daozai\agent-hook-backups\<file>.<timestamp>.bak`，仅在内容将变化时备份。
- 合并：只操作岛仔条目；用显式 `id`（如 `"daozai-claude"`）识别；不按宽泛字符串删除。
- 原子写入：复用 `model::atomic_write`。
- 幂等：重复安装结果一致；旧版本条目按 `id` 替换。
- 卸载：只删岛仔条目；空事件数组/空 `hooks` 容器按格式清理。
- 健康检查：扫描实际配置，校验 `id`、事件列表、命令路径；不只看缓存。
- 版本门控：Agent 版本不支持时禁用交互，保留只读或提示升级。
- 监控/审批开关只改变岛仔端处理（未启用 → 立即返回空决定），不写配置；只有显式「移除」才删除条目。

### 6.5 传输

- 中继：`daozai.exe --agent-hook <agent> --event <event>`。
- 在单实例检查之前处理；岛仔未运行时 300ms 内退出 0。
- 读取 `agent-bridge.json`（端口 + 令牌 + pid）；连接失败立即退出。
- 非阻塞事件：POST 后退出 0；Claude Code 可标记 `async: true`。
- 阻塞事件：等待响应，最长 90 秒；超时/错误打印 `{}` 并退出 0。
- 服务：`TcpListener` 绑定 `127.0.0.1:0`，随机端口；令牌校验；请求体上限 256 KB。
- 每个阻塞请求独立线程等待；UI 回写决定；岛仔退出时全部返回空决定。
- 备选：Windows 命名管道（CodeIsland Windows 的做法）。P0 若发现端口/防火墙问题再切换；接口不变。

### 6.6 各 Agent 适配要点

**Claude Code**

- 配置：`~/.claude/settings.json`。
- 事件：SessionStart、UserPromptSubmit、PreToolUse（AskUserQuestion）、PermissionRequest、Notification、PostToolUse、PostToolUseFailure、PermissionDenied、Stop、StopFailure、SubagentStop、PreCompact、PostCompact、SessionEnd。
- 审批输出：`hookSpecificOutput.decision.behavior`；`allow` 可带 `updatedPermissions`。
- 提问输出：`permissionDecision: "allow"` + `updatedInput.answers`。
- 2.1.319+ 支持终端先回答后撤卡；本机 2.1.323。

**Codex**

- 配置：`~/.codex/hooks.json`（新版）或 `config.toml`（旧版）。
- 事件：SessionStart、UserPromptSubmit、PreToolUse、PermissionRequest、PostToolUse、Stop、SessionEnd。
- 审批只返回 `decision.behavior`，不支持 `updatedPermissions`。
- 无提问回答；只能提醒 + 跳转。
- 首次需在 Codex 中 `/hooks` 信任。

**ZCode**

- 配置：`~/.zcode/cli/config.json` 的 `hooks`。
- 必须写 `hooks.enabled = true`；事件严格 7 个。
- 用 `type: "process"` + `command` + `args` + `timeoutMs`，避免 shell 转义。
- `PermissionRequest` 超时设大（如 86400000 ms），实际由中继 90 秒兜底。
- 审批返回只含 `behavior`；「总是允许」由岛仔按会话记住。
- 无提问回答。

**VS Code Copilot**

- 本机验证（VS Code 1.141，2026-11-19 读取安装包内实现）：
  - Hook 目录：用户级 `~/.copilot/hooks/*.json`（Copilot 格式：`hooks.<Event>[]`，条目支持 `command` / `windows` / `timeout`）；工作区 `.github/hooks`。
  - 同时会读取 `~/.claude/settings.json`（Claude 格式，受 `chat.useClaudeHooks` 控制）。
  - 事件白名单：SessionStart、SessionEnd、UserPromptSubmit、PreToolUse、PostToolUse、PreCompact、SubagentStart、SubagentStop、Stop、ErrorOccurred；**没有 PermissionRequest / 提问事件**，只能监控与提醒。
  - 需要开启预览设置 `chat.useHooks`（默认关）且工作区受信任。
- 岛仔写入 `~/.copilot/hooks/daozai.json`，不碰 `~/.claude`；安装提示中说明需开启 `chat.useHooks`。
- 跳转：VS Code 窗口按工作区/标题匹配。

**DSH**

- 无 Hook；文件监听。
- 只解析四种日志事件 + 投影缓存；只读状态。
- 状态：`turn/start` → 运行；未决 `approval/asked` 或 `questions.active` → 等待；`turn/end` → 完成；进程不存在 → 已退出。
- 不做审批/回答；跳转只聚焦主窗口。
- 本机未安装：先按上述格式实现并标记「未验证」，安装后补测。

### 6.7 Hook 清理

一次性迁移：岛仔启动时执行，不等「AI 会话」启用；设置页保留手动按钮。

- Claude Code：删除 `focusd-agent-*`、`curl.exe` 活动 Hook、`127.0.0.1:47830` HTTP Hook、`play-sound.ps1`。
- Codex：删除 `hooks.json` 里的 `play-sound.ps1`；另按标记块删除 `config.toml` 中 `# BEGIN/END FocuSD Agent Status Hooks` 之间的旧条目（2026-11-19 发现残留 14 处，已清理并备份）。保留 rtk、Pebrel。
- Pebrel：保留，不在清理范围。
- ZCode/Copilot：无旧条目。
- 清理前备份；清理后重新扫描；显示删除数量。
- 若 Fly Island 仍在运行并自动修复，提示用户关闭它的 Hook 功能或退出。
- 清理完成后，岛仔成为唯一审批方；审批只对勾选监控的 Agent 开启，其余返回空决定。

### 6.8 与现有代码的接入点

- 新模块：
  - `src/agent/`：注册表、描述表、归一化、安装器、状态机、会话存储、中继、文件监听。
  - `src/agent_ui.rs`：页面、详情卡片、常驻槽绘制。
  - `src/agent_sound.rs`：WAV 播放。
- `src/main.rs`：`--agent-hook` 提前处理；启动 loopback 服务；每帧轮询事件。
- `src/app.rs`：
  - `PAGE_NAMES` 加 `"AI 会话"`；`PAGE_SETTINGS_TABS` 加新设置页。
  - `visible_pages` / `page_order` 从 9 扩到 10，旧配置迁移（第 10 页默认开启）。
  - `render_page` 加分支；`expanded_height` 加新页高度（约 208）。
- `src/bar_layout.rs`：新增 `AGENT` 槽位；扩展 `WIDTHS` / `DEFAULT_ORDER`；更新测试。
- `src/resident_settings.rs`：槽位标签/预览。
- `src/extras.rs`：`AI 会话` 提醒类型/图标。
- `src/model.rs`：第 10 页迁移；AI 会话配置放独立 `ai-sessions.json`。
- 设置窗口：`SETTINGS_GROUPS` 加「AI 会话」，序号 15。

### 6.9 安全与隐私

- 只监听 `127.0.0.1`；令牌随机 32 字节，用户配置目录。
- 不把 prompt/命令/文件路径写入日志或磁盘；会话只在内存。
- 审批展示完整命令/路径；「总是允许」默认会话级。
- Agent 自身 deny 规则优先。
- 岛仔关闭/崩溃/超时返回空决定。
- 不修改其他工具 Hook；清理仅删除明确列出的旧条目。

### 6.10 失败与降级

| 情况 | 行为 |
|---|---|
| 岛仔未运行 | 中继立即退出，终端原生审批 |
| 端口被占用 | 换端口并写 bridge 文件 |
| 配置损坏/无权限 | 拒绝写入，显示路径，不破坏原文件 |
| Agent 版本不支持 | 显示「版本不支持」，禁用交互 |
| Codex 未信任 Hook | 显示「等待信任」，提示 `/hooks` |
| 终端先回答 | 活动事件撤卡，返回空决定 |
| 岛上超时 | 返回空决定 |
| 窗口找不到 | 行尾提示，不报错 |
| Fly Island 写回 Hook | 设置页提示，建议关闭其 Hook 功能 |

---

## 7. 交互与视觉

### 7.1 状态灯建议

**要状态语义，不要独立状态灯；不抄 CodeIsland 吉祥物。**

- 常驻槽位：Agent 图标 + 5px 状态点 + 会话数。
- 颜色：运行=蓝、等待=橙（呼吸）、失败=红（短暂）、可能中断=黄。
- 完成/闲置不显示，与「不再活跃」一致。
- 不做脱离内容的单独灯。
- 不做每工具像素吉祥物：美术成本高、与岛仔吉祥物冲突、不提升主链路。
- 可选 P4：岛仔本体对等待/完成做轻微表情反应。

### 7.2 颜色与文案

| 状态 | 颜色 | 文案 |
|---|---|---|
| 运行中 | 蓝 | `运行中 · 2 分钟` |
| 等待确认 | 橙 | `等待确认 · 点击处理` |
| 等待回答 | 橙 | `等你回答 · 点击处理` |
| 可能中断 | 黄 | `可能已中断` |
| 失败 | 红 | `失败 · 点击查看` |

- 不只靠颜色：同时有文字/图标。
- 完成/闲置不显示，只弹一次横幅提醒；完成/失败同时有提示音。

### 7.3 页面布局

```
┌──────────────────────────────────────────┐
│ AI 会话                    2 个 · 1 等待  │
│ ┌──────────────────────────────────────┐ │
│ │ [C] 项目A · 修复登录接口       ● 橙  │ │
│ │     等待确认 · 12 秒 · Bash          │ │
│ ├──────────────────────────────────────┤ │
│ │ [Z] 项目B · 重构设置页         ● 蓝  │ │
│ │     运行中 · 3 分钟 · 编辑文件       │ │
│ └──────────────────────────────────────┘ │
│              ● ● ● （页码豆）             │
└──────────────────────────────────────────┘
```

---

## 8. 数据与配置

`%APPDATA%\Daozai\ai-sessions.json`：

```json
{
  "enabled": true,
  "agents": {
    "claude": { "monitor": true, "approval": true },
    "codex": { "monitor": true, "approval": true },
    "zcode": { "monitor": true, "approval": true },
    "copilot": { "monitor": false, "approval": false },
    "dsh": { "monitor": false, "approval": false }
  },
  "retention_minutes": 30,
  "display": { "bar_indicator": true, "show_title": true, "rows": 3 },
  "notify": {
    "waiting": true, "done": true, "failed": true, "stale": true,
    "auto_expand": true, "repeat": "never"
  },
  "sound": {
    "enabled": true,
    "start": true, "submit": true, "approval": true, "complete": true, "error": true
  },
  "server": { "port": 0, "token": "..." }
}
```

- 审批按 Agent 单独开关，且必须 `monitor = true` 才可用；关闭时中继返回空决定。
- `port`/`token` 实际写入 `agent-bridge.json`。
- 会话只在内存，不落盘。
- 页面可见性/顺序在主 `Settings`，第 10 页迁移。
- Hook 安装状态以实际扫描为准。

---

## 9. 里程碑与估时

| 阶段 | 内容 | 估时 |
|---|---|---|
| P0 | 5 Agent Hook/文件格式实测（DSH 延后）；Copilot 注入位置；阻塞/超时/终端先答/窗口定位 | 2–3 天 |
| P1 | 注册表、安装器、中继、状态机、页面、常驻槽、提醒、跳转（Claude/Codex/ZCode） | 3–4 天 |
| P2 | 审批（Claude/Codex/ZCode）、旧 Hook 清理、岛仔提示音 | 2–3 天 |
| P3 | Claude 提问、VS Code Copilot 适配、DSH 只读监听、标签页/终端尽力匹配 | 2–3 天 |
| P4 | 标题、已读未读、免打扰、测试与文档 | 1–2 天 |

合计约 10–15 个工作日；P0 结果可能调整范围。

---

## 10. 验收标准

1. 5 个 Agent 都能被检测；未安装/未接入时显示明确状态，不报错。
2. 勾选并安装后，启动会话 2 秒内出现在列表。
3. 状态流转正确：运行 → 等待 → 运行 → 完成/失败。
4. 完成、关闭、闲置的会话从岛上消失；完成/失败在消失前弹一次横幅（约 3 秒自动收起）并播放提示音；记录保留 30 分钟（可改），到点清理。
5. 保留时间设置是数字步进器，不是下拉框；范围 1–60，默认 30，步长 5 分钟。
6. Claude Code 权限请求 1 秒内出现；允许/拒绝生效；超时回退终端；未勾选监控的 Agent 立即返回空决定，不弹窗。
7. Claude Code `AskUserQuestion` 单选/多选/自由输入可用。
8. ZCode 审批只返回 `behavior`；不写 ZCode 数据库；「总是允许」只在岛仔会话内生效。
9. 点击会话能切到对应窗口；标签页/终端尽力匹配；找不到时提示，不崩溃。
10. 关闭岛仔后 Agent 审批仍在终端正常出现，不卡住。
11. 清理旧 Hook 后，Claude Code/Codex 配置里不再有 Fly Island/`play-sound.ps1` 条目，RTK 和 Pebrel 保留；清理在启动时自动执行，清理前有备份。
12. 提示音由岛仔播放，事件映射正确；不再调用 PowerShell。
13. 新增一个 Agent 只需加描述 + 归一化 + 测试，不改 UI/状态机（代码评审检查项）。
14. AI 额度功能无回归；旧设置迁移不丢页面顺序/可见性。
15. 不把 prompt/命令写入日志；会话数据不落盘。

---

## 11. 风险与开放问题

### 风险

- **VS Code Copilot Hook 位置/格式未定**：P0 必须验证；可能只能做只读或 Copilot CLI。
- **ZCode 严格 schema**：事件名写错会丢整个 hooks 配置；审批输出不能带 `updatedPermissions`。
- **DSH 无 Hook**：只能只读；先实现后验证，文件格式可能随版本变化。
- **Codex 无提问 Hook**：只能提醒 + 跳转。
- **Fly Island 写回**：清理后若它仍运行并自动修复，会再次写回。
- **保留 Hook 的常驻开销**：关闭监控后 Hook 仍被调用，中继必须在 300ms 内退出，不能拖慢 Agent 启动。
- **第 10 页迁移**：数组、页面顺序、测试都要改；需要迁移测试。
- **UI 空间**：419×208，4 个问题/多选项会拥挤；用滚动详情子页。

### 已确认（2026-11-19）

1. 保留范围：1–60 分钟，默认 30，步长 5。
2. 完成/失败：岛内横幅，自动收起，不需要手动关；配合提示音。
3. 清理时机：现在清理；已备份到 `%APPDATA%\Daozai\agent-hook-backups`。
4. DSH：先按文件格式实现，安装后补测。
