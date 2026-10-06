# P5-29r：agent 管理工具里的卡片式看板（Kanban）一般怎么做

2026-10-07，paddock/research-kanban。只读调研：读公开网页、文档、开源仓库的 README 和源码；没有注册、登录或安装任何产品。任务文件：`docs/任务/P5-29r-Kanban集成调研.md`。

标注：
- **已查证**：看过文档或源码，后面附来源。
- **只见营销页**：只看到产品宣传页或更新日志，没看到使用文档或源码。
- **推测**：根据已查证的内容推出来的，没有直接依据。
- **不知道**：查过但没找到答案。

本文不定方案，也不出样稿。第 5 节列出可选方案，第 6 节列出要请用户拿主意的问题。

---

## 0. 一页结论

1. 业界大致有四种做法，各家常常混用：
   - **A 看板就是任务队列**：卡片是任务，拖到 In Progress 就开 agent。例子：AgentsRoom、2025 年版的 Vibe Kanban。
   - **B 看板只是 agent 或会话状态的另一种视图**：不能拖，列由状态推出来。例子：Claude Code 的 agent view 和 Projects Overview、Conductor 按状态分组的侧栏、Windsurf/Devin 的 Agent Kanban。
   - **C 同步外部 issue 系统**：把 issue 指派给 agent，状态靠 PR 事件自动推进。例子：GitHub Projects＋Copilot、Linear＋Codex/Cursor/Claude、OpenAI Symphony。
   - **D 仓库里的 Markdown 任务文件就是数据**：例子：Backlog.md、Nimbalyst。
2. **卡片谁来移动**：
   - 人拖的只管“开始”（AgentsRoom 拖到 In Progress 就开 agent）和“结束”（审完拖到 Done）。
   - 中间的状态几乎都由系统推出来：agent 开始干活、这一轮结束、PR 开出、PR 合并。
   - 推不准的地方都一样：agent “这一轮结束”不等于“做完”。Vibe Kanban 旧版里 agent 跑完或出错都会进 In Review；corral 自己也写明“只知道这一轮结束，不知道做完”。
3. **数据锁在工具里，确实出过事**：Vibe Kanban 的公司 2026-04-10 宣布关闭，云端的 kanban issue、项目、组织 30 天后下线，只剩本地 workspace 还能用。看板数据放在仓库文件或通用系统里的工具（Backlog.md、GitHub、Linear）没有这个问题。
4. **对 paddock 最要紧的现状**：
   - 任务文件、worktree、分支、corral agent 已经是一套事实上的流水线。看板可以只是这条流水线的视图，不必另建数据。
   - 用到的线索：任务文件第 3 行“交给 <agent>”、“在哪里干活”里的 worktree 和分支、`## 完成记录`、`## 主控审查`、main 上的“收尾:”空提交、corral 的 `cwd`。
   - 难点是“卡片和 agent 对上”。本仓库就有两例对不上（见 §4.3），这和业界常见的坑一样。
5. 第 5 节给出四个方案：
   - 方案一：从任务文件、git、corral 推出来的只读流水线看板。
   - 方案二：按 agent 状态分列。
   - 方案三：仓库内可拖的任务板，拖动可以派活。
   - 方案四：接 GitHub Issues/Projects。

   主控视角的倾向是先做方案一，理由和风险见 §5.5。这个倾向不是决定。

---

## 1. 范围与方法

- 看了 14 个产品或做法（§2），包括任务文件点名的全部，加上 Backlog.md、Nimbalyst、OpenAI Symphony、Claude Code agent view/Projects、Windsurf（Devin Desktop）、Codex cloud。
- 开源的直接读了源码或仓库文档：
  - Vibe Kanban：当前 main 和 `v0.0.138-20251217105424` 两个版本的 `crates/services` 和 `crates/db`。
  - 另外读了 Backlog.md、Sculptor、Claude Squad、Crystal、Symphony 的 README 或 SPEC。
- 闭源的只读公开文档和更新日志。
- 只读了 paddock 自己的 `app/src/corral.rs`、`card.rs`、`agents.rs`（分组和排序）、`docs/任务/` 下 66 份任务文件的结构，以及 `corral guide`。`corral ls` 只看了标签用了哪些键，没有动任何 agent。

---

## 2. 产品做法

先给总表，后面逐个说明依据。“—”表示没有这一项或不适用。

| 产品 | 卡片代表 | 列 | 谁来移动 | 卡片 ↔ agent／worktree／分支 | 数据在哪 | 审查／合并／收尾 | 多项目 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Vibe Kanban（旧版） | 任务（task） | To do／In progress／In review／Done／Cancelled，固定 | 两者都有：开 attempt 自动进 In progress，跑完进 In review，PR 合并进 Done；人也可拖 | 一个任务可有多次 attempt，每次一个 worktree＋分支 | 本地 SQLite | 内置 diff 审查，开 PR，监控合并 | 有 |
| Vibe Kanban（现版） | issue（云端）；本地以 workspace 为主 | 同上，可自定义 | 文档只写了手动拖 | 一个 issue 可连多个 workspace | issue 在云端（已随关停下线）；workspace 在本地 | 同上；PR 合并后自动归档 workspace | 有 |
| AgentsRoom | 任务 | TODO／In Progress／Pending／Done | 人拖；拖到 In Progress 即开 agent | 卡片连一个 agent；可设单独 worktree，卡片底部显示分支 | 说法矛盾（见下） | diff 审完人拖到 Done | 有 |
| Nimbalyst（原 Crystal） | tracker 条目（任务、bug、想法） | 按状态 | 两者都有：人拖，agent 完成时“可以”自动改 | 条目连到实现它的会话和文件 | 营销页称本地优先、开放格式 | 不知道 | 有 |
| Backlog.md | 仓库里一个 `.md` 任务文件 | 配置文件里的 `statuses`，默认 To Do／In Progress／Done | 人或 agent 用命令改；网页可拖 | 推荐一个任务 = 一次会话 = 一个 PR；不管 agent 进程 | 仓库里的 Markdown | 三个人工检查点；`task complete` 移出看板 | 单仓库，可加 `projects` 字段 |
| Conductor | workspace（= worktree＋分支＋会话） | 侧栏按 backlog／in progress／in review／done 分组 | 不知道 | 一个 workspace 一个 worktree 和分支 | 本地应用（另有云端版） | Diff Viewer→Create PR→合并→归档；有合并后自动归档设置 | 有 |
| Claude Squad | 会话（instance） | 无看板，只有列表 | — | 一个会话一个 tmux＋worktree＋分支 | 本地状态文件 | `s` 提交推送，`c` checkout 并暂停 | 不知道 |
| Sculptor | workspace（标签页） | 无看板 | — | 一个 workspace 一个 worktree＋分支，里面可以有多个 agent | 本地 | Create PR，按钮显示 PR 状态 | 有 |
| Paseo | agent 和 workspace | 无看板 | — | 支持 worktree | 本地 daemon | 不知道 | 有 |
| GitHub Projects＋Copilot | issue／PR 条目 | 自定义 Status 字段 | 两者都有：内置工作流在关闭或合并时设 Done；人可拖 | 把 issue 指派给 Copilot，它开 draft PR 和分支 | GitHub | 在 PR 里审，`@copilot` 迭代 | 有 |
| Linear＋agent | issue | 团队工作流状态 | 两者都有：PR 开出、合并按规则推进；agent 应把 issue 移到 started | 指派给 agent 是“delegate”，人仍是负责人；一个 issue 一个会话 | Linear | Reviews 标签页，可在 Linear 里合并 | 有 |
| OpenAI Symphony | tracker 里的 issue | tracker 的状态，配置 active 和 terminal 两类 | tracker 状态驱动调度；状态由 agent 写回 | 一个 issue 一个固定目录（workspace） | 外部 tracker＋仓库里的 `WORKFLOW.md` | 可停在 `Human Review` 这类交接状态 | — |
| Claude Code（agent view／Projects） | 会话／thread | 按状态分组：Ready for review／Needs input（Waiting on you）／Working／Landing／Idle／Completed（Resolved） | 系统推出来；Resolved 可手动标 | thread 各有分支和 PR | 本地（agent view），云端（Projects） | PR 状态标签，Merge it 等按钮 | 有 |
| Windsurf／Devin Desktop | agent 会话 | 按状态 | 只见更新日志 | 有 worktree 会话和 Merge 按钮 | 不知道 | 不知道 | 不知道 |
| Codex cloud | 任务（chat） | 列表，无看板 | — | 每个任务一个云环境；可多次尝试（best-of-N） | 云端 | 审改动后开 PR | 按环境 |

### 2.1 Vibe Kanban（BloopAI）

**两个版本**

- 2025 年的版本（下称“旧版”）是“看板＋本地执行”。
- 2026 年改成“云端 kanban issue＋本地 workspace”。公司 2026-04-10 宣布关闭，项目转为社区维护（Apache-2.0）。

**卡片和列**

- 已查证：旧版的卡片是 `Task`，有 `project_id`、`title`、`description`、`status`。状态枚举是 `Todo`、`InProgress`、`InReview`、`Done`、`Cancelled`，存在本地 SQLite（`sqlx::SqlitePool`）。（[task.rs](https://github.com/BloopAI/vibe-kanban/blob/main/crates/db/src/models/task.rs)）
- 已查证：现版文档里，issue 是基本工作单位。看板默认显示 To do、In progress、In review、Done 四列，Backlog 和 Cancelled 默认隐藏。（[Issue Management](https://vibekanban.com/docs/issue-management.md)）

**谁来移动（旧版，已查证源码 `v0.0.138`，[container.rs](https://github.com/BloopAI/vibe-kanban/blob/v0.0.138-20251217105424/crates/services/src/services/container.rs)、[pr_monitor.rs](https://github.com/BloopAI/vibe-kanban/blob/v0.0.138-20251217105424/crates/services/src/services/pr_monitor.rs)）**

- 开始一次 attempt（dev server 除外）时，任务自动设为 `InProgress`。
- 执行结束、没有下一步时设为 `InReview`。孤儿进程被判失败、启动失败时也设为 `InReview`。
- PR 合并后由 PR 监控设为 `Done`。
- 人也可以拖。

**谁来移动（现版）**

- 已查证：文档只写了手动拖。子 issue 全部完成，父 issue 也不会自动完成。
- 已查证：现版源码里 PR 合并后改为“所有 PR 都合并或关闭时归档 workspace”。（[pr_monitor.rs（main）](https://github.com/BloopAI/vibe-kanban/blob/main/crates/services/src/services/pr_monitor.rs)）
- 推测：现版不再在本地自动改 issue 状态。

**和 agent、worktree、分支的关联**

- 已查证：一个 workspace 建一个 git worktree 和一个新分支；一个 workspace 里可以开多个会话。（[Workspaces](https://vibekanban.com/docs/workspaces/index.md)）
- 已查证：一个 issue 可以连多个 workspace，用来让 agent 并行做同一功能的不同部分。（[Issue Management](https://vibekanban.com/docs/issue-management.md)）

**审查、合并、收尾**

- 已查证：内置 diff 审查，可以加行内评论送回 agent；能开 PR，PR 描述由 AI 生成。（[README](https://github.com/BloopAI/vibe-kanban)）

**数据和关停**

- 已查证：云端的 kanban issue、评论、项目、组织在公告 30 天后关闭；本地 workspace 继续可用，产品转向完全本地。（[关停公告](https://vibekanban.com/blog/shutdown)）
- 这是“数据锁在工具里”的直接例子。

### 2.2 AgentsRoom

只见营销页（[Backlog Task Board](https://agentsroom.dev/features/backlog-task-board)）。

**列和移动**

- 四列：TODO 放想法，In Progress 放正在跑的 agent，Pending 放卡住的任务，Done 放完成的。
- 拖到 In Progress 会立刻开一个临时 Claude agent，用卡片标题当提示词；可以一次拖多张，每张开一个 agent。
- 卡片实时显示所连 agent 的状态（干活中、等输入、完成、空闲），点卡片跳到 agent 的终端。
- 审查是人看 diff 后拖到 Done；要返工就留在 In Progress，再给 agent 下指令。

**worktree 和导入**

- 卡片可以设成在单独的 worktree 里跑，底部显示分支名，悬停显示完整分支以及是否自动合回。
- 可以从 GitHub Issues、Jira、Notion 导入；导入后的卡片已在进行中或已完成的，不会退回 TODO。

**数据在哪：不知道**

同一页面的小标题写“Stored in your repo”，正文却写“Tasks are synced to the cloud”。两种说法矛盾，没有文档可以核实。

### 2.3 Crystal → Nimbalyst

- 已查证：Crystal（stravu）2026 年 2 月弃用，由 Nimbalyst 取代。Crystal 本身是“每个会话一个 worktree”的多会话管理器，没有看板。（[Crystal README](https://github.com/stravu/crystal)）
- 只见营销页：Nimbalyst 有内置 tracker，记录任务、bug、功能和想法，按状态显示成看板，可以拖、可以按项目或负责人分组。Claude Code 或 Codex 完成任务时“可以”自动更新状态；条目连到实现它的会话和文件。（[Task Management](https://nimbalyst.com/features/task-management/)）
- 只见营销页：README 写“Local-first file model using open formats like Markdown, JSON, and CSV”。条目具体是不是存在 Markdown frontmatter 里：不知道。

### 2.4 Backlog.md（MrLesk，MIT）

已查证，来源是 [README](https://github.com/MrLesk/Backlog.md) 和 [ADVANCED-CONFIG](https://github.com/MrLesk/Backlog.md/blob/main/ADVANCED-CONFIG.md)。

**数据和列**

- 每个任务是仓库里 `backlog/` 目录下的一个 `.md` 文件。
- 列来自配置里的 `statuses`，默认 `[To Do, In Progress, Done]`。
- 有终端看板（`backlog board`）和本机网页看板（`backlog browser`，可以拖），读命令能输出 `--json`。

**和 agent 的关系**

- 工具不管 agent 进程，只靠约定：一个任务 = 一个上下文窗口 = 一个 PR。
- 三个人工检查点：审规格、审计划、审代码。
- `onStatusChange` 可以在状态变化时跑一条 shell 命令，文档举的例子正是“变成 In Progress 就启动 claude”。

**跨分支状态（和 paddock 直接相关）**

- `checkActiveBranches` 默认打开：在最近 `activeBranchDays`（默认 30）天里活跃的分支上核对任务状态，因为任务文件可能只在某个分支上被改过。
- 文档说明这样做在大仓库上会变慢。

**收尾**

`task complete` 把任务移出看板但保留记录；`task archive` 用于作废的任务。

### 2.5 Conductor（Mac 应用）

**做法**

- 已查证：工作单位是 workspace，由一个 worktree、分支、终端、diff、检查和 PR 组成。（[Parallel agents](https://www.conductor.build/docs/core/parallel-agents)）
- 已查证：流程是从 issue 建 workspace → 做 → 用 Diff Viewer 审 → Create PR → 合并 → 合并后归档 workspace，让它离开当前工作列表。（[Issue to PR](https://www.conductor.build/docs/guides/issue-to-pr)）
- 已查证：可以从 GitHub 或 Linear issue 建 workspace（0.66.0）。（[changelog](https://www.conductor.build/changelog/0.66.0-create-workspace-from-issue)）

**状态和看板**

- 只见更新日志：0.35.0 起侧栏按 backlog、in progress、in review、done 分组，有 PR 时用 PR 标题作名字（[0.35.0](https://www.conductor.build/changelog/0.35.0-workspace-status)）；之后侧栏显示 GitHub 状态（merged、CI 失败、冲突等）。
- 只见更新日志：设置里有“合并后归档”（archive-on-merge）。（[changelog 汇总](https://releases.sh/conductor/conductor-changelog.md)）
- 不知道：这四个状态是自动推出来的，还是人手动设的。
- 没有单独的看板，状态体现为分组列表。

### 2.6 Claude Squad（smtg-ai，AGPL-3.0）

已查证（[README](https://github.com/smtg-ai/claude-squad)）：

- 终端应用。一个会话对应一个 tmux 会话、一个 worktree 和一个分支。
- 只有列表，没有看板。
- 操作：`s` 提交并推送分支；`c` checkout，也就是提交并暂停会话；`r` 恢复会话。

### 2.7 Sculptor（Imbue）

已查证（[README](https://github.com/imbue-ai/sculptor)，以及仓库里的 [workspaces](https://github.com/imbue-ai/sculptor/blob/main/docs/help/workspaces.md)、[agents](https://github.com/imbue-ai/sculptor/blob/main/docs/help/agents.md)、[pull_requests](https://github.com/imbue-ai/sculptor/blob/main/docs/help/pull_requests.md) 文档）：

- 一个 workspace 是一个标签页，默认是 worktree，分支名默认 `<user>/<slug>`，有目标分支。
- 删除 workspace 时，可以按设置决定是否删掉分支。
- 一个 workspace 里可以有多个 agent 标签，它们共用同一份工作副本。
- Create PR 用 `gh`，按钮上自动刷新 PR 的开、合、关，审批和 CI 状态。
- 没有看板。
- 早期宣传说“每个 agent 一个 Docker 容器”，现在的文档把容器列为实验功能。

### 2.8 Paseo

只见文档索引（[llms.txt](https://paseo.sh/llms.txt)）：用 workspace 把目录、agent、终端、浏览器组织在一起，支持 worktree、定时任务和编排。没有找到看板或任务列表。

### 2.9 GitHub Projects＋Copilot coding agent

已查证：

- 可以在仓库的 issue 列表里把 issue 指派给 Copilot，也可以在 GitHub Projects 里打开 issue 时指派。Copilot 开一个标着 [WIP] 的 draft PR 来跟踪工作，做完请人审。之后在 PR 评论里 `@copilot` 让它继续改。所有任务汇总在 github.com/copilot/agents。（[Use cloud agent on GitHub](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/use-cloud-agent-on-github)、[GitHub Blog](https://github.blog/ai-and-ml/github-copilot/github-copilot-coding-agent-101-getting-started-with-agentic-workflows-on-github/)）
- Projects 默认打开两条内置工作流：issue 或 PR 关闭时 Status 设为 Done，PR 合并时设为 Done。其他工作流可以改，例如加入项目时设为 Todo。（[built-in automations](https://docs.github.com/en/issues/planning-and-tracking-with-projects/automating-your-project/using-the-built-in-automations)）
- 卡片在 agent 开始干活时会不会自动变成 In Progress：不知道，没有查到相应的内置工作流。

### 2.10 Linear＋Codex／Cursor／Claude

已查证：

- 把 issue 指派给 agent 叫“delegate”，人仍然是主要负责人。（[Agents in Linear](https://linear.app/docs/agents-in-linear)）
- Linear 对 agent 开发者的要求：被人 delegate 的 issue 如果不在 started、completed、canceled 这几类状态里，agent 开始干活时要把它移到第一个 started 状态；由自动化 delegate 的 issue 留在 triage，交给人处理。（[Agent best practices](https://linear.app/developers/agent-best-practices)）
- agent 会话有 6 个状态：pending、active、error、awaitingInput、complete、stale。Linear 根据 agent 最后发出的活动自动维护会话状态。（[Agent interaction](https://linear.app/developers/agent-interaction)）
- Coding sessions 在 Linear 管理的云沙箱里跑 Claude Code 或 Codex，起草 PR 并把 diff 挂到 issue 上；审查在 Reviews 标签页，可以直接在 Linear 里合并。（[Coding sessions](https://linear.app/docs/coding-sessions)）
- GitHub 集成：PR 开出时 issue 进 In Progress，合并时进 Done，可以自定义成 In Review 等状态；一个 issue 有多个 PR 时，等全部合并才推进。这一条只看到搜索摘要和 [changelog](https://linear.app/changelog/2023-11-15-github-workflow-updates)，GitHub 集成文档页没有取到正文。

### 2.11 OpenAI Symphony（Apache-2.0，工程预览）

已查证（[README](https://github.com/openai/symphony)、[SPEC.md](https://github.com/openai/symphony/blob/main/SPEC.md)）：

- 一个常驻服务：定时轮询 issue tracker，每个 issue 建一个固定的独立目录，在里面跑 agent。
- 配置分 `active_states` 和 `terminal_states`。issue 变成不符合条件的状态时，停掉正在跑的 agent。
- 状态转换、评论、PR 链接通常由 agent 自己写回 tracker，调度器只读。
- 成功的一轮可以停在 `Human Review` 这样的交接状态，不一定是 Done。
- 流程规则放在仓库里的 `WORKFLOW.md`。不需要持久数据库，重启后从 tracker 和文件系统恢复。

这是“看板就是任务队列＋外部 tracker 做唯一来源”最纯粹的形态。

### 2.12 Claude Code（agent view、Projects）

已查证：

- 本地的 `claude agents`（agent view）按状态分组：Pinned、Ready for review（有 PR 等审或检查失败）、Needs input、Working、Completed。每行右边是 PR 号，颜色表示 PR 状态（等检查、可合、已合并、草稿或关闭）。按 Ctrl+S 可以改成按目录分组。（[Agent view](https://code.claude.com/docs/en/agent-view)）
- 云端 Projects（公开测试版）：一个总对话负责协调，每件活开一个 thread，thread 有自己的分支和 PR。Overview 的 Threads 标签按 Ready for review、Waiting on you、Working、Landing、Idle、Resolved 分组。Resolved 有三种来源：人从菜单标记；人做完最后一步（如合并 PR）后由 Claude 标记；一周没动静自动标记。thread 卡片上有 Fix CI、Merge it、Create PR 等按钮。（[Projects](https://code.claude.com/docs/en/claude-projects)）

这和 paddock 的“主控派活 → agent 在 worktree 里做 → 审查合并”结构最像，而且它的分组完全是推出来的，不能拖。

### 2.13 Windsurf／Devin Desktop

只见更新日志（[changelog-next](https://docs.devin.ai/desktop/changelog-next)）：

- v2.0.1044（2026-04-15）：新增“Kanban-style view showing all local and cloud agent sessions, organized by status”。
- 另有“Open Agent Kanban View／Open Agent List View”命令，worktree 会话带 Merge 按钮。
- 列有哪些、能不能拖：不知道。

Devin 网页版的会话列表有持久的状态标签，例如“PR created”“Awaiting instructions”，可以按来源（Slack、Jira、Linear 等）筛选。这一条只见搜索摘要，没有核实。

### 2.14 Codex cloud

已查证（[CLI 参考](https://learn.chatgpt.com/docs/developer-commands?surface=cli)）：

- `codex cloud list --json` 每个任务给出 `id`、`url`、`title`、`status`、`updated_at`、`environment_id`、`summary`、`is_review`、`attempt_total` 等字段。
- `--attempts 1-4` 表示 best-of-N，一个任务可以跑几次。
- 审完改动后可以提交或开 PR。

列表、没有看板（推测，文档里没有看到看板）。

---

## 3. 共同模式和分歧

### 3.1 四种典型模式

| 模式 | 典型 | 卡片 | 谁来移动 | 适合谁、什么规模 | 主要的坑 |
| --- | --- | --- | --- | --- | --- |
| A 看板就是任务队列 | AgentsRoom、Vibe Kanban 旧版；Backlog.md 的 `onStatusChange` 例子 | 任务 | 人拖“开始”和“完成”，中间自动推 | 个人或小团队，任务多、想“拖一下就开跑” | 拖错就开出 agent；卡片状态和 agent 实际状态两份数据会漂；一个任务重试多次后卡片和 agent 不再一一对应 |
| B 看板只是 agent 状态的视图 | Claude Code agent view／Projects、Conductor 分组侧栏、Windsurf Agent Kanban | 会话或 workspace | 系统推出来，至多手动标 Resolved | 同时开很多 agent、关心“谁在等我” | 和 agent 列表重复；agent 关掉，卡片就没了，看不到“还没开始”和“已经做完”的活 |
| C 同步外部 issue 系统 | GitHub＋Copilot、Linear＋agent、Symphony、Conductor 从 issue 建 workspace | issue | 主要靠 PR 事件自动推，人也可拖 | 团队、已经在用 GitHub 或 Linear | 要联网、要账号；公开仓库的 issue 是公开的；数据在别家；agent 跑在本机时要自己回写状态 |
| D 仓库文件就是数据 | Backlog.md、Nimbalyst | 文件 | 命令、agent 或人改文件 | 个人或小团队，重视可移植和留档 | 状态写在分支上，main 看不到（Backlog.md 用跨分支扫描解决，代价是慢）；格式要稳定 |

### 3.2 分歧点

- **卡片是“任务”还是“会话”**：A、C、D 是任务，一个任务可以有多个会话（Vibe Kanban 一个 issue 多个 workspace，Codex best-of-N）；B 是会话。按任务做才看得到“还没开始”和“已收尾”的活，按会话做只看得到开着的 agent。
- **能不能拖**：B 不能拖；A 靠拖触发动作；C、D 能拖但多数靠自动。
- **“做完”由谁判断**：
  - Vibe Kanban 旧版把“这一轮结束”当成待审，失败也进 In Review。
  - Claude Projects 把“PR 开出”当成待审，把“合并后标记或一周没动静”当成 Resolved。
  - Symphony 让 agent 自己把 issue 写成 Human Review。
  - paddock 的现行约定是 agent 回复最后一行写 DONE，再加上任务文件里的 `## 完成记录`。
- **收尾**：Conductor、Vibe Kanban 现版在合并后归档 workspace；Sculptor 删 workspace 时按策略删分支；Claude Squad 的 checkout 会暂停会话。业界没有统一的“收尾”列，多数做法是“合并 → 归档或移出看板”。

### 3.3 常见的坑和依据

1. **状态不同步**
   - Vibe Kanban 旧版里出错也进 In Review（源码，§2.1）。
   - Conductor 更新日志里有“agent 结束后 Git、PR 状态不刷新”的修复。
   - corral guide 明写“corral 只知道这一轮结束，不知道做完”。
   - 推测：凡是“卡片状态单独存一份”的做法，都要有对账：Symphony 有专门的 reconciliation，按 tracker 当前状态停掉不符合条件的运行。
2. **卡片和 agent 对不上**：一个 issue 对多个 workspace、重试、改名都会打断一一对应。paddock 自己的例子见 §4.3。
3. **数据锁在工具里**：Vibe Kanban 云端 issue 随公司关停下线（§2.1）。
4. **跨分支**：任务文件的状态如果写在分支上，main 上的看板看不到，Backlog.md 要专门扫活跃分支（§2.4）。
5. **一拖就开 agent**：AgentsRoom 一拖就开，没有确认。Linear 明确要求自动化 delegate 的 issue 留在 triage，等人处理（§2.10）。推测：省一步，但容易误开 agent，也让“谁开的 agent”不好追溯。

### 3.4 趋势（推测）

几家都在把“看板”往“按状态分组的会话列表＋PR 状态”靠：

- Vibe Kanban 关停后转向完全本地的 workspace。
- Conductor 用分组侧栏，没有做单独的看板。
- Claude Code、Windsurf 的看板都是会话状态视图。

真正“拖卡片＝派活”的主要是 AgentsRoom 这类小产品。这只是本轮看到的样本，样本很少，不构成结论。

---

## 4. paddock 现状（和看板相关的部分）

### 4.1 corral 能给什么

`app/src/corral.rs` 的 `Agent` 有这些字段：`name`、`kind`、`instance`、`cwd`、`state`、`last_tool`、`turn_started`、`state_started`、`idle_for`、`attached`、`last_input_source`、`title`、`started`、`error`、`labels`。

- `labels` 现在用了 `role`、`effort`、`model`。`corral start` 本来就支持任意 `--label KEY=VALUE`，所以加一个新标签键（如 `task=P5-29r`）不用改 corral。但这需要主控派活时自己加上，或者改 ranch 里的 corral-dispatch 技能，后者属于 ranch 的任务。
- 不知道：已经 `stop` 的 agent 是否还在 `corral ls` 里。按 paddock 的 `Status::Exited` 推测，退出但没有 stop 的还在；stop 之后，看板只能靠任务文件和 git 知道这件活。本轮按规矩没有试。

### 4.2 任务文件能给什么

`docs/任务/` 下共 66 份任务文件，Markdown 没有固定的机器可读格式，但近几十份写法很一致：

- 第 3 行“`<日期>，paddock/main 交给 paddock/<agent>`”，加上“路由”和“类型”两行；
- “在哪里干活”一节写 worktree 路径和分支；
- 完成后 agent 在分支上追加 `## 完成记录`，主控审查后加 `## 主控审查`；
- 合并后 main 上有一条“收尾: …”空提交。

粗略统计：大多数 P4、P5 文件同时有“交给”、分支、完成记录、主控审查；P3 和 P5-5～P5-7 等较早或主控自己做的没有“交给”行。

### 4.3 已经能看到的“对不上”

- 任务文件写“交给 paddock/dev-browser”，实际 corral 名字是 `paddock/dev-browser-1`。`--unique` 会补后缀，HANDOFF 的主控教训里也写了这一点。
- `P5-28a-Browser原生宿主与工具栏.md` 里第一处出现的分支名是原型分支 `p5-13r-browser`，不是这件活自己的分支。照“第一处出现的分支”去解析会认错。
- 用 `cwd` 对 worktree 路径比较稳：任务文件写了 worktree 路径，corral 给出 `cwd`。但同一 worktree 先后开过两个 agent（例如返工）时，还是得靠时间或标签区分。

### 4.4 界面

- 右侧栏有 Changes（跟随焦点窗格）、Browser（在做）、Kanban（待定）三个标签。宽度有限，可以加宽。
- 推测：横排四五列的经典看板在默认宽度下放不下，更可能是“按列分段的竖向列表”，加宽后再横排。
- 左侧栏已经按项目前缀（`paddock/`）分组、可以按状态排序，卡片上有状态、预览、目录和分支。只按 agent 状态分列的看板会和它大量重复。

### 4.5 历史决定（不是限制，供参考）

- 用户 10-05 砍掉了 Saddle 的 Drover（任务队列插件，有自己的 `queue.md`、`tasks.state`），原话：“遥测和drover也只是我臆想出来的功能”。见 `docs/背景与决策记录.md` §6l。
- 这说明“另起一套任务数据和派发系统”之前被认为不值得。这次的 Kanban 是用户新提的需求，不等于反对任务界面，但方案越像 Drover，越值得先确认。

---

## 5. 对 paddock 的选项

四个方案都不需要改 corral 的命令或 JSON 输出。需要动公开约定的地方（新标签键、任务文件格式）单独标出来。

### 5.1 方案一：流水线看板（只读，从现有事实推出来）

- **卡片**：一份任务文件，即 `docs/任务/*.md`，只看焦点窗格所在仓库。
- **列**（全部推出来，不能拖）：
  1. 待派：任务文件在 main 上，没有对应的分支或 agent。
  2. 在做：对应的 corral agent 开着，用 `cwd` 对上任务文件写的 worktree，干活中或等输入。
  3. 待审：分支上的任务文件已有 `## 完成记录`，或 agent 空闲且回复最后一行是 DONE。
  4. 已合并：分支已并进 main，但还没有“收尾:”提交。
  5. 已收尾：main 上有“收尾:”提交；默认折叠，只显示最近几件。
- **卡片显示**：任务名、agent 种类图标和状态点（沿用左侧栏的样子）、分支、`+增 −删`。
- **点卡片**：开或聚焦那个 agent 的窗格，Changes 标签随之显示它的 worktree 改动；点任务名在终端或编辑器里打开任务文件。
- **数据**：不新增。读 main 和各分支上的任务文件（`git show <分支>:docs/任务/…`，只看还没合并的分支）、`git log` 里的收尾提交、`corral ls`、各 worktree 的 git 状态。
- **和现有部分的配合**：
  - corral：只读 `ls`，用 `cwd` 和 `labels` 对上。
  - 任务文件：照现有写法解析，不改格式。若想更稳，可以约定在第 3 行附近加一行机器可读的“分支／worktree”，这是改任务文件格式，要用户或主控定。
  - Changes：通过焦点窗格联动。
- **要不要改 corral／ranch**：不用。要更稳地对上卡片和 agent，可以约定派活时加 `--label task=<编号>`：主控自己加不用改 ranch；写进 corral-dispatch 技能就是 ranch 的任务，要用户另提。
- **工作量（推测）**：中等。主要在解析和对账：任务文件、分支、agent 三方对齐，加上边界情况，比如主控自己做、没有“交给”行的活，同一 worktree 开过两个 agent，分支已删。界面部分是竖向分段列表。
- **主要风险**：
  - 任务文件是自由格式，解析靠惯例，§4.3 那种情况会认错。
  - 扫分支有成本，可以只扫有 worktree 的分支。
  - 只覆盖“用任务文件派的活”，口头交代的小事不会出现。

### 5.2 方案二：agent 状态看板（最轻）

- **卡片**：一个 corral agent。
- **列**（全部推出来）：Waiting on you（等你、卡住、出错）、Working、Idle（待审或已答完）、Exited。可以按项目分组，参照 Claude Code agent view 和 Conductor 侧栏。
- **数据**：只有 `corral ls`，可以加上各 `cwd` 的 git 状态，左侧栏已经在读。
- **配合**：和左侧栏同一份数据；点卡片和左侧栏一样聚焦窗格，Changes 随之联动。
- **要不要改 corral／ranch**：不用。
- **工作量（推测）**：小。
- **主要风险**：
  - 和左侧栏高度重复，左侧栏已经能按状态排序。
  - 看不到“还没派”和“已收尾”的活，不算任务看板。
  - 用户若想要的是“管理要做的事”，这个方案满足不了。

### 5.3 方案三：仓库内任务板（可拖，任务文件为数据，拖动可派活）

- **卡片**：一份任务文件（同方案一），可以在看板里新建，生成任务文件草稿。
- **列**：待派、在做、待审、已合并、已收尾，加一列“想法或草稿”。人可以拖，系统也会自动推。
- **拖动**：
  - 拖到“在做”：弹出确认框（参照 New Agent 窗口），建 worktree 和分支，`corral start <名字> --cwd <worktree> --label task=<编号> --prompt "读 docs/任务/<文件> 照做…"`。
  - 拖到“已合并”或“已收尾”：只做标记，或者调用主控的合并、收尾步骤。要不要自动做，用户定。
- **数据**：
  - 任务文件本身，加一个状态字段（frontmatter 或固定格式的一行）。
  - 状态写在 main 还是分支上要定。写在 main 上，每拖一次就要在 main 上提交；写在分支上，就要像 Backlog.md 那样跨分支扫描。
- **配合**：
  - 把现在主控在终端里做的“写任务文件 → 派 agent”搬进界面。
  - Changes 联动同方案一。
  - 要和主控 agent 怎么分工得先定：界面开的 agent 由谁审查、谁合并。
- **要不要改 corral／ranch**：corral 不用改。如果要求派活技能也按同样格式写状态，就要改 ranch 的 corral-dispatch 技能，属于 ranch 的任务，要用户另提。
- **工作量（推测）**：大。要做看板拖放、新建和编辑任务、派发流程、状态写回和跨分支对账，还要改任务文件格式。
- **主要风险**：
  - 越来越像 Drover（§4.5）。
  - 卡片状态和 agent 实际状态是两份数据，要对账（§3.3 第 1 条）。
  - 一拖就开 agent，容易误开。
  - 界面派活和主控派活两条路并存，容易互相踩：同名 agent、同一个 worktree。

### 5.4 方案四：接 GitHub Issues／Projects

- **卡片**：GitHub issue 或 Project 条目。
- **列**：Project 的 Status 字段，靠 GitHub 内置工作流在合并时设 Done，人也可以在看板或 GitHub 上拖。
- **数据**：GitHub，通过 `gh` 命令读写，不加库。
- **配合**：
  - issue 正文可以就是任务文件内容，或者链到任务文件。
  - 开 agent 照方案三的派发流程，PR 合并后 GitHub 自动推到 Done。
  - Changes 照旧。
- **要不要改 corral／ranch**：不用。
- **工作量（推测）**：中到大。要做 `gh` 鉴权、网络、限流、离线时的表现，和 Project 字段映射。
- **主要风险**：
  - paddock 仓库是公开的，issue 也是公开的，任务里的私人信息要小心。
  - paddock 现在本地合并、不走 PR，GitHub 的“合并 → Done”自动化用不上，要改合并方式或自己回写。
  - 数据在 GitHub。
  - 和现行“任务文件在仓库里”的做法是两套。

### 5.5 主控视角的倾向（不是决定）

如果用户还没想好，倾向先做**方案一**：

- 不新增数据，不改 corral，不改现行流程。看板只是把已经存在的“任务文件 → worktree 和分支 → agent → 完成记录 → 审查 → 合并 → 收尾”画出来，状态推出来，就不会和实际状态漂开。
- 它补上了左侧栏没有的东西：还没派的活、已经停掉 agent 但还没合并的活、已收尾的活。
- 它的数据模型就是方案三的前半段。用户用一段时间后如果想“拖一下就派活”，可以在上面加拖动和派发，不用推倒。

方案一的代价是解析靠惯例。如果用户接受，建议同时定一条很小的约定：任务文件固定写 worktree 和分支的一行，派活时加 `--label task=<编号>`。这条约定要用户点头，见 §6 Q4。

方案二最快，但和左侧栏重复；方案四和公开仓库、本地合并的现状不合，主控不倾向这两个。

---

## 6. 要问用户的问题

**Q1. 看板上的一张卡片，你想让它代表什么？**
- a. 一件活（一份任务文件），没开始的、做完的也在看板上。方案一、三。
- b. 一个开着的 agent，像左侧栏换个排法。方案二。
- c. GitHub 上的一个 issue。方案四。
- d. 别的，比如一个 worktree 或分支，不管有没有任务文件。

**Q2. 卡片由谁来移动？**
- a. 全部自动推出来，只能看、不能拖。
- b. 我来拖，系统不自动改。
- c. 自动推，我也可以拖来纠正或标记，比如标“不做了”。

**Q3. 拖卡片或点按钮要不要直接触发动作？**
- a. 不要，只看；派活、合并、收尾照旧由主控在终端里做。
- b. 拖到“在做”就派 agent（建 worktree 和分支、`corral start`），先弹确认。
- c. 只在卡片上放按钮（派出、打开任务文件、看改动），不靠拖。
- d. 合并、收尾也要能在看板里做。

**Q4. 看板的数据放哪里？**
- a. 不新增，只读现有的任务文件、git、corral；接受偶尔认错。
- b. 同 a，另外约定任务文件固定写一行 worktree 和分支、派活时加 `--label task=<编号>`，让对应更准。
- c. 任务文件里加状态字段，看板可以写回。
- d. paddock 自己的本地文件，不进仓库。
- e. GitHub Issues 或 Projects。

**Q5. 看哪些项目、看多久？**
- a. 只看焦点窗格所在仓库。
- b. 所有项目（按 corral 名字前缀，如 `paddock/`、`ranch/`），可以切换。
- c. 只看“开着的或没合并的”，收尾后就从看板上消失。
- d. 已收尾的也保留最近 N 件，N 你来定。

---

## 7. 不知道、没核实

- Conductor 的 backlog、in progress、in review、done 是自动推的还是手动设的。
- AgentsRoom 的数据到底存在仓库还是云端（页面说法矛盾）；它的看板怎么和 worktree 合并配合，只见营销页。
- Nimbalyst 的 tracker 存储格式；agent “可以”自动改状态，具体怎么做。
- Windsurf／Devin 的 Agent Kanban 有哪些列、能不能拖。
- Linear GitHub 集成的状态推进细节：文档页没有取到正文，只有搜索摘要和 changelog。
- GitHub Copilot 开工时 Project 条目会不会自动变成 In Progress。
- corral `stop` 之后 agent 是否还留在 `ls` 里（规矩不允许对用户的 agent 试；可以用自己的测试 agent 试，本轮没做）。
- Claude Code Projects、Windsurf 这类功能都很新（2026 年），列的名字和行为可能还会变。

---

## 8. 来源

- Vibe Kanban：[GitHub 仓库](https://github.com/BloopAI/vibe-kanban)、[task.rs](https://github.com/BloopAI/vibe-kanban/blob/main/crates/db/src/models/task.rs)、[container.rs v0.0.138](https://github.com/BloopAI/vibe-kanban/blob/v0.0.138-20251217105424/crates/services/src/services/container.rs)、[pr_monitor.rs v0.0.138](https://github.com/BloopAI/vibe-kanban/blob/v0.0.138-20251217105424/crates/services/src/services/pr_monitor.rs)、[pr_monitor.rs main](https://github.com/BloopAI/vibe-kanban/blob/main/crates/services/src/services/pr_monitor.rs)、[Issue Management](https://vibekanban.com/docs/issue-management.md)、[Workspaces](https://vibekanban.com/docs/workspaces/index.md)、[关停公告](https://vibekanban.com/blog/shutdown)
- AgentsRoom：[Backlog Task Board](https://agentsroom.dev/features/backlog-task-board)
- Crystal／Nimbalyst：[Crystal README](https://github.com/stravu/crystal)、[Nimbalyst Task Management](https://nimbalyst.com/features/task-management/)
- Backlog.md：[README](https://github.com/MrLesk/Backlog.md)、[ADVANCED-CONFIG](https://github.com/MrLesk/Backlog.md/blob/main/ADVANCED-CONFIG.md)
- Conductor：[Parallel agents](https://www.conductor.build/docs/core/parallel-agents)、[Issue to PR](https://www.conductor.build/docs/guides/issue-to-pr)、[0.35.0 Workspace Status](https://www.conductor.build/changelog/0.35.0-workspace-status)、[0.66.0 Create from issue](https://www.conductor.build/changelog/0.66.0-create-workspace-from-issue)、[changelog 汇总](https://releases.sh/conductor/conductor-changelog.md)
- Claude Squad：[README](https://github.com/smtg-ai/claude-squad)
- Sculptor：[README](https://github.com/imbue-ai/sculptor)、[workspaces](https://github.com/imbue-ai/sculptor/blob/main/docs/help/workspaces.md)、[agents](https://github.com/imbue-ai/sculptor/blob/main/docs/help/agents.md)、[pull_requests](https://github.com/imbue-ai/sculptor/blob/main/docs/help/pull_requests.md)
- Paseo：[llms.txt](https://paseo.sh/llms.txt)
- GitHub：[Use cloud agent on GitHub](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/use-cloud-agent-on-github)、[Projects built-in automations](https://docs.github.com/en/issues/planning-and-tracking-with-projects/automating-your-project/using-the-built-in-automations)、[Copilot coding agent 101](https://github.blog/ai-and-ml/github-copilot/github-copilot-coding-agent-101-getting-started-with-agentic-workflows-on-github/)
- Linear：[Agents in Linear](https://linear.app/docs/agents-in-linear)、[Coding sessions](https://linear.app/docs/coding-sessions)、[Agent best practices](https://linear.app/developers/agent-best-practices)、[Agent interaction](https://linear.app/developers/agent-interaction)、[GitHub workflow updates](https://linear.app/changelog/2023-11-15-github-workflow-updates)
- OpenAI Symphony：[README](https://github.com/openai/symphony)、[SPEC.md](https://github.com/openai/symphony/blob/main/SPEC.md)
- Claude Code：[Agent view](https://code.claude.com/docs/en/agent-view)、[Projects](https://code.claude.com/docs/en/claude-projects)、[Sessions](https://code.claude.com/docs/en/sessions)
- Windsurf／Devin：[Desktop changelog](https://docs.devin.ai/desktop/changelog-next)
- Codex：[CLI 参考（codex cloud）](https://learn.chatgpt.com/docs/developer-commands?surface=cli)、[Codex cloud](https://learn.chatgpt.com/docs/cloud)
