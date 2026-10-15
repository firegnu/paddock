# 任务：看板 DONE 里的编号和别的栏一样亮，不再整条变淡

依据：P5-114 合并后用户转来提醒：“按 P5-114 的计划，DONE 栏整条照旧变淡到六成左右，你截图里看不清的 R8、R7 这类编号可能还是偏淡。”主控核对属实（`fade()` 给 DONE 整条 62% 的透明度，编号跟着淡），提议 DONE 不再整条变淡，只让该淡的淡；用户：“开始吧”。
执行：主控

## 在哪里干活
- worktree：`../paddock-worktrees/p5-114a-done-ids`，分支 `p5-114a-done-ids`（已从 main 建好）。

## 要做的（`app/src/kanban_view.rs`）
1. 去掉 DONE 整条的透明度（`fade()`）。
2. DONE 的标题改用颜色变淡：普通的 `dim`，Dropped 的再淡一些；时间照旧 `dim`；圈照 P5-114（灰色的勾、斜线），不再叠透明度。
3. 编号和别的栏一样（`muted`、12 号、稍粗）。Needs you 那一行本来就不变淡，照旧。

## 验证
- 纯画法改动，不另造失败的测试；跑 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
- 截图照 AGENTS.md（只截自己的窗口、`PADDOCK_NO_ACTIVATE=1`、临时 HOME、假 corral），7 个内置主题窄面板各一张，看 DONE 的编号和别的栏一样清楚、标题仍比别的栏淡；截图不入库。

## 怎么算做完
- 用户最初的原话：“kanban中的任务编号不好看清和识别”（截图里正是 DONE 的 R8、R7）；这件：“开始吧”。

## 不做
- 不改别的栏、框、条和圈的样子。

## 完成记录（主控 10-15）
- 做了什么（`app/src/kanban_view.rs`）：去掉 `fade()`（DONE 整条 62% 透明度）；DONE 标题按颜色变淡：普通 `dim`，Dropped `dim` 再乘 0.7（`DROPPED_TITLE`），等你的照旧 `muted`；编号、圈、时间不再叠透明度。`docs/DESIGN.md` 的 P5-114 一条下补一行。
- 验证了什么：纯画法，没另造失败的测试。`cargo test --all-targets` 666 个通过，`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 通过。截图（同 P5-114 的办法，本仓库本地克隆、假 corral、临时 HOME，只截自己的窗口）：7 个内置主题窄面板各一张，DONE 的编号和 Queued 的一样亮，标题仍比别的栏淡，Dropped 那条更淡；截图没入库，已删。
- 拿主意的地方：等你的那条 DONE（如 P5-28c）原来就不变淡，标题照旧 `muted`，比别的 DONE 标题亮一点。
- 没做的事：无。
