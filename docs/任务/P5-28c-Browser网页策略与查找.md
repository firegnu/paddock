# 任务：Browser 收尾：本地开发页面一定能打开，补新窗口、对话框和上传、下载、证书失败、Safari 调试、网页内查找

2026-10-07，paddock/main 交给 paddock/dev-browser-web（Claude Code，重：opus[1m] / xhigh）。
路由：重 / 交叉审查不要 / 影响面：改行为（路由：档拿不准（重 0.51、常规 0.49），同一块原生代码 28a、28b 都是重，定为重；交叉审查要、影响面碰要害（安全隐私 0.93）；推翻：用户 10-07 说这个 Browser 只给本地开发用、不当正经浏览器、不要在细节里绕，证书检查本轮不放松，下载文件名不越出 `~/Downloads` 靠测试和主控审查把关，定为改行为、不交叉审查）
类型：功能变更
依据：本轮让本地开发页面在打包后的 app 里一定能打开，其余几项做最简单能用的版本；焦点和快捷键分配、工具栏已有的样子、布局跟随、浮层隐藏不在本轮，除了 ⌘F 查找要接进现有的键盘记录。
依赖：P5-28b
待用户：按本文件完成记录末尾 8 条清单实测（第 1 条本地开发地址最要紧）。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」一节，以及「开发方式」里的看板约定。
- `docs/DESIGN.md` §13「P5-28 右侧栏 Browser（WKWebView）」：第 3、5 条和「P5-28c 的网页策略」（用户定的）。
- 调研 `docs/调研/P5-13r-Browser嵌入.md` §5、§5.1。
- `docs/任务/P5-28a-Browser原生宿主与工具栏.md`、`docs/任务/P5-28b-Browser焦点与快捷键.md` 的完成记录和主控审查。
- 代码：`app/src/browser.rs`、`app/src/browser_view.rs`（`address()`、`local()`、失败状态、“在默认浏览器打开”）、`app/src/menu.rs`（Find／FindNext／FindPrevious）、`app/src/view.rs`（终端的查找栏，照它的样子和用语）、`app/examples/bundle.rs`（Info.plist，最低系统 13.0）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-28c-browser-web`，分支 `p5-28c-browser-web`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-28c-browser-web`。
- 可以改：`browser.rs`、`browser_view.rs`、`menu.rs`、`window.rs`（只接线）、`examples/bundle.rs`（只加 ATS 项）、`Cargo.toml` 里 `objc2-web-kit`／`objc2-app-kit`／`objc2-foundation` 的特性（不加新库，锁文件不能多出包）。需要动别的文件，先停下来问。

## 要做的
用户原话（10-07）：“这个browser现在基本够用以及能支撑下面的开发了，尤其是本地的开发的页面一定要支持打开，coding agent本来就是给本地开发用的，我不会把它当作一个正经浏览器使用的”“不要在细节的里面绕太多”。所以第 1 条最要紧，第 2 到 7 条做最简单能用的版本，不追求浏览器级的完整。

1. **本地开发页面一定能打开**（打包后的 app 里）：开发服务常打印的地址都要能打开：`localhost`、`*.localhost`、`127.0.0.1` 等回环 IP、`[::1]`、`0.0.0.0`、本机的局域网 IP（`192.168.*`、`10.*`、`172.16–31.*`）、`*.local`，带不带端口和路径都行。不写协议时这些都补 `http://`（现在 `0.0.0.0` 和局域网 IP 补的是 `https://`，要改）；明写 `http://` 的照原样。打包里需要 ATS 项时用 `NSAllowsLocalNetworking` 这一类本地网络的项；不加 `NSAllowsArbitraryLoads`、`NSAllowsArbitraryLoadsInWebContent`，公网的 http 仍不放开。
2. **新窗口**：`target=_blank` 和 `window.open` 的 http(s) 在同一面板打开；`mailto:` 等交给系统；其他不打开。
3. **JS 对话框和文件上传**：alert、confirm、prompt 和 `<input type=file>` 用系统原生的、挂在 paddock 窗口上的面板（sheet）。
4. **下载**：存到 `~/Downloads`，文件名用网页给的、去掉路径部分，不能写到 `~/Downloads` 外面；同名自动改名，不覆盖。下完工具栏短暂显示文件名，点它在 Finder 里显示。
5. **证书失败**：自签名或不受信任的 https 证书不放行，失败状态里说明是证书不受信任，给一个用默认浏览器打开的按钮。
6. **Safari 调试**：系统支持时（macOS 13.3 起，按系统版本或 selector 判断，不抬高最低系统要求）打开网页的 `isInspectable`。
7. **网页内查找**：网页或地址栏有键盘时 ⌘F 打开 Browser 自己的查找栏（工具栏下方，不压在网页上），⏎／⌘G 下一个，⇧⏎／⇧⌘G 上一个，Esc 关掉并把键盘还给网页，找不到时说明；样子和用语照终端的查找栏。终端有键盘时 ⌘F 照旧。查找栏的输入框接进 `browser::Keys` 的记录。
8. 界面文字英文；只从主题取色，不新增主题颜色键；字号用 `ui.px`。

## 怎么算做完
- 上面八条达到。
- 验证只做这些：
  - 测试：`address()` 对第 1 条各种本地地址补 `http://`、公网名字补 `https://`；下载文件名去掉路径和同名改名；查找栏和 `Keys` 的交接。改 `address()` 的测试先写，确认改之前失败。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 打包验证（只验第 1 条）：在自己的编译目录 `cargo run --release --example bundle`（不加 `--install`），直接运行包里的程序（`PADDOCK_NO_ACTIVATE=1`、临时 HOME、假 corral、预写的布局文件），本机起临时 http 服务，确认 `http://[::1]:<端口>`、`http://0.0.0.0:<端口>`、`http://<本机局域网 IP>:<端口>` 都能打开（服务收到请求）。截一张自己窗口的图。
  - 其余几条不能模拟点击，在完成记录里写一份简短的手动检查清单（每条：怎么操作、应该看到什么）。
  - 觉得不够，在回复里说，不要自己加。

## 不要做
- 不做“清除网站数据”入口（用户：先不加）；不改网站数据的存放方式。
- 不放松证书检查，不实现“仍然继续”；不用私有 WebKit 接口（含 `_inspector`、`developerExtrasEnabled`）。
- 不改快捷键分配、工具栏已有的样子、布局跟随、`cover()` 的判断；不加新库，不改 `gpui-pre-*`；不复制 Zed 应用层（GPL）的代码。
- 测试和验证不写进用户真实的 `~/Downloads`、`~/Library`，不碰用户真实的配置、布局文件、网站数据和正在运行的 paddock；用临时 HOME。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口（`GPUI_TERM_WINDOW_ID=1`、`screencapture -x -o -l <编号>`），启动时一定加 `PADDOCK_NO_ACTIVATE=1`；截图不入库。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。不读 corral 的内部状态目录。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程和服务用记下的 PID。
- 不重新打包安装（不加 `--install`）。
- 第 1 条在打包后的 app 里有打不开的，停下来报告，等决定。第 2 到 7 条有公开接口做不到的，做到能做的部分，在完成记录里写明，不要绕路。
- 不合并到 main，不推送。只在 `p5-28c-browser-web` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、给用户的手动检查清单，各几句话。回复里只写这几样、截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- **做了什么**
  - 第 1 条：`browser_view.rs` 的 `local()` 扩到开发服务常打印的地址（`localhost`、`*.localhost`、`*.local`、回环 IP、`0.0.0.0`／`::`、私有 IPv4 `10.*`、`172.16–31.*`、`192.168.*`），不写协议时补 `http://`，其余补 `https://`，明写的协议照用。新增 `reachable()`：http(s) 地址的主机是 `0.0.0.0` 或 `[::]` 时换成 `127.0.0.1`／`[::1]`，端口、路径、用户名不变；`address()`（打字或粘贴，含明写 `http://`）和启动时恢复的布局地址都经过它，网页里的链接不改。`bundle.rs` 的 Info.plist 只加 `NSAppTransportSecurity` → `NSAllowsLocalNetworking`。
  - 第 2 条：页面委托实现 `WKUIDelegate` 的 `createWebView…`：`target=_blank`、`window.open` 的 http(s) 在同一个网页里打开（可后退），不建第二个网页；`decidePolicyForNavigationAction` 和新窗口里，`about:`／`blob:`／`data:`／`javascript:`／`file:` 以外的非网页地址（`mailto:`、`tel:` 等）交给系统（`NSWorkspace openURL`），网页内嵌框架发起的不打开。
  - 第 3 条：alert／confirm／prompt 用 `NSAlert` 作 sheet 挂在 paddock 窗口上（OK，confirm／prompt 另有 Cancel，Esc 取消；prompt 带输入框和默认文字）；`<input type=file>` 用 `NSOpenPanel` sheet，按网页要求允许多选、选文件夹。
  - 第 4 条：`<a download>`（`shouldPerformDownload`）、网页显示不了的回应、`Content-Disposition: attachment` 转成下载（`WKDownload`）。存到 `$HOME/Downloads`：`download_name()` 只取网页给的名字最后一段（按 `/` 和 `\` 切），去掉控制字符，空、`.`、`..` 改成 `download`；`download_path()` 同名时改成 `名字-2.扩展名`、`-3`……，已有的文件（含断开的符号链接）和正在下载的都算占用。下完工具栏地址栏右边出现约 6 秒的 “↓ 文件名”，点它在 Finder 里选中。
  - 第 5 条：`NSURLErrorDomain` 的 -1201／-1202／-1203／-1204（证书过期、不受信任、根证书未知、尚未生效）记为证书失败；失败状态写 “This site’s certificate is not trusted.”，Retry 旁多一个 “Open in default browser”。没实现认证质询回调，WebKit 照默认处理拒绝，没有“仍然继续”。
  - 第 6 条：建网页时 `respondsToSelector(setInspectable:)` 成立才 `setInspectable(true)`（macOS 13.3 起），最低系统仍是 13.0，没用私有接口。
  - 第 7 条：Browser 自己的查找栏，在工具栏下方、网页上方（网页让出位置，不压在网页上），样子和用语照终端的（焦点色边框、`agents_bg` 底、占位字 “Find”、红色 “No match”、↑ ↓ ×），字号用 `ui.px`。网页或地址栏有键盘时 ⌘F 打开并全选输入框；打字从当前匹配的开头接着找；⏎／⌘G／↓ 下一个，⇧⏎／⇧⌘G／↑ 上一个；Esc／× 关掉并把键盘还给网页。用公开的 `findString:withConfiguration:completionHandler:`（不分大小写、到头绕回），结果由 50ms 轮询读回。`browser::Keys` 加 `Owner::Find`；`route()` 在网页、地址栏、查找栏有键盘时把 ⌘F／⌘G／⇧⌘G 交给 Browser（`menu::finds`），终端有键盘时照旧给终端的查找栏。
  - 依赖（主控转用户批准）：`block2 = "=0.6.2"` 写成直接依赖；`objc2-web-kit`、`objc2-app-kit` 开 `block2` 特性，并加对话框、打开面板、下载、查找要用的特性（AppKit：`NSAlert`、`NSButton`、`NSControl`、`NSOpenPanel`、`NSPanel`、`NSSavePanel`、`NSTextField`；Foundation：`NSArray`、`NSData`、`NSURLResponse`；WebKit：`WKDownload`、`WKDownloadDelegate`、`WKFindConfiguration`、`WKFindResult`、`WKFrameInfo`、`WKNavigationAction`、`WKNavigationResponse`、`WKOpenPanelParameters`、`WKUIDelegate`、`WKWindowFeatures`）。锁文件没有多出包，只在 `paddock` 和 `objc2-web-kit` 的依赖表里各多一行已有的 `block2 0.6.2`。`window.rs` 没改。
- **验证了什么**
  - 测试先写：`address()` 的本地地址测试改之前失败（`0.0.0.0:8000` 得到 `https://`），改后通过；`0.0.0.0`／`[::]` 换回环地址的测试同样先失败（得到 `http://0.0.0.0:3000/`）再通过。新增 4 项：查找栏和 `Keys` 的交接（⌘F 从网页、地址栏进入，Esc 还给网页，点网页、点终端、⌘L、暂时隐藏、关右侧栏）、⌘F／⌘G／⇧⌘G 的去向、下载文件名去掉路径、同名改名且不出 `~/Downloads`；原有路由测试里 ⌘F 的期望改为交给 Browser。
  - `app/` 下 `cargo test --all-targets` 全过（库 233 项，共 273 项），`cargo clippy --all-targets -- -D warnings` 无警告，`cargo fmt --check` 通过。
  - 打包验证：自己编译目录里 `cargo run --release --example bundle`（未加 `--install`，Apple Development 签名，验签通过），直接运行包里的 `Contents/MacOS/paddock`：`PADDOCK_NO_ACTIVATE=1`、`GPUI_TERM_WINDOW_ID=1`、每次单独的临时 `HOME`／`CFFIXED_USER_HOME`／`XDG_STATE_HOME`／`XDG_CONFIG_HOME`、Rust 写的假 corral、预写布局（右侧栏开在 Browser，地址指向本机临时服务）。Rust 写的临时服务监听 `[::]` 随机端口。七个地址都打开了，服务都收到网页请求，网页 JS 也回连了服务（报 396×650）：`http://[::1]:端口`、`http://0.0.0.0:端口`（打开成 `127.0.0.1`）、`http://<本机局域网 IP，192.168.*>:端口`、`http://[::]:端口`（打开成 `[::1]`）、`http://<本机名>.local:端口`、`localhost`、`127.0.0.1`。换地址之前，加不加 ATS 项结果都一样：`0.0.0.0`、`[::]` 被 WebKit 拒绝（“Not allowed to use restricted network port”，请求没发出），其余都能打开。只截了自己的窗口，截图在 scratchpad，不入库。
  - debug 版跑过一次：新的委托方法注册正常，网页照常加载。
  - 真实 `~/Library` 下 `dev.paddock.app` 的 WebKit、Caches、Preferences 修改时间和运行前一样，`HTTPStorages`、`Saved Application State` 下仍没有；真实 `~/Downloads` 没有新文件。
- **拿主意的地方**
  1. 用户定（决定一，选 A）：WebKit 自己拒绝 `0.0.0.0`（与 ATS 无关，ATS 项改不了），打开时把主机 `0.0.0.0` 换成 `127.0.0.1`、`[::]` 换成 `[::1]`，端口和路径不变，打字或粘贴带 `http://` 的都换；网页里指向 `0.0.0.0` 的链接不改。启动时恢复的布局地址也走同一个换法，打包验证就是经这条路走到同一个函数。
  2. 用户定（决定二，选 A）：允许 `block2 = "=0.6.2"` 作直接依赖（GPUI 已经锁了同一版本并编进来），给 `objc2-web-kit`、`objc2-app-kit` 开 `block2` 和对话框、打开面板要的 AppKit 特性；锁文件不多包。第 3、4、7 条照标准接口（完成回调）做。
  3. ATS 项保留 `NSAllowsLocalNetworking`：本机 macOS 27 上不加它本地地址也能打开，加上是为了声明本地网络用途、在其他系统版本上不被拦；没加 `NSAllowsArbitraryLoads`／`…InWebContent`。
  4. “本地”还算上 `::`（Python `http.server` 等打印 `http://[::]:8000/`），和 `0.0.0.0` 一样对待；地址栏里“本地服务端口高亮”跟着覆盖所有本地开发地址。
  5. 非网页地址交给系统的范围：除了 `about`、`blob`、`data`、`javascript`、`file` 都交给系统（系统没有对应 app 时什么也不发生）；网页内嵌框架里发起的不交，免得隐藏框架悄悄拉起别的 app。`window.open` 的空白窗口（`about:blank` 再往里写）不支持。
  6. 下载：同名改名用 Safari 的 `名字-2.扩展名` 样式，按最后一个点分扩展名（`site.tar.gz` → `site.tar-2.gz`）；目录取 `$HOME/Downloads`，不存在就建；下载失败不提示；“↓ 文件名” 显示 6 秒，不加悬停说明（会压到网页、触发隐藏）。
  7. 查找：⌘F／⌘G 在网页有键盘时改成 Browser 的（P5-28b 是交给网页、按了没反应）；查找栏打开时点网页，栏保留，⌘G 接着找；再按 ⌘F 全选输入框；没打开过网页时 ⌘F 什么也不做；打字时先用一小段 JS 把网页选区收到开头，好让匹配在原处变长；↑ 是上一个、↓ 是下一个（终端的 ↑ 是更早的输出）；查找栏不浮动，所以不加阴影。
  8. JS 对话框的文字放在 `NSAlert` 的标题位置；网页暂时被隐藏时来的对话框照样弹出。
- **没做的事**
  - 下载没有进度和失败提示；网页里指向 `0.0.0.0` 的链接不改（照用户决定）；`window.open` 后靠 `window.opener` 往新窗口里写的页面（比如部分登录弹窗）不支持；查找只收主框架的选区。
  - 第 2 到 7 条没在真窗口里操作（不能模拟点击和按键），见下面的清单；公网 http 仍被拦这一点没另外验证。
  - `docs/DESIGN.md` 不在本轮可改范围，两条用户决定请主控补进 §13 P5-28c。
- **给用户的手动检查清单**（用装好的 paddock，右侧栏开 Browser。第 2 到 5 项先在 Safari 的 Develop 菜单里打开本页的 Web Inspector（见第 6 项），在 Console 里运行下面这一行，往任意本地页面里加测试用的链接和按钮：
  `document.body.insertAdjacentHTML('afterbegin','<p><a target=_blank href="https://example.com">blank</a> <a href="mailto:test@example.com">mail</a> <button onclick="window.open(\'https://example.com\')">open</button> <button onclick="alert(\'hi\')">alert</button> <button onclick="console.log(confirm(\'ok?\'))">confirm</button> <button onclick="console.log(prompt(\'name?\',\'paddock\'))">prompt</button> <input type=file multiple> <a download="report.txt" href="data:text/plain,hello">download</a></p>')`）
  1. 本地地址：地址栏输入 `0.0.0.0:<端口>`、`192.168.x.x:<端口>`、`<机器名>.local:<端口>`（不写协议）：都打开，地址栏显示 `127.0.0.1:<端口>`、`192.168.x.x:<端口>`……（端口高亮）；粘贴 `http://0.0.0.0:<端口>/` 也打开成 `127.0.0.1`；输入 `example.com` 打开的是 https。
  2. 新窗口：点 blank、点 open：example.com 在同一个面板打开，后退回到原页面；点 mail：系统的邮件 app 打开，面板不变。
  3. 对话框：点 alert：paddock 窗口上滑下系统的提示框，点 OK 关掉；confirm：OK 后 Console 打出 `true`，Cancel 或 Esc 打出 `false`；prompt：输入框里是 `paddock`，改了点 OK 打出改的字，Cancel 打出 `null`。
  4. 上传：点选择文件：窗口上滑下系统的打开面板，可以多选；选好后输入框显示文件名；取消则不变。
  5. 下载：点 download：`~/Downloads/report.txt` 出现，工具栏地址栏右边出现约 6 秒的 “↓ report.txt”，点它 Finder 里选中这个文件；再点一次 download：存成 `report-2.txt`，原来的不变。
  6. Safari 调试：Safari 设置 → 高级 → 勾“显示网页开发者功能”；Safari 的“开发”菜单 → 本机 → paddock 下面有这个网页，点开出现 Web Inspector。
  7. 证书：地址栏打开 `https://self-signed.badssl.com/`、`https://expired.badssl.com/`：显示 “Can’t open this page” 和 “This site’s certificate is not trusted.”，有 Retry 和 Open in default browser；点后者在默认浏览器里打开同一地址；paddock 里没有任何“继续”的办法。
  8. 查找：点网页后按 ⌘F：工具栏下方出现查找栏，网页往下让、不被盖住，输入框有光标；打一个页面里有的词：网页里选中第一个并滚过去；⏎ 或 ⌘G 下一个，⇧⏎ 或 ⇧⌘G 上一个，↑ ↓ 按钮同样；打一个没有的词：红色 “No match”；Esc：查找栏关掉，接着打字进网页。点地址栏后 ⌘F 同样打开；查找栏开着时点网页，栏还在，⌘G 继续找；终端有键盘时 ⌘F 打开的是终端自己的查找栏。

## 主控审查

2026-10-07，paddock/main。可以合并，已合并。
- 中途停下问的两件按规定报上来，用户都选了建议：`0.0.0.0`／`[::]` 换成回环地址；`block2 =0.6.2` 作直接依赖（锁文件只多两行指向已有包，主控核对过）。两条已补进 DESIGN §13（`34cc61d`）。
- 范围符合约定：改 `browser.rs`、`browser_view.rs`、`menu.rs`、`bundle.rs`（只加 `NSAllowsLocalNetworking`）、`Cargo.toml`／`Cargo.lock`；`window.rs` 没改；没加别的库，没用私有接口。
- 主控看了要害处：下载文件名只取最后一段、空／`.`／`..` 改成 `download`，路径不出 `~/Downloads`，同名不覆盖；没有实现证书质询回调，WebKit 照默认拒绝；对话框、上传面板每条路径都调用一次完成回调。
- 主控在分支上重跑：`cargo test --all-targets` 273 项全过，clippy、`cargo fmt --check` 通过。
- 取舍都同意，包括 ⌘F／⌘G 在网页有键盘时改给 Browser 查找栏（第 7 条本来要求的），`::` 算本地，嵌入框架不能拉起别的 app。
- 第 2 到 7 条要用户照上面 8 条清单实测。
