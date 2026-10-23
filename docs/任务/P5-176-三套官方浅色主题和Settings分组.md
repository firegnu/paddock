# 任务：加 Rosé Pine Dawn、Tokyo Night Day、Kanagawa Lotus 三套浅色主题，Settings 里深色、浅色分两组

2026-10-23，paddock/main 交给 paddock/dev-light3（Claude Code，opus[1m] / high，不套墙：主控没设 child sandbox）。
路由：中 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.86）、不要、拿不准；读 `middle-design` 格；影响面主控定：动主题配色和 Settings 排法，深色必须不变，按改行为）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
依据：P5-175 加了第一套浅色主题 Catppuccin Latte，界面已经能按主题深浅画。用户 10-23 要浅色主题和深色一样多（7 套），Settings 里深色、浅色分开摆。本轮加三套有官方配色的浅色版，并把 Settings 的主题选择分成两组；Dune、Tide、Lagoon 的浅色版要先出样稿，另开一件（P5-177），本轮不做。
执行：派出
待用户：重开 paddock 看 Settings 两组和三套浅色主题
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- 看了 Latte：“太cool了，还有两个事情，1: 把另外两个也做了，做好能和现在dark mode的数量一样（light theme的数量和dark theme的数量），2: Settings中dark theme和light theme最好能分开，你理解我意思吧？评估一下工作量。”
- 主控建议先做三套官方浅色版（Rosé Pine Dawn、Tokyo Night Day、Kanagawa Lotus）并把 Settings 分组，Dune、Tide、Lagoon 的浅色版先出样稿：“接受你的建议。”确认 Dune、Tide、Lagoon 也要做浅色版：“对”。Settings 分组发来一张截图（上面一组 Light Mode、下面一组 Dark Mode，各一排主题卡片）：“类似这种。”
- 主控问选法和卡片样子，用户选：“只是分组摆放，还是选一个”；“保留现在的小窗口缩略图”。

## 先读
- `docs/任务/P5-175-浅色主题CatppuccinLatte.md` 全文，特别是完成记录：浅色下哪些地方怎么处理的、对比度怎么调、`Theme::is_light()`、`shade()`、各处按深浅分支和守住深色不变的测试。这件照同样的做法加。
- `app/src/preset.rs` 的 `Preset`（`ALL`、`name`、`label`、`frost`）和 `latte()`、`rose_pine()`、`tokyo_night()`、`kanagawa()`。
- `app/src/theme.rs` 的 `LATTE`、`ROSE_PINE`、`TOKYO_NIGHT`、`KANAGAWA` 终端配色，按主题循环的测试。
- `app/src/settings_view.rs` 的 `theme_cards`（约 2205 行起）和调用它的地方（列数怎么定）。
- 样稿：`docs/设计稿/P5-175-浅色主题/Main.dc.html` 里 `dawn`、`day` 两组颜色，大致样子参考；具体颜色以各家官方发布的为准。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-176-light3`，分支 `p5-176-light3`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-176-light3`（已从 `.target/main` 克隆）。三项检查在 `app/` 下跑（仓库根没有 `Cargo.toml`；单元测试在 lib 目标里，`--bin paddock` 跑不到）。
- `docs/DESIGN.md` §13 补一条 P5-176（用户原话照上面抄）。

## 要做的
1. **三套浅色主题**，照 Latte 的加法：
   - Rosé Pine Dawn：配置名 `rosepine-dawn`，显示名 `Rosé Pine Dawn`，出自 rose-pine 官方（写明出处和许可）。
   - Tokyo Night Day：配置名 `tokyonight-day`，显示名 `Tokyo Night Day`，出自 folke/tokyonight.nvim 的 day 配色（Apache-2.0，原作 enkia 的 MIT，两处都写，照 P5-40 的写法）。
   - Kanagawa Lotus：配置名 `kanagawa-lotus`，显示名 `Kanagawa Lotus`，出自 rebelot/kanagawa.nvim 的 lotus 配色（写明出处和许可）。
   界面各角色照它们深色版的取法换成浅色版的对应色；终端 16 色、光标、选区照各家官方发布的浅色终端配色。对比度照现有规矩，达不到的只调到刚好够，注释写明原值和新值。
2. **浅色下照样理顺**：P5-175 做的浅色处理对这三套同样生效（按 `is_light()`）。逐套看一遍主窗口、Settings、New Agent，有哪套看不清或怪的，照 P5-175 的路子修（按深浅取方向或数值，不给单个主题开特例；真要开特例或加主题颜色键，先停下来问）。
3. **Settings 分两组**：主题选择分成上下两组，上面 `Dark`，下面 `Light`，各有一个小标题（照 Settings 里现有小标题的样子），下面一排卡片（还是现在的小窗口缩略图，卡片样子不变）。两组用同样的列数；浅色组的顺序跟着深色组：某套深色有浅色版的，浅色版排在和它对应的位置（Catppuccin Mocha ↔ Latte、Tokyo Night ↔ Tokyo Night Day、Rosé Pine ↔ Rosé Pine Dawn、Kanagawa ↔ Kanagawa Lotus）。Dune、Tide、Lagoon 的浅色版这次还没有，浅色组先按这个相对顺序排四张；以后补齐 7 套时，上下要能一一对齐（这件不用先留空位，但排法要让 P5-177 加进来时只是往前补）。整个 Settings 还是只选一个主题，选中的照旧有强调色圈和对勾。
4. **深色不变**：七套深色主题画出来和现在完全一样；Latte 也不变。

## 怎么算做完
- 用户原话：“把另外两个也做了”“Settings中dark theme和light theme最好能分开”；“类似这种”（上下两组）；“只是分组摆放，还是选一个”；“保留现在的小窗口缩略图”。「要做的」四条逐条做到。
- 用户装上后实际看：Settings 里上面一组 Dark 七张、下面一组 Light 四张（Latte、Tokyo Night Day、Rosé Pine Dawn、Kanagawa Lotus 照对应位置排），每套浅色选上都看得清、不怪；深色和以前一样。
- 验证只做这些：三套加进各个按主题循环的测试（对比度、外框、浅色判断等）；分组排法加一条测试（哪套在哪组、浅色组的顺序跟着深色组）；`git diff --check`；三项检查各跑一次（`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`）。第 2 条可以起测试窗口截图看（照下面「不要做」里的规矩），截图不入库；最终外观由用户看。觉得不够，在回复里说，不要自己加。

## 不要做
- 不做 Dune、Tide、Lagoon 的浅色版；不加跟随系统深浅自动换主题；不改卡片的样子；不改任何现有主题的颜色。
- 不改 Settings 里主题以外的部分，不加设置项。
- 不加新依赖，不改 `gpui-pre-*` 版本，不改用户本机的 `~/.config/paddock/config.toml`。
- 不复制 Zed 的 GPL 代码；主题色值照各自许可用，写明出处。
- 起测试窗口时：用一个临时 HOME 放测试用的 `.config/paddock/config.toml`，加 `PADDOCK_NO_ACTIVATE=1`，`CORRAL_HOME` 指向一个空的临时目录（临时 HOME 隔离不了 corral，不然侧栏会列出用户的 agent），记下 PID，用完按 PID 停；截图只截这个测试窗口自己（`GPUI_TERM_WINDOW_ID=1` 拿编号，`screencapture -x -o -l <编号>`），截图放 worktree 外面、不入库；不用脚本模拟键盘鼠标。临时加的调试钩子提交前删掉。
- 不用 Python（包括临时批量改文件）；命令里不用 `rm`（临时文件放 worktree 外的临时目录，留着不删）、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。改文件用编辑工具（P5-175 用 perl 改出过错）。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名。
- 不碰 `corral ls` 里现有的 agent，不对任何 agent `corral send`。
- 碰到要拿主意的事（比如某套要单独开特例、要加主题颜色键、Settings 分组怎么排都对不齐），停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等，不要自己换别的做法。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-176-light3` 上提交。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：做了什么（每套调了哪些颜色，第 2 条改了哪些地方）、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具或环境的行为和预想不一样的；没有就写「没有」），在你的分支里提交。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
