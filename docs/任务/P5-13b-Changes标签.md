# 任务：右侧栏 Changes 标签——只读地实时显示焦点窗格所在目录的 git 改动

2026-10-06，paddock/main 交给 paddock/dev-changes（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：三项都拿不准（档：常规 0.53、重 0.47，置信 0.29），按规则用常规；交叉审查拿不准，只读 git、不碰数据，定不要；影响面拿不准，定改行为）
类型：功能变更
依据：本轮只做“看”：读 git、显示改动；不做行上评论发给 agent、暂存、撤销、提交（用户 10-06：“先只做看”）。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」一节（编译目录、桌面窗口测试、`PADDOCK_NO_ACTIVATE`、不模拟按键、截图不入库）。
- `docs/DESIGN.md` §13 的原则，以及「P5-13 样稿定下的」「P5-13b Changes 标签」两条。
- 样稿：`docs/设计稿/P5-13b-Changes标签/README.md`，`Panel.dc.html`（窄面板，最要紧）、`Wide.dc.html`（加宽后）、`States.dc.html`（各种状态）、`Themes.dc.html`（三套主题）。读 HTML、内联样式和末尾脚本里的数据。
- 现在的代码：`app/src/right_panel.rs`（P5-13a 的外壳，Changes 现在是占位）、`app/src/window.rs`（`RightPanel` 的接线、`focus_pane`／`focus_active`）、`app/src/layout.rs`（`focus`）、`app/src/view.rs`（`cwd()`、`agent_metadata()`）、`app/src/git.rs`（`git()`、基准分支的认法、`Poller` 后台线程的做法）、`app/src/command.rs`、`app/src/theme.rs`、`app/src/preset.rs`、`app/src/layout_state.rs`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-13b-changes`，分支 `p5-13b-changes`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-13b-changes`。
- 可以改：`right_panel.rs`、`window.rs`（接线）、`lib.rs`（注册模块）、`layout_state.rs`（如要保存范围和布局）、`git.rs`（只加函数，不改现有行为）、`app/Cargo.toml` 和 `Cargo.lock`（只加 `syntect`、`similar` 两个依赖）；新建模块随意（如 `changes.rs`、diff 解析、高亮各一个）。

## 要做的
用户原话：“先把diff做了，这个我理解就实施实时显示所选的agent所在的目录的实时的git变化情况，对吧？”（主控答对）；“怎么显示，我认为这是一个非常成熟的方案。包括怎么展示，如何展示。现在的方案应该很成熟。要做的优雅。”；“可以，两个依赖都加，先只做看，出样稿吧”；看过样稿：“可以没问题的，派出去做吧。”

照样稿做，样稿和下面不一致的以下面为准：

1. **跟随焦点窗格**：焦点窗格是 agent 的，读它的目录（attach 时的 metadata）；是普通 shell 的，读它打开时的目录（`View::cwd()`，跟踪 shell 里后来的 `cd` 不要求）；切换焦点后很快跟上。顶部第一行：状态圆点、agent 名字（shell 显示 shell 名）、`⎇ 分支`、总增删行数。
2. **两种范围**：`Uncommitted`（已暂存＋未暂存＋未跟踪的新文件，对 HEAD）和 `Branch vs <基准分支>`（从基准分支和 HEAD 的分叉点到工作区，含未提交的）。基准分支沿用 `git.rs` 已有的认法，按钮上写实际的名字。默认 Uncommitted。当前就在基准分支上时怎么显示，你定，写进完成记录。
3. **实时**：右侧栏开着并且停在 Changes 时，在后台每一两秒读一次；收起或换到别的标签就停。读 git 不能卡界面；结果没变不重画。只用只读的 git 命令，带 `--no-optional-locks`（或 `GIT_OPTIONAL_LOCKS=0`），不能抢 agent 正在用的 index 锁。
4. **文件索引和逐个文件的 diff**：上面一份紧凑的文件索引（状态字母 M/A/D/R、目录淡色、文件名、增删数），点了跳到那个文件；下面所有文件依次排开，同一个滚动区；滚动时当前文件头贴在顶部；每个文件能折叠，顶部有“全部折叠／全部展开”。
5. **diff 正文**：hunk 头（`@@ … @@` 加函数名，淡底）；hunk 之间没改的行收成一行 “N unchanged lines”，点了展开；新旧两列行号、`+`／`−`；加删行整行淡底，行内改了的词用 `similar` 算、再深一档；`syntect` 语法高亮，颜色映射到现有主题颜色（关键字、函数、类型、字符串、数字、注释各取一个现有键，可加透明度），三套主题下都协调。长行不折行，文件内可横向滚动。
6. **加宽后**：面板宽到放得下时（阈值你定），左边换成文件树、顶部出现 Unified／Split 切换；Split 是左旧右新、删和加成对对齐。窄的时候只有 Unified。
7. **特殊文件**：改名（`R`，没改内容写 “Renamed without changes”）、删除（默认收起，“Deleted file · Show diff”）、二进制（“Binary file”，能拿到大小就写前后大小）、太大的 diff 和锁文件（默认收起，“Large diff · N lines · Show diff”，点了再渲染；阈值你定）。
8. **状态**：没有改动（“No uncommitted changes”／范围对应的说法）、不在 git 仓库里（“Not a git repository”）、git 出错（“Couldn’t read changes” 加一行原因和 Retry）、没有焦点窗格。都是安静的居中空状态：一个淡色图标、一行标题、一行淡色说明。
9. **大改动不卡**：几千行的 diff 滚动要顺；高亮和改词只算看得到的或先算一部分，怎么做你定（GPUI 的 `uniform_list`／`list` 之类），写进完成记录。
10. **优雅**（照 DESIGN §13 的视觉规则）：只从主题取色、可加透明度，不新增主题颜色键；字号用 `ui.px`；三套预置主题、界面字号 13 和 18 下都好看，顶部两行不折行；代码用终端字体；悬停、选中的样子和左侧栏一致。范围、Unified／Split 随布局保存（照 P5-13a 的 `Saved` 加 serde 默认值）。

## 怎么算做完
- 用户原话：实时显示所选 agent 所在目录的 git 变化情况，做得优雅，和样稿一致（用户认可了样稿）。上面十条达到。
- 验证只做这些：
  - 测试：git 输出解析成数据（普通修改、新增、未跟踪、删除、改名、二进制）、改词配对、两种范围用的命令在临时 git 仓库里读出的结果、`Saved` 旧布局文件读入默认值。
  - 在 `app/` 下跑一次 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`。
  - 用 `PADDOCK_NO_ACTIVATE=1`、临时 HOME、假 corral 和一个临时 git 仓库起自己的窗口截几张（窄面板 13 和 18、加宽 Split、空状态；截图不入库，放 scratchpad）。`screencapture` 截不到就在完成记录里说，不要想别的办法。
  - 点击、滚动、折叠、悬停、真实 agent 下的效果留给用户试，写进完成记录。觉得不够，在回复里说，不要自己加。

## 不要做
- 不做暂存、撤销、提交、行上评论、发给 agent；不写任何 git 仓库（只用只读命令，不用 `git add -N` 这类会改 index 的办法列未跟踪文件）。
- 不嵌网页，不碰 Browser 标签和 Kanban。不改左侧栏、弹出框、palette。
- 依赖只加 `syntect` 和 `similar`；`syntect` 选纯 Rust 的正则后端（`regex-fancy`），关掉不需要的默认功能，不引入要 C 编译的库。做不到就停下来报告，等决定。
- 不新增主题颜色键，不改 `gpui-pre-*` 的版本。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置和布局文件。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。不读 corral 的内部状态目录。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-13b-changes` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
