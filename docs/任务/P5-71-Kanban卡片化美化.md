# 任务：Kanban 重画成卡片，精细、优雅（照样稿 A）

2026-10-09，paddock/main 交给 paddock/dev-kanbancards（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档拿不准，倾向常规 0.57；交叉审查不要；影响面拿不准，主控定改行为：布局和悬停、确认这些交互的位置都变）
类型：样式／文案调整
依据：
- 用户 10-09（给了现状截图）：“kanban这边的界面能否再美化一下。要精细，要优雅。”主控列了现状的问题并出 A 卡片、B 精修列表两个方向，用户：“A”。
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- 样稿 `docs/设计稿/P5-71-Kanban美化/README.md`，以及 `Main.dc.html`（现状）、`A-Cards.dc.html`（采用）。B 不做。样稿颜色是近似值，一律从主题取色。
- `docs/DESIGN.md` §13 的 P5-29 整条（含 29a～29f：只读看板、窄面板分组、加宽五列、空组、TO REVIEW 显示主控、DONE 全部、Needs you、Clear）、P5-60a（优先级、格式警告、改档）、P5-60b（任务弹框、Edit）、P5-69（Done by、优先级色标）、P5-51／51b（左侧栏卡片的浮起样式，照着用）。
- `app/src/kanban_view.rs` 全部；`app/src/kanban.rs` 的 `Card`、`Board`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-71-kanban-cards`，分支 `p5-71-kanban-cards`（已从 main 建好）。
- 只动 `app/src/kanban_view.rs`、`app/src/kanban.rs`（只在显示需要的字段上）、`docs/DESIGN.md` §13，和本文件末尾的完成记录。
- 编译目录：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-71-kanban-cards`（已备好，增量编译）。

## 要做的（照样稿 A；窄面板为主，加宽五列用同一套卡片）
1. **卡片**：每件活一张卡片，样式照左侧栏 agent 卡片（P5-51／51b）：淡面、细边、顶边亮线、底边暗线；卡片之间约 6pt。
2. **卡片内**：编号放进小标签（等宽小字、淡底）；标题最多两行，再长用 “…”（全看板统一用 “…”，不再出现 “..”）；标记（Draft、High、Low、Waits for、Dropped 等）统一成胶囊，排在标题下一行；时间在编号那一行右侧。
3. **优先级色标**（P5-69）照旧在卡片左边缘；**草稿**用虚线边框（GPUI 不支持虚线就用更淡的实线加更淡的面，写进完成记录）。
4. **在做、待审**：卡片底部一道细线隔出 agent 那一行（头像带状态点、名字、状态、分支、增删、时间）；待审再一行主控（P5-29d）。用户原话要保留：“里面的每一项任务关联着agent的那个特别好，千万别删掉那个展示（包括可以跳转的三个按钮）”。
5. **悬停按钮**：三个按钮（打开任务文件、跳到 agent、看改动）和 Clear、Set priority、Edit 照现有规则出现，收在卡片右上角一个小底托里；就地确认（Clear、改档）照现有做法出现在卡片里。
6. **等你**：卡片一圈黄边，卡片里一块淡黄底写出要你做什么（`待用户：` 的内容，最多两行，长了 “…”），不再顶到面板边被切。
7. **格式警告**（P5-60a）照旧，放在编号那一行。
8. **分组标题**：折叠箭头、色点、名称、件数胶囊；空分组只留一行淡标题、不带箭头（统一，不再有的朝下有的朝右）；Done 的件数写 “3 of 130”。
9. **Done**：普通条目是一行紧凑行（编号、标题、时间），等你的、Dropped 的用卡片；“Show all 130” 做成按钮样子，点了照旧展开／收回。
10. **顶部**：“1 need you” 改 “1 needs you”（2 以上 “needs” → “need”，照英文单复数）；Needs you 改成淡黄胶囊；“+” 做成圆按钮。仓库名、分支照旧。
11. 所有现有功能一项不丢：点卡片、三个悬停按钮、Clear 和它的确认、Set priority、Edit／Open file、New task、折叠分组、加宽五列、空仓库新建、DRAFT 卡、Dropped、Waits for、Needs you 计数、`paddock ctl` 认的“用户在回答问题”（confirming）。
12. **DESIGN §13** 加 P5-71 一条。

## 怎么算做完
- 用户原话：“要精细，要优雅”“A”。
- 测试：已有的看板测试照过；新加的文字规则（“…” 截断、needs/need 单复数、Done 的 “of”）先写测试、确认失败再实现。
- 验证只做这些：上面的测试；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 各跑一次。觉得不够，在回复里说，不要自己加。
- 样子留给用户看。

## 不要做
- **不要起 paddock 窗口**（`cargo run`、截图、实测都不要）：10-09 出过事故，在 agent 会话里起的测试窗口会和所有 agent 归进同一个进程组，从 Dock 上退出它把全部 agent 一起结束了（见 `docs/任务/P5-52-Dock里第二个paddock进程.md`）。
- **不用 Python**（命令、脚本、测试都不用，`python3 --version` 也不要跑）；命令里不用 `rm`；前台命令别带会等标准输入的东西。
- 不做 B；不加拖动；不改看板的列怎么推、读什么、写什么（只改样子）；不碰左侧栏、Changes、Browser。
- 不加新依赖。
- 不要按项目名或路径批量杀进程；不要对 `corral ls` 里的 agent 做 stop、send、keys。
- 遇到要改上面“只动”以外的文件、或要改已定的看板规矩（本任务没提到的），停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-71-kanban-cards` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- **做了什么**：`kanban_view.rs` 把窄面板的行和加宽后的方块换成同一套卡片（`card`）：照左侧栏磨砂卡片的面、细边、顶边亮线、底边暗线（不加外投影），卡片间 6pt；第一行编号小标签、格式警告、时间；标题最多两行 “…”；Draft、High、Low、Waits for、Dropped 统一成胶囊排在标题下；优先级色标照旧；草稿虚线边（GPUI 支持 `border_dashed`）加更淡的面；在做、待审、已合并的卡片底部细线隔出 agent 一行，待审再一行主控；悬停按钮收进右上角小底托；等你的卡片琥珀边加卡片内淡琥珀块（粗体 Needs you＋`待用户：` 内容，一段字、两行 “…”）；Done 普通条目是紧凑一行，等你的、Dropped 的是卡片，Show all 是胶囊按钮；分组标题带件数胶囊，空分组一行淡标题、不带箭头；顶部 “N needs you／need you” 淡琥珀胶囊、圆形 “+”、main 放进等宽小胶囊。`kanban.rs` 只改了 `Board::count`（“3 of 130”）。DESIGN §13 加了 P5-71 一条。
- **验证了什么**：先写测试并看着失败——`one_needs_you_and_more_need_you`、`a_name_and_branch_too_long_together_are_cut_with_an_ellipsis`（“…” 截断，不出现 `..`）、`kanban.rs` 已有测试里的 “6 of 7”，三条都因目标缺陷失败后再实现。`app/` 下 `cargo test --all-targets`（全过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 各跑一次，都过。没起窗口，样子留给用户看。
- **拿主意的地方**：
  - `..` 的来源：GPUI 的省略号在和别的东西共用一行时算错（左侧栏代码里写过同样的结论，自己量宽度截）。所以和别的字挤在一行的（Done 行标题、agent 名、分支、状态）由 paddock 量宽度、用 `card::elide` 截；单独占行的标题和等你那段交给 GPUI 的 `line_clamp(2)`。挤的时候分支先让（留到四成），再让名字，最后截状态。
  - 时间：平时在编号那一行右边；悬停时那里让给按钮底托，有 agent 一行的时间移到那一行末尾（照样稿 A 悬停的那张），没有的悬停时不显示（和原来一样）。
  - 只因 agent 等用户而标的（没有 `待用户：`），淡琥珀块里只写 “Needs you”。
  - 空分组不带箭头，所以也点不了折叠（没东西可折）；悬停说明照旧。
  - 草稿不再整张变淡（原来 0.55），改由虚线边、更淡的面、稍淡的标题区分。
  - 加宽五列的按钮底托同样在右上角，列太窄时按钮折行、靠右。
  - `app/tests/kanban.rs` 不在“只动”的文件里，但它断言旧的 “5 / 7”；这正是本任务要改的写法，只把这一个字面值改成 “5 of 7”，别的没动。
- **没做的事**：没起 paddock 窗口、没截图；B 没做；没加拖动；列怎么推、读什么、写什么没动；左侧栏、Changes、Browser 没碰。卡片在真窗口里的高度、悬停底托和标题、两行截断的实际效果没亲眼看过，留给用户看。
