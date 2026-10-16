# 任务：在 paddock 里加载远程机器上 corral 托管的 agent（甲层）

2026-10-16，paddock/main 交给 paddock/dev-remote（Claude Code，opus[1m] / high）。
路由：重 / 交叉审查要 / 影响面：碰要害（路由：重、要、碰要害；推翻：强度按用户 10-16 “effort用high，不要用xhigh”用 high）
类型：功能变更
依据：用户 10-14：“再记一个draft task，我要能在paddock中加载远程的corral托管的agent”。调研 P5-90a 已合并；用户 10-15 定了本机和远程隔开（下文「用户定的」）；ranch R10 已在用户的 Linux 机器上装好 corral（10-16）。本轮做甲：远程 agent 看得到、能 attach 打字、读回复、收件箱能回、能停和暂停恢复；不做在远程开 agent（乙）和远程的 git、Changes 等（丙）。
依赖：P5-90a
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
优先：高
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`（规矩全文）。
- 本文件全部，尤其「用户定的」「要做的」「不要做」。
- `docs/任务/P5-90a-远程corral调研和方案.md` 的「调研结论」§1（paddock 哪些地方调 corral、读本机）、§3（传输、合并调用、引号、attach、测试）、§4.1（配置）、§4.4（断线和重连）、§5（远端要什么）。§4.2 的 `<机器>:<名字>` 和 §4.3 的「机器分区」已被用户 10-15 的定法取代，见下文。
- ranch 的公开约定只读：`corral guide`、`../ranch/docs/DESIGN.md` §3「在另一台机器（Linux）上装和升级」。不读 ranch 以外的东西，不改 ranch。
- 代码：`app/src/config.rs`、`corral.rs`（`Client`、`Poller`）、`viewer.rs`（attach 和 attach 前的核对）、`pty.rs`、`sidebar.rs`、`agents.rs`、`window.rs`、`window_inbox.rs`、`attention.rs`、`inbox.rs`、`pause.rs`、`new_agent.rs`（`stop`）、`diagnostics.rs`、`layout_state.rs`、`right_panel.rs`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-90-remote`，分支 `p5-90-remote`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-90-remote`（已从 `.target/main` 克隆）。
- 动 `app/src/` 下要动的文件、新增远程传输模块（比如 `remote.rs`）、相关测试、`docs/DESIGN.md` 和本文件。不加依赖；要加先停下来报告。

## 用户定的（照做，不改）
- 用户 10-15：“本地的agents和远程的agents隔离开，远程的就是远程的，本地的就是本地的。开agent和左侧agents显示区页隔离开”；“这样远程的也不会破坏现在本地的。”
- 用户 10-16：“不能本地corral托管的agents的东西，我理解其实就是两个隔离的东西，一个接入的是本地，一个接入的是远程”（主控理解为：远程不能影响本机 corral 托管的 agent）。
- 用户 10-15 定的四条：
  1. 一个窗口里切换机器：左侧栏顶上一条切换栏（`This Mac │ studio ●2`，没配远程机器时不出现）；切到哪台，左侧栏、窗格都换成那台的，每台机器各记一套窗格布局。远程视图里右侧栏的 Changes、Kanban、Cairn 只写一句说明，不读本机。
  2. 切走以后，那台机器的窗格保持连着，切回来马上能用。
  3. 收件箱和铃铛合在一起：所有机器的都列出来，标上机器名，点一下切过去。
  4. New Agent 在哪台机器的视图里开就开到哪台；这是乙层，甲先只做本机。
- 用户 10-16 同意：停、暂停/恢复并进甲；远端已有窗口接着同一个 agent 时，远程改成只读接入，本机照旧拒绝。
- 远端是什么（用户 10-15）：“远端一般不会常开，远端一般就是要么linux，要么是vps之类的”。

## 要做的
1. **配置**：`~/.config/paddock/config.toml` 加 `[[machines]]`，用户手写、paddock 只读（照 P5-90a §4.1）：`name`（paddock 里的叫法；小写字母、数字、`-`，不重复，不能叫 `local`）、`ssh`（交给 ssh 的目标：`~/.ssh/config` 里的别名或 `user@host`；不空、不含空白和控制字符、不以 `-` 开头）、可选 `corral`（远端 corral 的路径，开头 `~` 换成远端 `$HOME`）。配置错了照现有配置的报错方式说清楚，不影响本机。
2. **传输**（新模块）：
   - 只用系统的 `ssh`，登录全听用户自己的 ssh 配置；paddock 不读、不复制 `~/.ssh` 下的任何文件，不存凭证。
   - 每台机器一个 paddock 自己的复用连接：控制套接字放在短目录（如 `/tmp/paddock-<uid>/`，0700，检查属主、不是链接，路径不超过 macOS 的 104 字节上限）；命令行带 `ControlMaster=auto`、`ControlPersist`、`BatchMode=yes`、`ConnectTimeout`、`ServerAliveInterval=5`、`ServerAliveCountMax=3`，目标前加 `--`；不碰用户自己的 master。
   - 找远端 corral：配置里写了就用它；否则连上后在 ssh 跑命令的那个 shell 里 `command -v corral`，找不到试 `$HOME/.local/bin/corral`（ranch 装的位置；R10 实测远端非交互 ssh 的 PATH 里没有 corral，要靠这一步）。找到后记下绝对路径，之后直接调；再跑一次 `corral --version`，`contract` 不是 `"1"` 就标出来、不读列表。不在远端套交互 shell（`-i`），也不自己拼 PATH。
   - 远端命令每个参数按 POSIX 单引号转义（已有依赖 `shell-words`）；连上时做一次引号自检（带 `'`、`$()`、换行、中文的一段字原样回来），不对就把这台标成不支持。收件箱里用户打的任意文字也走这一条。
   - ssh 的错误分成几类给界面用：认证失败、host key 不认识、连不上、找不到 corral、contract 不对。
3. **轮询**：每台远程机器一个自己的轮询，约 2 秒一轮，一轮最多两次远端调用（`ls` 一次，各 agent 的 `status` 合成一条远端命令再按行拆开；或者每台一条常驻的远端 shell，二选一，在完成记录里说为什么）。名字照样加引号。
4. **隔离（验收重点）**：本机那份照现在不变：本机的列表、窗格、状态读取、每秒刷新、收件箱、看板、Changes、Cairn、Activity、workspace、按仓库分组、paddock ctl、`paddock --attach`、Settings 的行为都不变。远程的轮询和 ssh 不在界面线程、不和本机共用会被卡住的东西；远程断线、超时、变慢、报错都不能拖慢或卡住本机的刷新和界面。远程 agent 不进本机的 git 摘要、按仓库分组、workspace、Activity、Kanban 的 agent 认领（`seen()`、`agent_for`、`controller_for`）、New Agent 的目录候选、paddock ctl。没配 `[[machines]]` 时，界面和现在一模一样。
5. **切换栏**：配了远程机器时，左侧栏顶上一条切换栏：`This Mac`、各远程机器名；每台写要用户看的 agent 数（同现有铃铛的口径）和连接状态（在线一个点；`Connecting…`、`Reconnecting in 8s`、`Can't sign in`、`Host key unknown`、`corral not found`、`corral contract 2, needs 1` 这类，悬停写原因和要用户做的事，比如 `Run ssh <host> once in Terminal`；点连接出错的机器立即重试）。切到哪台：左侧栏换成那台的 agent 列表（远程的按名字前缀分组，读不到仓库），窗格区换成那台的窗格布局。每台机器的布局分开保存，重启后照本机的办法恢复，远程的等那台连上后再 attach。切走的机器窗格保持连着。
6. **远程视图里**：
   - 右侧栏 Changes、Kanban、Cairn 只写一句说明（比如 `<agent> runs on <machine>; this tab reads this Mac only.`，文案你定，英文），不读本机。左侧栏下面的 Activity 不显示。
   - New Agent 不开到远程（乙层）：在远程视图里点 New Agent 写明 `New agents start on This Mac only for now`（或类似英文），不要悄悄开到本机。
   - 卡片、窗格标题、命令面板照现在画；路径不按本机的 `~` 缩写（连上时问一次远端 `$HOME` 再缩写也可以）。命令面板只列当前机器的 agent。
7. **attach**：远程窗格在本机 PTY 里跑 `ssh -tt -e none … -- <host> <corral 绝对路径> attach <名字>`。
   - **走那台机器的复用连接**（`-S <控制套接字>`），带上面的保活参数：用户 10-16 实测清单 4，直接连和复用连接下 attach 都能打字、拖窗口改大小画面都跟着变（“现在看起来都是好的”）。
   - attach 前的核对照本机：实例对得上；`attached` 不是 0 时，**远程改成只读接入**，窗格顶上写 `Read-only: attached elsewhere`，前一个走了自动能打字（corral 本来如此，P5-90a §4.4）；**本机照旧拒绝**。
   - ssh 自己断了（退出码 255）：窗格写 `Lost connection to <machine> — reconnecting…`，不让打字；那台机器重新读到、agent 还在、实例没变，就自动重接（pen 会重放画面）；实例变了或不在了，照本机「agent 结束」处理。
   - 关窗格照现在（SIGINT，3 秒后 SIGKILL），远端 agent 不受影响。
8. **断线**：那台机器读不到时，它的 agent 留着上次的样子、画暗、写多久以前看到的，不算“消失”、不关窗格。自动重连退避：1 秒起翻倍，最多 60 秒左右，唤醒后立即试一次；后台不弹认证框。
9. **收件箱和铃铛**：所有机器的合在一起，每条标机器名（本机的不标，或标 `This Mac`，你定），点一下切到那台再定位到那个 agent；就地回一句走那台的 `send`。某台读不到时，按机器各出一行（照现在 `corral` 那一行，`attention.rs`）。
10. **停、暂停/恢复**：远程 agent 的停、暂停、恢复走那台的 corral；确认框里写上机器名。全部暂停（一键暂停）只管当前视图那台还是全部，你定，在完成记录里写明，确认框写清楚。
11. **Diagnostics**：每台远程机器一行：连没连上、corral 的路径、version 和 contract、最近一次错误。
12. `docs/DESIGN.md` 写进这些决定（§13 加一条 P5-90）。

## 怎么算做完
- 用户原话：“我要能在paddock中加载远程的corral托管的agent”；“本地的agents和远程的agents隔离开，远程的就是远程的，本地的就是本地的。开agent和左侧agents显示区页隔离开”；“不能本地corral托管的agents的东西……一个接入的是本地，一个接入的是远程”。上面「用户定的」四条和 10-16 的两条逐条达到，「要做的」1–12 逐条达到。
- 测试（照「测试不依赖真实 agent」）：用假 `ssh` 脚本（解析参数，把远端命令交给本机 `sh -c`）配上现有的假 `corral` 脚本，不连任何真机器。至少覆盖：
  - 配置校验（`ssh` 以 `-` 开头、含空白、重名、叫 `local`）；
  - ssh 参数拼法（`--`、保活、控制套接字路径长度）；
  - 引号：带 `'`、`$()`、换行、中文的名字和收件箱回复原样送到远端；
  - 找 corral：PATH 里没有时用 `$HOME/.local/bin/corral`；contract 不对时不读列表；
  - 一轮最多两次远端调用，输出按行拆对；
  - **隔离**：远程的假 ssh 卡住不返回或报错时，本机的列表照样按时刷新；没配机器时和现在一样；远程 agent 不进本机的分组、workspace、看板认领；
  - attach 前的核对：远程 `attached` 不是 0 时只读接入，本机照旧拒绝；
- 验证只做这些：在 `app/` 下带上面的 `CARGO_TARGET_DIR` 跑 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`，再跑 `git diff --check`。觉得不够，在回复里说，不要自己加。
- 用户 10-16：“不要测试任何断网的测试。”断线、断网、重连只做功能，不写这方面的测试。
- 不起窗口、不截图、不连真机器。真机器上的实测由主控合并前请用户在场时做。

## 不要做
- **不连任何真实的远程机器**，不跑 `ssh <主机>` 登录任何机器；不读 `~/.ssh` 下的任何文件（包括 `config`、`known_hosts`、私钥）。主机名、地址不写进仓库。
- 不读本机或远程 corral 的内部状态目录，只走公开命令；不改 ranch，要 corral 改的写下来报告。
- 不做乙（在远程开 agent）、丙（远程的 git、分支、改动、Changes、Kanban、Cairn、Activity）。
- 不动本机那份的行为（见「要做的」第 4 条）；要改本机共用的代码才做得成时，改了以后本机的行为、测试结果必须不变，完成记录里写明改了哪些共用的地方。
- 不调 `corral start`／`stop`／`send`／`keys`；`corral ls` 里的都是用户的 agent，不碰。
- 不加依赖；不用 Python；不依赖 ratatui、crossterm；不起 paddock 窗口；不用脚本模拟按键或鼠标；不碰别的仓库。
- 命令里不用 `rm`，不用 `sh -c` 包长命令（测试里的假 ssh 脚本除外），前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录。测试起的进程用记下的 PID 停。
- 不照抄 Zed 的 GPL 代码，也不照抄 herdr 的代码（只借 P5-90a 里记的思路）。
- 遇到：现有结构要大改才能把本机和远程隔开、要加依赖、要改 ranch、GPUI 画不出上面的样子，停下来报告，等决定，不要自己换别的办法。
- 不合并到 main，不推送。只在 `p5-90-remote` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

---

以下是这件活的来历，供参考。

## 现状（主控 10-14 查的，只读）
- paddock 只和本机的 corral 打交道：调的是配置里的那一个 `corral` 程序（`app/src/config.rs` 的 `corral`，默认就是 `corral`，启动时也能用 `--corral` 换），读列表、状态、回复、开和停都走它（`app/src/corral.rs` 的 `Client`）；窗格里显示 agent 是在本机起一个 `corral attach <名字>`（`app/src/viewer.rs`）。一个窗口只有一个 corral 来源。
- corral 自己没有“远程”这回事。
- paddock 里有不少东西是按 agent 的启动目录去读**本机**文件的：左侧栏卡片上的分支和改动数、按仓库分组（P5-80）、workspace 归属（P5-89）、右侧栏的 Changes、Kanban、Cairn，左侧栏下面的 Activity。

## 调研时参考（用户 10-15）
- 用户：“我觉得你调研的时候可以看下herdr的实现，他也支持remote agent了”。调研结论见 P5-90a。
- herdr 是分开的（主控 10-15 核对 herdr 文档 `connecting-machines.mdx`）：一个窗口里在左侧栏选机器，选中的那台接收输入和终端大小、提供窗格画面；workspace、窗格、agent 名字都按机器各自独立；没选中的机器后台只更新 agent 状态和通知。

## 远端（主控记）
- 主控 10-15 只读查了 ranch：corral 有 Linux 的代码分支。ranch R10（10-16）在用户的 Linux 机器上编译、修了 Linux 上的测试和 clippy 问题、打包、装好：`~/.local/bin/corral` 指向 ranch 版本目录，contract `"1"`。R10 从本机经非交互 ssh、用绝对路径实测过：pi、omp 都不在那个 PATH 里也能 start（corral 用 `-l -i` 重建环境）；状态 idle → working → idle；send 回 confirmed；reply 拿到原文；wait 返回 idle；pause 时进程停住、send 报退出码 10，resume 后接着跑；`ssh -tt -e none <host> $HOME/.local/bin/corral attach <名字>` 看得到画面、能打字、Ctrl-] 能退，退出后 `attached` 回到 0；名字 80 个字符能起，81 个报 `path_too_long`。
- R10 的提醒：本机这头必须是真终端；用管道喂输入去 attach 时，远端拿到的窗口大小是 0×0，什么都画不出。
- VPS 的 sshd 一般是用户自己管的，P5-90a §5 建议的 `ClientAliveInterval` 值得设，合并时提醒用户。

## 实测（10-15，主控在用户的 Linux 机器上，用户允许）
- 用户定的规矩：那台机器上只拿 pi、omp 做实验，任何情况下都不运行那里的 Claude Code 和 Codex；pi、omp 也不许委派任何活。主机名、地址不写进仓库。
- 机器：Linux x86_64，登录 shell 是 zsh。
- 清单第 1 步：`ssh -o BatchMode=yes <host> true` 退出码 0，免密能登。
- 清单第 2 步（当时）：远端没装 corral；pi、omp 只在交互 shell 的 PATH 里，ssh 直接跑命令时找不到。
- 清单第 3 步（经公网）：不复用连接一条命令 5.4 秒；复用连接每条 0.78–1.0 秒，一次远端调用里跑 10 条也是 0.94 秒；一条常驻远端 `sh` 里一问一答 0.39–0.42 秒。结论：一定要复用连接；远程按 2 秒左右一轮；attach 里打字的回显会慢 0.4 秒左右，是网络本身的。
- 清单第 4 步（10-16，用户跑）：直接连和复用连接下 attach 都正常，改窗口大小画面都跟着变；所以 attach 走复用连接。
- 清单第 5 步（断网）：不做。用户 10-16：“不要测试任何断网的测试。”断线判定用 `ServerAliveInterval=5`、`ServerAliveCountMax=3`。

## 完成记录

### 做了什么
- 配置（`config.rs`）：读 `[[machines]]`（`name`、`ssh`、可选 `corral`），按要求校验；写错的那一条不进 paddock，左侧栏底部写一句原因，其余配置和本机照常；没写时 `machines` 为空、什么都不说。
- 传输（新 `remote.rs`）：`Link` 一台机器一条 paddock 自己的复用连接（`/tmp/paddock-<uid>/`，0700、查属主、不是链接、套接字路径守住 104 字节；`-S`、`ControlMaster=auto`、`ControlPersist=600`、`BatchMode=yes`、`ConnectTimeout=10`、`ServerAliveInterval=5`、`ServerAliveCountMax=3`、目标前 `--`）。连上时引号自检 → 找 corral（配置的、`command -v corral`、`$HOME/.local/bin/corral`）→ `corral --version` 查 contract。错误分成认证失败、host key、连不上、找不到 corral、contract 不对、shell 不支持、本机起不来 ssh，各有切换栏的几个字和悬停说明。
- 轮询：`Poller::remote` 每台一个线程，`Rounds` 管节奏（在线 2 秒一轮、一轮两次远端调用；退避 1 秒起翻倍到 60 秒；Mac 唤醒后、要求刷新时立即试）。
- 窗口（`window.rs`）：每台机器一套左侧栏、布局、窗格（`Seat`），显示的那台在原来的字段里，别的停在 `seats` 里保持连着、照样处理结束的 agent 和断线重接；切换栏在左侧栏顶上（窄条时竖排）。布局按机器另存（`layout_state.rs` 的 `Board`），重启后远程窗格等那台读到再 attach。
- 远程视图：右侧栏 Changes／Kanban／Cairn 一句说明；没有 Activity、New Workspace；New Agent、New Shell、Shell／Agent shell 写 `… on This Mac only for now.`；命令面板和窗格标题按那台的 `$HOME` 缩写。
- attach（`viewer.rs`、`view.rs`）：远程走 `ssh -tt -e none -S …`；`attached` 不是 0 时远程只读接入（窗格顶上 `Read-only: attached elsewhere`，只剩这一个时自动能打字），本机照旧拒绝；ssh 退出 255 写 `Lost connection to <机器> — reconnecting…`、不收键盘，那台再读到同一实例自动重接。
- 断线：左侧栏里那台的 agent 画暗、错误行写 `read <多久> ago`，不关窗格。
- 收件箱和铃铛（`inbox.rs`、`window_inbox.rs`）：所有机器合在一起，草稿、已发、被拒、发送中按（机器，名字）记；远程行标机器名；`Open pane` 先切到那台；回复走那台的 `corral send`；读不到的机器各一行。
- 停、暂停恢复走显示着那台的 corral，确认框写机器名。
- Diagnostics 加 `Machines` 一节，每台一行。
- `docs/DESIGN.md` §13 加了 P5-90 一条。

### 验证了什么
- 在 `app/` 下带 `CARGO_TARGET_DIR=…/.target/p5-90-remote`：`cargo test --all-targets` 全过（691 个），`cargo clippy --all-targets -- -D warnings` 无警告，`cargo fmt --check` 通过；`git diff --check` 通过。
- 新测试：`tests/remote.rs`（假 `ssh` 解析参数后把命令交给本机 `sh -c`，假 corral 在假 home 的 `.local/bin`）：ssh 参数（`-S`、保活、`--` 在目标前）；控制目录（0700、改回 0700、别的用户、链接）和套接字长度；引号（带 `'`、`$()`、反引号、换行、中文、`{a,b}`、`!`、`^`）原样到 shell；PATH 里没有 corral 时找到 `~/.local/bin/corral`、配置的 `~` 换成那边 home；contract 不对时不读列表；一轮恰好两次远端调用、名字原样到 corral、状态按分隔行拆对；收件箱回复原样到远端；ssh 错误分类；远程假 ssh 卡住或报错时本机的列表照样按时刷新、停得下来；远程 `attached` 非 0 只读接入、本机照旧拒绝。`remote.rs` 单元测试（找 corral 的顺序、拆分、退避、错误分类）；`config.rs`（校验和没写时为空）；`tests/layout_state.rs`（按机器存、没有机器时不写这一项、坏一份不影响别的）。
- 没起窗口、没截图、没连真机器。切换栏、只读条、说明文字、画暗的样子没看过，留给合并前用户在场实测。

### 拿主意的地方
- 轮询选“一轮两次调用”而不是常驻远端 shell：实测两次调用 1 秒左右已够 2 秒一轮；常驻 shell 要自己管生死和输出边界，断线时更难收拾。
- 引号没用 `shell-words::quote`（它让 `{}`、`!`、`^` 不加引号），自己写了总加单引号的版本。
- 一键暂停只管显示着的那台：它的按钮本来就只看得到这台的 agent；要问时确认框加一行 `These agents run on <机器>; only they pause.`。
- 本机的收件箱行不标机器名，远程的标。
- 远程视图里不开 shell（New Shell、面板里的 Shell／Agent shell 写一句说明、面板不列）：shell 只能开在本机，放进远程那套窗格会把两边混起来。
- ~~看着远程时 `paddock ctl` 除 `inspect` 外回 busy~~：主控审查不接受，已返工，见下面「返工」。
- 启动时总在 This Mac；`ControlPersist=600`；切换栏、只读条、说明的英文文案。
- 配置里暂时去掉的机器，它在 `layout.json` 里那份留着，加回来时还在。

### 改了本机共用的地方（本机行为和原有测试结果不变）
- `corral.rs`：`Client` 加 `remote`（`Client::local` 建本机的），`json`／`collect`／`send` 按它分到本机命令或远端；`answer`／`checked`／`merge` 从原来的方法里提出来（逻辑不变）；`Poller` 的循环提成 `spawn`，本机仍是每 `every` 读一次、要求刷新时立即读。
- `viewer.rs`／`view.rs`：`Viewer::with`（`Viewer::new` 照旧收本机程序名）、`read_only`、`lost`、`wait`、`listed`；attach 前核对只在远程时放过 `attached` 非 0。
- `sidebar.rs`：`Sidebar::new` 多一个 `remote`（本机传 `None`，Git、workspace、Activity 照旧）；铃铛加上别的机器的；切换栏只在有远程机器时出现。
- `inbox.rs`：行多一个 `machine`（本机为 `None`），`Open::new`／`refresh` 走同一套按机器的实现。
- `window.rs`／`window_inbox.rs`／`window_ctl.rs`：上面说的座位、收件箱按机器、ctl 和退出提问只看本机；本机的 `view`／`fill`／`ended`／`save_layout` 等走原来的路。
- `pause.rs`／`new_agent.rs`：加 `run_with`／`stop_with`，原函数照旧。`layout_state.rs`：`machines` 为空时不写进文件。`diagnostics.rs`／`settings_view.rs`：加 Machines 一节，`diagnostic` 的标签改收 `impl Into<SharedString>`。

### 没做的事
- 乙（在远程开 agent）、丙（远程的 git、Changes、Kanban、Cairn、Activity）。
- 按用户 10-16，断线、断网、重连没写测试，只做了功能。
- “远程 agent 不进本机的分组、workspace、看板认领”没有界面层的自动测试：GPUI 窗口没有测试环境，这一条靠结构保证（每台一套 `Sidebar`，看板认领和 New Agent 候选只取本机那套），测试里覆盖了轮询互不拖累和没配机器时配置、布局文件不变。
- 没有真机器实测；用户 VPS 的 sshd 设 `ClientAliveInterval`（P5-90a §5）合并时提醒用户。

### 返工（主控审查 10-16：看着远程时 ctl 不能回 busy）
- 改了什么：去掉 `ctl_busy` 里“看着远程就回 `looking at another machine`”那一条，busy 又只在用户手上正有事时（拖分隔线、开菜单、面板、提问等，和原来一样）。座位的账提成纯数据的 `seating.rs`（`Seating`：哪台在字段里、别的各停在哪），`window.rs` 的 `here`／`seats`／`swap_seat` 换成它。`on_local` 原来只把字段对调、`here` 不动，所以看着远程时新开的窗格会按远程那台来做；现在它真的把本机那套请进字段（`here` 暂时是 0），`view`／`fill`／New Agent 晚到结果的放置都按本机走，做完再把远程那台请回来。
- `--focus` 选了“照常完成、不切换”：看着远程时，`--focus` 只让那个窗格成为本机那套的当前窗格，键盘留在用户眼前的远程窗格（`visiting` 期间 `focus_active` 不动键盘，只同步左侧栏和存布局）；用户切回 This Mac 时，`switch` 照常把键盘给本机的当前窗格。理由：ctl 多是 agent 发起的，替用户切走眼前的机器，就是本机的 ctl 反过来打断用户在远程的查看；而且键盘若跟着跳进一个看不见的本机窗格，用户正在打的字会落进去（P5-2 那类事）。
- `corral start` 晚到的结果：`control_tick` 不再因看着远程而 busy，结果一到就放进本机那套布局，请求照常回 `attaching`。
- 关窗格（`close`）：被关的本机窗格这时不在眼前、不会有键盘，`close_quietly` 照旧把键盘放回原处。
- 测试：`seating.rs` 加 `this_macs_ctl_works_on_its_own_seat_while_another_machine_is_on_show`（远程在字段里时把本机请进来，`here` 为 0、字段是本机那套、远程那台停着；空闲时 `busy_reason` 为 `None`、开着菜单仍是 busy；请回后复原）和 `bringing_the_seat_in_the_fields_or_none_changes_nothing`。`ctl_busy` 本身要 GPUI 窗口才能调，GPUI 的测试支持没开、开它要改依赖，所以测试落在它依据的两块纯代码上；这条测试是随新模块写的，没有在旧代码上先跑出失败。
- 跑了：`cargo test --lib seating`（2 个过）、`cargo test --lib control_ui`（19 个过）、`cargo clippy --all-targets -- -D warnings` 无警告、`cargo fmt --check` 通过。照主控说的没跑全套。
- 上面「拿主意」里 ctl 回 busy 那条作废；`docs/DESIGN.md` §13 P5-90「隔离」一条同步改了。
