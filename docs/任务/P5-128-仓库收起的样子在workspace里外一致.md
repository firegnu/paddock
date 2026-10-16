# 任务：仓库小标题收起来时，名字前面也画 agent 的叠卡（同收起的 workspace）

2026-10-17，paddock/main 交给 paddock/dev-group-stack（omp，默认）。
路由：轻 / 交叉审查不要 / 影响面：看得见（主控定，没问路由：用户点名派 omp，只改左侧栏一处画法）
类型：功能变更
依据：用户 10-17：“左侧agents一个repo下有多个agents且不在一个workspace中的时候，收缩起来是icon stack，但是在workspace就是完全收起来了。行为不一致。这个任务依然派omp来做”；主控查明仓库小标题收起来从来没有叠卡后，用户发来截图（收起的 workspace `My-Harness` 名字前的叠卡、收起的仓库 `global-mesh` 只有一行）：“我想要上面箭头那种效果，但是那个效果只在workspace收缩的时候。我要的是左侧sidebar只要有收缩agents的行为都要有那种stack的效果。”
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`（规矩全文）。
- `docs/DESIGN.md` §13 的 P5-89（仓库收起）、P5-91（workspace 收起时的叠卡）两条。
- 代码：`app/src/card.rs`（`Heading`、`Tray::stack`、`stacked`、`MOST`，以及造 `Heading`、`Tray` 的地方）、`app/src/sidebar.rs` 的 `fn heading`（仓库小标题）、`app/src/sidebar_workspaces.rs` 里 workspace 标题画叠卡的那段（`tab_stack::Face::of_kind`、`tab_stack::Inks::of`、`tab_stack::stack`）、`app/src/tab_stack.rs`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-128-group-stack`，分支 `p5-128-group-stack`（已从 main 建好）。
- 编译：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-128-group-stack`（已从 `.target/main` 克隆）。cargo 命令在 `app/` 下跑。
- 只动 `app/src/card.rs`、`app/src/sidebar.rs`、`app/src/sidebar_workspaces.rs`（要抽公共代码时）、相关测试、`docs/DESIGN.md` 和本文件。

## 要做的
1. 仓库小标题收起来时，名字前面画这一组 agent 的叠卡，画法、大小、张数和收起的 workspace 标题前那个完全一样：最多三张，最前面一张是活动窗格显示的那个 agent（它在这组里的话），否则是列表里的第一个，后面的按列表先后；只有 1 个 agent 时画一张。workspace 里、外的仓库小标题都这样。
2. 哪几张、谁在前照 `Tray::stack` 的算法在 `card.rs` 里算（给 `Heading` 加同样的字段，共用 `stacked`）；画叠卡的代码和 workspace 标题共用一处，不复制一份。
3. 展开时仓库小标题照旧（没有图标）；箭头、个数、强调色小圈、右边的状态细条、悬停、点击收起展开、按钮和菜单都不变。收起的 workspace 照旧。只加叠卡这个图标，不给仓库加 workspace 那种一摞卡片的底。
4. `docs/DESIGN.md` §13 P5-91 那条后面补一条：仓库收起时也画叠卡（改 P5-91 里“不在 workspace 里的仓库小标题、仓库的收起……都不变”），附用户原话。

## 怎么算做完
- 用户原话：“我想要上面箭头那种效果，但是那个效果只在workspace收缩的时候。我要的是左侧sidebar只要有收缩agents的行为都要有那种stack的效果。”
- 验证只做这些：`card.rs` 里给收起的仓库算叠卡补一条测试（谁在前、最多三张、活动窗格的 agent 不在这组时取第一个）；`git diff --check`；`app/` 下 `cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`cargo test --lib` 各跑一次。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改卡片、workspace 的托盘和一摞底、分屏标签的叠卡样子、左侧栏收成窄条时的画法、别的界面。
- 不起 paddock 窗口、不截图（样子留给用户看）；不碰 `corral ls` 里的任何 agent。
- 不用 Python；命令里不用 `rm`；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 不加依赖；不合并到 main，不推送。只在 `p5-128-group-stack` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 做了什么

- `card.rs`：`Heading` 加 `stack: Vec<String>` 字段（和 `Tray::stack` 同一份算法，共用 `stacked`），`listed` 的 `section` 里给每个仓库组算：`stack: stacked(agents, selected)`。`MOST`、`stacked` 的文档跟着说清两处都用。
- `sidebar_workspaces.rs`：把 workspace 标题里画叠卡那段抽成 `pub(super) fn Sidebar::stack_mark(&self, stack: &[String], ui: &UiFont) -> AnyElement`（照 `stack.iter()` 在 `self.listing.panel.agents` 里找 kind、`tab_stack::Face::of_kind` / `Inks::of` / `stack`，不挂状态点），`tray` 里那处改成调它，一行画法只留一处。
- `sidebar.rs` 的 `fn heading`（仓库小标题）：解构出 `stack`，收起时（`folded`，且这一组有 agent）在标题行最前面、名字前放同一个 `stack_mark`；展开时不画，别的（箭头、个数、强调色小圈、右边状态细条、悬停、点击收起展开、按钮、菜单、`above` 间距算法）一个没动。窄条（rail）走 `unfolded`、不画组标题，天然不变。
- `docs/DESIGN.md`：P5-91 那条「不在 workspace 里的仓库小标题、仓库的收起、左侧栏收成窄条、卡片本身都不变」里给「仓库的收起」加了括号说明 P5-128 改了它；其后新增 P5-128 一条（用户原话 + 画法、共用代码、不变项、不做项）。
- 测试：`card.rs` 加 `a_folded_groups_heading_stacks_its_agents_cards`（谁在前、最多三张、活动窗格的 agent 不在这组时取第一个、没 agent 在活动窗格时按列表先后、只 1 个 agent 时一张）。

### 验证了什么

- `cargo clippy --all-targets -- -D warnings`：通过（只有 `block v0.1.6` 的既有 future-incompat 提示，与本改动无关）。
- `cargo fmt --check`：通过。
- `cargo test --lib`：574 通过 0 失败；新用例单跑也通过。
- `git diff --check`：干净。

### 拿主意的地方

- 叠卡作为标题行的直接子元素放在名字前，和 workspace 标题把 mark 放行里一致。
- 行高：收起时这一行从约 21.7pt 变成 28pt（叠卡 20pt 高），比展开的行高；和 workspace 收起(34)/展开(30) 一样是「收起略高」，没有为它改间距算法。`above`、`mb(-HEADING_Y)` 那套算的是行与行之间的间距，行变高只把下面的行往下推，不压。
- `stack` 为空时不画（`tab_stack::stack` 对空切片取 `faces[0]` 会 panic）；现在仓库组至少 1 个 agent 走不到这里，只是留个安全边。

### 没做的事

- 没起 paddock 窗口、没截图（照任务「样子留给用户看」），所以叠卡实际画出来的样子只在编译层面验过，构图/间距的实际观感留给用户实测。
- 没跑集成测试（`app/tests/*`，含 `workspaces.rs` 里 workspace 的那条叠卡用例）：任务的验证清单只要求 `--lib`。`--all-targets` 的 clippy 已覆盖它们能编译。
- 没加别的测试、没改卡片、workspace 托盘、分屏标签叠卡、窄条画法。

### 踩过的坑

没有。

