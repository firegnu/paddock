# 任务：paddock ctl 的界面端——inspect、open、close、browse 在窗口里生效

2026-10-07，paddock/main 交给 paddock/dev-ctlui（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查要 / 影响面：碰要害（主控定：和 P5-39a 同属一件，关闭含运行 shell 的窗格和调用者身份是要害）
类型：功能变更
依据：本轮把 P5-39a 留下的界面接口接上，让命令真正在窗口里生效；不改传输层和命令行的约定（确有必要先停下来报告）。
依赖：P5-39a
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」。
- `docs/DESIGN.md` §3 第 5 步、§13「paddock ctl（P5-39）」。
- `docs/调研/P5-39-paddock-ctl方案.md`（第 2 节命令、第 4 节安全边界）。
- `docs/任务/P5-39a-ctl传输层与命令行.md` 末尾「完成记录」（给你的接口：`Server::process_pending(handler)`、处理者签名、`Records::update`／`get`／`values`、`Caller`、`Place`、`CloseTarget`）和 `docs/任务/P5-39a-审查.md`（交叉审查对接口的约定，如“返回 busy 前不能执行修改”）。
- Saddle（只读，`../saddle`，提交 `f7d1bbaf`）：`src/app_control.rs`（界面端的做法：调用者定位、`inspect` 的输出、`open` 的三种内容和相对位置、`close` 的确认凭据、`busy`、异步状态更新）、`skills/saddle/SKILL.md`（对外约定）。只参照，对着 paddock 重写。
- paddock：`app/src/control.rs`、`main.rs`（现在的 `unsupported` 处理者）、`window.rs`（窗口、标签、分屏、New Agent 的“开好再放到指定位置”、关闭窗格的确认、拖分隔线和对话框等“用户正在操作”的状态）、`layout.rs`、`browser.rs`／`browser_view.rs`（打开网址、本地地址补全）、`viewer.rs`／`pty.rs`（shell 启动和环境）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-39b-ctlui`，分支 `p5-39b-ctlui`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-39b-ctlui`。

## 要做的
用户 10-05：“这个也要迁移。”10-07：“继续吧。弄完再说。都做到这份上了”。
1. **调用者定位**：用 P5-39a 转发的身份认出“调用者自己的窗格”：corral 开的 agent 按 `CORRAL_NAME` 加 `CORRAL_INSTANCE`（实例不符就当认不出），paddock 开的 shell 按 `PADDOCK_INSTANCE` 加 `PADDOCK_PANE`。窗格换了内容以后，旧身份不能再指向它（照 Saddle 的 revision 思路，需要就给 shell 注入的身份带上版本）。认不出时，依赖“自己位置”的操作报错，不猜。
2. **给 shell 注入身份**：paddock 开的每个 shell 带上 `PADDOCK_INSTANCE`、`PADDOCK_PANE`（以及需要的版本）；不改 agent 的环境（它们走 corral）。
3. **inspect**：返回实例、当前标签和窗格、所有标签和窗格（种类、显示的 agent 或 shell 目录、分屏布局）、`caller.pane`（认得出时）。
4. **open**：`--place tab|left|right|up|down`，默认相对调用者自己的窗格，`--relative-to active` 或窗格 ID 可改；三种内容互斥：`--shell [--cwd]`（默认取来源窗格已知的目录）、`--agent NAME`（已在别处显示就挪过去，不重复接入）、`--name NAME [--cwd] [--role] -- PROGRAM ARG…`（经公开的 `corral start` 新开，复用 New Agent 的“开好再放”）；`--prompt` 只在显式给出时带上；默认不抢焦点，`--focus` 才切过去。异步的步骤先回 `accepted`／`starting`，完成或失败后用 `Records::update` 更新（`complete` 只表示显示好了）。
5. **close**：`--pane`／`--tab` 只断开 paddock 的显示，不停 agent。目标里有在跑的 shell 时，先回 `confirmation_required`、受影响的清单和一次性凭据，什么都不关；带 `--confirm-shells --confirmation 凭据`（和新的请求 ID）再来才关；目标变了凭据作废。
6. **browse**：在右侧栏 Browser 打开 http／https 网址（本地地址照 P5-28c 补全），右侧栏没开就打开它并切到 Browser；`--focus` 才把键盘交给网页。
7. **busy**：用户正在拖分隔线、开着对话框或确认框、New Agent 窗口正在创建时，修改类命令回 `busy`，且回之前不做任何改动（P5-39a 交叉审查约定）。
8. 不加任何发按键、读屏、停 agent、读网页内容的能力。

## 怎么算做完
- 上面八条达到。
- 验证（碰要害一档）：
  - 测试（不起真窗口能测的部分抽成纯函数测）：调用者定位（agent 身份、shell 身份、实例不符、窗格换了内容后旧身份失效、认不出）、open 的位置计算（五种 place、relative-to self／active／ID）、`--agent` 已显示时挪动而不重复接入、close 的确认流程（有 shell 先要确认、凭据一次性、目标变了作废、确认后才关）、busy 时不改动、browse 只收 http／https。边角：同时两个修改请求、窗格在请求处理中被关掉。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 实测一次（不模拟按键鼠标）：release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`／`XDG_RUNTIME_DIR`（短路径）、假 corral，起一个 paddock，用编出来的 `paddock ctl` 依次跑 inspect、open（shell 到右边、新开一个假 agent 到新标签）、browse 一个 `http://127.0.0.1:端口`（自己起的临时服务）、close 一个带 shell 的窗格（先拿到确认要求，再带凭据关），每步截一张自己的窗口看过；按 PID 停掉。截图放 scratchpad，不入库。
- 写完后主控会请另一个 Codex 做只读交叉审查。

## 不要做
- 不改 P5-39a 的传输层和命令行约定（需要改先停下来报告）；不改技能文字（P5-39c）。
- 不碰用户真实的配置、布局文件、运行目录、技能目录、`~/.local/bin`，不连用户正在运行的 paddock 实例。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys/pause，不 attach 上去打字；测试只用假 corral。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不加依赖；不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-39b-ctlui` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
