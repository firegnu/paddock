# 任务：Activity 叠在 agent 列表上、半透明，后面的卡片隐约透出（照样稿方案 1）

2026-10-09，paddock/main 交给 paddock/dev-activity（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档拿不准，倾向常规 0.75；交叉审查不要；影响面拿不准，主控定改行为：列表和面板的布局、点击、滚动都变）
类型：功能变更
依据：
- 用户 10-09（给了侧栏截图）：“注意下被那个提交记录的卡片挡住的。我想让这个提交记录的卡片投下去，能模模糊糊的看到被挡住的agent。”
- 主控说明 GPUI 没有模糊身后内容的功能，出样稿三张；用户问推荐，主控推荐方案 1（渐隐加半透明，便宜稳定；方案 3 假模糊每帧多画几遍、要和滚动逐帧对齐）；用户：“好，按方案1做”。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- 样稿 `docs/设计稿/P5-70-Activity毛玻璃/README.md`、`Main.dc.html`（现状）、`Fade.dc.html`（采用）。`Ghost.dc.html` 不做。
- `docs/DESIGN.md` §13 的 P5-32（活动格子图，含折叠）、P5-34、P5-51、P5-51b 条。
- `app/src/sidebar.rs`：侧栏的 `render`（列表 `list`、`self.activity`、`footer` 现在是上下排的 flex 列）、列表怎么滚；`app/src/activity_view.rs`：面板 `render`（`c.ground`、`c.edge`、折叠）。

## 现状（主控核对）
- 列表和 Activity 是同一个 flex 列里上下排的：列表在自己的底边被裁掉，并不在 Activity 底下，所以截图里那张卡片是被“切掉”，不是被“挡住”。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-70-activity-overlay`，分支 `p5-70-activity-overlay`（已从 main 建好）。
- 只动 `app/src/sidebar.rs`、`app/src/activity_view.rs`、`docs/DESIGN.md` §13，和本文件末尾的完成记录。
- 编译目录：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-70-activity-overlay`（已备好，增量编译）。

## 要做的
1. **列表延伸到 Activity 下面**：列表的可视区域一直到侧栏底部的 footer 上面，Activity 叠在列表底部之上（位置和现在一样）；列表内容底部多留出 Activity 的高度（加上和现在一样的间距），最后一张卡片能完整滚到 Activity 上方。Activity 折叠／展开、界面字号变化时，留出的高度跟着变。
2. **Activity 半透明**：面板底色约 75%～80% 不透明（照主题取色，磨砂和全屏都成立），后面滚过去的卡片隐约透出；格子、文字照旧清楚。
3. **渐变上沿**：Activity 上边缘往上一段（约 24～32pt），用 Activity 自己的底色从透明过渡到面板的不透明度，看起来卡片慢慢没入面板（GPUI 没有遮罩，用渐变色块做）。这段只是画，不挡鼠标：上沿区域里的卡片照样能点、能悬停。
4. **鼠标**：点在 Activity 面板上的不会穿到下面的卡片；面板自己的悬停卡片、折叠、点击照旧。列表滚动（触控板、滚轮）在 Activity 上方照旧，在 Activity 上滚动不影响列表（或照现在的行为，二选一写进完成记录）。
5. 列表很短、没滚到 Activity 下面时，样子和现在一样（不会凭空出现渐变遮住卡片——渐变只在下面真有内容时才有意义，但画出来也不能挡住内容的可读性；拿主意写进完成记录）。
6. **DESIGN §13** 加 P5-70 一条。

## 怎么算做完
- 用户原话：“能模模糊糊的看到被挡住的agent”“按方案1做”。
- 测试（先写、确认失败再实现）：列表底部留出的高度等于 Activity 的高度加间距（折叠、展开、两种字号）；最后一张卡片能滚到 Activity 上方（用现有的布局计算函数测，不测画面）。
- 验证只做这些：上面的测试；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 各跑一次。觉得不够，在回复里说，不要自己加。
- 样子留给用户看。

## 不要做
- **不要起 paddock 窗口**（`cargo run`、截图、实测都不要）：10-09 出过事故，在 agent 会话里起的测试窗口会和所有 agent 归进同一个进程组，从 Dock 上退出它把全部 agent 一起结束了（见 `docs/任务/P5-52-Dock里第二个paddock进程.md`）。
- **不用 Python**（命令、脚本、测试都不用）；前台命令别带会等标准输入的东西。
- 不做方案 3（错开的淡影）；不改卡片本身的样子（P5-51／51b）、窄条、footer。
- 不加新依赖。
- 不要按项目名或路径批量杀进程；不要对 `corral ls` 里的 agent 做 stop、send、keys。
- 遇到要改上面“只动”以外的文件、或要改已定的设计，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-70-activity-overlay` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- **做了什么**：
  - `sidebar.rs`：列表外面套一层（`flex_1`、`relative`），列表照旧在里面滚、一直到 footer 上面；Activity 放在同一层里的绝对定位块，贴底，叠在列表末尾之上。列表底部内边距从 10pt 改成 `list_end`＝10pt＋4pt（界面字号缩放，原来面板的上边距）＋面板高度。列表加 `ScrollHandle`，每次画时按“还能往下滚多少”算渐变浓淡（`fade`），通过 `Frame.fade` 交给面板；画完下一帧若滚动位置或最远处变了（比如列表变长）再画一次。Activity 折叠（点标题或按布局恢复）时侧栏也重画，留出的高度跟着变。
  - `activity_view.rs`：新增 `height(width, ui, folded)`，由各行算出面板高度；标题行、脚注行改成固定高度（字号×1.3 取整，和 GPUI 自己的行高一样），面板自己设 1.3 行高，所以算出的就是画出的。面板去掉上边距（间距算进列表末尾）、加 `occlude()`；底色改成侧栏底色稍往文字色靠，78% 不透明。`render` 外面包一层：`fade > 0` 时面板上面多一条 28pt 的渐变（透明 → 面板底色×浓淡），这条不带任何事件，不挡鼠标。`sizes`／`inner` 挪成按宽度算的自由函数，方法照旧调用它们。
  - `DESIGN.md` §13 加 P5-70 一条。
- **验证了什么**：先写三条测试并确认按目标失败（留出的高度：`left: 10.0, right: 197.94`；渐变：`left: 0.0, right: 0.5`；最后一张卡片那条在面板高度还是 0 的桩上会通过，补上真实高度后按断言失败），再实现到通过：
  - `the_list_ends_the_activity_panel_and_its_gap_below_its_last_card`：13pt、16pt 两种字号 × 折叠、展开，`list_end` ＝ 面板高度＋10＋4（缩放）；折叠比展开矮，字号大了两种都变高。
  - `the_last_card_scrolls_clear_above_the_activity_panel`：卡片比侧栏高、滚到底时，最后一张卡片底边在面板上沿之上，至少隔 10pt。
  - `the_panel_fades_into_the_list_only_where_the_list_goes_on_under_it`：列表短或滚到底时为 0，离底半个渐变高为 0.5，滚到顶为 1。
  - `app/` 下 `cargo test --all-targets`（全过）、`cargo clippy --all-targets -- -D warnings`（无警告）、`cargo fmt --check`、`git diff --check` 各跑过。没起 paddock 窗口，样子留给用户看。
- **拿主意的地方**：
  - 第 4 条二选一：在 Activity 上滚动不滚列表（和原来一样）。面板用 `occlude()`，挡住点击、悬停，也挡住滚动。
  - 第 5 条：渐变的浓淡跟着“列表在面板下面还有多少”走，列表短、没滚到面板下，或已经滚到底（最后一张卡片在面板上方）时不画；离底越远越浓，离底 28pt 起画满。这样短列表和滚到底时和原来一样，不会有一层色压在最后一张卡片上。
  - 面板 78% 不透明（任务给的 75%～80% 取中间），渐变 28pt（24～32pt 取中间），都随界面字号缩放。磨砂下面板底色也用侧栏底色（原来是文字色 4% 左右，几乎透明），所以磨砂时面板会比原来实一些、比周围侧栏深一些，这是“78% 不透明”在磨砂下的直接结果，用户看实物时留意。
  - 渐变宽度和面板一样（左右各 10pt），底边是直的，面板上沿是 12pt 圆角：渐变最浓的一截会填进面板两个上角外面的那点空，看起来面板上沿和渐变连成一片，圆角只剩边线。只在下面有卡片时才画，列表短或到底时面板照旧是圆角。
  - 面板高度用算的，不量画出来的：标题行、脚注行定死高度保证两者一致，测试能直接测。
- **没做的事**：方案 3、卡片、窄条、footer 都没动；没加新依赖；没起窗口、没截图。中途我用一条空命令误建了一个空文件 `paddock-worktrees/scratch_unused`，已用 `mv` 挪进会话临时目录，仓库里没有。
