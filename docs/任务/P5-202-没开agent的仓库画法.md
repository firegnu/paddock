# 任务：workspace 里没开 agent 的仓库重画成虚线空位，整行能点

2026-11-24，paddock/main。
类型：样式／文案调整
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
依据：用户嫌有 agent 的 workspace 里 `No agents` 那块不好看，要求只改界面、不动业务逻辑。主控画了 A（虚线空位）和 B（一排胶囊），用户：“按 A 来，整行能点，写任务文件吧”。只改 `vacancies`、`vacant` 两处画法和能点的范围；哪些仓库算没开、什么时候合成一行、展开收起及其存盘都不动。
执行：主控
（理由：只动 `sidebar_workspaces.rs` 的两个函数，这些行不参与侧栏的量高；主控刚读过这段代码、画了样稿，派出加审查的成本高过自己做。）

## 用户原话
- “还有这个地方要精细调整一下。如果一个workspace中有多个repo，但是没有全部开。图中红框框住的No agents部分我觉得不好看。你看看能否重新设计一下，但是不要改动任何业务逻辑，也就是是说只是界面部分的调整”
- 看过 A、B 两版和一个问题（整行能点，还是只有虚线圈能点）：“按 A 来，整行能点，写任务文件吧”

## 先读
- 样稿：`docs/设计稿/P5-202-没开agent的仓库/README.md`，照 `N1`、`N2`、`N3`。
- `docs/DESIGN.md` §13 的 P5-134 合成行、P5-164、P5-201 里讲 `No agents` 的几句。
- 代码：`app/src/sidebar_workspaces.rs` 的 `vacancies`、`vacant`；`app/src/card.rs` 的 `Line::Vacant`、`Line::Vacancies` 怎么出（只读，不改）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-202-vacant-slots`，分支 `p5-202-vacant-slots`（已从 main 建好）。
- 只动 `app/src/sidebar_workspaces.rs`、`docs/DESIGN.md` 和本文件；不加依赖，不加主题颜色键。

## 要做的（颜色一律从主题取，样稿色值只是示意）

1. **合成行展开时的标题**（`vacancies`，`open`）：一行小字 `No agents`、个数、`⌄`，三样挨着、靠左，箭头紧跟在个数后面（同底部 `Idle 9 ⌄` 的写法）。不要计数小签，不要细线，箭头不挪到最右。点它收起，同现在。
2. **合成行收起时**（`vacancies`，没 `open`）：两个叠着的小虚线圆（后一个带 `+`）、`cairn · ranch`（仓库名 ` · ` 连起来，过长省略号）、淡字 `no agents`、`›` 紧跟在后。点它展开，同现在。
3. **每个没开的仓库一行**（`vacant`，有仓库名，包括合成行展开后下面的几行和只有一个时夹在组之间的那行）：
   - 左边一个带 `+` 的虚线小圆，约 22pt，和卡片头像对齐的那一列；后面仓库名（同现在的字号、`agents_dimmer`）。只有一个、夹在组之间时，名字后面跟淡字 `no agents`；合成行下面的几行不写。
   - 鼠标停上去：整行强调色淡底和一圈细边，虚线圆和 `+` 变强调色，名字变亮，行尾出 `New Agent`（强调色）。
   - **整行能点**：点哪里都和现在点 `+` 一样，在这个仓库开 New Agent（同一个 `new_agent_in`，参数不变）。原来行尾单独的 `+` 按钮和它的悬停提示去掉，不再另放一个。
4. 浅色主题、字号调大时照样好看：虚线圆、淡底、细边都从主题色按透明度取。
5. `vacant` 没仓库名的那种（`Line::Empty`，托盘里一个都没有时的 `New Agent` 按钮）和放上来的空 workspace（`raised_body`）不动。
6. `docs/DESIGN.md`：P5-201 那条后面加 P5-202 一条，写这处新画法和整行能点；P5-134 合成行那句末尾的括号里注明 P5-202 又改了画法。

## 怎么算做完
- 用户原话照抄：“不要改动任何业务逻辑，也就是是说只是界面部分的调整”；“按 A 来，整行能点”。
- 验证：`git diff --check`；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`；打包装上，用户重开看。

## 不要做
- 不改 `card.rs` 出行的规则、`Folds` 和它的存盘、`workspaces.json`。
- 不按 hover／group_hover 切换 display（P5-201 返工 1 的闪退），悬停时出现的东西用 `invisible()`／`visible()` 或透明度。
- 不按名字批量杀进程；不截全屏，不模拟鼠标键盘。
