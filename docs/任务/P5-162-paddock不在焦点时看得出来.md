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
