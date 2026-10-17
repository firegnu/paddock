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
