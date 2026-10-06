# 任务：调研右侧栏 Browser 标签能不能嵌入原生网页视图，给出推荐方案

2026-10-07，paddock/main 交给 paddock/research-browser（Codex，重：gpt-6-astra / xhigh）。
路由：重 / 交叉审查不要 / 影响面：看得见（路由：档拿不准（重 0.73，置信低），活难、牵涉原生视图和 GPUI 的交界，定为重；交叉审查拿不准，只读调研、原型不合并，定不要；影响面看得见）
类型：调研
依据：本轮只查清可行性、比较方案、给推荐和风险；可以在自己的分支里做一个不合并的小原型来验证，不实现正式功能。
提示：区分已查证事实、推测和未知，给出直接依据；结论限定在本轮调查范围。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 背景
- paddock 是用 GPUI（`gpui-pre =0.3.8`，Zed 的 UI 框架快照）写的 macOS 桌面应用。右侧栏已有外壳和 Changes、Browser 两个标签（P5-13a），Browser 现在是占位。用户要的是：在右侧栏里看网页，典型场景是 agent 起的本地开发服务器（`localhost:5173` 之类）。
- 样稿（用户认可过的外壳）：`docs/设计稿/P5-右侧边栏/RightBrowser.dc.html`：后退、前进、刷新、地址栏、“在默认浏览器打开”，下面是网页本身。
- GPUI 自己不能渲染网页。paddock 已有一个往 GPUI 窗口里垫原生 AppKit 视图的先例：`app/src/frost.rs`（在窗口内容下面垫 `NSVisualEffectView`，用 `raw-window-handle` 拿到 `NSView`，`objc2 0.6`／`objc2-app-kit =0.3.2`）。注意 `Cargo.lock` 里同时有 `objc2 0.5.2` 和 0.6 两套（GPUI 依赖带进来的）。
- 用户 10-07：“明天集中精力弄brower和kanban”；同意先派调研：“可以，现在派出去调研”。

## 先读
- `AGENTS.md`「规矩」一节（尤其依赖、不复制 Zed 的 GPL 代码、系统安装、桌面窗口测试）。
- `docs/DESIGN.md` §13 里「P5-13 右侧边栏」「P5-13 样稿定下的」两条，§4 依赖隔离，§7 待定问题。
- `docs/设计稿/P5-右侧边栏/README.md`、`RightBrowser.dc.html`。
- 代码：`app/src/frost.rs`（原生视图怎么插进 GPUI 窗口、怎么跟随大小）、`app/src/right_panel.rs`（面板外壳、宽度、标签切换）、`app/src/window.rs`（右侧栏怎么挂进窗口、弹出框和 palette 画在哪一层）、`app/Cargo.toml`。
- GPUI 源码（Apache-2.0，可以读）：在 `~/.cargo/registry/src/` 下找 `gpui-pre-0.3.8` 和 `gpui-pre-platform-0.3.8`，看 macOS 窗口的 `NSView` 结构、Metal 层、事件和焦点怎么走、有没有现成的“外部视图／原生子视图”接口。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-13r-browser`，分支 `p5-13r-browser`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-13r-browser`。
- 结论写进 `docs/调研/P5-13r-Browser嵌入.md`（新建），并在本文件末尾追加完成记录。原型代码只放在这个分支里，**不会合并**；可以为原型临时加依赖、改 `Cargo.toml`／`Cargo.lock`，在结论里写清楚加了什么。

## 要查的
1. **方案比较**，至少这几种，每种写清做法、成熟度、许可、和现有依赖的版本兼容（`objc2` 0.5／0.6 两套、`objc2-app-kit =0.3.2` 已被 GPUI 锁定）、维护情况：
   - `wry`（Tauri 的网页视图库）作为子视图挂到 GPUI 窗口上；
   - 直接用 `objc2-web-kit` 建 `WKWebView`，照 `frost.rs` 的方式插进窗口；
   - 离屏渲染（比如 `WKWebView` 截图后作为图片画进 GPUI）；
   - GPUI 本身有没有可用的接口；
   - 你查到的其他方案。
2. **原生视图叠在 GPUI 上的问题**，逐条查清（能用原型验证的就验证）：
   - 层级：网页视图在 GPUI 的 Metal 层之上时，GPUI 画的弹出框、菜单、palette、悬停说明、拖动分割线时的遮罩会不会被网页盖住？有没有办法解决（比如弹出框出现时把网页截成图片临时替换、或隐藏网页视图）？
   - 裁剪和跟随：网页视图的位置和大小怎么跟着右侧栏的布局走（拖宽、加宽、收起、窗口缩放、全屏、标签切换时隐藏），圆角和边框怎么处理，会不会闪。
   - 焦点和键盘：点进网页后输入、⌘C／⌘V、⌘L（地址栏）、⌘R；paddock 自己的快捷键（⌘W、⌘T、⌥⌘B、⌘P、⌘B）在网页有焦点时还能不能用，焦点怎么在终端和网页之间交接；输入法。
   - 滚动、缩放、拖放、右键菜单、开发者工具（Web Inspector）能不能开。
   - 磨砂（`frost.rs`）和深浅色外观有没有冲突。
3. **网页本身的功能**：后退前进刷新、地址栏、加载进度和出错页、在默认浏览器打开、`localhost` 和 http（非 https）能不能打开（App Transport Security）、cookie 和存储放在哪、关掉再开是否保留。
4. **推荐**：推荐哪个方案、为什么；按工作量拆成几件可以派出去的活（先做什么、后做什么），每件的主要风险；有哪些要用户拍板的（例如加哪个依赖）。

## 怎么算做完
- 上面四条都有结论，分清“已查证（附依据：源码位置、文档链接、原型实测）”“推测”“不知道”。
- `docs/调研/P5-13r-Browser嵌入.md` 写完并提交在分支上。
- 验证只做这些：读源码和文档；做一个最小原型（在右侧栏 Browser 区域显示一个本地页面就够），用 `PADDOCK_NO_ACTIVATE=1`、临时 HOME、假 corral 起自己的窗口截几张图（截图放 scratchpad，不入库）。键盘、焦点这类需要真实交互的，写清楚哪些是推测、要用户实测。觉得不够，在回复里说，不要自己加。

## 不要做
- 不合并、不推送；原型不进 main。不改 main 上的任何文件。
- 不复制 Zed 应用层（`terminal`、`terminal_view`、`ui` 等，GPL）的代码；GPUI 本身可以读。参考的外部代码先核对许可。
- 不安装系统组件或工具链（Xcode 组件、新的 rustup 工具链等）；需要的话停下来报告。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件和正在运行的 paddock。原型要起本地页面时，用一个临时的静态文件或自己起的本地服务，用完按 PID 停掉。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。不读 corral 的内部状态目录。
- 不用 Python。写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到需要用户决定或需要权限才能继续的，停下来报告。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：查到了什么（分已查证、推测、不知道）、推荐、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和要主控或用户决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
