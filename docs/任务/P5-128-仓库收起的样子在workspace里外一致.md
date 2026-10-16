# 任务：仓库小标题收起来时，名字前面也画 agent 的叠卡（同收起的 workspace）

2026-10-17，paddock/main 交给 paddock/dev-group-stack（omp，默认）。
路由：轻 / 交叉审查不要 / 影响面：看得见（主控定，没问路由：用户点名派 omp，只改左侧栏一处画法）
类型：功能变更
依据：用户 10-17：“左侧agents一个repo下有多个agents且不在一个workspace中的时候，收缩起来是icon stack，但是在workspace就是完全收起来了。行为不一致。这个任务依然派omp来做”；主控查明仓库小标题收起来从来没有叠卡后，用户发来截图（收起的 workspace `My-Harness` 名字前的叠卡、收起的仓库 `global-mesh` 只有一行）：“我想要上面箭头那种效果，但是那个效果只在workspace收缩的时候。我要的是左侧sidebar只要有收缩agents的行为都要有那种stack的效果。”
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`（规矩全文）。
- `docs/DESIGN.md` §13 的 P5-89（仓库收起）、P5-91（workspace 收起时的叠卡）两条。
- 代码：`app/src/card.rs`（`Heading`、`Tray::stack`、`stacked`、`MOST`，以及造 `Heading`、`Tray` 的地方）、`app/src/sidebar.rs` 的 `fn heading`（仓库小标题）、`app/src/sidebar_workspaces.rs` 里 workspace 标题画叠卡的那段（`tab_stack::Face::of_kind`、`tab_stack::Inks::of`、`tab_stack::stack`）、`app/src/tab_stack.rs`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-128-group-stack`，分支 `p5-128-group-stack`（已从 main 建好）。
- 编译：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-128-group-stack`（已从 `.target/main` 克隆）。cargo 命令在 `app/` 下跑。
- 只动 `app/src/card.rs`、`app/src/sidebar.rs`、`app/src/sidebar_workspaces.rs`（要抽公共代码时）、相关测试、`docs/DESIGN.md` 和本文件。

## 要做的
1. 仓库小标题收起来时，名字前面画这一组 agent 的叠卡，画法、大小、张数和收起的 workspace 标题前那个完全一样：最多三张，最前面一张是活动窗格显示的那个 agent（它在这组里的话），否则是列表里的第一个，后面的按列表先后；只有 1 个 agent 时画一张。workspace 里、外的仓库小标题都这样。
2. 哪几张、谁在前照 `Tray::stack` 的算法在 `card.rs` 里算（给 `Heading` 加同样的字段，共用 `stacked`）；画叠卡的代码和 workspace 标题共用一处，不复制一份。
3. 展开时仓库小标题照旧（没有图标）；箭头、个数、强调色小圈、右边的状态细条、悬停、点击收起展开、按钮和菜单都不变。收起的 workspace 照旧。只加叠卡这个图标，不给仓库加 workspace 那种一摞卡片的底。
4. `docs/DESIGN.md` §13 P5-91 那条后面补一条：仓库收起时也画叠卡（改 P5-91 里“不在 workspace 里的仓库小标题、仓库的收起……都不变”），附用户原话。

## 怎么算做完
- 用户原话：“我想要上面箭头那种效果，但是那个效果只在workspace收缩的时候。我要的是左侧sidebar只要有收缩agents的行为都要有那种stack的效果。”
- 验证只做这些：`card.rs` 里给收起的仓库算叠卡补一条测试（谁在前、最多三张、活动窗格的 agent 不在这组时取第一个）；`git diff --check`；`app/` 下 `cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`cargo test --lib` 各跑一次。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改卡片、workspace 的托盘和一摞底、分屏标签的叠卡样子、左侧栏收成窄条时的画法、别的界面。
- 不起 paddock 窗口、不截图（样子留给用户看）；不碰 `corral ls` 里的任何 agent。
- 不用 Python；命令里不用 `rm`；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 不加依赖；不合并到 main，不推送。只在 `p5-128-group-stack` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
