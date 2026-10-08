# 任务：Lagoon 侧栏磨砂染色再调淡到 0.25

2026-10-08，paddock/main 自己做（`preset.rs` 一处）。
类型：样式／文案调整
依据：用户 10-08（P5-50 之后）：“为什么现在是0.45，我觉得还是有点高。”主控说明 0.45 是 P5-50 估的、没对比过几档；用户：“0.25试一下，你直接改，快”。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-63-wash`，分支 `p5-63-wash`（已从 main 建好）。

## 要做的
- `Preset::Lagoon` 的 `frost().wash` 由 0.45 改为 0.25；其余主题不动（Dune 0.30，另五套 0.45），用户看过 Lagoon 再定要不要跟。
- 小字可读性：现有测试按不染色的磨砂灰检查，染色降低照样成立；`dim`／`dimmer` 不动。

## 怎么算做完
- 用户原话：“0.25试一下”。重新打包安装，用户重启后看。

## 完成记录
- 做了什么：`preset.rs` 里 `Preset::Lagoon` 的 `wash` 由 0.45 改为 0.25，说明注释补一句 P5-63。其余主题不动。
- 验证了什么：`app/` 下 `cargo test --all-targets`（452 项全过，含按不染色磨砂灰检查侧栏小字对比度的测试）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 都过。
- 拿主意的地方：只改 Lagoon（用户常用、这次说的就是它），其余五套留 0.45、Dune 留 0.30，等用户看过再定要不要跟。
- 没做的事：没截图（磨砂透出的是用户屏幕上窗口后面的东西，测试窗口不在前台，截不准），留给用户重启后实际看。
