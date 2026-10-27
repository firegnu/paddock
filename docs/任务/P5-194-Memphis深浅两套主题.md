# 任务：加 Memphis、Memphis Light 两套主题

2026-10-27，paddock/main 交给 paddock/dev-memphis（Claude Code，opus[1m] / high，不套墙：主控没设 child sandbox）。
路由：中 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.85）、不要、拿不准；读 `middle-design` 格；影响面主控定：动主题配色和 Settings 排法，现有主题必须不变，按改行为，同 P5-177）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
依据：用户 10-27 发了一张孟菲斯风格配色，要一套浅色主题，看过样稿后又要一套对应的深色版，让深浅两组对齐。本轮照选定的样稿（浅色乙、深色丙）加这两套；只换颜色，不做孟菲斯的描边、硬投影、圆点花纹。
执行：派出
待用户：重开 paddock，在 Settings › Appearance 选 Memphis、Memphis Light 看效果（已从分支装好）
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

## 完成记录

### 做了什么
1. 两套主题：`Preset::Memphis`／`MemphisLight`，配置名 `memphis`、`memphis-light`，显示名 `Memphis`、`Memphis Light`，`Preset::ALL` 里排最后（Memphis 在前），`Preset::light` 配成一对（`preset.rs` 的 `memphis()`、`memphis_light()`，`theme.rs` 的 `MEMPHIS`、`MEMPHIS_LIGHT`）。注释写明出处：用户 10-27 给的孟菲斯配色，paddock 自己的主题。README 主题表、配置示例、主题说明补上，DESIGN §13 补了 P5-194 一条。
2. 取色（样稿 README 的两张表照抄，其余按同角色从这套配色里挑）：
   - **Memphis（丙）**：终端底 #1f1b3d、侧栏 #25214a、选中卡片 #3a3466（`selected`、`agent_selected`、`agents_faint`）、正文和亮字 #fff8ec、分支 #c9c3e6、淡字 #9c95c4（`muted`、`agents_dim`）、更淡 #7d76a8（`dim`、`agents_dimmer`）；强调、focus、干活中、蓝 #7a9bf5；空闲和沙箱薄荷 #a9e6c6、等你黄 #ffc93c、出错珊瑚 #ff5a5f、启动中淡紫 #d3beff、卡住粉 #ff8db5；connected 和 reply_heading 天蓝、unread 粉、reply_code 黄、input_text 取样稿窗口外框 #110f22。样稿没列的：线取侧栏到选中卡片一半 #302b58，边框取选中卡片到更淡字的三分之一 #504a7c、一半 #5c5587（`agents_border`）。agent 种类：Claude 珊瑚、Codex #7a9bf5、Kimi 粉、omp 淡紫，pi #fff8ec、amp 黄、agy 天蓝（README 建议值），七种不重复。终端：黑取选中卡片 #3a3466、白取分支 #c9c3e6、亮白正文，常规色照样稿，亮品红淡紫 #d3beff、亮黑 #7d76a8，其余亮色是常规色往白色调 25%（同 Tide、Lagoon）；光标黄、选区 #4a3f8a。深色版所有颜色本来就够，没调颜色。
   - **Memphis Light（乙）**：终端底 #fff8ec、左侧栏整块奶黄 #ffe7a6、选中卡片淡紫 #f1e8ff、正文 #25214a、终端底上的淡字 #5e5a7a／#8a859e（`muted`／`dim`）、侧栏上 #5a5470／#7d76a8（`agents_dim`／`agents_dimmer`，照 README；样稿 HTML 里乙的更淡写的是 #7d7586，和 README 不一致，照任务说的用 README）；强调、focus、干活中、蓝 #2e5be6；空闲 #1e7f52、等你 #8a6200、出错 #c8343a、启动中 #6d4bd1、卡住 #b83e7a；connected、reply_heading 青 #2484ba，reply_code 终端黄 #a87a00，unread #b83e7a。样稿没列的：线和空格子取样稿的暖灰一档 #e5ddd0（“zsh”标签胶囊的底），边框取它到更淡字一半 #b1aabc，分支取正文到淡字三分之一 #373257，沙箱同空闲绿（同 Lagoon Light）。agent 种类：Claude #c8343a、Codex #2e5be6、Kimi #b83e7a、omp #6d4bd1，pi #25214a、amp #8a6200、agy #1c76a8。终端：黑 #e5ddd0、白 #5e5a7a、亮白正文，常规色照样稿，亮黑 #8a859e，亮品红取启动中的深紫 #6d4bd1（深色版亮品红淡紫的镜像），其余亮色往黑色调 25%（同 Tide Light、Lagoon Light）；光标珊瑚 #e0454b。
3. 对比度（只调到刚好够，注释写了原值和新值；都是 Memphis Light）：
   - 奶黄侧栏上：`agents_green` #1e7f52→#1c774d（4.09→4.54:1）、`agents_red` #c8343a→#c23338（4.30→4.51）、`agent_stalled` #b83e7a→#b33c77（4.30→4.50），逐通道往黑色压；`agents_yellow` #8a6200 4.50:1、强调色 4.59:1，不用调。界面上的状态点（`agent_idle`、`agent_error`）、Kimi 色仍用样稿原值。
   - 终端选区 #ffe39a 和底只差 1.19 倍（测试要 1.25）：往黑色压会成土黄，改成往配色的黄 #ffc93c 挪四分之一，#ffdc82（1.26 倍）。
   - `agents_purple`（Changes 的关键字、远程机器标记）#6d4bd1→#6f4bd1：原值和钴蓝强调色只差 29.9°，机器标记测试要 30°；挪后 30.8°，两栏对比度仍 4.78:1 以上。
   - 终端 15 色在两套各自的底上都过 3:1，没调。
4. 浅色、深色处理：按 `Theme::is_light` 自动生效，按深浅分支的代码没动。改了一个磨砂参数：Memphis 的 `Frost.lit` 用 0.10（同 Dune；其他深色是 0.12），因为 Memphis 正文是很亮的暖白，0.12 时磨砂上活动格子图的“没提交”格子和第一级只差 1.07 倍（测试要 1.15）。`Frost` 本来就是每套一组，不算特例。Memphis Light 用其他浅色主题的那组，一次过。
5. Settings 8 对 8：只靠 `Preset::light` 的配对，`settings_view.rs` 没改（截图核实 Dark、Light 各 8 张，Memphis、Memphis Light 在最后一格上下对齐）。
6. 别的不变：diff 里现有 14 套主题的色值一行没动；删掉的行只有 `ALL` 的长度、groups 测试的注释、报错文字和下面这条测试断言。

### 验证了什么
- 两套加进按名字列举的测试：`presets_by_name`、`terminal_and_unknown_themes_are_errors`（报错列 16 个名字）、`the_light_presets_alone_are_light…`（浅色 8 套）；`groups::the_light_group_follows_the_dark_ones_order` 改成八对八。其余按 `Preset::ALL` 循环的测试自动包含两套。
- 改了一条测试断言：`sidebar::tests::cards_lift_in_the_text_colour_on_a_dark_theme_and_in_white_on_a_light_one` 原来用红通道判断“浅色主题的白卡面比侧栏亮”，奶黄 #ffe7a6 红通道已是 255，白色叠上去红通道不会再变大（绿、蓝都变亮）。改成比三个通道的和，意思不变，对已有 14 套判断结果一样。
- `git diff --check` 无输出；`app/` 下 `cargo test --all-targets --no-fail-fast` 全过（32 个目标，1097 条，lib 908 条）、`cargo clippy --all-targets -- -D warnings` 无警告、`cargo fmt --check` 通过。
- 截图（临时 HOME、`CORRAL_HOME=/tmp/p194corral` 空目录、`--corral` 指只回四个假 agent 的脚本、`PADDOCK_NO_ACTIVATE=1`，记 PID 按 PID 停，只截测试窗口）：Memphis Light 主窗口、Settings Appearance、New Agent；Memphis 的 New Agent（带主窗口）。窗口不在前台，截到的是失焦淡化的样子（奶黄侧栏被冲淡），字都看得清，卡片浮得起来。截图在会话临时目录，没进仓库。截图时临时在 `main.rs` 加过环境变量钩子（启动后打开 Settings 或 New Agent），提交前已删，diff 里没有。

### 拿主意的地方
- 卡片测试那条断言的改法（见上）。没有放宽：仍要求白面整体比侧栏亮。主控如果觉得应该改颜色而不是改测试，替代办法是把侧栏改成 #fee7a6 这类红通道不满的奶黄，我觉得是在迁就测试，没这么做。
- 选区不往黑色压，改为往配色的黄挪（任务里允许的“同色相更深”）。
- Memphis 的 `lit` 0.10（见上），而不是改样稿的强调色或正文色。
- 终端黑白槽：深色黑取选中卡片色、白取分支色；浅色黑取样稿暖灰 #e5ddd0、白取淡字 #5e5a7a。亮品红两边都用配色里的紫（淡紫／深紫）对齐。
- `agent_idle`、`agent_error`、Kimi 等只当点和图标用的颜色保留样稿原值，只压了当字读的 `agents_*` 和 `agent_stalled`（测试按字要求它）。

### 没做的事
- 孟菲斯的描边、硬投影、圆点花纹；跟随系统深浅；Memphis 的 Settings 截图（Settings 里两张卡片已在浅色那张截图里看到）。
- 没在 Settings 里实际点选切换、没看前台不失焦的样子（不模拟键鼠）；留给用户实际看。

### 踩过的坑
- P5-177 记的“调试版 New Agent 在 Controller 开着时崩”这次没出现（默认预设、Controller 选中也能打开），说明已经修了或不再触发；没深究。
- 其余没有。

## 主控审查

- 看了 diff：只动 `preset.rs`、`theme.rs`、`sidebar.rs` 一条测试、README、DESIGN 和本文件；现有 14 套主题色值没动，按深浅分支的代码没动，「不要做」里的一件都没做。Settings 8 对 8 靠 `Preset::light` 配对自动得到。
- 重跑：`cargo test --all-targets --no-fail-fast` 1097 过、0 失败，和 agent 说的一致；clippy、fmt、`git diff --check` 通过。
- 取舍表态：`sidebar.rs` 卡片测试从只比红通道改成比三通道之和，同意——原意是“白卡面整体比侧栏亮”，只比红通道是近似写法，奶黄侧栏红通道已满；对已有 14 套结果不变，不是放宽。不为迁就测试把侧栏改成 `#fee7a6`。选区改成往黄挪的 `#ffdc82`、`agents_purple` 转到 `#6f4bd1`、Memphis 磨砂 `lit` 0.10，都同意。样稿 HTML 与 README 的“更淡”色不一致时照 README，同意（HTML 里 `#7d7586` 是笔误）。
- 主控没起窗口看；已从分支提交 `5a66ff6` 打包装到 `~/Applications/paddock.app`，外观留给用户看。
- 结果：一次过
