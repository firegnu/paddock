# P5-29r2：Kanban 的“基本看板功能”有哪些，paddock 该做哪几样、哪里停

2026-10-07，paddock/research-kanban-basics。只读调研：读公开网页、文档、开源仓库的 README 和源码，本机 Drover 仓库和 Saddle 仓库（提交 `ff3c441` 之前）的源码和文档；没有注册、登录、安装任何产品，没有在 Drover、Saddle 里编译或运行。任务文件：`docs/任务/P5-29r2-Kanban基本功能调研.md`。

标注：
- **已查证**：看过文档或源码，后面附来源。
- **推测**：根据已查证的内容推出来的，没有直接依据。
- **按代码推断**：读 paddock 源码推出来的，没在真窗口里看。
- **不知道**：查过没找到。

上一轮 `docs/调研/P5-29r-Kanban集成.md` §2 查过的各家 agent 看板做法不再重查，这里只补“基本操作”这一面。

---

## 0. 一页结论

1. **看板的底线就五样**：按列分组看卡片；新建卡片；把卡片移到别的列（多数靠拖）；同一列里排先后；完成的卡片归档（移出看板但留着）。打开卡片看和改内容几乎都有，算第六样。标签、负责人、截止日、筛选、WIP 上限、泳道这些常见但可选；自动化规则、队列调度、统计报表、通知是重的（§1）。
2. **Drover 重在“另起一套任务数据和流程”**：自己的队列文件 `queue.md`、只追加的状态文件 `tasks.state`、四态状态机（待派→进行中→待验收→完成，可退回）、每次写都要令牌和确认、单进程独占数据的锁、主控登记、派发、通知、遥测、给主控用的 JSON 控制接口，以及更早的常驻引擎和 launchd。插件版约 1.1 万行 Rust。用户说的“重”，主要是这些和主控在终端里的流程重复的部分（§2）。
3. **对 paddock 的推荐**：卡片＝任务文件、状态从 git 和 corral 推出来的前提不动，只补两样能动手的，加两条约定：
   - **新建卡片**＝在主仓库工作区写一份**未提交**的任务文件草稿（标题＋用户原话），显示成已定的 C 草稿卡，交给主控补全。只新建文件，不碰已有文件，不提交（§3.2）。
   - **打开、编辑**＝照旧用“Open task file”打开文件，不做卡片里的编辑器（§3.5）。
   - **归档“不做了”的活**＝一条约定：主控在 main 上补空提交 `收尾: <编号> 不做：<原因>`，看板把它放进 DONE、标成 Dropped。看板本身只多认一个词（§3.6）。
   - **拖动**：先不能拖。推错的情况现在主要是约定定下之前的老任务，由主控补一批收尾空提交一次性修好，不需要拖来纠正（§3.3、§3.4）。
   - **列内排先后**：先不做。要做也只存在 paddock 本机，主控看不到；一旦让主控照这个顺序派活，就是 Drover 的 `queue.md`（§3.4）。
4. **不做的线**（§4）：自己的状态或队列文件、拖了就派活、看板上合并和收尾、状态机加确认令牌、常驻自动推进、看板专属通知、统计和记账、项目登记、跑验收命令、给主控的控制接口。
5. **要问用户的问题**五个，见 §5。

---

## 1. “基本看板功能”通常指什么

### 1.1 看了哪些

- 通用看板：Trello、GitHub Projects（看板视图）、Linear（board layout）、Jira（Kanban 看板和 Kanban backlog）。
- 本地或文件型：Obsidian Kanban 插件（看板就是一份 Markdown 文件）、Backlog.md（一个任务一份 Markdown 文件）。任务文件点名的 Kanboard、Planka 也看了，但它们**不是** Markdown 文件型：Kanboard 存 SQLite／MySQL／PostgreSQL（[requirements](https://docs.kanboard.org/v1/admin/requirements/)），Planka 存 PostgreSQL（[docker-compose.yml](https://github.com/plankanban/planka/blob/master/docker-compose.yml) 的 `DATABASE_URL`）。这里只用它们作“自托管看板有哪些功能”的样本。
- agent 看板：Vibe Kanban（现版文档），只补基本操作。

### 1.2 功能表

“几乎都有”＝上面八家里至少七家有。每格写做法，“—”是没找到或不适用。

**第一档：几乎都有（底线）**

| 功能 | 大致怎么做 | 依据 |
| --- | --- | --- |
| 按列看卡片 | 列＝一个状态字段的取值（GitHub、Linear、Jira、Vibe、Backlog.md 的 `statuses`），或者就是一组卡片（Trello 的 list、Obsidian 的二级标题）。 | 全部已查证，见下 |
| 新建卡片 | 列底或列头一个 “+”/“Add a card”，只填标题就建好，详情以后再补。Trello 在列底、列头或两卡之间都能加；GitHub 在看板底部敲一行建 **draft issue**（只存在项目里，要转成 issue 才能挂仓库、标签）；Linear、Vibe 列头 “+”；Kanboard 列名旁 “+”；Obsidian 在列底加一行；Backlog.md 用 `backlog task create`，可加 `--draft` 先建草稿、以后 `draft promote`。 | 已查证：[Trello Add a card](https://support.atlassian.com/trello/docs/adding-cards)、[GitHub adding items](https://docs.github.com/en/issues/planning-and-tracking-with-projects/managing-items-in-your-project/adding-items-to-your-project)、[Linear board layout](https://linear.app/docs/board-layout.md)、[Vibe Issue Management](https://vibekanban.com/docs/issue-management.md)、[Kanboard tasks](https://docs.kanboard.org/v1/user/tasks/)、[Backlog.md CLI](https://github.com/MrLesk/Backlog.md/blob/main/CLI-INSTRUCTIONS.md)「Draft Workflow」 |
| 移到别的列 | 拖过去，状态字段就跟着改（GitHub：“the value of those items will adjust to match the column”；Jira：拖＝做一次状态转换；Linear、Vibe 同样）。Trello 另有卡片菜单 “Move”；Backlog.md 命令行改 `-s`，网页看板可拖、可多选一起拖。 | 已查证：[GitHub board layout](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-board-layout)、[Trello Move cards](https://support.atlassian.com/trello/docs/moving-cards-or-lists)、[Backlog.md README](https://github.com/MrLesk/Backlog.md)、Linear、Vibe 同上 |
| 同一列里排先后 | 拖上拖下。多数家“手动排序”和“按字段排序”二选一：GitHub 设了排序就不能手动排（“When a board is sorted, you cannot manually reorder items within a column”）；Linear 按属性排序时位置自动定，手动时可用 ⌥⇧↑↓ 移到列首列尾；Vibe 要切到 Manual；Jira 在 backlog 里拖或右键“移到顶/底”定 rank；Backlog.md 存一个 `ordinal` 字段；Obsidian 就是文件里的行序。 | 已查证：GitHub 同上、[Linear board ordering](https://linear.app/changelog/2022-08-18-board-ordering)、Vibe 同上、[Jira Kanban backlog](https://support.atlassian.com/jira-software-cloud/docs/use-your-kanban-backlog/)、Backlog.md CLI（JSON 字段列表里有 `ordinal`）、Obsidian 见下 |
| 归档（移出看板但留着） | Trello 归档后不加载、可从 “Archived items” 恢复，删除才是永久的；GitHub 归档后可恢复，还能设内置工作流自动归档；Obsidian 归档到同一文件末尾的 `## Archive` 一节，勾选框可一键归档；Backlog.md 分两种：`task complete` 把做完的移出看板留档，`task archive` 用于作废、重复的；Kanboard 关闭（close）后从看板隐藏，可搜 `status:closed` 找回。 | 已查证：[Trello archive](https://support.atlassian.com/trello/docs/archiving-and-deleting-cards)、[GitHub archiving](https://docs.github.com/en/issues/planning-and-tracking-with-projects/managing-items-in-your-project/archiving-items-from-your-project)、[Obsidian 源码 list.ts](https://github.com/mgmeyers/obsidian-kanban/blob/main/src/parsers/formats/list.ts)、[Obsidian 卡片勾选框](https://github.com/mgmeyers/obsidian-kanban/blob/main/docs/Settings/Display%20card%20checkbox.md)、Backlog.md CLI「Complete／Archive」、Kanboard tasks 同上 |
| 打开、编辑卡片 | 点开一个详情面板改标题和正文（Trello、Linear、Vibe、GitHub、Jira）；文件型的直接开文件（Obsidian 卡片可“New note from card”生成并链到一篇笔记；Backlog.md 终端看板按 `E` 用编辑器打开）。 | 已查证：[Obsidian Create notes from cards](https://github.com/mgmeyers/obsidian-kanban/blob/main/docs/How%20do%20I/Create%20notes%20from%20cards.md)、Backlog.md CLI「Board Operations」、Linear、Vibe 同上 |

Obsidian 的数据格式（已查证，[list.ts](https://github.com/mgmeyers/obsidian-kanban/blob/main/src/parsers/formats/list.ts)）：一个看板一份 `.md`，标题是列，列下的列表项是卡片，归档是文件末尾 `## Archive` 一节，看板设置存在文件尾部的代码块里。仓库现在被转到 `community-archive/obsidian-kanban`，README 写着在找新维护者。

删除：Trello、GitHub 都有，但都把“删除”放在“归档”后面、明说不可恢复。删除不算看板的核心操作（推测）。

**第二档：常见但可选**

| 功能 | 大致怎么做 | 依据 |
| --- | --- | --- |
| 标签、优先级、负责人、截止日 | 卡片上的字段。Backlog.md 的任务字段里有 `labels`、`priority`、`assignees`、`dueDate`（CLI 文档的 JSON 字段表）；Vibe 有优先级四档和 tags；Obsidian 在卡片文字里写日期、时间、`#tag`。 | 已查证：Backlog.md CLI、Vibe、[Obsidian docs 目录](https://github.com/mgmeyers/obsidian-kanban/tree/main/docs/How%20do%20I) |
| 筛选、搜索 | GitHub、Linear、Jira 按字段筛；Backlog.md `backlog search` 模糊搜；Obsidian 有看板内搜索。 | 已查证：同上 |
| WIP 上限 | 每列一个上限，超了只是变色或加粗，不拦。GitHub 叫 column limit；Obsidian 在列名后写 `(5)`，超了计数变粗；Jira 列头显示“当前/上限”。 | 已查证：GitHub board layout、[Obsidian Set a WIP Limit](https://github.com/mgmeyers/obsidian-kanban/blob/main/docs/How%20do%20I/Set%20a%20WIP%20Limit.md)；Jira 只见搜索摘要 |
| 隐藏、折叠列 | Linear 可隐藏列；Vibe 默认隐藏 Backlog、Cancelled；paddock 已经能折叠、DONE 默认收起。 | 已查证：Linear、Vibe 同上 |
| 依赖、子任务 | Backlog.md 有 `--dep`、依赖图、`isReady`；Vibe 有子 issue；paddock 已有 `依赖：` 一行。 | 已查证：Backlog.md CLI、Vibe 同上 |
| 泳道 | Linear、Jira、GitHub（slice/group）、Kanboard 有。 | 已查证：Linear、GitHub、Kanboard 同上 |
| 多选批量移动 | Trello 最多 20 张、Backlog.md 网页、GitHub。 | 已查证：Trello、Backlog.md、GitHub 同上 |
| 模板、评论、附件 | Trello 模板卡；Planka 评论、附件、自定义字段；Backlog.md 评论只追加。 | 已查证：Trello、[Planka README](https://github.com/plankanban/planka)、Backlog.md CLI |

**第三档：重的**

| 功能 | 大致怎么做 | 依据 |
| --- | --- | --- |
| 自动化规则 | 事件→动作。Kanboard：移到 Done 自动关闭、移到某列自动指派、完成后复制到别的项目；GitHub：关闭或合并时设 Done、自动归档；Backlog.md：`onStatusChange` 状态变了跑一条 shell 命令（文档例子是“变成 In Progress 就启动 claude”）。 | 已查证：[Kanboard automatic actions](https://docs.kanboard.org/v1/user/automatic_actions/)、GitHub（上一轮 §2.9）、Backlog.md（上一轮 §2.4） |
| 队列调度、自动派发 | 拖到 In Progress 就开 agent（AgentsRoom）；常驻服务轮询 tracker、按状态开停 agent（Symphony）。 | 上一轮 §2.2、§2.11 |
| 统计报表 | Kanboard 六种：各列任务分布、累积流图、燃尽图、每列平均停留时间、前置时间和周期时间等。 | 已查证：[Kanboard analytics](https://docs.kanboard.org/v1/user/analytics/) |
| 通知 | Planka 接 100 多种通知渠道、webhook；Trello、Jira 自带。 | 已查证：Planka README |
| 多人实时协作、权限、外部同步、时间追踪 | Planka 实时同步、2FA、SSO（Pro）、秒表；Kanboard 记录每次移列的历史。 | 已查证：Planka README、Kanboard tasks |

### 1.3 从表里看出的两点（推测）

- **底线里只有“移到别的列”和“列内排序”会和“状态是推出来的”冲突**：其余几样（看、新建、归档、打开）不改状态，或只是加一份新东西。几家对这类冲突的做法都是“二选一”：GitHub、Linear、Vibe 一设了自动排序就不许手动排；GitHub 的内置工作流在关闭或合并时直接设 Status，不看人之前拖到哪。本轮看到的几家里，没有一家让手动和自动长期同时管同一个字段。
- **文件型看板都给“草稿”留了位置**：GitHub 的 draft issue、Backlog.md 的 `--draft`、Obsidian 卡片先是一行字、要时再“New note from card”。新建先轻后重，是这类工具的共同做法，和 paddock 已定的 C（未提交的任务文件显示成 DRAFT 卡）正好对上。

---

## 2. Drover 重在哪里

来源：本机 `../drover`（独立仓库，477 个提交，2026-09-30 归档，README 指向 Saddle 插件）；Saddle `ff3c441^` 的 `plugins/drover/`（README、`src/core.rs`、`src/queue.rs`）。用户 10-05 在 Saddle 的 `ff3c441`（“移除遥测 Drover 与插件系统”）里砍掉了它。

### 2.1 事实

**两代形态**
- 独立版（Python，`bin/drover_core.py` 456 行、`bin/drover-board` 1529 行）：定位是“外循环”，把一条任务队列“一件接一件地送进一个正在工作的 agent 主控，人不在场时也继续往前走”。常驻引擎 `drover loop` 每 5 秒一跳，可装成 launchd 服务、登录自启、退出自动重启。（已查证：`../drover/README.zh-CN.md`、`docs/手册.md` §1、§3）
- Saddle 插件版（Rust，`plugins/drover/src` 约 11,000 行，其中 `queue.rs` 2,896 行、`core.rs` 1,239 行、`plugin.rs` 1,061 行、`detail.rs` 916 行；Saddle 里有 20 个提交改它、49 个提交说明提到 Drover、14 份 Drover 相关任务文件）。2026-09-30 迁入，10-04 还在加遥测展示，10-05 砍掉。（已查证：`git -C saddle log ff3c441^ -- plugins/drover`）

**数据**（已查证：插件 README、`core.rs`）
- 每个项目的交接目录里：`queue.md`（待办，人写，`## 标题` 一块）、`tasks.state`（每行一个 JSON 事件，只追加：`start`、`submitted`、`accepted`、`returned`、`drop`，加旧版的 `done`、`go`）、`paused` 文件。
- 每个项目根目录一份 `.drover.conf`（`HANDOFF_DIR`、`MAIN_AGENT`、`CHECK_CMD`、`DONE_MARK`、`TASK_GATE`、`TELEMETRY_RECORD`），新接入时写进 `.gitignore`。
- 用户目录 `~/.drover/projects` 登记项目，`notifications.json` 存通知偏好。
- 锁：`.tasks.lock`（每次写）、`~/.drover/.plugin-owner.lock`（“每个用户的数据只允许一个 Drover 插件进程持有，第二个实例会明确失败”）、`.projects.lock`。写文件一律临时文件＋fsync＋改名。

**状态和流程**（已查证：插件 README「界面操作」、`core.rs` 的 `fold`）
- Pending → Running（显式派发）→ Awaiting release（Submit for review）→ Done（Accept）；Running 或 Awaiting 可以退回 Pending，必须填原因并确认“工作已停止”；Pending 可以 Drop。同一时刻最多一个 Running 或 Awaiting。
- 每次退回后再派是一个新 run，旧 run 存进 `previous_runs`。
- 每个写操作要带读的时候拿到的 `target_token` 或 `queue_token`，令牌过期要重读、重确认，“禁止默默替换令牌重试”。

**能做的事**（已查证：插件 README）
- 待办的添加、编辑、排序、删除；多项目切换和“All pending”；任务详情（正文、Run details、Links）。
- 显式派发：把选中的 Pending 用 `corral send` 送给配置的主控；没配主控就登记 Running、给出文本让人手动贴。
- 通知：系统通知（osascript）或 Saddle 内通知，有偏好页、去重基线。
- 遥测：派发时可选记录，Accept/Return 时关 trace；详情页链到 Saddle 的遥测查询页，显示主控的审查和收尾报告。
- 给主控用的控制接口：`saddle ctl plugin --plugin drover --method list|show|add|edit|move|drop|pause|resume|dispatch|submit|accept|return|register|…`，带请求 ID 去重、回执、48 KiB 上限。
- 在 Tasks 里接入项目：浏览目录、选主控、写 `.drover.conf`。

**独立版里后来退役的部分**（已查证：`docs/手册.md`，开头写明“旧 gate/loop/人工覆盖行为已退役”）
- 自动判断“做完了”：三道门（main 前进、本次分支都合进 main、`CHECK_CMD` 退出码 0）加一条依据（`base_sha..main` 里有收尾空提交，还要过“只有一个父提交、空提交、前缀对”四道闸）。
- 三档模式：手动、自动记完成但等人放行、全自动发下一件。

### 2.2 判断：哪些是“重”

用户 10-05 的原话是“遥测和drover也只是我臆想出来的功能……绑定很深的遥测，sdk还有插件以及drover什么的，只是我觉得有用罢了”。下面是我的判断（推测，用户没有逐条说过）：

1. **第二份任务数据**（`queue.md`＋`tasks.state`＋`.drover.conf`＋项目登记）。paddock 的任务文件和 git 已经是一份；Drover 等于把“这件活到哪了”在 git 之外又记了一遍，所以才需要锁、令牌、事件折叠、旧格式兼容。这是体量最大的来源（`core.rs`、`queue.rs` 两个文件四千多行）。
2. **在界面里重做主控的流程**：派发、提交待审、验收、退回，每步都要确认。主控本来在终端里按 corral-dispatch 做这些，Drover 又在旁边做了一套，两边还得对齐（令牌、run、binding）。
3. **常驻和独占**：常驻引擎、launchd、单进程持有数据、只允许一个插件实例。和 Saddle TUI 不能同时跑（`docs/背景与决策记录.md` §6）。
4. **外挂的系统**：遥测记录和查询链接、系统通知、`saddle ctl` 控制接口。
5. **自动判断“做完”**，后来自己退役了（手册里“为什么不能用别的信号”一节列了三个被真实流程打穿的信号）。

Drover 里**不重**、也正是 paddock 已经在做的：收尾空提交当“做完”的依据、只读 git、不解析主控写的话。paddock 的 DONE 列就是这条思路。

---

## 3. 对 paddock 的推荐

### 3.1 前提和一个要先知道的坑

前提照 DESIGN §13 P5-29：卡片＝`docs/任务/` 里的一份任务文件；五列全由任务文件、git、corral 推出；不派活、不合并、不收尾；不新增数据。

**坑：在主仓库工作区里改已经提交的任务文件，会挡住主控合并。** 分支会在同一份任务文件末尾追加「完成记录」，合并时 git 要改这个文件；如果 main 工作区里这份文件有没提交的改动，`git merge` 会报“Your local changes … would be overwritten by merge”并停下（git 的标准行为，推测会在这里发生；本轮没试）。所以看板上任何“写回”都只能**新建文件**，或者写在仓库外面，不能改已有的任务文件。“Open task file”现在打开的正是主仓库工作区里的那份（`kanban_view.rs` 的 `actions`：`b.repo.join(kanban::TASKS).join(&card.file)`，按代码推断），用户在那里改了没提交，也会碰到这个坑。

### 3.2 新建卡片：做

- **样子**：看板顶上（或 QUEUED 组头）一个 “New task”。弹出一个小框：编号（预填下一个空编号，可改）、标题、一段“用户原话”。确定后在**主仓库工作区**写 `docs/任务/<编号>-<标题>.md`：首行 `# 任务：<标题>`，下面一节「用户原话」照抄用户写的，别的节留给主控。不提交。写完它就是已定的 C 草稿卡（淡色 DRAFT）。
- **数据**：就是这个新文件。没有别的状态。
- **和推出来的状态打架吗**：不打架。它没提交，main 上没有，推不出任何列；C 已经规定这种文件显示成 DRAFT。
- **和主控流程打架吗**：基本不打架，主控本来就是先写任务文件、给用户看过再提交。要防三件事：
  1. 编号撞车：写之前查 main、工作区、各分支上有没有同编号；有就不让建。
  2. 被扫进别的提交：主控如果用 `git add -A` 会把草稿一起提交。AGENTS.md「看板约定」加一句“主仓库工作区里没提交的任务文件是草稿，提交时按路径加”。
  3. 格式两套：任务文件的完整格式由主控和 ranch 里的 corral-dispatch 技能定。paddock 只写首行和「用户原话」，不写「在哪里干活」等节，免得和技能的模板分叉。
- **做还是不做**：建议做。理由：这是底线里唯一缺的、又不碰推出来的状态的一样；用户的原话能原样进任务文件，正合“验收照抄用户原话”。
- **工作量（推测）**：小到中。一个弹出框（paddock 已有 New Agent 窗口和窗口内弹出框可参照）、编号建议和撞车检查（读 git，`kanban::read` 已经在读）、写一个新文件、测试。前提是 C 先做完。

### 3.3 移动卡片：先不能拖

列是推出来的，“拖”在 paddock 里只有四种意思：

| 意思 | 数据写在哪 | 代价 | 建议 |
| --- | --- | --- | --- |
| a. 不能拖（现状） | 不写 | 推错时看板上改不了，要靠约定和主控修（§3.6、§3.7） | **采用** |
| b. 只在 QUEUED 里排先后 | 只能写在 paddock 本机（仓库外的一个文件，按仓库路径和编号记顺序）。写进仓库就要每拖一次在 main 上提交，还会改多份任务文件，撞上 §3.1 的坑 | 顺序只有用户自己看得见，主控看不到；要让主控照着派，就得让主控读这份顺序，那就是 Drover 的 `queue.md` | 先不做，见 §3.4 |
| c. 拖来纠正推错的状态 | 本机一份“人工覆盖”文件，或者任务文件里一行状态 | 两份状态要对账：覆盖什么时候失效（推出来的状态变了以后？返工时？），推测：GitHub 的内置工作流在关闭或合并时把 Status 设成 Done，不看人之前拖到哪。改任务文件又撞 §3.1 的坑 | 不做。现有推错的都能用约定修（§3.7） |
| d. 拖了就派活 | 要么不写（直接 `corral start`），要么写状态 | 界面和主控两条派活的路并存，同名 agent、同一 worktree 互相踩；就是上一轮的方案三、就是 Drover | 不做（§4） |

### 3.4 列内排先后：先不做

- 现在 QUEUED 按编号排，编号是主控按顺序起的，大体就是派活顺序；真有先后约束的，已经有 `依赖：` 一行，卡片上写 “Waits for …”。
- 要做只能是 §3.3 b：存在本机、只给用户自己看。价值有限：主控看不到，派活顺序还是用户口头说了算。
- 如果用户想要“我排好、主控照着派”，那就需要一份主控也读的顺序，跨过了 §4 的线，要用户明确拍板。
- 工作量（推测）：本机顺序版是中等（GPUI 里做拖放、存一份文件、编号增删时对齐）。

### 3.5 打开、编辑卡片：照旧

- 用“Open task file”用系统默认程序打开。不做卡片详情（已定 B 不做）、不做卡片内编辑。
- 建议在打开已提交的任务文件时提醒一句会碰到 §3.1 的坑？这是主控视角的建议，不是用户验收，列在 §5 Q5 让用户定。
- 工作量：0。

### 3.6 归档“不做了”的活：一条约定，看板多认一个词

- 现在没有办法表示“这件活不做了”：任务文件留在 main 上，就一直挂在 QUEUED（或停在哪一列算哪一列）。其他工具都有归档或“作废”（Backlog.md 的 `task archive` 专门给作废的活）。
- **样子**：主控在 main 上补一条空提交，首行 `收尾: <编号> 不做：<原因>`。看板照旧按收尾提交把它放进 DONE，看到“不做”两字就在卡片上标 Dropped（淡色）。
- **数据**：一条空提交，和现在的收尾记号同一种东西，没有新文件。
- **打架吗**：不打架。DONE 本来就由收尾提交决定，`wrapped_id` 已经能认出 `收尾: <编号> …` 的编号（`kanban.rs`）。
- **做还是不做**：建议做。理由：底线里的“归档”在 paddock 本来就有（DONE＋折叠），缺的只是“没做完就收掉”这一种。
- **工作量（推测）**：小。看板认一个词、卡片多一个状态字、一条测试；AGENTS.md「看板约定」加一句。
- 备选：任务文件里加一行 `不做：<原因>`。不推荐：要改已提交的任务文件，碰 §3.1 的坑；还要主控另提交一次，不比空提交省事。

### 3.7 现有的推错：主控补一批收尾空提交

按代码推断（`kanban::column`，没在真窗口看），约定定下之前的老任务会推错：

- 收尾提交一次写了好几件、或者没写编号，`wrapped_id` 只认第一个编号：例如 `收尾: P5-3 Command palette、P5-8 …、P5-9 …`、`收尾: P5-4 弹出界面……、P5-10 …、P5-11 …`。
- 结果：P5-8、P5-9、P5-10-11、T76 的任务文件在 main 上有「完成记录」、没有对应的合并和收尾提交，会停在 **TO REVIEW**；P4-3、P5-2、P5-4a、P5-4b、P5-4c 有“合并 <编号>”、没有自己的收尾提交，会停在 **MERGED**。共 9 张。
- 建议：主控在 main 上补 9 条空提交 `收尾: <编号> （补记，原收尾见 <提交>）`，一次修好，零代码。以后照「看板约定」写就不会再出现。
- 备选：看板认“一条收尾提交里的多个编号”。不推荐：要猜“、”后面哪些是编号，和 `P5-24a` 不能当成 `P5-24` 的规则冲突，换来的只是修历史。

### 3.8 第二档里值得说一句的

| 功能 | 建议 |
| --- | --- |
| 筛选、搜索 | 不做。一个仓库七十来份任务文件、DONE 只留 5 件，看得过来。 |
| WIP 上限 | 不做。并行开几个 agent 是主控按任务定的，不是看板该管的。 |
| 标签、优先级、负责人、截止日 | 不做。都要往任务文件里加字段，主控得跟着写。“交给谁”已经由 agent 显示。 |
| 折叠列、依赖 | 已有。 |
| 泳道、多选、模板、评论 | 不做。 |

### 3.9 最小功能集一览

| 底线功能 | paddock 里 | 状态 |
| --- | --- | --- |
| 按列看卡片 | 五列，全推出来 | 已有（P5-29a） |
| 新建卡片 | “New task”写未提交的任务文件草稿，显示成 DRAFT | **建议做**，在 C 之后 |
| 移到别的列 | 不能拖 | 不做 |
| 列内排先后 | QUEUED 按编号，`依赖：` 表示先后 | 不做 |
| 归档 | DONE＋折叠；“不做了”用 `收尾: <编号> 不做：…`，标 Dropped | **建议做**（约定＋小改） |
| 打开、编辑 | Open task file | 已有 |
| （修历史） | 主控补 9 条收尾空提交 | **建议做**（零代码） |

---

## 4. 不做的线

下面任何一条一做，看板就开始像 Drover：

1. **自己的状态或队列文件**（像 `queue.md`、`tasks.state`、`.drover.conf`）：有了第二份数据就要锁、令牌、对账、格式兼容，这正是 Drover 最大的一块。
2. **拖了就派活、或卡片上的“Dispatch”按钮**：界面和主控两条派活的路并存，同名 agent、同一 worktree 会互相踩；主控在终端里已经在派。
3. **看板上合并、收尾、提交待审、验收、退回**：这是在界面里重做主控（DESIGN §13 P5-29 已定不做），Drover 为这套确认流程写了令牌和 run 绑定。
4. **人工覆盖推出来的状态（拖来纠正）**：两份状态必然要定“谁说了算、什么时候失效”，Drover 的状态机就是从这里长出来的；推错的用约定修。
5. **让主控读看板上的顺序或标记**：看板就从“视图”变成了“主控的输入”，那是 Drover 的队列。
6. **常驻引擎、后台自动推进、launchd**：paddock 关着时什么都不该发生。
7. **看板专属通知**：“Needs you”标记（已定 A）够用；agent 的提醒已有 Attention。Drover 为通知做了偏好页、去重基线、系统通知线程。
8. **统计、记账、遥测**：用户 10-05 已经砍掉遥测；看板不记时间序列。
9. **项目登记、多仓库切换**：已定只看焦点窗格的仓库（E）。Drover 有自己的项目登记和接入页。
10. **跑验收命令、自动判断“做完”**：Drover 自己试过又退役了；paddock 只认收尾提交。
11. **给主控或脚本用的看板控制接口**：主控直接用 git 和 corral，不需要绕看板。

---

## 5. 要问用户的问题

**Q1. 要不要在看板上新建任务？**
- a. 要：“New task”填编号、标题和你的原话，在主仓库工作区写一份不提交的任务文件草稿，显示成 DRAFT，主控接着补全再提交。
- b. 不要：任务还是你口头说、主控写。
- c. 要，但只记一句想法，不生成任务文件（得另找地方存，等于新增数据）。

建议 **a**，排在 C（草稿卡）之后。

**Q2. 拖卡片在 paddock 里该是什么意思？**
- a. 不能拖（现状）。推错的用约定修（Q4）。
- b. 只能在 QUEUED 里排先后，顺序只存在本机，主控看不到。
- c. 拖来纠正推错的状态，覆盖存在本机。
- d. 拖到 IN PROGRESS 就派活。

建议 **a**。b 价值有限，c、d 都会长出第二份状态（§3.3、§4）。

**Q3. “不做了”的活怎么收掉？**
- a. 主控补一条空提交 `收尾: <编号> 不做：<原因>`，看板放进 DONE、标 Dropped。
- b. 任务文件里加一行 `不做：<原因>`，主控提交。
- c. 看板上点一下“隐藏”，只存在本机。
- d. 不需要，没做的任务文件删掉就行。

建议 **a**：和现有收尾记号同一种东西，不改任务文件，不新增数据。

**Q4. 约定之前的 9 件老任务（§3.7）会停在 TO REVIEW 或 MERGED，怎么办？**
- a. 主控在 main 上补 9 条收尾空提交，一次修好。
- b. 看板去认“一条收尾提交里写了好几个编号”的老写法。
- c. 不管，旧卡留着。

建议 **a**，零代码。（这 9 件是读代码推断的，修之前请你在真窗口里看一眼它们是不是真在这两列。）

**Q5. 在主仓库工作区里改已提交的任务文件会挡住主控合并（§3.1），要不要管？**
- a. 不管，你和主控都知道就行（写进 AGENTS.md 一句）。
- b. “Open task file”在任务还没合并时，改为打开 worktree 里分支上的那份。
- c. 打开前提示一句“改这里会挡住合并”。

建议 **a**：先写进约定，等真碰上再考虑 b。

---

## 6. 不知道、没核实

- Jira 的 WIP 上限只见搜索摘要，没取到 Atlassian 官方页面正文；Kanban backlog 页面已查证。
- Obsidian Kanban 的文档站 `publish.obsidian.md/kanban` 取不到正文，改读了仓库里的 `docs/` 和解析源码。
- Kanboard 里任务在列内的顺序怎么调，文档摘要里没写清楚。
- §3.1 “未提交改动挡住合并”是 git 的标准行为，本轮没有动手试。
- §3.7 的 9 张卡是读 `kanban::column` 推出来的，没有在真窗口里看。
- 各工具的数字（卡片上限、Trello 一次多选 20 张）以官方页面当时的写法为准，可能会变。

---

## 7. 来源

- Trello：[Add a card](https://support.atlassian.com/trello/docs/adding-cards)、[Move cards or lists](https://support.atlassian.com/trello/docs/moving-cards-or-lists)、[Archive or delete a card](https://support.atlassian.com/trello/docs/archiving-and-deleting-cards)
- GitHub Projects：[Customizing the board layout](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-board-layout)、[Adding items](https://docs.github.com/en/issues/planning-and-tracking-with-projects/managing-items-in-your-project/adding-items-to-your-project)、[Archiving items](https://docs.github.com/en/issues/planning-and-tracking-with-projects/managing-items-in-your-project/archiving-items-from-your-project)
- Linear：[Board layout](https://linear.app/docs/board-layout.md)、[Board ordering（2022-08-18）](https://linear.app/changelog/2022-08-18-board-ordering)、[Multi-team boards and manual ordering（2021-08-27）](https://linear.app/changelog/2021-08-27-multi-team-boards-and-manual-ordering)
- Jira：[Use your Kanban backlog](https://support.atlassian.com/jira-software-cloud/docs/use-your-kanban-backlog/)
- Obsidian Kanban：[仓库](https://github.com/mgmeyers/obsidian-kanban)、[解析源码 list.ts](https://github.com/mgmeyers/obsidian-kanban/blob/main/src/parsers/formats/list.ts)、[docs/How do I](https://github.com/mgmeyers/obsidian-kanban/tree/main/docs/How%20do%20I)、[docs/Settings](https://github.com/mgmeyers/obsidian-kanban/tree/main/docs/Settings)
- Backlog.md：[README](https://github.com/MrLesk/Backlog.md)、[CLI-INSTRUCTIONS](https://github.com/MrLesk/Backlog.md/blob/main/CLI-INSTRUCTIONS.md)
- Kanboard：[Tasks](https://docs.kanboard.org/v1/user/tasks/)、[Automatic actions](https://docs.kanboard.org/v1/user/automatic_actions/)、[Analytics](https://docs.kanboard.org/v1/user/analytics/)、[Requirements](https://docs.kanboard.org/v1/admin/requirements/)
- Planka：[README](https://github.com/plankanban/planka)、[docker-compose.yml](https://github.com/plankanban/planka/blob/master/docker-compose.yml)
- Vibe Kanban：[Issue Management](https://vibekanban.com/docs/issue-management.md)
- Drover（本机，只读）：`../drover/README.zh-CN.md`、`../drover/docs/手册.md`；Saddle `ff3c441^:plugins/drover/README.md`、`src/core.rs`、`src/queue.rs`
- paddock：`app/src/kanban.rs`（`column`、`board`、`wrapped_id`）、`app/src/kanban_view.rs`（`actions`）、DESIGN §13 P5-29、`docs/调研/P5-29r-Kanban集成.md`
