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

## 完成记录

- **做了什么**
  - 依赖：加 `objc2-web-kit =0.3.2`，`objc2-foundation =0.3.2` 写成直接依赖，特性都只开用到的；锁文件只新增 `objc2-web-kit` 一个包，其他版本不变，`gpui-pre-*` 和 objc2 0.5/0.6 链没动。另给已有的 `objc2-app-kit` 特性表补了 `NSWindow`（取、交还 firstResponder 要用；`objc2-web-kit` 本来就会打开它，编译结果不变）。任务的依赖条没写到这一项，这里说明。
  - `browser.rs`（原生宿主）：每窗口一个 `WKWebView`，用默认的持久网站数据，作为 GPUI 视图上方的兄弟视图；第一次显示 Browser 标签时创建（推到这一帧画完之后，不在 GPUI 绘制中途动 AppKit），不调用任何 activate。导航委托（自定义类 `PaddockPageDelegate`）记下加载失败的网址和系统给的原因（忽略“取消” -999 和 WebKit 102），以及网页进程退出。`Page::place` 按布局设 frame 或 `setHidden`，隐藏时如果网页拿着键盘就交还 GPUI 的视图；`Drop` 时清掉委托、停止加载、移出视图。圆角：原生层 `cornerRadius` + `masksToBounds`（用 `msg_send` 发，不为两次调用引入 Core Animation 绑定），截图放大确认裁剪生效。
  - 统一判断：`browser::Scene::page()`——侧栏开着、在 Browser、内容区有面积、没在拖分割线、没有任何浮层和网页区相交，才显示。浮层登记只有一种写法：在浮层里放 `browser::cover()`（空 canvas，排版时记下自己的范围）；窗口根元素最后一个 canvas 在绘制阶段（这时连悬停说明也排好了）调 `BrowserView::place` 做一次判断、摆放网页。已登记：palette（整窗压暗）、新标签和分屏面板、Attention 列表、侧栏动作菜单、标题栏悬停说明 `BarTip`、右侧栏悬停说明 `Tip`（工具栏共用）。拖动用“正在拖”的标志，左右两条分割线都算。
  - `browser_view.rs`（工具栏视图）：照样稿依次是后退、前进（不可用时变淡）、刷新（加载中变成停止）、地址栏、在默认浏览器打开。地址栏不在输入时显示当前页面的最终网址，去掉 `http(s)://`，本地服务的端口亮、其余淡（和样稿一样）；点一下变成输入框，⏎ 打开，Esc 放弃；加载中输入框底边一条 2pt 强调色细线表示进度。“在默认浏览器打开”打开当前页面的网址（失败时是失败的那个），不是输入框里没提交的字。没打开过网址：地球图标 + “Open a page”。加载失败：“Can’t open this page”、系统的原因、Retry（重试失败的那个网址）；HTTP 404/500 不算失败，照常显示网页。地址规范化 `address()`：写了 `scheme://` 的照用；否则 `localhost`、`*.localhost`、回环 IP（127.0.0.0/8、`::1`）补 `http://`，其他补 `https://`；空的、中间有空格的不打开。页面状态每 50ms 读一次，变了才重画。
  - `right_panel.rs`：`Saved`、`RightPanel` 加 `url: Option<String>`（`Saved` 本来就有结构级 `#[serde(default)]`，旧文件读成 `None`）；`render` 改成接收当前标签的内容；`placeholder` 公开给 Browser 空状态用，Browser 的那行字改成 “Open a page”；`Tip` 公开、加构造函数和 `cover()`。
  - `window.rs`：持有 `Entity<BrowserView>`，订阅 `Visited` 把显示的网址写回布局；改主题时同步；GPUI 里任何鼠标按下都把键盘交还 GPUI 的视图（推到这个事件之后）；上述浮层加 `cover()`；摆放网页的 canvas。
  - `footer_icon.rs`：加 Back、Forward、Reload、External 四个图标（样稿的路径按 14 点的格子放大）；停止用已有的 Close。
- **验证了什么**
  - 测试：`browser` 3 项（显示判断：收起、切到 Changes、空或失败、零高度、palette 整窗、不相交的菜单、压到顶边和只挨着顶边的悬停说明、多个浮层、拖动，以及几种组合；坐标换算：未翻转和翻转的父视图、GPUI 视图有偏移、小数点），`browser_view` 2 项（补协议、localhost 和回环判断、近似但不算本机的、保留已写的协议、前后空白、空和有空格），`layout_state` 的右侧栏测试补了 url 的往返和两种旧文件读成 `None`。测试是和实现一起写的，没先跑红；事后把 `dragging` 判断和 `localhost` 判断各临时改坏一次，对应 3 项测试失败，改回后通过。
  - `app/` 下 `cargo test --all-targets` 全过（库 212 项，共 249 项），`cargo clippy --all-targets -- -D warnings` 无警告，`cargo fmt --check` 通过。
  - 截图：用 release 程序、`PADDOCK_NO_ACTIVATE=1`、每次单独的临时 `HOME`／`CFFIXED_USER_HOME`／`XDG_STATE_HOME`／`XDG_CONFIG_HOME`、Rust 写的假 corral（没有 agent）、预写的布局（右侧栏开在 Browser，记住的网址指向 Rust 写的临时本地服务 `http://localhost:<随机端口>/settings`）、`--bounds 40,40,1280,800`。一个 Rust 监督程序起服务和 paddock、只截 `window-id` 给的那个窗口、按 PID 停 paddock、停服务线程。看过的图：420 宽 Dune 正常显示页面（页面 JS 报 396×650，服务收到 `GET /settings`）；布局宽度写 620 当“加宽”（不能点 Wider，窗口 1280 时被限到约 524，JS 报 500×650）；空状态；加载失败（指向没人听的端口，显示 “Could not connect to the server.” 和 Retry）。另截了 Tide 18 页面、Lagoon 13 页面、Lagoon 18 失败三张看工具栏：一行不折，颜色正常。放大看了网页四角：原生圆角生效，外面一圈淡边。布局文件写回了 url。网站数据落在临时 HOME 的 `Library/WebKit/paddock`；真实 `~/Library/WebKit`、`HTTPStorages`、`Caches` 下没有 paddock 目录（运行前后都查过）。
- **拿主意的地方**
  1. 浮层范围用一张“本帧浮层表”（thread_local，按窗口分），在绘制阶段统一判断：GPUI 的悬停说明在根元素排版之后才排，只有到绘制时才知道它在哪。
  2. 工具栏只给“在默认浏览器打开”加悬停说明（样稿也只有它有 title）。后退、前进、刷新不加：GPUI 的悬停说明挂在鼠标右下方，会压到网页顶部，按规则网页会暂时隐藏。
  3. 网页隐藏时如果拿着键盘就交还 GPUI（否则 ⌘P 打开 palette 后打的字会进隐藏的网页）。代价：鼠标停在“在默认浏览器打开”上出说明时，网页也会交出键盘，需要再点一下网页。
  4. 左右两条分割线拖动时都隐藏网页（resizing 遮罩本来就盖住整窗）。
  5. `*.localhost` 也算本机（RFC 6761）；`0.0.0.0`、局域网 IP 不算，补 `https://`。明写 `http://` 的任意网址照原样打开、不改写，能不能打开由系统 ATS 决定，失败就显示错误状态；没有为它放宽任何东西。
  6. ⏎ 打开后键盘交给网页（像 Safari）；Esc 放弃输入、恢复显示当前网址。
  7. 网页区圆角 8pt、1pt 淡边（`agents_rule`），原生层在边里面、圆角 7pt；空状态和失败状态不画边框，沿用原占位的样子。
  8. 页面状态用 50ms 定时读，不用 KVO：沿用侧栏、Changes 的定时读法，免得把 ObjC 回调接回 GPUI；只有失败原因靠导航委托记下。
  9. 没给网页设明暗外观（`NSAppearance`），网页的 `prefers-color-scheme` 跟系统；加载前的白底也没处理。
- **没做的事**
  - 点网页拿键盘、点终端或地址栏交回、键盘和输入法、拖宽／缩放／全屏时会不会闪或错位、palette／菜单／悬停说明出现时的隐藏、窗口关闭时的释放、“在默认浏览器打开”，都没在真窗口里操作（不能模拟键鼠），留给用户实际用。
  - 给 P5-28b：①网页拿键盘时，⌘C／⌘V／⌘W／⌘P 等组合先到 GPUI 的视图还是 WKWebView、会不会落到终端，未验；②在地址栏输入到一半去点网页，GPUI 焦点还在输入框（光标还画着），键盘却已经在网页；③悬停说明引起的暂时隐藏会让网页丢键盘（见上 3）；④网页拿键盘时 GPUI 内部焦点仍停在原来的终端；⑤动作菜单、Attention 列表不和网页相交时网页照常显示，这时点网页不会关掉菜单（点击被网页接走）。
  - 给 P5-28c：打包后的 `http://localhost` 和 `http://127.0.0.1` 已验（见下面的补充）；`::1`、局域网 IP、任意 http、自签名 HTTPS 都没验。没设 UIDelegate：`target=_blank`／`window.open`、JS 对话框、文件选择、下载目前都不处理；没开 Inspector。
  - 没登记的悬停说明：只剩 `changes.rs` 的 `Tip`，它只在 Changes 标签出现，那时网页本来就隐藏。`sidebar.rs` 的两个已按主控同意补上（见下）。

### 补充（主控回复后）

- 主控同意后，`sidebar.rs` 的 `Tip`、`RailTip` 各加了一行 `.child(crate::browser::cover())`，这个文件只改了这两处。`objc2-app-kit` 补 `NSWindow` 特性，主控已接受。
- 打包验证：在自己的编译目录跑 `cargo run --release --example bundle`（没加 `--install`，没碰 `~/Applications`），打出 `paddock.app`（Apple Development 签名，验签通过；Info.plist 里没有 ATS 项）。直接运行包里的 `Contents/MacOS/paddock`，条件和前面的截图相同。`http://localhost:<端口>/` 和 `http://127.0.0.1:<端口>/` 都正常打开：服务各收到一次 `GET /`，页面 JS 报 396×650。没被 ATS 拦，所以 `bundle.rs` 没改。网站数据落在临时 HOME 的 `Library/WebKit/dev.paddock.app`，说明这次用的是包里的 Info.plist 和 bundle id；真实 `~/Library` 下 `dev.paddock.app` 的 WebKit、HTTPStorages、Caches、Saved Application State 都不存在，`Preferences/dev.paddock.app.plist` 的修改时间没变（运行前后都查过）。
- 补完后 `app/` 下 `cargo test --all-targets` 249 项全过，`cargo clippy --all-targets -- -D warnings` 无警告，`cargo fmt --check` 通过。

## 主控审查

2026-10-07，paddock/main。可以合并，已合并并安装。
- 范围符合约定：新增 `browser.rs`（原生宿主、统一的“网页该不该显示”判断 `cover()`）、`browser_view.rs`（工具栏）；改 `right_panel.rs`、`window.rs`、`layout_state.rs`、`footer_icon.rs`、`lib.rs`；依赖只加用户批准的 `objc2-web-kit =0.3.2` 和直接写出的 `objc2-foundation =0.3.2`，锁文件只多一个包。
- 主控看了截图：工具栏、地址栏（端口高亮）、圆角网页和样稿一致，JS 正常。
- 对方问的两件：`sidebar.rs` 的 `Tip`、`RailTip` 登记成浮层，同意并已补（`b5a9ec0`）；`objc2-app-kit` 补 `NSWindow` 特性，同意。
- 主控要求补的打包验证：用 `bundle` 打出的 app（未安装）里的程序打开 `http://localhost` 和 `http://127.0.0.1` 都正常，没被 ATS 拦，`bundle.rs` 未改；网站数据只落在临时 HOME。
- 取舍同意：后退前进刷新不加悬停说明（GPUI 说明会压到网页、触发隐藏）；网页隐藏时交还键盘；`*.localhost` 算本机、`0.0.0.0` 和局域网 IP 补 `https://`；页面状态每 50ms 读一次。
- 主控在分支上重跑：`cargo test --all-targets` 249 项全过，clippy、`cargo fmt --check` 通过。
- 留给用户和 P5-28b／28c：点击、键盘、输入法、拖宽／全屏是否闪、浮层隐藏、⌘C／⌘V 等走向；`::1`、局域网、自签名证书、弹窗、下载、Inspector。
