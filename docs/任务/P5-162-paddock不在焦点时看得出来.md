# 任务：paddock 窗口不在前台时整个窗口去色，一眼看得出来

2026-10-20 用户提出，记成草稿；同天用户说“开始做 P5-162.”，主控逐条问定，用户同意验收。paddock/main 交给 paddock/dev-unfocused（Claude Code，opus[1m] / high，套 fence `dev` 墙）。
依据：用户 10-20：“再加一个draft task，当焦点不在paddock窗口的时候，要能识别出来现在paddock不是当前的焦点。”本轮只改显示，不按焦点改别的行为。
待用户：重开 paddock，切到别的应用再切回来，照两条验收看（整窗变灰变淡、切回马上恢复；光标在前台实心、不在前台空心）
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
