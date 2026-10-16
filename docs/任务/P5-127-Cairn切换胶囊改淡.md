# 任务：Cairn 的 Notes | Ledger | Lessons 选中的胶囊改淡，和左侧栏机器切换栏一致

2026-10-17，paddock/main 交给 paddock/dev-cairn-pill（omp，默认）。
路由：轻 / 交叉审查不要 / 影响面：看得见（主控定，没问路由：用户点名派 omp，只改一处画法）
类型：样式／文案调整
依据：用户 10-17：“右侧sidebar的cairn下的notes ledger还有lessons三个胶囊的颜色也是实心的，和刚才的不一致，改为一致吧。”“你直接派出omp吧。”“刚才的”是主控 10-17 刚改的左侧栏机器切换栏（提交 `90b7da6`）。
执行：派出
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`（规矩全文）。
- `docs/DESIGN.md` 最后一条「凹槽里选中的胶囊改淡（10-17…）」。
- `git show 90b7da6 -- app/src/sidebar.rs app/src/sunk.rs`：左侧栏改成什么样。
- 代码：`app/src/sunk.rs`（凹槽、实心胶囊、计数小签）、`app/src/cairn_view.rs` 的 `fn switch`、`app/src/sidebar.rs` 的 `fn switcher` 和常量 `SHOWN_FILL`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-127-cairn-pill`，分支 `p5-127-cairn-pill`（已从 main 建好）。
- 编译：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-127-cairn-pill`（已从 `.target/main` 克隆）。cargo 命令在 `app/` 下跑。
- 只动 `app/src/sunk.rs`、`app/src/cairn_view.rs`、`app/src/sidebar.rs` 和本文件。

## 要做的
1. 把左侧栏选中胶囊的画法（淡强调色底 `SHOWN_FILL`、`crate::window::TAB_RING` 宽的边：选中的 `RING_ACTIVE` 强调色、没选中的透明；选中的亮色字加粗）抽成 `sunk.rs` 里一处，左侧栏和 Cairn 切换都用它，两处看起来一样。
2. Cairn 切换选中的那颗照这个画：底、边、字（亮色用 `c.title`）都同左侧栏；里面的计数小签和没选中的一样（`sunk::count` 按没选中画）。没选中的照旧，悬停照旧。
3. 改完没人用的东西删掉：`sunk::capsule` 和它的常量；`sunk::count` 的 `on` 参数如果两处都只传 `false`，连同只给 `on` 用的常量一起去掉。`sunk.rs` 开头的说明和 `cairn_view.rs` `fn switch` 上面的说明改成现在的样子。

## 怎么算做完
- 用户原话：“右侧sidebar的cairn下的notes ledger还有lessons三个胶囊的颜色也是实心的，和刚才的不一致，改为一致吧。”
- 验证只做这些：`git diff --check`；`app/` 下 `cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`cargo test --lib` 各跑一次。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改凹槽本身、没选中的胶囊、右侧栏标签、别的界面。
- 不起 paddock 窗口、不截图（样子留给用户看）；不碰 `corral ls` 里的任何 agent。
- 不用 Python；命令里不用 `rm`；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 不加依赖；不合并到 main，不推送。只在 `p5-127-cairn-pill` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
