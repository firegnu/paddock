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

## 完成记录

- **做了什么**
  - 统一记录 `browser::Keys`（纯逻辑）：键盘归 `Paddock`（窗格或 paddock 别的东西）、`Address`、`Page` 之一。`settle(Seen) -> Moves` 每帧（摆放网页之后）和每 50ms 轮询时跑一次：比较 GPUI 焦点、AppKit firstResponder 是否在网页视图子树里、网页显示状态（`Shown`／暂时 `Hidden`／`Away`）、有没有弹出框，和上次自己留下的状态，判断是谁动了，再给出让两边一致的动作。点网页 → GPUI 焦点跟到网页；GPUI 焦点移走（点终端、点地址栏、⌘L、切窗格、切标签）→ AppKit 键盘交回 GPUI 的视图；网页暂时隐藏（悬停说明、拖分割线、错误状态）→ 交回，重新显示且之前在网页 → 还给网页；关右侧栏或切到 Changes／Kanban → 交给当前窗格；弹出框打开时借走网页的键盘，Esc 关掉后还回网页，选了别的（开 agent、新 shell）就留在那里；弹出框开着时点网页 → 关掉弹出框。
  - GPUI 一侧：`BrowserView` 有自己的网页焦点句柄（挂在网页区域和错误状态上），网页有键盘时 GPUI 焦点就在它上面，于是终端的光标变空心、地址栏的光标消失；`BrowserView` 根上是 `PaddockBrowser` 按键上下文，⌘L（`FocusAddress`）、⌘R（`ReloadPage`）只绑在这个上下文里，终端有键盘时不变。⌘L 先交还 AppKit 键盘再聚焦地址栏，等输入框画出来后全选。⏎ 和 Esc 都把键盘还给网页（Esc 以前是 blur）。⌘R 刷新，失败状态下重试。
  - AppKit 一侧：网页改用 `WKWebView` 的子类 `PaddockWebView`，只重写 `performKeyEquivalent:`。网页有键盘时，paddock 自己的快捷键（`menu::bindings()` 里不带上下文的绑定，去掉跟着键盘走的 Copy、Paste、Find、Find Next/Previous；加上 ⌘L／⌘R）直接交给 GPUI 的视图并吞掉，网页看不到，只执行一次；不论 AppKit 先问 GPUI 的视图还是网页。其他键先交给 WebKit（网页先处理）；网页不要的 ⌘C／⌘V／⌘X／⌘A／⌘Z／⇧⌘Z 在 WebKit 重发回来时，像 Edit 菜单那样发 `copy:`／`paste:`／`cut:`／`selectAll:`（`sendAction:to:from:`），撤销／重做走窗口的 undo manager。不带 ⌘ 的键一律不碰（输入法不经过 paddock）。
  - `view.rs`：终端的 Copy／Paste 在 AppKit 键盘在网页里时 `propagate`，不复制、不粘贴进终端（防“刚点网页、记录还没跟上”那一下）。
  - `menu.rs`：两个新动作和绑定；`follows_keyboard`、`browser` 两个分类；Edit 菜单的 Copy／Paste 改成系统的 `copy:`／`paste:`（`MenuItem::os_action`），网页有键盘时用鼠标点菜单也给网页，GPUI 的视图有键盘时照旧走 GPUI。
  - `window.rs`：订阅 `Handoff`（`Dismiss` 关弹出框；`ToPane` 先交还 AppKit 键盘再聚焦当前窗格）；`close_popup` 在记录归网页时把键盘还给网页，否则照旧给窗格；`focus_pane` 在点当前窗格（含标题）而键盘不在它里面时把键盘拿回来；去掉 P5-28a 的“GPUI 里任何鼠标按下都交还键盘”。
  - `Cargo.toml`：`objc2-app-kit` 加 `NSApplication`、`NSEvent` 特性（任务允许）；锁文件不变。
- **验证了什么**
  - 新测试 10 项（`browser`）：键盘在网页／终端／地址栏之间随点击、⌘L、⏎、Esc、切窗格交接；悬停说明等暂时隐藏后还给网页，期间被别处拿走就不还；关右侧栏、离开 Browser 标签交给窗格；弹出框（有／无输入框）借走和归还，选了别的就不还；点网页关掉弹出框；快捷键去向：⌘W ⌘T ⌥⌘B ⌘P ⌘B ⌘⇧P ⌘D 等在三种归属下都是 paddock 的，且逐条核对 `menu::bindings()`；⌘L／⌘R 只在网页、地址栏有键盘时是 Browser 的；⌘C ⌘V ⌘X ⌘A ⌘Z ⇧⌘Z、⌘F、网页自己的 ⌘K 给有键盘的那一方；标准编辑键的识别；按键命名。`menu` 新增 1 项、改 2 项（新绑定、Copy／Paste 是系统动作）。
  - 测试是在实现之后写的；把六处规则（弹出框借键盘、Find 等跟着键盘、弹出框开着不给网页、重新显示后归还、离开时交给窗格、点网页关弹出框）各临时改坏一次，每次都有对应测试失败，改回后通过。
  - `app/` 下 `cargo test --all-targets` 全过（库 229 项，共 269 项），`cargo clippy --all-targets -- -D warnings` 无警告，`cargo fmt --check` 通过。
  - release 程序起了一个自己的窗口：`PADDOCK_NO_ACTIVATE=1`、`GPUI_TERM_WINDOW_ID=1`，临时 `HOME`／`CFFIXED_USER_HOME`／`XDG_STATE_HOME`／`XDG_CONFIG_HOME`，Rust 写的假 corral（没有 agent），预写布局（右侧栏开在 Browser，网址指向 Rust 写的临时本地服务），`--bounds 40,40,1280,800`。能启动，Browser 显示了页面（服务收到 `GET /keys`，页面 JS 报 396×650），只截了这个窗口，截图在 scratchpad，自己看过，不入库；之后按 PID 停掉。网站数据落在临时 HOME 的 `Library/WebKit/paddock`，真实 `~/Library/WebKit/paddock`、`HTTPStorages/paddock` 不存在。
  - 真实键盘、输入法、点击没有模拟，见下面的清单。
- **拿主意的地方**
  1. 记录从“看到的状态”推出来，每帧和轮询时统一结算，而不是在各处调用点逐个改；新的焦点去处（以后的新弹出框、新输入框）不用登记。
  2. 点 GPUI 里不拿焦点的地方（工具栏的后退／刷新、标题栏、侧栏空白）不再把键盘从网页拿走，像 Safari 的工具栏；点窗格（含标题）、地址栏、弹出框才拿走。这改了 P5-28a 的“任何点击都交还”，免得网页在失焦／得焦之间来回闪。
  3. Copy、Paste、Find、Find Next／Previous 算“跟着键盘走”的命令：网页有键盘时 ⌘F／⌘G／⌘⇧G 给网页（WKWebView 自己没有查找界面，网页不处理就什么也不发生），Edit 菜单里这几项此时变灰。任务写的是“paddock 菜单里已有的照样生效”，⌘F 也在菜单里，这条请主控定（见回复）。
  4. 弹出框（Attention、分屏、动作菜单、palette、新标签）打开时网页暂交键盘，Esc／点外面关掉后回到网页；在里面选了东西则去选中的地方。以前 `close_popup` 一律交给当前窗格。
  5. 网页有键盘时，paddock 的快捷键即使 GPUI 没处理也吞掉，不交给网页（保证网页看不到 ⌘W 等）。
  6. Edit 菜单 Copy／Paste 改为系统动作，菜单上没有新增 Cut、Select All、Undo 项；⌘X／⌘A／⌘Z 靠子类在网页放弃后补发。
  7. ⌘R 在加载中也是重新加载（按钮在加载中是停止，不变）；⌘L 在已在编辑时只全选不重置文字。
  8. 地址栏 Esc 改为把键盘还给网页；空状态（还没打开过网址）不挂网页焦点，点它不拿走终端的键盘。
- **没做的事**
  - 地址栏没有撤销：`TextInput` 本身不支持 ⌘Z，`text_input.rs` 不在本轮可改范围；地址栏里 ⌘Z 什么也不做（也不会到终端）。
  - AppKit 先问哪个视图、WebKit 把网页放弃的键重发回来的路径，只按源码和文档推定，没在真窗口里按键验证；两种顺序都按设计处理了。若 ⌘X／⌘A／⌘Z 在网页输入框里不起作用（⌘C／⌘V 另有菜单兜底），需要再加 Edit 菜单项，另议。
  - 没改 DESIGN；工具栏、布局跟随、浮层判断、ATS、网站数据、弹窗、Inspector 都没动。
- **给用户的手动检查清单**（用装好的 paddock，右侧栏开 Browser，打开一个有输入框的页面，例如 `https://github.com`；涉及 ⌘W 的项先开一个可以关的空标签）
  1. 点网页里的输入框打字：字进网页；左边终端的光标变成空心。
  2. 再点终端打字：字进终端，终端光标变实心；网页输入框的光标消失。
  3. 点地址栏打几个字，不按回车，直接点网页：地址栏的光标消失，地址栏恢复显示当前网址；接着打字进网页。
  4. 网页有键盘时按 ⌘L：地址栏获得光标，整个网址被选中；打一个新网址按 ⏎：页面跳转，接着打字进网页。地址栏里按 Esc：恢复显示网址，打字回到网页。
  5. 网页或地址栏有键盘时按 ⌘R：页面刷新。终端有键盘时按 ⌘R、⌘L：页面不刷新，地址栏不动。
  6. 网页输入框里打字、选中，⌘C 再 ⌘V：在网页里复制粘贴；终端里没有出现任何粘贴内容。终端里选中文字 ⌘C，再点网页输入框 ⌘V：贴进网页，不进终端。
  7. 网页输入框里 ⌘A（全选）、⌘X（剪切）、⌘Z（撤销）、⇧⌘Z（重做）：都作用在网页里。
  8. 地址栏里 ⌘A、⌘C、⌘X、⌘V：作用在地址栏。终端里 ⌘C／⌘V：和以前一样。
  9. 网页有键盘时用鼠标点菜单栏 Edit → Copy／Paste：作用在网页。
  10. 网页有键盘时依次按 ⌘T（新标签面板出现一次，不是闪一下就关）、Esc、⌘P（palette 打开一次）、Esc、⌘⇧P、Esc、⌘D（分屏面板）、Esc、⌘B（左侧栏收起／展开一次）、⌘W（关掉当前窗格一次）：每个都只生效一次；每次 Esc 后接着打字回到网页。
  11. 网页有键盘时按 ⌥⌘B：右侧栏关闭，接着打字进终端；再按 ⌥⌘B 打开，键盘仍在终端。右侧栏切到 Changes 再切回 Browser：同样键盘在终端。
  12. 网页有键盘时按 ⌘1／⌘2 或点另一个窗格：键盘到那个终端。
  13. 网页自己的快捷键：在 github.com 按 ⌘K（或 `/`）：网页自己的搜索框出现，paddock 没有反应。
  14. 网页输入框里打字时，把鼠标停在地址栏右边“在默认浏览器打开”按钮上：网页暂时隐藏、出现说明；移开后网页回来，接着打字仍进网页（不需要再点）。拖动右侧栏分割线后同样。
  15. 点左下角的菜单按钮打开动作菜单（或点铃铛打开 Attention），然后点网页：菜单关闭，键盘在网页。网页有键盘时按 ⌘⇧A 打开 Attention，按 Esc：打字回到网页。
  16. 网页有键盘时点 Browser 工具栏的后退／刷新按钮：键盘仍在网页。
  17. 输入法（中文拼音）：在网页输入框里打 `nihao`，候选框出现在网页输入框旁，空格或数字选词，字进网页；再打一段拼音按 Esc：组字取消，什么都没进网页或终端；再打一段拼音不选词，直接点终端：网页里的组字结束，之后终端里拼音输入正常。
  18. 终端里的中文输入和以前一样。
  19. 地址指向一个没开的本地端口（失败页面），按 ⌘R：重试。

## 主控审查

2026-10-07，paddock/main。可以合并，已合并并安装；功能要靠用户照上面 19 条清单实测。
- 范围符合约定：改 `browser.rs`（`Keys` 统一记录、`PaddockWebView` 只重写 `performKeyEquivalent:`）、`browser_view.rs`、`menu.rs`、`view.rs`（终端 Copy／Paste 在网页有键盘时让开）、`window.rs`；`objc2-app-kit` 加 `NSApplication`、`NSEvent` 特性，锁文件不变，没加新库。不带 ⌘ 的键一律不碰，输入法不经过 paddock。
- 主控在分支上重跑：`cargo test --all-targets` 269 项全过，clippy、`cargo fmt --check` 通过。
- 取舍都同意：点工具栏、标题栏、侧栏空白不拿走网页的键盘（像 Safari）；弹出框关掉后键盘回到网页；网页有键盘时 paddock 快捷键即使没处理也吞掉；地址栏 Esc 把键盘还给网页。
- 对方问的 ⌘F／⌘G：主控定为网页有键盘时给网页（不去打开终端的查找栏，否则键盘在网页却搜终端，更怪）；WKWebView 没有查找界面，按了没反应。页面内查找（⌘F 搜网页）记入 P5-28c 的候选，问用户。
- 未验证、要用户试：⌘X／⌘A／⌘Z 在网页里是否生效（靠子类补发，按源码推定）；若不灵，在 Edit 菜单加 Cut、Select All、Undo 另议。地址栏没有撤销（`text_input.rs` 本身不支持）。

## 用户实测

2026-10-07，用户照上面 19 条清单在装好的 paddock 里试过，全部正常（含网页有键盘时 ⌘C／⌘V 不进终端，⌘A／⌘X／⌘Z／⇧⌘Z 作用在网页里）。Edit 菜单不需要补 Cut、Select All、Undo。
