# 任务：paddock 窗口不在前台时整个窗口去色，一眼看得出来

2026-10-20 用户提出，记成草稿；同天用户说“开始做 P5-162.”，主控逐条问定，用户同意验收。paddock/main 交给 paddock/dev-unfocused（Claude Code，opus[1m] / high，套 fence `dev` 墙）。
依据：用户 10-20：“再加一个draft task，当焦点不在paddock窗口的时候，要能识别出来现在paddock不是当前的焦点。”本轮只改显示，不按焦点改别的行为。
执行：派出
（理由：只改显示，但要动主题颜色怎么取、终端绘制、侧栏、标签和窗格的绘制，跨好几个模块，按分工表第 6 条读 `design` 格。主控审，装上给用户看。）
路由：常规 / 交叉审查不要 / 影响面：看得见（路由：常规（置信 0.28）、不要、看得见）
类型：样式／文案调整
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话和定下的
- 10-20：“再加一个draft task，当焦点不在paddock窗口的时候，要能识别出来现在paddock不是当前的焦点。”
- “识别出来”指用户一眼看得出来（用户选“你一眼看得出来”），不是按焦点改行为。
- 用户贴了两张截图，“我想这样”：一张不在前台（整窗去色），一张在前台。主控复述成下面“要做的”，用户：“是这个意思，改样稿吧。”
- 样稿：https://claude.ai/artifact/EuKVV21F9th9HR7ZxkBtoX（左在前台，右不在前台；变淡程度是主控估的，以截图为准）。
- 验收，用户：“同意这两条验收”。

## 先读
- 截图（墙里能读）：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/.target/p5-162-unfocused/shots/` 下 `unfocused.png`（不在前台，要的样子）和 `focused.png`（在前台，现在的样子）。不要拷进仓库。
- 现在已经在看窗口焦点的地方：`app/src/view.rs`（`window.is_window_active()` 算光标有没有键盘、失焦时取消鼠标操作）、`app/src/window.rs`（窗格标签发光）、`app/src/sidebar.rs`（`active`）、`app/src/settings_view.rs`（`observe_window_activation`）。
- 主题颜色从哪来：`app/src/theme.rs`，以及各处 `self.fg(|t| …)`、`theme.fg(…)` 的取法；终端的颜色在 `app/src/view.rs` 画格子的那层。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-162-unfocused`，分支 `p5-162-unfocused`（已从 main 建好）。动 `app/src/` 里取颜色和绘制的地方，`docs/DESIGN.md` §13 补一句。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-162-unfocused`（从 `.target/main` 克隆的）。

## 要做的
paddock 主窗口不在前台时：
1. 整个窗口去色：背景从带青色的深色变成偏中性的灰；所有文字和颜色（终端里的普通输出、彩色文件名、提示符色块，侧栏卡片、标签圈、Needs you 的琥珀色、新回复小点等）一起变淡变灰，颜色还认得出、不刺眼；不压黑，内容照样看得清。程度照截图 `unfocused.png`。
2. 终端光标变空心（现在方块光标已经会，其他形状也要看得出不在前台）。
3. 左上角红黄绿是系统画的，不用管；窗口回到前台，一切马上恢复原样。
- 怎么做（一个总的去色，还是每处各自取淡的颜色）开发定；要让以后新加的界面不用每处记得自己处理最好。
- Settings 窗口要不要也这样，开发判断，在完成记录里写明。

## 怎么算做完
- 用户同意的两条：
  1. 切到别的应用，paddock 整个窗口像截图 `unfocused.png` 那样变灰变淡，内容照样看得清；切回来马上恢复。
  2. 光标在不在前台时都对：在前台实心，不在前台空心。
- 验证只做这些：去色用的颜色换算写一条针对测试；`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各一次，加 `git diff --check`。两条验收留给用户装上看，你不起 paddock 窗口、不截图。觉得不够，在回复里说，不要自己加。

## 不要做
- 不按焦点改别的行为（通知、提醒、刷新频率等）。
- 不加依赖，不加新的主题颜色键；不改用户的主题文件和设置。
- 不把截图放进仓库。
- 不用 Python；命令里不用 `rm`、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 不起 paddock 窗口，不截图，不碰 `corral ls` 里现有的 agent，不对任何 agent `corral send`。
- 不合并到 main，不推送。只在 `p5-162-unfocused` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑，各几句话。踩过的坑指试了不行、换了办法的，工具或环境的行为和预想不一样的；没有就写「没有」，不要凑。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录
- 做了什么：
  - 新增 `app/src/unfocused.rs`：窗口不在前台（`window.is_window_active()` 为假）时，在窗口最上面盖一层中性灰 `#2b2b2b`、51% 不透明（`deferred`，优先级最高，画在所有浮层之后；没有鼠标监听，GPUI 不给它点击区域，点击照样落到下面）。一层盖全窗，所有颜色（背景、终端文字和彩色输出、提示符色块、侧栏卡片、琥珀色、小点等）一起变成约 `0.49×原色 + 22`，以后新加的界面不用各自处理。GPUI 在窗口活动状态变化时会整窗重画，所以切回前台立即去掉。
  - 主窗口（`window.rs`）和 Settings 窗口（`settings_view.rs`）的根上各加一行。
  - 右侧栏 Browser 的网页是系统的 `WKWebView`，GPUI 盖不到：不在前台时把它的 `alphaValue` 调成 0.49（`browser.rs` 的 `Page::fade`，`browser_view.rs` 每帧摆放时调），透出下面已盖灰的窗口，效果约同一个换算。
  - 光标（`view.rs`）：没有键盘时（窗口不在前台，或不是焦点窗格）不论方块、竖线、下划线都画空心方框；有键盘照原形状实心。光标滑动的目标框跟着改，免得滑动时形状对不上。
  - `docs/DESIGN.md` §13 补了 P5-162 一条。
- 验证了什么：
  - 针对测试 `unfocused::tests::the_veil_turns_the_front_windows_colours_into_the_ones_the_user_asked_for`：用从两张截图量出来的六组颜色（文字、红/紫文件名、绿、提示符黄块和蓝块），按 GPUI 的混合（BGRA8Unorm，在 sRGB 值上混）算盖灰后的颜色，每个通道和 `unfocused.png` 相差不超过 3；终端背景 `#1b2223` 变成更亮、接近中性的灰。通过。
  - `cargo fmt --check` 通过；`cargo clippy --all-targets -- -D warnings` 通过；`git diff --check` 通过。
  - `cargo test --all-targets`：818 过，1 个不过：`view::links::tests::a_shell_s_relative_paths_are_from_where_it_has_gone_to`。查了：shell 已经 `cd` 到位，但墙里 `clear` 没清屏，第一行还是敲进去的命令，5 秒后超时。这个测试只走 pty 和链接查找，和本次改动无关，是环境原因；没改这个测试。
  - 两条验收按任务文件留给用户装上看；没起 paddock 窗口、没截图。
- 拿主意的地方：
  - 一层总的盖灰，不是每处各取淡色：截图量出来正好是“往 `#2b2b2b` 混一半”（各色各通道都对得上 `0.49×c+22`），一层半透明灰就是这个换算，而且代码里很多颜色是现场拼的（`hsla(...)`、透明度叠加），逐处改漏得多。
  - 按窗口是否前台，不按 paddock 应用是否前台：和系统把红黄绿变灰同一个时机。所以 Settings 在前台时主窗口也会变灰。
  - Settings 窗口也加：同一行代码，所有 paddock 窗口看法一致。New Agent、About 两个小窗没加。
  - 未聚焦窗格的竖线、下划线光标也改成空心方框（原来只有方块会变空心），这样“没有键盘＝空心”一条规则；这在前台窗口的分屏里也看得到。
  - 截图里背景是 `#282928`，按这个换算是 `#232727`，稍暗、带一点点青；截图里的标题栏也和换算对不太上，估计和系统材质在窗口不活动时变灰有关。没为背景另加一层，以文字颜色对准为主。
- 没做的事：New Agent、About 窗口不变灰；GPUI 自己的悬停提示（tooltip）画在所有元素之后，会在盖灰层之上，不在前台时如果出现会是原色；不按焦点改任何行为。
- 踩过的坑：
  - 量截图颜色时 `magick` 没有 PNG 解码，`sips` 转格式要写系统临时目录被墙挡；改成在 scratchpad 里写了个一次性的 Rust 小程序（`png` 包，离线编译）读像素。
  - 起初模块叫 `veil`，发现代码里分屏未聚焦窗格的变暗层已经叫 veil，改名 `unfocused`，函数叫 `unfocused::veil`。

## 主控审查

10-20，paddock/main。agent 实际名字是 `paddock/dev-unfocused-1`（Claude Code opus[1m] / high，套 fence `dev` 墙）。
- diff（`main...p5-162-unfocused`）小而集中：新 `unfocused.rs`（窗口不在前台时在最上层盖一层不接鼠标的中性灰 `#2b2b2b`、51%，比例是从用户两张截图量出来的）、`window.rs` 和 `settings_view.rs` 各挂一行、内置 Browser 页面同时调透明度、`view.rs` 光标没键盘时一律空心、DESIGN 一句。「不要做」一件没犯，截图没进仓库。
- 主控墙外重跑 `cargo test --all-targets`：999 个通过、0 个失败；clippy、fmt 都过。墙里那条 pty 测试失败是老问题。
- 对方拿主意的都同意：一层总的盖灰（以后新加的界面不用各自处理）；Settings 窗口也盖灰；跟着窗口是否在最前（Settings 在前时主窗口也变灰，和系统把红黄绿变灰同一时刻）。
- 建议改（不挡合并）：New Agent、About 小窗没盖灰；GPUI 的悬停提示画在最上层，不在前台时仍是原色；同一窗口里没键盘的分屏窗格，竖线和下划线光标现在也变空心框（以前只有方块）。
- 已从分支装上，等用户照两条验收看。

## 返工 1 记录
- 改了什么：
  - 去掉那层平的盖灰（`veil`）和 Browser 网页调透明（`Page::fade`），这两处恢复成任务前的样子。
  - `app/src/unfocused.rs` 改成去色换算 `fade`：饱和度留一半、色相不变；明暗朝 96（偏暗的中间值）收 15%，深色背景略亮、文字略暗，字仍清楚；灰仍是灰；颜色的不透明度不动，所以窗口的磨砂和透明照旧，系统对磨砂材质的处理也不碰。另有 `fade_hsla`（GPUI 颜色，保留透明度）、`fade_palette`、`fade_cells`。
  - 总入口在主题：`Theme::faded()` 给一份去色副本，`fg`、`bg`、`frame`、`color`、`terminal()`（终端调色板）都给去色后的颜色；新加 `own_terminal()` 始终是设定的原色，终端报给程序的颜色（OSC 查询）改用它，不随前台变。
  - 主窗口（`window.rs`）记着 Settings 选定的主题（`chosen`），`observe_window_activation` 里失去前台换成去色副本、回到前台换回，走和 Settings 改主题同一条路（抽成 `share_theme`）：各台机器的侧栏和窗格、Browser 工具栏，以及窗口上的 New Agent、远程开 agent、任务弹框（这三个各加了 `set_theme`，New Agent 的 `restyle` 改成用窗口给的主题）。Changes、Kanban、Cairn 每帧从窗口取主题，按主题换了就重算颜色，自动跟上。`theme()` 给 Settings、About 的还是选定的原主题。
  - 终端格子（`view.rs`）：先按设定原色解析（程序自己的真彩色、改过的调色板也在内）、做完最低对比度，再整格去色；光标颜色同样去色。
  - 文字输入框（`text_input.rs`）：`Colors` 加 `faded`，窗口不在前台而颜色又不是取自去色主题时，输入框画的时候自己去色。这样终端查找栏、收件箱回复、看板搜索、工作区改名这类打开时才定颜色的输入框都跟着变，以后新加的也一样。各处造 `Colors` 的地方补了 `faded: theme.is_faded()`。
  - 宠物的像素画（`pet.rs`）画的时候按同一换算去色。
  - Settings 窗口同样记 `chosen`、失去前台换去色副本；抽出 `use_theme` 统一给所有打开的输入框换色（字段、字体搜索、加字体名、机器改名、Instructions 编辑、Dispatch 模型）；主题预览、主题卡片和色块也按当前状态去色。
  - 光标部分（没键盘一律空心方框）没动。`docs/DESIGN.md` §13 的 P5-162 一条改写成现在的做法。
- 验证：
  - 去色换算的针对测试（`unfocused::tests`）：截图里的几种颜色去色后饱和度剩一半（差不超过 1.5）、最强最弱通道不变；灰仍是灰；背景亮 3～15、文字暗 5～25，明暗顺序不变；`fade_hsla` 和 `fade` 一致、透明度不变。主题测试 `a_faded_theme_hands_out_faded_colours_and_tells_programs_the_ones_set`：七个预设主题里，去色副本的各个取色口都等于原色去色，`own_terminal()` 等于原调色板，磨砂副本只去色一次，正文对背景的对比度下降但仍 ≥ 4.5:1。先写这条测试、确认因为缺 `faded`/`own_terminal` 编不过，再实现到通过。
  - `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`git diff --check` 都过。`cargo test --all-targets --no-fail-fast`：30 个测试目标，1001 个通过、1 个失败，还是墙里 `clear` 不清屏的那条 pty 测试（`view::links::tests::a_shell_s_relative_paths_are_from_where_it_has_gone_to`）。
  - 两条验收留给用户装上看；没起 paddock 窗口、没截图。
- 拿主意的地方：
  - 去色程度：饱和度照截图量出的约一半；对比只收 15%，没照截图那样整体压暗（截图里文字从约 210 暗到约 125），因为这次要“字仍清楚利落、不压扁成一片灰”。中间值取 96，让深色主题的背景只略微抬亮（截图里的终端背景 `#1b2223` 去色后约 `#272b2b`，和截图的 `#282928` 很接近）。
  - 换主题副本而不是在每次绘制时临时开关：Changes、Kanban、Cairn、活动图这些地方按主题对象是否换了来决定重算颜色，换一份副本它们都会自然重算；临时开关会让它们缓存住错的颜色。代价是失去和回到前台时各通知一次侧栏和所有窗格。
  - Browser 网页：保持原样。它是系统的 WKWebView，显示的是网页自己的颜色；调透明会透出背后、等于压暗，用 Core Image 滤镜给系统视图去色在不能起窗口验证的情况下没把握。网页周围 paddock 自己的工具栏、地址栏、边框都会跟着去色。
  - 文字输入框用 `faded` 标记避免二次去色：窗口换主题时会重新给色的输入框拿到的是去色主题的颜色（已标 `faded`），只在打开时定色的输入框由它自己去色。
- 做不到完全统一的地方：
  - Browser 网页本身（见上）。
  - agent 种类图标里的品牌图片（Claude、Codex 等，是栅格化的图片），保持原色。
  - GPUI 的白、黑半透明叠加（阴影、高光）没去色，本来就是中性色。
  - New Agent、About 是独立小窗，没加；About 用的是选定的原主题。New Agent 独立窗口打开时取的是主窗口当时的主题。
  - 任务弹框以外，窗口上其余面板如果自己存了从主题算出的颜色、又不在 Settings 改主题那条路上，也不会跟着变；查过现有的都在这条路上或每帧取色，没发现别的。
- 踩过的坑：
  - 改 `Theme` 后 clippy 报 `terminal()` 返回了别的字段（`misnamed_getters`），把“设定的原色”那份字段改名为 `own`。
  - 上一轮的 `cargo test --all-targets` 因为库测试那条墙里失败，cargo 在第一个失败目标处就停了，其余 29 个测试目标没跑；这次加 `--no-fail-fast` 全跑了一遍。
  - 用 perl 以 `|` 作分隔符改代码时，模式里的 `\|` 变成了“或”，有一处替换错位，手工改正。

### 返工 1 补：按应用是否在前台去色
- 用户定：切到别的应用才去色；在 paddock 里弹出 Settings、New Agent 等自己的窗口时主窗口不去色，Settings 在主窗口前面时 Settings 也不去色。光标空心那条不变（看窗格有没有键盘）。
- 怎么拿“应用是否在前台”：GPUI 只给单个窗口是否最前（`is_window_active`／`observe_window_activation`），应用代理只接了启动、重开、退出等，没有激活变化。所以在 `unfocused::watch` 里问 AppKit：启动时读 `NSApplication.isActive` 定初值；向系统通知中心登记 `NSApplicationDidBecomeActiveNotification`、`NSApplicationDidResignActiveNotification`（主队列、block 观察者，`objc2` 的 `msg_send` 加 `block2`，都是已有依赖，没改 Cargo.toml）。回调里只往 `async-channel` 通道发一个布尔值，GPUI 的任务收到后改全局状态 `unfocused::Front`（`global_mut` 会通知观察者），不在 AppKit 的回调里直接动 GPUI，免得重入。`main.rs` 启动时调一次。
  - 没选“窗口焦点变化时再读 `isActive`”：Settings 在前时切到别的应用，主窗口自己的焦点没变，收不到通知；而且读的时机要赶在系统改完状态之后，靠不住。
- 改了什么：主窗口和 Settings 不再看窗口焦点，改成观察 `Front`（`observe_global_in`），建窗时也按当前状态先定一次；文字输入框和宠物画的时候看 `unfocused::in_front(cx)`。Settings 原来“回到前面重读文件”的窗口焦点观察留着，只去掉了去色。DESIGN §13 那一条改成按应用判断，写明怎么拿。
- 验证：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`git diff --check` 都过；`cargo test --all-targets --no-fail-fast`：30 个目标，1001 个通过、1 个失败，仍是墙里 `clear` 不清屏的那条 pty 测试。系统通知这部分没有单元测试（要真的切应用），留给用户装上看：切到别的应用，所有 paddock 窗口去色；开 Settings、New Agent，主窗口保持原样。
- 踩过的坑：没有。

## 主控审查（返工 1）

10-20，paddock/main。
- 返工 1 照用户两条决定改：去掉盖灰，改成主题出一份去色副本、终端格子颜色在画之前去色（饱和度留一半、明暗向偏暗的中间收 15%）；是否去色看 paddock 是不是当前应用（启动读 `NSApplication.isActive`，之后听系统的激活／失活通知，经通道交给 GPUI 更新全局状态），不再看单个窗口。光标空心照旧看窗格有没有键盘。没加依赖。
- 主控墙外重跑 `cargo test --all-targets --no-fail-fast`：1002 个通过、0 个失败；clippy、fmt 都过。
- 对方上一轮报的“818 个通过”是因为没带 `--no-fail-fast`、只跑了库测试一个目标；主控上一轮墙外重跑的 999 个全过不受影响。
- 记建议改（不挡合并）：Browser 网页本身、agent 品牌图标、New Agent／About 小窗不去色。
- 已从分支装上，等用户照验收看。

## 主控调深（10-20）
- 用户装上返工 1 后：“有点浅。”（附截图）“你直接干吧”。主控自己改 `unfocused.rs` 三个数：饱和度留 0.5 → 0.4，明暗朝的中间值 96 → 50，收的比例留 0.85 → 0.62（文字亮度约从 211 降到 150，背景约从 33 升到 39；用户截图量出来约是留 0.49、中间值 43，这次没压到那么深）。测试里“文字略暗”的范围跟着改成 40 到 80；DESIGN 那句同步。
- 墙外 `cargo test --all-targets --no-fail-fast`：1002 个通过、0 个失败（7 个预设主题正文对背景仍不低于 4.5:1）；clippy、fmt、`git diff --check` 都过。已从分支装上。

## 返工 1 的用户决定（主控 10-20 记在 main 上，合并时挪到这里）
- 用户装上看过：“灰的没有质感”。主控问往哪改，用户选“只去色，不盖灰”：不盖平的灰，每种颜色本身褪色，字仍清楚，背景磨砂透明感保留。
- 用户又问：“但是按照你的设计，settings弹出的时候是不是整个窗口也会灰掉？”主控说是、建议按应用是不是当前应用判断，用户选“切到别的应用才去色”：paddock 自己的窗口（Settings、New Agent 等）在前时主窗口不去色。
