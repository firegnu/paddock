# 任务：agent 停掉后，显示它的窗格自动关闭，右边不再出现空白加红字

2026-10-08，paddock/main 交给 paddock/dev-panes（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档拿不准（常规 0.78），交叉审查拿不准（核心规则 0.6），影响面拿不准；主控定：只关本地窗格、不碰 corral 和数据，按改行为、不交叉审查；“读不到 agent 名单时不能当成停了”写进要求）
类型：功能变更
依据：本轮只改“agent 停掉之后窗格怎么办”和“窗口里一个窗格都不剩时显示什么”；不改 agent 暂停、不改 corral。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」。
- 代码：`app/src/viewer.rs`（attach 子进程、`poll_session` 里 attach 退出后写的 note、`agent has exited before attach`／`agent identity changed before attach`／`agent attached elsewhere` 的检查）、`app/src/window.rs`（`restore` 里“agent 不在了就说一声”、`stop_agent`、`close_pane`、`new_shell`、状态行消息、agent 名单刷新后的处理）、`app/src/layout.rs`（`close_pane`、`remove_tab`：最后一个标签关掉后留一个空窗格）、`app/src/agents.rs`（名单刷新、`exited` 状态、读失败）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-47-panes`，分支 `p5-47-panes`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-47-panes`。

## 要做的
用户 10-08：“停掉某一个agent之后。就停在一个空页面（有一些红字要attach之类的），我觉得这个体验很不好”。主控先提了按停法分开处理、显示卡片等方案，用户：“其实我就是不想右边出现空白+红字。”主控收成一条规则（agent 一停，显示它的窗格就自动关掉；窗口一个窗格都不剩时开 shell；状态行提示一句；代价是崩溃时最后的输出看不到），用户：“可以，写任务文件吧。”

1. **agent 停了 → 显示它的所有窗格自动关闭**：不管右边当前看不看得见、在哪个标签里，也不管怎么停的（paddock 里的 Stop、agent 里 `/exit`、终端里 `corral stop`、崩溃退出）。同名 agent 关了又重开（新实例）也算旧的停了，旧窗格照样关。
   - 关窗格沿用现在的 `close_pane`：标签里没窗格了就连标签关掉，焦点移到相邻的窗格或标签。
   - 窗格里正开着查找条、选区等，一并收掉，不留残影。
2. **判断“停了”要确实**：只有确认 agent 已退出（attach 子进程退出、并且 corral 能读到它已 `exited` 或已不在名单里），才关窗格。**读不到 agent 名单（corral 调不通、超时、输出坏了）不能当成停了**，这时不关任何窗格，照现在的方式提示读取失败。暂停中的 agent 不是停了，不关。
3. **启动时恢复布局**：保存的布局里有已经不在的 agent，那个窗格不再显示“不在了”的红字，同样直接不恢复（关掉），规则同上。
4. **窗口里一个窗格都不剩时开一个 shell**：和刚启动 paddock、按 ⌘N 新开 shell 一样（目录照现在 ⌘N 的取法）。这适用于所有“关到最后一个”的情况，包括用户手动关最后一个标签；现在留下的空窗格不再出现。
5. **状态行提示**：每关掉一个 agent 的窗格（或一次关掉几个），左下角状态行提示一句 “<名字> ended”，几秒后消失，沿用现在的消息位置和样式。
6. 不再出现 `attach exited`、`agent has exited before attach`、`agent identity changed before attach` 这类红字窗格。`agent attached elsewhere`（agent 没停、只是在别处接着）不在本轮，照旧；真正的错误（corral 本身调不通）照旧用红字。

## 怎么算做完
- 用户原话：“停掉某一个agent之后。就停在一个空页面（有一些红字要attach之类的），我觉得这个体验很不好”“其实我就是不想右边出现空白+红字。”“可以，写任务文件吧。”
- 先写会失败的测试再实现，覆盖：显示中的 agent 停了窗格关闭；不在当前标签的也关；一个 agent 在两个窗格里两个都关；标签空了一起关；最后一个窗格关掉后是 shell 而不是空窗格（手动关最后一个标签也一样）；读名单失败时不关；暂停的不关；恢复布局时不在的 agent 不恢复。
- `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 都过。
- 用假 corral 走一遍（只截自己的测试窗口，临时 HOME／XDG、`PADDOCK_NO_ACTIVATE=1`）：显示着一个 agent，让假 corral 改成它已退出，截图看窗格已关、状态行有提示；截图放 scratchpad，不入库。真实 agent 停掉的体验留给用户试。

## 不要做
- 不改 corral、ranch；不改配置和布局文件格式（读到旧布局照常能用）；不改暂停。
- 不对用户的真实 agent 做任何操作；需要真实 agent 才能验证的，留给用户。
- 不加新依赖；不用 Python。
- 命令里不用 `rm`，不用 `sh -c` 包长命令。
- 不合并到 main，不推送。只在 `p5-47-panes` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
