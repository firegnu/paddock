# 任务：Kanban 加 New task——窗口内小弹框填编号和标题，在主仓库写一份不提交的任务文件草稿并打开

2026-10-07，paddock/main 交给 paddock/dev-kanban-c（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档常规 0.90；交叉审查拿不准，只新建文件、不覆盖，定不要；影响面拿不准，看板第一次写文件，定改行为）
类型：功能变更
依据：本轮只加“新建任务草稿”：新建一个文件并打开，不提交、不改已有文件、不派活；DRAFT 卡的显示由 P5-29b 做。
依赖：P5-29b
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「开发方式」里的**看板约定**（尤其草稿一条：主仓库工作区里没提交的任务文件是草稿）。
- `docs/DESIGN.md` §13「P5-29 右侧栏 Kanban」里“基本看板功能”一段和其后用户再定的两条。
- `docs/调研/P5-29r2-Kanban基本功能.md` §3.2（新建卡片，含编号撞车、被扫进别的提交、格式两套三件要防的事）。
- 代码：`app/src/kanban_view.rs`（顶部一行、“Open task file” 怎么打开文件）、`app/src/kanban.rs`（`task_id`、`natural`、`read`）、`app/src/popover.rs` 和用它的窗口内弹出框（新标签框、分屏框）、`app/src/text_input.rs`（单行输入框）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-29c-kanban`，分支 `p5-29c-kanban`（等 P5-29b 合并后从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-29c-kanban`。
- 可以改：`kanban.rs`、`kanban_view.rs`、`window.rs`（接线）、`footer_icon.rs`（加图标）；`popover.rs`、`text_input.rs` 只在必要时小改。

## 要做的
用户原话：“我不想变成drover那么重，但是基本的看板的功能起码得有。”看过主控的建议：“按你的建议来。”New task 的样子用户在选项里选了“小弹框，建完打开文件”：

```
┌ New task ─────────────────┐
│ ID     [P5-30        ]    │
│ Title  [______________]   │
│                           │
│  Opens the draft in your  │
│  editor to add details.   │
│          [Cancel] [Create]│
└───────────────────────────┘
→ 写 docs/任务/P5-30-<标题>.md
  # 任务：<标题>
  ## 用户原话
  （空，等你写）
→ 用默认编辑器打开
```

1. **入口**：看板顶部一行加一个 New task 按钮（图标加悬停说明），看板显示的是某个仓库时才有。
2. **弹框**：窗口内小弹框，照现有弹出框的样子；两个单行输入框，编号预填“下一个空编号”（规则你定，例如当前最大系列编号加一，写进完成记录），可改；Return 确定，Esc 取消。
3. **建文件**：在**主仓库工作区**写 `docs/任务/<编号>-<标题>.md`，内容只有首行 `# 任务：<标题>`、空行、`## 用户原话` 一节（留空）。标题里不能进文件名的字符换掉。不提交、不暂存。写完用系统默认程序打开（和 “Open task file” 一样）。看板下一次刷新时它就是 P5-29b 的 DRAFT 卡。
4. **不让建的情况**（弹框里一行提示说明原因，不关弹框）：编号不合 `task_id` 的规则；main、工作区或任何本地分支上已有同编号的任务文件；目标文件已存在；标题为空。绝不覆盖已有文件。

## 怎么算做完
- 上面四条达到；界面文字英文，只从主题取色、不新增主题颜色键，三套预置主题、界面字号 13 和 18 下都好看。
- 验证只做这些：
  - 测试：下一个空编号、编号撞车（main、工作区、分支各一）、文件名里的非法字符、文件已存在不覆盖、写出的内容（在临时 git 仓库里做）。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 截图一张弹框（照 P5-29a 的做法：release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME、假 corral、预写的布局文件、临时仓库；打不开弹框就截看板顶部的按钮，写明），自己看过。截图放 scratchpad，不入库。
  - 点按钮、输入、Return／Esc、打开编辑器留给用户实际用，写进完成记录。觉得不够，在回复里说，不要自己加。

## 不要做
- 不提交、不暂存、不改或删除任何已有文件；不在 worktree 或分支上建文件；不派活、不合并、不收尾；不做拖动或排序；不新增数据存储。
- 不做多行输入框，不做卡片详情或卡片内编辑。
- 不改 corral／ranch，不读 corral 的内部状态目录；不另起 `corral ls` 轮询。
- 不改 Changes、Browser 标签和左侧栏卡片。不新增主题颜色键，不加依赖。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件和正在运行的 paddock；测试和截图只在临时仓库里建文件，不在 paddock 仓库里建。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-29c-kanban` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
