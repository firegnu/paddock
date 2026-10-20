# 任务：看板加 Loop：把选进 LOOP 的任务一件接一件送给主控开工

2026-10-20，paddock/main 交给 paddock/dev-loop（Claude Code，opus[1m] / high，套 fence `dev` 墙）。
依据：用户 10-15：“增加一个draft task，kanban加一个loop的功能，enable loop之后，任务从queue中按照一定的次序排队开发（暂时还没想好如何挑能够并行的），这个loop只在kanban和paddock中，不能牵涉到ranch和corral”；用户 10-20 和主控逐条定了做法（见「用户定的」），样稿（claude.ai 上的“Paddock 看板 Loop 样稿”）上选 A。
路由：重 / 交叉审查不做 / 影响面：碰要害（路由：重（置信 0.97）、要、碰要害；不做交叉审查：用户定主控自己审，不另开交叉审查）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户定的
- **边界**：loop 只在 paddock 和看板里做，不改 ranch、corral 的代码，不往公开约定里加东西；调它们已有的公开命令（`corral send`、`corral ls` 等）可以（用户 10-15：“是指不改corral，调用公开命令没问题”）。一件接一件，不做并行。
- **谁来干**：轮到下一件时，paddock 用 `corral send` 给这个仓库的主控送一句话让它开工，由主控照它的规矩派活、审查、合并、收尾；paddock 不自己开开发 agent（用户 10-20：“甲，给主控送话”）。
- **loop 是外循环，不改主控的内循环**（用户 10-20：“不要改变现在的内循环的逻辑，这个loop我理解其实就是外循环”）：loop 只管“这件收尾了就送下一件”；主控照现在的规矩写任务、派活、审查、等用户实测和点头、合并、收尾，每件照旧停下来等用户确认再合并。
- **看板上的交互（样稿 A）**（用户 10-20：“我还要讨论kanban那边我如何定义哪些任务接下来进loop。这个交互要设计”，选样稿：“A”）：
  - Queued 上面单独一组 LOOP，组标题上有 Loop 开关和一行状态（开没开、正在做哪件、后面还有几件、在等什么）。
  - Queued 卡片悬停多一个 `Add to loop`（循环箭头），加到 LOOP 末尾；草稿没有这个按钮；`依赖：` 没做完的按钮发灰，提示在等哪件。
  - LOOP 里的卡片前面有序号，悬停有 `Move up`、`Move down`、`Remove from loop`；撤出后回到 Queued。卡片在 LOOP 里时原有的按钮（Move to draft、Drop task、改优先级等）照旧。
  - 开关打开后把 LOOP 第一件送给主控，卡片照常进 IN PROGRESS，带 `Loop` 标记；收尾后送下一件；LOOP 空了自己关上。中途关掉，正在做的那件照常做完，只是不再送下一件。
- **撤出**：进了 loop 的任务，只要还没进行中，就可以撤出（用户 10-20：“进了loop只要这个任务不再进行中，就可以从loop中撤出去”）。
- **次序**：照 LOOP 里用户排的顺序，不照 Queued 的高中低。
- **记在哪**（用户 10-20：“同意，记在paddock里”）：LOOP 里有哪些、什么顺序、开关开没开、送出去的是哪件，按仓库各存一份，放在 paddock 的数据目录（和 `kanban-repos.json` 放一起），不提交 git。看板读时对一遍 git：已开工以外的原因不在 Queued 了（做完、Drop、退回草稿、文件没了）的自动从 LOOP 拿掉。
- **开关**（用户 10-20：“同意，每个仓库一个，重开保持”）：每个仓库一个，看板切到哪个仓库就管哪个仓库的 loop。paddock 重开后开关和 LOOP 都保持原样；重开不会立刻送话，照下面的条件来。
- **怎么算一件做完**：只认 git：main 上出现这件的收尾提交（DONE，含 `不做` 的 Dropped）。主控回复里的 DONE 不算。
- **在等用户**：送出的那件的任务文件有 `待用户：` 时，LOOP 显示 Needs you、等着，不跳去下一件。这靠主控照 CLAUDE.md 已有的规矩写 `待用户：`；不另加规矩。
- **送话的时机**（用户 10-20 指出“没人 attach 才送”在 paddock 里开着主控窗格时会一直不送，改成下面这样）：不看有没有 attach，靠 `corral send` 自带的保护：主控在忙（`not_idle`）或人刚在它的窗口里操作过（`human_active`）时它会拒，loop 不加 `--force`，过一会儿再试（比如 30 秒后），不算出错、不标 Needs you。`paused`、`not_delivered` 才标 Needs you、写原因、给 `Retry`。送成功但 `merged_with_draft` 为 true（输入框里原有用户没提交的字被一起提交了）时，不重送，收件箱提醒一条。
- **卡住**（用户 10-20：“同意，15分钟”；返工 1 改规则，用户 10-20：“同意，交回去改吧”）：送出后主控 idle，并且这件活的开发 agent（带 `task=<编号>` 标签、不是主控的那些）也都不在干活（working），这样满 15 分钟、这件既没收尾也没写 `待用户：`，算卡住；计时从主控和这件的开发 agent 里最后一次有动静（状态变化）算起。开发 agent 停在 blocked 时卡片本来就有 Needs you，不算卡住。卡住时：LOOP 标 Needs you、状态写 `Stuck: <编号> not wrapped up, controller idle 15m`，收件箱来一条；paddock 不自己催。卡住时 LOOP 上有 `Nudge`（再送一句让主控接着做或说明卡在哪）、`Skip`（这件放回 Queued，接着送下一件）；也可以关掉 Loop。
- **停下来和告诉用户**（用户 10-20：“同意”）：
  - LOOP 都做完了：自己关上，状态写 `Loop finished · N done`，收件箱留一条。
  - 这个仓库没有主控（没有 `role=controller` 的 agent，或已退出）：不关、不送，标 Needs you、写 `Waiting: no controller`；主控回来接着送。
  - 两个以上主控：同样不送，写 `Waiting: more than one controller`。
  - `corral send` 报错：不自动重试，标 Needs you、写原因，给 `Retry`。
  - 排第一的现在不能开工了（加了依赖、写了 `待用户：` 等）：跳过它送下一件能开工的，它留在 LOOP 里标出在等什么。
  - 要用户处理的（卡住、没有主控、送话失败）LOOP 标 Needs you，收件箱也来一条；不用处理的（全部做完）只在收件箱留一条。

## 现在是怎样（主控 10-20 对着代码核对的）
- 看板（`app/src/kanban.rs` 读、`app/src/kanban_view.rs` 画）的分组是 `Column`：Queued、In progress、To review、Merged、Done，全从任务文件、git 提交、worktree 和带 `task=<编号>` 标签的 agent 推出来；Queued 按 `优先：` 高中低、同档草稿在前、再按编号排；`依赖：`、`待用户：`、格式问题会标出来。卡片悬停按钮（Queue task、Move to draft、Drop task、Set priority 等）和就地确认的做法见 P5-151、P5-153。
- 看板能切仓库（P5-85），选过的仓库记在 `kanban_repos.rs` 的 `kanban-repos.json`（`layout_state::default_path()` 同一目录）。
- 给 agent 送话：`corral.rs` 的 `Client::send`；agent 的角色标签 `role=controller` 由 `corral::Role` 读出。收件箱（`inbox.rs`、`window_inbox.rs`，P5-87）列需要人处理的 agent 和主控的新回复。
- 收尾提交首行「收尾: <编号> 」把卡片挪进 DONE（`kanban.rs` 的 `wrapped_id`、`dropped`）。

## 要做的
1. **LOOP 组和卡片按钮**：照「用户定的」的看板交互做。界面文字用英文（`Loop`、`Add to loop`、`Remove from loop`、`Move up`、`Move down`、`Nudge`、`Skip`、`Retry` 和上面写的几句状态，措辞可以调顺）。
2. **loop 的状态和保存**：按仓库存、读、对 git 校正，规则照「用户定的」。
3. **送话**：条件全部满足才送：开关开着；这个仓库正好一个主控；仓库里没有进行中的活（In progress、To review、Merged 里都没有卡片，包括不是 loop 送出的，和主控“上一件没收尾不派新活”一致）；LOOP 里有能开工的。送的话：`开始做 <编号>（<标题>）：照 docs/任务/<文件名> 和你的派活规矩做。— paddock loop`（措辞可以调，要能看出是 loop 送的、哪件、哪个文件）。`corral send` 被拒和报错照「用户定的」的送话时机处理；送出后记下是哪件、什么时候送的。
4. **判断做完、等用户、卡住、停下**：照「用户定的」。15 分钟、送话的话放成一处常量，测试里能换成短的。
5. **收件箱**：上面要用户处理的几种、和全部做完，各来一条，点了切到看板这个仓库。
6. `docs/DESIGN.md` §13 加一条 P5-106，写清 loop 是外循环、做完只认收尾提交、状态存在哪；CLAUDE.md（就是 AGENTS.md）「看板约定」补一句 Loop 会用 `corral send` 给主控送 `开始做 <编号>`。

## 怎么算做完
- 用户原话：“kanban加一个loop的功能，enable loop之后，任务从queue中按照一定的次序排队开发”，以及「用户定的」里每条后面的原话。
- 主控建议的验收（请用户确认）：
  1. 有针对测试先红后绿，边角也要覆盖（用合成数据、临时 git 仓库和假 `corral` 脚本，不碰本仓库的 git、不送给真 agent）：送话的条件（各条缺一不送）、送的内容、`not_idle`／`human_active` 时过一会儿再试而不报错、`paused`／`not_delivered` 标 Needs you、`merged_with_draft` 提醒；收尾提交出现后送下一件；`待用户：` 时等着；卡住按 idle 时长算、working 不计；跳过不能开工的；没有主控、多个主控、送话失败；LOOP 空了自己关；按仓库保存、重开读回、对 git 校正拿掉不在 Queued 的；撤出、上移下移。
  2. 三项检查全过（`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`）。
  3. 用户实际用：把一两件小任务加进 LOOP、打开开关，看它送给主控、收尾后接着送下一件、都做完自己关上。
- 验证只做这些：上面第 1 条先红后绿，三项检查各跑一次（`CARGO_TARGET_DIR` 用下面写的目录）。第 3 条留给用户。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 ranch、corral，不往它们的公开约定里加东西；不做并行。
- 不改主控的规矩和派活技能；loop 只送一句话。
- 不改看板读 git 的方式、列的划分、圈的画法；不改 Queue task、Move to draft、Drop task、Delete draft、Set priority、Clear 的行为。
- “进了 loop”不写进任务文件、不产生 git 提交。
- 不加新的主题颜色键；不加锁文件里没有的依赖；不改 `gpui-pre-*` 的版本。
- 不复制 Zed 的代码（GPL），只参考思路。
- 不起 paddock 窗口、不截图、不用脚本模拟按键或鼠标；不碰 `corral ls` 里的 agent，测试里不对真 agent `corral send`。
- 不靠删测试、放宽断言、缩短超时来通过。
- 界面文字用英文。
- 不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。不按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 有一个老测试 `view::links::tests::a_shell_s_relative_paths_are_from_where_it_has_gone_to` 可能因为 PATH 里别的 `clear` 失败，和本件无关；碰到时用 `PATH="/usr/bin:/bin:$PATH"` 跑。
- 不装 paddock.app（`bundle --install` 由主控审过后来做）。
- 不合并到 main，不推送。只在 `p5-106-kanban-loop` 上提交，提交说明用中文、首行以 `P5-106：` 开头。
- 遇到「用户定的」在代码里做不到或互相冲突，停下来写进回复，等主控决定，不要自己换办法。

## 先读
- 本文件全部；`docs/DESIGN.md` §13 里 P5-29（看板约定）、P5-85（切仓库）、P5-87（收件箱）、P5-151、P5-153 几条。
- 代码：`app/src/kanban.rs`、`app/src/kanban_view.rs`（Queued 卡片的按钮和就地确认）、`app/src/kanban_repos.rs`、`app/src/corral.rs`（`Client::send`、`Role`）、`app/src/inbox.rs`、`app/src/window_inbox.rs`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-106-kanban-loop`，分支 `p5-106-kanban-loop`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-106-kanban-loop`（从 `.target/main` 克隆，已清掉 paddock 自己的产物）。开你时已经用 `--env` 设好，cargo 命令直接用。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：做了什么、验证了什么（哪些测试先红后绿）、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具的行为和预想不一样的；没有就写「没有」），在你的分支里提交。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 做了什么
- `app/src/kanban_loop.rs`（新）：loop 的状态和判断，不碰界面。
  - `Loop` 存开关、LOOP 里的编号和次序、送出的那件（编号、送出时间、Nudge 时间）、做完几件、全部做完、送话失败的原因。
  - `settle` 每次读完对一遍 git：送出的那件收尾了（含不做）就算做完一件；做完、Drop、退回草稿、文件没了的从 LOOP 拿掉；LOOP 空了自己关上，记下 `Loop finished · N done`。
  - `look` 按「用户定的」顺序判断现在该怎样：关着、等用户、没有主控／多个主控、送话失败、卡住、做着、仓库里有进行中的活、没有能开工的、30 秒后再试、送给谁。
  - `deliver` 调 `corral send`（不加 `--force`）：`not_idle`、`human_active` 算稍后再试，别的拒绝和报错算失败。
  - `Loops` 按仓库存在数据目录的 `kanban-loops.json`，原子写（先写 `.saving` 再改名）。
  - 15 分钟、30 秒、送的那两句话都放在文件开头（`Timing`、`start_words`、`nudge_words`），测试里能换。
- `kanban.rs`：卡片多一个 `waits`（Queued 卡片没做完的依赖编号，给发灰的 `Add to loop` 和判断能不能开工用）；`controllers_for` 列出仓库的全部主控（原来的 `controller_for` 取第一个，照旧）。
- `corral.rs`：`Client::deliver` 返回送成功时的 `merged_with_draft`；`send` 改成调它，行为不变。
- `kanban_view.rs`：
  - Queued 上面加 LOOP 组：循环箭头、`Loop`、件数、Needs you、开关；下面一行状态，需要时带 `Retry`，或 `Nudge`、`Skip`。窄的单列和宽的多列都有。
  - Queued 卡片悬停多 `Add to loop`；依赖没做完时发灰，提示 `Waits for <编号>`；草稿没有这个按钮。
  - LOOP 里的卡片圈换成序号，悬停有 `Move up`、`Move down`、`Remove from loop`，原有按钮照旧。送出还没开工的那件换成循环箭头，可以 `Remove from loop`。进了 In progress 等列的带 `Loop` 标记。
  - 推进 loop 的是窗口里唯一的那个看板：开关开着的仓库每 5 秒在后台线程读一次 git，结果交给 `settle`／`look`，该送就在后台 `corral send`。不管 Kanban 标签显不显示、切到哪个仓库都推进。启动 5 秒后才开始读，重开不会立刻送。
- 收件箱（`inbox.rs`、`window_inbox.rs`、`window.rs`）：
  - loop 要用户处理的（没有主控、多个主控、送话失败、卡住、等用户）在 `Needs you` 各一行，只要还是这样就留着。
  - 只是告诉用户的（全部做完、`merged_with_draft`）在 `New replies` 一行。
  - 铃铛也数这些行。点这一行或按 ↩：关掉收件箱，切回本机，右侧栏开到 Kanban 并转到那个仓库。右边显示那句话和 `Show board`。
- 文档：`docs/DESIGN.md` §13 加 P5-106；AGENTS.md「看板约定」补一句 Loop 用 `corral send` 送 `开始做 <编号>`。

### 验证了什么
- 先红后绿：`kanban_loop.rs` 的 12 个单元测试先写，跑红（缺实现）后实现到绿。
  - 送话条件各缺一条就不送；送的内容；跳过不能开工的；收尾后送下一件；`待用户：` 时等着；卡住只算 idle 时长、working 不计、Nudge 后重新算。
  - 没有主控、多个主控、送话失败要用户处理；LOOP 空了自己关。
  - 对 git 校正拿掉不在 Queued 的；加、撤、上移下移；按仓库保存和读回。
  - 假 `corral` 脚本：`not_idle`／`human_active` 算稍后再试，`paused`／`not_delivered` 和报错算失败，`merged_with_draft` 读得出，参数里没有 `--force`。
- 以下测试是写完实现后补的，没有经过红：
  - `tests/kanban_loop.rs` 的 2 个，在临时 git 仓库和假 `corral` 上走完整过程：两件先后送出、收尾后送下一件、从文件读回、做完自己关上，记录的调用逐字核对；git 里离开 Queued 的被拿掉，手动开工的挡住送话。
  - `tests/inbox.rs` 的 1 个：loop 的行进对组、说完就走。
- 三项检查：`cargo test --all-targets` 全过（老测试 `view::links::tests::a_shell_s_relative_paths_are_from_where_it_has_gone_to` 因 PATH 里别的 `clear` 失败，照本文件用 `PATH="/usr/bin:/bin:$PATH"` 重跑全过）；`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 通过。
- 界面（LOOP 组、按钮、收件箱行）没有起窗口看，留给用户实际用。

### 拿主意的地方
- 所有仓库的 loop 存在一个文件 `kanban-loops.json` 里，按仓库路径分开，和 `kanban-repos.json` 放一起。
- 送出的那件从 LOOP 的排队里移到“送出”一格：序号只排还没送的。
- 不是 loop 送出就开工了的那件留在 LOOP 里、带 Loop 标记，并按“仓库里有进行中的活”挡住送话（「用户定的」只说已开工以外的原因才拿掉）。
- 卡住的计时 = 现在减去（送出时间、Nudge 时间、主控进入现在这个状态的时间）三者最晚的，只在主控 idle 时算。
- Nudge 被 `not_idle`／`human_active` 拒：不算失败，不提示（主控在忙就不算卡住）；别的错标 Needs you、给 Retry。
- 送话失败的原因存进文件，重开后照样等 Retry，不自动送。
- 只是告诉用户的收件箱行只在内存里，点过就没了；要用户处理的跟着 loop 当时的情况，好了就走。
- LOOP 空着时开关不能打开（提示说先加任务）。`N done` 把标成不做的也算进去。
- 状态里“等用户”写成 `Needs you: <编号> — <待用户的内容>`。

### 没做的事
- 没起窗口、没截图，界面和真实主控上的整套流程留给用户实测（验收第 3 条）。
- 没对真 agent 送过话。

### 踩过的坑
- 沙箱里提交时 git 报 `Unable to create '.../paddock/.git/packed-refs.lock': Operation not permitted`，但提交已经落在分支上（HEAD 和分支一致）。
- `/tmp` 不能写，测试的临时文件改用 scratchpad。
- 读不到 `../ranch`，也不能跑 `corral`：`merged_with_draft` 等字段的含义按本仓库文档（P5-87）来。那里记着多行、空输入框时这个字段不可靠；loop 送的是一行。
- 违反规矩各一次，已停：
  - 早先查环境时跑过一次 `python3 --version`。
  - 用 `rm -f` 删过一个空的临时文件。

### 返工 1
主控审查后，用户 10-20 同意改卡住的规则：“同意，交回去改吧”。

**改了什么**
- `kanban_loop.rs` 的 `look` 改了卡住的判断。
  - 这件的开发 agent：带 `task=<编号>` 标签、不是主控的 agent。
  - 有开发 agent 在 working，或停在提问上（Waiting，卡片本来就标 Needs you），就不算卡住。
  - 都没在干活时，计时从这几样里最晚的算起：送出时间、Nudge 时间、主控进入当前状态的时间、这件每个开发 agent 进入当前状态的时间。
  - 没有开发 agent 时照旧。
  - 状态那句 `Stuck: <编号> not wrapped up, controller idle 15m` 没改。
- `STUCK` 常量的说明、任务文件「用户定的」卡住那条、DESIGN §13 P5-106 都改成新规则。
- 拿主意的地方：
  - 开发 agent 退出（Exited）也算一次状态变化，计入“最后一次有动静”。
  - Working、Waiting 以外的状态（Idle、Error、Stalled、Starting 等）都算不在干活。

**验证了什么**
- 先红后绿：新测试 `while_its_developers_work_the_task_is_not_stuck_and_counts_from_their_last_change`。
  - 改之前跑红，失败在“开发 agent 在干活、主控 idle 很久”那一条，被判成了卡住。
  - 改后变绿。它覆盖了这些情况：
    - 开发 agent 在干活时，主控 idle 再久也不算卡住。
    - 开发 agent 停在提问上，不算卡住。
    - 开发 agent 5 000 秒时停下，5 899 秒不算卡住，5 900 秒算卡住（idle 900）。
    - 别的任务的开发 agent不算。
    - 带 task 标签的主控仍当主控，不当开发 agent。
- 回归：
  - 原有的 `stuck_counts_only_the_time_the_controller_idles`（没有开发 agent 的情况）照旧通过。
  - `kanban_loop` 单元测试 13 个、`tests/kanban_loop.rs` 2 个、`kanban::` 单元测试 31 个都通过。
- `cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次，通过。
- 照主控的要求，没有重跑全套测试。

**踩过的坑**：没有。

## 主控审查（10-20）
- 范围：新文件 `kanban_loop.rs` 管 loop 的状态、保存（数据目录 `kanban-loops.json`，不进 git）、对 git 校正、送话条件和判断；`kanban_view.rs` 加 LOOP 组、开关、状态行、Retry／Nudge／Skip，Queued 卡片 `Add to loop`，LOOP 卡片序号和上移下移撤出；`corral.rs` 的 `Client::deliver` 带回 `merged_with_draft`；收件箱加 loop 的提醒；开关开着的仓库每 5 秒在后台推进。没改 ranch、corral，没改看板读 git 的方式和已有按钮。DESIGN §13、AGENTS.md 补了。
- 第一轮重跑（`.target/p5-106-kanban-loop`）：`cargo test --all-targets` 977 过 0 失败；clippy `-D warnings` 无警告；fmt、`git diff --check` 过。
- 返工 1：对方指出“主控派出开发 agent 后自己 idle，开发超过 15 分钟就会报卡住”，是任务文件规则漏了；用户 10-20 同意改成“主控和这件的开发 agent 都不在干活才计时”。改后针对测试先红后绿；主控重跑 `kanban` 单元测试 50 个、`tests/kanban_loop.rs` 2 个，都过。
- 对方的取舍：Skip 已开工的那件后，下一件仍等它收尾（和“上一件没收尾不派新活”一致）：同意；`merged_with_draft` 不交 ranch：同意；不是 loop 送出却开工的那件留在 LOOP 带标记并挡住下一件、送话失败的原因存盘重开后等 Retry、LOOP 空着时开关打不开：同意。
- 对方违反规矩各一次（跑过 `python3 --version`、用过 `rm -f` 删空临时文件），没有后果，已提醒。
- 建议改（不挡合并）：后台每 5 秒读一次 git，只在开关开着的仓库跑，用户实测时留意耗电。
- 结果：返工 1 次
