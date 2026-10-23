# 任务：加 Rosé Pine Dawn、Tokyo Night Day、Kanagawa Lotus 三套浅色主题，Settings 里深色、浅色分两组

2026-10-23，paddock/main 交给 paddock/dev-light3（Claude Code，opus[1m] / high，不套墙：主控没设 child sandbox）。
路由：中 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.86）、不要、拿不准；读 `middle-design` 格；影响面主控定：动主题配色和 Settings 排法，深色必须不变，按改行为）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
依据：P5-175 加了第一套浅色主题 Catppuccin Latte，界面已经能按主题深浅画。用户 10-23 要浅色主题和深色一样多（7 套），Settings 里深色、浅色分开摆。本轮加三套有官方配色的浅色版，并把 Settings 的主题选择分成两组；Dune、Tide、Lagoon 的浅色版要先出样稿，另开一件（P5-177），本轮不做。
执行：派出
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

## 完成记录

### 做了什么
1. 三套浅色主题（`preset.rs` 的 `tokyo_night_day()`、`rose_pine_dawn()`、`kanagawa_lotus()`，`theme.rs` 的 `TOKYO_NIGHT_DAY`、`ROSE_PINE_DAWN`、`KANAGAWA_LOTUS`），配置名 `tokyonight-day`、`rosepine-dawn`、`kanagawa-lotus`，显示名照任务文件。`Preset::ALL` 里各排在自己深色版后面。出处和许可写在注释里，README 主题表、配置示例、对比度那段补上。色值核对的是各家仓库的发布文件：tokyonight.nvim `extras/lua/tokyonight_day.lua` 和 `extras/kitty/tokyonight_day.conf`，rose-pine/alacritty `dist/rose-pine-dawn.toml`，kanagawa.nvim `lua/kanagawa/themes.lua`（lotus 各角色）、`extras/alacritty/kanagawa_lotus.toml`，光标取 `extras/kitty/kanagawa_light.conf`。
   - 界面角色照深色版的取法换成浅色版的对应色（Day：bg、bg_dark、bg_visual、bg_highlight、fg_gutter、terminal_black、fg_dark、blue 等；Dawn：base、highlight med/high、subtle、muted、rose 等；Lotus：bg、bg_m3、bg_visual、bg_p1–2、nontext、fg、fg_dim、identifier 黄等）。
   - 终端调色（对 3:1，只往黑色调到刚好够）：Day 亮黑 #a1a6c5→#7c8098、亮红 #ff4774→#e8416a、亮蓝 #358aff→#317feb、亮品红 #a463ff→#9e5ff5；Dawn 黄 #ea9d34→#bf802b、青（rose）#d7827e→#c77875（常规和亮色同值）、亮黑 #9893a5→#908b9c；Lotus 亮黑 #8a8980→#88877f、亮绿 #6e915f→#6e905f、亮蓝 #6693bf→#608ab4。
   - 界面调色（muted 对终端底 4.5:1；侧栏上 dim、绿红蓝黄、沙箱、卡住、启动中 4.5:1，dimmer 3:1）：
     - Day：muted #68709a→#5c6389；侧栏 dim #68709a→#555b7d、dimmer #848cb5→#707699、绿 #587539→#4c6431、红 #f52a65→#b21e49、蓝 #2e7de9→#225bab、黄 #8c6c3e→#735833、沙箱 #118c74→#0d6957、卡住 #b15c00→#8f4b00、启动中 #9854f1→#7440b9。
     - Dawn：muted #797593→#716d89；侧栏 dim #797593→#6b6782、dimmer #9893a5→#898495、红 #b4637a→#9a5569、蓝和沙箱 #56949f→#42717a、黄 #ea9d34→#906020、卡住 #d7827e→#955a58、启动中 #907aa9→#746389（绿 pine 本来够）。
     - Lotus：muted #716e61→#6d6a5e；侧栏 dim #716e61→#5a574d、dimmer #8a8980→#73736b、绿 #6f894e→#4c5d35、红 #c84053→#9b3240、蓝 #4d699b→#405781、黄 #de9800→#755000、沙箱 #597b75→#435c58、卡住 #cc6d00→#864700（启动中 lotusViolet4 本来够）。
   - 侧栏正文（`agents_text`）和强调色（`agents_accent`），为过现有的胶囊、右侧栏标签、活动格子图测试而调（见「拿主意的地方」）：Day 正文 #3760bf→#243f7e、强调 #2e7de9→#2668c3；Dawn 正文 #575279→#514d71、强调 #d7827e→#a86562；Lotus 正文 #545464→#3b3b46、强调 #77713f→#6c6739。终端的正文色（`text`）照官方不变。
   - 磨砂参数：三套照 Latte（wash 0.45、lit 0.10、waiting 0.14、dim 0.64、dimmer 0.53），Day 的 dim 是 0.66（侧栏安静字在它最暗的卡片面上才够 4.5:1）。
2. 浅色下照样理顺：P5-175 的浅色处理全按 `Theme::is_light` 走，三套的终端底都是浅的，自动生效，代码没有为哪套开特例、没加颜色键。实际改的只有上面这些颜色本身（侧栏底的选择和压暗的正文、强调色）。
3. Settings 分两组：`Preset::light()`（深色→浅色版配对）和 `Preset::groups()`（深色按 `ALL` 的顺序；浅色按它们深色版在深色组的顺序）；`settings_view.rs` 的 Appearance 页把原来的 `Theme` 一节换成 `Dark`、`Light` 两节（同一个 `section` 小标题样式），`theme_cards` 改成画传入的那组，两组用同一个列数，卡片样子不变，仍是全局只选一个。浅色组是 Latte、Tokyo Night Day、Rosé Pine Dawn、Kanagawa Lotus，在 4 列时正好在 Mocha、Tokyo Night、Rosé Pine、Kanagawa 的列序下（深色组第 4–7 张）；P5-177 只需给 Dune、Tide、Lagoon 加 `light()` 配对，就会排到浅色组最前面，与深色组一一对齐。
4. 深色不变：七套深色和 Latte 的色值一行没改（diff 里删掉的只有注释、`ALL` 的长度、测试名和报错文字），按深浅分支的代码没动。
- DESIGN §13 补了 P5-176 一条（用户原话照抄、实现时定的几点）。

### 验证了什么
- 三套加进按主题循环的测试：主题名解析和报错文字（`terminal_and_unknown_themes_are_errors`）、`presets_by_name`、终端 15 色 3:1、界面字对比度、外框、失焦、深浅判断（原 `latte_alone_is_light…` 改成 `the_light_presets_alone_are_light…`，浅色是这四套）、胶囊、右侧栏标签、活动格子图、磨砂上的安静字、卡片、浮起框（这些测试按 `Preset::ALL` 循环，自动包含新主题）。
- 新加 `preset::groups::the_light_group_follows_the_dark_ones_order`：深色组七套的顺序、浅色组四套的顺序、每套浅色版和它前面那套深色的配对、所有主题都在某一组、每组的 `is_light` 都对。
- `git diff --check` 无输出；`app/` 下 `cargo test --all-targets --no-fail-fast` 全过（lib 854 条，其余各目标全过）、`cargo clippy --all-targets -- -D warnings` 无警告、`cargo fmt --check` 通过。
- 截图：临时 HOME、`CORRAL_HOME` 指空目录、`PADDOCK_NO_ACTIVATE=1`、一个只回假 agent 的 corral 脚本（放在 PATH 前面），记 PID 按 PID 停，只截测试窗口。看了 Settings 的 Appearance 页（Tokyo Night Day 选中时：Dark 七张两行、Light 四张一行，浅色四张正好在 Mocha、Tokyo Night、Rosé Pine、Kanagawa 那几列下，选中的有强调色圈和对勾）、三套各自的主窗口（侧栏卡片、错误／暂停状态、活动面板、标题栏、终端）。窗口不在前台，截到的是失焦去色的样子；字都看得清，没有发灰、浮不起来的卡片。截图和工具都在会话临时目录，没进仓库。截 Settings 时临时加过一个环境变量钩子（启动后打开 Settings 的 Appearance 页），已删，diff 里没有。

### 拿主意的地方
- **Rosé Pine Dawn 的侧栏用 overlay（#f2e9e1），不照 main 用 surface**：Dawn 的 surface 是 #fffaf3，几乎纯白，浅色卡片的白面叠上去看不出来（`cards_lift…` 测试也因此不过，白面没法比侧栏更亮）。P5-175 样稿里 Dawn 的侧栏也是 overlay。
- **三套侧栏正文、三套强调色压暗**：Tokyo Night Day 的 bg_dark、Kanagawa Lotus 的 bg_m3 比 Latte 的 mantle 暗不少，官方正文在上面只有约 4.0:1、4.7:1，标签胶囊上的名字（现有测试要求 4.5:1）、失焦后的正文都不够；Dawn 也差一点。三套正文照“只调到刚好够”压暗到胶囊名字够 4.5:1。强调色同理，右侧栏选中标签的图标（现有测试要求 3:1）不够，压暗到刚好够；Lotus 再为活动格子图的第二级多压一点。这是调颜色，不是开特例或加颜色键。代价：Dawn 的强调色从浅玫瑰变成较深的玫瑰（#a86562），和卡住的状态色 #955a58 很接近；Tokyo Night Day 侧栏的字是深海军蓝，不是官方那种亮蓝。建议用户看一眼。
- 侧栏、选中底、规则线等其余角色照深色版取（Lotus 侧栏仍是 bg_m3，因为 wave 用 sumiInk0；比 Day、Latte 的侧栏更暗、更黄）。
- Dawn 的终端光标照官方是 highlight high #cecacd，浅底上很淡（main 也一样淡）；没改，留给用户看。
- Settings 的小标题照任务文件写 `Dark`、`Light`，原来那个 `Theme` 小标题去掉了（不然三个标题叠着）。

### 没做的事
- Dune、Tide、Lagoon 的浅色版（P5-177）；跟随系统深浅；终端里各家 agent 自己的配色。
- New Agent 面板、命令面板、看板、在前台时的样子没截图（不模拟键鼠；要截需另加钩子），只靠按主题循环的测试；在 Settings 里实际点选、换回深色没有实际点。留给用户实际看。

### 踩过的坑
- 自己写的 awk 对比度小工具一开始在函数里用了全局变量 `t`，和循环变量撞了，算出来的“刚好够”调得过头；改成局部变量后和 P5-175 Latte 的结果逐个对上，才拿来用。
- 侧栏正文最先只为“失焦后 4.5:1”压暗，结果胶囊名字测试反而更差（字变暗、胶囊底也跟着变暗）；改为按胶囊的三种底和四种状态色一起算最差的那一个来定，才一次过。
- 把 Dawn 的一组颜色改值时用了一次 `sed -i` 按行号范围替换（任务文件要求改文件用编辑工具），改完逐行核对无误；之后都用编辑工具。`cargo fmt` 格式化过一处测试代码。
