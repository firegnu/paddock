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

## 完成记录

### 做了什么

- 从 Saddle `f7d1bbafc84102edfef49b47bf49d5df90e2b2e3` 迁入传输层和 CLI，文件头保留来源与差异；无新增依赖。每实例随机 ID／独立 Unix socket，运行目录取 `$XDG_RUNTIME_DIR/paddock`，否则 `$TMPDIR/paddock`；检查私有基目录和 paddock 目录的当前用户归属、0700，拒绝符号链接或权限不符，不替用户修权限。socket 为 0600，客户端连接前也检查归属、类型和权限，路径达到 macOS 104 字节边界时拒绝并提示使用较短的私有 XDG 目录。
- 保留 64 KiB 消息上限、2 秒读写超时、500 ms 连接等待、4 个工作线程、8 个候选连接、32 项界面队列和 1 秒界面回复等待。超时返回 `uncertain` 与原 ID；自动 ID 在进入界面队列前生成。socket 本身作为实例登记，探测不可连接的残留并忽略；正常退出删除本实例 socket，等待线程结束，不删其他实例文件。
- 请求账本由 `control` 管理，最多 256 条且不淘汰；相同 ID／参数／身份重放返回原结果，不重复执行；改变参数或身份返回 `request_conflict`。记录已满拒绝新修改，原记录仍可查。界面返回 `busy` 时不消费 ID；状态由界面处理结果写入，后续可更新。
- 程序入口在清理继承环境、读取 GUI 配置及启动窗口之前分流 `ctl`／`install-skills`，保留并发送四个调用者身份变量。实现 JSON 帮助、退出码 0／1，以及方案中的六条 ctl 命令。`browse` 复用 Browser 本地地址补全和已有 GPUI 重导出的 URL 校验；原始 JSON 也只接受带 http／https scheme 的地址。
- `main.rs` 接上监听、GPUI 线程顺序消费与退出清理；`inspect`／`open`／`close`／`browse` 统一返回 `unsupported`（暂不支持），修改的失败结果可用 `request` 查询。`instances` 不依赖界面处理者。
- `install-skills` 内嵌占位技能，固定写 `.claude/skills/paddock/SKILL.md` 和 `.agents/skills/paddock/SKILL.md`；支持预览、列文件确认、`--yes`、`--remove`，只覆盖／删除有归属标记的文件。外来文件、二进制文件、符号链接及非普通文件报告并跳过；确认后再次检查归属，避免按过期计划覆盖外来文件。

### 验证了什么

- 保留真实 RED → GREEN：原迁入逻辑擅自修正 0755 目录；请求账本缺少同 ID 冲突保护；CLI 尚不识别 `browse`；技能安装逻辑会覆盖无标记文件；程序入口尚不识别 `ctl`；原始 JSON 的非 web 地址能到达处理者。分别用对应行为断言复现，再修复通过；编译错误不算 RED 证据。
- 新增 15 个 control 单测、3 个技能安装单测、3 个真实 CLI 子进程测试，覆盖目录／socket 权限、外来目录归属、符号链接／占位文件、路径过长、坏消息／超长消息／未知字段、读超时／界面超时、满队列、256 条记录与重放／冲突／状态更新、三种 open 内容互斥、browse 的 scheme 限制、两个实例与残留发现／清理、技能安装 dry-run／拒绝／确认／归属标记／移除和环境身份转发。全部使用独立临时夹具，未使用真实 agent 或用户技能、运行目录。
- 在 `app/` 下、独立 `.target/p5-39a-ctl` 中完成 `cargo test --all-targets`（最终 376 项全部通过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`；`git diff --check` 通过。上游 `block 0.1.6` 仍有已有的 future-incompatibility 提示，无本次 Clippy 警告。
- CLI 身份测试夹具最初在 macOS 临时目录多嵌套一层，触发真实路径长度拒绝；改用短的私有 `/tmp/pctl-<随机值>`，未放宽产品边界。没有启动桌面窗口，GPUI 启停接线经编译检查，传输服务启动／Drop 清理用无窗口测试验证。

### 拿主意的地方与 P5-39b 接口

- 选择有界通道，界面不接触 socket。`Server::start()` 返回实例 `id`；`main.rs` 的 GPUI 前台任务每 10 ms 调用 `Server::process_pending(handler)`，每轮最多消费 32 条，按入队顺序处理。P5-39b 将当前 `control::unsupported` 替换为通过 `main.update(...)` 操作 `PaddockWindow` 的闭包即可。
- 处理者签名为 `FnMut(&Message, &Records) -> serde_json::Value`，必须返回 JSON 对象；传输层处理 `instances`、`request`、ID 重放／冲突／容量，处理者负责其余命令的目标定位、用户忙碌状态、shell 关闭确认与实际界面动作。返回值放 `ok`、`state`、`pane`、`revision` 等业务字段；传输层补齐 `instance`／`request_id`。异步动作先返回 `accepted`／`starting`，完成后在界面线程调用 `server.records.update(id, value)`；该方法保留请求身份。`Records::get(id)` 查询原结果，`Records::values()` 只读遍历已有结果，供关闭确认 token 查找及目标检查。
- 协议中的 `Caller` 只带 `name`、`corral_instance`、`paddock_instance`、`pane`，缺失或解析不了的字段留空；不带 Saddle revision。P5-39b 注入 shell 的实例／pane，并按公开 Corral 身份验证调用者。`Place` 仅有 tab／left／right／up／down，`CloseTarget` 仅有 pane／tab。
- 实例发现走独立 `instances` 消息，不把未接入的 `inspect` 当作实例存活条件。CLI 遇多个实例且有 Corral 身份时再逐个 `inspect` 尝试解析自己的 pane，无法唯一定位则返回 `ambiguous_instance`；不按时间或名字猜。
- 沿用 Saddle 不淘汰请求的规则及进程内幂等边界，重启后不承诺原记录存在。残留 socket 忽略而不主动清掉。没有改变已批准的设计，也没有需要主控另行决定的事项。

### 没做的事

- 未改 `window.rs`，未实现界面动作、shell 环境注入、正式技能正文；分别留给 P5-39b／P5-39c。未运行真实 GUI 交互或真实 agent 实测。
- 未修改 Saddle／ranch，未增加或升级依赖，未改 Cargo 清单／锁文件；未安装技能到用户目录、未卸载 Saddle 技能、未打包安装应用、未改全局链接。
- 只在本任务分支提交；未合并 main、未推送。交叉审查留给主控另行安排。
