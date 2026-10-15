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
