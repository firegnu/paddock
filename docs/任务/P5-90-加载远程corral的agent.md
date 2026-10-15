# 任务：在 paddock 里加载远程机器上 corral 托管的 agent

草稿（2026-10-14，paddock/main 记）。只记了用户的话和主控查到的现状，范围和验收还没定，开工前和用户过一遍。
依据：用户 10-14：“再记一个draft task，我要能在paddock中加载远程的corral托管的agent”。
依赖：P5-90a
待用户：跑 P5-90a「请用户跑的实测清单」1–5，并确认两件：停/暂停/恢复并进甲；远端已有窗口接着时远程改成只读接入、本机照旧拒绝（都是主控建议）

## 现状（主控 10-14 查的，只读）
- paddock 只和本机的 corral 打交道：调的是配置里的那一个 `corral` 程序（`app/src/config.rs` 的 `corral`，默认就是 `corral`，启动时也能用 `--corral` 换），读列表、状态、回复、开和停都走它（`app/src/corral.rs` 的 `Client`）；窗格里显示 agent 是在本机起一个 `corral attach <名字>`（`app/src/viewer.rs`）。一个窗口只有一个 corral 来源。
- corral 自己没有“远程”这回事：`corral --help`、`corral guide` 和 ranch 的文档里都没有连别的机器、指定主机的写法（主控只搜了文档和帮助，没读 corral 的代码）。
- paddock 里有不少东西是按 agent 的启动目录去读**本机**文件的，远程 agent 的目录本机没有：
  - 左侧栏卡片上的分支和改动数、按仓库分组（P5-80）、workspace 里“整个仓库”的归属（P5-89）：都靠在那个目录跑 git。
  - 右侧栏的 Changes、Kanban、Cairn，左侧栏下面的 Activity：都是读那个目录所在的仓库。
  - 收件箱读最后一条回复、就地回一句（P5-87）走的是 corral 命令，不读本机文件。

## 开工前要定
1. “远程”指什么：另一台自己的机器（走 ssh）、容器或虚拟机、还是别人机器上的。有几台、要不要同时看几台。（用户 10-15：“自己的另一台机器，ssh 能连上。”几台、同时看几台还没说。）
2. “加载”到哪一步。主控看到的几层，用户挑：
   - 甲：远程的 agent 出现在左侧栏里，能看状态、能点开在窗格里看和打字（等于远程的 `corral attach`）、收件箱能读能回。目录相关的（分支、改动数、Changes、Kanban、Cairn、Activity）对远程 agent 不显示或写明是远程的。
   - 乙：甲，加上能在 paddock 里往远程机器上开新 agent、停 agent。
   - 丙：再加上远程 agent 的分支、改动、Changes 等也能看（要在远程跑 git、把结果拿回来，另一件大活）。
3. 怎么连过去，是 paddock 的事还是 corral 的事：
   - 只在 paddock 里做：比如把每台远程机器当成“另一个 corral 来源”，命令通过 ssh 在那边跑。不动 ranch。
   - corral 自己支持远程：要在 ranch 仓库立任务、改公开约定，paddock 这件等它。
   主控还没调研哪条走得通（ssh 下 `corral attach` 的终端大小、断线重连、延迟、凭证都没试），定了方向先做调研。
4. 远程和本机的 agent 重名怎么办、左侧栏里怎么看出哪个是远程的（比如按机器再分一层，和 workspace 怎么摆）。
5. 断线时怎么显示、要不要自动重连。
6. 谁来做、优先级：用户没说，先不写（按中、主控定）。

## 调研时参考（用户 10-15）
- 用户：“我觉得你调研的时候可以看下herdr的实现，他也支持remote agent了”。herdr 是第三方的终端 agent 多路复用器（本机 Homebrew 装的 0.9.5，`/opt/homebrew/bin/herdr`；主页 https://herdr.dev；Homebrew 标的许可是 Apache-2.0，源码以 GitHub 上的为准，读之前核对仓库里的 LICENSE）。`herdr --help` 里有 `--remote <ssh-target>`（经 SSH 连远程的 herdr server）、`herdr machine`（存 SSH 机器）、`--machine <label-or-id>`（在存好的机器上跑 API 命令）、`--remote-keybindings`、`--handoff`。调研看它怎么连、怎么存机器、断线和延迟怎么处理、远程的终端怎么接到本机；只借思路，要照搬代码先确认许可并保留声明。
- 主控 10-15 建议的路子（用户还没定）：先开 P5-90，第一步只调研、不改代码；倾向甲层、只在 paddock 里经 ssh 调远程的 corral 公开命令，不改 corral。

## 本机和远程隔开（用户 10-15 定）
- 用户原话：“本地的agents和远程的agents隔离开，远程的就是远程的，本地的就是本地的。开agent和左侧agents显示区页隔离开”；“因为我记得herdr就是分开的。”；定了下面四条后：“这样远程的也不会破坏现在本地的。”
- herdr 确实是分开的（主控 10-15 核对 herdr 文档 `connecting-machines.mdx`）：一个窗口里在左侧栏选机器，选中的那台接收输入和终端大小、提供窗格画面；workspace、窗格、agent 名字都按机器各自独立；没选中的机器后台只更新 agent 状态和通知。P5-90a §4.3 说 herdr 把远程 agent 混进本机的组，是错的。
- 用户定的四条（主控 10-15 给的选项和线框）：
  1. 一个窗口里切换机器：左侧栏顶上一条切换栏（`This Mac │ studio ●2`，没配远程机器时不出现）；切到哪台，左侧栏、窗格都换成那台的，每台机器各记一套窗格布局。远程视图里右侧栏的 Changes、Kanban、Cairn 只写一句说明，不读本机。
  2. 切走以后，那台机器的窗格保持连着，切回来马上能用。
  3. 收件箱和铃铛合在一起：所有机器的都列出来，标上机器名，点一下切过去。
  4. New Agent 在哪台机器的视图里开就开到哪台；这是乙层，甲先只做本机。
- 远端是什么（用户 10-15）：“远端一般不会常开，远端一般就是要么linux，要么是vps之类的”。即远端多半是 Linux 或 VPS，平时不开 paddock，所以“远端已有窗口接着”基本只在断网后留下旧连接时碰到。
  - 主控 10-15 只读查了 ranch：corral 有 Linux 的代码分支（`crates/corral/src/freeze.rs` 非 macOS 走 `/proc`，`state.rs` 的套接字长度上限按平台取），但 ranch 没有 CI，主控没见过它在 Linux 上编过、跑过。远端没装 corral、或在 Linux 上跑不起来，是 ranch 那边的事（在 ranch 仓库立任务），不在 paddock 里做。
  - Linux 上 ssh 跑命令时，bash 常常不读 `~/.profile`，`~/.local/bin` 不在 PATH 里，所以 P5-90a 说的“找不到再试 `$HOME/.local/bin/corral`”要做。
  - VPS 的 sshd 一般是用户自己管的，P5-90a §5 建议的 `ClientAliveInterval` 更值得设，到时提醒用户。
- 这样定了以后，P5-90a 推荐里的 `<机器>:<名字>` 改名和到处“按机器排除”大多用不着了：每台机器各一份 agent 列表和窗格，本机那份照现在不变。甲的规模、做法写甲的任务文件时再估。

## 不做（现在就能定的）
- 不读远程或本机 corral 的内部状态目录，只走公开命令。
- 不在 paddock 里存远程机器的密码或私钥；怎么登录用系统已有的（比如 ssh 自己的配置）。
- 不在 paddock 里改 ranch；要 corral 改的写成需求在 ranch 仓库做。
- 实测只用自己开的测试 agent，不动用户的。

## 实测（10-15，主控在用户的 5090 上，用户允许）
- 用户定的规矩：那台机器上只拿 pi、omp 做实验，任何情况下都不运行那里的 Claude Code 和 Codex；pi、omp 也不许委派任何活。主机名、地址不写进仓库。
- 机器：Linux x86_64，登录 shell 是 zsh；平时负载很低（`uptime` 0.14）。
- 清单第 1 步：`ssh -o BatchMode=yes <host> true` 退出码 0，免密能登。
- 清单第 2 步：远端**没装 corral**（PATH 里没有，`~/.local/bin/corral` 不在，也没有 `~/.local/share/ranch`）。pi 在 nvm 的目录里，omp 在 `~/.local/bin`，都只在交互 shell 的 PATH 里，ssh 直接跑命令时找不到：证实 P5-90a 说的 PATH 问题。远端有 cargo 1.97、git、`/usr/bin/python3`。
- 清单第 3 步（经公网）：
  - 不复用连接：一条命令 5.4 秒（主要是握手）。
  - 复用连接：每条 0.78–1.0 秒；一次远端调用里跑 10 条也是 0.94 秒。
  - 一条一直开着的远端 `sh` 里一问一答：0.39–0.42 秒，这大约就是网络来回的时间。
  - 结论【实】：一定要复用连接；每轮刷新只能是一次远端调用（`ls` 加全部 `status` 合在一起，约 1 秒），本机的每秒刷新对远程做不到，远程按 2 秒左右一轮更现实；或者每台机器开一条常驻的远端 shell 来跑轮询，每轮约 0.4 秒。attach 里打字的回显也会慢 0.4 秒左右，这是网络本身的，paddock 改不了。
