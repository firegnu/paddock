# 任务：调研 agent 管理工具里的卡片式看板（Kanban）一般怎么做，给 paddock 的方案选项

2026-10-07，paddock/main 交给 paddock/research-kanban（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：看得见（路由：档拿不准（轻 0.79），要对比多个产品并结合 paddock 现状给选项，按规则用常规；交叉审查拿不准，只读调研，定不要；影响面看得见）
类型：调研
依据：本轮只查业界做法、整理成对 paddock 的几种可选方案和取舍；不定方案（用户还没想好），不出样稿，不写代码。
提示：区分已查证事实、推测和未知，给出直接依据；结论限定在本轮调查范围。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 背景
- paddock 是 GPUI 写的 macOS 桌面应用，用来查看和操作多个 AI 编程 agent（Claude Code、Codex、pi、omp），agent 由命令行工具 `corral` 管理（`corral ls/start/stop/send/reply`，每个 agent 有名字、目录、状态、标签）；派活时常常一个任务一个 git worktree、一个分支。
- 右侧栏定为 Changes（已做，看焦点窗格目录的 git 改动）、Browser（在做）、Kanban 三个标签。用户 10-06：“还有右侧的sidebar再加一个kanban的功能……需求随后聊”；10-07：“我还没想好。现在的卡片式的kanban任务系统集成在一个咱们这种系统中一般是怎么做的？派一个agent去调查。”
- 主控和用户在这个项目里的做法：每件活先在仓库 `docs/任务/` 写一份 Markdown 任务文件（做什么、范围、怎么算做完），主控用 corral 开 agent 在独立 worktree 里做，做完审查、合并、收尾。这套流程目前不在 paddock 界面里。

## 先读
- `AGENTS.md`「规矩」一节（尤其“和 Saddle 生态只通过 corral、ranch 公开命令”“不读 corral 内部状态”）。
- `docs/DESIGN.md` §13 里「右侧栏加 Kanban 标签」「P5-13 样稿定下的」两条，§3 和 Saddle 的关系。
- 浏览 `docs/任务/` 里两三份任务文件的格式（例如 `P5-25-卡片操作图标与菜单图标.md`），了解现在的任务长什么样。
- `app/src/corral.rs`（agent 有哪些字段）、`app/src/card.rs`（左侧栏卡片现在显示什么）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-29r-kanban`，分支 `p5-29r-kanban`（已从 main 建好）。
- 结论写进 `docs/调研/P5-29r-Kanban集成.md`（新建），并在本文件末尾追加完成记录。不改代码。

## 要查的
1. **业界产品怎么做**：至少看这些（找得到公开资料的），每个写清：卡片代表什么（任务、agent 会话、issue、PR）；列怎么分（按状态、按流程、自定义）；卡片谁来移动（人拖、按 agent 状态自动、两者都有）；卡片和 agent、worktree、分支怎么关联（一卡一 agent？开卡就建 worktree？）；数据存在哪（本地数据库、仓库里的文件、GitHub Issues／Projects、Linear 等外部服务）；审查、合并、收尾怎么接到看板上；有没有多项目。
   - Vibe Kanban（BloopAI）、Conductor、Claude Squad、Crystal、Sculptor（Imbue）、Paseo、agentsroom 一类的多 agent 工具；
   - GitHub Projects＋Copilot coding agent、Linear 接 Codex／Cursor／Claude 一类“把 issue 指派给 agent”的做法；
   - Codex 云端、Claude Code 网页版、Devin 一类有任务列表的产品（只看任务列表／看板这部分）；
   - 你查到的其他相关做法。
2. **共同模式和分歧**：归纳出几种典型模式（比如“看板即任务队列，拖到 In Progress 就开 agent”“看板只是 agent 状态的另一种视图”“看板同步外部 issue 系统”），各自适合什么人、什么规模，常见的坑（状态不同步、卡片和 agent 对不上、数据锁在某个工具里）。
3. **对 paddock 的选项**：结合 paddock 现状（corral 只提供公开命令；仓库里已有 `docs/任务/` 任务文件；有 worktree／分支的派活习惯；右侧栏宽度有限），列出 3～4 个可选方案，每个写：卡片是什么、列和移动方式、数据存在哪、和 corral／任务文件／Changes 标签怎么配合、要不要改 corral 或 ranch（改的话属于公开约定变化，要用户另提需求）、工作量大概多大、主要风险。不替用户选，但可以写主控视角的倾向和理由。
4. **要问用户的问题**：整理成最多五个，每个给选项，帮用户“想好”。

## 怎么算做完
- 上面四条都有内容；产品做法附来源链接，分清“已查证（来源）”“推测”“不知道”（比如只看到营销页、没看到文档的，标明）。
- `docs/调研/P5-29r-Kanban集成.md` 写完并提交在分支上。
- 验证只做这些：读公开网页和文档、开源仓库的 README 和源码。不注册、不登录任何服务，不安装这些产品。觉得不够，在回复里说，不要自己加。

## 不要做
- 不写代码、不改 main 上的文件、不出样稿。
- 不复制外部产品的代码或界面素材进仓库；引用只写链接和自己的话总结。
- 不注册、不登录、不安装任何服务或工具；不用 Python。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。不读 corral 的内部状态目录。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。
- 遇到需要用户决定的，写进报告，不要自己拍板。
- 不合并到 main，不推送。只在 `p5-29r-kanban` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：查到了什么、给出的选项、验证了什么、没做的事，各几句话。回复里只写这几样和要问用户的问题。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
