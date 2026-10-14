# 任务：左侧栏的仓库能收起，再加 workspace 把几个仓库归成一个大项目

2026-10-14，paddock/main 交给 paddock/dev-workspace（Claude Code，常规档：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：三项都拿不准，主控定：改的是左侧栏怎么分组、怎么显示，加一份只在本机的分组名单；不碰 corral、agent 本身、并发和核心规则）
类型：功能变更
依据：用户 10-14：“再加一个draft任务，现在的左侧的agents是按照repo来列的。我想记一个所谓的workspace的概念。我是这么想的，这个workspace我可以新建一个workspace。这个workspace下我可能会加多个repo，这多个repo是一个大项目，例如一个workspace下有cairn，paddock，saddle， cairn这几个。然后还有可能没建立workspace的可能就是现在的样子。我还可以新建一个repo，然后选择将现在的那些已经开的repo加入到这个workspace中。如果理解了我的意思的话，就先设计。出设计稿给我。注意下兼容现在的样子（以及前两天修复的委派出去的agent和主控开在同一个repo下）。先设计吧。”看过第一版样稿后：“另外我加几个我的需求：1: 现在的同一个repo下的多个agents可以按照repo收缩起来。workspace我理解也一样的效果 2: 可以选择将已经开的repo下的一个或者多个agents加入到某一个worspace下。”主控出了第二版样稿，列了做法、一处后果（只加主控一个 agent 时，它之后派出去的不会跟进来）和几条建议，用户：“都按你的建议来，写任务文件吧。”看过任务文件和主控另外拿的九条主意：“九条都同意，派出去吧”。本轮只改 paddock 左侧栏怎么分组和显示；不改 agent 的名字，不改 corral，不动看板、Activity、右侧栏。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`（规矩全文）。
- 样稿 `docs/设计稿/P5-89-workspace/`（README 和八个 `.dc.html`；采用的是托盘画法 `1-Tray`，`1b-Flat` 没采用）。配色是近似值，实现时从主题取色；侧栏宽 340、workspace 的名字和卡片上的字都是示意。
- `docs/DESIGN.md` §13 的 P5-80（按仓库分组的规则，本任务不改它）、P5-85（`Choose folder…` 和本机名单的做法）、P5-74（卡片展开箭头，悬停才出的画法）、P5-45（界面文字的大小写）。
- 代码：
  - `app/src/agents.rs`：`Panel::group`（agent 归哪个组）、`Panel::ordered`（顺序）、`move_selection`（上下键）。
  - `app/src/card.rs`：`Line`、`lines`（组头、个数、状态细条怎么来的）、`Details` 和 `Cell`（展开的卡片里那几格）。
  - `app/src/sidebar.rs`：画 `Line::Group` 的那一段、`rail`（收成窄条）和 `RailTip`、卡片详情里 Copy／Pause／Stop… 那一行。
  - `app/src/git.rs` 的 `main_repository`、`app/src/activity.rs` 的 `main_repository`：一个目录属于哪个主仓库，现成的读法。
  - `app/src/kanban_repos.rs` 和 `app/src/kanban_view.rs` 里 `Choose folder…` 那一段：本机名单怎么存、怎么选目录，照这个做法。
  - `app/src/layout_state.rs`：`sidebar_collapsed`、`activity_folded` 是怎么加进布局、旧文件怎么兼容的。
  - `app/src/popover.rs`、`app/src/new_agent_view.rs`、`app/src/text_input.rs`：浮层、菜单行、输入框的现成画法。
  - `app/src/menu.rs`（View 菜单）、`app/src/window.rs` 里命令面板的命令表（搜 `"Fold Agents"`）、`app/src/search.rs`（⌘P 按项目找 agent）。
  - `app/tests/repo_groups.rs`、`app/tests/kanban_repos.rs`、`app/tests/layout_state.rs`：现成测试的写法。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-89-workspace`，分支 `p5-89-workspace`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-89-workspace`。
- 只动 `app/` 下和左侧栏分组、收起、workspace 有关的代码和测试，`docs/DESIGN.md` 和本文件。不依赖 GPUI 的那部分（谁归哪个 workspace、名单怎么存、名字合不合法、行怎么排）放进能单独测的模块，照 `app/src/kanban_repos.rs` 的做法。
- 分两步做、各自提交：先做「仓库收起」，测试过了再做 workspace。

## 现在是怎样
左侧栏按仓库分组（P5-80）：agent 启动目录所属的主仓库是一组，worktree、子目录都算进主仓库；读不到仓库的按名字前缀。组头是仓库名、个数、右边每个 agent 一道状态细条。组不能收起，组上面也没有别的层。

## 要做的

### 第一步：仓库能收起（样稿 5）
- 点仓库小标题，这个组的卡片收起，只剩小标题那一行；再点展开。个数和状态细条照旧，收起后靠细条看里面的状态。
- 平时小标题和现在一样。鼠标停在小标题上，或者这个组收着的时候，个数后面出一个小箭头（展开朝下、收起朝右）；箭头出现不能把名字和个数挤动。
- 收着的组里有活动窗格的那个 agent 时，箭头后面多一个强调色的小圈（和卡片上表示“正在显示”的记号同一个意思）。
- 不自动展开：里面的 agent 变成等待或出错时，组照旧收着，细条变色、铃铛照旧计数。从别处打开这个 agent 的窗格也不展开。
- 上下键在卡片之间移动时跳过收着的组。
- 哪些组收着跟布局一起存，重开还在；旧的布局文件打开是全展开。
- 收成窄条时不管收起：照旧每个 agent 一个格子。
- View ▸ `Fold Agents`（收起卡片详情）是另一回事，不动。

### 第二步：workspace

**是什么**
- workspace 是用户起了名字的一组成员，成员有两种：
  - 整个仓库：按主仓库（主工作区）的路径记。这个仓库现在的和以后在这里开的 agent 都算，worktree、子目录里的也算。
  - 单个 agent：按 agent 的全名记。只算这一个。
- 一个仓库只能在一个 workspace 里，一个 agent 的名字也只能单独记在一个 workspace 里；加到另一个就是挪过去。
- 一个 agent 归哪：先看它的名字有没有被单独记在某个 workspace 里；没有，再看它所属的主仓库在不在某个 workspace 里；都没有，就不在任何 workspace 里。所以整个仓库在甲里、其中一个 agent 被单独加进乙时，这个 agent 在乙，其余在甲。
- 单独记的 agent 停掉以后，记录留着：同名的 agent 再开起来还在这个 workspace 里。
- 读不到仓库、按名字前缀分组的 agent：前缀和某个 workspace 里一个仓库的目录名相同（且只有一个 workspace 有这个名字的仓库）时，跟着那个仓库进 workspace，和 P5-80 的“组名相同就是同一组”一致；否则不在任何 workspace 里。这样的组不能整组加进 workspace，里面的 agent 可以单独加。
- 只存在本机：paddock 状态目录里、布局旁边的一个文件（比如 `workspaces.json`），存取照 `kanban_repos.rs` 的做法。坏文件、读不出来当作没有 workspace。不写进任何仓库，不改 agent 的名字和标签。

**怎么显示（样稿 1，托盘画法）**
- workspace 排在最上面，按名字排；下面是不在任何 workspace 里的组，和现在完全一样。一个 workspace 都没有时，左侧栏和现在一模一样（除了第一步的收起）。
- 每个 workspace 是一个浅底、细边、圆角的托盘。第一行是标题：箭头、workspace 图标、名字、里面 agent 的个数。下面照旧是各组的小标题和卡片，组内怎么分、怎么排都是 P5-80 和现有的规则，卡片本身不变。
- workspace 里整个加进来、现在没有 agent 的仓库，留淡淡的一行：仓库名加 `No agents`。workspace 不因为 agent 都停了就消失；一个成员都显示不出来时，标题下面写一行 `No agents`。
- 同一个仓库的 agent 一部分在 workspace 里、一部分不在时，这个仓库名在 workspace 里和下面各出现一次，各管各的个数和细条（样稿 6）。
- 点 workspace 的标题收起成一行：名字、个数，右边是里面全部 agent 的状态细条（展开时标题上不画细条，细条在各组小标题上）。收起、存取、不自动展开、上下键跳过、有活动 agent 时的小圈，都和第一步的仓库一样。workspace 里的组也能各自收起。
- 收成窄条：workspace 和 workspace 之间、workspace 和下面的组之间用一条比组间分隔更明显的线隔开；悬停说明里的分组名写成 `<workspace> › <组名>`。

**新建（样稿 2）**
- 入口四处：不在 workspace 里的仓库小标题上的按钮菜单里的 `New workspace…`（进来时这个仓库已勾上）；卡片详情 `Workspace` 一格菜单里的 `New workspace…`（进来时这个 agent 已勾上）；View 菜单的 `New Workspace…`；命令面板的 `New Workspace…`。
- 浮层 `New workspace`：
  - `Name`：必填，去掉首尾空白；不分大小写不能和已有的重名。不合格时 `Create` 不能点，下面写一句原因。
  - `Repositories and agents`：左侧栏里现在有的每个组一行。属于某个主仓库的组，行首的勾表示“整个仓库”，行尾是个数和一个小箭头，点开列出这个组的 agent，可以只勾其中几个；只勾了一部分时行首画成“部分”的样子，行尾写 `1 of 2` 这样的数。按前缀分的组没有“整个仓库”的勾，只能点开勾 agent。
  - 已经在别的 workspace 里的仓库或 agent，行尾淡淡写 `in <名字>`；勾上就是挪过来。
  - `Choose folder…`：系统的选目录框，取那个目录所属的主仓库，加成一行并勾上；选的目录不在 Git 仓库里时写一句说明，不加。
  - 列表下面一句说明：`A ticked repository brings every agent in it, and any started there later. Tick single agents to add only those.`
  - `Cancel`、`Create`；Esc 取消，名字合格时 Return 等于 `Create`。

**加进去、挪、移出（样稿 3、6）**
- 不在 workspace 里、属于某个主仓库的组：鼠标停在小标题上，状态细条前面出一个小按钮（提示 `Add to workspace`），点开菜单：`Add <组名> to` 下面列出各 workspace，点一个就把整个仓库加进去；最下面 `New workspace…`。
- workspace 里的组：同一个位置的按钮，菜单是 `Move to` 加其余各 workspace，和 `Remove from <workspace>`。这个组是整个仓库进来的，挪和移出的是整个仓库；是单独加的几个 agent，挪和移出的就是这几个。
- 展开的卡片详情里多一格 `Workspace`：值是所在 workspace 的名字，不在就是 `None`，是个按钮，点开菜单列出各 workspace（现在所在的打勾）和 `New workspace…`。选另一个，就把这个 agent 单独记到那个 workspace。这个 agent 是单独加的时候，菜单里另有 `None`，选了就移出；是跟着整个仓库进来的，没有 `None`（要拿出去，移出整个仓库，或把它单独挪到别的 workspace）。

**workspace 的菜单（样稿 4）**
- 鼠标停在 workspace 标题上，行尾出 `⋯`（提示 `Workspace options`），点开：
  - `Rename`：标题里的名字就地变成输入框，Return 确认、Esc 取消；名字规则同新建。改名后成员和收起状态都跟着。
  - `Add repositories…`：打开和新建同一个浮层，不带名字那一栏，标题 `Add to <名字>`，勾的就是现在的成员，可以加也可以去掉，按钮是 `Done`。单独记着、现在没在运行的 agent 列在最下面 `Not running` 一段，去掉勾就是忘掉它。
  - `Remove workspace`，下面一行小字 `Agents keep running. Its repositories go back to the list.`。点了先在菜单里就地确认一次，再点才去掉。去掉的只是这个分组，agent 不受影响，成员回到下面的列表。

**别处**
- ⌘P 按项目找 agent 时，也认 workspace 的名字；每行写的项目名照旧是组名。
- Attention／收件箱、新标签和分屏弹层、命令面板里 agent 行的项目名，看板、Activity、右侧栏跟焦点，`paddock ctl` 的输出：都不变。

**文档**
- `docs/DESIGN.md` §13 加一条 P5-89：用户原话、做法、上面定下的行为、不做的事；P5-80 那条后面注明分组规则没变，上面多了一层 workspace。

## 用户定了的（10-14：“都按你的建议来，写任务文件吧。”）
1. workspace 是仓库上面的一层；没建 workspace 时和现在一样。
2. 仓库能收起，workspace 也一样；收着的时候里面有 agent 在等也不自动展开。
3. 能加整个仓库，也能只加其中一个或几个 agent。只加了主控一个 agent 时，它之后派出去的 agent 不会跟进来（corral 的公开输出里看不出谁派的谁），想在一起就加整个仓库；用户知道这个结果。
4. 一个仓库、一个 agent 各只能在一个 workspace 里；整个仓库在甲、其中一个 agent 单独加进乙时，这个 agent 在乙。
5. 单独加的 agent 停掉后记录留着，同名的再开起来还在。
6. 画法用托盘。
7. 第一版只管左侧栏；⌘P 多认 workspace 的名字，别处不变。
8. workspace 里没有 agent 的仓库留一行。
9. workspace 排在最上面、按名字排；只存在本机；界面上叫 `Workspace`。
10. 收成窄条时 workspace 之间一条线，悬停说明写 `workspace › 组名`。

## 主控想加、问过用户的（10-14：“九条都同意，派出去吧”）
样稿上没有或看不出来，是主控的主意，用户九条都同意。
1. 读不到仓库、按前缀分组的 agent，前缀和 workspace 里某个仓库同名时跟着进去（沿用 P5-80“组名相同就是同一组”）。
2. `Remove workspace` 要在菜单里就地再点一次确认。
3. `Add repositories…` 用新建的同一个浮层；停掉的单个 agent 的记录在这里的 `Not running` 一段清。
4. 卡片详情的 `Workspace` 菜单里，`None` 只在这个 agent 是单独加的时候给。
5. 新建的入口多两处：View 菜单和命令面板的 `New Workspace…`。
6. 没有 agent 的仓库那一行只是一行字，这一版不给“在这里开 agent”的入口。
7. 收成窄条时不管收起，照旧每个 agent 一个格子。
8. 收起状态跟布局存，workspace 的成员名单另存一个文件。
9. 两样（仓库收起、workspace）放在这一件活里，一个 agent 先后做。

## 怎么算做完
- 用户原话：“这个workspace下我可能会加多个repo，这多个repo是一个大项目”“没建立workspace的可能就是现在的样子”“选择将现在的那些已经开的repo加入到这个workspace中”“兼容现在的样子（以及前两天修复的委派出去的agent和主控开在同一个repo下）”“同一个repo下的多个agents可以按照repo收缩起来。workspace我理解也一样的效果”“可以选择将已经开的repo下的一个或者多个agents加入到某一个worspace下”——上面「要做的」逐条达到，样子和样稿 1、2、3、4、5、6 一致。
- 先写测试、确认它因为功能还没有而失败，再实现。下面几样各有测试即可，不做覆盖矩阵，都用合成的 agent 和临时目录：
  - 没有 workspace、没有收着的组时，`lines` 出来的行和改之前一样。
  - 归属：整个仓库（主仓库里的和 worktree 里的 agent 都进去）；单独记的 agent 先于仓库；都不在的照旧；前缀同名的跟着进。
  - 只能在一个里：仓库、agent 加到另一个 workspace 就是挪。
  - 行的顺序：workspace 在前按名字排，里面按组；没有 agent 的仓库留一行；同一个仓库一部分在 workspace 里时出现两次。
  - 收起：收着的组、收着的 workspace 不出卡片，个数和细条还在；上下键跳过它们。
  - 存取：成员名单写出去再读回来一样，坏文件当作没有；布局里收着的那些，旧文件打开是全展开。
  - 名字：空的、重名的不行。
  - ⌘P 按 workspace 的名字找得到里面的 agent。
- 验证只做这些：在 `app/` 下带上面的 `CARGO_TARGET_DIR` 跑 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`，再跑 `git diff --check`，各一次。觉得不够，在回复里说，不要自己加。
- 不起窗口、不截图。样子和手感留给用户实际看：托盘和卡片的间距、悬停才出的箭头和按钮、三个菜单的位置、新建浮层里点开和勾选、中文输入法下起名字、就地改名、窄条、界面字号调大和侧栏拖窄时的样子。

## 不要做
- 不改 P5-80 的分组规则，不改卡片的内容和画法（只在详情里加 `Workspace` 一格）。
- 不改 agent 的名字和标签，不调 `corral start`／`stop`／`send`／`keys`；不读 Corral 的内部状态目录；`corral ls` 里的都是用户的 agent，不碰。
- 不做：拖动卡片或组来归类、手动调 workspace 的先后、一个仓库或 agent 同时在两个 workspace、workspace 套 workspace、给 workspace 配颜色或图标、按 workspace 过滤看板或收件箱、在“没有 agent 的仓库”那一行开 agent、设置页的入口、`paddock ctl` 的命令。
- 不把 workspace 写进任何仓库，不同步到别的机器。
- 不加依赖；不用 Python；不依赖 ratatui、crossterm；不起 paddock 窗口；不用脚本模拟按键或鼠标；不碰别的仓库。
- 命令里不用 `rm`，不用 `sh -c` 包长命令，前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录。
- 不照抄 Zed 的 `terminal`、`ui` 等 GPL 代码；GPUI 本身的源码可以读来确认接口。
- 遇到要改本文件没列的模块才做得成、GPUI 做不到上面的行为（比如悬停才出的按钮、就地改名）、或者某条规则和 P5-80 现有的行为对不上，停下来报告，等决定，不要自己换别的办法。
- 不合并到 main，不推送。只在 `p5-89-workspace` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

2026-10-14，paddock/dev-workspace。分两步、两次提交，都在 `p5-89-workspace` 上，没合并、没推送。

**做了什么**
- 第一步（提交 `99152d8`）：点仓库小标题收起、再点展开；个数和细条照旧；悬停或收着时个数后面出箭头，位置一直留着；收着的组里有活动窗格的 agent 时多一个强调色小圈；不自动展开；`Panel::move_selection` 跳过收着的组；收起状态进布局文件的 `sidebar_folded`，旧文件打开是全展开；窄条不管收起。
- 第二步：
  - `app/src/workspaces.rs`（新，不依赖 GPUI）：成员名单、归属（单独记的 agent 先于仓库，前缀同名且只有一个 workspace 有才跟着进）、只能在一个里（加就是挪）、名字规则、`workspaces.json` 的存取（坏文件当作没有）、浮层里的勾选（整个仓库／其中几个／部分）。
  - `agents.rs`、`card.rs`：`Folds` 分成不在 workspace 里的组、workspace、workspace 里的组三份；`Line` 多了 workspace 标题、没有 agent 的仓库、空 workspace 三种行；workspace 在前按名字排，同一仓库一部分在里一部分在外时出现两次；卡片多带它在哪个 workspace、是不是单独加的。
  - `sidebar.rs` 和新的子模块 `sidebar_workspaces.rs`：托盘和标题行（收起成一行带全部细条）、没有 agent 的仓库那一行、小标题上的按钮和菜单（`Add <组名> to`／`Move to`／`Remove from <workspace>`／`New workspace…`）、卡片详情的 `Workspace` 一格和菜单、标题上的 `⋯` 菜单（`Rename` 就地改名、`Add repositories…`、`Remove workspace` 点两次）、`New workspace`／`Add to <名字>` 浮层（含 `Choose folder…` 和 `Not running`）、窄条的分隔线和悬停提示 `<workspace> › <组名>`。
  - `menu.rs`、`window.rs`：View ▸ `New Workspace…`（命令面板跟着有），窗口把这个动作交给左侧栏。
  - `search.rs`：⌘P 的 agent 行多搜 workspace 名，行上显示的项目名没变。
  - `docs/DESIGN.md` §13 加了 P5-89 一条，P5-80 那条后面注明分组规则没变。

**验证了什么**
- 在 `app/` 下带 `CARGO_TARGET_DIR=…/.target/p5-89-workspace`，最终代码上：`cargo test --all-targets` 全过（库内 460 个，`tests/workspaces.rs` 9 个，其余各文件都过）；`cargo clippy --all-targets -- -D warnings` 过；`cargo fmt --check` 过；`git diff --check` 过。
- 没做到“各一次”，照实写：第一步提交前跑过一次全量测试；第二步做完跑了一轮，clippy 报了一处类型太复杂（改成类型别名），随后对着任务文件又查出漏了一条（“workspace 不因为 agent 都停了就消失”，原先 agent 全停时列表只画中间那块空状态），改完三条命令又各跑了一遍。开发中还单独跑过 `--test workspaces`、`--test repo_groups`、`--test layout_state`、`--lib menu::` 看红和绿。
- 先红后绿：`tests/workspaces.rs` 的 9 个里 8 个先在没有行为的空壳上因断言失败（“没有 workspace 时行和以前一样”那个本来就该过）；菜单的两个测试先因为没有 `New Workspace…` 失败；“加到不存在的 workspace 不动别处”的断言先失败再修。
- 任务文件列的八样都有测试：行不变、归属、只能在一个里、行的顺序（含没有 agent 的仓库、出现两次）、收起（个数和细条还在、上下移动跳过）、存取（名单读写、坏文件、布局里旧文件全展开）、名字、⌘P。
- **不够的地方**：GPUI 那一层（托盘、三个菜单、浮层、就地改名、焦点和 Esc／Return、系统选目录框回来以后浮层还在不在）没有自动测试，也没起窗口看过，只是编译通过、对着 GPUI 源码核过接口。这部分对不对只能靠用户实际点。

**拿主意的地方**
- 收着的组／workspace 上的小圈，认的是“活动窗格显示的那个 agent”（任务文件的写法），不是这个窗口里开着的所有 agent。
- “上下键跳过”做在 `Panel::move_selection` 里并有测试；现在界面上没有在卡片之间移动的上下键，所以这条眼下只在模型里。
- agent 全停、但有 workspace 时，列表画各个托盘，不画中间那块 `No agents yet` 和 `New Agent` 按钮（一个 workspace 都没有时照旧）。
- workspace 里的组，按钮的悬停提示写的是 `Move or remove`（任务文件只给了外面那个的 `Add to workspace`）；`Remove workspace` 点第一下后那一行变成 `Click again to remove`。名字不合格时的两句：`Name the workspace to create it.`、`Another workspace has this name.`。选的目录不在 Git 仓库时：`<目录名> isn't in a Git repository`（和看板那句一样）。菜单里 workspace 名后面淡淡写它有几个仓库（`4 repos`，照样稿 3）。
- workspace 里的组，按钮挪和移出的是：这个 workspace 里目录名等于组名的整个仓库，加上这个组里单独记着的 agent。没有 agent 的仓库那一行没有按钮，要去掉走 `Add repositories…`。
- 浮层里勾“整个仓库”时，已经单独记在别的 workspace 里的 agent 不跟着过来，除非把它自己也勾上。
- `workspaces.json` 是一个数组；每次改之前先重读，每读一次 corral 也重读（别的窗口可能改过）；写失败在页脚那条提示里说。加到一个已经不存在的 workspace 不做任何事。
- 布局文件里收起状态是 `sidebar_folded {groups, workspaces, within}`；改名、去掉 workspace 时收起状态跟着改。
- 浮层、菜单、改名框在点外面或键盘焦点离开时关掉；窗口退到后面时（比如系统选目录框在前面）浮层留着、勾的不丢。就地改名时点别处算取消。
- 从 View 菜单或命令面板打开时侧栏如果收成了窄条，浮层照样出在窄条上。
- 浮层打开时，只勾了一部分 agent 的组先展开着。
- `search::Row` 多了一个不显示、只参与搜索的字段 `also`。
- 托盘里的卡片比外面窄，卡片上按宽度裁字的地方按窄的算。
- 比任务文件列的多一个测试（浮层里勾选的规则），因为那段逻辑不小。

**没做的事**
- 任务文件「不要做」里的都没做：拖动归类、手动调先后、同时在两个 workspace、workspace 套 workspace、颜色或图标、按 workspace 过滤看板或收件箱、在“没有 agent 的仓库”那一行开 agent、设置页入口、`paddock ctl` 命令；没改 P5-80 的分组规则，没改卡片的内容和画法（只加了 `Workspace` 一格），没碰 corral 和用户的 agent，没加依赖。
- 没起窗口、没截图。留给用户实际看的：托盘和卡片的间距、悬停才出的箭头和按钮、三个菜单的位置、新建浮层里点开和勾选、中文输入法下起名字（选字时的 Return 归不归输入法）、就地改名、窄条、界面字号调大和侧栏拖窄时的样子。
- 没合并、没推送、没收尾；`HANDOFF.md` 没动。
