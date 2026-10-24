# 任务：shell 窗格左边加宽，灰条好点

2026-10-24，paddock/main 交给 paddock/dev-gutter（omp，默认模型；用户点名：“派omp去做”）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.79）、不要、拿不准；主控定：动到列数和鼠标坐标换算，算改行为；家由用户点名 omp）
类型：样式／文案调整
依据：P5-149 的灰条画在终端原有的 6px 左边留白里，用户嫌窄；本轮只把 shell 窗格的左边留白照样稿加宽、灰条能点的地方跟着变大，不改分段的逻辑。
执行：派出
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- 10-24（P5-149a 装好后实际看）：“有效果了，灰条宽度有点窄”。
- 主控说明过代价（加宽后 shell 窗格少一两列、和 agent 窗格左边对不齐）和下面的做法，用户：“派omp去做”。

## 先读
- 仓库根的 `CLAUDE.md`（规矩，尤其「独立编译目录」「桌面窗口测试」「验证」）。
- `docs/DESIGN.md` 的「终端按命令分段（P5-149）」和 §2「终端层守住能拆出去的边界」。
- 样稿：`docs/设计稿/P5-149-终端按命令分段/`（`Main.dc.html` 里左边 18px 的 `.gut`）。
- 代码：`app/src/view.rs`（`GRID_INSET` 及所有用到它的地方：内边距、列数、鼠标坐标换算、`paint_blocks_*`、位置条、选区工具条、提示的位置）、`app/src/view/blocks.rs`（点灰条的范围）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-149b-gutter`，分支 `p5-149b-gutter`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-149b-gutter`（已从 `.target/main` 克隆）。命令在 `app/` 下跑。

## 要做的
1. 只改 shell 窗格（开了命令分段的，普通 Shell 和 Agent shell）：左边留白从 6px 加到样稿的 18px（随界面缩放）；上、右、下不变。agent 窗格、浏览器等别的窗格一律不变。
2. 灰条照样稿画在这 18px 的中间：平时失败命令的 2px 红线、鼠标托起时 4px 灰条，位置都居中在留白里；能点的地方是整条左边留白（从提示符行到输出最后一行），不再只有 6px。托起的淡底照旧从左边留白伸到右边。
3. 少掉的宽度照常算进终端列数（列数随之少一两列）；鼠标点选、拖选、双击、链接、选区工具条、找字、位置条、⌘↑／⌘↓ 跳转、提示 `Click the bar to select output` 的位置都和现在一样对得上。
4. 文档：DESIGN「终端按命令分段（P5-149）」里“画在现有的左边 6px 留白里（没有照样稿另加 18px 的边）”那句改成新的做法，注明 P5-149b 和用户原话。

## 怎么算做完
- 用户原话是“灰条宽度有点窄”：shell 窗格的灰条照样稿变宽、整条左边留白都能点；agent 窗格不变；文字、选区、红线、刻度都对得上。
- 验证只做这些（改行为档）：给“shell 窗格左边留白 18、agent 窗格仍是 6”和“点在左边留白任意处都算点灰条、点到第一列文字不算”各加一条针对性测试；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。觉得不够，在回复里说，不要自己加。
- 画面留给主控合并安装后用户实际看。

## 不要做
- 不改分段的逻辑（`segments.rs`、`shell_marks.rs`），不改 agent 窗格和别的窗格的样子；不加样稿以外的东西。
- 终端层不依赖侧栏、corral、看板、Settings（§2）；窗格是不是 shell 用终端层已有的信息判断。
- 不开 paddock 窗口、不截图；不用脚本模拟按键或鼠标。
- 不开任何 corral agent，不碰 `corral ls` 里已有的 agent（不 stop／send／keys）。
- 测试不读用户真实的 shell 配置（`~/.zshenv` 里有用户的密钥），不改用户的任何文件。
- 不改 ranch、Saddle 仓库；不用 Python；不复制 Zed 等的 GPL 代码。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录。停自己起的进程用启动时记下的 PID。
- 碰到要拿主意的事，停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等，不要自己换别的办法。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-149b-gutter` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑，各几句话。踩过的坑指试了不行、换了办法的，工具或环境的行为和预想不一样的；没有就写「没有」，不要凑。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- 做了什么：新增 `app/src/view/gutter.rs`。shell 窗格（`queued_shell` 或 `viewer.shell` 有值，普通 Shell 和 Agent shell 都算）的左边留白取样稿的 18（随 `UiFont` 缩放），上、右、下仍 6；agent 窗格、命令窗格等别的窗格仍是 6。`TerminalView` 的 padding 改成单独设左边，`inset_left` 随帧记下，列数照少掉的宽度算。4px 灰条和失败命令的 2px 红线都以留白中线放；托起淡底、绘制裁剪区跟到新留白，灰条命中范围和鼠标离开窗格的边界也按新留白算，选区工具条左移同样的量。DESIGN 的 P5-149 画法一句改成新做法，注明 P5-149b 和用户原话。
- 验证了什么：新增 `view::gutter` 两条针对性测试（shell 留白 18、agent 仍 6，随界面缩放；整条左边留白都算灰条、第一列文字不算）；`app/` 下 `cargo test --all-targets`（1083 通过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次，都通过。按任务要求没开窗口、没截图。
- 拿主意的地方：shell 窗格用终端层已有的 `queued_shell`／`viewer.shell` 判断，没加侧栏、corral 等依赖；左边留白按 `UiFont` 缩放，其余三边保持原来的固定 6；灰条和红线都按留白中线算，灰条命中抽成 `gutter::on_bar` 以便测试。没有要主控决定的事。
- 没做的事：没改分段逻辑（`segments.rs`、`shell_marks.rs`）、agent 窗格和别的窗格；没合并、没推送。画面留给主控合并安装后用户实际看。
- 踩过的坑：没有。
