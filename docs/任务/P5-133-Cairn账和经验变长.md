# 任务：Cairn 面板的 Ledger、Lessons 改成一行一条，分组可收起，加过滤框

2026-10-17，paddock/main 交给 paddock/dev-cairn-compact（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规、拿不准、拿不准；主控按规则定：只动 Cairn 面板的显示和一个本机偏好文件，不碰数据模型、公开约定和并发）
类型：功能变更
依据：用户 10-17：“另外Ledger和lessons的显示现在是完全平铺，随着item越来越多，会不会显得很长？你要设计一下。”主控出了样稿（画布「P5-133 Cairn 账和经验变长样稿」，源文件在 `docs/设计稿/P5-133-Cairn列表变长/`），A 一行一条 + 分组可收起 + 过滤，B 保留预览每组露 5 条；用户：“选 A，只展开 State，组内新的在上”。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

派出的理由：面板列表的画法、过滤框和收起状态，界面实现按分工给 Claude Code。

## 先读
- `AGENTS.md`（规矩全文）。
- 本文件全部。
- 样稿：`docs/设计稿/P5-133-Cairn列表变长/README.md` 和 `A-Compact`、`A-Filter`、`A-Lessons` 三张；`Main` 是现在的样子。
- `docs/DESIGN.md` §13 P5-118 那条（含 P5-118a、P5-121、P5-121a 的补充）。
- 代码：`app/src/cairn_view.rs`（`entries`、`entry`、`ledger`、`lessons`、`switch`）、`app/src/cairn.rs`（`Group`、`Entry`、`ledger()` 里 P5-132 给 Lessons 排序那段）、`app/tests/cairn.rs`；文字框 `app/src/text_input.rs`，用法参照 `app/src/browser_view.rs` 的查找栏；⌘F 怎么分给 Browser：`app/src/menu.rs` 的 `finds`；本机状态文件的读写参照 `app/src/remote_start.rs` 的 `recent_path` 和 `RecentDirs`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-133-cairn-compact`，分支 `p5-133-cairn-compact`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-133-cairn-compact`（已从 `.target/main` 克隆）。
- 动 `app/src/` 下要动的文件、相关测试、`docs/DESIGN.md` 和本文件。不加依赖。

## 要做的（照样稿 A；颜色从主题取，样稿色值只是示意）
1. **一行一条**（Ledger、Lessons 两页）：每条一行，折叠箭头、`#编号`（等宽、右对齐，照旧）、标题（一行，过长省略号）、右边淡色日期（取 Added 的日子，如 `Oct 16`；不是今年的带年份）。不再画两行正文预览。点一条就地展开、同时只开一条、展开后的正文和 Added／Updated／By／Entry、没带上的那行，都照现在。下个会话没带上的标题照旧画淡。
2. **组内新的在上**：Ledger 每组都按编号从大到小（Lessons 已是，P5-132）。
3. **分组可收起**（Ledger 页）：组头整行可点，前面 `▾`／`▸`，后面条数小签照旧；收起时只剩组头。默认只展开 State，Open、Conventions、Other 收着。用户点过的收起状态**每个仓库各自记住**，paddock 重开后还在：存一个本机状态文件，和 `recent-remote-dirs.json` 放一处、互不干扰；读不到或坏了当没有，用默认。Lessons 页只有一组，不画组头，不可收（照现在）。
4. **过滤框**（两页各一个）：在 `Notes | Ledger | Lessons` 切换下面，占位字 `Filter <N> entries`（Lessons 页 `Filter <N> lessons`），右边 `⌘F`。
   - 按标题和正文过滤，不分大小写，按子串。有字时：框描边用强调色，右边写 `<命中> of <总数>` 和一个清除 `×`；组头条数写 `<命中> of <该组总数>`；标题里命中的字高亮；没有命中的组不显示；收着的组有命中时照样展开显示（不改记住的收起状态，清掉过滤后恢复）。一条都没命中时，列表处写 `No entries match “<字>”`（Lessons 页 `No lessons match …`）。
   - Esc 清掉过滤；点 `×` 也清掉。
   - ⌘F：键盘在 Cairn 面板里时进这一页的过滤框；在别处照旧是终端、Browser 的查找（照 `menu::finds` 给 Browser 的分法）。
   - Ledger、Lessons 的过滤字各自记着，换仓库清空（照现在换仓库复位开着的那条）。不存盘。
5. `docs/DESIGN.md` §13 P5-118 那条后面补一条 P5-133，写清以上（顺带把 P5-132 Lessons 新的在上写进去，如果还没写）。

## 怎么算做完
- 用户原话：“选 A，只展开 State，组内新的在上”。
- 验证只做这些：针对这件活写测试，覆盖过滤（标题、正文、不分大小写、命中数、组没命中时不显示）、组内顺序（编号大的在上）、收起状态（默认只开 State、按仓库分开、读写状态文件，读不到或坏了当默认）、日期短写法（今年和非今年）。`git diff --check`，`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑过一次。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 Notes 页；不改切换胶囊的画法；不加写账的命令；不调 `cairn` 的新命令或参数（只用现在的 `ledger --json`），不读 cairn 的数据库和配置。
- 不改、不读用户真实的 `~/.config/paddock/` 下的配置和状态文件，测试一律用临时目录；不起 paddock 窗口截图（样子留给用户看）。
- 不改终端、Browser 的 ⌘F 行为（键盘不在 Cairn 面板时）。
- 不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西；不要按项目名或路径批量杀进程。
- 遇到要改公开约定（`cairn` 的命令或输出）才能做的，停下来报告。
- 不合并到 main，不推送。只在 `p5-133-cairn-compact` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的，工具或环境和预想不一样的；没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

2026-10-17，paddock/dev-cairn-compact（Claude Code）。

### 做了什么
- `app/src/cairn.rs`：`Entry` 加 `day`（Added 的日子短写，`Oct 16`，不是今年写 `Nov 3, 2025`，按本机时钟），`ledger()` 多收一个 `now`；每组都按编号从大到小（原来只有 Lessons）。新加纯逻辑：`found`（不分大小写的子串位置，给标题高亮用）、`matches`、`filter`（按标题和正文过滤，没命中的组去掉）、`Folds`（每个仓库各自的收起状态，`load`/`open`/`set`/`save`，默认只开 `OPEN_AT_FIRST` = State）和 `folds_path()`（`layout.json` 同目录的 `cairn-folds.json`）。
- `app/src/cairn_view.rs`：一行一条（箭头、`#编号`、标题一行省略、右边淡色日期；去掉两行正文预览，展开部分照旧）；Ledger 组头整行可点 `▾`／`▸` 收起，点一下立即写状态文件；两页各一个过滤框（放大镜、`Filter <N> entries/lessons`、`⌘F`；有字时强调色描边、`<命中> of <总数>`、`×`；组头 `<命中> of <总数>`；标题命中处淡黄底高亮；没命中的组不画；收着的组有命中时展开；全无命中写 `No entries/lessons match “…”`）。面板有自己的焦点，点进去键盘归它；`⌘F` 进当前页过滤框并全选，Esc 或 `×` 清掉当前页过滤；换仓库清空两页的过滤字。
- `app/src/window.rs`：`CairnView::new` 多传一个 `cx`（建文字框、焦点要用）。
- `docs/DESIGN.md` §13 P5-118 那条后面补了 P5-132（Lessons 新的在上）和 P5-133。
- `app/tests/cairn.rs`：原有两处账的测试补 `day`、State 组改成新的在上；新加三个测试：组内顺序和日期短写法（今年、去年、跨年按东八区落到今年）；过滤（标题、正文、不分大小写、前后空格、命中数、组没命中不显示、高亮位置）；收起状态（默认只开 State、按仓库分开、写入再读回、读不到或坏了当默认、文件名和位置）。

### 验证了什么
- `app/` 下 `cargo test --all-targets` 全过（含新加的 3 个）、`cargo clippy --all-targets -- -D warnings` 无警告、`cargo fmt --check` 通过；`git diff --check` 通过。
- 测试都用临时目录和假 `cairn`，没读写真实的 `~/.config/paddock/`、`~/.local/state/paddock/`；没起 paddock 窗口，样子、点击、⌘F、Esc 留给用户实际看。

### 拿主意的地方
- 键盘怎样算“在 Cairn 面板里”：面板根上挂自己的焦点（`track_focus`），在面板里任意处点一下键盘就归面板，点过滤框归过滤框。代价：点一条来看以后，键盘离开了终端，要回终端打字得点一下终端。终端、Browser 拿着键盘时 ⌘F 不变（它们各自先接到）。
- Esc 借用现有查找栏的键位上下文 `menu::FIND`（`CloseFind`），没加新键位；它在整个面板里都生效（不只在过滤框里）。⌘F 在 Notes 页、或这一页没有条目（没有过滤框）时不做事。
- 过滤字前后的空格不算，只有空格当没过滤。
- 过滤时组头不能点（不然点了看不出变化，又会悄悄改记住的状态）；箭头画成 `▾`。
- 收起状态按 `Found.repo`（主 worktree 路径）记；每次点击同步写文件（很小），免得两次后台写顺序颠倒；写失败不提示。
- 占位字一条时用单数（`Filter 1 entry`、`Filter 1 lesson`）。
- 收着的组名用正文色，展开的用标题色（样稿里收着的组名也淡一点）。
- 样稿 A-Lessons 里有一个 `Lessons 6 · Newest first` 组头；照任务文件“Lessons 页只有一组，不画组头（照现在）”，没画。

### 没做的事
- 没改 Notes 页、切换胶囊、cairn 命令和参数；没加写账的命令。
- 正文展开后不高亮命中的字（只要求标题）；展开那条没有加样稿里的淡强调色底（任务说展开后照现在）。
- 面板换到别的标签或收起时，不把键盘还给终端。

### 踩过的坑
- 测试里 `|created: &str| (created, created)` 闭包推不出返回的生命周期，改成内部 `fn`。
- clippy 的 `single_range_in_vec_init` 不让 `assert_eq!(…, [6..12])`，改成 `vec![6..12]`。
- 先写了实现再补测试，没有先看到失败的那一步。

## 主控审查

- 看了 diff：只动了任务里的文件，「不要做」没碰；一行一条、组内新的在上（所有组按编号倒排）、默认只开 State、收起状态按仓库存 `cairn-folds.json`（读不到或坏了当默认）、过滤（标题和正文、不分大小写、命中数、没命中的组不显示、收着的组过滤时展开不改记住的状态）、⌘F 只在键盘在 Cairn 标签时进过滤框、换仓库清空，都对得上任务和样稿 A。
- 重跑一次：`app/` 下 `cargo test --all-targets` 全过（lib 594，另有各集成测试），clippy、fmt、`git diff --check` 干净，和对方说的一致。
- 对方的取舍：Esc 借查找栏的键位上下文、只输入空格当没过滤、过滤时组头不能点、点一下立即写状态文件——同意。没画样稿 A-Lessons 的组头——同意（任务就这么写的）。
- 要用户定的一处：在 Cairn 标签里点一下（包括点开一条），键盘就离开终端归这个标签，回终端打字要再点一下终端。这是 ⌘F 能进过滤框的前提；以前点 Cairn 标签不抢键盘。留给用户实际用过后定。
- 界面画法、深浅主题、⌘F、Esc、中文输入法都没在窗口里看过，留给用户。
