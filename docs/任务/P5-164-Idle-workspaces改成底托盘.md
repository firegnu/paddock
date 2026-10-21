# 任务：左侧栏底下的 Idle workspaces 改成一个淡托盘，合成行展开后不再重复仓库名

2026-10-21，paddock/main 交给 paddock/dev-idle-shelf（omp，默认模型，不套墙）。
依据：用户 10-21 嫌左侧栏底下 `Idle workspaces` 的样子有问题，主控画了甲（收紧成清单）、乙（休眠托盘）、丙（底托盘）三种，推荐丙；另外指出有 agent 的 workspace 里合成行展开后仓库名重复、没缩进。用户：“选丙，重复那行也一起改，写任务文件吧”。本轮只改画法，P5-134 定的规则（谁算闲着、点开放上来、收起放回去、记住哪些收着）一条不动。
执行：派出
（理由：用户点名交给 omp。不套墙：套 fence `dev` 墙的 omp 起来十几秒就停（已交 ranch 查），P5-159、P5-160、P5-163 都是 omp 不套墙做的。主控审，装上给用户看。）
路由：常规 / 交叉审查不要 / 影响面：看得见（路由：常规（置信 0.86）、不要、拿不准；影响面主控定：只改左侧栏的画法和画法要用的行数据，规则和存盘不动，做错了一眼看得出）
类型：样式／文案调整
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- “这个sidebar的 Idle workspace能否重新设计一下？我觉得这个功能对我帮助很大，但是总感觉这个ui的设计有点问题。”
- 看了样稿：“选丙，重复那行也一起改，写任务文件吧”

主控指出的问题（用户据此选的丙）：右边 `2 repos` 后面给 `⋯` 留着空位，跟上面卡片的右边对不上；闲着的 workspace 名字比小组标题和上面托盘里的字还亮，轻重颠倒；上面都装在托盘里，这块却是散在背景上的一行行，前面各一个灰图标，像文件列表；`2 repos` 想不起这个 workspace 是干嘛的。

## 先读
- 样稿：`docs/设计稿/P5-164-IdleWorkspaces底托盘/README.md`，`C-Shelf.dc.html` 是要的样子，`Main.dc.html` 是现在的样子。
- `docs/任务/P5-134-空workspace收到底部.md`（规则的来历）和 `docs/DESIGN.md` §13 的 P5-91、P5-129a、P5-134 几条。
- 代码：
  - `app/src/card.rs`：`Line`（`Vacant`、`Vacancies`、`Idle`、`IdleWorkspace`）和 `listed`（约 740–985 行）。
  - `app/src/sidebar_workspaces.rs`：`tray`（展开的托盘怎么画、标题怎么画）、`well`、`idle_heading`、`idle_workspace`、`vacancies`、`vacant`、`options_button`，以及 `IDLE_*` 那几个常量。
  - `app/src/sidebar.rs` 约 2580–2760 行：哪些行装进托盘、`Idle` 那几行怎么接在列表末尾、`No agents yet` 时怎么摆。
  - `app/tests/workspaces.rs`：现在怎么验这些行。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-164-idle-shelf`，分支 `p5-164-idle-shelf`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-164-idle-shelf`（从 `.target/main` 克隆的）。
- 动 `app/src/card.rs`、`app/src/sidebar_workspaces.rs`、`app/src/sidebar.rs`、相关测试、`docs/DESIGN.md` 和本文件。不加依赖。

## 要做的（照样稿丙；颜色一律从主题取，样稿色值只是示意）

1. **闲着的 workspace 装进一个淡托盘**
   - 去掉现在的淡分隔线；列表末尾放一个托盘，外框（圆角、内边距、边线粗细）照展开的 workspace 托盘，底色和边线比在用的托盘淡一档，一眼看得出它是“放着不用的”（样稿里在用的是黑 38%、边 16%，这个是黑 14%、边 9%，只是示意，开发从主题和 `well` 那套里取）。它和上面的距离同两个 workspace 托盘之间的距离。
   - 托盘标题照 workspace 标题的样子和高度：箭头、空心层叠图标（淡色，不用强调色）、`Idle`（字号同 workspace 名，字色淡一档）、个数。点标题收起、展开（就是现在的 `fold_idle`，记住的方式不变）。收起时只剩标题这一条，还在淡托盘里，不画成一摞。
   - 界面上的字从 `Idle workspaces` 改成 `Idle`。
2. **托盘里每个闲着的 workspace 一行**
   - 不要前面的图标。名字左边对齐托盘里仓库小标题（`paddock` 那行）的字，字号、字重、颜色同仓库小标题。
   - 右边是它的仓库名（取仓库目录名，同合成行的取法），按名字排、用 `, ` 连起来，淡字，靠右；放不下时先省略仓库名，再省略 workspace 名。没有仓库的写 `Empty`。不再写 `N repos`。
   - 行高大约 26pt（跟着界面字号缩放）。
   - 鼠标停上去：整行淡底变亮，名字和仓库名各亮一档，行尾出 `⋯`（提示照旧 `Workspace options`），`⋯` 占用的是仓库名让出来的地方；平时不给 `⋯` 留空位，仓库名一直贴到右边。菜单打开时这行保持亮着。菜单内容、`Rename` 就地改名、点行放上来，都照现在。
   - 按名字排，照现在。
3. **有 agent 的 workspace 里，合成行展开后不重复**
   - 收着时照现在：`▸ cairn, ranch  No agents`。
   - 展开后，这一行变成 `▾ No agents 2`（`No agents` 的字号、字重同现在仓库名那段，后面的个数同仓库小标题的个数），不再列仓库名。
   - 下面每个仓库一行，只写仓库名，不再带 `No agents`；缩进到和箭头后面的字对齐；鼠标停上去右边 `+`，照现在。
   - 只有一个没 agent 的仓库（不合成）、点开放上来的空 workspace 里一个仓库一行，这两种照现在 `<仓库> No agents`，不变。
4. **不变的**：谁算闲着、点开放上来、点标题放回去、有 agent 回到上面、哪些收着记在布局里，全照 P5-134；没有任何 agent 时 `No agents yet` 和这个托盘的摆法照现在（托盘在列表末尾）；窄条里这些都不出现；上下移动照旧只走 agent；远程视图不变。
5. `docs/DESIGN.md` §13 P5-134 那条后面补一条 P5-164，写清以上；P5-134 那条里描述画法（细线、`Idle workspaces` 标题、`N repos`）的地方注一句“P5-164 改成底托盘”。

## 怎么算做完
- 用户原话：“选丙，重复那行也一起改”。上面「要做的」逐条做到，样子照 `C-Shelf.dc.html`。
- 用户装上后实际看：底下的 Idle 和上面的托盘是一套画法，又明显淡一档；行尾对齐；合成行展开后不重复。
- 验证只做这些：`app/tests/workspaces.rs` 里跟着改测试，覆盖闲着的那行带出按名字排好的仓库名（含没有仓库的）、合成行展开后下面各行和不合成的单行能分得出来；原有 P5-134 的规则测试照旧全过（只改期望里因数据形状变了的地方，不放宽规则）。`git diff --check`，`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。外观留给用户看，你不起窗口、不截图。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 P5-134 的规则和布局文件里存的字段，不改 `workspaces.json` 的格式和成员规则。
- 不改在用的 workspace 托盘、卡片、收起的叠卡、仓库小标题的画法；不改 View 菜单、命令面板、看板、收件箱。
- 不加新的主题颜色键，不加依赖，不改 `gpui-pre-*` 版本。
- 不改、不读用户真实的 `~/.config/paddock/`、`~/.local/state/paddock/` 下的配置和状态文件，测试一律用临时目录和合成数据。
- 不碰 `corral ls` 里的 agent，不对任何 agent `corral send`；不起 paddock 窗口。
- 不用 Python；命令里不用 `rm`、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西；不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 碰到要拿主意的事（比如淡托盘在浅色主题下分不出层次、要加主题键才行），停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-164-idle-shelf` 上提交。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具或环境的行为和预想不一样的；没有就写「没有」），在你的分支里提交。主控审过后装上给用户看，用户说可以再合并。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 做了什么

- `app/src/card.rs`：`Line::IdleWorkspace` 的 `repos` 从个数改成按名字排好的仓库目录名（`kanban_repos::name`，排序去重）；`Line::Vacant` 加 `grouped`，把合成行展开后的行和单独一行的 `<仓库> No agents` 分开。`Line::lines` 的注释改成 `Idle` 托盘。
- `app/src/sidebar_workspaces.rs`：
  - `idle_heading` 换成 `idle_shelf`：标题和各闲着的 workspace 行进同一个淡托盘。外框圆角、内边距、边线粗细照展开的 workspace 托盘；面色用新增的 `shelf`（黑遮罩 14%／4%，对在用的 26%／7%），边用 `agents_text` 0.09（对在用的 0.17），没加主题键。和上面的距离同两个 workspace 托盘之间（列表间距 + 4，第一个 8）。
  - 托盘标题照 workspace 标题的样子和高度（30）：箭头、空心层叠图标（`agents_dimmer`，不用强调色）、`Idle`（13、`agents_dim`）、个数（11、`agents_dimmer`）；点标题仍走 `fold_idle`。收起时只剩标题，仍在托盘里，不画成一摞。
  - `idle_workspace`：去掉图标；名字 11.5 半粗 `agents_dim`，左对齐托盘里仓库小标题的字；右边仓库名 11 `agents_dimmer`、靠右、按名字 `, ` 连起来，没有仓库写 `Empty`；行高 26（`ui.px` 缩放）。悬停整行淡底变亮，名字亮到 `agents_text`、仓库名亮到 `agents_dim`；`options_button` 加 `compact`：平时宽 0 不占位（仓库名贴到右边），悬停才展开 22pt 露出 `⋯`，菜单开着时保持亮；菜单和就地改名照旧。
  - `vacancies`：展开时标题变成 `▾ No agents <个数>`（`No agents` 11.5 半粗 `agents_dimmer`，个数 11、更深一档）。`vacant` 加 `grouped`：合成行下面只写仓库名，左内边距 19（4 + 箭头 9 + 间距 6），不带 `No agents`，悬停 `+` 照旧；不合成的单行和空 workspace 的行不变。
- `app/src/sidebar.rs`：列表末尾把 `Line::Idle` 和随后所有 `Line::IdleWorkspace` 收进 `idle_shelf` 一个托盘；窄条仍忽略这些行，上下移动、`No agents yet` 的摆法、远程视图都没动。
- `app/tests/workspaces.rs`：辅助格式改成 `Idle <个数>`、`~ <名字> <仓库名…|Empty>`、合成行展开 `v No agents <个数>` 且下面只有仓库名；相应期望更新（含没有仓库的、多个仓库按名字排的）。
- `docs/DESIGN.md`：P5-134 里描述细线、`Idle workspaces` 标题、`N repos` 的画法处注了「P5-164 改成底托盘」；P5-134 后新增 P5-164 一条。

### 验证了什么

- `app/tests/workspaces.rs` 跟着改完，覆盖：闲着的那行带出按名字排好的仓库名（`zeta` 的 `a, b, c`）和没有仓库的 `Empty`；合成行展开后是 `v No agents 3` 加三行只有仓库名的行，不合成的单行仍是 `<仓库> No agents`；原有 P5-134 的规则测试照旧全过。
- `git diff --check` 通过。
- `app/` 下 `cargo test --all-targets` 全部通过。
- `app/` 下 `cargo clippy --all-targets -- -D warnings` 无警告。
- `app/` 下 `cargo fmt --check` 通过。
- 按任务没起窗口、没截图；外观留给用户看。

### 拿主意的地方

- 淡托盘的色值：沿用 `well` 的取法新增 `SHELF = [0.14, 0.04]`、`SHELF_EDGE = 0.09`，比在用的 `WELL = [0.26, 0.07]`、`WELL_EDGE = 0.17` 淡一档；没有新增主题键。
- 托盘里标题和各行之间、各行之间用 2pt 间距（照样稿），外框和内边距照 workspace 托盘。
- `⋯` 不占位用宽 0 + `group_hover` 展开到 22pt，而不是绝对定位：绝对定位时看不见的按钮仍会接点击。
- 合成行下面的缩进直接写成 4 + 9 + 6，和标题箭头后的字对齐；个数和 `No agents` 的颜色分别用 `dimmer.opacity(0.6)`、`agents_dimmer`，与仓库小标题一致。

### 没做的事

- 没有改 P5-134 的规则、布局文件里的字段、`workspaces.json` 的格式和成员规则。
- 没有改在用的 workspace 托盘、卡片、叠卡、仓库小标题、View 菜单、命令面板、看板、收件箱。
- 没有加主题颜色键、依赖，没改 `gpui-pre-*` 版本。
- 没有合并到 main，没有推送。

### 踩过的坑

没有。

### 返工 1（主控审查）

- 主控审查指出：`vacant` 里 `grouped` 的左内边距写死成 `px(4.0 + CHEVRON + 6.0)`，而 `vacancies` 那行的箭头是 `ui.px(CHEVRON)`、间距是 `ui.px(6.0)`，会跟界面字号缩放；字号不是默认时下面的仓库名对不上箭头后面的字。改成 `px(4.0) + ui.px(CHEVRON + 6.0)`（左内边距的 4 和上面那行一样不缩放，箭头和间距跟着缩放）；`docs/DESIGN.md` 里 P5-164 那句同步写清「4 + 箭头 9 + 间距 6，箭头和间距跟界面字号缩放」。
- 返工后重跑：`cargo test --test workspaces`（19 个全过）、`cargo clippy --all-targets -- -D warnings`（无警告）、`cargo fmt --check`（通过）、`git diff --check`（通过）。别的不动。

## 主控审查

2026-10-21，paddock/main。
- 看了 diff：只动了 `card.rs`（`IdleWorkspace` 带仓库名、`Vacant` 加 `grouped`）、`sidebar_workspaces.rs`、`sidebar.rs`、`agents.rs`（只改注释）、`app/tests/workspaces.rs`、DESIGN 和本文件；P5-134 的规则、布局字段、`workspaces.json` 没动，没加主题键和依赖，“不要做”里的都没做。淡托盘的外框、和上面的距离同 workspace 托盘（`TRAY_*`、头一个 8 其余 4）。
- 取舍：淡托盘色值照 `well` 的取法另立 `SHELF`——同意；`⋯` 用宽 0、悬停展开而不是绝对定位（免得看不见的按钮接点击）——同意。
- 返工 1 次：合成行展开后各行的左内边距写成固定 19pt，而上面那行的箭头和间距随界面字号缩放，字号不是默认时对不上（「要做的」第 3 条）；改成 `px(4.0) + ui.px(CHEVRON + 6.0)`，复核通过。
- 影响面“看得见”，没重跑全套；从分支编了 release 装上（`~/Applications/paddock.app`），样子留给用户看。
结果：返工 1 次
