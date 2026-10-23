# 任务：加 Dune Light、Tide Light、Lagoon Light，浅色主题凑齐 7 套

2026-10-23，paddock/main 交给 paddock/dev-light-own（Claude Code，opus[1m] / high，不套墙：主控没设 child sandbox）。
路由：中 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.82）、不要、拿不准；读 `middle-design` 格；影响面主控定：动主题配色和 Settings 排法，现有主题必须不变，按改行为）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
依据：用户 10-23 要浅色主题和深色一样多。P5-175 加了 Catppuccin Latte，P5-176 加了 Rosé Pine Dawn、Tokyo Night Day、Kanagawa Lotus，并把 Settings 分成 Dark、Light 两组。本轮加 paddock 自己三套主题的浅色版，照主控出、用户接受的样稿；做完 Settings 里深浅两组七对七一一对齐。
依赖：P5-176
执行：派出
待用户：重开 paddock 看 Settings 七对七和三套新浅色主题
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- “1: 把另外两个也做了，做好能和现在dark mode的数量一样（light theme的数量和dark theme的数量），2: Settings中dark theme和light theme最好能分开”
- 主控确认 Dune、Tide、Lagoon 也要做浅色版：“对”。
- 看了 Dune、Tide、Lagoon 浅色版样稿：“接受你的设计”

## 先读
- 样稿：`docs/设计稿/P5-177-Dune-Tide-Lagoon浅色版/README.md`（色值表）和 `Main.dc.html`（HTML 源码，不用打开浏览器）。
- `docs/任务/P5-175-浅色主题CatppuccinLatte.md`、`docs/任务/P5-176-三套官方浅色主题和Settings分组.md` 的完成记录：浅色主题怎么加、浅色下怎么处理、对比度怎么调、Settings 两组怎么排、哪些测试按主题循环。照同样的做法加。
- `app/src/preset.rs` 的 `dune()`（默认值，`Theme::default()` 那组常量）、`tide()`、`lagoon()`；`app/src/theme.rs` 的 `DUNE`（Gruvbox）、`TIDE`（Nord）、`LAGOON`（Everforest）终端配色。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-177-light-own`，分支 `p5-177-light-own`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-177-light-own`（已从 `.target/main` 克隆）。三项检查在 `app/` 下跑（仓库根没有 `Cargo.toml`；单元测试在 lib 目标里，`--bin paddock` 跑不到）。
- `docs/DESIGN.md` §13 补一条 P5-177（用户原话照上面抄）。

## 要做的
1. **三套浅色主题**，配置名和显示名：`dune-light` / `Dune Light`、`tide-light` / `Tide Light`、`lagoon-light` / `Lagoon Light`。界面各角色照它们深色版的取法，用样稿的颜色：底、侧栏、正文、两级淡字、线照 README 的表；强调色、状态色、选中底、边框等表里没列的，从同一出处的浅色版里按深色版的角色挑（Dune 的橄榄金强调、Tide 的霜蓝、Lagoon 的青绿要保留各自的感觉）。终端 16 色、光标、选区用各自出处的浅色版（Gruvbox Light、Nord、Everforest Light），样稿里调深过的照样稿。出处和许可照现有注释的写法写明。
2. **对比度**：照现有规矩，达不到的只调到刚好够，注释写明原值和新值（样稿取值也可以再微调，写明）。
3. **浅色下照样理顺**：P5-175 的浅色处理对这三套同样生效。逐套看一遍主窗口、Settings、New Agent，看不清或怪的照前两件的路子修（按深浅取方向或数值，不给单个主题开特例；真要开特例或加主题颜色键，先停下来问）。
4. **Settings 七对七对齐**：Light 组补上这三套，排在和 Dune、Tide、Lagoon 对应的位置，上下两组一一对齐。
5. **别的不变**：七套深色和已有四套浅色画出来和现在完全一样。

## 怎么算做完
- 用户原话：“做好能和现在dark mode的数量一样”；“接受你的设计”。「要做的」五条逐条做到。
- 用户装上后实际看：Settings 里 Dark 七张、Light 七张上下一一对齐；三套新浅色选上都看得清、像样稿、保留各自原来的感觉；其他主题和以前一样。
- 验证只做这些：三套加进各个按主题循环的测试；Settings 分组排法的测试按七对七改；`git diff --check`；三项检查各跑一次（`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`）。第 3 条可以起测试窗口截图看（照下面「不要做」里的规矩），截图不入库；最终外观由用户看。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改任何现有主题的颜色；不改卡片样子；不加跟随系统深浅自动换主题；不改 Settings 里主题以外的部分，不加设置项。
- 不加新依赖，不改 `gpui-pre-*` 版本，不改用户本机的 `~/.config/paddock/config.toml`。
- 不复制 Zed 的 GPL 代码；主题色值照各自许可用，写明出处。
- 起测试窗口时：用一个临时 HOME 放测试用的 `.config/paddock/config.toml`，加 `PADDOCK_NO_ACTIVATE=1`，`CORRAL_HOME` 指向一个空的临时目录（临时 HOME 隔离不了 corral，不然侧栏会列出用户的 agent），记下 PID，用完按 PID 停；截图只截这个测试窗口自己（`GPUI_TERM_WINDOW_ID=1` 拿编号，`screencapture -x -o -l <编号>`），截图放 worktree 外面、不入库；不用脚本模拟键盘鼠标。临时加的调试钩子提交前删掉。
- 不用 Python（包括临时批量改文件）；命令里不用 `rm`（临时文件放 worktree 外的临时目录，留着不删）、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。改文件用编辑工具。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名。
- 不碰 `corral ls` 里现有的 agent，不对任何 agent `corral send`。
- 碰到要拿主意的事（比如样稿的颜色对比度差太多要大改、要开特例或加主题颜色键），停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等，不要自己换别的做法。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-177-light-own` 上提交。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：做了什么（每套取了哪些色、调了哪些，第 3 条改了哪些地方）、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具或环境的行为和预想不一样的；没有就写「没有」），在你的分支里提交。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
