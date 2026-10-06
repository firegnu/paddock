# 任务：修 Changes 标签截图里看到的五处显示问题

2026-10-06，paddock/main 交给 paddock/dev-changes-fix（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（0.91，置信 0.86）；交叉审查不要；影响面拿不准（看得见 0.58），其中左右对照的配对是行为，定为改行为）
类型：Bug 修复
依据：本轮只修 P5-13b 截图里看到的五处；不加新功能，不改读 git 的部分。
提示：依据证据定位并修复导致问题的原因，保持无关行为不变。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」一节（编译目录、桌面窗口测试、`PADDOCK_NO_ACTIVATE`、截图不入库）。
- `docs/任务/P5-13b-Changes标签.md`：「要做的」和「完成记录」，知道这个标签是怎么做的。
- 样稿：`docs/设计稿/P5-13b-Changes标签/Panel.dc.html`（窄面板，整行铺满的样子）、`Wide.dc.html`（左右对照：两栏各占一半、中线笔直）。
- `app/src/changes.rs`：行的绘制、文件头、hunk 头、`pair`（左右对照的配对）和它的测试 `side_by_side_puts_deletions_beside_the_additions_after_them`、`fold_all` 和顶部按钮、空状态。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-13c-changes-fix`，分支 `p5-13c-changes-fix`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-13c-changes-fix`。
- 可以改：`app/src/changes.rs`；确有需要时 `app/src/footer_icon.rs`（换图标）。别的文件不动。

## 要做的
用户看过主控列的问题后：“好的，按照你的建议走吧。”主控在截图里看到的（Dune，界面字号 13，窄面板宽 500；加宽 720、Split）：

1. **整行没铺满**：加删行的底色、hunk 头的淡底、文件头下面的分隔线，只画到这一行文字结束的地方，各行长短不一、右边参差。要和样稿一样铺满面板（窄面板 Unified 和加宽 Split 都是）。横向滚动时，底色要跟着内容一起铺满，不能露出一截没底色的。
2. **Split 里纯新增的行画到了左栏**：`Cargo.toml` 新加的两行、新文件的全部内容都在左（旧）栏，右栏空着。纯新增要在右栏、左栏留空；纯删除在左栏、右栏留空；成对的删和加左右对齐（现在这种情况是对的）。
3. **Split 两栏不是固定各占一半**：每一行在自己文字结束处分栏，中线弯曲，右栏被挤到面板边缘截断。两栏要固定各占一半，中线笔直；每栏里放不下的长行在本栏里截断或随横向滚动看到，不能把另一栏挤走。
4. **“全部折叠／全部展开”的图标像 ×**，容易和关闭混淆。换成一眼能看出是折叠／展开的图标（例如上下收拢、上下展开的双箭头），悬停说明照旧。
5. **空状态里留着用不上的控件**：“Not a git repository”、没有焦点窗格这类读不到仓库的状态，不显示范围切换；0 个文件时不显示“全部折叠”按钮和文件数。

## 怎么算做完
- 上面五条达到，和样稿一致。
- 验证只做这些：
  - 第 2 条先写一个测试：纯新增（和纯删除）在 Split 里的配对，先确认它因为现在的 bug 失败，再修到通过。
  - 在 `app/` 下跑一次 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`。
  - 截图（现在能截了）：用装好的 `~/Applications/paddock.app/Contents/MacOS/paddock` 不行，它是旧代码，所以用你编出来的 release 程序起自己的窗口：加 `PADDOCK_NO_ACTIVATE=1`、`GPUI_TERM_WINDOW_ID=1`；HOME、`XDG_CONFIG_HOME`、`XDG_STATE_HOME` 指到临时目录；PATH 最前面放一个假 `corral`（`ls` 输出 `{"agents":[],"ok":true}`，其他输出 `{"ok":true}`）；临时布局文件里让右侧栏打开在 Changes、设宽度和 split；在一个造好改动的临时 git 仓库目录里启动（焦点 shell 的目录就是它），等 6～8 秒后 `screencapture -x -o -l <window-id>`；窗口大小可用启动参数 `--bounds X,Y,W,H`。拍三张：窄面板 Unified（宽 500）、加宽 Split（窗口约 1700 宽、面板 720）、“Not a git repository”。截图放 scratchpad，不入库，自己打开看过五条是否解决。
  - 觉得不够，在回复里说，不要自己加。

## 不要做
- 不改读 git、解析 diff、刷新的逻辑，不改别的标签、左侧栏、弹出框。不新增主题颜色键，不加依赖。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置和布局文件，不碰用户正在运行的 paddock。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。不读 corral 的内部状态目录。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-13c-changes-fix` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上三张截图的路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

**做了什么**
- 第 1、2、3 条是同一个原因：GPUI 的 `list` 按内容宽度排每一行，行没有占满面板。所以底色、hunk 淡底、文件头分隔线只画到文字结束处。Split 两半用的 `flex_1` 的 `flex-basis: 0%` 在不定宽的行里按内容算，各行在自己文字结束处分栏；纯新增那一行左半边没有内容，缩成一条线，右半边就从最左边开始画，看上去落在左栏。修法：diff 区的每一种行（文件索引、文件头、说明行、hunk 头、未改动行、Unified 行、Split 行）都设 `w_full()`。共用的 `sideways` 一处覆盖 hunk 头、Unified 和 Split 行。两栏因此固定各占一半，中线笔直；长行在本栏里截断，横向滚动照旧（滚的是栏里的代码，底色在整行上，不会露出没底色的一截）。
- 第 4 条：“全部折叠”换成上下两个带杆箭头收向中间一道横线，“全部展开”是从横线向上下离开。悬停说明不变。
- 第 5 条：不在仓库、没有目录可读时不显示范围切换（窄面板连同那一行一起不显示；“没有焦点窗格”原来就不显示）；0 个文件时不显示文件数和“全部折叠”按钮。
- 只改了 `app/src/changes.rs`。

**验证了什么**
- 按要求先写测试 `side_by_side_puts_pure_additions_right_and_pure_deletions_left`（Cargo.toml 中间加两行、新文件、删除的文件在 Split 里的配对）。**它在修之前就通过了，没有得到失败**：`sides()` 的配对本来就是对的，第 2 条是上面的布局原因，不是配对出错。测试留着，作为配对的保护。
- 修之前先用同样的环境截图，复现了第 1～4 条（`before-narrow.png`、`before-split.png`）；修之后截了三张：窄面板 Unified（宽 500）、加宽 Split（窗口 1700、面板 720）、“Not a git repository”，自己看过：整行铺满、中线笔直、新增在右栏、新文件只在右栏、不在仓库时没有范围切换、折叠图标不像 ×。截图在 scratchpad，没入库。
- `app/` 下 `cargo test --all-targets` 全过（lib 192 项），`cargo clippy --all-targets -- -D warnings` 通过。`cargo fmt --check` 只报 `sidebar.rs` 一处，是 main 上原有的，不在本任务范围，没动。

**拿主意的地方**
- 读完之前（还不知道是不是仓库）：这个目录上次读到是仓库，就显示范围切换，避免切换范围时它闪一下；第一次读一个目录时，读到结果后才出现。
- 范围切换在 Failed（git 出错）和 No base branch 时仍然显示，因为切到另一种范围可能就能读。
- 文件索引行和说明行也设了整行宽度：原因相同，悬停底色和右侧的增删数因此对齐到右边。

**没做的事**
- 0 个文件、读出错的状态没有另外截图；悬停、点击、横向滚动、真实 agent 下的效果留给用户实际看。
- 加宽时 “Not a git repository” 下 Unified／Split 切换还在，任务没列，没改。
