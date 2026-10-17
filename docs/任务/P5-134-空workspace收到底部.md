# 任务：左侧栏没有 agent 的 workspace 收到列表末尾的 Idle workspaces 小组，有 agent 的 workspace 里没 agent 的仓库合成一行

2026-10-17，paddock/main 交给 paddock/dev-idle-workspaces（Claude Code，opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.3）、拿不准、拿不准；主控按规则定：只改左侧栏的行怎么排、怎么画和布局里存的收起状态，不改 `workspaces.json` 的格式和成员规则，不碰并发和公开约定）
类型：功能变更
依据：用户 10-17：“如果我有一个workspace中有好几个agent，但是我如果把这些agent都关闭的话，界面上就会留一个空的workspace很难看。但是如果选择了关闭wokspace，下一次启动的时候就得重建workspace。”主控给了甲（空了自动收到底部）、乙（手动 Close）、丙（只画小），用户：“甲的话先出一个设计图”；看过样稿（画布「P5-134 空 workspace 收到底部样稿」，源文件在 `docs/设计稿/P5-134-空workspace收到底部/`）：“选甲·二，规则都按你的来，附图也要，写任务文件吧”。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

派出的理由：左侧栏的新交互和画法，按分工给 Claude Code。

## 先读
- `AGENTS.md`（规矩全文）。
- 本文件全部。
- 样稿：`docs/设计稿/P5-134-空workspace收到底部/README.md`，和 `A2-Group`、`A3-Open`、`B-IdleRepos` 三张；`Main` 是现在的样子，`A1-Shelf` 没采用。
- `docs/DESIGN.md` §13 的 P5-89、P5-91、P5-93、P5-105、P5-128、P5-129a 几条（workspace 是什么、托盘画法、从 workspace 里开 agent、只按仓库收、仓库收起的叠卡、字号）。
- 代码：`app/src/card.rs`（`Line`、`listed`、`Tray`、`Heading`）、`app/src/agents.rs`（`Folds`、`Panel`）、`app/src/workspaces.rs`、`app/src/sidebar.rs`（画每种行、上下移动、窄条）、`app/src/sidebar_workspaces.rs`（workspace 标题、`⋯` 菜单、`vacant` 那一行、就地改名）、`app/src/layout_state.rs`（`sidebar_folded`）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-134-idle-workspaces`，分支 `p5-134-idle-workspaces`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-134-idle-workspaces`（已从 `.target/main` 克隆）。
- 动 `app/src/` 下要动的文件、相关测试、`docs/DESIGN.md` 和本文件。不加依赖。

## 要做的（照样稿；颜色从主题取，样稿色值只是示意）

“闲着”：workspace 标题上的 agent 个数是 0（照现有归属规则算，包括按名字前缀跟进来的）。

1. **闲着的 workspace 收到底部**（样稿 `A2-Group`）
   - 闲着的 workspace 不再在列表上面画托盘，统一放进列表最末尾（不在 workspace 里的组之后）的一个小组：上面一条淡分隔线，小组标题 `Idle workspaces <个数>`，样子同仓库小标题（箭头、名字、个数）；点标题收起、展开。没有闲着的 workspace 时整个小组和分隔线都不画。
   - 小组里每个闲着的 workspace 一行：空心层叠图标（淡色，不用强调色）、名字、右边淡字成员数（`Empty`、`1 repo`、`3 repos`，同卡片 `Workspace` 菜单里的写法）。鼠标停上去整行变亮，行尾出 `⋯`（提示 `Workspace options`），菜单和 workspace 标题上的 `⋯` 一样（`New Agent…`、`Rename`、`Add repositories…`、`Remove workspace`），`Rename` 就地在这一行改。
   - 小组里按名字排，同上面 workspace 的排法。
   - 它的任何一个仓库里有了 agent（不管从哪开的），这个 workspace 自动回到上面，照它记着的收起状态画。它最后一个 agent 停掉，它马上回到底下。
   - 没有任何 agent 时，中间照旧是 `No agents yet`，小组在列表末尾照样有。
2. **点开闲着的 workspace**（样稿 `A3-Open`）
   - 点小组里的一行：这个 workspace 回到上面它按名字该在的位置，展开（照 P5-91 的凹槽画法），里面照现在：每个仓库一行 `<仓库> No agents`，鼠标停上去右边 `+`；没有仓库的照现在 `No agents` 加 `New Agent`。
   - 新建的 workspace（不管从哪个入口建）也照这样留在上面、展开。
   - 留在上面的空 workspace，你点它的标题收起时回到底下（不记成“收着”，它记着的收起状态不变）；它里面开了 agent 以后就照常规矩：有 agent 在上面，agent 全停了回到底下。
   - 哪些是“点开留在上面的”只在内存里记，不存盘：paddock 重开后空 workspace 都在底下。改名、删掉 workspace 时跟着改、跟着去掉。
3. **有 agent 的 workspace 里，没 agent 的仓库合成一行**（样稿 `B-IdleRepos`）
   - 有 agent 的 workspace 展开时，没有 agent 的仓库有两个以上，就不再一个仓库一行夹在各组中间，改成放在这个 workspace 最后的一行：箭头 `▸`、仓库名按名字用 `, ` 连起来（过长省略号）、`No agents`，整体淡色（字号同现在 `<仓库> No agents` 那一行：仓库名 11.5、`No agents` 11）。点这一行展开：箭头 `▾`，下面照现在一个仓库一行（`+` 照旧），再点收起。
   - 只有一个没 agent 的仓库时照现在，一行 `<仓库> No agents`，不合成。
   - 第 2 条里留在上面的空 workspace 不合成，照现在一个仓库一行。
4. **记住的东西**：`Idle workspaces` 小组收没收着、哪些 workspace 的合成行展开着，和别的收起状态一样跟布局一起存（布局文件的 `sidebar_folded` 里加字段）；旧布局文件没有这些字段时，小组展开、合成行收着。workspace 改名、删掉时跟着改、跟着去掉（照现有收起状态的做法）。
5. **别的地方**
   - 上下移动照旧只走 agent，跳过这些新行。
   - 左侧栏收成窄条时，闲着的 workspace 不占地方，也不画它的分隔线；合成行在窄条里不出现。
   - 远程视图本来没有 workspace，不变。
6. `docs/DESIGN.md` §13 P5-129a 那条后面补一条 P5-134，写清以上；P5-89 那条里“agent 全停了 workspace 也还在（这时列表里是各个托盘……）”一句后面注一句“P5-134 改成收到列表末尾的 `Idle workspaces`”。

## 怎么算做完
- 用户原话：“选甲·二，规则都按你的来，附图也要”。“规则”指本文件第 1、2、4 条里写的：任何仓库里有 agent 就回到上面；点开的、新建的空 workspace 留在上面直到点收起；小组里按名字排；重开 paddock 后空的都在底下、小组收没收着记住。
- 验证只做这些：针对这件活写测试，覆盖行怎么排（闲着的进小组、按名字排、有 agent 回到上面、点开和新建的留在上面、收起后回到底下、改名删掉时跟着走、合成行两个以上才合、点开后逐行）和收起状态的存读（旧布局文件当默认）。`git diff --check`，`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑过一次。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 `workspaces.json` 的格式和 workspace 的成员、归属规则；不改卡片、托盘、叠卡的画法；不改 View 菜单、命令面板、⌘P、看板、收件箱。
- 不加手动 Close、拖动排序、给 workspace 配颜色这些没要的。
- 不改、不读用户真实的 `~/.config/paddock/`、`~/.local/state/paddock/` 下的配置和状态文件，测试一律用临时目录；不起 paddock 窗口截图（样子留给用户看）。
- 不碰 `corral ls` 里的 agent（都是用户的）；测试用合成数据。
- 不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西；不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 遇到要改 workspace 成员规则或存盘格式才能做的，停下来报告，等决定，不要自己换别的办法。
- 不合并到 main，不推送。只在 `p5-134-idle-workspaces` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的，工具或环境和预想不一样的；没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 做了什么
- `app/src/agents.rs`：`Folds` 加 `idle`（`Idle workspaces` 收着）、`vacant_open`（哪些 workspace 的合成行展开着），改名、删掉时跟着走；默认值时不写进布局文件。`Panel` 加只在内存里的 `opened`（点开的、新建的空 workspace），`raise`（放上来并展开）、`lower`（放回底下，收起状态不动）、`rename_workspace`、`forget_workspace`；每次读到 corral、读到 Git、改名单后，已经有 agent 的从 `opened` 里去掉，这样 agent 全停后它回到底下。
- `app/src/card.rs`：`Line` 加 `Idle`、`IdleWorkspace`、`Vacancies`，`Tray` 加 `opened`。闲着、没点开的 workspace 进列表最末尾的小组（按名字）；点开的照展开画、不管记着的收起；有 agent 的 workspace 里没 agent 的仓库两个以上合成最后一行，展开时后面逐行。
- `app/src/sidebar.rs`、`app/src/sidebar_workspaces.rs`：画小组标题（淡分隔线、同仓库小标题的样子）、小组里的行（层叠图标、名字、`Empty`/`1 repo`/`3 repos`、悬停 `⋯` 和同一个菜单、就地改名）、合成行；点行放上来、点放上来的空 workspace 的标题放回底下；新建 workspace 后放上来；改名、删掉跟着走。没有 agent 时 `No agents yet` 在小组上面剩下的地方居中。窄条里这些行都不占地方。`⋯` 按钮抽成 `options_button`，标题和小组里的行共用。
- `docs/DESIGN.md`：P5-89 那句后注明 P5-134 改了；P5-129a 后加 P5-134 一条。

### 验证了什么
- 新测试：`app/tests/workspaces.rs` 三条（闲着的进小组、按名字排、小组收起、有 agent（含按前缀跟进来的）回到上面并照记着的收起状态画、agent 停了回到底下、没有 agent 时只剩小组；点开和新建的留在上面并展开、点标题回到底下且收起状态不变、开了 agent 后照常、停了回到底下、建的时候就有 agent 的不留、改名删掉跟着走、新 `Panel`（相当于重开）都在底下；合成行两个以上才合、点开逐行、workspace 收起时都不显示、剩一个时照旧夹在组中间），`app/tests/layout_state.rs` 一条（`idle`、`vacant_open` 存读，旧布局文件当小组展开、合成行收着）。
- 原有两条测试的期望按新规矩改了：`workspaces_come_first_…` 里空的 `Zoo` 从托盘变成末尾小组里的一行；`a_folded_workspace_stacks_…` 里空的 `Vacant` 不再有标题。其余原有测试不动，`folded_groups_are_saved_…` 的“原样写回”照旧通过（新字段默认值不写）。
- `git diff --check`；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 都过了。
- 没起窗口截图（任务要求），样子留给用户看。

### 拿主意的地方
- 点开小组里的一行时，它记着的收起状态一并改成展开：不然记着“收着”的 workspace 点开后、里面开了 agent，就会突然收成一摞。点标题放回底下时收起状态照任务不动。
- `Idle workspaces` 标题照现有仓库小标题的画法：名字、个数、个数后面的箭头（鼠标停上去或收着时出现），没照样稿把箭头放在名字前；小组里的行左边和标题的字对齐（样稿也是图标对着标题名字）。
- 合成行照样稿：箭头在前，平时 0.75 透明度，鼠标停上去变亮加淡底。
- 新字段在默认值时不写进布局文件，没有用到时文件和以前一样。
- 新建时带进来的仓库已有 agent 的，不记进 `opened`（它本来就在上面）。
- 没有 agent 时 `No agents yet` 放在小组上面剩下的地方居中；corral 还没读到时小组不画，免得开窗口时先在底下出现、读到后又跳上去。

### 没做的事
- 收起的 workspace 现在一定有 agent，托盘里“收起又没有 agent”时的 `LAYERS_FILLED` 图标实际画不到了，按“不改托盘画法”留着没删。
- 鼠标停在小组标题上的样子、浅色主题、窄条，样稿没画，照现有仓库小标题的做法，没有另外设计。
- 测试是先写实现再写的，没有先跑出红。

### 踩过的坑
没有。

## 主控审查

2026-10-17，paddock/main。
- 看了 diff：只动了 `agents.rs`（`Folds` 加 `idle`、`vacant_open`，`Panel` 加内存里的 `opened`）、`card.rs`（`Line` 加 `Vacancies`、`Idle`、`IdleWorkspace`）、`sidebar.rs`、`sidebar_workspaces.rs`、两份测试和 DESIGN；没改 `workspaces.json` 格式和成员规则，“不要做”里的没做。
- 重跑 `app/` 下 `cargo test --all-targets`：28 组 763 个全过；`clippy -D warnings`、`fmt --check`、`git diff --check` 通过。
- 取舍：点开底下那行时一并把它记着的收起状态改成展开——同意（否则点开后一开 agent 会突然收成一摞）；`Idle workspaces` 标题箭头照仓库小标题放在个数后面、悬停或收着时才出——同意（任务写的是“样子同仓库小标题”）；建时就带着有 agent 的仓库的不记成“点开留在上面”、corral 没读到时不画小组、默认值不写进布局文件——同意。`LAYERS_FILLED` 不再画到但留着，照任务不改托盘画法，记一笔，不删。
- 主控没起窗口看样子：画法、悬停、改名、窄条、浅色主题留给用户实际看。
结果：一次过
