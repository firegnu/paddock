# 任务：Cairn 面板的 Lessons 页改成新的在上

依据：用户 10-17：“我发现paddock中的cairn中的lesson排列好像是倒叙，如果有问题。就修一下”。主控查明：cairn `ledger --json` 给的 lesson 按编号从小到大（旧的在前），paddock 照原样画，所以是旧的在上，和 Ledger 页、`cairn ledger` 一致；问用户后选了“改成新的在上”（只改 Lessons 页，Ledger 页不动）。
执行：主控
优先：高

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-132-lessons-order`，分支 `p5-132-lessons-order`（已从 main 建好）。

## 要做的
- `app/src/cairn.rs` 读账时，Lessons 那一组按编号从大到小排；State、Open、Conventions、Other 照 cairn 的顺序不动。

## 怎么算做完
- 用户选的：“改成新的在上”。
- 先改 `tests/cairn.rs` 里 Lessons 那条测试的期望（新的在前），确认红，再改到绿；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check`。
