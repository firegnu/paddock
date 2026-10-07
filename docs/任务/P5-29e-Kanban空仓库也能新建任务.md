# 任务：还没有任务文件的仓库也能用 New task，缺任务目录就先建

2026-10-07，paddock/main 交给 paddock/dev-kanban-e（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档常规 0.98；交叉审查拿不准，只新建目录和文件、不覆盖，定不要；影响面拿不准，定改行为）
类型：功能变更
依据：本轮只让“没有任务文件”的仓库也有 New task，并在建草稿时补上缺的 `docs/任务/` 目录；其余看板行为不变。
依赖：P5-29d
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「开发方式」里的**看板约定**。
- `docs/DESIGN.md` §13「P5-29 右侧栏 Kanban」里 New task 的几条（新建卡片、小弹框）。
- 代码：`app/src/kanban.rs`（`read` 里返回 `Read::NoTasks` 的地方、`create_draft`、`next_id`、`taken`）、`app/src/kanban_view.rs`（空状态，约 370 行；顶部一行和 New task 按钮、弹框）、`app/src/window.rs` 里 New task 的接线（只看）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-29e-kanban`，分支 `p5-29e-kanban`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-29e-kanban`。
- 可以改：`kanban.rs`、`kanban_view.rs`、`tests/kanban.rs`；`window.rs` 只在接线确实需要时小改。

## 要做的
用户原话（问“没有任务文件的仓库要不要也显示 New task”）：“显示，如果用户点击了New task，检查满足任务存储的条件不，不满足就建立文件夹之类（我记得必须是有一个类似任务的文件夹）”。

1. 仓库有 main、但 main 上没有任务文件（`docs/任务/` 不存在或是空的）时，空状态里也要有 New task：可以是空状态里的一个按钮，也可以照常显示顶部一行和 `+`，你定，和现有空状态的风格一致；说明文字顺带讲清楚建了以后会放在哪。
2. 弹框照旧；没有任何编号时编号框不预填，提示里给出格式例子（现在已有 “use the form P5-30”）。
3. 建草稿时，主仓库工作区里没有 `docs/任务/` 就先建这个目录（只建缺的那几层），再照旧用“只新建、已存在就失败”写文件。不提交、不暂存。路径上已有同名的普通文件、建不了目录时，弹框里说明原因，不关弹框。
4. 不在仓库里、没有 main 的情况照旧，不加 New task。

## 怎么算做完
- 上面四条达到；界面文字英文，只从主题取色、不新增主题颜色键。
- 验证只做这些：
  - 测试（在临时 git 仓库里）：没有 `docs/` 时建草稿会建出目录和文件；`docs/任务` 是普通文件时拒绝并说明；已有任务目录时行为不变。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 截图一张空仓库的看板（照 P5-29a 的做法：release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`、假 corral、预写的布局文件、临时仓库），自己看过。截图放 scratchpad，不入库。
  - 点击和输入留给用户实际用，写进完成记录。觉得不够，在回复里说，不要自己加。

## 不要做
- 不提交、不暂存、不改或删除任何已有文件；只新建缺的目录和新的任务文件。
- 不改看板其余行为（列、卡片、Needs you、DRAFT、Dropped、空组、主控行、Show all）。
- 不改 corral／ranch，不读 corral 的内部状态目录；不另起 `corral ls` 轮询。不新增主题颜色键，不加依赖。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件和正在运行的 paddock；测试和截图只在临时仓库里建文件。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-29e-kanban` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- 做了什么：
  - `kanban.rs`：`Read::NoTasks` 带上主 worktree 路径 `repo`，空仓库也知道草稿写到哪里。`create_draft` 在写文件前按层检查 `docs`、`docs/任务`：已是目录就跳过，缺的那一层用 `create_dir` 建（只建缺的），是普通文件就拒绝，新加 `Refused::NotFolder`，提示 “docs/任务 is a file, not a folder”（`docs` 是文件时说 `docs`）。检查放在编号和标题校验、撞号检查之后，被拒的情况不会留下空目录。之后照旧用 `create_new` 只新建文件，不提交、不暂存。
  - `kanban_view.rs`：“No task files” 空状态下面加一个带 `+` 的描边按钮 “New task”，说明改为 “Task files go in docs/任务 on main. New task writes a draft to docs/任务 in the main worktree, making the folder if it's missing.”。按钮用和顶部 `+` 同一个锚点格子，弹框挂在它下面；`new_task` 在没有看板时从 `Read::NoTasks` 取仓库路径。颜色只用已有的 `rule`、`text`、`muted`。
  - 弹框没改：没有任何编号时 `next_id` 返回 `None`，编号框本来就不预填；格式例子仍在编号不对时的提示 “Not a task ID: use the form P5-30” 里。
  - 不在仓库里、没有 main、Git 读失败的空状态照旧，没有 New task。`window.rs` 没改。
- 验证了什么：
  - 先写测试 `a_new_draft_makes_the_missing_task_folder_but_never_over_a_file`，看它因为 “No such file or directory” 失败，再实现：没有 `docs/` 时不预填编号，建草稿会建出 `docs/任务/` 和文件，`git status` 只有一行 `??`，读出来变成看板；`docs/任务` 是普通文件时拒绝并说明，文件原样，工作区没变化；`docs` 是普通文件时同样拒绝并点名 `docs`。已有任务目录的老测试 `a_new_draft_is_written_once_and_never_over_a_taken_id` 原样通过；`quiet_states_without_a_board` 改为带 `repo` 比较。
  - 在 `app/` 下：`cargo test --all-targets` 全过，`cargo clippy --all-targets -- -D warnings` 无警告，`cargo fmt --check` 通过。
  - 截图：release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`CFFIXED_USER_HOME`／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`、假 corral（sh 脚本，`ls` 返回空列表）、预写的布局（右侧栏开在 Kanban），在只有一条空提交的临时仓库里启动，只截自己的窗口（窗口号来自 `GPUI_TERM_WINDOW_ID=1`），看过后按记下的 PID 停掉。画面：Kanban 图标、“No task files”、两行半说明、下面 “+ New task” 按钮，风格和左侧栏空状态一致。截图在 scratchpad，没入库。
- 拿主意的地方：
  - 选在空状态里放按钮，不显示顶部一行和 `+`：顶部一行要有看板数据（摘要、⎇ main），空仓库没有可摘要的东西，按钮也更显眼。
  - 建目录在所有校验之后、写文件之前，只建缺的那几层；建目录本身失败（比如没有权限）时沿用 `Refused::Write`，提示 “Couldn't write the file: …”。
  - 测试里的 `git status` 加了 `-c core.quotePath=false`，中文路径才不会被转义。
- 没做的事：
  - 点击和输入留给用户实际用：点空状态的 New task 弹框出现的位置；空仓库编号框为空，填一个编号（例如 P1-1）和标题后建出 `docs/任务/` 和草稿，看板随后从空状态变成一张 DRAFT 卡，并在编辑器里打开；`docs/任务` 是文件时弹框里出红字、不关弹框。
  - 弹框的编号占位文字还是 “ID”，没加格式例子（任务说弹框照旧，例子已在拒绝提示里）。
