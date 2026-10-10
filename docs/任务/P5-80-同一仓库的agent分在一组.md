# 任务：同一个仓库的 agent 在左侧栏分在同一组

2026-10-10，paddock/main 交给 paddock/dev-repo-groups（Codex，常规档：gpt-6-astra / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：三项都拿不准，主控定：改的是显示的分组规则，不碰数据模型、并发、安全和核心规则）
类型：功能变更
依据：用户 10-10 给了左侧栏截图（`jb-finetune` 一组里是 `codex-1`，`jbfinetune` 另一组里是它派出去的 `dev-single-command-generation-1`）：“这个也补一个任务，jbfine的两个codex，codex-1委派任务的时候倒是成功了，但是没有在一个jb-finetune下。”主控列了甲（按仓库分组）、乙（同仓库的组合并）、丙（改那个项目的前缀）三种做法，用户：“P5-80按甲来，写任务文件吧”；看过任务文件后：“P5-80那四条都按你的来，派出去吧”。本轮只改 paddock 里 agent 归到哪一组、组叫什么；不改 agent 的名字，不改 corral，不改别的项目的规矩。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`（规矩全文）。
- `docs/DESIGN.md` §13 里讲左侧栏分组的几处（搜“分组”：P2-1、P5-1、P5-24、P5-45）。
- 代码：`app/src/agents.rs`（`group`、`Panel::ordered`）、`app/src/card.rs`（`lines`、`card`，组头和卡片短名）、`app/src/sidebar.rs`（`Listing`、Git 摘要按目录存）、`app/src/git.rs`（`repository`、摘要的读取）、`app/src/activity.rs` 的 `main_repository`（已有的“主工作区”读法）、`app/src/window.rs` 里两处 `crate::agents::group`（Attention 列表、新标签／分屏弹层和命令面板的 agent 行）、`app/src/search.rs`（⌘P 按名字或项目找 agent）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-80-repo-groups`，分支 `p5-80-repo-groups`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-80-repo-groups`。
- 只动 `app/` 下和分组有关的代码、测试，`docs/DESIGN.md` 和本文件。

## 现在是怎样、为什么分成两组
- 左侧栏按 agent 名字里第一个 `/` 前面那一段分组（`agents.rs` 的 `group`），不看它在哪个仓库干活。
- `jb-finetune/codex-1`（目录 `…/jb-finetune`）的前缀是 paddock 新建 agent 时按目录名起的；它派出去的 `jbfinetune/dev-single-command-generation-1`（目录 `…/jb-finetune-worktrees/single-command-generation`）的前缀照的是那个项目自己的规矩。两个前缀差一个短横，所以成了两组；那个 worktree 的 Git 公共目录是 `jb-finetune/.git`，其实是同一个仓库。

## 要做的
**组怎么定**
- agent 的目录（`corral ls` 给的 `cwd`，是它启动时的目录）在 Git 仓库里：它属于那个仓库的主工作区这一组，组名是主工作区的目录名。worktree、子目录都算进主仓库。
- 不在 Git 仓库里、目录已经不存在、Git 读不出来：照旧用名字前缀当组名（没有前缀的照旧）。
- 组名相同就是同一组：比如在临时目录里的 `paddock/test-a` 按前缀叫 `paddock`，和 paddock 仓库那一组并在一起。
- 属于哪个仓库跟着现有的 Git 摘要一起在后台读，不另起轮询、不在画界面的线程里跑 git。还没读到时先按名字前缀放，读到后挪到该在的组。
- 组头的样子（名字、个数、状态细条）、组的先后（按组名排）、组里的先后规则都不变，只是“哪些 agent 是一组、组叫什么”换成上面的规则。

**跟着用同一个组名的地方**
- 左侧栏的分组和排序、上下键移动的顺序、收起成窄条后悬停说明里的分组名。
- Attention 列表、新标签／分屏弹层、命令面板里 agent 行名字旁的项目名。
- ⌘P 搜索：按项目找时认组名；按名字找照旧认全名（搜 `jbfinetune` 仍能找到那个 agent）。
- 改之前先全仓库搜一遍还有哪里把名字前缀当“项目”或“分组”用，逐处列在完成记录里：跟着改了的、有意没改的各是哪些。

**不变的**
- agent 的名字、卡片上显示的短名（第一个 `/` 后面那段）、头像字母的取法。
- 标签页、窗格标题、Changes 头部的名字规则（`window.rs` 的 `agent_name`：撞名才带前缀）。
- 新建 agent 时建议的前缀、Kanban 里任务和 agent 怎么对上、Activity 的仓库列表、`paddock ctl` 的输出。
- 例外：同一组里有两张卡片短名相同时（比如 `jb-finetune/main` 和 `jbfinetune/main`），这几张卡片的名字带上各自的前缀，免得分不清。

**文档**
- `docs/DESIGN.md` §13 加一条 P5-80（带用户原话），写明分组规则从“名字前缀”改成“所属仓库，读不到时名字前缀”。

## 主控拿的主意（用户 10-10：“P5-80那四条都按你的来”）
1. 卡片上的名字照旧只显示短名，不因为一组里前缀不同就都带上前缀；只有同组短名相同才带。
2. Attention、弹层、命令面板、⌘P 里的项目名跟着改成组名，免得同一个 agent 在左侧栏和别处叫两个项目。
3. 按 agent 启动时的目录算，不跟着它之后 `cd` 去哪变（corral 只给启动目录）。所以 `paddock/main` 被叫去改 cairn 时仍在 paddock 组。
4. 在临时 Git 仓库里起的 agent（比如测试用的）会进以那个临时目录命名的组，不再跟着名字前缀。

## 怎么算做完
- 用户原话：“jbfine的两个codex，codex-1委派任务的时候倒是成功了，但是没有在一个jb-finetune下。”——主仓库里的 agent 和它 worktree 里的 agent 在同一组，组名是仓库目录名。
- 上面「要做的」逐条达到。
- 验证只做这些：先写测试并看到它因为没实现而失败，再实现（合成的 agent 加给定的“目录属于哪个仓库”：前缀不同、同一仓库的进同一组；不在仓库里的按前缀；组名相同并成一组；还没读到时按前缀；同组短名相同时带前缀；搜索按组名和全名都找得到。再加一项用临时仓库和它的 worktree 测读“主工作区”）；`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 各跑一次。觉得不够，在回复里说，不要自己加。
- 不起窗口、不截图：样子留给用户实际看。

## 不要做
- 不改 agent 的名字，不改 corral、ranch、派活技能，不改别的项目的 AGENTS.md。
- 不读 Corral 的内部状态目录；只用 `corral ls` 已经给的字段和只读的 git 命令。
- 不做：手动把 agent 拖到别的组、给组改名、按别的东西（标签、种类）分组的选项、设置项。
- 不用 Python；不依赖 ratatui、crossterm；不复制 Zed 的 GPL 代码。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）；不要起 paddock 窗口；不要用脚本模拟按键或鼠标。
- 不碰 `corral ls` 里别的 agent（都是用户的）。
- 发现某处用名字前缀的地方改不改拿不准、或者改了会动到「不变的」里列的东西：停下来报告，等决定，不要自己换别的办法。
- 不合并到 main，不推送。只在 `p5-80-repo-groups` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 做了什么

- `git::Summary` 随现有后台 Git 轮询携带主工作区路径，复用 `activity::main_repository` 的只读命令；同一 worktree 的各子目录仍共享一次摘要，分支与改动数仍各按自己的 worktree 读取。没有新增轮询，也没有在 UI 线程执行 Git。
- `Panel::group` 统一用主工作区目录名分组，未读取／不可读时退回名字前缀。侧栏分组、组名排序、上下键顺序、组头计数和状态条、窄条悬停说明，以及 Attention、新标签／分屏弹层、命令面板的项目名使用这一结果。⌘P 同时认组名和原全名。
- 卡片仍取第一个 `/` 后的短名，只给同组重名卡片显示全名；不同组的同名卡片不受影响。DESIGN §13 已加入 P5-80 及用户原话。

### 前缀用途清单

检索了仓库内的 `agents::group`／`group(&…name)`、按 `/` 拆名、从短名反推前缀，以及“前缀／项目／分组”相关说明：

| 位置 | 处理 |
| --- | --- |
| `agents.rs` 的 `Panel::ordered`、由它驱动的 `move_selection` | 改用缓存里的仓库组名；原自由函数 `group(name)` 保留为前缀回退及短名拆分工具。 |
| `card.rs` 的 `lines`（组头、计数、状态条）、`card` | 组头按统一组名，卡片携带组名；短名只在同组重名时带原前缀。 |
| `sidebar.rs` 的 `Listing`／Git 结果接收、`RailTip::of` | 沿用摘要接收流程；悬停不再用全名减短名反推组名，直接使用卡片的组名。 |
| `window.rs` 的 Attention 行及 `Choice::Agent` 行（新标签／分屏） | 两处原 `crate::agents::group` 调用改用 `Panel::group`。 |
| `search.rs::agents`、`window.rs::palette_groups` | 传入已有 Panel，命令面板详情列入组名，匹配原全名和组名。 |
| `card.rs::card`／`title` 的名字前缀处理 | 有意保留：短名与会话题目去掉自身名字前缀的规则不随仓库组名变化。 |
| `sidebar.rs::kind_mark`／`initial` | 有意保留：头像字母仍从原本第一个 `/` 后的短名取，不从重名时显示的全名取。 |
| `window.rs::agent_name` 及标签、窗格、Changes 的使用处 | 有意保留：原撞名时带前缀的标题规则不变。Attention／弹层行自身的短名取法也没改。 |
| `window.rs::agent_matches`／`choices` | 有意保留：新标签弹层原本对全名做文字过滤，没有独立的项目字段；本任务仅改变它显示的项目名，新增按组名检索限定在 ⌘P。 |
| `new_agent.rs::suggest_prefix`／Form、`new_agent_view.rs`、`agent_shell.rs` | 有意保留：新建 agent 的前缀建议、完整名字与启动规则不变。 |
| `kanban_view.rs::short_name`、`kanban.rs` 的 agent 对应关系 | 有意保留：任务里的 agent 短名及匹配规则不变。 |
| `activity.rs`／`activity_view.rs`、`control.rs`／`control_ui.rs` | 有意保留：复用主工作区读取函数但不改其行为；Activity 仓库列表、ctl 输出不变。 |
| `HANDOFF.md`、旧任务与 `docs/调研/P5-29r-Kanban集成.md` 的前缀分组描述 | 有意保留：属于本任务范围外的交接及历史记录；新规则集中记入 DESIGN §13。 |

其余 `/` 拆分用于目录缩写、文件路径、URL 或程序名；Settings／GPUI 的 `group` 是设置分类或悬停样式分组，均与 agent 仓库归组无关，没有修改。

### 验证了什么

- RED：先加 `Summary` 归属字段及给定归属的合成输入，尚未实现归组／读取时运行测试。`repo_groups` 三项因仍按前缀分组、重名仍只写 `main`、搜 `jb-finetune` 无结果而失败；现有临时 Git 仓库／worktree 测试扩充的主工作区断言因返回 `None` 失败，均非编译或夹具错误。
- GREEN：实现后 `cargo test --manifest-path app/Cargo.toml --test repo_groups --test git` 10 项通过。覆盖不同前缀同仓库、未读取时回退、不可读／非仓库回退、组名相同并组、原排序及移动顺序、同组短名冲突、跨组短名不冲突、按组名及原全名搜索；临时真实仓库、worktree 及子目录均读到主工作区。
- 使用独立 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-80-repo-groups`，前台完成：`cargo test --manifest-path app/Cargo.toml --all-targets`（545 项通过）、`cargo clippy --manifest-path app/Cargo.toml --all-targets -- -D warnings`、`cargo fmt --manifest-path app/Cargo.toml --check`、`git diff --check`，四项各运行一次并通过。Cargo 仍提示已有依赖 `block v0.1.6` 的 future-incompatibility，未改依赖。

### 拿主意的地方、没做的事

- 复用现有主工作区读取函数，在每个 worktree 的摘要中保存路径，由纯内存分组函数取目录名；无新配置和持久化格式。测试复用现有临时仓库／worktree 夹具，没有另建一套 Git 测试设施。
- 没起窗口、没截图、没操作真实 agent，也没改 corral／ranch、别的项目或任务。视觉效果留给用户实际看。只在 `p5-80-repo-groups` 提交，不合并、不推送。
- 要主控决定的事：无。

### 主控审查补充：⌘P 省略重复组名

- 按主控补充，只调整 `search.rs::agents`：组名非空且不同于名字第一个 `/` 前的前缀时才放进 detail；相同或没有额外组名信息时只显示原来的状态和目录。不同前缀的仓库组名仍可搜索，相同前缀可由原全名匹配，DESIGN §13 已补充此规则。
- 恢复 search 单元测试原来的三条 detail 断言；保留 `repo_groups` 的不同前缀按组名／全名搜索测试，新增同前缀不重复显示的测试并核对搜索结果。先运行新增测试，因实际仍显示 `paddock · Working · ~/…/paddock` 而失败，再实现并通过。
- 本轮仅运行指定验证，均使用原独立编译目录、前台完成：`cargo test --manifest-path app/Cargo.toml --test repo_groups`（4 项通过）、`cargo test --manifest-path app/Cargo.toml --lib search::tests`（11 项通过）、`cargo clippy --manifest-path app/Cargo.toml --all-targets -- -D warnings`、`cargo fmt --manifest-path app/Cargo.toml --check`。没有重跑全量测试，没有其他功能改动；仍只在本分支提交。无需主控决定事项。

## 主控审查

2026-10-10，paddock/main。结论：可以合并。

- diff（`de3e50f`、`aaff442`）只动了分组相关的代码、测试、`docs/DESIGN.md` 和本文件；「不要做」里的一件都没做（没改名字、corral、新建前缀、Kanban、Activity、ctl）。「怎么算做完」逐条达到：主仓库和它 worktree 里的 agent 同组，组名是主工作区目录名；读不到退回前缀；组名相同并组；同组短名相同显示全名。
- 第一轮后主控在它的 worktree 里重跑 `cargo test --all-targets`：545 项通过，和它报的一致。
- 交回去改过一处（主控的补充，任务文件没写清，不算没达到验收）：⌘P 的 agent 行原先每行都加组名，和名字前缀重复；改成只在两者不同时才写。返工后主控只复跑了 `cargo test --test repo_groups`（4 项）和 search 的单元测试（11 项），通过。
- 它列的取舍，同意：归属放进现有 Git 摘要一起读（每个 worktree 每轮多一条 `git worktree list`）；新标签弹层的文字过滤仍只认全名，按组名找限定在 ⌘P。
- 没验证的：没起窗口。启动时或新 agent 刚出现时会先按前缀放、读到仓库后挪一次组，实际看着突不突兀留给用户。
