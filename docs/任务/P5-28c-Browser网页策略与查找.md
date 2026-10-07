# 任务：Browser 收尾：本地开发页面一定能打开，补新窗口、对话框和上传、下载、证书失败、Safari 调试、网页内查找

2026-10-07，paddock/main 交给 paddock/dev-browser-web（Claude Code，重：opus[1m] / xhigh）。
路由：重 / 交叉审查不要 / 影响面：改行为（路由：档拿不准（重 0.51、常规 0.49），同一块原生代码 28a、28b 都是重，定为重；交叉审查要、影响面碰要害（安全隐私 0.93）；推翻：用户 10-07 说这个 Browser 只给本地开发用、不当正经浏览器、不要在细节里绕，证书检查本轮不放松，下载文件名不越出 `~/Downloads` 靠测试和主控审查把关，定为改行为、不交叉审查）
类型：功能变更
依据：本轮让本地开发页面在打包后的 app 里一定能打开，其余几项做最简单能用的版本；焦点和快捷键分配、工具栏已有的样子、布局跟随、浮层隐藏不在本轮，除了 ⌘F 查找要接进现有的键盘记录。
依赖：P5-28b
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
