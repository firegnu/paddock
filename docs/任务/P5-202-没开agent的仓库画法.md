# 任务：workspace 里没开 agent 的仓库重画成虚线空位，整行能点

2026-11-24，paddock/main 交给 paddock/dev-vacant-slots（Claude Code，`opus[1m]` / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.97）、不要、改行为）
类型：样式／文案调整
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
依据：用户嫌有 agent 的 workspace 里 `No agents` 那块不好看，要求只改界面、不动业务逻辑。主控画了 A（虚线空位）和 B（一排胶囊），用户：“按 A 来，整行能点，写任务文件吧”。只改 `vacancies`、`vacant` 两处画法和能点的范围；哪些仓库算没开、什么时候合成一行、展开收起及其存盘都不动。
执行：派出
（理由：主控本想自己做，用户：“派吧。”样子已定，只是照样稿实现，按「中／界面和设计」读表：Claude Code `opus[1m]` high。）
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- “还有这个地方要精细调整一下。如果一个workspace中有多个repo，但是没有全部开。图中红框框住的No agents部分我觉得不好看。你看看能否重新设计一下，但是不要改动任何业务逻辑，也就是是说只是界面部分的调整”
- 看过 A、B 两版和一个问题（整行能点，还是只有虚线圈能点）：“按 A 来，整行能点，写任务文件吧”

## 先读
- 样稿：`docs/设计稿/P5-202-没开agent的仓库/README.md`，照 `N1`、`N2`、`N3`。
- `docs/DESIGN.md` §13 的 P5-134 合成行、P5-164、P5-201 里讲 `No agents` 的几句。
- 代码：`app/src/sidebar_workspaces.rs` 的 `vacancies`、`vacant`；`app/src/card.rs` 的 `Line::Vacant`、`Line::Vacancies` 怎么出（只读，不改）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-202-vacant-slots`，分支 `p5-202-vacant-slots`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-202-vacant-slots`（从 `.target/main` 克隆的）。
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
- 验证只做这些：`git diff --check`；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。不录屏、不截图、不打包安装（主控装好后由用户重开看）。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 `card.rs` 出行的规则、`Folds` 和它的存盘、`workspaces.json`。
- 不按 hover／group_hover 切换 display（P5-201 返工 1 的闪退），悬停时出现的东西用 `invisible()`／`visible()` 或透明度。
- 不按名字批量杀进程（`pkill -f paddock` 这类）；不截屏，不模拟鼠标键盘，不起 paddock 窗口。
- 不读、不改用户真实的 `~/.config/paddock/`、`~/.local/state/paddock/`；不碰 `corral ls` 里的任何 agent。
- 碰到要拿主意的事（例如样稿在 GPUI 里做不出、整行能点和别的点击冲突），停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不自己换别的办法。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-202-vacant-slots` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑，各几句话。踩过的坑指试了不行、换了办法的，工具或环境的行为和预想不一样的；没有就写「没有」，不要凑。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- 做了什么：只改 `app/src/sidebar_workspaces.rs` 的 `vacancies`、`vacant`（和 `count_chip` 一句过时的注释）。合成行展开：`No agents`、个数（淡一档的字，不再是小签）、`⌄` 挨着靠左，去掉细线。收起：两个叠着的虚线小圆（一张 SVG，后一个带 `+`，一起对准卡片头像那一列）、`cairn · ranch`（省略号）、淡字 `no agents`、`›` 紧跟。每个没开的仓库一行：行高 32，22 的虚线圆带 `+`、圆心对准头像那一列，仓库名；夹在组之间的单行后面跟 `no agents`。悬停：整行强调色 7% 淡底、18% 细边，圆和 `+` 变强调色、圆里垫 12%，名字亮到 `agents_text`，行尾出强调色 `New Agent`（透明度显隐）。整行是一个可点的元素，点了调同一个 `new_agent_in(workspace, path)`；原来的 `+` 按钮和 `New Agent in <repo>` 提示去掉。`Line::Empty`（`repo` 为 `None`）那条路径原样保留，`raised_body` 没动。`docs/DESIGN.md`：P5-134 合成行那句括号里注明 P5-202，末尾加 P5-202 一条。
- 验证了什么：`git diff --check`；`app/` 下 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 都通过。没截图、没起窗口，界面效果留给用户重开看。
- 拿主意的地方：虚线圆用 SVG 遮罩画（`stroke-dasharray`），没用 GPUI 的 `border_dashed`。这样两种圆的虚线一个样，叠着的那对用 SVG 的 `mask` 抠掉下面那个被盖住的一段，不靠填底色去盖，托盘悬停变亮时也不会露出一块色。圆和 `+` 用同一个颜色（`agents_text` 28%），样稿里 `+` 稍亮一点（32%），没另分。单仓库行和合成行下面的几行，左缩进都改成对准头像那一列（原来合成行下面的几行是缩进到箭头后面的字），照样稿。行高取 32（样稿 34）：托盘里行间距是 8，样稿是 2，在本文件范围里改不了行间距，略收一点。
- 没做的事：没加测试（纯画法，`card.rs` 出行规则没动，现有测试照旧覆盖）；没改行间距。
- 踩过的坑：没有。

## 主控审查

2026-11-24，paddock/main。
- 看了 diff：只动了 `sidebar_workspaces.rs` 的 `vacancies`、`vacant`（加几个常量和两个 SVG）、`DESIGN.md` 和本文件；`card.rs`、`Folds`、`workspaces.json`、`Line::Empty` 那行、`raised_body` 都没动。整行点击调的还是原来的 `new_agent_in`，参数不变。悬停出现的 `New Agent` 改的是透明度，没有按悬停切换 display。
- 重跑 `app/` 下 `cargo test --all-targets`：1123 个全过。
- 取舍：虚线圆用 SVG 画、叠着的两个圆用遮罩挖掉被挡的部分——同意；圆和 `+` 同一个颜色——同意；行高 32、行间距沿用托盘的——同意（行间距是托盘的规则，不在这件活里）；合成行展开后下面几行对齐头像那一列——同意，样稿就是这样。
- 已从分支提交 `0148487` 打包装到 `~/Applications/paddock.app`，等用户重开看。
- 用户重开看过：“重开了，可以，合并吧”。合并提交 `0588083`，main 上全套测试 1123 个全过，clippy、fmt 通过。
- 结果：一次过
