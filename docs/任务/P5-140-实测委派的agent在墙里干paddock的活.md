# 任务：实测委派的 agent 套 sandbox 后能不能干 paddock 的活

2026-10-19，paddock/main 自己做（用户：“P5-140可以做了”）。
执行：主控
依据：用户 10-19 定下「主控不套墙，它派出去的 agent 可以在 paddock 里勾选改成套墙」（见 P5-141），用户选了做法甲：“选甲，写草稿吧”。主控指出 paddock 现有的派活流程在墙里有几处走不通，要先实测，P5-141 的勾才有意义。

## 为什么要先实测
`fence check --profile dev`（10-19 本机）给委派 agent 的规则：能写的只有自己的 worktree、启动时的临时目录、`~/.cargo/registry`、`~/.cargo/git`、主仓库 `.git` 的对象和本分支的 ref、这一家自己的状态目录；HOME 其余不能读；网络只放行这一家的模型服务和 crates.io。对照 paddock 的规矩，可能卡住的地方：
1. **编译目录**：规矩要求 `CARGO_TARGET_DIR=…/paddock-worktrees/.target/<分支>`，在仓库外。fence 只在**它自己启动时的环境**里有 `CARGO_TARGET_DIR` 才放行这个目录（`${target}`，10-19 实测 `CARGO_TARGET_DIR=… fence check` 会把它加进能写的）。派活时现在是在每条 cargo 命令前临时加，fence 看不到，`cargo build` 会写不进。要验的是：`corral start --sandbox dev --env CARGO_TARGET_DIR=…` 能不能让 fence 放行它。
2. **交叉审查记录**：现在审查 agent 只写主仓库工作区的 `docs/任务/P5-xxx-交叉审查.md`；墙里主仓库只放行 `.git`，写不进。
3. **其余要顺手看的**：编译、`cargo test --all-targets`（测试里起子进程、建 PTY、用临时目录）、`clippy`、`fmt`、在 worktree 里 `git commit`；读 `~/.cargo/registry/src` 里 GPUI 源码；Claude Code、Codex、pi、omp 四家各自能否照常起来（Codex 有 `weaker_network_isolation` 警告）；agy、amp 不支持套墙。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-140-fence-test`，分支 `p5-140-fence-test`（已从 main 建好）。测试 agent 在这里干，分支只用来试提交，测完连同编译目录 `.target/p5-140-fence-test` 一起删，不合并。

## 做法
- 自己开测试 agent `paddock/test-fence`（`--label role=test`），在一个临时 worktree、临时分支上套 `--sandbox dev` 起，让它照上面逐项跑一遍，用 `fence denials` 看被拦了什么；测完 `corral stop`，清掉 worktree、分支、编译目录。不碰用户的 agent。
- 根据结果定 paddock 的规矩怎么改（改 `CLAUDE.md` 和派活任务书模板，例如起 agent 时用 `--env` 传编译目录、交叉审查记录写在审查 worktree 里再由主控搬回），改动另写进本件或另开一件。
- 第 1 条若 `--env` 传不进 fence，属于 corral／fence 的事，写成需求交给 ranch，paddock 不自己绕。

## 主控定的（用户说可以做时没另外定）
- 先测 Claude Code，全部项目都走一遍；再测 Codex 的编译、提交、写主仓库三项。pi、omp 在 ranch R17b 已实测能在墙里照常干活，不重复。
- 本件只实测、出结论和建议的规矩改动；改 `CLAUDE.md`、派活模板等用户同意后再做。

## 规矩
- 只调 `corral`、`fence` 公开命令；不读 `~/.fence/`、corral 内部文件；不改 ranch。
- 测试 agent 只用自己开的 `paddock/test-*`，用完就停。

## 怎么算做完
- 用户原话：“选甲，写草稿吧”。本件交付：实测结果和建议的规矩改动，写在本文件末尾，给用户看。

## 实测结果（paddock/main，10-19）
测试 agent 都套 `--sandbox dev`、带 `--env CARGO_TARGET_DIR=…/.target/p5-140-fence-test`，在临时 worktree `p5-140-fence-test` 上跑；用完已停。

**Claude Code（`paddock/test-fence`，sonnet）：能干活，有两处卡。**

| 项 | 结果 |
|---|---|
| 仓库外编译目录（`--env CARGO_TARGET_DIR`） | 能写（ranch/main 也实测确认：`--env` 会传给 fence 的 check 和 run，必须是绝对路径） |
| `cargo build`、`clippy -D warnings`、`fmt --check` | 都过 |
| `cargo test --all-targets --no-fail-fast` | 28 个目标里 4 个有失败，共 18 个测试：`control::transport_tests` 13、`ctl_cli` 3、`agent_shell` 1、`remote` 1。全是测试在写死的 `/tmp/…` 建目录被拒（`app/src/control_tests.rs:5`、`app/tests/ctl_cli.rs:17`、`app/tests/agent_shell.rs:595`、`app/tests/common/mod.rs:47`）；墙里的 `TMPDIR` 是 `~/.cache/fence/tmp/<实例>`。不是代码对错 |
| 读 `~/.cargo/registry/src` 的 GPUI 源码 | 能 |
| worktree 里 `git commit` | 能；但每次报 `Unable to create <主仓库>/.git/packed-refs.lock: Operation not permitted`（不挡提交），`git count-objects` 报 `<主仓库>/.git/worktrees/<wt>/refs` 是 garbage |
| 写主仓库工作区 `docs/任务/` | 被拦（预期） |
| `corral ls` | 被拦，退出码 127（`~/.local/bin/corral` 在 HOME 禁读里），不是 CONTRACT 说的 6 |
| 连 github.com | 被拦（预期）；crates.io 通 |
| 读 `~/.ssh/config`、`~/Documents` | 被拦（预期） |

fence 记录里另有：Claude Code 写 `~/.claude.json` 的锁和临时文件被拦十几次；zsh heredoc 往 `/private/tmp/zsh*` 写临时文件被拦（agent 用 `cat <<EOF` 写多行提交说明会失败）；npm 更新检查被拦（噪音）。

**Codex（`paddock/test-fence-codex`，codex-cli 0.163.0）：起不来。** 两次都是起来 1–2 秒退出码 1（带不带 `--yolo --disable memories` 一样），原因只在终端里、读不到。已交 ranch/main 查。

## 结论和建议（等用户定）
1. **派出去的 Claude Code 现在就能套墙干 paddock 的活**，前提是起 agent 时加 `--env CARGO_TARGET_DIR=<绝对路径>`。Codex 等 ranch 查清再用；pi、omp 照 R17b 能用；agy、amp 不能套。
2. **18 个测试在墙里必失败**（写死 `/tmp`）。建议另开一件 paddock 的小活：这几处测试改用 `TMPDIR`（墙里是 `~/.cache/fence/tmp/<实例>`，路径够短，放得下 unix socket），在 `TMPDIR` 太长时才退回 `/tmp`。不建议用 `.fence.toml` 放行整个 `/tmp`。
3. **交叉审查记录**：墙里写不进主仓库工作区。套墙的审查 agent 改成把意见写在自己 detached 审查 worktree 里的同名文件，主控审完搬回主仓库（或只写在回复里）。
4. **规矩怎么落**：起 agent 时带 `--env CARGO_TARGET_DIR`，写进 `CLAUDE.md`「独立编译目录」和派活约定（只在套墙时需要，不套墙也无害）。这几条改动等用户同意。
5. **交给 ranch 的**（已发 ranch/main，不挡 paddock）：Codex 起不来；heredoc 的 `/tmp` 临时文件；`~/.claude.json` 写不了；墙里 `corral` 退出码 127 而不是 6；`packed-refs.lock` 报错和 worktree 下的 `refs` garbage。
