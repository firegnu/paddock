# 任务：在 paddock 里给远程机器开一个 agent（简单版）

2026-10-17，paddock/main 交给 paddock/dev-remote-start（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：拿不准、拿不准、拿不准，档位倾向常规；主控按规则定：新界面加远端起 agent，不改用户配置、不动已在跑的 agent、沿用现有的远程连接和引号做法）
类型：功能变更
依据：用户 10-16：“再加一个draft task：就是remote也可以启动agent（但是功能比较简单的那种，就是能启动一个就行）”。之前 P5-122 讨论时用户说过“这个功能有必要吗？paddock只需要列出来远程机器corral托管的agent就好了”，那时没做；现在要一个简单版。10-17 主控列的五条建议用户选了“全照建议”；5090 只用 pi、omp 的规矩“不写进代码”；问入口后要“先出设计稿”，看过样稿（画布「P5-124 远程开 agent 样稿」，源文件在 `docs/设计稿/P5-124-远程开agent/`）：“照样稿做吧。做的精致一些。”
依赖：P5-123
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
优先：高
你是被委派的 agent：照本文件做，不要再开别的 agent。

派出的理由：新对话框、窗口里远程视图的入口、远端起 agent 的调用，跨好几个模块；界面实现按分工给 Claude Code。

## 先读
- `AGENTS.md`（规矩全文）。
- 本文件全部。
- 样稿：`docs/设计稿/P5-124-远程开agent/README.md` 和五个 `.dc.html`（`A-Menu`、`B-CapsuleMenu`、`C-Dialog`、`D-States`、`E-Offline`）。
- `docs/DESIGN.md` §13 里 P5-90、P5-122、P5-123 那几条（远程视图的已定做法）。
- 代码：`app/src/local_only.rs`（远程视图里哪些只在本机）、`app/src/window.rs`（`only_here`、`show_new_agent`、`open_new_agent`、`NewAgentEvent::Started` 的处理、`on_local`）、`app/src/sidebar.rs`（底部菜单、胶囊右键菜单 P5-123）、`app/src/remote.rs`（`Link::run`、`Link::corral`、`connect`、`command_line`、`quote`、`Snapshot.home`）、`app/src/new_agent.rs`（`Tool`、`Form::args`、`expand_home`、最近目录）、`app/src/new_agent_view.rs`（本机表单的样子，精简版照它的样式）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-124-remote-start`，分支 `p5-124-remote-start`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-124-remote-start`（已从 `.target/main` 克隆）。
- 动 `app/src/` 下要动的文件、相关测试、`docs/DESIGN.md` 和本文件。不加依赖。

## 要做的
1. **简单到什么程度**：只填三样——哪种 agent、目录、名字。不选模型、强度，不填第一句话，不出插件提示，不做 Controller 开关（一律 `role=regular`），不选摆在哪：开起来后照本机 New Agent 的默认摆放（当前窗格；当前窗格有在跑的 shell 就新开标签），窗格 attach 到远端这个 agent，照远程 agent 现有的 attach 做法。
2. **入口**：远程视图里 New Agent（底部菜单那一行、⇧⌘N、菜单栏、新标签面板）不再变灰、不再出 `Switch to This Mac` 提示卡，打开精简对话框；那台机器胶囊的右键菜单加一行 `New Agent on <显示名>…`（放在 `Rename…` 前面，打开同一个对话框，开在那台机器上，没显示着就先切过去）。New Shell、New Workspace 照旧只在本机。机器没连上（连接中、连不上）时 New Agent 那一行照现在变灰，右边写那台的短状态。
3. **能开哪几种**：打开对话框时经那台机器的登录 shell 查一次 `command -v claude codex pi omp`，只列找得到的；查的时候种类那行显示 `Checking…`；一个都没有时写明没找到、Start 不能点。amp、agy 简单版不列（一个要装插件、一个必须带第一句话）。5090 只用 pi、omp 的规矩不写进代码。
4. **目录**：默认远端 `$HOME`（显示成 `~`），可手填；只收绝对路径、`~`、`~/…`，别的当场提示。`~` 在远端展开（用连接时已拿到的远端 HOME，或交给远端 shell 展开，你定，但不能用本机的 HOME）。不浏览远端目录。最近用过的目录按机器分开记几个（键用机器的 `name`），对话框里能点选；和本机 New Agent 的最近目录互不混。目录不存在等错误把远端报的原文显示在对话框里，对话框不关。
5. **名字**：可留空。留空时用 `<目录最后一段>/<种类>`（目录是 `~` 时最后一段用 `home`），加 `--unique`，以 corral 返回的名字为准。手填的规则和本机一样（不空格、不以 `-` 开头，要带 `/`），手填的也加 `--unique`。
6. **怎么在远端起**：经远端登录 shell 跑 `corral start <名字> --unique --cwd <目录> --label role=regular -- <种类的命令>`（种类的命令照 `Tool::command()`），免得找不到 agent 程序（10-16 实测 pi 装在 nvm 的 node 下，非交互 ssh 找不到，要 `zsh -l` 这样的登录 shell）。登录 shell 用远端用户自己的 `$SHELL`（`-l -c`），不写死 zsh。目录、名字等用户填的字一律经现有的 `quote`/`command_line` 引起来，不能被远端 shell 当成命令。起不来时把 corral 的错显示在对话框里。起好后对话框关掉，远程侧栏刷新一次，新 agent 出现并在窗格里 attach。

### 样子：照样稿
- 五张样稿各对应上面的入口和对话框（见 README 的表）。颜色一律从主题取，样稿上的色值只是示意；控件（胶囊、输入框、按钮、菜单行、弹出层）用 paddock 现有的画法，不另起一套。
- 用户要“做的精致一些”，按这几条做：
  1. **对齐和尺寸**：标签列等宽、输入框左缘对齐、三行间距一致；尺寸、字号都走现有的界面缩放（`ui.px` 那一套），界面字号调大、调小时照样整齐；深浅两种主题都成立。
  2. **每个控件都有完整的状态**：悬停、按下、键盘焦点（焦点环）、不能点；Start 按下后变 `Starting…` 并不能再点，防重复起。
  3. **键盘走得通**：打开时焦点在 Directory；Tab／⇧Tab 在三样之间走；Agent 胶囊能用左右键换；⌘↩ 开始、Esc 取消；最近目录点一下填进 Directory。中文输入法选字时的 Return 归输入法。
  4. **状态切换不跳**：`Checking…` 换成胶囊、出错提示出现和消失时，对话框宽度不变、按钮不左右挪；长目录、长错误文字在框内换行，不撑破对话框、不被截掉。改了目录或名字，错误提示和目录框的红边就收起。

## 怎么算做完
- 用户原话：“remote也可以启动agent（但是功能比较简单的那种，就是能启动一个就行）”；10-17 选的“全照建议”，即上面「要做的」1–5；“照样稿做吧。做的精致一些。”——样子和样稿一致，并做到上面「精致」那四条。
- 验证只做这些：针对这件活写测试，覆盖远端起 agent 的命令行（登录 shell 包装；目录、名字里带空格、引号、`$`、`;` 时引对；`~`、`~/x` 按远端 HOME 展开；留空名字的默认名）、目录的校验（相对路径、空、`~user` 拒掉）、`command -v` 结果到可选种类的解析、最近目录按机器分开、远程视图里 New Agent 不再走 `Switch to This Mac` 而 New Shell/New Workspace 照旧。测试只用合成数据和假的 ssh/corral 输出，不连真实机器。`git diff --check`，`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑过一次。觉得不够，在回复里说，不要自己加。
- `docs/DESIGN.md` §13 P5-123 那条后面补一条 P5-124，写清入口、三样字段、远端怎么起；并把 P5-90/P5-122 里“New Agent 只在本机”的说法改成现在的做法。

## 不要做
- 不连远程机器（包括 5090），不跑真实 ssh；不对任何机器上 `corral ls` 里的 agent 做 `stop`、`send`、`keys`、attach；不做任何断网、断线测试。
- 不改、不读用户真实的 `~/.config/paddock/config.toml`，测试一律用临时目录；不起 paddock 窗口截图（样子留给用户看）。
- 不改本机 New Agent 的行为；不改 P5-90、P5-122、P5-123 的连接、重连、attach、只读判断、改名藏删这些行为。
- 不做远端选目录框、不列远端目录、不做 amp/agy、不加模型强度和第一句话。
- 不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西；不要按项目名或路径批量杀进程。
- 遇到要改公开约定（`corral`、`ranch` 的命令或输出）才能做的，停下来报告。
- 不合并到 main，不推送。只在 `p5-124-remote-start` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的，工具或环境和预想不一样的；没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 做了什么
- 新增 `app/src/remote_start.rs`（规则）：登录 shell 包装 `exec "${SHELL:-/bin/sh}" -l -c '<内层>'`；查种类的命令（`claude codex pi omp` 每种 `command -v`，找得到的写一行种类名）和解析；目录校验（只收绝对路径、`~`、`~/…`）；`~` 用连接时拿到的远端 `$HOME` 展开；留空名字的默认名 `<目录最后一段>/<种类>`（`~` 时 `home`）；手填名字照本机规则；`corral start <名字> --unique --cwd <目录> --label role=regular -- <种类的命令>`，内外两层都用 `remote::quote`；读 corral 的回答（成功取 corral 给的名字，失败原样给出它那行 JSON，`bad_cwd` 标在目录上）；最近目录按机器分开记 5 个（`recent-remote-dirs.json`，和本机的 `recent-projects.json` 分开）。
- 新增 `app/src/remote_agent_view.rs`（对话框，样稿 C、D）：`New Agent on` 加机器小签；Agent（`Checking…`／凹槽胶囊／没找到／查不成）、Directory（可折行的输入框、当场提示、Recent 小签）、Name（占位字是会用的名字）；出错框；Cancel、Start ⌘↩（`Starting…`，宽度固定）。键盘：开时焦点在 Directory，Tab／⇧Tab 在三样间转（没有可选种类时跳过 Agent），Agent 有焦点时 ←→ 换，⌘↩ 开始，Esc／⌘W／点暗处关。
- `local_only.rs`：New Agent 移出“只在本机”，加 `new_agent`（本机／那台）和 `new_agent_row`（那台没连上时变灰、右边写短状态）；New Shell、New Workspace 不变。
- `window.rs`：`Popup::RemoteAgent`；New Agent 各入口在远程视图打开那台的对话框（没连上时左侧栏底部写 `Can't start an agent on <机器> now: <短状态>`）；底部菜单那一行按 `new_agent_row` 画；胶囊右键 `New Agent on <显示名>…` 先切过去再开；起好后用新的 `on_seat`（`on_local` 改成调它）落在那台的那一套里，走本机 `open_started` 的摆放和 attach，并刷新那台侧栏；设置改了主题时对话框跟着换色。
- `sidebar.rs`：胶囊右键菜单最上面加那一行和分隔线（`MachineAsk::NewAgent`）；一处过时注释改了。`main.rs` 注册对话框的按键；`new_agent_view.rs` 的 `hold_main_window` 改成 `pub(crate)` 复用。
- `docs/DESIGN.md`：P5-123 后补 P5-124 一条；P5-90 远程视图、P5-122 戊里“New Agent 只在本机”的说法改成现在的做法。

### 验证了什么
- `remote_start.rs` 8 个单元测试：登录 shell 包装；目录、名字带空格、单双引号、`$`、`;`、反引号、`$(…)` 时按 POSIX 规则拆回来一字不差；`~`、`~/x` 按给定的远端 HOME 展开（不是本机的）；留空名字的默认名、手填和留空都带 `--unique`；目录拒掉空、相对路径、`./x`、`~root`、带换行；名字规则；`command -v` 输出（夹着 profile 的问候、路径、amp/agy）到可选种类；corral 的成功、`bad_cwd`、别的错、无输出、没名字；最近目录按机器分开、只留 5 个、存读一致、和本机文件互不影响。
- `app/tests/remote_start.rs` 4 个集成测试：假 ssh 把命令交给本机 `sh`，PATH 里没有 agent；`$SHELL` 指向一个假登录 shell，`-l` 时才读 `.profile` 把 nvm 样的目录加进 PATH；假 corral 在 `~/.local/bin`。验证：只列登录 shell 找得到的（claude、pi，amp 不列），普通命令找不到，登录 shell 收到的是 `-l -c`；带怪字符的目录和名字原样到 corral，profile 跑过、没有被当成命令执行（没有 `pwned`）；留空名字时问的是 `home/pi`，以 corral 给的名字为准；目录不在时错误是 corral 那行 JSON 原文、标在目录上。
- `local_only.rs`：远程视图 New Agent 是“在那台开”、菜单那一行连着时可点带快捷键、没连上时灰并写短状态；New Shell、New Workspace 照旧给 `Switch to This Mac`。
- `git diff --check`；`app/` 下 `cargo test --all-targets`（全过，lib 584 个）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑过。

### 拿主意的地方
- 查种类没用一句 `command -v claude codex pi omp`：它对 alias、函数打印的不是路径，分不清哪行是哪种；改成每种各 `command -v`，找得到就写种类名，仍是一次远端调用。
- `~` 用连接时拿到的远端 `$HOME` 展开（可测，也和窗格里写 `~` 同一个来源）。
- 起不来时显示的“原文”：有 corral 的 JSON 时只显示那一行（登录 profile 先打印的问候略过），没有时显示 stdout、stderr。目录框只在 corral 说 `bad_cwd` 时标红（样稿 D2 的例子是目录不在；别的错不标，免得误指）。
- 那台没连上时：底部菜单那一行灰；快捷键、菜单栏、面板等别的入口在左侧栏底部写一句 `Can't start an agent on <机器> now: <短状态>`（任务没说这时怎样）；胶囊右键那一行不灰，点了切过去后同样写这句。
- 摆放照本机：从底部菜单、⇧⌘N、菜单栏、胶囊右键来的是当前窗格；从新标签、分屏面板的 `Agent…` 来的照那里（新标签／那一侧），和本机 New Agent 一样；对话框里不选。
- 每次打开都是新的对话框：目录回到 `~`，种类重新查、默认第一种找得到的；按了 Start 后关掉的那个还在起时，再要 New Agent 回到它，起好照样落地。
- 目录、名字的问题当场写在框下（红字、红边）并让 Start 不能点；名字留空时框下写 `Leave empty to use <名字>.`。
- Agent 胶囊 ←→ 到两头停住，不绕回。最近目录记 5 个。
- 标题里的机器小签用 P5-122 乙的机器色，圆点用绿色（能打开时那台一定连着）。

### 没做的事
- 没起 paddock 窗口、没截图：样子、深浅主题、字号缩放、键盘和输入法留给用户实际看。界面画法没有测试（同 P5-122、P5-123）。
- 没连任何机器（包括 5090），没跑真实 ssh、corral start。
- 没做远端选目录、列目录、amp/agy、模型强度、第一句话、Controller 开关、摆放选择（任务说不做）。

### 踩过的坑
- 集成测试先用真的 `/bin/sh -l` 当远端登录 shell，它会读本机的 `/etc/profile`（path_helper 把本机的 PATH 带进来），结果“找到”了本机装的 codex；改成假登录 shell 脚本，只读假 HOME 里的 `.profile`，顺便也验证了用的是 `$SHELL -l -c`。
- 假 profile 打印的问候混进了错误原文；改成有 corral JSON 时只显示 JSON。
- 用 perl 批量替换 window.rs 时，`|` 作分隔符让转义的 `\|` 变成了“或”，把第一行文档注释里的 `row` 替换掉了；看 diff 发现后恢复成原文，其余替换核对过。

## 主控审查

2026-10-17，paddock/main。结论：可以合并。
- 看了 diff：只动了任务里的文件，「不要做」一件没碰；`remote_start.rs` 两层引号（`command_line` 再整句 `quote` 交给 `$SHELL -l -c`）、`~` 用连接时拿到的远端 HOME 展开，测试里带 `$(…)`、反引号、`;`、单双引号的目录和名字原样到达 corral；本机 New Agent 那条路径没改，New Shell、New Workspace 照旧走 `Switch to This Mac`。
- 重跑：`cargo test --all-targets` 全过（lib 584，加各集成测试，含新的 `tests/remote_start.rs`），`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 干净。
- 对方的取舍：查种类改成每种各查一次、仍是一次远端调用——同意；只在 corral 报 `bad_cwd` 时目录框标红——同意；没连上时用快捷键、菜单栏要 New Agent，侧栏写 `Can't start an agent on <机器> now: <短状态>`，胶囊右键那一行不变灰、点了切过去后也写这句——同意（任务没写，主控定）；新标签、分屏面板进来的照那里的位置摆——同意；每次打开是新对话框、还在起的那个关掉后再要会回到它——同意。
- 界面画法、深浅主题、字号缩放、键盘和输入法没有测试、没人看过，留给用户实际看。对方在 `/tmp/claude-501/p.txt` 留了一个空文件（仓库外、无害）。
