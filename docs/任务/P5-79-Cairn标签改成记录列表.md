# 任务：Cairn 标签正文改成一条条记录的列表，点开看单条

2026-10-10，paddock/main 交给 paddock/dev-cairn-notes（Claude Code，常规档：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规、不要、改行为）
类型：功能变更
依据：用户 10-10 看了 P5-55 的 Cairn 标签（正文是整段原文）后：“这块的界面要整理一下，现在的太乱了。我不知道展示的是什么信息。而且没有cairn中的我理解是不是一条条的item，然后可以按照时间点开看这种。而且你要让我看得懂，这条记录是什么时间之类的。这个你要好好规划一下展示”。更早一次对同一块：“我看不懂这个”。主控出了样稿、列了三处要定的事（就地展开、cairn 那一半交给 cairn 主控、现场对比只留在底部原文里），用户：“都按你的建议来，写任务文件吧”；cairn 0.3.0 装好、字段补全后：“可以，P5-79 派出去吧”。本轮只改 Cairn 标签的正文和它读 cairn 的部分，不改 cairn，不改别的标签。
依赖：P5-78
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`（规矩全文）。
- `docs/DESIGN.md` §3 的 cairn 一行、§13 的「右侧栏第四个标签 Cairn（P5-55）」和「P5-78」两条：现在的读法、各种状态、正文的画法。
- 样稿 `docs/设计稿/P5-79-Cairn记录列表/`（README 和两个 `.dc.html`）。配色是近似值，实现时从主题取色；内容是示意。
- 代码：`app/src/cairn.rs`、`app/src/cairn_view.rs`、`app/tests/cairn.rs`；时间用到 `app/src/card.rs` 的 `short_time` 和 `app/src/activity.rs` 的 `local_offset`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-79-cairn-notes`，分支 `p5-79-cairn-notes`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-79-cairn-notes`。
- 只动 `app/src/cairn.rs`、`app/src/cairn_view.rs`、`app/tests/cairn.rs`、`docs/DESIGN.md` 和本文件；别的文件确实要动（比如 `markdown.rs`）在完成记录里写明为什么。

## cairn 给的东西
已有的（cairn 0.2.0，cairn DESIGN §7.1 的公开约定，P5-55、P5-78 在用）：`cairn status --json`；`cairn show --json` 的 `text`（下次会话开场收到的全文）和 `record_ids`（这段全文里包含的记录编号）。

新加的两条是 cairn 的 F3（cairn 0.3.0，10-10 已装到本机；0.2.0 及更早没有）。约定全文在 cairn DESIGN §7.1 的「`cairn list --json`」「`cairn show <ID> --json`」两条，以那里为准；下面是 paddock 用到的部分，主控 10-10 在 paddock 仓库里对着 0.3.0 的真实输出核过（字段名、`total`、不认识的参数退出码 2、记录不存在退出码 1）。

**`cairn list --json --limit N`**：只读（不收取暂存区、不建库、不升级库），所以刚存、还没收取的记录不在里面。退出码 0，stdout 一个对象：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `total` | 非负整数 | 符合条件的记录条数，按 `--limit` 截断之前 |
| `records` | 数组 | 记录，新的在前，最多 N 条 |

- 默认只给当前目录所在项目里没被删除、没被撤回、没被取代的记录，**各种 `kind` 都在内**（更正、撤回、恢复也各算一条，`total` 也把它们算进去）。
- 还没有数据库、项目从没采用过、没有记录：都是 `{"total":0,"records":[]}`。
- 出错：退出码 1，stderr 一行原因。0.2.0 及更早不认识 `--json`，退出码 2。
- `records` 里每条：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `id` | 字符串 | 记录编号 |
| `created_at` | 字符串 | 保存时间，RFC 3339 UTC 毫秒，如 `2026-10-10T08:50:43.687Z` |
| `agent` | 字符串 | `claude`、`codex`，或 `local`（不属于哪一家的 hook 会话，比如手动保存、用户命令写的更正）；以后可能有新值 |
| `session_id` | 字符串或 `null` | 那家 agent 的会话编号；`local` 时为 `null` |
| `source_id` | 字符串 | 来源 |
| `line_path` | 字符串 | 工作线（worktree 根目录的绝对路径） |
| `branch` | 字符串或 `null` | 保存时所在的分支 |
| `kind` | 字符串 | `checkpoint`、`correction`、`retraction`、`restore`；以后可能有新值 |
| `target_id` | 字符串或 `null` | 更正、撤回、恢复所指的记录 |
| `deleted_at`、`replaced_by`、`retracted` | 字符串或 `null`、字符串或 `null`、布尔 | 状态；默认的列表里三个都是空 |
| `summary` | 字符串 | 一行摘要（正文“## 停点”一节的第一个非空行）；没有时是空字符串 |

**`cairn show <ID> --json`**：会先收取暂存区（会写数据库）。还没有数据库：退出码 0，`{"status":"no_data"}`。记录不存在：退出码 1，stderr 一行。否则退出码 0，stdout 一个对象：上面每条里除 `summary` 外的全部字段，加上：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `body` | 字符串或 `null` | 正文原文（Markdown）；正文已删除或这条记录本来没有正文时为 `null` |
| `correction` | 对象或 `null` | 这条记录最新的一条更正（用户用 `cairn correct` 写的），没有时为 `null`；对象里是同样的字段（含 `body`） |

不认识的字段一律忽略。

## 要做的
样子以样稿为准，下面是规则。界面文字用英文；记录的摘要和正文是 agent 写的，原样显示。

**读（`app/src/cairn.rs`）**
- 现有的 `status --json`、`show --json` 照旧，状态判断照旧。已采用、至少一家装了 hook 时，在 `show --json` 之后（它会收取刚存的记录）再跑 `cairn list --json --limit 50`。
- 列表里只留 `kind` 是 `checkpoint` 的；更正、撤回、恢复和不认识的种类不画。
- 点开某条时才跑 `cairn show <ID> --json` 读全文，读到的留着，标签开着期间不重复读。
- 旧版 cairn（`list --json` 退出码 2）：不算出错，正文照现在的样子画（只有那段原文，不折叠）。退出码 1 或读不出 `records`：和别的命令出错一样画 `Could not read cairn`。
- 测试只用假 `cairn` 脚本。

**画（`app/src/cairn_view.rs`）**
- 顶上两行（仓库名、分支、`Adopted`；Hooks 状态行）不变。没装 cairn、没装 hook、没采用、出错、Adopt 确认这些状态不变。
- 标题一行 `Handover notes`（不写条数：cairn 的 `total` 把更正、撤回也算在内，对不上画出来的条数），下面一行小字：`Agents write one when a turn ends. A new session in this repository starts from the newest.`
- 记录按本地日期分组，新的在上。组头：今天 `Today`、昨天 `Yesterday`，后面淡色写日期（`Sat, Oct 10`）；更早的只写日期；不是今年的带年份。组头只首字母大写（P5-45）。
- 每条：
  - 第一行：折叠箭头；本地时间 `17:44`（等宽字）；今天的记录后面淡色写离读取时多久，写法和 Hooks 那行一样再加 ` ago`（`45s ago`、`2m ago`、`2.1h ago`），更早的不写；右边是哪家（`agent` 是 `claude` 写 `Claude`、`codex` 写 `Codex`、`local` 写 `Manual`，别的值原样写）和分支小签（没有分支就不画）。
  - 下面摘要，最多两行，放不下截断。
- `show --json` 的 `record_ids` 里的记录（下次会话会接到的）画成高亮卡片：强调色的淡底和细边，顶上一行 `Next session starts from this`，摘要最多三行。
- 点一条就地展开，再点收起，同时只开一条；窗格换了仓库就收起：
  - 先四行：`Saved`（本地完整时间，`Oct 10, 2026 at 17:44:51`）、`By`（哪家，加 `session` 和会话编号前 8 位；没有会话编号就只写哪家）、`Branch`、`Record`（记录编号）。
  - 再是正文（`body`），按 Markdown 画，沿用现在正文的画法（`##` 是加粗小标题，段落里的换行照换行）。
  - 这条记录有更正（`correction` 不是 `null`）时，正文后面加一条细线、一行淡色的 `Corrected` 加更正的保存时间（写法同 `Saved`），再画更正的正文。样稿里没画这种情况。
  - 全文还没读到时写 `Loading…`；读失败在展开的地方写 `cairn show <编号>: ` 加错误第一行，列表照常。
- 列表末尾：`records` 的条数比 `total` 少时一个按钮 `Show older`（不写条数，理由同标题），点了把 `--limit` 加 50 再读。
- 最下面一行收起的 `Exact text the next session receives`，点开在它下面画现在的那段原文（现在正文的画法原样保留，包括“现场对比”等），再点收起。主界面不单独画“现场对比”和“之后观测到的事件”。
- 已采用但一条记录都没有：照现在的 `No records yet` 那两句，不画标题和列表。
- 离现在多久的文字在读的时候算好再存（和 P5-78 一样），面板只在读到的东西变了才重画。

**文档**
- `docs/DESIGN.md`：§3 的 cairn 一行补上新用到的两条输出；§13 加一条 P5-79（带用户原话），并把 P5-55 那条里“不做：历史记录列表、点开单条记录”改掉。

## 怎么算做完
- 用户原话：“这块的界面要整理一下，现在的太乱了。我不知道展示的是什么信息。而且没有cairn中的我理解是不是一条条的item，然后可以按照时间点开看这种。而且你要让我看得懂，这条记录是什么时间之类的。”
- 用户看过样稿后：“都按你的建议来”——上面「要做的」逐条达到，样子和样稿一致。
- 验证只做这些：先写测试并看到它因为没实现而失败，再实现（`app/tests/cairn.rs`，假 `cairn` 脚本：读到列表、按天分组和时间文字、哪些是下次会接到的、不是 checkpoint 的不画、还有更早的、点开读单条、带更正的、读单条失败、旧版 cairn 的 `list --json` 退出码 2、一条记录都没有）；`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 各跑一次。觉得不够，在回复里说，不要自己加。
- 不起窗口、不截图：样子留给用户实际看。

## 不要做
- 不改 cairn 仓库，不读 cairn 的数据库、配置和 hook 文件，不调用本文件没列的 cairn 命令。
- 不做：被撤回、被取代记录的显示和筛选；更正、撤回、删除记录；Unadopt；安装或卸载 hook 的按钮；搜索、按 agent 或分支筛选；手动刷新按钮；设置项。
- 不加日期库；不用 Python；不依赖 ratatui、crossterm。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）；不要起 paddock 窗口；不要用脚本模拟按键或鼠标。
- 不碰 `corral ls` 里别的 agent。
- cairn 的输出和本文件「cairn 给的东西」对不上，或者样稿和规则有冲突：停下来报告，等决定，不要自己换别的办法。
- 不合并到 main，不推送。只在 `p5-79-cairn-notes` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

2026-10-10，paddock/dev-cairn-notes。

- **做了什么**：
  - `app/src/cairn.rs`：已采用、装了 hook 时在 `show --json` 之后跑 `cairn list --json --limit N`，只留 `checkpoint`，在读的时候按本机日期分好组、把时间写成文字（`17:44`、今天的 `2.1h ago`、组头 `Today`／`Yesterday` 加 `Sat, Oct 10`、不是今年的带年份）、标出 `record_ids` 里的那些、算出还有没有更早的；`Body::Records` 从一段文字改成 `Records { text, notes }`，旧版 cairn 时 `notes` 是 `None`。新增 `note()` 跑 `cairn show <ID> --json`，给出 `Saved` 的完整时间、哪家、会话编号前 8 位、分支、正文和更正。`read` 多两个参数（要几条、本机时区，测试用固定东八区），`Found` 多一个 `repo`（主工作区路径，用来判断窗格是不是换了仓库）。命令出错的前缀取第一个选项之前的词，所以读单条失败写成 `cairn show <编号>: …`；退出码 2 单独记一种，只有 `list` 把它当成旧版。
  - `app/src/cairn_view.rs`：`Handover notes` 标题和说明、按天分组的列表、下次会接到的高亮卡片、点开就地展开（四行、正文、更正；`Loading…`；读失败）、`Show older`、底部收起的 `Exact text the next session receives`。原来画正文的函数拆成“往容器里加 Markdown 各块”一个，旧版 cairn、底部原文、单条正文、更正的正文都用它。
  - `app/tests/cairn.rs`：新增三项，假 `cairn` 脚本多认 `show <ID>`（读 `show-<ID>.out` 等）；原有的测试跟着改了参数和 `Records` 的形状，七种状态那项后半段改成旧版 cairn（`list` 退出码 2）。
  - `docs/DESIGN.md`：§3 的 cairn 一行补上两条新命令和用到的字段；§13 加 P5-79 一条，P5-55 的“不做”里去掉历史记录列表和点开单条。
- **验证**：先加类型和两个空实现（列表当作读不到、读单条直接报错），写好测试，看到新增三项因为没实现而失败（读单条得到 `Err("… not read")`；列表是 `None`；`list` 退出码 1 没有变成出错），原有四项通过；再实现，七项全过。三项分别覆盖：读到列表、按天分组（含本地时间跨过零点、去年的）和时间文字、哪些是下次会接到的、不是 checkpoint 的不画、还有更早的和 `--limit 100`；点开读单条、带更正的、读单条失败（退出码 1、读不出、时间读不出、没有 cairn）；旧版 `list --json` 退出码 2、退出码 1 和读不出 `records` 算出错、一条记录都没有（含 `no_data` 时不跑 `list`）。`cargo test --all-targets` 545 项通过，`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 干净，各跑一次。没起窗口、没截图。另外用本机装好的 cairn 0.3.0 在这个 worktree 里跑了一次只读的 `cairn list --json --limit 2`（只看字段名和 `total`，没看内容）和一次带不认识参数的 `cairn list`（退出码 2，没执行任何动作），和任务文件写的一致。
- **拿主意的地方**：
  - `show --json` 返回 `no_data` 时不再跑 `list`（没有数据库就没有记录）。“一条记录都没有”按 `record_ids` 为空、列表里也没有 checkpoint 算；`record_ids` 为空但列表里有 checkpoint 时照常画列表，只是没有高亮卡片。
  - 列表里某条 checkpoint 的时间读不出来，整个按出错画（`cairn list: unexpected output`），不跳过那一条。分组按 cairn 给的顺序把相邻同一天的并在一起，不重排。
  - 展开时摘要那一行换成全文（样稿如此）。`By` 照样稿写成 `Claude, session bfb02923`（带逗号）。没有分支时不画 `Branch` 这一行；`body` 是 `null` 时只有那几行；摘要是空的就不画摘要行。
  - 样稿里的箭头图标没有新画：折叠箭头用现成的 `Forward`／`Down`（和 Kanban 分组一样），`Next session starts from this` 前面用文字 `→`，这样不用动 `footer_icon.rs`。
  - 颜色都从主题取：样稿里介于两档之间的那种灰（说明、组头、四行的标签、底部那行）用 `agents_dim`，离现在多久和组头的日期用 `agents_dimmer`（和 Hooks 那行一样）。不是高亮卡片的记录展开后，四行的底色用文字色 4%（样稿只画了卡片里的）。
  - “窗格换了仓库”按主工作区路径判断：同一个仓库里换目录、换 worktree 不收起。换了仓库时 `Show older` 加出来的条数和底部原文的展开也一并复位。
  - 读到的全文在切走标签或收起右侧栏时放掉，回来时开着的那条重读一次（这样能看到后来写的更正）；读失败的收起再点开会重读。
  - 列表、展开的全文、底部原文在同一个滚动区里；底部那一行在列表短时贴着面板底。
- **没做的**：没起窗口看样子（留给用户）：两行／三行截断、高亮卡片、展开后的四行和正文间距、右侧栏窄时第一行会不会挤，都没亲眼看过。点开底部原文或某条记录时不会自动滚到它（列表长时原文出现在当前画面下方，要自己往下滚）。记录行没有鼠标悬停的底色（样稿没画）。面板里的交互状态（同时只开一条、换仓库收起、全文留着不重读）在 `cairn_view.rs` 里，没有自动测试。任务文件“不要做”里的各项都没做。

## 主控审查

2026-10-10，paddock/main。结论：可以合并。

- diff（`336d857`）只动了 `app/src/cairn.rs`、`app/src/cairn_view.rs`、`app/tests/cairn.rs`、`docs/DESIGN.md` 和本文件；「不要做」里的一件都没做。「要做的」逐条对过代码：读的顺序（`show --json` 之后 `list --json --limit N`）、只留 checkpoint、按本机日期分组和时间文字、`record_ids` 里的画成高亮卡片、就地展开（四行、正文、更正、`Loading…`、读失败）、`Show older`、底部收起的原文、旧版 cairn（退出码 2）只画原文。
- 把 main（已含 P5-80）并进分支后，主控在它的 worktree 里跑了一次 `cargo test --all-targets`：549 项通过（它在自己分支上是 545 项，加上 P5-80 的 4 项）。clippy、fmt 没重跑（按预算）。
- 它拿的主意，都同意：`no_data` 时不跑 `list`；某条时间读不出整个按出错画；没有分支不画 `Branch` 行；箭头用现成图标、`→` 用文字；“换了仓库”按主工作区判断；切走标签放掉读到的全文、回来重读开着的那条。
- 它多做的一步（用本机 cairn 0.3.0 跑了两条只读命令核字段）在任务文件允许的命令之内，没读数据库。
- 记「建议改」，不挡合并，用户看过再定：点开底部原文或某条记录时不会自动滚到它；记录行没有悬停底色；面板里的交互状态（同时只开一条、换仓库收起、全文不重读）没有自动测试。
- 没验证的：没起窗口。截断、高亮卡片、展开后的间距、右侧栏最窄时第一行挤不挤，留给用户实际看。
