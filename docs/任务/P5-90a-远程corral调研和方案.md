# 任务：远程 corral 的 agent 怎么接进 paddock：调研和方案

2026-10-15，paddock/main 交给 paddock/dev-remote-research-1（Claude Code，重：opus[1m] / xhigh）。
路由：重 / 交叉审查不要 / 影响面：看得见（路由：重 0.99，交叉审查要（安全隐私 0.83、并发 0.78），影响面碰要害；推翻：本轮只读、不改代码，产出是一份方案交用户定，按「只读研究」算看得见、不交叉审查；方案里涉及的 ssh 凭证和并发，到实现那件再按碰要害审）
类型：设计
依据：P5-90 草稿（`docs/任务/P5-90-加载远程corral的agent.md`）的第一步。用户 10-14：“我要能在paddock中加载远程的corral托管的agent”；10-15 问先开哪件时：“我有点想开90.但是觉得有点复杂。”主控建议先只调研、定范围，再决定做到哪一层；用户：“我觉得你调研的时候可以看下herdr的实现，他也支持remote agent了”；远程指什么：“自己的另一台机器，ssh 能连上。”本轮只出方案，不实现。
执行：派出
提示：围绕已知约束给出推荐及关键取舍；有实质备选时再比较，不凑方案数量。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- 仓库根的 `CLAUDE.md`（规矩，尤其「与 Saddle 分开」「Corral 只走公开命令」「不复制 Zed 的 GPL 代码」）。
- `docs/任务/P5-90-加载远程corral的agent.md`：用户原话、主控查到的现状、开工前要定的六个问题、调研参考、不做。
- `docs/DESIGN.md` §3（和 Saddle 生态只通过公开约定打交道）、§13 里 P5-80（按仓库分组）、P5-87（收件箱）、P5-105（workspace 只按仓库收）。
- ranch 只读：`/Users/firegnu/Developer/personal_projs/ranch/docs/CONTRACT.md`（corral 的公开命令和 JSON 输出）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-90a-remote-research`，分支 `p5-90a-remote-research`（已从 main 建好）。
- 只写本文件（追加结论）。不改代码。
- herdr 源码：放进 scratchpad 或 `/tmp` 下一个新建的空目录（例如 `git clone --depth 1 https://github.com/herdrdev/herdr.git`，或 Homebrew 用的 `v0.9.5` 源码包），只读，不在里面跑它的构建或脚本。读之前核对仓库里的 LICENSE（Homebrew 标的是 Apache-2.0）。本机装着 `herdr` 0.9.5，可以跑 `--help` 这类只看帮助的命令。

## 要做的
用户要的：在 paddock 里看得见、用得上**用户自己另一台机器（ssh 能连上）**上 corral 托管的 agent。不改 corral 和 ranch（用户 10-15：“是指不改corral，调用公开命令没问题”），只在 paddock 里做，经 ssh 调远程机器上 corral 的公开命令。

查清楚并写出：
1. **paddock 现在哪些地方调 corral、读本机**：按模块列（`corral.rs`、`viewer.rs` 的 attach、左侧栏、收件箱、New Agent、看板、Changes、Activity、Cairn、workspace 归属等），每处说清楚：经 ssh 调远程 corral 能不能照样用；依赖本机目录的，远程 agent 怎么办（藏起来、写明是远程的、还是要在远程跑 git 再拿回结果）。
2. **herdr 的远程怎么做的**（`--remote <ssh-target>`、`herdr machine`、`--machine`、`--remote-keybindings`、`--handoff`）：怎么存机器、怎么起连接（每个命令一次 ssh，还是常驻连接、ControlMaster 之类）、远程终端怎么接到本机、终端大小、断线和重连、延迟、登录凭证放哪。哪些思路 paddock 能借、哪些不适合（它是终端多路复用器，paddock 是 GPUI 窗口加 corral）。只借思路，不照搬代码。
3. **分层方案和推荐**：照 P5-90 草稿的甲（看、attach 打字、状态和回复、收件箱）、乙（加开停）、丙（加目录相关）三层，每层要动 paddock 的哪些地方、大概多大、风险在哪；推荐先做哪层、为什么。
4. **P5-90 草稿六个问题里，现在能给出建议的**：机器怎么配（存在哪、写什么）、远程和本机重名、左侧栏怎么看出是远程的（和 workspace、按仓库分组怎么摆）、断线怎么显示和重连。每条给推荐和关键取舍，交用户定。
5. **远程机器那边要什么**：装 corral 的哪个版本（本机是 `corral` 0.1.0、contract 1）、PATH、ssh 配置；版本不一致时 paddock 怎么发现。

## 怎么算做完
- 上面五项都有结论，写进本文件末尾「## 调研结论」：区分**查证的事实**（附代码位置或命令输出）、**推测**和**未知**；最后一段「推荐」，三五条，交用户定。
- 验证只做这些：读代码、读文档、读 herdr 源码、跑只看帮助的命令（`herdr --help`、`corral --help`、`ssh -G localhost` 这类）。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 paddock 的任何代码、配置、DESIGN；不改 ranch、corral、Saddle。
- **不连用户真实的远程机器**，不跑 `ssh <主机>` 去登录任何机器，不读 `~/.ssh` 里的私钥（`~/.ssh/config` 也不读：里面有用户的主机名）。要实测的，写成“请用户跑”的命令清单。
- 不起 herdr 的 server、不 attach，不在 herdr 源码目录里跑构建或脚本。
- 不对 `corral ls` 里现有的 agent 做 `stop`、`send`、`keys`、attach；不用 corral 开 agent。
- 不起 paddock 窗口、不截图。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录，一条命令能把它们全杀掉。
- 结论里不写用户的主机名、用户名、IP（仓库是公开的），用 `<host>` 代替。
- 不合并到 main，不推送。只在 `p5-90a-remote-research` 上提交。

## 做完
在本文件末尾追加「## 调研结论」（在你的分支里提交），再追加「## 完成记录」：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写推荐那几条，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 调研结论

2026-10-15，paddock/dev-remote-research-1。记号：**【实】**查证的事实，后面附代码位置或命令输出；**【推】**推测，没实测；**【未】**不知道，要用户实测或回答。paddock 的路径写 `app/src/…`，ranch 的写 `../ranch/…`；herdr 的写 `herdr:…`，指 scratchpad 里浅克隆的 `herdrdev/herdr`（main `916fcf8`，2026-10-14，`Cargo.toml` 版本 0.9.5，`LICENSE` 是 Apache-2.0）。herdr 只读，没在里面构建、没跑它的脚本。

总的判断：只在 paddock 里做，经系统自带的 `ssh` 调远程 corral 的公开命令，走得通，corral 和 ranch 都不用改。难处不在调不调得到，在三处：
- 一轮刷新现在要调 N+1 次 corral，经 ssh 必须复用连接、合并调用；
- paddock 里处处按 agent 名字认人，远程和本机会重名；
- 好几样功能拿 agent 的目录在本机跑 git，远程 agent 的目录在本机**可能恰好也存在**（两台 Mac 用户名相同、目录布局一样），会读出本机仓库的东西。所以要按“哪台机器”排除，不能靠“目录不存在”。

### 1. paddock 现在哪些地方调 corral、读本机

调 corral 的入口：
- 【实】多数调用走 `corral::Client { program }`，`json()` 就是 `command::run(program, args)` 再读一行 JSON（`app/src/corral.rs:88-113`）。左侧栏的 `Poller` 每隔 `refresh_ms` 读一轮（默认 1000，`app/src/config.rs:73`）：先 `corral ls`，再对每个 agent 各调一次 `corral status`（`corral.rs:119-150`），也就是**每秒 N+1 个进程**。`Replies` 读 `corral reply`（`corral.rs:155`），收件箱回复走 `corral send`（`corral.rs:168`）。
- 【实】也有不经 `Client`、直接拿程序名拼命令的地方：
  - attach 的命令行 `[corral, "attach", name]`（`app/src/viewer.rs:230`）；
  - `new_agent::send_first` 的 `command::run(corral, ["wait", …])`；
  - paddock ctl 的 `ls`（`window_ctl.rs:130`）；
  - `pause.rs:79`；
  - `integrations.rs` 的 `corral integration …`；
  - `diagnostics.rs:85` 的 `corral --version`；
  - agent shell 里注入的 `PADDOCK_AGENT_CORRAL`。

  要做远程，这些都得收到同一个“在哪台机器跑”的入口。

逐项看（“经 ssh”指把同一条 corral 命令放到远端跑）：

| 地方 | 现在怎么做 | 经 ssh 能不能照样用 | 远程 agent 怎么办（建议） |
|---|---|---|---|
| 列表、状态、卡片（`sidebar.rs`、`agents.rs`、`card.rs`） | `Poller` 读 `ls` 和 `status` | 能，输出一样；N+1 次要合并（§3） | 照常显示，放进机器分区（§4.3） |
| 认人：`agents.rs:19-45` 的 `selected`、`expanded`、`unread`、`first_seen`、回复缓存，workspace 名单，保存的布局 | 全按 agent 名字 | — | 会重名串号，改用带机器的名字（§4.2） |
| agent 消失就关窗格（`window.rs:1862` `SidebarEvent::Alive`） | 这一轮没读到的名字，它的窗格关掉 | — | 一台机器读不到时，它的 agent 不算“消失”，要留着 |
| 窗格 attach（`viewer.rs:226-260`、`pty.rs`） | 本机 PTY 里跑 `corral attach <名字>`；attach 前用 `corral status` 核对实例，还要 `attached == 0`，否则报 `agent attached elsewhere`（`viewer.rs:238-251`）；改大小用 `master.resize`；断开先发 SIGINT，3 秒后 SIGKILL（`pty.rs:186-240`） | 能：同一个本机 PTY 里改跑 `ssh -tt -e none … <host> corral attach <名字>`（§3） | 照常；断线另外处理（§4.4） |
| 收件箱（`window_inbox.rs:340`、`inbox.rs`、`attention.rs`） | `reply` 读、`send` 回；corral 读不到时出一行 `corral`（`attention.rs:103-107`） | 能 | 读不到的那行按机器各出一条 |
| 停、暂停/恢复（`window.rs:2127` 调 `new_agent::stop`，`pause.rs:79`） | 各一条命令 | 能 | 乙；改动很小，可以并进甲 |
| New Agent（`new_agent.rs`、`new_agent_view.rs`） | `corral start … --cwd <目录>`，`~` 用**本机**的 `HOME` 展开（`new_agent.rs:446,790`）；目录候选取各 agent 的 cwd（`sidebar.rs:1321` → `window.rs:2050`）；`Choose folder…` 是本机的选目录框；模型表读本机的 `codex debug models`（`models.rs`）；插件提示读本机的 `corral integration status` | `start` 本身能，周边都是本机的 | 乙；甲里远程 cwd 不能进本机的目录候选 |
| 卡片上的分支和改动数（`git.rs` 的 `Poller`，`sidebar.rs:1394` `git.watch(cwds)`） | 拿 agent 的 cwd 在本机跑 git | 不能（要在远端跑 git，属于丙） | 不显示。要**按机器排除**：远程 cwd 在本机恰好存在时，会显示本机仓库的分支 |
| 按仓库分组（P5-80，`agents.rs:403` `group()`） | 用上面 git 读到的主仓库，读不到时按名字前缀 | — | 甲里按名字前缀分组 |
| workspace 归属（P5-89、P5-105，`agents.rs:328`、`workspaces.rs:211`） | 看主仓库；读不到仓库时，名字前缀和哪个 workspace 里仓库的目录名相同，就跟进那个 workspace | — | 远程 agent 一律不进 workspace。否则远程的 `paddock/main` 会按前缀进到本机放着 paddock 的那个 workspace |
| 左侧栏下面的 Activity（`activity.rs`，`sidebar.rs:1396`） | 本机 `git log` | 不能 | 排除远程的 cwd |
| 右侧栏 Changes、Kanban、Cairn（`window.rs:2864` `follow()` → `window.rs:5949-5960` 的 `Frame { cwd }`） | 跟着焦点窗格的 cwd 读本机 | 不能 | 焦点在远程窗格时不传 cwd，写一句说明，比如 `<name> runs on <machine>; this tab reads this Mac only.`（文案到时再定） |
| Kanban 认 agent（`kanban.rs:1820` `agent_for` 先认 `task=` 标签再认 worktree 路径；`kanban.rs:1851` `controller_for` 按 cwd 前缀认主控） | 用左侧栏的全部 agent（`sidebar.rs:1294` `seen()`） | — | 排除远程。否则远程带 `task=P5-xx` 的 agent 会挂到本机看板同编号的任务上 |
| 命令面板（`search.rs:130`）、卡片上的路径（`card.rs:535`） | 写出 cwd，`~` 按本机 HOME 缩写 | 能显示 | 加上机器名；远程路径不缩写（远程 HOME 不知道，或者连上时问一次） |
| 保存的布局（`layout_state.rs:53` `Content::Agent { name, cwd, instance }`） | 按名字恢复 attach | — | 名字带上机器就行。校验只要求不空、不以 `-` 开头、不含空白（`layout_state.rs:171-176`），`<机器>:<名字>` 能通过 |
| paddock ctl（`window_ctl.rs`、`control*.rs`）、`paddock --attach`（`main.rs:209-219`）、Settings › Integrations | 本机 | — | 甲里只管本机，不动 |
| Diagnostics（`diagnostics.rs:85`） | 本机 `corral --version` | 能 | 每台机器加一行：连不连得上、corral 在哪、版本和 contract |

### 2. herdr 的远程怎么做的

下面这些是查证过的：herdr 的文档（`docs/next/website/src/content/docs/connecting-machines.mdx`、`persistence-remote.mdx`）和源码。

- **怎么存机器**：
  - `herdr machine add <ssh 目标> [--label] [--remote-session]` 写到 `<状态目录>/client/endpoints.json`（`herdr:src/client/endpoint/catalog.rs:373`）。每台只记 `id`（不透明）、`label`、`target`、`session`、`enabled`（`catalog.rs:19-27`）；目标里带密码会被拒（`catalog.rs:62-67`）。另有一份丢了也不要紧的缓存，记远端的系统和可执行文件路径。
  - 文档原话：“Herdr does not store passwords, private keys, agent tickets, or SSH control sockets in the catalog. Authentication stays with OpenSSH.”
  - `machine status` 检查连接时不会要求认证；`machine reconnect` 在终端里认证一次。
- **怎么起连接**：
  - 调系统的 `ssh`，没用 Rust 的 SSH 库。每台机器一条常驻的 ssh 通道，在远端跑 `herdr remote-client-bridge`（或 `remote-api-bridge`），把 stdio 转到远端 herdr server 的 unix socket（`herdr:src/remote.rs:15-33`、`src/remote/attach.rs:5401`）。通道用 `-T`，远端不要终端。
  - 连接复用：
    - 私有目录 `/tmp/hssh-<uid>/`，权限 0700，检查属主、不能是链接；文件名是哈希加 `%C`，还给 OpenSSH 绑定时用的临时名留了 17 字节，免得超过 unix socket 的长度上限（`herdr:src/platform/unix_common.rs:666-715`）。
    - `ControlMaster=auto`、`ControlPersist=600`（`attach.rs:1414-1424`）。注释写明 “never stop or unlink a master belonging to the user's SSH setup”。
  - 生成一份临时 ssh 配置：先 `Include` 用户的 `~/.ssh/config` 和 `/etc/ssh/ssh_config`，再补上 `ServerAliveInterval 15`、`ServerAliveCountMax 4`，这样用户自己写的值优先（`attach.rs:3140-3166`）。
  - 后台连接一律带 `BatchMode=yes`、`NumberOfPasswordPrompts=0`、`StrictHostKeyChecking=yes`、`ConnectTimeout=10`、`ConnectionAttempts=1`（`attach.rs:1387-1403`）；默认加 `-C` 压缩。
- **怎么找远端程序**：先在用户的登录 shell 里跑 `command -v herdr`；遇到不认 `command -v` 的 shell（xonsh 之类），改用 `/bin/sh` 再跑一次；再去 `~/.local/bin/herdr`、Homebrew、mise、nix 这些常见位置找（`attach.rs:1890-1960`）。找到后检查能力，缓存路径。后台连接从不安装、升级或重启远端。
- **远程终端怎么接到本机**：不转原始字节。远端 server 自己做终端模拟，把画面按格子的增量发过来（`herdr:src/protocol/` 下的 `surface_delta`、`surface_reuse`、`surface_scroll`），由本机来画；输入按语义编码（`protocol/endpoint.rs:20-21` `shell.input.semantic.v1`）。只有当前选中的那台机器传画面，别的机器只传 workspace、agent 状态和通知。
- **终端大小**：选中的机器收本机窗格的大小；同一时刻只有一个 controller 拥有输入和大小（`persistence-remote.mdx` 153-164 行）。这和 corral “第一个接入的能打字，尺寸跟着它走”是同一个思路。
- **断线和重连**：
  - 每台机器各管各的，一台断了不影响别的；本机不等远程，先打开。
  - 断线后，文档原话：“the last workspace and agent state remains visible but dimmed… Input and navigation into those cached panes stay disabled until a fresh connection and matching screen arrive”。
  - 自动重试：0.5 秒起翻倍，最多 120 秒；连上要稳定 60 秒才重置。睡眠唤醒后 30 秒内每 2 秒重试一次（`herdr:src/client/endpoint/supervisor.rs:11-17`）。
  - 应用层心跳 5 秒一次，10 秒没回就算断（`health.rs:3-4`）。
  - 共享连接开不出新通道、新连接却能通时，弃掉旧 master 重连；点一下状态标记立即重试。
  - 只读命令最多重试一次；可能已经改了远端状态的命令，绝不自动重发。
- **凭证**：全交给 OpenSSH（ssh-agent、钥匙串、ssh 配置）。后台从不弹密码框：需要认证时提示用户在终端跑 `herdr machine reconnect`，带口令的私钥先 `ssh-add`。host key 不认识或变了时标成需要处理（`herdr:src/remote.rs:43-60` 专门识别这两类错误）。不替用户开 `ForwardAgent`。
- **延迟**：靠压缩减少画面的流量，文档说这 “does not change how typing or other input is delivered”。连接一分钟没有流量就关掉远端的 bridge，server 和窗格照常跑。
- **`--remote-keybindings local|server`**：`herdr --remote` 时用本机还是远端的快捷键配置，默认本机。
- **`--handoff`**：远端 server 要换版本时，实验性地把运行中的窗格热交接给新进程。

paddock 能借的，只借思路、不搬代码（真要搬，保留 Apache-2.0 声明）：
1. 用系统 OpenSSH，登录方式全听 ssh 自己的配置；paddock 不存密码和私钥，后台只用 `BatchMode=yes`，认证失败就提示用户去终端处理。
2. paddock 自己的 ControlPath 目录（0700，检查属主和链接，检查长度），配 `ControlMaster=auto` 加 `ControlPersist`；不碰用户自己的 master。
3. 找远端的 corral：先找 PATH 里的 `corral`，找不到再试 `$HOME/.local/bin/corral`（ranch 装的位置）；连上先看 `corral --version` 的 `contract`。
4. 每台机器各有一份状态，本机不等远程；断线时保留最后的样子、画暗、不让打字；退避重连，唤醒后快速重试，点一下立即重试；共享连接坏了就换新的。
5. 只读命令（`ls`、`status`、`reply`、`--version`）可以重试；`send`、`start`、`stop` 不自动重发，和 corral 自己“不要重送”的要求一致。
6. 配了远程机器才出现机器标记；机器名（label）和 ssh 目标分开写。

不适合 paddock 的：
- 远端的 bridge、herdr 自己的协议、远端的终端模拟和画面增量：paddock 不往远端装自己的程序，corral 也只有公开命令和 attach 的原始字节流。paddock 本地本来就用 alacritty 解析字节，直接用就行。
- `-T`：paddock 的 attach 必须用 `-tt`，因为 `corral attach` 要求两头都是终端，不是就报 `not_a_tty`（`../ranch/crates/corral/src/attach.rs:60`）。有了终端，ssh 的转义字符 `~` 就生效：按 `man ssh`，行首打 `~.` 会断开连接；本机 `ssh -G localhost` 显示 `escapechar ~`。所以要加 `-e none`。herdr 没有这个问题。
- `--remote-keybindings`：paddock 的快捷键都在本机 GPUI 里处理，远端只收到字节，没有对应的东西。
- `--handoff`、自动安装或升级远端：远端换版本是 `corral upgrade` 和 ranch 部署的事，paddock 不做。
- 临时 ssh 配置文件：paddock 用命令行 `-o` 就够。命令行的值优先，只覆盖 paddock 必须管的几项（ControlPath、ControlMaster、ControlPersist、BatchMode、ConnectTimeout、ServerAlive*），其余照用户的配置；paddock 不读也不复制 `~/.ssh/config`。代价是用户自己写的 ServerAlive 会被覆盖，影响很小。

### 3. 分层方案

先说三层都要的底座，甲就得做：

- **传输**【推，要实测】：
  - 每台机器一个 paddock 自己的 ssh master。每条 corral 命令跑一次 `ssh -S <路径> -o ControlMaster=auto -o ControlPersist=<秒> -o BatchMode=yes -o ConnectTimeout=… -o ServerAliveInterval=5 -o ServerAliveCountMax=3 -- <host> <命令>`。复用连接后，一条命令大约是一个来回，加上远端起一个登录 shell 的时间。不复用的话每条命令都要重新握手，一秒 N+1 次不现实。
  - 控制套接字的路径要短。【实】本机 `$TMPDIR` 规范化后是 57 字节；`<它>/paddock-ctl/ssh-0123456789ab.sock` 再加 OpenSSH 的 17 字节临时名，一共 107 字节，超过 macOS 的上限 104。所以要么用很短的文件名，要么用 `/tmp/paddock-<uid>/` 这样的短目录。
- **合并调用**：一轮读两次。先 `corral ls`，再把各 agent 的 `status` 拼成一条远端命令；每条输出一行 JSON，按顺序拆开。不管有几个 agent，都是两个来回。名字只含 corral 允许的字母、数字、`.`、`_`、`-`、`/`，照样加引号。
- **远端命令的引号**：
  - 【实】sshd 把命令交给用户的登录 shell 加 `-c` 来跑（`man sshd`：“execute via the user's shell using its -c option”）。所以每个参数都要按 POSIX 单引号转义；paddock 已经依赖 `shell-words`（`app/Cargo.toml:28`）。收件箱里用户打的任意文字也走这一条。
  - fish 之类非 POSIX 的登录 shell 转义规则不一样。【推】连上时做一次引号自检：送一段带 `'`、`$()`、换行和中文的字，看是不是原样回来；不对就把这台标成不支持。
- **attach**：在本机 PTY 里跑 `ssh -tt -e none -S <路径> … -- <host> corral attach <名字>`。
  - 【实】`corral attach` 每 100 毫秒看一次自己终端的大小，变了就发改尺寸（`../ranch/crates/corral/src/attach.rs:140,172-183`）。ssh 会把本机窗口大小的变化送到远端终端，所以 paddock 现有的 `master.resize` 不用改。
  - 【实】pen 收到 attach 时先重放当前画面（`../ranch/crates/corral/src/pen.rs:382` `self.term.replay()`），断了重接，画面能回来。
  - 【推，要实测】经 master 复用的会话也能传窗口大小（OpenSSH 的复用客户端会把 SIGWINCH 转给 master）。不行的话，attach 就各用一条不复用的连接，每次 attach 多一次握手，可以接受。
  - 【推】paddock 关窗格时给子进程发 SIGINT，ssh 随之退出，远端 sshd 关掉终端；`corral attach` 读到结束（`attach.rs:148-151`）或读出错时退出，agent 不受影响。3 秒后还有 SIGKILL 兜底（`pty.rs:235`）。
- **身份**：paddock 内部把远程 agent 叫 `<机器>:<名字>`（§4.2）。
- **测试**：照“测试不依赖真实 agent”的规矩，用一个假 `ssh` 脚本（解析参数，把远端命令交给本机 `sh -c`）配上现有的假 `corral` 脚本，不连任何真机器。

**甲：看、attach 打字、状态和回复、收件箱**
- 要动的地方：
  - `config.rs` 加 `[[machines]]`（§4.1）。
  - 新模块（比如 `remote.rs`）：拼 ssh 参数、管 master、POSIX 引号、引号自检、检查 `corral --version`，把 ssh 的错误分成认证、host key、连不上、找不到 corral、contract 不对几类。
  - `corral.rs` 的 `Client` 带上“哪台机器”；直接拿程序名拼命令的地方（attach、attach 前的核对、`send_first`、`pause`）都改成走它。
  - 每台机器一个 `Poller`，结果按机器并进 `Panel`。某台读不到时保留它上次的 agent，`Alive` 里照样算上，不关窗格。
  - 左侧栏的机器分区、连接状态、断线画暗；窗格的断线提示和自动重接（§4.4）；收件箱按机器的读不到行。
  - 排除远程 agent：git 摘要和分组、Activity、Changes/Kanban/Cairn 的跟随、Kanban 的 `seen()`、workspace 归属、New Agent 的目录候选、paddock ctl。
  - 窗格标题、命令面板、收件箱、提示里写上机器名；Diagnostics 每台机器一行。
- 大小【推】：2500–4000 行，含测试，比 P5-89 workspace 大。按影响面算碰要害（ssh 凭证、并发、身份），实现那件要交叉审查。
- 风险：
  - 身份改动面广，凡是按名字找 agent 的地方都要过一遍。
  - 同路径误读本机仓库：上表里要“按机器排除”的地方漏掉一处，就会显示错的分支、归错 workspace、在看板上挂错任务。
  - ssh 的细节：登录 shell 的引号和 PATH、控制套接字的路径长度、host key、复用连接下的窗口大小。
  - 断网后远端留下的那个 attach 一直占着“能打字”的位置（§5 最后一条）。
  - 远端机器上自己也开着 paddock、窗格里接着同一个 agent 时，这边会被 `attached == 0` 的核对挡住（`viewer.rs:246-251`），只能等那边关掉，或者改成只读接入（§4.4，要用户定）。

**乙：甲加上开新 agent、停 agent**
- 停、暂停/恢复：各一条命令，改动很小，确认框里写上机器名就行。建议并进甲。
- 开新 agent，即 New Agent 能选机器：
  - 目录手填，不弹本机的选目录框；`~` 不能再按本机 HOME 展开（`new_agent.rs:446`），要么连上时问一次远端的 `$HOME`，要么要求写绝对路径；最近用过的目录按机器分开记。
  - 模型和强度的列表沿用本机的，它们只是写进命令行的参数。
  - amp、agy 的插件提示：要么先不给，要么到远端跑 `corral integration status`（要一两秒，见 ranch CONTRACT）。
  - amp 的第一句照现在的 `wait` 加 `send`，经远端跑。
  - 从 workspace 里开、从 paddock ctl 开的，不管远程。
- 大小【推】：800–1500 行。风险在 New Agent 表单分岔多（`new_agent.rs` 1738 行、`new_agent_view.rs` 2654 行）；`corral start` 经 ssh 最长 120 秒（`new_agent.rs:22`）。

**丙：再加目录相关的（分支、改动、分组、workspace、Changes、Kanban、Cairn、Activity）**
- 要把 `git.rs`、`changes.rs`（2474 行）、`diff.rs`、`kanban.rs`（3286 行，还直接读草稿任务文件）、`cairn.rs`、`activity.rs` 的读取都改成“在哪台机器跑命令”，读文件也得改成远端命令。workspace 的成员要从“本机主仓库路径”改成“机器加路径”，`workspaces.json` 的格式和 P5-105 的规则都要动。Changes 一次要跑好几条 git，延迟和引号安全的问题都会放大。
- 大小【推】：改动 3000–6000 行，是另一件大活。可以拆：丙1 只做卡片上的分支和改动数，再加按仓库分组（`git.rs` 一次摘要只有几条命令，能合成一次远端调用）；Changes、Kanban、Cairn、Activity 往后放。

推荐先做甲，停、暂停/恢复并进去。理由：
- 用户 10-14 的原话是“我要能在paddock中加载远程的corral托管的agent”，甲正好是这个。
- 甲已经把最难、风险最大的底座做了（ssh 传输、身份、断线、排除本机目录功能），乙和丙都是在上面加东西。
- 乙的 New Agent 和丙的目录功能互不相干，等用户用过甲再挑。

### 4. P5-90 草稿六个问题里，现在能给建议的

第 1 条（远程指什么）用户已经答了；第 3 条（paddock 的事还是 corral 的事）用户已经定了只在 paddock 里做；第 6 条（谁做、优先级）不归这里。下面讲机器怎么配，和第 4、5 条。

**4.1 机器怎么配**
- 推荐写在 `~/.config/paddock/config.toml`，用户手写，paddock 只读，和现在的 `corral = …` 一样：
  ```toml
  [[machines]]
  name = "studio"        # paddock 里的叫法，也是名字前缀 studio:；小写字母、数字、-，不能重复，不能叫 local
  ssh = "<host>"         # 交给 ssh 的目标：~/.ssh/config 里的 Host 别名，或 user@host、ssh://user@host:port
  # corral = "~/.local/bin/corral"   # 可选：远端 PATH 里找不到 corral 时写；开头的 ~ 由 paddock 换成远端的 $HOME
  ```
  - 校验：`ssh` 不空，不含空白和控制字符，不以 `-` 开头，不带密码；拼命令时在 ssh 目标前加 `--`。
  - 登录全听 ssh 自己的配置：端口、用户、私钥、跳板机都写在 `~/.ssh/config` 里，paddock 不存任何凭证。
- 取舍：herdr 用命令行加一份单独的 JSON 存机器，有不透明的 ID，改名不换身份。paddock 先用手写的配置，省掉“添加机器”的界面。代价是改 `name` 就等于换了身份，保存的布局里那台机器的窗格恢复不了，可以接受。以后要在 Settings 里加机器，再改成单独的文件。
- 几台、同时看几台：用户没说。推荐一开始就支持多台（每台一个 `Poller`，代价和一台差不多），实测先用一台。

**4.2 远程和本机重名**
- 推荐：paddock 内部把远程 agent 的名字写成 `<机器>:<名字>`，比如 `studio:paddock/main`。
  - 【实】corral 的名字只能由字母、数字、`.`、`_`、`-` 和分段用的 `/` 组成（基础约定“名字”一节；实现在 `../ranch/crates/corral/src/state.rs:42-54`，不合格报 `bad_name`）。`:` 不会出现在任何 corral 名字里，所以永远不会和本机的撞，也不会和别的机器的撞。只在调 corral 时去掉前缀。
  - 这样，现在所有按名字的地方（选中、展开、未读、回复缓存、保存的布局、窗格）都照旧能用，改动集中在“调 corral”和“显示”两头。
- 显示：机器分区里的卡片只写原名；离开分区的地方（窗格标签和标题、收件箱、命令面板、提示）写原名加机器名的小标记，比如 `paddock/main · studio`。
- 取舍：另一种做法是给 `Agent` 加一个机器字段，所有按名字查的地方都改成按（机器，名字）查。更干净，但要改的地方多得多，保存的布局也要改格式。不推荐。
- paddock ctl 和 `paddock --attach` 在甲里只认本机的名字；给了带 `:` 的名字，就报一个说得清楚的错。

**4.3 左侧栏怎么看出是远程的（和 workspace、按仓库分组怎么摆）**
- 推荐：
  - 没配远程机器时，左侧栏和现在一模一样。
  - 配了以后，本机的（workspace、仓库组）照旧在上面，下面每台远程机器一个分区。分区的标题行写机器名、连接状态、agent 个数，能收起（借 workspace 托盘的画法和 `Folds`）。分区里按名字前缀分组（甲读不到远程的仓库），组也能收起。
  - 远程 agent 不进任何 workspace；workspace 的菜单和浮层里不列远程的组。
  - 本机那部分不加标题（比如 `This Mac`），保持现在的样子。
- 取舍：
  - 另一种是 herdr 的做法：远程 agent 混进本机的组，卡片上标机器名。paddock 的组按本机主仓库分，甲里远程 agent 读不到仓库，只能按前缀落成 `studio:paddock` 这样的组，夹在本机的组中间，很乱；P5-89 的前缀规则还会把它们拉进本机的 workspace。不推荐。
  - 丙做完以后，远程仓库可以按（机器，路径）进 workspace，比如一个大项目里同时有本机和远程的仓库，到时再定。

**4.4 断线怎么显示、要不要自动重连**
- 推荐：
  - 机器的标题行写状态：`Connecting…`；在线时不写字，只画一个点；`Reconnecting in 8s`、`Can't sign in`、`Host key unknown`、`corral not found`、`corral contract 2, needs 1`。点标题行立即重试；悬停时写原因和要用户做的事，比如 `Run ssh <host> once in Terminal`。
  - 断了以后：这台机器的 agent 留着上次读到的样子，画暗，状态改成“多久以前看到的”，不算“消失”，不关窗格。收件箱和铃铛里按机器各加一行“读不到”，照现在 `corral` 那一行的做法（`attention.rs:103`）。
  - 窗格：
    - attach 的 ssh 退出码是 255 时（ssh 自己的连接错误；`corral attach` 主动断开是 0，agent 退出是 2），显示 `Lost connection to studio — reconnecting…`，不让打字。
    - 机器重新读到、这个 agent 还在、实例没变，就自动重接（pen 会重放画面）；实例变了或者不在了，照本机现在“agent 结束”的办法处理。
  - 自动重连：退避，从 1 秒起翻倍，最多 60 秒左右；唤醒后立即试一次；后台从不弹认证框（`BatchMode=yes`）。
- 要用户定的一处：远端已经有别的窗口接着这个 agent（远端机器上的 paddock 开着它，或者断网后残留的那个 attach），现在的核对会直接拒绝（`viewer.rs:246-251`）。可以改成只读接入，写一句 `Read-only: attached elsewhere`：corral 本来就让后接入的只读，前一个走了，自动换它能打字（`pen.rs:298-310`）。也可以保持拒绝。推荐远程的改成只读接入，本机照旧拒绝。
- 取舍：自动重连省事，但网络一直不通时，后台会一直定期起 ssh。退避上限多大、要不要“停用这台”的开关（herdr 有 `machine disable`），用过再说。

### 5. 远程机器那边要什么
- **corral**：
  - 【实】本机的 `corral --version` 输出 `{"contract":"1","ok":true,"version":"0.1.0"}`，`~/.local/bin/corral` 指向 ranch 装的版本目录 `~/.local/share/ranch/versions/f86d1c2/bin/corral`。
  - 【实】ranch 里 corral 包的版本一直是 0.1.0（`../ranch/crates/corral/Cargo.toml:3`），所以 `version` 分不出 ranch 的不同版本，能看的只有 `contract`。
  - 要求：远端用 ranch 装 corral，`contract` 是 `1`。甲只用基础约定里的命令（`ls`、`status`、`reply`、`send`、`attach`，加上 `--version`），任何 contract 1 的 corral 都行；没有 `paused` 字段时，paddock 已经当作没暂停。
  - 乙里 amp 要 R6 以后、agy 要 R8 以后的 corral（ranch CONTRACT），旧版会报 `unknown` 或 `usage`，照报错提示用户。
  - 推荐两台装同一个 ranch 版本（在远端照 ranch 的部署办法装），不强制。
  - paddock 怎么发现版本不一致：连上先跑 `corral --version`。contract 不是 `1` 就标出来，不读列表；版本号和找到 corral 的路径写进 Diagnostics。想精确比较 ranch 版本，得 ranch 在 `--version` 里加构建号，这是改公开约定，走 ranch 的流程，甲用不着。
- **PATH**：
  - 【实】ssh 跑命令用的是用户的登录 shell 加 `-c`（`man sshd`），不是交互式 shell。【推】zsh 这时只读 `~/.zshenv`，bash 看发行版可能什么都不读，`~/.local/bin` 常常不在 PATH 里。
  - 所以 paddock 先试 `corral`，再试 `$HOME/.local/bin/corral`，都不行报 `corral not found`；也可以在配置里写 `corral =`。
  - agent 自己的环境是 `corral start` 从远端的登录 shell 取的（基础约定 `start` 的“环境”一条），和 ssh 无关。
- **登录 shell**：zsh、bash、sh 这类 POSIX 的都行；fish 等【推】不支持，自检会报出来。
- **corral 自己的要求**：远端要有 `/usr/bin/python3`，Claude Code 和 Codex 的钩子要用（基础约定 `start` 一节），和本机一样。
- **ssh**：
  - 不输密码就能登上：私钥放在 ssh-agent 或 macOS 钥匙串里（`UseKeychain`、`AddKeysToAgent`），或者是没口令的私钥。
  - host key 已经在 `known_hosts` 里：先在终端里 `ssh <host>` 一次。
  - 远端是 Mac 的话要打开“远程登录”，而且不能睡眠（远端睡了，agent 也停）。
  - 【实】本机是 OpenSSH 10.3p1；`ssh -G localhost` 里的默认值是 `serveraliveinterval 0`、`connecttimeout none`、`controlmaster false`、`escapechar ~`，所以这几项 paddock 必须自己带上。
- **建议（可选，要改远端的系统配置，用户定）**：在远端的 `sshd_config` 里设 `ClientAliveInterval 15`、`ClientAliveCountMax 3`。【推】不然网络突然断掉（不是正常关闭）时，远端 sshd 要很久才发现，留下的 `corral attach` 会一直占着能打字的位置（pen 只让第一个接入的打字，`pen.rs:376`），重连上来的只能只读，直到它被清掉。

### 请用户跑的实测清单
主控和 agent 都没连过任何远程机器。把 `<host>` 换成你的机器；第 4、5 步开的是测试 agent，用完要 stop。
1. 免密登录和 host key：`ssh -o BatchMode=yes <host> true; echo $?`，应该是 0。不是 0 的话，先在终端里 `ssh <host>` 一次接受 host key；私钥有口令就先 `ssh-add`。
2. 远端的 corral 和 PATH：`ssh -o BatchMode=yes <host> 'command -v corral; corral --version; echo "$SHELL"'`，看找不找得到、contract 是不是 1、登录 shell 是什么。找不到就再试 `ssh <host> '$HOME/.local/bin/corral --version'`。
3. 一条命令要多久：
   - 不复用：`time ssh -o BatchMode=yes <host> corral ls`。
   - 复用：先 `ssh -o ControlMaster=yes -o ControlPersist=60 -S /tmp/pdk-test -fN <host>`，再连跑几次 `time ssh -S /tmp/pdk-test <host> corral ls`，最后 `ssh -S /tmp/pdk-test -O exit <host>`。
4. attach 和改窗口大小：
   - 在远端开一个测试 agent，用远端已经能用的 agent，比如 `ssh <host> 'corral start paddock/test-remote --cwd /tmp --label role=test -- claude'`。
   - 在本机终端里跑 `ssh -tt -e none <host> corral attach paddock/test-remote`：打几个字；拖终端窗口改大小，看画面跟不跟；按 Ctrl-] 退出。
   - 再用复用连接试一次改大小：照第 3 步先起 master，然后跑 `ssh -tt -e none -S /tmp/pdk-test <host> corral attach paddock/test-remote`。
5. 断网：attach 着的时候关掉 Wi-Fi 30 秒再打开，看本机的 ssh 多久退出。然后重新 `ssh -tt -e none <host> corral attach paddock/test-remote`，看能不能打字；另开一个终端跑 `ssh <host> corral status paddock/test-remote`，看 `attached` 是几。最后 `ssh <host> corral stop paddock/test-remote`。
6. 问一句：远端那台机器上平时开不开 paddock，会不会开着窗格接着 agent？会的话，这边 attach 会被挡住（§4.4）。

### 推荐（交用户定）
1. 先做甲：远程 agent 看得到，能在窗格里 attach 打字，看得到状态和回复，收件箱能读能回；停、暂停/恢复各是一条命令，一起放进来。开远程 agent（乙）和远程的分支、改动、Changes 等（丙），等用过甲再定。
2. 只在 paddock 里做：用系统的 ssh，每台机器一个 paddock 自己的复用连接，一轮刷新合成两次调用。不改 corral 和 ranch，不往远端装东西，不存凭证。
3. 机器写在 `config.toml` 的 `[[machines]]` 里（`name`、`ssh`，可选 `corral`）。paddock 内部把远程 agent 叫 `<机器>:<名字>`；左侧栏在本机的下面，每台机器一个能收起的分区；远程 agent 不进 workspace、不进看板、不读本机目录。
4. 断线时保留上次的样子并画暗，机器标题行写出原因，自动退避重连，窗格能接上时自动重接。远端已经有别的窗口接着这个 agent 时，远程的改成只读接入，本机照旧拒绝。
5. 开工前请你照上面的实测清单跑 1–6，尤其第 4 步的改大小和第 5 步的断网。结果决定 attach 走不走复用连接，以及要不要建议你在远端设 `ClientAliveInterval`。

## 完成记录
- 做了什么：
  - 读了 `CLAUDE.md`、P5-90 草稿、DESIGN §3 和 P5-80、P5-87、P5-89、P5-105，ranch 的 `docs/CONTRACT.md` 和基础约定（corral 仓库 `6923da1` 的 `docs/CONTRACT.md`，用 `git show` 只读），还有 `corral guide`。
  - 按模块查了 paddock 调 corral 和读本机目录的地方。
  - 为了弄清 attach 的行为，只读了 ranch 的 `crates/corral/src/attach.rs`、`pen.rs` 里 attach 那部分和 `state.rs` 的名字校验。
  - 把 herdr 浅克隆到 scratchpad，读了文档、`src/remote*`、`src/platform/unix_common.rs`、`src/client/endpoint/` 下的 `catalog.rs`、`health.rs`、`supervisor.rs`，还有 `src/protocol/`。
  - 结论写在上面。
- 验证了什么：
  - 只跑了看帮助和版本的命令：`corral --help`、`corral --version`、`corral guide`、`herdr --help`、`herdr machine … --help`、`ranch --help`、`ssh -V`、`ssh -G localhost`（只筛了几项默认值），以及 `man ssh`、`man sshd`、`man ssh_config`；量了一下本机 `$TMPDIR` 的长度。
  - 没连任何远程机器，没读 `~/.ssh`，没起 herdr server、没 attach，没起 paddock 窗口，没碰 corral 里的 agent。
  - 有一处多余：误跑了一次空的 `python3 -c 1`，什么也没做。
- 拿主意的地方：
  - 身份用 `<机器>:<名字>`，依据是 `:` 不在 corral 名字允许的字符里。
  - 机器配置放 `config.toml`，不另开 JSON。
  - 建议把停、暂停/恢复并进甲。
  - ssh 选项用命令行 `-o`，不学 herdr 的临时配置文件。
  - 远程窗格遇到已接入时改成只读接入。
  - 各层的行数是读代码估的，不准。
- 没做的事：
  - ssh 的行为一样都没实测：复用连接下的改大小、断网后留下的 attach、每条命令的耗时都是推测，列进了实测清单。
  - 没读 OpenSSH 的源码确认复用连接转发 SIGWINCH。
  - 没看 herdr 的手机端和 Windows 部分。
  - 没出界面样稿。
  - 没改代码和 DESIGN。

## 主控审查（10-15，paddock/main）
- 只改了本文件；没改代码、没连远程机器、没读 `~/.ssh`；结论里只有 `<host>` 占位，gitleaks 扫过无发现。读了 ranch 的 corral 源码（只读、为弄清 attach 和重放的行为），不改 ranch，可以。herdr 只借思路、标了文件位置，没有照搬代码。
- 五项都有结论，事实、推测分开标了（【实】／【推】），附代码位置和 man 页原文；推荐五条、要用户定的四件写清楚了。
- 同意推荐的顺序：先请用户跑实测清单，再定甲的范围写任务文件；甲实现时按碰要害做交叉审查。

## 更正（10-15，paddock/main）
- §4.3 取舍里说 herdr “远程 agent 混进本机的组，卡片上标机器名”，不对。herdr 文档 `connecting-machines.mdx` 写的是一个窗口里在左侧栏选机器，选中的那台提供窗格画面、接收输入；workspace、窗格、agent 名字按机器各自独立（“Workspace, tab, pane IDs, and agent names are scoped to one server”）；只有 agent 行在有多台机器时多一个 `machine` 标记。用户 10-15 据此定了本机和远程隔开，见 P5-90「本机和远程隔开」一节。
