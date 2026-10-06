# 任务：右侧栏 Kanban 标签——只读的流水线看板（任务文件 → worktree／分支 → agent → 审查 → 合并 → 收尾）

2026-10-07，paddock/main 交给 paddock/dev-kanban（Claude Code，常规：opus[1m] / high）。
路由：（派出时补）
类型：功能变更
依据：本轮做只读看板：从任务文件、git、corral 推出每件活的状态并显示；不能拖、不派活、不合并、不收尾，不写任何文件。
依赖：P5-28a
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」一节，以及「开发方式」里的**看板约定**（任务文件名、首行、worktree／分支那一行、`依赖：`、`--label task=`、合并和收尾提交的首行）。
- `docs/DESIGN.md` §13 的「P5-29 右侧栏 Kanban」（用户定的五条和样稿结论）。
- 调研 `docs/调研/P5-29r-Kanban集成.md` 的 §3.3（常见的坑）、§4（paddock 现状和已经对不上的例子）、§5.1（方案一）。
- 样稿：`docs/设计稿/P5-29-Kanban/README.md`、`Board.dc.html`（窄面板用 `layout=list`，加宽用 `layout=wide`；`columns` 未采用）。
- 代码：`app/src/right_panel.rs`（标签、宽度、加宽、`Saved`）、`app/src/changes.rs`（另一个右侧栏标签怎么跟随焦点窗格、后台读 git、加宽阈值、空状态，照这个风格做）、`app/src/git.rs`（只读的 git 调用和选项）、`app/src/agents.rs`／`app/src/corral.rs`（左侧栏已经拿到的 corral agent 列表和标签，复用它，不要另起一个 `corral ls` 轮询）、`app/src/card.rs`（agent 种类图标和状态记号）、`app/src/window.rs`（跳到某个 agent 的窗格是怎么做的）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-29a-kanban`，分支 `p5-29a-kanban`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-29a-kanban`。
- 可以改：新建 Kanban 模块（数据推断一个、视图一个，名字你定）、`right_panel.rs`（加第三个标签）、`window.rs`（接线、跳到 agent）、`layout_state.rs`／`Saved`（各组折叠状态）、`footer_icon.rs`（标签图标、卡片上的小图标）、`lib.rs`。`git.rs` 只加函数。

## 要做的
用户原话：“我还没想好。现在的卡片式的kanban任务系统集成在一个咱们这种系统中一般是怎么做的？派一个agent去调查。”看过调研和主控推荐后：“可以，按你的推荐出样稿。”看过样稿：“按你的推荐，窄面板用 A，加宽用并排的列”。

1. **第三个标签 Kanban**：照样稿加在 Changes、Browser 后面（图标照样稿），开关、宽度、当前标签照现有的保存。
2. **看哪个仓库**：跟焦点窗格所在的 git 仓库（主仓库，worktree 也归到它的主仓库），和 Changes 一致；读这个仓库 main 上的 `docs/任务/*.md`。没有这个目录、不在仓库里、没有焦点窗格时，给安静的空状态（图标、一行标题、一行说明）。
3. **每张卡片是一份任务文件**：编号取文件名开头（`P5-29a`），标题取首行 `# 任务：` 后面的字；worktree 和分支取「在哪里干活」那一行；`依赖：` 一行可选。
4. **状态推断（只读）**，从后往前判断，命中就停：
   - **DONE**：main 上有首行以 `收尾: <编号>` 开头的提交（编号要整词匹配，`P5-24` 不能匹配到 `P5-24a`）。
   - **MERGED**：分支已合进 main，或 main 上有首行以 `合并 <编号>` 开头的提交；但还没有收尾提交（例如 worktree 还在）。
   - **TO REVIEW**：没合并，分支上的这份任务文件里已有「## 完成记录」，或者对应的 agent 已经空闲且这一轮回复以 DONE 结尾（只用左侧栏已有的数据，拿不到就只看完成记录）。
   - **IN PROGRESS**：没到上面三步，但 worktree 存在或对应的 agent 存在。
   - **QUEUED**：以上都不是。写了 `依赖：` 而依赖的那件还没 DONE 时，卡片写 “Waits for <编号>”。
   - 对应的 agent：先找标签 `task=<编号>` 的，找不到再按 agent 的目录等于 worktree 路径找。两个都找不到就当没有 agent，不要猜名字（corral 会给名字加 `-1`）。
   - 推不准的情况（比如分支已删但没有合并提交、研究任务只摘了文档）宁可放在更早的一列，不要放到更后面去；写进完成记录。
5. **刷新**：右侧栏开着并停在 Kanban 时，后台定时读（间隔你定，和 Changes 同一个量级即可），收起或换标签就停；读 git 只用只读命令、带 `--no-optional-locks`，不卡界面；结果没变不重画。
6. **窄面板（样稿 A）**：五组上下排：QUEUED、IN PROGRESS、TO REVIEW、MERGED、DONE；组头有颜色小方块、名字、数量、右侧一句说明；每组可以折叠，DONE 默认收起且只留最近 5 件；折叠状态随布局保存。卡片照样稿：第一行编号（等宽、淡色）、标题、多久之前；第二行有 agent 时显示种类图标和状态记号（在做时是转动的圈圈，和左侧栏卡片同一套）、agent 名、状态、分支、增删行数（有就显示）；DONE 的卡片变淡、只一行。顶部一行写仓库名、分支，右边一句摘要（几件在做、几件待审）。
7. **加宽后（样稿 wide）**：面板宽到放得下时（阈值同 Changes）换成五列并排，每列可以单独滚动；卡片照样稿。
8. **悬停入口（只读）**：鼠标移到卡片上，右上角出现：打开任务文件（用系统默认程序打开那个 `.md`）、跳到 agent（有 agent 时：和点左侧栏卡片一样把它的窗格调到前面）、看改动（有 agent 时：跳到它的窗格并把右侧栏切到 Changes）。没有任何会改变工作状态的按钮。
9. 界面文字英文，任务标题照文件原文显示；只从主题取色、可加透明度，不新增主题颜色键；字号用 `ui.px`；三套预置主题、界面字号 13 和 18 下都好看，标题太长省略不折行（加宽的列里最多两行）。

## 怎么算做完
- 上面九条达到，和样稿一致。
- 验证只做这些：
  - 测试：任务文件解析（编号、标题、worktree／分支行、`依赖：`、缺行时的默认）、状态推断（在临时 git 仓库里造出五种状态各一件，含 `P5-24`／`P5-24a` 编号不串、分支已删、研究任务只摘文档）、agent 对应（按标签、按目录、都没有）、`Saved` 旧布局文件读入默认值。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 截图：用你编出来的 release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`、假 corral（`ls` 给一两个带 `task=` 标签的 agent）、预写的布局文件（右侧栏开在 Kanban），在一个造好五种状态的临时仓库里启动，`--bounds` 设窗口大小，截窄面板和加宽各一张，自己看过。截图放 scratchpad，不入库。
  - 点击、悬停、跳转留给用户实际用，写进完成记录。觉得不够，在回复里说，不要自己加。

## 不要做
- 不做拖动、派活、合并、收尾、写回任务文件或任何文件；不新增数据存储（折叠状态进布局文件除外）。
- 不改 corral／ranch，不读 corral 的内部状态目录；不另起 `corral ls` 轮询，用左侧栏已有的数据。
- 不改 Changes、Browser 标签的内容，不改左侧栏卡片。不新增主题颜色键，不加依赖。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件和正在运行的 paddock。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-29a-kanban` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方（尤其推不准时放哪一列）、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
