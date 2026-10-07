# 任务：Kanban 窄面板空组变矮变淡、TO REVIEW 显示主控、DONE 能看全部

2026-10-07，paddock/main 交给 paddock/dev-kanban-d（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档常规 0.94；交叉审查不要；影响面改行为）
类型：功能变更
依据：本轮改看板的显示：空组样式、TO REVIEW 卡片多一行主控、DONE 组头和“Show all”；仍然只读，不写任何文件。
依赖：P5-29c
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「开发方式」里的**看板约定**。
- `docs/DESIGN.md` §13「P5-29 右侧栏 Kanban」，尤其最后“用户 10-07 看真窗口后再定三条”。
- 代码：`app/src/kanban.rs`（`Seen`、`agent_for`、`board`、`Board::count`、`DONE_KEPT`）、`app/src/kanban_view.rs`（组头、卡片的 agent 一行、悬停三个按钮、加宽五列）、`app/src/window.rs`（看板接线、跳到 agent 的窗格）、`app/src/new_agent.rs` 里 `role=controller` 标签从哪来。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-29d-kanban`，分支 `p5-29d-kanban`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-29d-kanban`。
- 可以改：`kanban.rs`、`kanban_view.rs`、`window.rs`（只在接线需要时）、`tests/kanban.rs`。另有一件活（P5-30）同时在改 `sidebar.rs`，你不要动它。

## 要做的
用户看着窄面板说：“看板的这个窄界面，还有优化的空间吗？里面的每一项任务关联着agent的那个特别好，千万别删掉那个展示（包括可以跳转的三个按钮），还有进入To Review的时候，只显示了委派出去的agent，没有main的情况。”又问：“现在Done的只显示5个了？”主控给了选项，用户选：空组变矮变淡；审查时显示 main；DONE 组头和 Show all 两样都要。

1. **空组变矮变淡**（窄面板）：五组位置不变；没有卡片的组只留一行紧凑的淡色组头（比现在矮，组与组之间不再空一大段）；组头右侧那句说明（not started、not merged 等）不再常显，改成组头的悬停提示，有卡片的组也一样。
2. **TO REVIEW 显示主控**：卡片在 TO REVIEW 时，在 dev agent 那一行下面再加一行主控：标签 `role=controller`、目录是这个仓库的主仓库或在它下面的 agent（有几个就取第一个没退出的）。这一行和 agent 行同一套样子（种类图标、状态记号、名字、状态），可以用一个小字说明它在审查（你定）；点这一行跳到主控的窗格，和左侧栏点卡片一样。悬停三个按钮仍对着 dev agent，行为不变。找不到主控就不加这一行。加宽五列里同样。
3. **DONE 能看全部**：组头数字写 `列出 / 全部`（例如 `5 / 73`；全部列出时只写一个数）；DONE 展开时列表末尾一行 “Show all 73”，点了列出全部，这一行变成 “Show fewer”，再点收回到最近 5 件（Needs you 的卡照旧总是列出）。展开状态只在内存里，不存进布局；加宽五列里 DONE 列同样。

**不能动的**：卡片上的 agent 一行、悬停三个按钮（打开任务文件、跳到 agent、看改动）的样子和行为；Needs you、DRAFT、Dropped、New task。

## 怎么算做完
- 上面三条达到，“不能动的”都还在；界面文字英文，只从主题取色、不新增主题颜色键，三套预置主题、界面字号 13 和 18 下都好看。
- 验证只做这些：
  - 测试：主控的认法（按标签和目录，有、没有、在别的仓库、已退出）、只在 TO REVIEW 加主控、DONE 组头的 `列出 / 全部` 和展开后列出全部。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 截图窄面板一张（照 P5-29a 的做法：release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`、假 corral 里有一个 `role=controller` 的主控和一个带 `task=` 的 dev agent、预写的布局文件、临时仓库里有空组、一张 TO REVIEW 卡、超过 5 件的 DONE），自己看过。截图放 scratchpad，不入库。
  - 点击、悬停留给用户实际用，写进完成记录。觉得不够，在回复里说，不要自己加。

## 不要做
- 不写任何文件、不做拖动或排序、不派活、不合并、不收尾；不新增数据存储；DONE 的展开状态不存进布局。
- 不改 corral／ranch，不读 corral 的内部状态目录；不另起 `corral ls` 轮询。
- 不改 `sidebar.rs`、Changes、Browser 标签和左侧栏卡片。不新增主题颜色键，不加依赖。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件和正在运行的 paddock。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-29d-kanban` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
