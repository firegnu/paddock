# 任务：Agents 侧栏（列表、点击接入、新 shell）

2026-10-05，paddock/main 交给 paddock/dev-agents（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规，交叉审查拿不准，影响面拿不准）
类型：功能变更
依据：本轮实现 DESIGN §9 的左侧 Agents 列表，每个 agent 一行（用户选的精简版）；不做分支、强度、标题等更多信息，不做主题内容。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md` 全文，尤其是“不要干扰用户正在用的 agent”“测试不依赖真实 agent”“桌面窗口测试”
- `docs/DESIGN.md` §9 第一阶段
- `docs/任务/P1-T1-应用骨架.md` 的「要做的」第 2、4 条（窗口布局和 `Theme` 取色接口）
- Saddle 只读参考：`~/.cargo/git/checkouts/saddle-c681c520e7db068f/df1c727/src/` 下的 `corral.rs`（`Client`、`Poller`、`Agent`）、`agents.rs`（`Panel`、`Status`、`group`）、`viewer.rs`（`select_agent`、`start_shell`、`disappeared`），以及 `ui.rs` 里 Agents 面板对状态颜色的用法

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p1-agents`，分支 `p1-agents`（T1 合并后从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target`。
- 启动窗口、截图会弹权限确认，要用户手动点。需要截的图集中在一次里截完，不要零散地反复启动截图。
- 只动 `app/src/window.rs`、新建的 `app/src/sidebar.rs`，以及 `main.rs`、`lib.rs` 中接线所需的几行。如果需要从 `view.rs` 切换窗格接的对象，只在那里加最少的方法，不动绘制代码。
- 并行任务：paddock/dev-theme 在分支 `p1-theme` 改 `theme.rs`、`palette.rs`、`rows.rs` 和 `view.rs` 的绘制部分。你不要动这些文件，也不要动 `config.rs` 的结构。
- 颜色一律通过 `Theme::fg/bg` 按 Saddle 的字段名取（例如 `|t| t.agents_text`、`|t| t.agent_working`），不写死色值。这样主题任务合并后，侧栏自动跟随主题。

## 要做的

1. **列表**：用 `saddle::corral::Poller` 定时取 `corral ls`，交给 `saddle::agents::Panel` 排序和判断状态。
   - 按 `saddle::agents::group` 分组，组名单独一行。
   - 每个 agent 一行：状态色点、去掉组前缀的名字、状态文字。
   - 状态到颜色的对应照 Saddle Agents 面板的做法（`agent_working`、`agent_idle`、`agent_blocked` 等）。
   - corral 不可用或报错时，侧栏里显示一行错误说明，窗口不崩溃。
2. **以后会加信息**：用户以后想逐步加强度、会话标题、分支和改动数。
   - 每个 agent 的一行做成独立的组件；列表的滚动和点击判定不能假定每个 agent 只占固定的一行高。
   - agent 多到放不下时，侧栏可以滚动。
3. **点击接入**：点一个 agent，右侧窗格用 `Viewer::select_agent` 接入它（即 `corral attach`）。
   - 当前接入的那个用 `agent_selected` 底色标出，标题栏显示它的名字。
   - 点完后键盘输入回到终端窗格。
   - 接入的 agent 从列表里消失时，按 `Viewer::disappeared` 处理。
4. **新 shell**：侧栏底部固定一项“＋ 新 shell”，点了在右侧窗格开交互 shell，cwd 用启动时的 `--cwd`，没有就用当前目录；标题栏显示 `shell · <cwd>`。
5. **一次只接一个**：沿用 `Viewer` 现有的语义，右侧窗格一次只显示一个对象；从 shell 切到 agent 时，原来的 shell 结束。

## 怎么算做完
- 侧栏按组列出 `corral ls` 里的 agent，状态色和状态文字随 corral 刷新。
- 点 agent 接入、点“＋ 新 shell”开 shell，标题栏和选中标记都对；点完直接打字能进终端。
- 每一行是独立组件，列表不假定固定行高；放不下时能滚动；corral 出错时侧栏有说明、窗口不崩。
- 颜色全部经 `Theme::fg/bg` 取。
- 验证只做这些：
  - 列表模型（分组、排序、状态到颜色的对应、去前缀）用合成的 `Agent` 数据写针对性测试；
  - corral 出错的情况用假 `corral` 脚本测（`Client.program` 指向它），不依赖真实 agent；
  - 对 `app/Cargo.toml` 跑一次 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`；
  - 启动一次，只截本窗口，看侧栏显示。
  - 点击操作不要用脚本模拟，留给用户实际体验，在完成记录里写明。

  觉得不够，在回复里说，不要自己加。

## 不要做
- 不加分支、强度、角色、会话标题、最近工具等信息，不接 `saddle::git`。
- 不做分屏、标签、多个窗格，不做侧栏折叠和拖动宽度，不做右键菜单，不做启动或停止 agent 的按钮。
- 不 attach 到 `corral ls` 里现有的 agent，也不对它们 `corral stop/send/keys`。你的窗口会把用户所有的 agent 列出来，这没关系，但你自己不要点它们。需要真实 agent 时，自己开 `paddock/test-agents`（`corral start paddock/test-agents --cwd <临时目录> --label role=test -- claude`），用完 `corral stop`。
- 不读 Corral 的内部状态目录，只走 Saddle 对公开命令的封装。
- 不用脚本模拟系统按键或鼠标；不截全屏或屏幕区域。
- 不改 Saddle 仓库，不改 Saddle 的引用提交，不用路径依赖。不复制 Zed 的 GPL 代码。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录，一条命令能把它们全杀掉。停自己起的进程用启动时记下的 PID。
- 现有测试不删、不放宽断言。
- 遇到必须改 Saddle 或改 `Theme` 接口才能继续的情况，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p1-agents` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

2026-10-05，paddock/dev-agents。

**做了什么**
- 新建 `app/src/sidebar.rs`：`Listing` 把 `saddle::corral::Poller`（每秒，同 Saddle 默认 `refresh_ms`）的结果交给 `saddle::agents::Panel`，按 `Panel::ordered` 排序、`Panel::status` 判状态，按 `group` 输出组名行和 agent 行（去组前缀；无前缀的组照 Saddle 显示为 `agents/`）。corral 报错时在列表顶部加一行错误说明，保留上一次的列表；下一次成功读取后错误行消失。
- 每个 agent 一行是独立组件 `AgentRow`（`RenderOnce`）：状态色点、名字、状态文字；点击用各自元素的 `on_click`，列表是 `overflow_y_scroll` 的弹性布局，不假定行高，放不下时滚动。底部固定“＋ 新 shell”。
- 颜色全部经 `Theme::fg/bg` 按 Saddle 字段名取。
- `view.rs` 只加了切换方法，不动绘制：`target`、`attach`（`shell = None` 后 `Viewer::select_agent`，元数据带 agent 的 cwd 和 instance，同 Saddle 的 `attach_at`）、`start_shell`（`Viewer::close` 结束当前会话，等 `Viewer::closed()` 后再 `start_shell`，避免旧会话退出把新 shell 标成 exited）、`disappeared`（转给 `Viewer::disappeared`，传入未 exited 的 agent 名，同 Saddle）。`Options` 加 `corral` 字段，`Launch::Agent` 去掉其中重复的 `corral`；原来 shell 启动时 `Viewer` 固定用 `"corral"`，现在跟随 `--corral`。
- `window.rs` 把占位换成侧栏；`main.rs` 把 `$SHELL` 和 `--cwd`（缺省当前目录）交给侧栏的“新 shell”；`lib.rs` 加模块。
- 选中标记：`TerminalView::target()`（正在接入或已接入的 agent）那一行用 `agent_selected` 底色；标题栏显示 agent 名或 `shell · <cwd>`。点击后把键盘焦点交回终端窗格。

**验证了什么**
- 合成 `Agent` 数据的针对性测试：分组和组内排序、去前缀、行里带的 cwd/instance、状态随每次列表刷新、8 种状态到标签和颜色的对应、error/stalled 由 Panel 判断。
- 假 `corral` 脚本（临时目录里的 sh 脚本，`Client.program` 指向它）：`ok:false` 报错、非 JSON 输出、程序不存在三种都变成错误行且保留旧列表、之后成功读取清掉错误；另一个假脚本走通 `ls`+`status` 到行。
- 对 `app/Cargo.toml`：`cargo test --all-targets` 42 项通过（新增 5 项），`cargo clippy --all-targets -- -D warnings` 通过。
- 启动一次，只截本窗口（`screencapture -l`）：侧栏按组列出 agent，色点、名字、状态文字和底部“＋ 新 shell”显示正常。截图已删除。
- 点击 agent、点“＋ 新 shell”、点完直接打字、滚动，都没有用脚本模拟，留给用户实际体验。没有 attach 任何现有 agent，也没有开 `paddock/test-agents`。

**拿主意的地方**
- 状态颜色照 Saddle Agents 面板实际的 `look()`：waiting→`agents_yellow`、error→`agents_red`、stalled→`agent_stalled`、working→`agents_blue`、starting→`agent_starting`、unknown→`agents_dim`、idle→`agents_green`、exited→`agents_faint`（名字也用 `agents_faint`）。任务书举例的 `agent_working`、`agent_idle`、`agent_blocked` 在 Saddle 面板里并不用于状态行。
- 色点画成圆点，不用 Saddle 的字符点（◐○? 等）。组名行只写组名，不带 Saddle 的计数和分隔线。
- 点 agent 时，若 agent 带 instance，Saddle 的 attach 会先检查 `attached == 0`，在别处（如 Saddle TUI）已接入的 agent 会报“agent attached elsewhere”，这是沿用 Saddle 的行为。
- `Launch::Command` 的会话被切走时在后台线程里结束（`Session` 的 drop 会等进程退出，最多 3 秒，不能卡界面线程）。

**没做的事**
- 没加分支、强度、角色、标题等信息；没做悬停效果、键盘选择、空列表提示。
- `app/README.md` 第 3 行仍写“侧栏目前是空白占位”，不在本任务可改的文件里，没改。

## 主控审查

2026-10-05，paddock/main。结论：通过，已合并（`973299d`）。
- 范围：只动了允许的文件；`view.rs` 只加切换方法，未动绘制；“不要做”各条均未触犯。
- 对照“怎么算做完”逐条达到；每行为独立组件、列表可滚动；颜色全部经 `Theme::fg/bg`。主控在独立编译目录（`.target/review`）重跑 `cargo test --all-targets` 42 项通过、clippy 无警告，与完成记录一致。
- 取舍：状态色照 Saddle 面板实际所用字段（`agents_yellow/blue/green` 等），同意；圆点代替字符点、组名不带计数，同意；沿用 Saddle“已在别处接入”的检查，同意，需告知用户。
- README 第 3 行由主控合并后改正。
- 点击、打字、滚动未实际操作，留给用户体验。
- 对方提出的共用编译目录问题属实，T2 也遇到，规矩是否调整交用户决定。
