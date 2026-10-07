# 任务：Kanban 加强（只读）——Needs you 标记、DRAFT 草稿卡、Dropped、两处小修

2026-10-07，paddock/main 交给 paddock/dev-kanban-b（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档常规 0.97；交叉审查拿不准，只读 git、不写文件，定不要；影响面拿不准，改看板的推断和显示，定改行为）
类型：功能变更
依据：本轮只在现有只读看板上多认几样东西并显示出来；不新建卡片、不写任何文件、不能拖（New task 是下一件 P5-29c）。
依赖：P5-29a
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「开发方式」里的**看板约定**（含 10-07 新加的四条：一条提交一个编号、`收尾: <编号> 不做：`、草稿、`待用户：`）。
- `docs/DESIGN.md` §13「P5-29 右侧栏 Kanban」整条，尤其“加强”A、C、D 和“基本看板功能”两段。
- `docs/调研/P5-29r2-Kanban基本功能.md` §3.6（Dropped）。
- 代码：`app/src/kanban.rs`（`read`、`Facts`、`column`、`board`、`wrapped_id`）、`app/src/kanban_view.rs`（组头数字、卡片、`−{deleted}`），`app/src/agents.rs` 的 `Status`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-29b-kanban`，分支 `p5-29b-kanban`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-29b-kanban`。
- 可以改：`kanban.rs`、`kanban_view.rs`；`git.rs` 只加函数。

## 要做的
用户原话：“A，C，D我觉得可以列到接下来要做的了”；10-07 对基本看板功能：“按你的建议来”，并选定 `待用户：` 写在任务文件里。

1. **A “Needs you”**：下面两种任一成立，卡片上显示 “Needs you”（醒目但只用主题色），不改它所在的列：
   - 对应的 agent 状态是 `Waiting`（卡在权限框或提问框上），用左侧栏已有的数据；
   - main 上这份任务文件开头有一行 `待用户：<什么事>`；卡片上能看到这件事（第二行或悬停说明，你定，和样稿风格一致）。已收尾（DONE）的卡片有这一行也要显示，并且这样的卡片在 DONE 折叠时也不能被藏掉（放法你定，写进完成记录）。
   - 顶部摘要加上 “N need you”（N 为 0 不显示）。
2. **C DRAFT 卡**：主仓库工作区 `docs/任务/` 里有、main 上没有的 `.md`（没跟踪的或已暂存未提交的），显示成淡色卡，标 DRAFT，放在 QUEUED 组里最前面；编号、标题照常从文件名和首行取，取不到编号的也显示（用文件名）。悬停只有“打开任务文件”。
3. **Dropped**：收尾提交首行是 `收尾: <编号> 不做：…` 的，卡片仍在 DONE，标 Dropped（淡色），悬停说明显示原因。
4. **D 小修**：增删为 0 的那一边不显示（不出现 “−0”、“+0”）；DONE 组头的数字等于实际列出来的件数。

## 怎么算做完
- 上面四条达到；界面文字英文，只从主题取色、不新增主题颜色键，三套预置主题、界面字号 13 和 18 下都看得清。
- 验证只做这些：
  - 测试：`待用户：` 行的解析（有、无、在 DONE 卡上）、agent `Waiting` 时的 Needs you、草稿识别（在临时 git 仓库里造一份未跟踪、一份已暂存的任务文件）、`不做：` 收尾的 Dropped、`−0` 不显示、DONE 组头数字。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 截图一张窄面板（照 P5-29a 的做法：release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`、假 corral、预写的布局文件、造好的临时仓库，含 Needs you、DRAFT、Dropped 各一张卡），自己看过。截图放 scratchpad，不入库。
  - 悬停、点击留给用户实际用，写进完成记录。觉得不够，在回复里说，不要自己加。

## 不要做
- 不写任何文件、不新建卡片、不做拖动或排序、不派活、不合并、不收尾；不新增数据存储。
- 不改 corral／ranch，不读 corral 的内部状态目录；不另起 `corral ls` 轮询。
- 不改 Changes、Browser 标签和左侧栏卡片。不新增主题颜色键，不加依赖。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件和正在运行的 paddock。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-29b-kanban` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
