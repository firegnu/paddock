# 任务：paddock ctl 换专用运行目录；ctl 起不来时照常启动、只提示

2026-10-07，paddock/main 自己做（急修）。
类型：Bug 修复
依据：用户 10-07 退出后 paddock 打不开：Browser 的 WebKit 先在 `$TMPDIR/paddock` 建了 0755 的缓存文件夹，ctl 要求同名文件夹 0700 而拒绝启动，`main.rs` 又在 ctl 启动失败时退出整个程序。本轮只改这两处。
依赖：P5-39b
提示：依据证据定位并修复导致问题的原因，保持无关行为不变。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-39d-ctlfix`，分支 `p5-39d-ctlfix`（已从 main 建好）。

## 要做的
用户 10-07 同意主控的两条：
1. ctl 的运行目录换成专用的 `paddock-ctl`（`$XDG_RUNTIME_DIR/paddock-ctl`，否则 `$TMPDIR/paddock-ctl`），服务端和命令行用同一处，不再和 WebKit 的 `paddock` 文件夹共用；权限和身份检查照旧。
2. ctl 起不来时 paddock 照常启动，左下角提示“paddock ctl 不可用”和原因（界面文字英文）；其他功能不受影响。

## 怎么算做完
- 两条达到；测试：运行目录名、`$TMPDIR/paddock` 已被别人以 0755 占用时 ctl 照常在 `paddock-ctl` 起来；ctl 启动失败时不退出（能测的部分抽成函数测）。`app/` 下 `cargo test --all-targets`、clippy、fmt。帮助文字、方案文档、DESIGN 里的目录名一起改。

## 完成记录

- 做了什么：`control.rs` 运行目录名改为常量 `DIR = "paddock-ctl"`（服务端和命令行共用），拆出 `runtime_dir_in(base)` 便于测试；`main.rs` 中 `Server::start()` 失败时不再退出，记下原因、`NewShell.instance` 为空；`window.rs` 打开窗口后用侧栏的 `note` 红字显示 `paddock ctl unavailable: <原因>`；`window_ctl.rs` 没有 ctl 时不给 shell 注入身份。帮助文字、方案文档、DESIGN 里的目录名一起改。
- 验证了什么：新测试 `a_browser_cache_folder_named_paddock_does_not_stop_ctl`（同一临时根下已有 WebKit 式 0755 的 `paddock`，ctl 照常在 `paddock-ctl` 起来、不动 `paddock`），把目录名临时改回 `paddock` 时它在“运行目录必须 0700”处失败，改回后通过。`app/` 下 `cargo test --all-targets`（425 项）、clippy、fmt 都过。实跑两次（临时 HOME、假 corral、`PADDOCK_NO_ACTIVATE=1`）：一、临时根里预放 0755 的 `paddock`，paddock 启动、`paddock-ctl` 为 0700、`paddock ctl instances` 找到它；二、临时根 0755 让 ctl 起不来，paddock 照常打开，左下角红字提示原因（截图看过）。
- 没做的事：用户机器上旧的 `$TMPDIR/paddock`（WebKit 缓存，另一个会话改成了 0700）和里面残留的旧套接字不动，不影响使用。
