# 任务：调研 Kanban 的“基本看板功能”有哪些，给 paddock 一份最小功能集和不做的线（不变成 Drover）

2026-10-07，paddock/main 交给 paddock/research-kanban-basics（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：看得见（路由：档拿不准（重 0.83），按规则用常规；交叉审查不要；影响面看得见）
类型：设计
依据：本轮查清“看板的基本功能”通常指什么、Drover 重在哪里，结合 paddock 现状推荐一个最小功能集并划出不做的线；不写代码、不出样稿、不改 main。
提示：围绕已知约束给出推荐及关键取舍；有实质备选时再比较，不凑方案数量。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 背景
- paddock 右侧栏的 Kanban 已做完只读版（P5-29a）：一张卡片是一份任务文件（`docs/任务/*.md`），五列 QUEUED／IN PROGRESS／TO REVIEW／MERGED／DONE 全由任务文件、git、corral 推出来，不能拖、不能新建、不能改。已定的加强 A（“Needs you”标记）、C（未提交的任务文件显示成 DRAFT 卡）、D（两处小修）也仍是只读。
- 用户 10-07 原话：“kanban加强你打算怎么做？加什么功能？我不想变成drover那么重，但是基本的看板的功能起码得有。调研一下吧。”
- 这说明用户觉得纯只读可能不够“看板”，但又明确不要 Drover 那样重。Drover 是 Saddle 里已经砍掉的任务队列插件，用户 10-05 砍它时说：“遥测和drover也只是我臆想出来的功能……绑定很深的遥测，sdk还有插件以及drover什么的，只是我觉得有用罢了。”
- 主控的工作方式（派活、审查、合并、收尾）在终端里做，靠任务文件和 git；看板上的任何“能动手”都要和这套流程不打架。

## 先读
- `AGENTS.md`「规矩」和「开发方式」（尤其**看板约定**）。
- `docs/DESIGN.md` §13 的「P5-29 右侧栏 Kanban」整条（含“加强”A～F 和用户原话）。
- 上一轮调研 `docs/调研/P5-29r-Kanban集成.md`：§2 各产品做法已查过，不要重查一遍，只补本轮要的“基本功能”角度；§3.3 常见的坑、§5.3 方案三（可拖、能派活）的风险。
- 现有实现：`app/src/kanban.rs` 开头注释和 `column()`、`board()`；`app/src/kanban_view.rs` 粗看界面有什么。
- Drover：`/Users/firegnu/Developer/personal_projs/drover`（独立仓库），以及 Saddle 仓库 `/Users/firegnu/Developer/personal_projs/saddle` 在提交 `ff3c441` 之前的 Drover 代码和文档（`git -C <saddle> show ff3c441^:<路径>`、`git log`）。**两处都只读**。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-29r2-kanban`，分支 `p5-29r2-kanban`（已从 main 建好）。
- 结论写进 `docs/调研/P5-29r2-Kanban基本功能.md`（新建），并在本文件末尾追加完成记录。不改代码。

## 要做的
1. **“基本看板功能”通常指什么**：看通用看板（Trello、GitHub Projects 看板视图、Linear 看板、Jira 的基础看板）、本地或文件型看板（Obsidian Kanban 插件、Backlog.md、Kanboard／Planka 之类自托管的、以 Markdown 文件为数据的看板工具），以及上一轮看过的 agent 看板（Vibe Kanban 等，只补“基本操作”这一面）。把常见功能列成一张表，分三档：几乎都有的（底线）、常见但可选的、重的（自动化规则、队列调度、统计报表等）。每项写它在这些工具里大致怎么做。
2. **Drover 重在哪里**：读源码和文档，列出 Drover 的功能和数据（自己的队列文件、状态文件、单进程持有数据、派发、遥测等），指出哪些是用户说的“重”。只写事实和你的判断，标清哪些是推断。
3. **对 paddock 的推荐**：在“卡片＝任务文件、状态尽量从 git 和 corral 推出”的前提下，给一个最小功能集。对第 1 条底线里的每一项写：
   - 在 paddock 里具体是什么样（例如“新建卡片＝在主仓库工作区生成一份任务文件草稿”“移动卡片＝？”）；
   - 数据写在哪（任务文件里加一行、paddock 自己的本地文件、不写）；会不会和推出来的状态打架、和主控在终端里的流程打架，怎么避免；
   - 做还是不做、理由，大致工作量。
   特别要回答：列是推出来的，“拖动卡片”在 paddock 里该是什么意思（不能拖／只能在 QUEUED 里排先后／拖来纠正推错的状态／拖了就派活），各自的代价。
4. **不做的线**：明确写出哪些功能一做就开始像 Drover，建议不做，各一句理由。
5. **要问用户的问题**：最多五个，每个给选项和你的建议。

## 怎么算做完
- 上面五条都有内容；外部产品的说法附来源链接，分清“已查证（来源）”“推测”“不知道”。
- `docs/调研/P5-29r2-Kanban基本功能.md` 写完并提交在分支上。
- 验证只做这些：读公开网页和文档、开源仓库的 README 和源码、本机的 Drover 和 Saddle 源码（只读）。不注册、不登录任何服务，不安装这些产品。觉得不够，在回复里说，不要自己加。

## 不要做
- 不写代码、不改 main 上的文件、不出样稿。
- 不修改 Drover、Saddle 仓库里的任何东西（包括 worktree、分支、文件），不在里面编译或运行。
- 不复制外部产品的代码或界面素材进仓库；引用只写链接和自己的话总结。
- 不注册、不登录、不安装任何服务或工具；不用 Python。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。不读 corral 的内部状态目录。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 遇到需要用户决定的，写进报告，不要自己拍板。
- 不合并到 main，不推送。只在 `p5-29r2-kanban` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：查到了什么、推荐的最小功能集、验证了什么、没做的事，各几句话。回复里只写这几样和要问用户的问题。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- **查到了什么**：看了 Trello、GitHub Projects、Linear、Jira、Obsidian Kanban、Backlog.md、Kanboard、Planka、Vibe Kanban。底线是五样：按列看、新建、移到别的列、列内排先后、归档（外加打开和编辑）。标签、筛选、WIP 上限、泳道等常见但可选；自动化规则、派发调度、统计、通知是重的。Kanboard、Planka 其实存数据库，不是 Markdown 文件型。Drover 的“重”主要在第二份任务数据（`queue.md`、`tasks.state`、`.drover.conf`、项目登记）和在界面里重做主控流程（派发、提交、验收、退回，加令牌和锁），另外还有常驻引擎、独占进程、通知、遥测和控制接口。插件版约 1.1 万行 Rust。
- **推荐的最小功能集**：新建卡片（写未提交的任务文件草稿，显示成 DRAFT，排在 C 之后）；“不做了”用 `收尾: <编号> 不做：<原因>` 空提交，标 Dropped；主控补 9 条收尾空提交，修好约定之前的老任务（按代码推断它们停在 TO REVIEW 或 MERGED）；不能拖、不做列内排序、打开编辑照旧。还发现一个坑：在主仓库工作区改已提交的任务文件会挡住合并，所以看板上的写回只能新建文件。
- **验证了什么**：只读公开文档、开源仓库的 README 和源码（Obsidian 解析源码、Backlog.md CLI 文档），本机 Drover 仓库和 Saddle `ff3c441^` 的 Drover 源码（只读，没编译没运行），paddock 的 `kanban.rs`、`kanban_view.rs`，以及 main 上的提交说明（`git log`）。没注册、登录、安装任何东西，没碰 corral 的 agent。
- **没做的**：没写代码、没出样稿、没改 main。9 张推错的卡没在真窗口里看；“未提交改动挡住合并”没动手试；Jira WIP 上限只见搜索摘要，Obsidian 文档站取不到正文，改读仓库 `docs/`。
