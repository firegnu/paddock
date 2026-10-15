# 任务：New Agent 底边那行放不下时不再换行，模型名让位截短

2026-10-15，paddock/main 记。
依据：用户 10-15 发来 New Agent 截图（Claude、`opus[1m] · xhigh`，Controller 掉到第二行）：“在集成agy之前先修一下这个界面的问题。下面那一行按钮在模型名字比较长的情况下，controller那个按钮会被挤到最下面”。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-102-foot-row`，分支 `p5-102-foot-row`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-102-foot-row`。
- 动 `app/src/new_agent_view.rs` 的底边（`foot`）和本文件。

## 要做的
- 底边的小块（项目、agent 种类、模型·强度、打开位置、Controller）始终排一行，不换行。
- 放不下时模型那块先让位：模型名截短、末尾 `…`，强度和下拉箭头照常显示；项目名过长时也截短。agent 种类、打开位置、Controller、`</>`、开始按钮不缩。

## 怎么算做完
用户原话：“下面那一行按钮在模型名字比较长的情况下，controller那个按钮会被挤到最下面”。
