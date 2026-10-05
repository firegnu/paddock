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
