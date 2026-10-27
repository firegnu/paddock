# 任务：加 Memphis、Memphis Light 两套主题

2026-10-27，paddock/main 交给 paddock/dev-memphis（Claude Code，opus[1m] / high，不套墙：主控没设 child sandbox）。
路由：中 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.85）、不要、拿不准；读 `middle-design` 格；影响面主控定：动主题配色和 Settings 排法，现有主题必须不变，按改行为，同 P5-177）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
依据：用户 10-27 发了一张孟菲斯风格配色，要一套浅色主题，看过样稿后又要一套对应的深色版，让深浅两组对齐。本轮照选定的样稿（浅色乙、深色丙）加这两套；只换颜色，不做孟菲斯的描边、硬投影、圆点花纹。
执行：派出
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- （发了一张配色截图：黄 #FFC93C、粉 #FF8DB5、钴蓝 #2E5BE6、珊瑚红 #FF5A5F、薄荷绿 #A9E6C6、天蓝 #8DD3F6、淡紫 #D3BEFF，深靛描边 #25214A）“能否增加一个额外的浅色主题。具体的color你可以从这个里面取。我的意思是增加一个light theme。”
- “另外能不能在设计一套对应的dark 孟菲斯。这样颜色就对齐了。”
- “乙，深色照丙，写任务文件吧”

## 先读
- 样稿：`docs/设计稿/P5-194-Memphis主题/README.md`（两套的色值表和取色思路）和 `Main.dc.html`（HTML 源码，不用打开浏览器；乙、丙两个窗口是选定的，甲没选）。
- `docs/任务/P5-177-Dune-Tide-Lagoon浅色版.md` 的「要做的」和「完成记录」：加一对主题要动哪些地方、对比度怎么调、注释怎么写、哪些测试按主题循环。照同样的做法加。
- `app/src/preset.rs` 的 `Preset`（`ALL`、`light`、`groups`、`name`、`label`、`frost`）和一套深色、一套浅色的定义（如 `dune()`、`dune_light()`）；`app/src/theme.rs` 里对应的终端配色常量。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-194-memphis`，分支 `p5-194-memphis`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-194-memphis`（已从 `.target/main` 克隆）。三项检查在 `app/` 下跑（仓库根没有 `Cargo.toml`；单元测试在 lib 目标里，`--bin paddock` 跑不到）。
- `docs/DESIGN.md` §13 补一条 P5-194（用户原话照上面抄）；`app/README.md` 的主题表和配置示例补上两套。

## 要做的
1. **两套主题**，配置名和显示名：`memphis` / `Memphis`（深色），`memphis-light` / `Memphis Light`（浅色）。`Preset::ALL` 里排在最后（Memphis 在前，Memphis Light 跟在后面），用 `Preset::light` 配成一对。
2. **照样稿取色**：底、侧栏、选中卡片、正文、淡字、强调色、六种状态色、终端颜色、光标、选区照 README 的两张表。样稿没列的键（边框、分支、线、focus、connected、unread、input_text、Changes 的代码色、磨砂参数等），按其他主题同一角色的取法，从这套配色里挑（深色版用亮色原样，浅色版用同色相深一档）；终端的黑、白、亮白等槽位照现有深色、浅色主题的惯例排。agent 种类色样稿只画了 Claude、Codex、Kimi、omp，pi、amp、agy 用 README 的建议值，七种不能重复。注释写明出处：用户 10-27 给的孟菲斯配色，paddock 自己的主题。
3. **对比度**：照现有规矩，过不了现有测试的只调到刚好够，注释写明原值和新值。浅色版左侧栏是整块奶黄 `#ffe7a6`，状态字和安静字在它上面、在淡紫选中卡片上、在磨砂上都要够；往黑色压会发灰发脏的，可以改成同色相更深的颜色，写明。
4. **浅色、深色处理照样生效**：P5-175 起的浅色处理按 `Theme::is_light` 对 Memphis Light 生效，深色处理对 Memphis 生效。两套都看一遍主窗口、Settings、New Agent，看不清或怪的照前几件的路子修（按深浅取方向或数值，不给单个主题开特例）。
5. **Settings 8 对 8**：Dark 组最后是 Memphis，Light 组最后是 Memphis Light，上下对齐。应该只靠第 1 条的配对自动得到。
6. **别的不变**：现有 14 套主题画出来和现在完全一样。

## 怎么算做完
- 用户原话：“具体的color你可以从这个里面取”；“这样颜色就对齐了”；“乙，深色照丙”。「要做的」六条逐条做到。
- 用户装上后实际看：Settings 里 Dark、Light 各 8 张，上下一一对齐；两套新主题选上都看得清，浅色像样稿乙（左侧栏整块奶黄），深色像样稿丙；其他主题和以前一样。
- 验证只做这些：两套加进各个按主题循环、按名字列举的测试；Settings 分组排法的测试按 8 对 8 改；`git diff --check`；三项检查各跑一次（`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`）。第 4 条可以起测试窗口截图看（照下面「不要做」里的规矩），截图不入库；最终外观由用户看。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改任何现有主题的颜色；不改主题卡片样子；不加跟随系统深浅自动换主题；不改 Settings 里主题以外的部分，不加设置项。
- 不做描边、硬投影、圆点花纹这类颜色以外的孟菲斯效果；不加主题颜色键，不给单个主题开特例。真觉得非要不可，先停下来问。
- 不加新依赖，不改 `gpui-pre-*` 版本，不改用户本机的 `~/.config/paddock/config.toml`。
- 不复制 Zed 的 GPL 代码。
- 起测试窗口时：用一个临时 HOME 放测试用的 `.config/paddock/config.toml`，加 `PADDOCK_NO_ACTIVATE=1`，`CORRAL_HOME` 指向一个空的临时目录（临时 HOME 隔离不了 corral，不然侧栏会列出用户的 agent），记下 PID，用完按 PID 停；截图只截这个测试窗口自己（`GPUI_TERM_WINDOW_ID=1` 拿编号，`screencapture -x -o -l <编号>`），截图放 worktree 外面、不入库；不用脚本模拟键盘鼠标。临时加的调试钩子提交前删掉。
- 不用 Python（包括临时批量改文件）；命令里不用 `rm`（临时文件放 worktree 外的临时目录，留着不删）、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。改文件用编辑工具。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名。
- 不碰 `corral ls` 里现有的 agent，不对任何 agent `corral send`。
- 碰到要拿主意的事（比如样稿的颜色差太多要大改、要开特例或加主题颜色键），停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等，不要自己换别的办法。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-194-memphis` 上提交。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：做了什么（每套取了哪些色、调了哪些，第 4 条改了哪些地方）、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具或环境的行为和预想不一样的；没有就写「没有」），在你的分支里提交。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
