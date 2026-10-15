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

## 完成记录

**做了什么**
- `workspaces.rs`：归属只看主仓库路径，读不到仓库的照 P5-89 前缀规则；去掉按名字记 agent 的一套（`add_agent`、`remove_agent`、`of_agent`、`taken`，`Picks` 的单个 agent 勾选）。`join` 改成带仓库：仓库不在任何 workspace 就整个加进来，已在别处不挪，读不到仓库不记。新增 `named`、`adopt`（旧名单换算）；`agents` 字段照读、写出恒为空（`serialize_with`）。
- `agents.rs`：`workspace()` 不再看名字；`groups()` 只给有仓库的组（前缀组不列），`Listed.agents` 改成个数；新增 `named_read`（名单里的名字对应的、在 `corral ls` 里的 agent 的仓库 Git 是否都读到了）和 `adopt_named`。
- `card.rs`：`Heading` 去掉 `singles`；`Card.alone` 换成 `Card.repo`（卡片 `Workspace` 一格挪的仓库，前缀组为空）。
- `sidebar_workspaces.rs`：浮层每个组一个勾（整个仓库），去掉展开箭头、单个 agent 行、`Not running` 一段和部分勾选的样子；列表标题改 `Repositories`，说明去掉 “Tick single agents…” 一句。卡片菜单选 workspace 挪整个仓库、`None`（仓库在某个 workspace 里时才有）移出整个仓库、`New workspace…` 带上仓库；前缀组的 agent 这一格没有按钮。仓库小标题按钮只挪/移出仓库。`join_started` 不再要 agent 名。`edit_workspaces` 改名单前先按已读到的换算旧名单，前后一样就不写文件；新增 `adopt_named` 自动换算。
- `sidebar.rs`：每次读到 corral 或 Git 结果后调 `adopt_named`；卡片 `Workspace` 一格没有按钮时画成普通格子（字），宽度按普通格子算。
- `window.rs`：`join_started` 的调用去掉 agent 名（一行；P5-104 不动这个文件）。
- `docs/DESIGN.md` P5-105 补了实际做法和界面细节两条。

**验证了什么**
- 先在旧代码上确认 RED：旧名单里按名字记着 `paddock/main`、同仓库 worktree 里另有一个 agent 时，左侧栏出现两个 paddock 小标题（断言 1 个，得到 2 个）；改完后同一检查在新测试里通过。
- `app/tests/workspaces.rs`（13 条全过）：归属只看仓库（主仓库和两个 worktree 的 agent、前缀跟进、名字不起作用、两个 workspace 同名仓库时前缀不跟）；一个仓库只在一个 workspace；P5-93 归属（仓库不在任何 workspace 带进来、已在本 workspace 不记、在别的 workspace 不挪、读不到仓库不记、workspace 已删不记）；旧名单换算（等 Git 读完才算、仓库已在别的 workspace 只去名字、同一仓库名字记在两个 workspace 归先换到的、worktree 和前缀 agent 跟着仓库走、不在运行和读不到仓库的名字留着不起作用、存盘写空、坏文件当没有）；`card.rs` 行排列（同仓库的 agent 只在一个小标题下，名字记在别处、两个 worktree、前缀 agent 都一样；卡片带的仓库）；浮层整仓库勾选和前缀组不列。其余旧测试照新规则改了写法（去掉单个 agent 的部分）。
- `cargo test --all-targets` 全过（lib 533 条及全部集成测试）、`cargo clippy --all-targets -- -D warnings` 无警告、`cargo fmt --check`、`git diff --check` 通过。没起窗口、没截图。

**拿主意的地方**
- 旧名单什么时候换：存盘时 `agents` 写空，若一部分仓库读到就换算存盘，还没读到的名字会丢，所以自动换算等名字对应的运行中 agent 的仓库都读到（读到“不在仓库里”也算）再做；用户改名单前也先按已读到的换一遍，没读到的照规则在这次存盘时去掉。
- “正在运行”照 P5-89 `Not running` 的读法：在 `corral ls` 里。
- 同一仓库的名字记在两个 workspace 里：按 workspace 名字顺序，先换到的拿仓库，后面的只去名字。
- 卡片菜单的 `None` 只在它的仓库在某个 workspace 里时给（不在时 `None` 就是现状）。
- 前缀组 agent 的 `Workspace` 一格画成和别的格子一样的字，不画按钮和箭头。
- 改名单前后一样就不写文件（例如从 workspace 里开的 agent 的仓库已在别处），免得无事存盘把还没读到的旧名字去掉。
- 浮层列表标题、说明文字随规则改（见上）。

**没做的事**
- 没起窗口实测，菜单、浮层、卡片格子的样子和点击留给用户实际看。
- 没读写用户真实的 `workspaces.json`；装上后，用户现有的按名字记的主控会在 paddock 读到它们的仓库后自动换成整个仓库。
- DESIGN 里 P5-89、P5-93 原文的旧说法没改（已有“P5-105 改…”的标注），只补了 P5-105 一条。

## 主控审查（10-15，paddock/main）
- 看了 `workspaces.rs`、`agents.rs`、`card.rs`、`sidebar.rs`、`window.rs` 的 diff 和 `sidebar_workspaces.rs` 的归属、换算部分：归属只看主仓库（`of(repo, prefix)`），单记 agent 的增删查、按 agent 勾选、`Not running` 都删干净；`join` 只在仓库不在任何 workspace 时带进来；旧名单换算等到所有运行中、按名字记着的 agent 的仓库都读到才做，名单没变不写盘。「不要做」没碰；`window.rs` 一行是 `join_started` 签名跟着改，同意。
- 重跑一次 `cargo test --all-targets`：全过，合计 655 条。
- 对方的五条主意都同意（等读全再换、`corral ls` 里就算运行、同仓库名字在两处按 workspace 名先后、`None` 只在仓库已在某个 workspace 时出现、无变化不写）。
- 界面那一层（浮层、卡片格子、菜单）没有自动测试，留给用户看。路由碰要害，派 Codex 重档交叉审查，结论见 `docs/任务/P5-105-交叉审查.md`。

## 返工记录（10-15，交叉审查 R1、R2、R3）

**做了什么**
- R1（没变化的回调不借部分换算绕过等待存盘）：改名单的逻辑挪进 `Panel::edit_workspaces`（`agents.rs`，左侧栏的 `edit_workspaces` 只调它、报错、刷新）。先重读文件、按已读到的换算，再做这次改动；“这次改动本身改变了名单”就存盘（名单里剩下的名字随之去掉，照已接受的规则不变），“没改变”时只有换算过、而且重读后的名单里剩下的名字都读到了（`named_read`）才存盘，否则内存退回读到的样子、不写文件。P5-93 的 `join_started` 仓库已在别处、读不到仓库、workspace 已删，以及左侧栏刷新时的空改动，都不再把只换了一部分的名单存下去。左侧栏自动换算的入口只看名单里还有没有名字，等不等 Git 由 `edit_workspaces` 判断。
- R2（浮层开着时完成的换算不被 `Done` 撤掉）：`Picks` 多记 `base`（浮层打开时这个 workspace 的仓库）；`set_members` 换成 `Workspaces::set_picked`，`Done` 只按打开以来勾上、勾掉的仓库改，其余照文件现在的样子。浮层开着时名单重读或改过（换算、别的窗口加的），`Picks::follow` 把这个 workspace 新得的仓库补进 `base` 并勾上，浮层里能看到；不勾掉它，`Done` 就留着。
- R3（成员浮层按主仓库路径列项）：`Panel::groups()` 换成 `Panel::repositories()`，按主仓库路径一项（`Listed { repo, agents }`），按目录名再按路径排；浮层每个路径一行、分别勾（`toggle_repo`，去掉 `whole`、`toggle_whole`）；`in <名字>` 按这一个路径算。两行目录名相同时，名字后面加一段淡色的上级目录（`~` 开头，超过 24 个字符时用 `card::shorten` 中间省略）。workspace 自己的仓库、`Choose folder…` 选的仓库没有 agent 时也各占一行。左侧栏的分组（P5-80 同名组合并）没动。
- `docs/DESIGN.md` P5-105 的“实际做法”“界面细节”两条按上面改了。

**验证了什么**
- 补的三条回归测试（`app/tests/workspaces.rs`）：
  - R1 `a_change_that_changes_nothing_keeps_no_name_from_waiting_for_git`：两个旧名字一个读到、一个还在等 Git，依次走“仓库在别的 workspace”“读不到仓库”“workspace 已删”三个 P5-93 回调和一次空改动，文件一个字节不变、两个名字都在；等的那个读到后一次存盘，两个仓库都进来、名字清空。
  - R2 `done_keeps_what_names_gave_way_to_while_the_panel_was_open`：按旧名单打开成员浮层，Git 结果到了、自动换算存盘，不改勾选按 `Done`（用打开时的勾选和跟随后的勾选各按一次），原归属都还在；跟随后勾掉再 `Done` 才去掉。
  - R3 `two_repositories_of_one_name_are_ticked_apart_and_the_list_keeps_one_group_of_them`：`/one/demo` 在 One、`/two/demo` 在 Two；不在 workspace 时左侧栏仍是一个 `demo/` 组，浮层是两行；在 One 的浮层按 `Done`、勾掉、再勾上，Two 的那个始终不动，agent 各跟各的仓库。
- RED：R1、R2 把旧逻辑临时放回去跑（存盘条件改回“和读到的不一样就存”，`set_picked` 改回旧 `set_members` 的“清空再按勾选加”），两条都失败：R1 第一个无变化回调就把只换了一部分的名单写进文件、等 Git 的 `ranch/dev` 丢了；R2 `Done` 后只剩 `/w/cairn`，`/w/paddock` 被撤掉。之后恢复原样，用 `cmp` 确认文件一致。R3 用的 `groups()`、`toggle_whole` 已删掉，测试在旧接口上编不过，没跑 RED；旧代码同名合并成一项、一勾挪走两个仓库这一点，审查已经查实。
- 原来的两条跟着改了：`a_repository_is_in_one_workspace_only` 改用 `set_picked`；浮层勾选那条改名 `the_panel_ticks_whole_repositories_and_lists_none_for_agents_that_go_by_a_prefix`，按 `repositories()`、`toggle_repo`、`set_picked` 写。
- `cargo test --test workspaces` 16 条全过（其他测试文件不用这些接口）；`cargo clippy --all-targets -- -D warnings` 没有警告；`cargo fmt --check`、`git diff --check` 都通过。照要求没重跑全套。没起窗口、没截图。

**拿主意的地方**
- R1 没存盘时，内存里也退回读到的样子，不留只换了一部分的结果，和“名字存盘之前不起作用”保持一致。
- R2 两手都做了：`Done` 按差异改，浮层也跟着勾上。只做跟随的话，名单在别处变了而浮层没刷新时还会撤掉；只做差异的话，浮层看不到新进来的仓库。另一个窗口在浮层开着时去掉的仓库，`Done` 也不会加回来。
- R3 每行的个数只数 Git 读到在这个路径里的 agent。按前缀分组的不算：两个同名仓库时，没法知道它属于哪个路径。
- 同名行的上级目录限 24 个字符，超出时中间省略，和卡片路径用的是同一个缩短函数。

**没做的事**
- 浮层同名两行的样子、跟随勾选的效果都没起窗口看，留给用户实际看。
- 没动审查文件 `P5-105-交叉审查.md`。

## 用户看过（10-15）
- 用户重开看过：“我重开看了，S1先不做”。交叉审查的建议改 S1（同名仓库上级目录缩短后可能一样）不做。
