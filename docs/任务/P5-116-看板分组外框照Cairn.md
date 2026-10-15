# 任务：看板分组的外框照 Cairn 的外框：离边 8pt、边更清楚，颜色留灰

依据：用户 10-15 发来 Cairn 和 Kanban 两张截图：“kanban中queued的那个外框和cairn中的那个外框样式不一样。”“我说的只是那个外框的样式，里面不用动”；问统一成哪边时用户：“你的建议呢？”主控建议 Kanban 的外框照 Cairn 的形状、颜色留灰、Cairn 不动；用户：“可以，开始吧”。
执行：主控

## 在哪里干活
- worktree：`../paddock-worktrees/p5-116-kanban-frame`，分支 `p5-116-kanban-frame`（已从 main 建好）。

## 现状（主控 10-15 查的）
- Cairn 的 Next session 外框（`cairn_view.rs`）：离面板边 8pt，圆角 10，1px 强调色 26% 的边、强调色 7% 的面。
- Kanban 窄面板的分组外框（`kanban_view.rs`，P5-113）：离面板边 12pt（`LIST_X`），圆角 10，1px 文字色 7% 的边、文字色 3.5% 的面。

## 要做的（只改窄面板的外框，里面不动）
1. 外框离面板边改成 8pt；分组标题的位置不动（仍和 Cairn 的标题对齐），DONE 的条跟着框对齐。
2. 外框的边加深到文字色 12% 左右，和 Cairn 的边差不多醒目；面、圆角不变；颜色照旧从主题取，不用强调色。
3. 框里的条、细线、悬停、加宽后的五列、Cairn 都不动。

## 验证
- 纯画法改动，不另造失败的测试；跑 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
- 截图照 AGENTS.md：7 个内置主题各截 Kanban 一张，再截一张 Cairn 对照外框的位置；截图不入库。

## 怎么算做完
- 用户原话：“kanban中queued的那个外框和cairn中的那个外框样式不一样。”“我说的只是那个外框的样式，里面不用动”；看过建议：“可以，开始吧”。

## 不做
- 不改 Cairn；不给 Kanban 的框用强调色。
