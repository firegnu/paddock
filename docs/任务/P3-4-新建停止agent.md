# 任务 P3-4：新建、停止 agent

2026-10-05，paddock/main 自己做。
类型：功能变更
依据：DESIGN §12 第 4 项。照 Saddle（`df1c727` 的 `src/launch.rs` 和 README “New agents”、Controls 的 Stop）。用户 10-05 同意算在这一批（只调用公开的 `corral start`、`corral stop`）；新建用独立窗口、确认用系统提示框、选目录用系统对话框（DESIGN §12「窗口与对话框按 macOS 做」）；不逐件等回复。

## 要做的

1. **新建 agent 窗口**（菜单 Agent → New Agent… ⇧⌘N、侧栏底部 New agent、`+` 和 `Split ▾` 的选择框里的 New agent…；已开着就提到最前）。信息照 Saddle：
   - 项目目录：输入框，加 Choose…（系统选文件夹对话框），下面列出可选的目录（paddock 启动目录和现有 agent 的工作目录，去重）。默认取当前窗格的目录。
   - Codex / Claude（默认 Codex，命令 `codex --yolo`；Claude 为 `claude`）。
   - Controller / Regular（`--label role=controller|regular`；Controller 名字固定 `main`，Regular 可改）。
   - 名字前缀（照 Saddle 拼成 `前缀/名字`）：默认取项目目录名，用户改过就不再跟着变。
   - Advanced 展开后：完整命令（可改）、第一条消息（可选）、在哪里打开（当前窗格、新标签页、左右上下分屏；从选择框打开时预选对应位置）、将要执行的 `corral start …` 调用预览。
   - Create（⌘↩）：后台调用 `corral start`；成功后关窗口，在选定位置接入新 agent；失败时窗口不关、填的内容保留、显示原因。校验照 Saddle（目录必填，名字和前缀不能有空格或以 `-` 开头，前缀不含 `/`，命令引号要闭合）。
   - 当前窗格是运行中的 shell 时不替换它，改为新标签页（paddock 已有的规则）。
2. **停止 agent**：菜单 Agent → Stop Agent…、侧栏底部 Stop，作用于当前窗格显示的 agent；系统提示框确认后后台调用 `corral stop`，结果显示在侧栏。没有选中 agent 时不可用或给出说明。
3. 菜单新增 Agent（New Agent…、Stop Agent…）。

## 怎么算做完

- 照 Saddle：信息不少于 Saddle 的新建表单；只调用公开的 `corral start`、`corral stop`。
- 新增测试：参数拼装与校验（默认 Codex Controller、Claude、Regular 名字、前缀建议与改过后不跟随、第一条消息、命令引号）、调用预览；用假 corral 脚本验证 start 结果解析和 stop 调用。现有测试照样通过；clippy 无警告。
- 截图留到这一批做完一并给用户看；实际新建、停止的操作留给用户体验。开发中不对用户的 agent 做任何操作；如需真实实测只开 `paddock/test-*`，用完停掉。

## 不要做

- 不读 corral 内部状态；不碰用户现有的 agent。
- 不做 Saddle 的 `saddle ctl` 开 agent。
- 不改 Saddle 仓库。
