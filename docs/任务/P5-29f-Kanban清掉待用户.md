# 任务：Kanban 的 Needs you（待用户）卡片加 Clear，用户自己清掉

2026-10-07，paddock/main 交给 paddock/dev-kanban-f（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档拿不准（常规 0.82），按规则用常规；交叉审查拿不准，只改一行、只提交一个文件、各种不对就不提交，定不要；影响面拿不准，看板第一次改已有文件和提交，定改行为）
类型：功能变更
依据：本轮只加“清掉 `待用户：`”这一个动作：删掉 main 上那份任务文件里的那一行并只提交这一个文件；不推送，不做其他写操作。
依赖：P5-29e
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「开发方式」里的**看板约定**（`待用户：` 一行怎么写、怎么删）。
- `docs/DESIGN.md` §13「P5-29 右侧栏 Kanban」最后几条，尤其“用户自己清掉 Needs you”。
- 代码：`app/src/kanban.rs`（`待用户：` 的解析 `asks`、`create_draft` 的写法和拒绝原因 `Refused`、`read`）、`app/src/kanban_view.rs`（卡片悬停按钮、New task 弹框里在卡片／弹框上显示原因的做法）、`app/src/git.rs`（`git::git` 的选项）、`app/src/window.rs`（看板接线，只看）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-29f-kanban`，分支 `p5-29f-kanban`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-29f-kanban`。
- 可以改：`kanban.rs`、`kanban_view.rs`、`git.rs`（只加函数）、`tests/kanban.rs`；`window.rs` 只在接线确实需要时小改。另有一件活（P5-32）同时在改 `sidebar.rs`、`layout_state.rs`、`motion.rs`，你不要动这三个文件。

## 要做的
用户看到两条 Needs you 不知道怎么处理，主控说明后用户问：“我觉得要不加一个按钮，我可以自己清掉这个状态，你觉得如何？”主控给了做法和保护，用户确认“点击按钮之后，need you就消失了”后说：“好的”。

1. **按钮**：只在因为任务文件里有 `待用户：` 而显示 Needs you 的卡片上，悬停时多一个 `Clear`（图标加悬停说明，和现有三个按钮同一套）。只因 agent 在 `Waiting` 而显示的 Needs you 没有这个按钮。窄面板和加宽五列都有。
2. **确认**：点了不马上改，卡片上就地出现一行确认（例如 `Clear "Needs you"? Commits to main.` 加确认／取消），确认后才做；鼠标移开或点取消就收回。
3. **做的事**：在主仓库工作区里，从这份任务文件删掉开头那行 `待用户：…`（只删这一行，别的字节不动），然后只提交这一个文件，提交首行 `<编号>：用户已处理 待用户`，正文照抄删掉的那行；不推送。提交后看板照常刷新，卡片上的 Needs you 和顶部摘要里的数字随之消失。
4. **不提交的情况**（卡片上说明原因，什么都不改）：主仓库当前分支不是 main；正在合并、变基、cherry-pick 或 revert；这份任务文件在工作区或暂存区里已有别的未提交改动；main 上这份文件里已经没有这一行；写文件或 git 失败（例如 `index.lock` 被占）。别的文件有没有未提交的改动、有没有暂存的东西都不管，也不能被带进这次提交。
5. 在后台做，不卡界面；同一张卡片做的时候不能再点。

## 怎么算做完
- 上面五条达到；界面文字英文，只从主题取色、不新增主题颜色键。
- 验证只做这些：
  - 测试（在临时 git 仓库里）：正常清掉（只删那一行、只提交这一个文件、提交首行和正文对、别的已暂存文件仍暂存且没进提交）；不在 main；合并进行中；这份文件已有别的改动；那一行已经不在；agent `Waiting` 的卡片没有按钮。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 截图一张带 Clear 按钮或确认行的卡片（照 P5-29a 的做法：release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`、假 corral、预写的布局文件、临时仓库；悬停截不到就临时加一个只在截图时用的开关，截完撤掉不提交，写明），自己看过。截图放 scratchpad，不入库。
  - 点击、确认留给用户实际用，写进完成记录。觉得不够，在回复里说，不要自己加。

## 不要做
- 不推送；不提交任务文件以外的任何东西；不改任务文件里 `待用户：` 以外的内容；不做别的写操作（派活、合并、收尾、拖动、改列）。
- 不改 corral／ranch，不读 corral 的内部状态目录；不另起 `corral ls` 轮询。
- 不改 `sidebar.rs`、`layout_state.rs`、`motion.rs`、Changes、Browser 和左侧栏卡片。不新增主题颜色键，不加依赖。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件和正在运行的 paddock；测试和截图只在临时仓库里提交，绝不在 paddock 仓库或任何真实仓库里提交。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-29f-kanban` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- **做了什么**：`kanban.rs` 加 `asks_line`（找到开头那行 `待用户：`，和 `parse` 的认法一致，连行尾一起）、`Card::clearable`（只有任务文件里有 `待用户：` 的卡片才算，草稿和只因 agent `Waiting` 的不算）、`clear_asks`（在主仓库工作区里删掉这一行，`git commit --only -- <这份文件>`，首行 `<编号>：用户已处理 待用户`，正文照抄删掉的那行；不推送）。`git.rs` 只加了 `git_write`：和 `git` 选项一样（去掉继承的 `GIT_*`、关 fsmonitor），超时 30 秒，因为提交会跑仓库的钩子。`kanban_view.rs`：悬停按钮条多一个 ✓（悬停提示 `Clear “Needs you”`），点了卡片下方就地出现 `Clear “Needs you”? Commits to main.` 加 Cancel／Clear；确认后在后台做，做的时候显示 `Clearing…`，按钮不再出现；成功后取消进行中的读取、马上重读，卡片上的 Needs you 和顶部摘要数字随之消失；不提交时这一行换成红字 `Not cleared: <原因>`。鼠标移开收回问句和原因。窄面板和加宽五列共用同一段代码。`window.rs` 没改。
- **不提交的情况**（都是什么都不改）：主仓库当前分支不是 main（或者 HEAD 游离）；有 `MERGE_HEAD`、`rebase-merge`、`rebase-apply`、`CHERRY_PICK_HEAD`、`REVERT_HEAD`；`git status -- <这份文件>` 有输出（工作区或暂存区里有改动）；main 上没有这份文件、文件里已经没有这一行，或者那一行和卡片上显示的不一样；读写文件或 git 失败。提交失败时把文件原样写回。
- **验证**：`tests/kanban.rs` 新增三个测试，在临时仓库里跑（仓库自己的配置固定了 user、`commit.gpgsign=false`、空的 `core.hooksPath`，因为提交时不继承测试的环境）：正常清掉（只删那一行、引用在小节里的同一行和行尾空格都不动，HEAD 只多一条提交且只含这份文件，存下的提交说明逐字节核对，别的已暂存文件仍暂存且没进提交，未暂存的改动还在，读回来的看板 `need_you()==0`；再清一次说那一行已不在、HEAD 不变）；不在 main（另一个分支、HEAD 游离）；冲突停下的合并；这份文件有未暂存、已暂存的改动；那一行已不在、内容对不上；`index.lock` 被占（提交失败，文件原样写回）；agent `Waiting` 的卡片 needs_you 但不 clearable。`kanban.rs` 单元测试加 `asks_line` 和 `parse` 一致（CRLF、最后一行没换行、小节里的、缩进的）。在 `app/` 下跑了 `cargo test --all-targets`（全过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`，都过。
- **截图**：release 程序，`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`CFFIXED_USER_HOME`／`XDG_STATE_HOME`／`XDG_CONFIG_HOME`、假 corral（sh 脚本，`ls` 返回空列表）、预写的布局（右侧栏开在 Kanban、宽 400），临时仓库三件活（一件 IN PROGRESS 带 `待用户：`）。悬停截不到，临时加了一个只在截图时用的环境变量开关（`PADDOCK_SHOT_CLEAR`，让第一张可清的卡片显示成悬停并且在问），截完已撤掉，没有提交。只截自己的窗口（`GPUI_TERM_WINDOW_ID=1`），截完按记下的 PID 停掉。我看过：按钮条里有 ✓，卡片下方是问句和 Cancel／Clear。第一张截出来时窄面板里的两个按钮在换行时被拆开了，改成两个按钮放在一组里一起换行，重截确认。加宽五列那张没截成：布局里侧栏宽 400，达不到五列并排的阈值。
- **拿主意的地方**：按钮图标用现有的 ✓（`Icon::Check`），不新增图标；确认按钮用 Needs you 标记同一套黄色（主题里已有的 `agents_yellow`），没加颜色键。卡片上显示的那句和 main 上的那行要对得上才删，免得删掉用户没看见的新内容。提交照常跑仓库的钩子和签名配置（没加 `--no-verify`），所以超时放到 30 秒。原因显示到鼠标离开卡片为止。
- **没做的事**：真实点击、确认、取消、鼠标移开收回、后台做的时候按钮消失，都没有实际操作过，留给用户实际用。加宽五列没截图。实现是先写的，测试后写，测试没有先跑出失败。
