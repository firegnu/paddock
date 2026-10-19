# 任务：测试在 sandbox 里也能跑：短临时目录不写死 `/tmp`

依据：P5-140 实测（10-19）：套 `--sandbox dev` 的 agent 在 paddock 里跑 `cargo test --all-targets --no-fail-fast`，18 个测试失败，全是测试在写死的 `/tmp/…` 下建目录被拒（墙里只放行 fence 给的 `TMPDIR`，`~/.cache/fence/tmp/<实例>`），和代码对错无关。用户 10-19 问“接下来是干啥？”，主控提出先做这件。
依赖：P5-140
执行：主控
（理由：只改 4 处测试辅助函数，主控对原因和验证方法都清楚；验证要再开一个套墙的测试 agent，主控来开更顺。）

## 现在知道的
- 写死 `/tmp` 是故意的：这些测试要建 unix socket（paddock ctl 的控制口），macOS 的 socket 路径最长 104 字节，`/var/folders/…/T/` 这种常规 `TMPDIR` 太长（`app/tests/common/mod.rs` 的 `short_tempdir` 注释写明了）。
- 4 处：`app/src/control_tests.rs:5` 的 `directory()`（13 个测试）、`app/tests/ctl_cli.rs:17` 的 `sandbox()`（3 个）、`app/tests/agent_shell.rs:595`（1 个）、`app/tests/common/mod.rs:47` 的 `short_tempdir()`（`remote` 的 1 个，以及别的用它的测试）。
- 墙里的 `TMPDIR` 是 `/Users/<用户>/.cache/fence/tmp/<12 位实例>`，约 45 字节，比 `/tmp` 长，但多数情况下还放得下 socket。

## 要做的
1. 这 4 处先照旧在 `/tmp` 下建；**只有在 `/tmp` 下建不了（权限被拒）时**，才退回 `TMPDIR` 下建。墙外的行为一点不变。
2. 退回 `TMPDIR` 后路径太长、放不下 socket 时，测试要报清楚原因（例如 “TMPDIR too long for a control socket”），不要只是 unwrap 出一个看不懂的错。
3. 能共用的就共用一个辅助函数（`tests/common` 给集成测试用；`src/control_tests.rs` 是单元测试，用不到 `tests/common`，按现有结构处理，不为此加公开接口以外的东西）。
4. 和祖先目录权限有关的测试（如 `runtime_below_non_sticky_writable_ancestor_is_refused`）在两种位置下都要仍然测到它要测的东西；如果退回 `TMPDIR` 后某条测试的前提不成立，说明清楚，不放宽断言。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-142-test-tmp`，分支 `p5-142-test-tmp`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-142-test-tmp`（从 `.target/main` 克隆）。

## 怎么算做完
- 墙外：`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 照旧全过。
- 墙里：开测试 agent（Claude Code 和 Codex 各一个，`--sandbox dev --env CARGO_TARGET_DIR=…`，`--label role=test`）在这个 worktree 跑 `cargo test --all-targets --no-fail-fast`，全过；用完 `corral stop`。两家墙里看到的 `TMPDIR` 不一样（Claude Code 是 `~/.cache/fence/tmp/<实例>`，Codex 是 `/tmp/claude`，见 P5-140 补测），都要过。
- 用户原话：“接下来是干啥？”；上面两条是主控提的，用户 10-19：“都同意，先做P5-142”。

## 不要做
- 不用 `.fence.toml` 或 `~/.fence/local.toml` 放行 `/tmp`（墙会变松）。
- 不改测试要测的东西，不删测试、不放宽断言、不缩短超时。
- 不碰用户的 agent；测试 agent 用完就停。

## 实测记录（paddock/main，10-19，还没做完）
改动在 `156ca33`：4 处辅助函数先在 `/tmp` 建，权限被拒（墙里是 EPERM，Rust 归为 `PermissionDenied`）才退回 `TMPDIR`，建好后检查 `目录 + /paddock-ctl/<16 位>.sock` 不到 104 字节，否则报 `TMPDIR too long for a control socket`。`tests/common` 加 `short_dir`，`ctl_cli`、`agent_shell`、`short_tempdir` 共用；`control_tests.rs` 是单元测试，照同一规则自己写一份。

- 墙外：`cargo test --all-targets` 792 过，clippy、fmt 干净。
- 墙里（Claude Code `paddock/test-tmp-cc`，`TMPDIR=~/.cache/fence/tmp/<实例>`；Codex `paddock/test-tmp-cx`，`TMPDIR=/tmp/claude`）：两家结果一样。临时目录都能建了；`remote` 1 个、`ctl_cli` 2 个、`control` 1 个由失败变过。**还剩 14 个失败，全是建 unix socket（`UnixListener::bind`／`Server::start_in`）报 `Operation not permitted`**：`control::transport_tests` 12、`ctl_cli` 1（`caller_identity_is_forwarded_before_gui_environment_cleanup`）、`agent_shell` 1（`shim_requests_its_own_pane_handoff_and_waits_for_the_listing_check`）。fence 记录是 `network-bind <TMPDIR>/…/<id>.sock`：dev 档案不许在任何目录建 unix socket（`fence check` 的 `settings.network` 里没有放行 unix socket 的项），和放在哪个目录无关，paddock 改测试解决不了。
- 另有一个实测时自己造成的坑：给测试 agent 用的编译目录是从 `.target/p5-142-test-tmp` 克隆的，源码没变时 cargo 不重编，集成测试里 `CARGO_BIN_EXE_paddock` 仍指向原目录的 paddock，墙里读不了，`agent_shell` 7 个报退出码 126。agent 在自己的编译目录 `cargo clean -p paddock` 重编后这 7 个都过。P5-141 的 `.fence.toml` 放行整个 `paddock-worktrees/.target` 后也不会再碰到。
- 测试 agent 已停，`.target/p5-142-cc`、`.target/p5-142-cx` 已删。

## 完成记录（先合并，墙里验收待 ranch）
- 用户 10-19：“选1，先合并，需求写好转给ranch”。即：剩下 14 个由 ranch 放行墙里建 unix socket（只限 agent 自己的临时目录）来解决，paddock 的改动先合并。
- 做了什么、验证了什么：见上面「实测记录」。墙外三项全过；墙里临时目录两家都能建了，剩 14 个都是建 unix socket 被拒。
- 拿主意的地方：退回条件只认权限被拒，别的错误照旧报；`control_tests.rs` 不为共用加公开接口，照同一规则单写；`short_tempdir` 改用随机名（原来用进程号加计数，要先删旧目录），和另外 3 处一致。
- 没做完的：墙里两家全过这一条，等 ranch 放行、部署后，主控再开两家测试 agent 验一次，结果补在这里，过了再记收尾。
