# 任务：左侧栏 workspace 精修：Idle 能标星、按最近用过排、能过滤，⌘P 能搜 workspace，整体画法做漂亮

2026-11-24，paddock/main 交给 paddock/dev-ws-polish（Claude Code，`opus[1m]` / xhigh）。
路由：重 / 交叉审查不要 / 影响面：改行为（路由：重（置信 0.72）、拿不准、拿不准；交叉审查和影响面主控定：`workspaces.json` 只加两个可选字段、旧文件照读，不动成员和归属规则，不算碰要害；用户定过派出的活由主控审、不另开审查者）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
依据：用户 11-24 发来四张左侧栏截图，嫌闲着的 workspace 一多就找不到要开的那个，要搜索和星标，其余精修交给主控设计。主控画了一版功能（A），用户：“都按照你的建议来”，又要“再做的漂亮一些”；主控画了二版画法（B）并提了三处要改以前定下的地方，用户：“三处都按你的来，写任务文件吧”。改 P5-89／P5-91／P5-128／P5-134／P5-164 的画法，以及 P5-134 的排序；成员、归属规则不动。
执行：派出
（理由：面广但边界清楚：`workspaces.rs` 加两个存盘字段和排序、`agents.rs` 记最近用过、`card.rs` 出行、`sidebar_workspaces.rs`／`sidebar.rs` 画、`search.rs` 加一组，样稿齐全，规则在本文件写死，适合派出后由主控审。路由判重、新界面，按「高／界面和设计」读表：Claude Code `opus[1m]` xhigh。）
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- “左侧边栏的workspace的各种情况能否再精细设计一下。我现在的碰到的问题主要是idle的workspace一多，我就不好找我要打开的了，我觉得应该有搜索以及加一个星星那种功能。还有其他的你觉得精细调整要有的都可以设计。我给出的4复截图是各种情况下的workspace展示。”
- 看过一版样稿和五个问题（四样都要吗、没标星的按最近用过排还是按名字、收起时露出标星的、Idle 要不要限高、星标颜色）：“都按照你的建议来，关于workspace的相关的ui能不能再做的漂亮一些？”
- 看过二版样稿和三处改动（字母签自动配色、展开收起箭头挪到标题最右、收起的 Idle 用胶囊）：“三处都按你的来，写任务文件吧”

主控的建议（即用户同意的）：星标、最近排序、Idle 里过滤、⌘P 搜 workspace 四样都做；没标星的按最近用过排；Idle 收起时标星的仍露着；Idle 不限高；星标用中性色（不用黄、琥珀，琥珀在 paddock 是“要你处理”）；字母签按名字自动配色，不让用户选、不存盘；箭头挪到标题最右；收起的 Idle 里标星的画成胶囊。

## 先读
- 样稿：`docs/设计稿/P5-201-workspace侧栏精修/README.md`。功能照 `A-*`，画法照 `B-*`，冲突时照 B。README 里写了不采用的两处。
- `docs/DESIGN.md` §13 的 P5-89、P5-91、P5-93、P5-105、P5-128、P5-134、P5-164 几条（现在的规则和画法从哪来）。
- 代码：
  - `app/src/workspaces.rs`：`Workspace`、`Workspaces`（存盘、排序、改名）。
  - `app/src/agents.rs`：`Panel` 的 `raise`／`lower`／`opened`、`Folds`、`edit_workspaces`、每次读到 corral 后怎么算哪个 workspace 有 agent。
  - `app/src/card.rs`：`Line`（`Idle`、`IdleWorkspace`、`Vacancies`、`Vacant`）、`Tray`、`Heading`、`listed`、`short_time`。
  - `app/src/sidebar_workspaces.rs`：`tray`、`well`、`shelf`、`idle_shelf`、`idle_workspace`、`vacancies`、`vacant`、`options_button`、`stack_mark`。
  - `app/src/sidebar.rs`：仓库小标题怎么画、`Idle` 托盘接在列表末尾、`No agents yet` 时怎么摆、键盘上下移动。
  - `app/src/search.rs` 和命令面板的画法：现在的分组（Agents、Tabs、Settings、Commands）和 ↵ 怎么分派。
  - `app/src/theme.rs`：终端 16 色、`toward` 和对比度下限那套（约 750–970 行）。
  - `app/src/text_input.rs`：现成的输入框。
  - `app/tests/workspaces.rs`：现在怎么验这些行。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-201-workspace-polish`，分支 `p5-201-workspace-polish`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-201-workspace-polish`（从 `.target/main` 克隆的）。
- 动 `app/src/` 下上面列的文件、相关测试、`docs/DESIGN.md` 和本文件。不加依赖，不加主题颜色键。

## 要做的（颜色一律从主题取，样稿色值只是示意）

### 一、存盘：星标和上次在用
1. `workspaces.json` 每项多两个字段：`starred`（布尔，`false` 时不写）和 `used`（上次在用的时刻，Unix 秒，没有时不写）。旧文件照读（都当没有）；写出时其余字段照旧。改名带着走，删掉一起去掉。改这两个字段也走 `edit_workspaces` 那套（先重读文件再改）。
2. 什么时候记 `used`（记成当时的时刻，只在这些时候写文件，不每次轮询都写）：
   - 读到 corral 后，一个 workspace 从“没 agent”变成“有 agent”，或从“有 agent”变成“没 agent”；以上一次读到的为准，paddock 刚开时第一次读到不算变化，不记。
   - 从 Idle、⌘P 点开放上来时；新建时。
3. 星标：Idle 行上和 `⋯` 菜单里改（见下），在 workspace 标题的 `⋯` 菜单里也加同样一项。菜单项文字 `Star`／`Unstar`，放在 `Rename` 上面。

### 二、Idle 托盘
4. **排序**：标了星的在前，按名字排；其余按 `used` 从近到远，没有 `used` 的排最后、按名字排。有标星的时，两段前面各一个小号大写标签 `STARRED`、`RECENT`（9.5 左右、加粗、字距略开、`agents_dimmer` 再淡一点）；一个都没标星时不写标签，直接是列表。
5. **标题**：空心层叠图标、`Idle`、个数、小箭头（跟在个数后面，展开朝下、收起朝右），右边一个放大镜按钮（提示 `Filter idle workspaces`，浅底小方块）。点标题其余地方收起、展开，照旧记在布局里。
6. **每行两行字**（行高约 38，跟着界面字号缩放）：
   - 最前面是这个 workspace 的字母签（见第七节），22 左右。
   - 上一行名字（12.5、半粗）；标了星的名字后面一个小的实心 ★（中性色，`agents_dim` 一档）。
   - 下一行它的仓库名（取法同现在），按名字用 ` · ` 连起来，10.5、`agents_dimmer`；放不下时写到放得下为止，后面 ` · +N`；没有仓库写 `Empty`。
   - 右边上次在用多久以前，用 `card::short_time`（10.5、更淡）；没有 `used` 时不写。
   - 鼠标停上去：整行淡底变亮，名字和仓库名各亮一档；名字后面出空心 ☆（标了星的就是实心那颗），点它标星／去星；右边时间换成两个按钮：`+`（强调色淡底，提示 `New Agent in <名字>`，做的事同 `⋯` 菜单的 `New Agent…`，P5-93 那套）和 `⋯`（菜单照旧，加上 `Star`／`Unstar`）。菜单开着时这行保持亮着。点行其余地方照旧放上来。
7. **收起时**：标题下面，标了星的 workspace 画成一排胶囊（自动折行）：圆形字母签（20 左右）加名字，淡底细边圆角胶囊；鼠标停上去变亮，提示写它的仓库名，点了放上来。没有标星的就只剩标题。
8. **过滤**（点放大镜，或 Idle 托盘里任何一行有键盘焦点时直接打字——后者做不到就只做按钮，写进完成记录）：
   - 标题那一条换成输入框：放大镜、输入框（占位 `Filter idle workspaces`）、`<命中数> of <总数>`、清除按钮（提示 `Clear filter`）。框带强调色描边和一圈很淡的强调色外光。收着时点放大镜先展开，过滤结束后回到原来收没收着。
   - 按空白分词，每个词都要在名字或任一仓库名里出现（不分大小写）。命中的字用强调色。结果照第 4 条的顺序，但不写 `STARRED`／`RECENT` 标签。
   - 第一条默认选中（强调色淡底，行尾一个 `↵` 小签）；↑↓ 换选中；↵ 放上来并结束过滤；⌘↵ 在它里面 New Agent（同 `+`）并结束过滤；Esc 或清除按钮清空并收回成标题。
   - 一个都不中时写 `No idle workspace matches “<字>”`。
   - 底部居中一行快捷键：`↑↓ select  ↵ open  ⌘↵ new agent  esc clear`，键名亮一档。
   - 过滤的字只在内存，不存盘。
9. Idle 不限高（一版 `… 3 more` 不做）。

### 三、⌘P 搜 workspace
10. 命令面板打了字时，在 `Agents` 后面多一组 `Workspaces`（带个数）：每个 workspace 一行，名字和它的任一仓库名命中（规则同面板现在的匹配）就列出。标题是名字（标了星的后面小 ★），右边淡字写仓库名 ` · ` 连起来，再 ` · Idle` 或 ` · N agents`。没打字时不列这一组。
11. 对它按 ↵：闲着的放上来（同点 Idle 行，记 `used`），有 agent 的展开它（收着的话）；两种都把左侧栏滚到它那里。面板的 `⌘↵` 和 “New Agent in …” 命令不做。

### 四、字母签和配色
12. 字母：名字按 `-`、`_`、`.`、空白拆开，取前两段的首字（大写）；只有一段时取第一个字（大写）。中文等也照取第一个字。
13. 颜色：从主题的终端色里取 7 个色调（蓝、品红、青、绿、红、亮蓝、亮品红；不用黄，黄和琥珀在 paddock 表示要处理），按名字的稳定哈希（不分大小写）选一个。签底是这个色调叠在侧栏底色上约 20%（浅色侧栏约 13%），字是这个色调；字和签底的对比度不够时（用 `theme.rs` 现成的对比度那套，下限 4.5），把字往 `agents_text` 拉到够为止。这样每个主题自动有一套，不加主题键。改名后颜色可能变，这是预期的。
14. 用在：Idle 行、Idle 收起的胶囊、展开的 workspace 标题、放上来的空 workspace 标题。收起的 workspace 标题前照旧是 agent 的叠卡（P5-91），不换成签。

### 五、上面在用的 workspace 托盘
15. **标题行**（展开时）：字母签、名字（13.5、半粗）、标了星的小 ★、空、`N agents`（`1 agent`）、展开收起的箭头挪到最右（朝下）。鼠标停上去 `⋯` 出在 `N agents` 前面。点标题其余地方收起，照旧。
16. **收起时**：叠卡、名字、★、空、状态细条、个数、最右箭头朝右；一摞的底照旧（P5-91）。
17. **淡光**：托盘（展开和收起都算）左上角一层这个 workspace 签色的淡光，往右下淡出（深色约 10%，浅色约 8%）；托盘顶边里侧一条 1px 的很淡的亮线。GPUI 没有径向渐变就用斜向的线性渐变，再不行就去掉淡光只留亮线，写进完成记录。
18. **仓库小标题**（workspace 里和不在 workspace 里的都改）：名字后面的个数改成小签（10.5、半粗、`agents_dimmer`，文字色约 7% 的底、圆角 5）；个数后面一条 1px 细线一直连到右边的状态细条。收起的仓库小标题前的叠卡（P5-128）、强调色小圈、按钮和菜单照旧。
19. **没 agent 的仓库合成的那一行**（P5-164）：收着时 `cairn · ranch  No agents`，后面细线，最右小箭头朝右，整行淡（0.8 透明，悬停变亮）。展开后 `No agents <个数>`、细线、最右箭头朝下，下面的仓库行照 P5-164。

### 六、点开放上来的空 workspace（P5-134 的 opened）
20. 托盘：边线虚线（`border_dashed`），底色比在用的淡一档；标题：字母签（淡一档）、名字、★、空、`Put back` 按钮（往下放进托盘的图标加字，淡底小按钮）、`⋯`。点 `Put back` 同现在点标题（放回 Idle，不记成收着）；点标题其余地方也照旧放回。
21. 托盘里面：居中两行 `Nothing running here`（12.5 半粗）和 `Start an agent in one of its repositories.`（11、淡）；下面每个仓库一颗胶囊按钮 `+ <仓库名>`（强调色淡底、强调色细边，悬停加深），点了同现在仓库行的 `+`（P5-93，目录预填这个仓库）。没有仓库时第二行写 `Add a repository or start an agent.`，下面一颗 `+ New Agent`（同现在的 `New Agent` 按钮）。

### 七、不变的
22. 谁算闲着、点开放上来、有 agent 回到上面、哪些收着记在布局里，照 P5-134；成员和归属照 P5-105；`No agents yet` 和 Idle 托盘的摆法照现在；左侧栏收成窄条时照现在（Idle 不出现，字母签和淡光也不进窄条）；上下移动照旧只走 agent；远程视图本来没有 workspace，不变。
23. `docs/DESIGN.md` §13 末尾补一条 P5-201 写清以上和实际做法；P5-89、P5-91、P5-134、P5-164 里被改掉的画法、排序处各注一句“P5-201 改了，见下面 P5-201”。P5-89、P5-134 里“不给 workspace 配颜色”那句注明 P5-201 改成按名字自动配色（用户 11-24 同意）。

## 怎么算做完
- 用户原话：“idle的workspace一多，我就不好找我要打开的了，我觉得应该有搜索以及加一个星星那种功能”“再做的漂亮一些”“三处都按你的来”。上面「要做的」逐条做到，样子照 `B-*`。
- 用户装上后实际用：能标星、标星的在前且收起时还在；没标星的最近用过的在前；Idle 里能过滤、⌘P 能搜到 workspace 并打开；整体比现在好看。
- 验证只做这些：
  - `app/tests/workspaces.rs`（或 `workspaces.rs` 的单测）覆盖：`starred`、`used` 读写（不写默认值、旧文件照读、改名跟着、删掉去掉）；字母签的取字（多段、一段、中文、空白）；配色选择稳定且不分大小写；Idle 的排序（标星按名字、其余按 `used`、没有 `used` 的按名字在最后）；`used` 只在有无 agent 变化、放上来、新建时记，第一次读到不记；过滤的匹配（多个词、仓库名命中、不分大小写、不中）；收起时只出标星的胶囊行。
  - `search.rs` 的单测覆盖 `Workspaces` 一组：没打字不出、按名字或仓库名命中、行上写 `Idle`／`N agents`。
  - 原有 P5-134、P5-164 的规则测试照旧全过（只改期望里因数据形状变了的地方，不放宽规则）。
  - `git diff --check`，`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。外观留给用户看，你不起窗口、不截图。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 workspace 的成员和归属规则、P5-134 谁算闲着的规则、布局文件里已有的字段；`workspaces.json` 只加 `starred`、`used` 两个字段。
- 不改卡片本身、分屏标签的叠卡、看板、收件箱、Activity、右侧栏、`paddock ctl`、View 菜单；不给 workspace 加手选颜色或图标、不做拖动排序、不做 Idle 限高、不做命令面板的 `⌘↵`。
- 不加主题颜色键，不加依赖，不改 `gpui-pre-*` 版本。
- 不改、不读用户真实的 `~/.config/paddock/`、`~/.local/state/paddock/` 下的配置和状态文件（包括真实的 `workspaces.json`），测试一律用临时目录和合成数据。
- 不碰 `corral ls` 里的 agent，不对任何 agent `corral send`；不起 paddock 窗口。
- 不用 Python；命令里不用 `rm`、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西；不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 碰到要拿主意的事（比如某个主题下签色对比度怎么拉都不够、GPUI 做不出某种画法、过滤框的键盘焦点和侧栏现有按键冲突），停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-201-workspace-polish` 上提交。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具或环境的行为和预想不一样的；没有就写「没有」），在你的分支里提交。主控审过后装上给用户看，用户说可以再合并。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 做了什么
- 存盘（`workspaces.rs`）：`Workspace` 多 `starred`（false 不写）、`used`（Unix 秒，没有不写）；`Workspaces::star`、`mark_used`；改名带着走、删掉一起去掉（字段在同一项里）。另有 `initials`（字母签取字）、`tint`（名字小写的 FNV-1a 选 7 色之一）、`matches`（过滤：每个词在名字或某个仓库名里，不分大小写）、`hits`（命中处，给画强调色）。
- 记 `used`（`agents.rs`）：`Panel::turned` 算出哪些 workspace 从没 agent 变有、或从有变没，记着上一次；`Panel::mark_used` 走 `edit_workspaces`（先重读文件）；`Panel::bring_up` = 放上来 + 记 `used`，Idle 行、胶囊、过滤 ↵、⌘P 和新建都走它。`sidebar.rs` 的 `poll` 每次读到 corral 或 Git 后调 `turned` 并记。远程视图不记。
- 行（`card.rs`）：`Idler`、`idle`（标星的按名字 → 其余按 `used` 近到远、同刻按名字 → 没有 `used` 的按名字）、`holds`（仓库名 ` · ` 连，放不下写到放得下再 ` · +N`，没有写 `Empty`）；`Line::IdleWorkspace` 带 `Idler`；`Tray` 多 `starred`；收起的 Idle 只出标星的行（胶囊），过滤时（`Panel::filtering`）当作展开。
- ⌘P（`search.rs`、`window.rs`）：打了字（不只是空白）时 `Agents` 后面多 `Workspaces` 一组，按名字或仓库名匹配，说明写 `<仓库> · … · Idle`／`N agents`，标星的标题后小 ★；↵ 调 `Sidebar::reveal_workspace`：闲着的放上来，有 agent 且收着的展开，再把列表滚到它的托盘（列表加了 `ScrollHandle`）。
- 签色（`theme.rs`）：`Theme::monogram`：终端色 4、5、6、2、1、12、13，签底比例 20%／浅色 13%，字对比度不到 4.5 时往 `agents_text` 二分拉到够。
- 画法（`sidebar_workspaces.rs`、`sidebar.rs`）：照任务第二、五、六节——Idle 托盘标题（层叠图标、`Idle`、个数、箭头、放大镜）、`STARRED`／`RECENT`、两行的 Idle 行（签、名字、★、仓库、时间；悬停 ☆、`+`、`⋯`）、收起的胶囊、过滤框（输入框、`<命中> of <总数>`、清除、强调色边和外光、选中行 `↵`、↑↓／↵／⌘↵／Esc、不中的话、底部快捷键）；在用托盘的标题（签、13.5 名字、★、`N agents`、最右箭头）和收起的一摞（叠卡、★、细条、个数、最右箭头）、淡光和亮线；仓库小标题的个数小签和细线；合成行（` · `、细线、最右箭头、0.8 透明）；放上来的空托盘（虚线框、淡签、`Put back`、`⋯`、`Nothing running here`、仓库胶囊／`+ New Agent`）；`⋯` 菜单 `Rename` 上面加 `Star`／`Unstar`。
- `docs/DESIGN.md`：§13 末尾加 P5-201；P5-89、P5-91、P5-134、P5-164 被改的画法、排序处各注一句，P5-89、P5-134 的“不配颜色”注明改成按名字自动配色（用户 11-24 同意）。

### 验证了什么
- `app/tests/workspaces.rs` 新增 7 个：`starred`／`used` 读写（不写默认值、旧文件照读、改名跟着、删掉去掉、取消星标不留字段）；字母签取字（多段、一段、中文、空白、连续分隔符、空名）；配色稳定、不分大小写、几个名字的色号钉死、分散到多种色；Idle 排序；`used` 只在有无 agent 变化（等 Git 读到才算、第一次不算、加一个 agent 不算）、放上来、新建时记，记的时候先重读文件（别的窗口改的星标留着），改名后照新名字算；过滤匹配（多个词、仓库名命中、不分大小写、不中）和命中位置、`holds` 截法；收起时只出标星的行，过滤时展开、结束后照原样。
- `search.rs` 单测 1 个：没打字或只有空白不出这一组、排在 `Agents` 后、按名字或仓库名命中（多词、不分大小写）、不按说明字匹配、行上写 `Idle`／`2 agents`／`Empty · Idle`、星标、个数。
- `theme.rs` 单测 1 个：每个预设、每种签色，签字对签底对比度都够 4.5。
- 原有 P5-134、P5-164 等测试全过；只改了数据形状（`Workspace` 字面量加 `..Workspace::default()`、`Line::IdleWorkspace` 换成带 `Idler`、`search::everything` 多一个参数），期望没改。
- `git diff --check` 无输出；`app/` 下 `cargo test --all-targets`（全过）、`cargo clippy --all-targets -- -D warnings`（无告警）、`cargo fmt --check`（通过）。
- 没起窗口、没截图，外观留给用户看。

### 拿主意的地方
- “paddock 刚开时第一次读到不算”：要等 Git 把每个 agent 的目录都读到（agent 按仓库归 workspace，没读到前按名字前缀暂分，会误判），第一次读全时只当基线；另外只在 corral 成功读过一次后才算（第一次读失败不当基线）。
- 放上来和新建记 `used` 做成 `Panel::bring_up`，好在不起窗口的测试里验证。
- 过滤时展开靠内存里的 `Panel::filtering`，不改布局里记的 `idle`，所以结束后自然回到原样。
- 过滤框里 ↑↓、↵、Esc 用现成的 `menu::DIALOG` 绑定；⌘↵ 没有绑定，用输入框外层的 `on_key_down` 接，没改 `menu.rs`。
- 淡光：GPUI 没有径向渐变，用 135° 线性渐变（签色深色 10%／浅色 8%，到 60% 处淡没），垫在托盘最底下；亮线用 1px 内阴影（白色，深色 5%、浅色 35%）。放上来的空托盘也画淡光，不画亮线。
- 签：22（胶囊里 20，圆形），圆角 7，字 10 加粗；标题上的签内侧一圈签色 25% 的细环，放上来的空托盘上整体淡到 0.7。⌘P 行前用强调色层叠图标。
- Idle 行的仓库名按界面字体量宽（扣掉签、间距和时间）截到放得下；悬停时时间换成更宽的两个按钮，这时由 GPUI 自己的省略号截。
- 星标的 ☆ 加了悬停提示 `Star`／`Unstar`（任务没写提示文字）。
- 仓库小标题悬停才出的箭头留在个数小签后面，细线从它后面开始连到细条。
- ⌘P 的说明：没有仓库写 `Empty · Idle`；只打空白不算打了字。
- 放上来的空托盘：`card.rs` 照旧出每个仓库一行（老测试不动），`sidebar.rs` 画的时候把这些行收拢成 `Nothing running here` 那块。

### 没做的事
- “Idle 托盘里任何一行有键盘焦点时直接打字”：行本来不接键盘焦点，没做，只有放大镜一个入口。
- `STARRED`／`RECENT` 的字距：GPUI 没有字距设置，没拉开。
- 外观没看过（不起窗口、不截图），留给用户。

### 踩过的坑
- macOS 的 sed 不认 `\b`，第一次批量改名没改全；换成 `[[:<:]]`／`[[:>:]]`。
- 签色的类型起名 `Tile` 和 `sidebar.rs` 里已有的 `Tile`、`TILE` 撞名，改叫 `Monogram`（主题）和 `Sign`（侧栏）。
- clippy 的 `single_range_in_vec_init` 不让 `[6..10]` 这种单个区间的数组当期望，测试里改成比较 `(start, end)`。

## 主控审查

2026-11-24，paddock/main。
- 看了 diff：只动了任务列的 `workspaces.rs`、`agents.rs`、`card.rs`、`sidebar_workspaces.rs`、`sidebar.rs`、`search.rs`、`theme.rs`，加上 `window.rs`（命令面板分派和画 ★，属于 ⌘P 那一节）和测试、DESIGN；没加依赖、没加主题键（签色是 `theme.rs` 里的常量，从终端色取），`workspaces.json` 只多 `starred`、`used`，成员和归属规则没动。“不要做”里的都没做。
- 重跑 `app/` 下 `cargo test --all-targets`：1123 个全过，和 agent 说的一致。
- 取舍：刚开时等 Git 读全再起算、第一次不记——同意（否则按前缀暂分组会误记）；淡光用斜向线性渐变、亮线用内阴影——同意；过滤框外层接 ⌘↵ 不改 `menu.rs`——同意；“行有焦点时直接打字”没做、`STARRED`／`RECENT` 没拉字距（GPUI 不支持）——同意，任务里本来写了做不到就只做按钮。
- 已从分支提交 `620e98b` 打包装到 `~/Applications/paddock.app`，等用户重开看。
