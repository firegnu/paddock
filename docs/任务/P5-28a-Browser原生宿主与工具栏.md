# 任务：右侧栏 Browser 接入 WKWebView：原生宿主、布局跟随、浮层时隐藏、样稿工具栏

2026-10-07，paddock/main 交给 paddock/dev-browser（Claude Code，重：opus[1m] / xhigh）。
路由：重 / 交叉审查不要 / 影响面：改行为（路由：档拿不准（重 0.53、常规 0.47），新的原生视图生命周期、跨 window／right_panel 和浮层，定为重；交叉审查拿不准，不碰数据模型、并发和安全规则，定不要；影响面拿不准，定改行为）
类型：功能变更
依据：本轮把 Browser 做到能用：嵌入、跟随、隐藏、工具栏和导航状态；焦点和快捷键的完整交接（P5-28b）、打包的 ATS 和网站数据策略、弹窗和 Inspector（P5-28c）不在本轮。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」一节（编译目录、依赖、桌面窗口测试、`PADDOCK_NO_ACTIVATE`、截图不入库）。
- `docs/DESIGN.md` §13 最后一条「P5-28 右侧栏 Browser（WKWebView）」（用户定的五条和拆活）。
- **调研报告** `docs/调研/P5-13r-Browser嵌入.md`：第 1、3 节（为什么用 `objc2-web-kit`、不用 `wry`）、第 4.1、4.2 节（层级、隐藏、坐标换算、bounds 从布局来）、第 5 节（导航接口、委托）、第 6 节（原型怎么做的）。
- 原型代码：本地分支 `p5-13r-browser` 的提交 `9422921`（`git show 9422921`），`app/src/browser_probe.rs` 和它在 `window.rs`、`right_panel.rs` 里的接线；只作参考，正式代码自己写干净。
- 样稿：`docs/设计稿/P5-右侧边栏/Right.dc.html`（`start` 取 browser 时的工具栏：后退、前进、刷新、地址栏、在默认浏览器打开，下面是网页）、`RightBrowser.dc.html`。
- 代码：`app/src/frost.rs`（原生视图插进 GPUI 窗口的已有做法）、`app/src/right_panel.rs`（Browser 标签现在的占位、宽度、加宽、`Saved`）、`app/src/window.rs`（右侧栏接线、palette、各种弹出框和悬停说明、拖动分割线）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-28a-browser`，分支 `p5-28a-browser`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-28a-browser`。
- 可以改：`app/Cargo.toml`／`Cargo.lock`（只加 `objc2-web-kit =0.3.2`、把 `objc2-foundation =0.3.2` 写成直接依赖，特性按需最少）、新建 Browser 模块（原生宿主一个、工具栏视图一个，名字你定）、`right_panel.rs`、`window.rs`、`lib.rs`、`layout_state.rs`（如要保存当前网址）、`footer_icon.rs`（工具栏图标）。

## 要做的
用户原话：“明天集中精力弄brower和kanban”；“暂时不要，还是优先webview。你的5条建议我都接受。”（五条见 DESIGN P5-28。）

1. **原生宿主**：第一次切到 Browser 标签时，在当前窗口建一个 `WKWebView`（每个窗口一个），作为 GPUI 视图上方的原生兄弟视图；用默认的持久网站数据；创建时不得把应用抢到前台（遵守 `PADDOCK_NO_ACTIVATE` 的精神：不调用 activate 一类）。窗口关闭时正确释放。
2. **跟随布局**：网页的位置和大小取自右侧栏 Browser 内容区的实际布局（扣掉工具栏和边距），换算成 AppKit 坐标；拖宽、加宽、窗口缩放、全屏、标题栏高度变化时跟着走。网页区域和面板的圆角、边框协调（原生层的圆角裁剪能做就做，做不到写进完成记录）。
3. **隐藏**：右侧栏收起、切到 Changes、内容区为空时隐藏（`setHidden`，不销毁，保留页面和历史）；palette、标题栏和侧栏的弹出框、菜单、悬停说明、Attention 列表等任何 GPUI 浮层出现时，如果和网页区域相交就隐藏网页；拖动分割线期间隐藏。做成**一个统一的判断**（“现在网页该不该显示”），新加浮层时只需登记一处，不要在各处零散地隐藏。
4. **工具栏（照样稿）**：后退、前进（不可用时变淡）、刷新（加载中变成停止）、地址栏（显示当前页面的最终网址；回车跳转；没写协议时，`localhost`／回环地址补 `http://`，其他补 `https://`）、“在默认浏览器打开”（打开当前页面，不是地址栏里没提交的字）；加载进度（细线或类似的安静样式）；加载失败时网页区域显示安静的错误状态（一行标题、一行原因、Retry），HTTP 404/500 照常显示网页本身。还没打开过任何网址时，显示一个安静的空状态（图标、一行标题如 “Open a page”、地址栏可输入）。
5. **焦点（本轮只做最基本的）**：点网页时网页拿到键盘；点终端或地址栏时键盘回到 paddock（`makeFirstResponder` 交还给 GPUI 的视图）。完整的快捷键路由、⌘C／⌘V 不落到终端、输入法是 P5-28b，本轮发现的问题写进完成记录，不必解决。
6. **保存**：右侧栏 `Saved` 里记住最后的网址（照 `sidebar_collapsed` 的做法用 serde 默认值，旧布局文件读进来默认空），重开时 Browser 标签加载它。
7. 界面文字一律英文；只从主题取色、可加透明度，不新增主题颜色键；字号用 `ui.px`；三套预置主题、界面字号 13 和 18 下工具栏都好看、不折行。

## 怎么算做完
- 上面七条达到；在右侧栏里能打开本地开发服务器的页面并正常使用。
- 验证只做这些：
  - 测试：地址规范化（补协议、localhost／回环判断）、“网页该不该显示”的判断（各种浮层、拖动、收起、切标签的组合）、`Saved` 旧布局文件读入默认值、坐标换算。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 截图：用你编出来的 release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`、假 corral、预写的布局文件（右侧栏开在 Browser、记住的网址指向你自己起的临时本地服务），`--bounds` 设窗口大小，截几张：窄面板显示页面、加宽、空状态、加载失败（指向一个没人监听的端口）。本地服务用完按 PID 停掉。截图放 scratchpad，不入库，自己看过。
  - 点击、键盘、输入法、拖宽时会不会闪、浮层出现时的隐藏，都留给用户实际用，写进完成记录。
  - 觉得不够，在回复里说，不要自己加。

## 不要做
- 不做完整的快捷键路由、⌘L／⌘R（P5-28b）；不改打包脚本的 ATS 项、不做弹窗／文件选择／新窗口策略、不开 Inspector（P5-28c）。不用 `wry`、Servo 或别的网页库。不改 Changes 标签、左侧栏、磨砂。
- 只加上面说的依赖；`gpui-pre-*` 不动，不升级已有的 `objc2` 0.5／0.6 依赖链。
- 不复制 Zed 应用层（GPL）的代码。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件、网站数据和正在运行的 paddock。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。不读 corral 的内部状态目录。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的（比如浮层隐藏在某处接不上、原生层圆角裁剪做不了），停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-28a-browser` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事（含给 P5-28b、P5-28c 的发现），各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
