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

## 完成记录

2026-10-05，paddock/main。按用户“不用写完一个就等我回复”，自查后直接合并。

- **做了什么**：
  - `new_agent.rs`：表单规则照 Saddle——默认 Codex（`codex --yolo`）、Controller 名字固定 `main`、Regular 可命名、`--label role=…`、可选 `--prompt`、命令按 shell 规则拆分；校验和提示文字同 Saddle；`~` 展开；调用预览按 shell 规则加引号。与 Saddle 的不同：前缀默认取项目目录名（Saddle 固定 `agents`；目录名为空时仍用 `agents`），用户改过就不再跟随。`start`、`stop` 只调用公开的 `corral start`、`corral stop`（超时 120 秒，同 Saddle）。
  - `new_agent_view.rs`：New Agent 独立窗口（640×600），项目目录输入框＋Choose…（系统选文件夹对话框）＋可选目录列表（paddock 启动目录和各 agent 的目录）、Codex/Claude、Controller/Regular、前缀/名字、打开位置（当前窗格、新标签页、四个方向）、Advanced（命令、第一条消息、将要执行的调用）；Create（⌘↩）在后台运行，期间按钮变灰并显示 Starting…；失败显示原因并标红出问题的输入框，窗口和内容保留；Cancel、⌘W、红点关闭。
  - 主窗口：`seed` 给出默认目录（当前窗格 agent 的目录、shell 的目录，或启动目录）；`open_started` 按选定位置接入（当前窗格是运行中的 shell 时改开新标签页）；Stop 作用于当前窗格的 agent，系统提示框确认后在后台 `corral stop`，侧栏底部显示 Stopping…/Stopped/错误并立即刷新列表。
  - 菜单新增 Agent（New Agent… ⇧⌘N、Stop Agent…）；侧栏底部改为 ＋ Agent、＋ Shell、■ Stop（没有当前 agent 时变暗）；`+` 和 `Split ▾` 的选择框加 New agent…（预选新标签页或对应方向）。
  - 新增直接依赖 `shell-words 1`（锁文件里本来就有 1.1.1，未增加包）。
- **验证了什么**：128 项测试通过（新增：参数拼装 6 项、假 corral 的 start/stop 2 项、菜单测试更新）；clippy、fmt 通过。用临时演示代码和一个也能回答 start/stop 的假 corral（草稿目录）打开窗口截图（默认、Advanced 展开），并由演示代码触发 Create：假 corral 收到 `start paddock/main --cwd … --label role=controller -- codex --yolo`，主窗口在新标签页接入、侧栏显示 Started；接入时显示“identity changed”是因为假 corral 返回的实例号与它列表里同名 agent 不一致，属于假数据，paddock 拒绝接入是正确的。侧栏底部按钮第一次放不下，已缩短标签。演示代码已删、未提交；没有对用户的任何 agent 做操作，也没有开真实测试 agent。
- **拿主意的地方**：前缀默认取目录名；“在哪里打开”放在主区域而不是 Advanced（Saddle 放在 Advanced）；Stop 只作用于当前窗格的 agent（要停别的先点开它）；第一条消息是单行输入（输入框只支持单行）；Create 期间关窗口，agent 照样会启动，但不会自动接入，可从侧栏打开。
- **没做的事**：真实 corral 的新建、停止实测（留给用户，或需要时只开 `paddock/test-*`）；系统提示框的点击。
