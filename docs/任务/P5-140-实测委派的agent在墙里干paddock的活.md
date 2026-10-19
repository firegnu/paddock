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
