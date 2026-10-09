# 任务：查 Dock 里偶尔多出第二个 paddock 后台进程

2026-10-09，paddock/main 写，主控自己做。
类型：调研（先只查不改）
依据：
- 用户 10-08：“有时候paddock会有第二个paddock的bakcground进程显示在dock中，要查一下”。
- 用户 10-09：“p5-52你自己做”。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-52-dock-second`，分支 `p5-52-dock-second`（已从 main 建好）。

## 要做的
1. 找出第二个 Dock 图标是哪个进程、怎么来的。主控的猜测：`~/.local/bin/paddock` 链到 app 包里的程序，agent 或终端跑 `paddock ctl …`、`paddock install-skills` 时以 app 包里的程序身份启动，macOS 也给它挂了 Dock 图标；另查有没有残留的实例（`paddock ctl instances`）。
2. 只读地查：进程列表、`main.rs` 里命令行分支走到哪一步才碰 AppKit／GPUI、可复现就复现（只开自己的临时进程，用完按 PID 停）。
3. 原因、证据和改法建议写进完成记录；不改程序（改不改、怎么改由用户看了再定）。

## 怎么算做完
- 用户原话：“有时候paddock会有第二个paddock的bakcground进程显示在dock中，要查一下”“p5-52你自己做”。
- 有原因和证据（或说明没能复现、查到哪一步），有改法建议；只改文档时 `git diff --check` 过。
