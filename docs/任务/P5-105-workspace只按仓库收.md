# 任务：workspace 只按仓库收，同一仓库的 agent 永远挂在一起

2026-10-15，paddock/main 交给 paddock/dev-workspace-repo-1（Claude Code，重：opus[1m] / xhigh）。
路由：重 / 交叉审查要 / 影响面：碰要害（路由：档拿不准（重 0.83），交叉审查要（核心规则 0.80），影响面碰要害；主控按重定档）
类型：功能变更
依据：用户 10-15 发来截图：主控 `paddock/main` 在 workspace My-Harness 里，委派出去的 `paddock/dev-agy-1` 落在外面另一个 paddock 组。主控查明 My-Harness 当时只按名字记了两个主控（P5-89 允许单个 agent 入组）。用户：“我觉得这样会不会复杂了点？哈哈”“不管agent开在workspace里面还是外面开了单独的agent（不再workspace下），我的理解就是一个repo下的所有agent都要挂在这个repo下。”从 workspace 里 New Agent、仓库还不在任何 workspace 时，主控给了甲（把仓库带进来）乙（不带）：“选甲”。“能并行派出去，就并行派出去”（和 P5-104 并行）。规则已写进 `docs/DESIGN.md` §13 P5-105。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- 仓库根的 `CLAUDE.md`（规矩，尤其「独立编译目录」「桌面窗口测试」「验证」）。
- `docs/DESIGN.md` §13 的 P5-105（本件的规则，以它为准），以及它改的 P5-89（workspace）、P5-93（在 workspace 里新建 agent）两条；P5-80（同仓库分组）。
- `docs/任务/P5-89-workspace.md`、`docs/任务/P5-93-在workspace里新建agent.md` 的「要做的」，看原来怎么做的。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-105-workspace-by-repo`，分支 `p5-105-workspace-by-repo`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-105-workspace-by-repo`。
- 动 `app/src/workspaces.rs`、`app/src/sidebar_workspaces.rs`、`app/src/card.rs`，需要时 `app/src/sidebar.rs`、`app/src/agents.rs`；测试；`docs/DESIGN.md` 的 P5-105 一条（照实际做法补细节，规则要改先停下来问）和本文件。
- 并行：`paddock/dev-agy-followups-1` 在分支 `p5-104-agy-followups` 改 `new_agent_view.rs`、`skills.rs`、`integrations.rs`，你不要动这些文件。P5-93 的归属代码在 `sidebar_workspaces.rs` 的 `join_started` 和 `workspaces.rs` 的 `join`，改那里就够；要动 `new_agent_view.rs` 才做得下去时，停下来报告。

## 要做的
照 DESIGN P5-105：
1. **规则**：workspace 的成员只有整个仓库（主仓库路径），一个仓库只在一个 workspace 里。agent 归哪只看它的主仓库；同一仓库的 agent 在左侧栏只出现在一个仓库小标题下。读不到仓库、按名字前缀分组的 agent 照 P5-89 的前缀规则跟着走，不能手动加入。
2. **入口**：新建和 `Add repositories…` 浮层每个仓库一个勾，不再展开勾单个 agent、不再列 `Not running`，前缀组不列；卡片详情 `Workspace` 一格选某个 workspace 挪整个仓库，`None` 移出整个仓库，前缀组的 agent 这一格只显示、不能改；仓库小标题上的按钮照旧。
3. **从 workspace 里开**（P5-93 三个入口）：开起来后，仓库不在任何 workspace 里就把整个仓库加进这个 workspace；已在别的 workspace 里就不挪；读不到仓库的什么都不记。
4. **旧名单**：`workspaces.json` 里按名字记着的 agent，读到它正在运行、知道主仓库时换成它的仓库（仓库已在别的 workspace 就只去掉名字）；不在运行或读不到仓库的名字不再起作用，下次改名单时去掉。存盘格式不变（`agents` 字段留着、读进来，写出时为空）。坏文件、读不出来照旧当作没有 workspace。
5. 不再用到的代码（按 agent 勾选、`Not running`、单记 agent 的菜单项）删掉，不留死代码。

## 怎么算做完
- 用户原话：“不管agent开在workspace里面还是外面开了单独的agent（不再workspace下），我的理解就是一个repo下的所有agent都要挂在这个repo下。”从 workspace 里 New Agent、仓库不在任何 workspace 时：“选甲”（把仓库带进来）。
- 验证只做这些：`workspaces.rs` 的规则和旧名单换算写针对性测试，含边角（仓库已在别的 workspace、名字对应的 agent 不在运行、前缀组、坏文件、同一仓库多个 worktree 的 agent）；`card.rs` 行排列一条（同仓库的 agent 只出现在一组）；P5-93 归属一条。再跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check`。不起窗口、不截图。觉得不够，在回复里说，不要自己加。

## 不要做
- 不读、不写用户真实的 `~/.local/state/paddock/workspaces.json` 和布局文件；测试用临时目录。
- 不改 P5-80 的分组规则、卡片的内容和画法、workspace 托盘的画法（P5-91）、收起状态的存法。
- 不做：同一仓库分在两个 workspace、按 agent 挑成员、给前缀组单独记成员、拖动归类。
- 不对 `corral ls` 里现有的 agent 做 `stop`、`send`、`keys`、attach；不用 corral 开真 agent。
- 不起 paddock 窗口、不截图、不用脚本模拟按键或鼠标。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录，一条命令能把它们全杀掉。停自己起的进程用起的时候记下的 PID。
- 遇到 DESIGN P5-105 的规则说不通、或要改上面「不改」里的东西才能做下去，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-105-workspace-by-repo` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
