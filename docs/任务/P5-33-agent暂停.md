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

## 完成记录

做了什么：
- `corral.rs`：`Agent` 读 `paused`、`paused_at`；旧版 corral 没有这两个字段就当作没暂停。
- `agents.rs`：`Status` 加 `Paused`；新 `Panel::shown` 只给左侧栏卡片和排序用（出错、退出照旧显示 Error/Exited），Kanban、Attention 还读 `Panel::status`，没变。
- `card.rs`：暂停的卡片预览写 `Paused`（代替原来的状态字、工具和标题，头像角上的转圈换成暗点），时间算暂停了多久，详情里 Status 是 Paused、多一格 “Paused for”；组状态条排在 idle 后面。
- 新 `pause.rs`：`every`（一键：有没暂停的就暂停这些，全部已暂停就继续全部，已退出的不算）、`question`（working 或 state 认不出的列名字确认）、`run`（逐个调 `corral pause/resume`，每个最多 30 秒）、`doing`/`summary`（底部提示；`unsupported` 写成 `needs corral upgrade`，其他错误码照原样写）。
- `sidebar.rs`：头部铃铛左边一键按钮（`Pause all agents`/`Resume all agents`），窄条里放在铃铛下面；卡片详情里 Copy 和 Stop 之间（右边 Stop 旁）加 Pause/Resume（`Pause agent`/`Resume agent`），干活中点 Pause 在卡片里问 `Working — pause anyway?`（Cancel/Pause，鼠标离开卡片就收回）；暂停的卡片和窄条格子整体变淡；菜单项文字 `pause_item`；底部提示被截短时悬停显示全文。
- `window.rs`：接 Pause 事件，后台调 corral，要确认时用系统对话框（Pause/Cancel），开始时提示 `Pausing …`，做完提示结果并立刻刷新列表；左下角菜单 Stop 上面加 `Pause {name}`/`Resume {name}`；每次列表刷新和 attach 时告诉各窗格它的 agent 暂没暂停；暂停的窗格在 58% 高处浮一块 `Paused` 和 `Resume`。
- `viewer.rs`（From Saddle，头注写了改动）：加 `paused` 和 `send`，暂停时丢掉输入；换别的 agent、开 shell、关掉时清掉。`view.rs`：打字、粘贴、拖文件、输入法、鼠标上报、滚轮转方向键都走这一个口；暂停时视图不跳到底，鼠标改成本地选择，滚轮滚本地历史。
- `footer_icon.rs`/`motion.rs`：Pause（两条竖线）、Resume（三角）图标；Pause 悬停时两条线一起往下按 1.5 点再回来（320ms），Resume 沿用 P5-27 的 shift 往右滑一下；减弱动态效果时不动。

验证了什么：
- 测试（假 corral 脚本，没碰真实 agent）：`tests/corral.rs` 读到 `paused`，旧版没有字段当作没暂停；`card.rs` 暂停卡片的字、时间、排序、状态条，同时 `Panel::status` 仍是 Working；`pause.rs` 单元测试（一键的方向和对象、确认的有无和措辞）；`tests/pause.rs`：名字全都发到、部分失败列出名字和原因（needs corral upgrade）、全部失败、corral 起不来、从假 corral 的列表算一键：没有 working 不问、有 working 问且全部名字都在；`tests/viewer.rs` 用假 `corral attach`：暂停时不发输入，继续后只收到继续后打的字（暂停时的没攒着），换 agent/开 shell/关闭后不再暂停；`motion.rs` 测 dip 和减弱动态效果。
- 在 `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 全过。
- 截图（release、`PADDOCK_NO_ACTIVATE=1`、临时 HOME/`CFFIXED_USER_HOME`/`XDG_CONFIG_HOME`/`XDG_STATE_HOME`、shell 写的假 corral 四个 agent），只截自己的窗口，按 PID 停掉，自己看过：a 有的暂停有的没（头部暂停图标、两张 Paused 卡片变淡排在干活的后面、窗格浮层）；b 全部暂停、界面字号 18、最窄侧栏 318（头部换成继续图标，不挤到标题和铃铛）；c 窄条（铃铛下面的按钮、变淡的格子、窗格浮层）。截图在 scratchpad，没有入库。

拿主意的地方：
- 头部多一个 28 点的按钮，侧栏能拖到的最窄宽度跟着变：基础字号 224→254，字号 18 约 318。`the_narrowest_sidebar_still_holds_the_header_row_in_the_title_bar` 照新尺寸改了断言（小字号仍更窄，10 点时仍是 220 下限）。
- 一键：已退出的不算；还有没暂停的就显示暂停，只暂停这些；全部已暂停才显示继续。没有活着的 agent 时不显示按钮。
- 确认：卡片和菜单只在 state 是 working 时问（菜单用系统对话框，写成 `1 agent is working: … Pause anyway?`）；一键照任务写的 working 或认不出的状态问，有认不出的写成 “may be working”。
- 暂停时卡片原来的预览（包括 idle 的回复）都换成 Paused；出错、退出的仍显示 Error/Exited，按钮按 `paused` 显示 Resume。
- 底部提示加了悬停全文：几个失败的名字一行放不下。
- 浮层由窗口画在终端上面，只挡自己那一块；输入的闸门放在 Viewer（它管 `corral attach`），这样能用假 corral 测。

没做的事：
- 点击、确认、悬停动效、真实冻结和继续都没实测，留给用户（没有模拟鼠标键盘，也没对任何真实 agent 暂停）。卡片的 Pause/Resume 和卡片里的确认要先点开卡片才出现，截图里没有。
- 窗格标签和窗格头上的状态点、命令面板、Attention 铃铛仍按 corral 的 state（例如暂停中的 idle agent 标签点是绿的）；活动格子图“有 agent 在干活”也仍按 state，暂停中的 working agent 会让今天那格继续呼吸。要不要改请主控定。
- Kanban、Changes、Browser、活动格子图没动；应用菜单栏没加 Pause 命令。
