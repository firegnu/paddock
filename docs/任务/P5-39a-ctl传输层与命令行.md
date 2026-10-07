# 任务：paddock ctl 的传输层和命令行（照搬 saddle ctl），加 install-skills

2026-10-07，paddock/main 交给 paddock/dev-ctl（Codex，常规：gpt-6-astra / high）。
路由：常规 / 交叉审查要 / 影响面：碰要害（路由：交叉审查要（安全 0.97），影响面碰要害；档拿不准，主要是照搬，定常规）
类型：功能变更
依据：本轮做传输层、命令行、请求记录和 install-skills，界面端只留接口（下一件 P5-39b 接）；技能文字下一步由主控写（P5-39c），这里先放占位。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」（尤其“与 Saddle 分开”：迁入的代码开头注明 From Saddle 和提交，不加 saddle 依赖；不改 Saddle 仓库）。
- `docs/DESIGN.md` §3 第 5 步、§13 最后一条「paddock ctl（P5-39）」。
- `docs/调研/P5-39-paddock-ctl方案.md` 全文（命令、实现、安全边界）。
- Saddle（只读，`../saddle`，记下当前提交号写进文件头）：`src/control.rs`（传输层）、`src/control_cli.rs`（命令行）、`src/app_control.rs`（界面端，只看它从传输层要什么，不搬）、`skills/saddle/SKILL.md`（命令的对外约定）。
- ranch 的 `crates/corral/src/skills.rs`（只读，`../ranch`）：`install-skills` 的确认、`--dry-run`、`--remove`、只认自己写的文件的做法，照这个写。
- paddock：`app/src/main.rs`（启动参数）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-39a-ctl`，分支 `p5-39a-ctl`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-39a-ctl`。
- 另有一件活（P5-38）在改 `window.rs` 的标签栏，你不要动 `window.rs`；需要接线的地方留给 P5-39b。

## 要做的
用户 10-05：“这个也要迁移。”10-07 看过方案：“配，saddle 技能也卸掉，按你的方案派出去做吧”。
1. **传输层**（迁入 `control.rs`）：每个运行中的 paddock 实例一个私有 Unix 套接字。目录用 `$XDG_RUNTIME_DIR/paddock`，没有时用用户私有临时目录（macOS 的 `$TMPDIR`）下的 `paddock/`；目录 0700、套接字 0600，建之前核对目录归属和权限，不对就拒绝。注意 macOS 套接字路径长度上限。JSON 消息，有长度上限、超时和队列上限，照 Saddle。实例 ID、实例登记和清理（进程不在了的残留要能认出并忽略／清掉）照 Saddle。
2. **请求记录**：每次修改有 `request_id`（可由调用方预先给），记录状态（accepted、starting、complete、failed、target_invalid、uncertain、busy 等，照 Saddle），最多 256 条，满了拒绝新修改；同一 ID 不同参数报冲突；`request` 查询。
3. **命令行**：paddock 程序加 `ctl` 子命令，不起窗口，发一条消息、打印 JSON 就退出。命令和参数照方案第 2 节：`inspect`、`instances`、`open`（`--place tab|left|right|up|down`、`--shell [--cwd]`／`--agent NAME`／`--name NAME [--cwd] [--role] -- PROGRAM ARG…`、`--relative-to`、`--focus`、`--prompt`、`--request-id`、`--instance`）、`close`（`--pane`／`--tab`、`--confirmation`、`--confirm-shells`）、`request`、**`browse URL [--focus]`**（只收 http／https）。`--help` 也输出 JSON。退出码照 Saddle。
4. **调用者身份**：从环境读 `CORRAL_NAME`、`CORRAL_INSTANCE`（corral 开的 agent）和 `PADDOCK_INSTANCE`、`PADDOCK_PANE`（paddock 开的 shell，注入由 P5-39b 做），随请求发给界面端；认不出就不带，不猜。
5. **界面端接口**：窗口一侧起监听，收到的请求按顺序交给界面线程处理，处理者是一个留给 P5-39b 实现的接口（trait 或通道，你定，写清楚）。本件里界面端对 `inspect`／`open`／`close`／`browse` 统一回“暂不支持”，`instances`／`request` 由传输层自己答；在 `main.rs`／`app` 里接上监听的启动和退出时的清理（不碰 `window.rs`）。
6. **`paddock install-skills`**：照 corral 的做法，把技能写到 `~/.claude/skills/paddock/SKILL.md` 和 `~/.agents/skills/paddock/SKILL.md`：先列出要写的文件、确认（或 `--yes`）、`--dry-run`、`--remove`；文件里带“由 paddock install-skills 写入”的标记，只覆盖或删除带这个标记的文件，别人的同名文件报告并跳过。技能文字编进程序，来源文件放 `app/resources/skills/paddock/SKILL.md`，本件先写一份简短占位（P5-39c 主控改写）。不碰 `saddle` 技能（卸载由用户自己做）。

## 怎么算做完
- 上面六条达到。
- 验证（碰要害一档）：
  - 测试：目录／套接字权限和归属检查（不对时拒绝）、超长和坏消息、超时、队列满、请求记录（预给 ID、重复 ID 同参数／不同参数、满 256 条）、命令行参数解析（含 `browse` 只收 http／https、三种 open 内容互斥）、实例清理、install-skills（dry-run 不写、确认后写、带标记才覆盖或删除、别人的文件跳过）。测试用临时目录，不碰用户真实的技能目录和运行目录。
  - 边角：套接字路径过长、目录被别人占用或权限过宽、同时两个实例。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
- 写完后主控会请另一个 Codex 做只读交叉审查。

## 不要做
- 不改 Saddle 仓库，不加 `saddle` 依赖；迁入文件开头写“From Saddle `src/…` at commit `<号>`”和改了什么。
- 不改 `window.rs` 和界面；不实现 `inspect`／`open`／`close`／`browse` 的界面行为（P5-39b）。不加发按键、读屏、停 agent 一类命令。
- 不碰用户真实的 `~/.claude/skills`、`~/.agents/skills`、`~/.local/bin`、运行目录；不运行 install-skills 写真实目录。
- 不要用 `osascript`、System Events 模拟按键或鼠标；不启动用户正在用的 paddock。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys/pause，不 attach。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不加依赖（Saddle 这几个文件用到的库 paddock 已有就用；没有时停下来报告）。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-39a-ctl` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话，并写清给 P5-39b 的接口。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
