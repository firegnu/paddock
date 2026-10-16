# 任务：Cairn 的 Notes | Ledger | Lessons 选中的胶囊改淡，和左侧栏机器切换栏一致

2026-10-17，paddock/main 交给 paddock/dev-cairn-pill（omp，默认）。
路由：轻 / 交叉审查不要 / 影响面：看得见（主控定，没问路由：用户点名派 omp，只改一处画法）
类型：样式／文案调整
依据：用户 10-17：“右侧sidebar的cairn下的notes ledger还有lessons三个胶囊的颜色也是实心的，和刚才的不一致，改为一致吧。”“你直接派出omp吧。”“刚才的”是主控 10-17 刚改的左侧栏机器切换栏（提交 `90b7da6`）。
执行：派出
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`（规矩全文）。
- `docs/DESIGN.md` 最后一条「凹槽里选中的胶囊改淡（10-17…）」。
- `git show 90b7da6 -- app/src/sidebar.rs app/src/sunk.rs`：左侧栏改成什么样。
- 代码：`app/src/sunk.rs`（凹槽、实心胶囊、计数小签）、`app/src/cairn_view.rs` 的 `fn switch`、`app/src/sidebar.rs` 的 `fn switcher` 和常量 `SHOWN_FILL`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-127-cairn-pill`，分支 `p5-127-cairn-pill`（已从 main 建好）。
- 编译：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-127-cairn-pill`（已从 `.target/main` 克隆）。cargo 命令在 `app/` 下跑。
- 只动 `app/src/sunk.rs`、`app/src/cairn_view.rs`、`app/src/sidebar.rs` 和本文件。

## 要做的
1. 把左侧栏选中胶囊的画法（淡强调色底 `SHOWN_FILL`、`crate::window::TAB_RING` 宽的边：选中的 `RING_ACTIVE` 强调色、没选中的透明；选中的亮色字加粗）抽成 `sunk.rs` 里一处，左侧栏和 Cairn 切换都用它，两处看起来一样。
2. Cairn 切换选中的那颗照这个画：底、边、字（亮色用 `c.title`）都同左侧栏；里面的计数小签和没选中的一样（`sunk::count` 按没选中画）。没选中的照旧，悬停照旧。
3. 改完没人用的东西删掉：`sunk::capsule` 和它的常量；`sunk::count` 的 `on` 参数如果两处都只传 `false`，连同只给 `on` 用的常量一起去掉。`sunk.rs` 开头的说明和 `cairn_view.rs` `fn switch` 上面的说明改成现在的样子。

## 怎么算做完
- 用户原话：“右侧sidebar的cairn下的notes ledger还有lessons三个胶囊的颜色也是实心的，和刚才的不一致，改为一致吧。”
- 验证只做这些：`git diff --check`；`app/` 下 `cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`cargo test --lib` 各跑一次。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改凹槽本身、没选中的胶囊、右侧栏标签、别的界面。
- 不起 paddock 窗口、不截图（样子留给用户看）；不碰 `corral ls` 里的任何 agent。
- 不用 Python；命令里不用 `rm`；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 不加依赖；不合并到 main，不推送。只在 `p5-127-cairn-pill` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

2026-10-17，omp（paddock/dev-cairn-pill）。

### 做了什么
- `app/src/sunk.rs`：选中胶囊的画法收成一处的 `pub fn chosen(pill: Stateful<Div>, on: bool, accent: Hsla, text: Hsla, bright: Hsla) -> Stateful<Div>`：`rounded_full()`、`border(px(TAB_RING))`；选中时 `bg(accent.opacity(SHOWN_FILL))`、`border_color(accent.opacity(RING_ACTIVE))`、`font_weight(SEMIBOLD)`、`text_color(bright)`；没选中时透明边、`text_color(text)`、悬停转 `bright`。`SHOWN_FILL = 0.18` 从 `sidebar.rs` 挪进来。删掉没人用的 `sunk::capsule` 和 `PILL_LIGHT`／`PILL_DARK`／`PILL_SHADOW`／`PILL_GLOW`。
- `sunk::count` 去掉 `on`（两处都只传 `false`），只给 `on` 用的 `COUNT_ON` 删除；`ground` 参数只喂 `on` 那条分支，一并去掉，签名成 `count(count: usize, text: Hsla, ui: &UiFont)`。
- `app/src/sidebar.rs`（`fn switcher`）：胶囊改成 `sunk::chosen(div()…, on, accent, text, bright)` 再接着挂 tooltip、点击、名字、状态、计数；自己的 `SHOWN_FILL` 常量删掉；计数小签改传 `text`。
- `app/src/cairn_view.rs`（`fn switch`）：三颗胶囊都走 `sunk::chosen`，选中那颗亮色字用 `c.title`（和左侧栏的 `bright` 是同一个主题值 `t.bright`）；里面的计数小签按没选中画（`sunk::count(count, c.text, ui)`）；`pill_shadows` 删掉。`fn switch` 上面的说明和 `sunk.rs` 开头的说明改成现在这样。

### 验证了什么
- 只做任务点名的那四条（编译目录 `.target/p5-127-cairn-pill`）：`git diff --check` 干净；`cargo clippy --all-targets -- -D warnings` 无警告；`cargo fmt --check` 通过；`cargo test --lib` 573 通过、0 失败。
- 没起 paddock 窗口、没截图（任务写明样子留给用户看），所以两种主题下的淡底和细环的实际观感没实测。觉得不够的话：可以做一次临时 HOME 加假 corral 起的窗口看，按任务没自己加。

### 拿主意的地方
- 共享函数叫 `sunk::chosen`，把 `rounded_full` 和 `border(px(TAB_RING))` 也收进函数，两处调用点相应删掉这两句——渲染结果不变。
- 没选中的边在共享函数里显式设成 `transparent_black()`（照左侧栏原来的写法）。Cairn 未选中的胶囊原来没有边，现在跟着有一条 1px 透明边；GPUI 按 border-box 排版，外框尺寸不变（内容内缩 1px，同左侧栏的胶囊），所以看不出来。
- `sunk::count` 的 `ground` 跟着 `on` 一起去掉，调用点从 `(agents, false, ground, text, &ui)` 变成 `(agents, text, &ui)`，`(count, on, c.ground, c.text, ui)` 变成 `(count, c.text, ui)`。
- Cairn 选中的胶囊内容因为多了 1px 边而内缩 1px，这是“边同左侧栏”的必然结果。
- 提交没写 `Co-Authored-By: Claude …`（历来的提交都有）：这一件是 omp 做的，我不想冒名，也没找到 omp 的署名约定。要不要给 omp 定一个 trailer，请主控定。

### 没做的事
- 不改凹槽、不改没选中的胶囊的外观和悬停、不改右侧栏标签和别的界面。
- 没起窗口、没截图、没碰任何 corral agent；没合并、没推送；没加依赖。

### 踩过的坑
- 没有。
