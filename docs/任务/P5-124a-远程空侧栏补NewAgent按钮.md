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

## 完成记录

- 做了什么：`sidebar.rs` 空列表那段把远程和本机合成一支：远程机器读到了、没有 agent 时，也画 New Agent 图标和 `New Agent` 按钮（点了发 `SidebarEvent::NewAgent`，经 P5-124 的入口打开那台的精简对话框）。标题照旧 `No agents on <显示名>`；下面那句从 “The agents corral runs there show here.” 改成和本机一样的 “Start one to see its status and replies here.”，因为现在有按钮了。
- 验证：`git diff --check`；`app/` 下 `cargo test --all-targets` 全过（lib 584），`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 干净。纯界面加按钮、走已有事件，没另写失败测试；没起窗口，样子留给用户看。
- 拿主意的地方：远程空状态那句说明改成和本机一样。
- 没做的事：没有。
- 踩过的坑：没有。
