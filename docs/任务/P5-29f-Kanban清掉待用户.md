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
