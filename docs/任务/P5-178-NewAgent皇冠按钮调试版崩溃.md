# 任务：修调试版里 Controller 开着时打开 New Agent 就崩

2026-10-23，paddock/main 交给 paddock/dev-hover-crash（omp，默认模型，用户点名）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.88）、不要、拿不准；用户点名 omp，不按常规档换家；影响面主控定：调试版会崩，按改行为）
类型：Bug 修复
依据：P5-177 的 agent 起测试窗口时发现，P5-102（提交 4c2a52c）起就有。本轮只修这个崩溃，New Agent 的外观和行为不变。
提示：依据证据定位并修复导致问题的原因，保持无关行为不变。
执行：派出
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- “崩溃那个派 omp 修”

## 现象和原因（P5-177 的 agent 用 backtrace 定位的）
- 调试版（`cargo build`／`cargo run`，不带 `--release`）里，New Agent 面板在 Controller 开着时一打开就崩：GPUI 的 `debug_assert!` 报 “hover style already set”。
- 原因：`app/src/new_agent_view.rs` 的 `foot()` 里，Controller 皇冠按钮（`chip-role`）用 `icon_button` → `ghost()` 做，`ghost()` 已经设了 `.hover(…)`（浅灰底），Controller 开着时 `.when(controller, …)` 又调了一次 `.hover(move |style| style.bg(accent.opacity(0.2)))`。
- 发布版里 `debug_assert` 不生效，后一个 hover 覆盖前一个，所以 Controller 开着时悬停是强调色底。用户装的是发布版，不受影响；受影响的是用调试版起测试窗口的开发。

## 先读
- `app/src/new_agent_view.rs` 的 `ghost()`、`foot()`（`icon_button`、`chip-role` 那段）。
- GPUI 的 `.hover()` 和那条 `debug_assert`：在 `~/.cargo/registry/src/` 下的 `gpui-pre-0.3.8` 里搜 “hover style already set”，只读确认接口。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-178-hover-crash`，分支 `p5-178-hover-crash`（已从 main 建好）。只动 `app/src/new_agent_view.rs`，加上本文件的完成记录。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-178-hover-crash`（已从 `.target/main` 克隆）。命令在 `app/` 下跑（仓库根没有 `Cargo.toml`）。

## 要做的
1. 让每个元素的 hover 只设一次：Controller 开着时皇冠按钮悬停仍是 `accent.opacity(0.2)` 底，关着时仍是 `ghost()` 原来的悬停（浅灰底、字变正文色），和发布版现在的样子一样。改法自己定（比如给 `ghost()` 传悬停底，或者这个按钮不走 `ghost()` 的 hover），不给 GPUI 打补丁。
2. 同一个文件里别的地方有没有同样“`ghost()` 或别的已设 hover 的元素又调 `.hover()`”的，一并查；有就照同样改，没有在完成记录里写“查过，没有”。

## 怎么算做完
- 用户原话：“崩溃那个派 omp 修”——调试版里 Controller 开着时打开 New Agent 不再崩，悬停颜色和发布版现在一样。
- 验证只做这些：
  - 先复现：调试版起一个测试窗口，Controller 开着时打开 New Agent，确认崩、报 “hover style already set”；修完同样操作确认不崩。打开 New Agent 可以临时在 `main.rs` 加一个环境变量钩子（P5-177 的 agent 就是这么做的），提交前删掉，diff 里不能有 `main.rs`。
  - `git diff --check`；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。
  - 仓库里没有 GPUI 的界面测试设施，不为此加 `test-support` 或新依赖；不另写单元测试。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改按钮的颜色、尺寸、图标、提示文字和点击行为；不动 New Agent 以外的文件（临时钩子除外，提交前删）。
- 不加新依赖，不改 `gpui-pre-*` 版本，不改 Cargo 文件；不改用户本机的 `~/.config/paddock/config.toml`。
- 不复制 Zed 的 GPL 代码。
- 起测试窗口时：用一个临时 HOME 放测试用的 `.config/paddock/config.toml`，加 `PADDOCK_NO_ACTIVATE=1`，`CORRAL_HOME` 指向一个空的临时目录（临时 HOME 隔离不了 corral），记下 PID，用完按 PID 停；要截图只截这个测试窗口（`GPUI_TERM_WINDOW_ID=1` 拿编号，`screencapture -x -o -l <编号>`），截图放 worktree 外、不入库；不用脚本模拟键盘鼠标。
- 不用 Python（包括临时批量改文件）；命令里不用 `rm`（临时文件放 worktree 外的临时目录，留着不删）、不用 `sh -c` 包长命令、不用 `sed -i`、`perl -i` 改文件；前台命令别带等标准输入的东西。改文件用编辑工具。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名。
- 不碰 `corral ls` 里现有的 agent，不对任何 agent `corral send`。
- 碰到要拿主意的事（比如改了会让悬停颜色和现在不一样），停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等，不要自己换别的做法。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-178-hover-crash` 上提交。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：做了什么、验证了什么（复现前后各是什么结果）、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具或环境的行为和预想不一样的；没有就写「没有」），在你的分支里提交。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- 做了什么：
  - `ghost()` 增加 `ground: Option<Hsla>`：悬停底色由这里一次设好，`None` 是原来的浅灰底并让字变正文色，`Some` 只用给的底色（与发布版里第二次 `hover` 整体覆盖第一次的效果一致）。所有调用点按其原有外观传 `None`。
  - `chip-role` 不再在自己的 `.when` 里第二次调用 `.hover`：Controller 开着时把 `accent.opacity(0.2)` 作为 `ground` 传进 `ghost()`，关着时传 `None`，静态底色 `accent.opacity(0.14)` 仍在 `.when` 里设。尺寸、图标、提示文字、点击行为未动。
  - 同文件 `chip-sandbox` 的 `Some(profile)` 分支也有同样问题（`ghost()` 已设 hover，又调 `.hover(color.opacity(0.2))`），照同样办法改成 `base(Some(color.opacity(0.2)))`，颜色和原来一样。
  - 逐处查了本文件所有 `.hover(`：除上述两处外，其余都是在未选中分支上对同一元素只设一次；`menu_row`、预设 pill、effort 标签、split tile 等都没有第二次调用。
- 验证了什么：
  - 复现（修前）：调试版（`cargo build`）临时 HOME、`CORRAL_HOME` 指向空临时目录、`PADDOCK_NO_ACTIVATE=1`、`main.rs` 里临时 `PADDOCK_TEST_NEW_AGENT` 钩子打开 New Agent（默认 Controller 开着），panic：`hover style already set`，位置 `gpui-pre-0.3.8/src/elements/div.rs:843`，进程以 134 退出。
  - 修复后同样命令运行 12 秒不再崩（`timeout` 返回 124，说明窗口一直活着）；临时钩子已删，`git diff` 只有 `app/src/new_agent_view.rs`。
  - `git diff --check` 通过；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次，均通过。
- 拿主意的地方：`ground` 只替换悬停底色，`Some` 时不再设 `text_color`，因为发布版里第二个 `hover` 会把第一个整体覆盖；皇冠按钮唯一子节点是明确着色的图标，沙盒按钮的文字有自己的 `text_color`，所以悬停观感与发布版一致。悬停颜色本身无法在不让脚本模拟鼠标的前提下实测（任务禁止模拟输入），依据是代码路径与发布版覆盖后的样式相同。
- 没做的事：没有加单元测试或 `test-support`（任务明确不写）；没有截图；没有改动 New Agent 以外的文件。
- 踩过的坑：`env PATH=...` 直接传未加引号的 `$PATH` 时，路径里的空格（Fusion.app 那段）被 `env` 拆成额外参数，命令以 127 退出；加引号后正常。另：panic 之后 GPUI 报 `failed to initiate panic` 并以 134 退出，判断是否崩溃要看 stderr 里的 assert，不能只看退出码。

## 主控审查

- diff 只动了 `app/src/new_agent_view.rs` 和本文件，没有 `main.rs`。`ghost()` 多一个悬停底参数，皇冠按钮（Controller 开着）和沙盒按钮（选了档案）把各自原来第二次 hover 的底色传进去，hover 每个元素只设一次。
- 核对 GPUI `div.rs:841`：`hover()` 是整个替换 `hover_style`，发布版里第二次调用只留背景色、不留字色；agent 的 `Some` 分支只设背景，和发布版现在的样子一致，用户看不出变化。
- 沙盒按钮同一种问题，在任务第 2 条范围内，同意一起修。
- 主控在分支上重跑 `cargo test --all-targets --no-fail-fast`：30 个目标，1034 passed，0 failed。复现前后崩不崩以 agent 的记录为准（主控不起测试窗口）。
结果：一次过

## 用户验收

2026-10-23 用户：“合并吧”。正式版外观不变，没重装。
