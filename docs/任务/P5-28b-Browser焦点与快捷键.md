# 任务：Browser 的焦点和快捷键交接（网页、终端、地址栏之间）

2026-10-07，paddock/main 交给 paddock/dev-browser-keys（Claude Code，重：opus[1m] / xhigh）。
路由：重 / 交叉审查不要 / 影响面：改行为（路由：重（1.0）；交叉审查拿不准，主控定不要（不碰数据、并发、权限，误粘贴风险靠测试和用户实测把关）；影响面拿不准，定改行为）
类型：功能变更
依据：本轮只做键盘归属：谁拿键盘、快捷键给谁、怎么交接；工具栏、布局、打包的网络设置、弹窗不在本轮。
依赖：P5-28a
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」一节，以及「开发方式」里的看板约定。
- `docs/DESIGN.md` §13「P5-28 右侧栏 Browser（WKWebView）」第 4 条（快捷键分配，用户定的）。
- 调研 `docs/调研/P5-13r-Browser嵌入.md` §4.3（两套焦点：GPUI 的 focus 和 AppKit 的 firstResponder；`performKeyEquivalent:`；建议的做法和风险）。
- `docs/任务/P5-28a-Browser原生宿主与工具栏.md` 的完成记录和主控审查（“给 P5-28b”的几条）。
- 代码：`app/src/browser.rs`（原生宿主、`cover()`、隐藏时交还键盘）、`app/src/browser_view.rs`（地址栏）、`app/src/menu.rs`（`bindings`、菜单）、`app/src/view.rs`（终端的 copy／paste）、`app/src/window.rs`（焦点、`focus_pane`）。GPUI 源码（Apache-2.0）里 macOS 窗口的 `performKeyEquivalent:`、`handle_key_event`、菜单快捷键。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-28b-browser-keys`，分支 `p5-28b-browser-keys`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-28b-browser-keys`。
- 可以改：`browser.rs`、`browser_view.rs`、`menu.rs`、`window.rs`、`view.rs`（只改 copy／paste 的去向判断）；需要的 `objc2-app-kit`／`objc2-web-kit` 特性可以加（不加新库）。

## 要做的
用户原话（P5-28 第 4 条，用户接受主控建议）：⌘W、⌘T、⌥⌘B、⌘P、⌘B 保持 paddock 原意；网页里加 ⌘L 聚焦地址栏、⌘R 刷新；⌘C／⌘V 给当前正在编辑的地方（网页有焦点就给网页，不能落到终端）。

1. **一个“现在键盘归谁”的统一记录**：终端（GPUI 视图里的某个窗格）、地址栏、网页三者之一。点网页、点终端、点地址栏、⌘L、网页隐藏和重新显示、切窗格、切标签、关右侧栏时都要更新，并且让 AppKit 的 firstResponder 和 GPUI 的 focus 一致（交还 paddock 时先 `makeFirstResponder` 再更新 GPUI focus；判断网页是否拿着键盘要看 firstResponder 是否在网页的视图子树里，不能只比较是否等于 WKWebView 本身）。
2. **快捷键**：
   - ⌘W、⌘T、⌥⌘B、⌘P、⌘B（以及 ⌘⇧P、⌘D 等 paddock 菜单里已有的）在网页有键盘时照样生效，而且只触发一次。
   - ⌘L：聚焦地址栏并全选；⌘R：刷新网页（网页或地址栏有键盘时）。
   - ⌘C、⌘V、⌘X、⌘A、⌘Z：网页有键盘时交给网页的标准编辑；地址栏有键盘时给地址栏；终端有键盘时照旧给终端。**不能**网页有键盘时粘贴进终端。
   - 网页自己的快捷键（比如页面里的 ⌘K 搜索框）在没有和上面冲突时照常交给网页。
3. **P5-28a 留下的几条**：地址栏输入到一半去点网页，地址栏的光标要消失（不能显示还在编辑而键盘已在网页）；菜单弹出时点网页要关掉菜单；网页因悬停说明暂时隐藏又出现后，若之前键盘在网页，恢复给网页（P5-28a 的取舍是隐藏时交还，这里补上恢复）。
4. **输入法**：网页里中文输入（组字、候选、Esc 取消、组字中切到终端）不能被 paddock 截走或打断；终端的输入法照旧。

## 怎么算做完
- 上面四条达到。
- 验证只做这些：
  - 测试：把“键盘归谁”的状态变化和“某个快捷键在某种归属下交给谁”做成可测的纯逻辑，写测试覆盖上面列的快捷键和交接情形。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 用你编出来的 release 程序起一次自己的窗口（`PADDOCK_NO_ACTIVATE=1`、临时 HOME、假 corral），确认能启动、Browser 能显示页面，截一张。**真实的键盘、输入法、点击都不能模拟**，在完成记录里写一份给用户的手动检查清单（每条：怎么操作、应该看到什么），主控会交给用户试。
  - 觉得不够，在回复里说，不要自己加。

## 不要做
- 不改工具栏样子、布局跟随、浮层隐藏的判断（`cover()` 的登记照旧）；不做 ATS、网站数据、弹窗、Inspector（P5-28c）。不加新库，不改 `gpui-pre-*`。
- 不把网页收到的所有按键都转给 GPUI（会破坏网页编辑和输入法）；只截 paddock 自己要的那几个快捷键。
- 不复制 Zed 应用层（GPL）的代码。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件、网站数据和正在运行的 paddock。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。不读 corral 的内部状态目录。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的（比如某个快捷键在网页有键盘时根本收不到），停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-28b-browser-keys` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、给用户的手动检查清单，各几句话。回复里只写这几样和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
