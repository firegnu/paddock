# 任务：Cairn 面板里不能断行的长文字（长路径、行内代码）被框切掉（omp 做的一份）

2026-10-17，paddock/main 交给 paddock/dev-cairn-wrap-omp（omp，默认：deepseek/deepseek-flash:high）。
路由：常规 / 交叉审查不要 / 影响面：看得见（路由：常规（置信 0.53）、不要、看得见）
类型：Bug 修复
依据：P5-117 完成记录（10-15）：“截图时 Cairn 第一条摘要里一段不能断行的长路径（`` `~/Applications/paddock.app` `` 一行）右边被框切掉”；HANDOFF 一直挂着“待另开任务”。用户 10-17 想找一件难一点的活考一考 agy，主控提了这件，用户：“这个好像还没任务呢？”看过草稿：“换行就行，不让它开窗口”。agy 那份（P5-137）做了 70 分钟没动代码，用户：“不管他了，你开一个omp，同样的问题送过去。看看deepseek + omp的能力”。本轮查明原因并修好；不改 Cairn 面板别的样子。
执行：派出
提示：依据证据定位并修复导致问题的原因，保持无关行为不变。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 现在知道的（主控 10-17 记的，没查原因）
- 右侧栏 Cairn 面板把 cairn 的摘要（Markdown）一段段画出来：`app/src/cairn_view.rs` 里 `markdown::blocks_by_line` 那一段（标题、段落、列表），字的样式用 `app/src/markdown_view.rs` 的 `styled`（行内代码等宽字、底色）。
- 现象：一段里有一个很长、中间没有空格的词（多半是行内代码里的路径），面板窄的时候它右边被框切掉，不换行，也没有 `…`。
- 原因不知道：可能是 GPUI 换行时不拆超过一行宽的长词，可能是某一层 `whitespace_nowrap`／`overflow_hidden`／宽度没传下去，也可能是别的。
- 同一套 Markdown 画法在看板（`kanban_view.rs`）、收件箱（`window_inbox.rs`）、Settings 的说明（`settings_instructions.rs`）里也在用，可能有同样的毛病。

## 先读
- `AGENTS.md`（规矩全文）。
- 本文件全部。
- `docs/DESIGN.md` 里 Cairn 面板的几条（搜 `P5-55`、`P5-79`、`P5-133`）。
- 代码：`app/src/cairn_view.rs` 画摘要的地方、`app/src/markdown.rs`、`app/src/markdown_view.rs`；GPUI 的文字换行（`gpui` 的 `LineWrapper`、`StyledText`，源码在 cargo 的依赖目录里，只读）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-137a-cairn-wrap-omp`，分支 `p5-137a-cairn-wrap-omp`（已从 main 建好）。
- 编译：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-137a-cairn-wrap-omp`（已从 `.target/main` 克隆）。cargo 命令在 `app/` 下跑。
- 只动 `app/src/` 下和这个毛病直接相关的文件、相关测试、`docs/DESIGN.md` 和本文件。

## 要做的
1. 查明原因：为什么长词不换行、被切掉。写清楚是哪一层、证据是什么（代码位置、GPUI 的哪段行为）。
2. 修好：Cairn 摘要里放不下一行的长词换到下一行接着写（在词中间断开），整段文字都看得到，不被框切掉、不出横向滚动。
3. 原因如果在共用的 Markdown 画法里，就在共用的地方修，看板、收件箱、Settings 说明跟着好；原因只在 Cairn 一处，就只修 Cairn，别处有没有同样的毛病写进完成记录，不顺手改。
4. 修法如果要改 GPUI 本身（`gpui-pre-*` 依赖），停下来报告，不要改依赖、不要 fork。

## 怎么算做完
- 用户验收原话：“换行就行”。
- 先写一条能复现的测试（例如用 GPUI 的测试环境量出这段文字排成几行、最宽一行有没有超出给定宽度），改之前跑一次确认因为这个毛病失败，再修到通过。测试环境量不了真实字宽、写不出这条测试的，在完成记录里说明为什么，样子留给用户装好后看。
- 验证只做这些：上面的测试；`git diff --check`；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 Cairn 面板别的样子（框、标题、切换、列表）；不改 Markdown 的解析规则。
- 不改 `gpui-pre-*` 依赖，不加依赖，不 fork GPUI。
- 不读用户真实的 cairn 数据库和记录，不运行会写 cairn 数据的命令（`cairn save`、`adopt`、`summarize` 这类）；假 cairn 只用合成数据。
- 不起 paddock 窗口、不截图（用户 10-17 定）；不用脚本模拟按键或鼠标；不碰 `corral ls` 里的任何 agent。
- 不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录。
- 不看、不碰同一件活的另一份（agy 的 worktree `p5-137-cairn-wrap` 和它的分支）。
- 不合并到 main，不推送。只在 `p5-137a-cairn-wrap-omp` 上提交，提交说明用中文、首行以 `P5-137a：` 开头。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：原因（哪一层、证据）、做了什么、验证了什么（写明改前测试怎么失败的）、拿主意的地方、没做的事（别处有没有同样的毛病）、踩过的坑（没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 原因
- 在 GPUI 的截断里，不是 Cairn 的宽度链或 Markdown 画法：`cairn_view.rs` 的 `written`（笔记全文、原文）和普通段落经测量都按实际字宽在行尾折行，长词也在词中间断，没有超出框。
- 出问题的是 Next session 框里摘要那一行：它同时用了 `text_ellipsis()`（`TextOverflow::Truncate("…")`）和 `line_clamp(2)`。GPUI（`gpui-pre 0.3.8`）这时走 `TextLayout::layout` → `LineWrapper::truncate_wrapped_line`（`src/text_system/line_wrapper.rs:361`）：它用 `width_for_char`（`TextSystem::layout_width`，逐字、按元素的基础字体量）估算宽度来决定在哪截断，而不是用整行实际排出来的宽度。中文（字体回退的 CJK 字）按字估算会少算，于是它以为截断后的文字还放得进两行、没有截够；随后 `shape_text` 用真实字宽重排，`LineLayout::compute_wrap_boundaries`（`src/text_system/line_layout.rs:193`）在 `line_clamp` 用满（`boundaries.len() >= max_lines - 1`）时直接 `break`，把剩下的文字全留在最后一行——最后一行因此比框宽，框的 `overflow_hidden` 从右边切掉，省略号也在被切掉的尾部，所以“没有 …”。
- 证据（不提交的测量程序，`app/examples/measure_text.rs`，跑完已移出仓库到 `/tmp/p5-137a-measure_text.rs`）：在一个 344pt 宽、和 Next session 框相同的内边距、同一套 `text_ellipsis()+line_clamp(2)` 的文本元素上量 `StyledText` 的 `TextLayout`。中文摘要（含 `~/Applications/paddock.app` 一类路径）60 条里 12 条最后一行 357.6–370.2pt，比 344pt 宽 13–26pt；换成不带截断、只折行的同一批文本，0 条超出。英文摘要 60 条加 40 档宽度都不超出，所以现象只在中文摘要上出现（历史摘要多半是中文）。
- 早先记录的“写一行摘要的 Earlier 行”不是这里：它的截断走的是单行 `truncate_line`，本次量到的 114pt 宽的同一行中文摘要排出来 109.4pt，没超。

### 做了什么
- `app/src/cairn_view.rs`：Next session 的摘要不再 `text_ellipsis()`、不再 `line_clamp(2)`，整段按实际字宽折行（长词在词中间断），框里放得下的都看得到，也没有横向溢出。`session()` 的说明相应改掉“两行”。
- 原来 `summary()` 上的 `text_ellipsis()` 挪到 Earlier 行那个 `flex_1` 容器上（那一行照旧一行、截断带 `…`），行为不变。`summary()` 本身只留字号、行高和颜色。
- `docs/DESIGN.md`：P5-110 那条里 Next session 的“摘要最多两行”改成“整段换行显示（P5-137a）”，写明原因。
- 没改 GPUI、没加依赖、没动 Cairn 别的样子（框、标题、切换、Earlier 的列表）。

### 验证了什么
- 改前：上面的测量程序按原代码（`text_ellipsis()+line_clamp(2)`）跑 60 条中文摘要，12 条最后一行超出 344pt（最多 370.2pt），并确认最后一行就是被裁到框外的那一行；同样的文本改成折行后 0 条超出。
- 改后：同一测量程序里，去掉 `text_ellipsis()+line_clamp(2)` 的 60 条中文摘要全部 ≤ 344pt（最宽一行都在框内），长词按真实字宽在中间断开。
- `git diff --check`、`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets` 全部通过。
- 没有窗口测试（任务要求不起 paddock 窗口、不截图），实际观感留给用户装好后看。

### 拿主意的地方
- 没有自己写 GPUI 级的截断/折行来代替 `line_clamp`（要拿窗口里的 text system 和每处宽度，等于重做一段文本布局），也没有加 GPUI 的 `test-support`（会引入 `proptest` 等新包，任务不许加依赖）。按用户“换行就行”“整段文字都看得到”，Next session 的摘要整段折行、不截断；Earlier 行仍按 P5-110 的设计一行截断。
- 中文摘要才是能复现的条件，英文/ASCII 摘要量不出超出；没有把这个当成“英文没问题”，只是记录清楚触发面。

### 没做的事（别处有没有同样的毛病）
- 别处用 `text_ellipsis()+line_clamp` 或单行截断的地方（看板标题行、收件箱、Settings 说明里的截断文本等）仍用 GPUI 的估算截断；本次量到中文内容在 `line_clamp` 下会多留、最后一行超出框。Cairn 里其余 `text_ellipsis`（仓库名、分支小签、Earlier 摘要、记录标题）本次没改；单行截断的样本量到没超出，但没有逐个内容验证。
- 没改 `markdown_view::styled`：Markdown 正文（`written`）的折行本身没问题，问题在截断那一层；共用画法这次不需要动。
- 没合并到 main、没推送；不在 main 上提交任务文件。

### 踩过的坑
- 一开始按“英文路径 + 普通折行”复现，怎么量都不超出，差点以为改不出来；换成中文摘要（含路径）后才复现，原因是 GPUI 逐字估算在 CJK 回退字上少算。
- 先写的那版 `cairn_view` 改动把 `summary()` 的 `text_ellipsis()` 删掉后，Earlier 行的容器上并没有 `text_ellipsis()`，会变成“不换行也不带 …”地硬切；已把省略号补到容器上并量过（114pt 的框里截到 109.4pt、带 `…`）。
