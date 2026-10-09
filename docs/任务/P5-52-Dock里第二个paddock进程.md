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

## 完成记录

- 用户补充（10-09）：多出来的不是 paddock 的图标，而是 shell 那种图标；在 Dock 上，不是菜单栏。
- 原因：**从编译目录直接启动的 paddock**（`.target/<子目录>/debug/paddock` 或 `release/paddock`，不在 `.app` 包里）。它会向系统登记成前台应用（Foreground），不在 app 包里就没有自己的图标，Dock 用通用的可执行文件图标（像 shell）显示，名字是 paddock。主控和派出去的 dev agent 截图、实测时都按 AGENTS.md 这样起测试窗口（临时 HOME、假 corral、`PADDOCK_NO_ACTIVATE=1`），它不抢前台，所以看着像个“后台进程”；测试窗口跑的那几秒到几分钟里 Dock 上就多一个，有人忘了停就一直挂着——所以是“有时候”。
- 证据：
  - 照截图的做法起了一个 debug 版（临时目录、假 corral、`PADDOCK_NO_ACTIVATE=1`），`lsappinfo` 显示它登记为 `type="Foreground"`、`fileType="????"`（没有 app 包）、可执行路径在编译目录；4 秒后按 PID 停掉。
  - 排除：`paddock ctl`／`install-skills` 在 `main.rs` 一开头就分支退出，不碰 GPUI；一边连续跑 4 秒 `paddock ctl instances` 一边反复查登记，始终只有主窗口一个。全量 `cargo test` 跑的整个过程里反复查登记，没有任何测试程序登记成应用（`tests/ctl_cli.rs` 只跑命令行）。Browser 带出来的 “AutoFill (paddock)” 是系统的 `BackgroundOnly` 助手，不进 Dock。重装 app（删掉再复制）会让 Dock 认不出正在跑的那个，但那样多出来的是 paddock 自己的图标，和用户看到的 shell 图标对不上。
  - 主控查的时候没有残留的测试实例（只有 `~/Applications/paddock.app` 那一个）。
- 改法建议（未做，等用户定）：
  - A（主控建议）：`PADDOCK_NO_ACTIVATE` 设了时，启动后调 GPUI 的 `cx.set_activation_policy(ActivationPolicy::Accessory)`，测试窗口不进 Dock（GPUI 现成接口；照样能按窗口编号截图，要实测确认）。只影响测试和截图起的实例，用户自己开的 paddock 不变。
  - B：只靠规矩——起测试窗口的人用完立即按 PID 停（AGENTS.md 已有），但截图那几秒里照样会闪一下。
  - 不带 `PADDOCK_NO_ACTIVATE` 的 `cargo run`（少见）照样会出现，A 管不到。
- 没做的事：没改程序。只改文档，`git diff --check` 过。
