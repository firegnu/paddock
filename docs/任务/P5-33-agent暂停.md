# 任务：agent 暂停（冻结）——卡片和窗格显示 Paused，单个和一键 Pause／Resume

2026-10-07 起草，paddock/main 交给 paddock/dev-pause（Claude Code，重：opus[1m] / xhigh）。
路由：重 / 交叉审查不要 / 影响面：改行为（路由：重 0.94；交叉审查拿不准（并发 0.76），冻结本身在 corral 里做、那边交叉审查，这边只是调用和显示，定不要；影响面拿不准，定改行为）
类型：功能变更
依据：本轮只在 paddock 界面接上 corral 的 `pause`／`resume`，显示暂停状态；暂停本身由 corral 做（ranch `docs/任务/R1-corral暂停与继续.md`），这边不发信号、不碰进程。
依赖：ranch R1（corral 新版装好、在运行的 agent 已 `corral upgrade`）
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」。
- `docs/DESIGN.md` §13 最后一条「agent 暂停（冻结）」。
- `corral guide` 里 `pause`、`resume`、`paused` 字段和错误码 `paused`／`unsupported`。
- 代码：`app/src/corral.rs`（`collect` 读 `ls` 和 `status`）、`app/src/agents.rs`（agent 的状态）、`app/src/sidebar.rs`（卡片和 Copy／Stop 图标按钮、`SidebarEvent::Stop`、左下角菜单的 `Stop {name}…`）、`app/src/window.rs`（菜单、Stop 的确认和底部提示的接线）、终端窗格视图（attach 和键盘输入）、`app/src/motion.rs`／`footer_icon.rs`（图标悬停动效）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-33-pause`，分支 `p5-33-pause`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-33-pause`。

## 要做的
用户 10-07：“agent能否有一个pause的功能。pause就是暂停而不是停止……不让其对外有连接或者之类的行为”，选了“冻结”；用法：“一般的暂停，我都会等全部任务都完成之后，打一个结了之后才会暂停……有的时候网络不好的时候或者晚上我要下班了不再开发了，我不想一个个agent关掉”。一键暂停管左侧栏里的全部 agent（用户：“全部”），包括主控自己（用户：“同意”）。

1. **显示**：`paused` 为 true 的 agent，卡片显示 `Paused`（代替原来的状态字和转圈，整张卡片变淡），排序按状态时排在 idle 后面；Kanban 照原样，不改。
2. **单个**：卡片的图标按钮在 Copy、Stop 之间加一个 Pause（暂停时换成 Resume），悬停说明 `Pause agent`／`Resume agent`，不用确认；agent 在干活时点 Pause 先就地确认（`Working — pause anyway?`）。左下角菜单里当前 agent 那一项旁边加 `Pause {name}`／`Resume {name}`。
3. **一键**（用户 10-07 选 B）：侧栏头部、铃铛左边加一个暂停图标（悬停说明 `Pause all agents`）；全部 agent 都已暂停时换成继续图标（`Resume all agents`）；有的暂停有的没暂停时显示暂停图标。窄条模式和界面字号 18 时不挤到铃铛和标题。`Pause All Agents`：先看有没有 `state` 是 working 或认不出的，有就列出名字确认（`2 agents are working: … Pause anyway?`），没有就直接全部暂停；`Resume All Agents` 直接全部继续。结果在底部提示里说（`Paused 5 agents`），有失败的列出名字和原因（`unsupported` 写成 `needs corral upgrade`）。
4. **暂停的窗格**：终端画面保留，不接受键盘输入（不往 corral 发，也不在本地攒着），窗格中间偏下浮一块 `Paused` 和 `Resume` 按钮；继续后浮层消失、输入照常。
5. 暂停、继续在后台调 corral，不卡界面；做完立刻刷新一次 agent 列表。图标的悬停动效照 P5-26 的风格自己定一个（例如两条竖线一起往下按一下），尊重“减弱动态效果”。

## 怎么算做完
- 上面五条达到；界面文字英文，只从主题取色、不新增主题颜色键。
- 验证：
  - 测试（假 corral 脚本，不用真实 agent）：读到 `paused`；旧版 corral 没有这个字段时当作没暂停；Pause all 有 working 的要确认、没有的不确认、全部名字都发到；部分失败时提示列出名字；暂停的窗格不发输入。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 截图：一张左侧栏有暂停卡片、一张暂停的窗格（release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`、假 corral），自己看过。截图放 scratchpad，不入库。
  - 点击、确认、真实冻结和继续留给用户实际用，写进完成记录。觉得不够，在回复里说，不要自己加。

## 不要做
- 不自己发信号、不碰 agent 进程；不改 corral／ranch，不读 corral 的内部状态目录。
- 不对 `corral ls` 里的 agent pause、resume、stop、send、keys，也不 attach 上去打字；只用假 corral 测。
- 不改 Kanban、Changes、Browser、活动格子图；不新增主题颜色键，不加依赖。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件和正在运行的 paddock。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-33-pause` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
