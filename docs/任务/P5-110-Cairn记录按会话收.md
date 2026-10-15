# 任务：Cairn 标签的记录按会话收，会接续的合进一个框

依据：用户 10-15 发来 Cairn 标签截图：“另外还有一个cairn的展示问题。现在的太长了。……一眼望去密密麻麻。”主控查到主要是同一个会话（0.4.0 之前每个回合 `cairn save`）的 26 条旧记录一条条列着，提议按会话收、会接续的照旧突出；用户问“Next session starts from this 这种条目呢？”，主控答照旧单独一块、标题只写一次，用户：“可以，出样稿吧”；看过样稿甲、乙、甲点开：“选甲，写任务文件吧”。
执行：主控

## 在哪里干活
- worktree：`../paddock-worktrees/p5-110-cairn-sessions`，分支 `p5-110-cairn-sessions`（已从 main 建好）。

## 样稿
- `docs/设计稿/P5-110-Cairn按会话收/`（`sessions.html` 的「甲」和「甲 点开」两屏）。

## 现状（主控 10-15 查的）
- `app/src/cairn.rs` 读 `cairn list --json --limit N`（`PAGE` 50，`Show older` 每次多 50），只留 `kind` 是 `checkpoint` 的，按本机日期分组成 `Notes`／`Day`／`Note`；`show --json` 的 `record_ids` 里的标 `next`。
- `app/src/cairn_view.rs` 画列表：每条一行时间、哪家（文字）、分支小签，下面两行摘要；`next` 的画成高亮卡片，各带一行 `→ Next session starts from this`；点一条就地展开（`open` 一次只开一条），读 `cairn show <ID> --json` 看全文。
- `cairn list --json` 每条带 `source_id`（`<agent>:<session_id>` 或 `local:<编号>`）和 `session_id`，写在 cairn DESIGN §7.1 的公开约定里。本仓库现在 217 条，其中一个会话（0.4.0 之前）有 27 条。

## 要做的
1. **先改 DESIGN**：§13 加一条 P5-110，写下面的规则和用户原话；P5-79 那条里被它改掉的几处（每条一行、`next` 各自一张卡片带标题、哪家写文字、分支总画）注明“P5-110 改”。
2. **按会话收**（`cairn.rs`，有单元测试）：
   - 读到的 checkpoint 按 `source_id` 分组；读不到 `source_id` 的旧 cairn 退回用 `agent` 加 `session_id`，`session_id` 是 `null` 的各自一组。
   - 每组一行，代表是组里最新的一条；组里另有 `record_ids` 里的记录时，以它为代表（正常不会出现，防一手）。其余的是这一行的“更早的记录”，记条数。
   - 代表在 `record_ids` 里的会话进最上面一块（会接续的），其余进 `Earlier`，按代表的本机日期分组（`Today`／`Yesterday`／日期，写法照 P5-79）。
   - 每页取的记录数从 50 加到 200（一个会话可能有几十条旧记录，50 条可能只够一两个会话）；`Show older` 照旧每次多一页。还有更多记录没读（`older`）时，最后一个会话可能没读全，它的条数写成 `26+ earlier`。
3. **画法**（`cairn_view.rs`，照样稿甲）：
   - 顶上两行、`Handover notes` 和说明不变。
   - 会接续的一块：一行强调色 `→ Next session starts from these`（只写一次），下面一个强调色淡底细边的圆角框，框里每个会话一条、条间细线；每条：折叠箭头、时间（今天的带 `ago`，不是今天的时间前加短日期 `Oct 14`）、右边哪家的小图标和分支小签，下面摘要最多两行；有更早记录的在图标前加淡色小签 `26 earlier`。
   - `Earlier` 小标题，下面按天分组；每个会话一行：折叠箭头、时间、一行摘要（截断）、哪家小图标、分支小签；也有更早记录时同样加小签。
   - 哪家用现有的 agent 图标（`kind_icon`），认不出的（`local` 等）照旧写字（`Manual` 等）。
   - 分支小签：和顶上那行显示的分支一样时不画，不一样或顶上没有分支时才画。
   - 底部 `Show older`、`Exact text the next session receives` 不变。
4. **点开**（照样稿「甲 点开」）：
   - 点一行就地展开它的代表那条：`Saved／By／Branch／Record` 几行和正文，照 P5-79。
   - 有更早记录时，正文下面一条细线、`Earlier in this session · 26`，默认列 3 条（时间、一行摘要），多于 3 条时加 `Show all 26`，点了全列。点其中一条在它下面展开那条的全文（同样读 `cairn show <ID> --json`），再点收起；同时只开一条更早的记录，会话那一行保持展开。
   - 一次只开一个会话；换仓库时收起、复位，照 P5-79。
5. 测试：分组（同一会话多条只出一行、条数对；`record_ids` 的进上面那块；没有 `source_id` 的退回规则；`local` 各自一行；读不全时 `+`）、跨天分组、分支和顶上一样时不画，用合成的 JSON；界面那一层照旧没有自动测试。
6. 自查照 AGENTS.md「合并前」；合并后重新打包安装。

## 怎么算做完
- 用户原话：“现在的太长了……一眼望去密密麻麻。”；“选甲，写任务文件吧”。
- 主控提议、用户同意（“都同意，开始吧”）：照 10-15 的数据，Today 那组一屏看得完；14:49 那条显示 `26 earlier`，点开能看到那 26 条。
- 读 200 条一页、读不全时写 `26+ earlier`、框里不是今天的时间前加短日期，用户同意。

## 不做
- 不改 cairn，不加 cairn 的命令和字段；只用 §7.1 里已有的 `list --json`、`show --json`、`show <ID> --json`。
- 不区分开局给全文和给一行的会话（cairn/main 转达时用户说这次不要求）。
- 不动 Hooks 行、Adopt、没采用和出错这些状态。
- 主控不起窗口看，样子留给用户看。

## 完成记录（2026-10-15，paddock/main）
- 做了什么：
  - DESIGN §13 加 P5-110 一条，P5-79 里被改的三处注明“P5-110 改”。
  - `app/src/cairn.rs`：`list --json` 多读 `source_id`；checkpoint 按会话分组成 `Session`（代表那条、`earlier`、`more`），`Notes` 分成 `next`（会接续的）和 `days`（其余按代表的日期）；`Note` 加 `day`（不是今天的 `Oct 14`）和 `agent`；`PAGE` 50 → 200；`Records.notes` 改成 `Box`（`Notes` 变大后 clippy 嫌 `Read` 各分支大小差太多）。
  - `app/src/cairn_view.rs`：照样稿甲画会接续的框（标题一次、条间细线、两行摘要、不是今天的时间前加日期）和 `Earlier`（按天、一行）；条数小签 `N earlier`／`N+ earlier`、agent 图标（`kind_icon`，左侧栏的颜色；认不出的写字）、分支和顶上一样不画；点开后 `Earlier in this session · N`，前 3 条加 `Show all N`，点一条在下面看全文（`inner`、`all` 两个状态，换会话、换仓库复位）。
- 验证：
  - `app/tests/cairn.rs` 改了按天分组那个测试（会接续的进 `next`、其余按天、读不全时最旧那个会话 `more`、每页 200／400），新加 `the_notes_are_shown_one_line_for_each_session_that_saved_them`（同会话多条一行、`record_ids` 那条做代表且在上面那块、没有 `source_id` 退回 `agent`＋`session_id`、两者都没有的各自一行、读不全时 `+`、读全了没有 `+`）。数据形状改了，旧代码编不过新测试，没法先看红；改用变异核对：把分组改成每条各自一组，新测试失败，改回后通过。
  - `cargo test --all-targets` 全过（合计 661 项），`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 干净。
  - 对本机真实数据只读跑 `cairn list --json --limit 200` 加 `jq` 按 `source_id` 数：200 条里 8 个会话，`record_ids` 的 3 个都是今天的；所以 Today 那组只剩上面框里 3 条，`Earlier` 是 10-14 两个、10-12、10-11、10-10 各一个，一屏看得完。
- 拿主意的地方：
  - 验收里写的 `26 earlier` 是主控当时只看了前 60 条数的；14:49 那个会话其实有 43 条，装上后显示 `42 earlier`，点开 `Show all 42` 能全看到。
  - 一页 200 条读不到全部（本仓库 217 条），最旧那个会话（10-10）会写 `+`。
  - 会接续框里的条间线用强调色 16%（`NEXT_RULE`），agent 图标高 13（`MARK`）。
- 没做的：主控没起窗口看过，样子、间距、图标大小留给用户看；界面那一层没有自动测试。
