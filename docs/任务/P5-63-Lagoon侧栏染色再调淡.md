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
