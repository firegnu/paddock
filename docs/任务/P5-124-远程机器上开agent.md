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
