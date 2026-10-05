# 任务：New Agent 和 About 两个窗口照样稿重新排版

2026-10-06，paddock/main 交给 paddock/dev-newagent（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：看得见（路由：常规（0.94）；交叉审查不要；影响面拿不准（看得见 0.56），只改显示，定为看得见）
类型：样式／文案调整
依据：本轮只改这两个窗口的样子；表单的字段、校验、启动 agent 的命令和放到哪里的逻辑都不变。设置窗口、窗口内弹出框、侧栏由别的活在做。
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」一节（编译目录、桌面窗口测试、`PADDOCK_NO_ACTIVATE`、不模拟按键）。
- `docs/DESIGN.md` §13 开头的原则，以及末尾「P5-4、P5-10、P5-11 样稿定下的」一条。
- 样稿：`docs/设计稿/P5-弹出界面/README.md`、`NewAgent.dc.html`、`About.dc.html`。
- 现在的代码：`app/src/new_agent_view.rs`、`app/src/new_agent.rs`（只读，看字段和 `preview`）、`app/src/about.rs`、`app/src/icon.rs`（只读，app 图标怎么画的）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-4a-windows`，分支 `p5-4a-windows`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-4a-windows`。
- **只改** `new_agent_view.rs`、`about.rs`。窗口大小如果在 `windows.rs` 里定、需要改，只改那两个窗口的尺寸。
- **并行任务**：paddock/dev-sidebar 在 `p5-10-sidebar` 改 `sidebar.rs`、`window.rs`、`menu.rs`、`footer_icon.rs`、`layout_state.rs`，新建 `popover.rs`；paddock/dev-settings 在 `p5-4b-settings` 改 `settings_view.rs`、`settings.rs`。这些你都不要动。

## 要做的
1. **New Agent**（照 `NewAgent.dc.html`）：
   - 标签在左、右对齐，控件在右，行距一致。
   - Agent、Role、Open in 用分段按钮（一个淡底框里几段，选中的一段亮底）；Agent 两段前面带种类颜色的小圆点。
   - Open in 三段：Current pane、New tab、Split；选 Split 时右边出现四个方向小图块（左右上下，画一个小方框、亮起对应的一半），对应现在的 ←→↑↓ 四个选项。现有的 6 个放置选项一个都不少，只是换了排法。
   - 前缀和名字两个输入框中间一个淡色 `/`；Controller 时名字固定是 `main`、变灰不能改（和现在的规则一样）。
   - Advanced 可以展开收起，箭头转动；里面是 Command 和 First message。
   - 底部一个深色小区块，标题小号大写 `WILL RUN`，下面用等宽字体写现在 `preview` 给出的完整命令（P5 留下的建议：这里的字体用等宽）。
   - 右下 Cancel（无底色）和 Create ⌘↩（主题强调色底的主按钮）；正在启动时的提示和出错提示保留，放在按钮左边。
2. **About**（照 `About.dc.html`）：顶上 app 图标（用 `icon.rs` 现有的画法，约 84pt 的圆角方块），下面 paddock、Version、一句介绍、一条短线、来源说明小字，居中；文字内容和现在一样。界面字号大（如 18）时也不挤、不出界（P5 留下的建议）。
3. 界面文字一律英文，沿用现有措辞；只从主题取色，可以加透明度，不新增主题颜色键；字号用 `ui.px`；三套预置主题、界面字号 13 和 18 下都要好看、不折行。

## 怎么算做完
- 用户原话：“主窗体点击某些按钮弹出的界面，我觉得这些界面也要重新设计……然后也要细调并作的优雅精细。”——这两个窗口和样稿一致，上面三条达到。
- 验证只做这些：`git diff --check`；在 `app/` 下跑一次 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`；用 `PADDOCK_NO_ACTIVATE=1`、临时 HOME 和假 corral 起自己的窗口，各截一张（New Agent 的 Advanced 展开时一张；截图不入库，窗口在后面不重画时写进完成记录）。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 `new_agent.rs` 的表单、校验、命令拼法，不改 agent 放到哪里的逻辑；不改别的窗口。
- 不新增主题颜色键，不加依赖，不改 `Cargo.lock`。
- 不要用 `osascript`、System Events 等任何方式模拟按键或鼠标。截图只截自己开的窗口，启动时加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral。不要在 New Agent 里真的点 Create 启动 agent。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-4a-windows` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- **做了什么**：
  - `new_agent_view.rs` 照 `NewAgent.dc.html` 重排：标签在左、右对齐（96pt 一列，随界面字号放大），控件在右，行距一致（14pt）；Agent（带种类颜色小圆点，取主题 `claude`/`codex`）、Role、Open in 都换成分段按钮（淡底框、选中段亮底）；Open in 三段 Current pane / New tab / Split，选 Split 时右边出现四个方向小图块（小方框亮起对应一半），六个放置选项都在；从别处点 Split 默认 Right，已在 Split 时保留原方向。前缀和名字中间淡色 `/`；Controller 时名字是变灰、不能改的 `main`。Advanced 可展开收起，箭头 150ms 转动（只在手动点过之后才转，打开窗口时不转）；里面 Command（等宽）和 First message。底部深色区块 `WILL RUN` 加等宽字体的 `preview` 完整命令，平时就显示（样稿如此，原先只在 Advanced 里）。右下 Cancel（无底色）和 Create ⌘↩（`agents_accent` 底）；启动中提示和出错提示在按钮左边。正在输入的框边线淡淡亮起，出问题的框标红（原有）。
  - `about.rs` 照 `About.dc.html`：顶上 84pt 圆角方块的 app 图标，照 `icon.rs` 的算法画（站姿的猫裁到自身边界、圆角 0.225、整数倍像素、略低于中心），下面 paddock、Version、介绍、短线、来源小字，居中，文字和原来一样。
  - `windows.rs` 只改这两个窗口的尺寸：New Agent 580×600、About 400×330，都乘界面字号比例（13 为 1），字号 18 时窗口跟着放大，不挤不出界。
- **验证了什么**：`git diff --check`、`cargo fmt --check`；`app/` 下 `cargo test --all-targets`（库 154 项及集成测试）全部通过，`cargo clippy --all-targets -- -D warnings` 无警告（只有上游 `block v0.1.6` 的未来兼容提示）；`Cargo.lock` 没变。截图用临时 HOME、临时 `XDG_STATE_HOME`、假 corral（只回 `{"agents":[]}`）、`PADDOCK_NO_ACTIVATE=1`，只截自己的窗口，不入库：New Agent（Advanced 展开、Split → Right）和 About 各在 Dune 13、Dune 18、Tide 13、Lagoon 13 下一张，共 8 张。窗口在后面也画出了完整内容。为了不按键就能打开窗口并展开 Advanced，截图期间临时在 `main.rs` 和 `NewAgentView::new` 里加了按环境变量打开的代码，截完已删掉，不在提交里（沿用 P5-3 的做法）。
- **拿主意的地方**：
  - 输入框、分段框、WILL RUN 的深色底：三套预置主题里终端背景都等于 `agents_bg`，没有更深的主题色，所以用黑色加透明度（0.3／0.22）压暗面板色，和 `window.rs` 的遮罩、阴影同一做法；About 图标的阴影也用黑色加透明度。其余颜色都从主题取：标签 `agents_dim`，次要文字 `agents_branch`，更淡的 `agents_dimmer`，边线 `agents_rule`/`agents_border`，选中 `agent_selected`，强调 `agents_accent`。
  - 目录候选（paddock 的目录和各 agent 的目录）样稿里没画，功能保留：放在 Project 框下面一行小号等宽字，安静样式（无边框，选中的亮底），路径太长时省略开头，留住项目名。
  - Agent 两段按样稿 Claude 在前、Codex 在后；默认仍是 Codex（表单逻辑没动）。
  - 名字旁原来的小字 “a controller is always main” 去掉了，样稿没有，变灰不能改的 `main` 已经说明。
  - First message 仍是单行框，和其他框一样高：样稿画的是多行高框，但 `text_input.rs` 只支持单行，不在本件范围。
  - WILL RUN 用一种颜色显示整条命令：样稿里名字和命令分了颜色，但 `preview` 只给一整串，要分色就得在视图里重新拆命令，没做。
  - 字母间距（`WILL RUN` 的 letter-spacing）GPUI 不支持，没做。
- **没做的事**：
  - 窗口标题栏：样稿里标题栏和窗口内容同色、连成一片；现在这两个窗口用的是系统标题栏（深灰）。要做成样稿那样，得在 `windows.rs` 给这两个窗口开 `appears_transparent` 并自己画标题、留出红绿灯位置，超出“只改尺寸”的范围，**留给主控决定**。
  - `new_agent.rs` 的 `Place::ALL` 和 `Place::label()` 视图不再用了（是 `pub` 项，不报警告）；按要求没动 `new_agent.rs`，要不要删由主控定。
  - 键盘、鼠标都没实际操作过，留给用户试：各分段和方向图块的点击与悬停、Advanced 的箭头转动、输入框聚焦时的边线、Choose…、目录候选的点击、出错时的提示位置。没有真的点 Create。

## 返工

主控决定（10-06）：先合并 main（P5-4b 设置窗口已合并）；New Agent、Settings、About 三个窗口照样稿改成透明标题栏，红绿灯和窗口内容连成一片，窗口仍能拖动；删掉没人用的 `Place::ALL`、`Place::label`；界面字号 18 下红绿灯不压到内容。

- **做了什么**：
  - `git merge main`，没有冲突（`windows.rs` 里 Settings 的 960×720 保留）。
  - `windows.rs`：`options()` 加了标题栏行高这个参数，三个窗口都用透明标题栏（`appears_transparent`），红绿灯放在该行竖直居中（沿用 `window::traffic_lights`）；新加 `title_bar(base, ui)`，算法和主窗口一样：随界面字号变高，不低于基准高度。三个视图各自定基准：New Agent 38、About 34、Settings 46（就是它原来页面标题那一行的高度）。`app_owns_titlebar_drag` 保持默认 false，拖动交给 AppKit 原生的顶部标题栏区域；三个窗口顶部那一条里都没有可点的控件。New Agent 窗口高 600 改成 630、About 330 改成 350，补上标题行占去的高度。
  - 三个视图每次重画都按当前界面字号重新放一次红绿灯：在 Settings 里改了界面字号并保存后，打开着的窗口红绿灯仍然在顶行居中。
  - New Agent 顶部一行，标题 “New Agent” 居中，小号、淡色（`agents_dim`），表单上边距从 18 改成 10。About 顶部只留一行空位，没有标题。Settings 只改顶部：导航顶部留出标题栏行高（原来 12），红绿灯落在导航上方；右边页面标题那一行用同一个高度，和红绿灯对齐。
  - `new_agent.rs` 删掉 `Place::ALL` 和 `Place::label()`；测试里没有用到它们，所以测试没改。
- **验证了什么**：`git diff --check`、`cargo fmt --check`；`app/` 下 `cargo test --all-targets`（库 154 项及集成测试）全部通过，`cargo clippy --all-targets -- -D warnings` 无警告（只有上游 `block v0.1.6` 的未来兼容提示）。截图用临时 HOME、假 corral、`PADDOCK_NO_ACTIVATE=1`，只截自己的窗口，不入库：三个窗口在 Dune 13 和 Dune 18 下各一张，共 6 张。红绿灯都落在各自顶行的正中，字号 18 下也不压到内容；Settings 的红绿灯在导航上方，和页面标题同一行。截图期间临时加的打开窗口代码已删，不在提交里。
- **拿主意的地方**：拖动交给 AppKit（主窗口是自己处理拖动，这三个窗口没有标签栏，不需要）；红绿灯每次重画都重新放一次，不另加状态记录，Settings 只能改顶部，也就加不了这个状态。
- **没做的事**：没有用鼠标试过拖动、双击标题栏缩放、红绿灯本身的点击；截图里的红绿灯是灰的，因为测试窗口不在前台，这是系统的正常样子。这些留给用户实际看。
