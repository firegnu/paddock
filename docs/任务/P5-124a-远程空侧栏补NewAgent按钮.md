# 任务：远程机器侧栏空着时也给 New Agent 按钮

依据：P5-124 合并后用户问入口，主控核对时发现本机侧栏空着时中间有 `New Agent` 按钮、远程侧栏空着时只写 `No agents on <机器>`；用户 10-17：“加吧，空侧栏补个 New Agent 按钮”。
执行：主控
优先：高

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-124a-empty-button`，分支 `p5-124a-empty-button`（已从 main 建好）。

## 要做的
- `app/src/sidebar.rs` 空列表那段：远程机器（已读到、没有 agent）时也画本机那样的图标和 `New Agent` 按钮，点了发 `SidebarEvent::NewAgent`，走 P5-124 的入口打开那台的精简对话框。标题照旧 `No agents on <显示名>`。

## 怎么算做完
- 用户原话：“加吧，空侧栏补个 New Agent 按钮”。
- 纯界面上加一个按钮，走已有事件，不另写失败测试；`git diff --check`，`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
