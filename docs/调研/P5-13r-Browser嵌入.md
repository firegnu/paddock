# P5-13r：在右侧栏嵌入 Browser

调研日期：2026-10-07。基线：`p5-13r-browser` / `e80c510`。范围：macOS、`gpui-pre =0.3.8`，源码与官方文档调查，加一个不合并的最小原型。本文是推荐，**不改 DESIGN、不批准正式依赖或功能**。

标记：**已查证·源码/文档**表示接口或实现存在；**已查证·原型**表示本机实际观察到；**推测/建议**不是实测结果；**未知**表示本轮不能作保证。接口存在不等于 paddock 已接好。

## 1. 结论

- **能嵌入。已查证·原型**：直接用 `objc2-web-kit 0.3.2` 创建 `WKWebView`，作为 GPUI 视图上方的原生兄弟视图，右侧 Browser 能显示临时 localhost HTTP 页面，JavaScript 已执行。420pt 和 620pt 两种预设侧栏宽度都落在正确区域。
- **不能直接当成普通 GPUI 元素。已查证·原型**：网页盖住 palette 右侧及其压暗背景；GPUI 的绘制顺序、`occlude()`、裁剪不会改变原生视图层级。隐藏网页后 palette 完整显示。
- **建议直接 WKWebView，首版用“浮层/拖动期间隐藏网页”的方案**，先把焦点和快捷键交接单独做通。快照替身可改善隐藏时的观感，但本轮没有验证，不应成为首版默认承诺。
- **未知**：真实键鼠、输入法、交接后的快捷键、连续拖宽/缩放/全屏是否闪烁，以及 `.app` 的 ATS 和持久存储。只凭本轮“页面显示出来”不能宣布完整 Browser 可交付。

## 2. 依据与调查边界

本仓库先读了 AGENTS、DESIGN（重点 §4、§7、§13 P5-13）、背景、HANDOFF、原型记录与 T76 参考方案。样稿入口 `RightBrowser.dc.html` 实际导入 `Right.dc.html`；后者约 175–187 行包含导航按钮、地址和默认浏览器入口。本轮不改样稿。

主要源码定位（行号以本轮读取版本为准，符号名可搜索）：

| 编号 | 来源与关键位置 | 证明什么 |
| --- | --- | --- |
| G1 | Cargo registry `gpui-pre-macos-0.3.8/src/window.rs`：`build_classes` 约 126、`GPUIView` 创建 1085、`addSubview` / `makeFirstResponder` 1196、`HasWindowHandle` 2411、`handle_key_event` 约 2640 | contentView 中放一个 GPUIView；原生句柄返回它；按键和输入法入口装在这个类上 |
| G2 | `gpui-pre-apple-0.3.8/src/metal_renderer.rs`：`new` / `from_layer` 约 149；`gpui-pre-0.3.8/src/window.rs`：`focus` 2346、`blur` 2359、`HasWindowHandle` 7435；`src/platform.rs` 的 `PlatformWindow` | Metal 渲染与原生句柄；GPUI 的 focus 只改内部焦点，没调用 `NSWindow.makeFirstResponder` |
| P1 | `app/src/frost.rs`：`install`、`fit`、`settle` | 已有原生兄弟视图接入；AppKit 与 GPUI 更新不同步已需要处理 |
| P2 | `app/src/right_panel.rs`：`Room::clamp`、`shown`、`render`；`app/src/window.rs`：`right_grip`、`palette_panel`、`Render::render` | 侧栏布局、拖宽、卡片裁剪；菜单/palette 是同一 GPUI 画面内的浮层 |
| P3 | `app/src/menu.rs`：`bindings`、`menus`；`app/src/view.rs`：`mouse_down`、`copy`、`paste_clipboard` | 现有宿主快捷键与终端输入路径 |
| W1 | [wry 0.57.0 源码包](https://static.crates.io/crates/wry/wry-0.57.0.crate)：`Cargo.toml`；`src/wkwebview/mod.rs`：`new_as_child` 179、安装视图 664–704、`set_bounds` 1031、`focus_parent` 1066、`open_devtools` 922 | 版本要求、子视图模式、主动激活、坐标/焦点和私有 Inspector 调用 |
| W2 | [objc2-web-kit 0.3.2 源码包](https://static.crates.io/crates/objc2-web-kit/objc2-web-kit-0.3.2.crate)：`Cargo.toml`；`src/generated/WKWebView.rs`、`WKNavigationDelegate.rs`、`WKUIDelegate.rs`、`WKWebsiteDataStore.rs` | 原生 API 与 feature 门控、对象/委托的生命周期要求 |
| C1 | [gpui-kit 固定提交 c0bebdc](https://github.com/longbridge/gpui-kit/tree/c0bebdc8a8cf9a74f9f52feaca278a52a9cfcca2/crates/webview)：README、Cargo.toml、`src/lib.rs` 的 `WebView`、`WebViewElement` | 实验状态、遮挡声明、set_bounds/隐藏/焦点恢复的现成封装 |

GPUI 包头和 Cargo 清单标为 Apache-2.0，快照对应 Zed `279fe070bb389b79652e52065b2f001edcc0b11b`。本轮没有读/复制 Zed `terminal`、`terminal_view`、`ui` 的应用层实现。外部源码先核对 Cargo license 或文件许可头；原型是本仓库原有插入方式加公开绑定调用，没有复制外部实现。

## 3. 方案比较

| 方案 | 做法和兼容性（已查证·源码，另注明实测） | 成熟度、维护、许可 | 判断 |
| --- | --- | --- | --- |
| **wry 0.57.0** | `WebViewBuilder::build_as_child(&Window)` 使用 raw-window-handle 0.6；macOS 后端仍是 WKWebView。要求 objc2 `^0.6.4`、AppKit/Foundation/WebKit `^0.3.2`，与本锁文件 0.6.4 / 0.3.2 兼容；并不要求把 objc2 0.5.2 换掉。仅核对声明和源码，没有在本工程编译/运行 wry | Tauri 使用的跨平台库；[官方发布页](https://github.com/tauri-apps/wry/releases/tag/wry-v0.57.0)显示 2026-09-08 发布，持续有修复；Apache-2.0 OR MIT，MSRV 1.85。库仍是 0.x；引擎成熟不等于 GPUI 集成成熟 | 有助于导航回调、下载、脚本和多平台；不能解决原生叠层。此次 macOS 需求不必先引入它 |
| **直接 objc2-web-kit 0.3.2** | 主线程创建配置和 WKWebView，经现有 raw handle 挂到 GPUI 同一 contentView；自己管理 bounds、隐藏和 responder。要求 objc2 `>=0.6.2,<0.8`、AppKit/Foundation 0.3.2；**本机已编译运行**，既有包均未升级 | Apple 系统 WebKit 是成熟的网页引擎，随 OS 更新；绑定是较底层的 0.x API，不提供成品浏览器。包许可 Zlib OR Apache-2.0 OR MIT、MSRV 1.71；[索引](https://github.com/rust-lang/crates.io-index/blob/master/ob/jc/objc2-web-kit)的最新该包仍为 2025-10-04 的 0.3.2；[objc2 上游](https://github.com/madsmtm/objc2/commit/78216adc63fd8616cdfe0360ec628dc54a3fa9f4)在 2026-10-05 仍有维护 | **推荐**。和 frost 的边界相近，没有额外跨平台包装；代价是自己写导航/UI delegate、KVO 和焦点适配 |
| **WKWebView 快照作为 GPUI 图片** | `takeSnapshotWithConfiguration` 返回原生图片，转换成 GPUI 图像后在 Metal 内合成；绑定同上，增加 `WKSnapshotConfiguration` / `block2` 等 features。未做原型 | 快照是公开 API，许可/维护同上；它不是连续帧 OSR 接口。未找到该版本 WKWebView 可直接给 GPUI 共享纹理并完整转发输入的公开接口 | 可作浮层期间的静态替身；不推荐把整个交互浏览器做成定时截图。延迟、频率、视频/WebGL、IME、无障碍和事件转发全需另验 |
| **GPUI 本身** | 调查 0.3.8 的 Window、PlatformWindow、Element/Canvas 与 macOS 实现，未找到 WKWebView、NativeView element 或自动参与布局/遮挡/焦点的外部视图宿主。可用入口是 raw handle + 自定义元素拿布局边界 | Apache-2.0；仍 pre-1.0；已有的 0.5/0.6 objc2 共存不妨碍用裸指针作为边界 | 能提供接入口，不能独自渲染网页；此结论不覆盖其他版本、分叉和未来 API |
| **gpui-wry 0.7.1** | [发布索引](https://github.com/rust-lang/crates.io-index/blob/master/gp/ui/gpui-wry)为 2026-10-05；精确依赖 gpui-pre `=0.3.8`，所以**这一版与本项目版本声明相符**。用的是 `lb-wry ^0.53.3`，不是上面的官方 wry；后者 objc2 `^0.6`、AppKit `^0.3.0` 可容纳 0.3.2。未编译验证 | Apache-2.0；lb-wry Apache-2.0 OR MIT。gpui-kit 活跃，但封装 README 仍标 experimental；lb-wry [索引](https://github.com/rust-lang/crates.io-index/blob/master/lb/-w/lb-wry)最后发布 0.53.3 在 2025-10-09，不能将主仓库活跃等同于分叉持续同步 | 可参考接线方式，不推荐只为 Browser 增加这层。README 明说网页覆盖 GPUI，并建议独立窗口/Popup；不会替 paddock 解决本轮主要难题 |
| **CEF / wef 离屏** | CEF 提供真正离屏渲染方向；需要新的引擎运行时、纹理/像素和输入接线，不是当前 objc2 版本冲突能否解决的问题。没有核对完整依赖解析，也未安装/构建 | [wef 官方 README](https://github.com/longbridge/wef)称实验尚不满意、CEF 带来约 1GB 体积、实际仍用 wry；页面只有 3 次提交记录，不作为成熟集成推荐。wef Apache-2.0；[CEF 自身为 BSD 三条款](https://github.com/chromiumembedded/cef/blob/master/LICENSE.txt)，Chromium 还有第三方许可，本轮未完成分发许可清点 | 工作量明显超过本轮本地网页侧栏；不推荐。不能借成熟 Chromium 宣称这份 GPUI 桥接成熟 |

**wry 的额外实证风险**：W1 `new_ns_view` 在 `if is_child` 分支之后，无条件调用 `NSApplication::activate`（旧系统用 `activateIgnoringOtherApps`）。`with_focused(false)` 不绕过这段调用；`PADDOCK_NO_ACTIVATE` 是 paddock 自己的约定，wry 不认识。lb-wry 0.53.3 也有同类调用。因此本轮没有启动这两个候选库的原型。正式采用需先解决后台创建抢前台的问题，不能默默接受它。

两套 objc2 是两个不同的 Rust 类型体系，不能直接交换它们的 `Retained` 或 `NSView` 类型。已有 raw handle 是 `NonNull<c_void>` 边界；主线程上按真实 Objective-C 类借用成 0.6 的类型即可，和 frost 一致。**不要为了去重顺手升级 GPUI 或旧 objc2 依赖链。**

这些 macOS WKWebView 路线调用系统 WebKit，不另打包一套浏览器引擎；绑定 crate 的开源许可不等于整个系统 WebKit 的分发许可结论。wry 的 child 构造接口接受 `HasWindowHandle`，不要求把 GPUI 的窗口换成 tao；但实际并用的事件/生命周期仍待验证。

## 4. 原生视图与 GPUI 的交界

### 4.1 层级与浮层

本原型的次序由上至下：WKWebView → GPUIView/Metal → NSVisualEffectView。网页局限在 Browser 内容矩形，但矩形内会挡住整个 GPUI 画面。Frost 能正常叠在底下，不能据此推出交互网页放在上面也没有问题。

| 对象 | 结论与依据 |
| --- | --- |
| palette 和压暗遮罩 | **已查证·原型**：`palette.png` 右侧被网页切掉，网页没有被压暗；`hidden.png` 隐藏网页后完整 |
| GPUI 新标签/分屏/Attention/底部菜单、tooltip | **已查证·源码**：它们都在同一个 GPUI 画面中；一旦几何范围与网页相交，就无法画在原生网页之上。是否碰巧相交取决于窗口与位置；本轮没有逐个截图 |
| 拖分割线和拖动浮层 | **推测**：网页原生命中区也会接走经过它的鼠标，既影响遮罩又可能影响拖动完成。建议开始宿主拖动就隐藏网页，结束恢复；不能只让 GPUI 的 `occlude` 挡事件。真实拖动未知 |
| macOS 原生菜单、系统 sheet、独立 Settings/About 窗口 | **源码/平台结构判断**：不是同一 Metal 画面内的层级，可走系统窗口/菜单合成；不等于所有弹窗都被网页盖住。此次未验证其激活/返回焦点 |

可行处理方式：

1. **建议首版**：由窗口统一报告“网页应显示吗”：Browser 激活、面板展开、bounds 非空，且无需要覆盖它的宿主浮层/拖动。隐藏用 `setHidden`，保留 WKWebView 实例和历史；显式交还原生 responder。对 tooltip，可选择放在网页外或在重叠时隐藏网页，不能漏管。原型只验证 palette 的静态隐藏，没有完成这个总开关。
2. **推测，可后做**：隐藏前异步取快照，GPUI 画静态替身再画浮层。不要为了等快照延迟显示 modal；无快照/失败时先用面板底色，丢弃过期导航/尺寸的回调。恢复真网页的时机也须与绘制协调。[快照 API](https://developer.apple.com/documentation/webkit/wksnapshotconfiguration)可用，原型未实现，不能承诺无白闪。
3. **不建议首版**：把浮层搬到原生子窗口，或网页放 GPUI 下方再给 Metal 打透明孔。前者增加坐标、全屏/Spaces、焦点和窗口生命周期；后者即便画面透过去，GPUIView 仍覆盖命中区，需要改 native hitTest/GPUI 平台实现。都超出“照 frost 插一块视图”的工作量。

### 4.2 裁剪、位置与大小

- **已查证·原型**：在 Browser 内容的 canvas `prepaint` 得到最终 `Bounds<Pixels>`，转换为 AppKit points；非 flipped 父视图用 `parent_height - top - height`。不乘 Retina scale：这里两边都是逻辑点。两种预设宽度中，网页 JS 的 `innerWidth` 从 406 到 606，与侧栏增加 200pt 相符。
- **建议**：正式版以实际布局内容区为唯一 bounds 来源，扣掉工具栏、边框；在这一层更新 native frame，避免另算一套右侧栏位置。标签切到 Changes、收起、空区域或销毁时应隐藏/移除；保留对象还是卸载由产品决定。本轮源码有可见性条件，但没做交互切换测试。
- **已查证·源码**：`right_panel::Room` 保留 360pt 终端空间并限制侧栏占比；窗口的 `.overflow_hidden()` 和 GPUI 圆角只裁自己的图元，不裁原生兄弟视图。
- **建议**：原生 host 用 layer 的圆角/mask（仅网页区域所需的角）裁剪 WKWebView，GPUI 仍画卡片/工具栏边框；网页 frame 留足边框 inset。本原型只留 6pt 内边距、网页矩形仍是直角，**没有验证圆角方案**。
- **未知**：连续拖宽、点加宽、窗口缩放、跨屏 scale 变化、全屏过渡、切标签/收起时是否有一帧错位或闪烁。Frost 的 `fit/settle` 已说明 AppKit 与 Metal 提交可不同步；静态截图不能代替这些测试。不要只靠 NSView autoresizing：侧栏宽度与标题栏高度并非固定窗口比例。

### 4.3 焦点、键盘和输入法

**已查证·源码**：GPUI 的 `Window::focus`/`blur` 只改变内部 focus handle；原生 firstResponder 是另一套状态。G1 在创建窗口时设置 GPUIView 为 firstResponder；WKWebView 内部又有自己的输入视图。单独调用终端的 `window.focus()` 不足以证明从网页拿回原生键盘。

**不能简单说“所有快捷键都丢失”。** GPUIView 自己实现 `performKeyEquivalent:`，GPUI 还给原生菜单填了快捷键（`gpui-pre-macos/src/platform.rs` 约 429）。[AppKit 文档](https://developer.apple.com/documentation/appkit/nsview/performkeyequivalent(with:))说明默认实现向子视图分发。因此有些组合仍可能进入 GPUI，甚至进入它遗留的终端 focus 路径；实际优先级本轮未交互验证。

| 操作 | 已有事实 | 推荐的正式行为与待验内容 |
| --- | --- | --- |
| 网页打字、中文组字 | WKWebView 是原生网页视图，有系统输入处理；paddock 终端有自己的 InputHandler | **推测**原生网页输入可正常工作；必须人工验中文候选框、组字中切换、组合字符、Esc，不能从终端 IME 已工作推出网页也过关 |
| ⌘C / ⌘V | paddock 全局有 Copy/Paste action，终端会据此复制或往 PTY 粘贴 | 网页焦点时应让 WebKit 执行标准编辑，不能误发给仍保有 GPUI focus 的终端。实际当前路由 **未知** |
| ⌘L / ⌘R | 当前 paddock 没有 Browser 对应绑定；WKWebView 不是带地址栏的 Safari 窗口 | **建议** Browser 内 ⌘L 聚焦 GPUI 地址栏并取回原生 responder，⌘R 调 WK reload；切回终端后不强占终端语义 |
| ⌘W | 本仓库绑定的是 **ClosePane**，不是关 Browser/关窗口；⌘⇧W 才是 CloseTab | **建议**保留宿主语义并经既有确认流程；网页有焦点时能否调用、要不要改成关闭 Browser，需要产品明确，不能由包装库默认值决定 |
| ⌘T、⌥⌘B、⌘P、⌘B | 分别是 NewTab、ToggleRightSidebar、Search、ToggleSidebar，存在原生菜单/GPUI action 路径 | **建议**这些继续是宿主入口；网页焦点时逐项人工验证，不能被网页吞掉、触发两次或走到旧终端 |
| 终端 ↔ 网页 ↔ 地址栏/浮层 | 两套焦点；W1 `focus_parent` 明确调用 `makeFirstResponder`，C1 在隐藏/点网页外时调用它 | **建议**统一持有“当前原生输入属于谁”，进入 GPUI 时先 `makeFirstResponder(GPUIView)` 再更新 GPUI focus；隐藏前检测 firstResponder 是否属于该网页子树。不要只比较 responder 是否等于 WKWebView 本体 |

建议先用仅本应用的事件/菜单路由，按窗口与网页 responder 子树筛选，明确少量宿主快捷键；需要派发 GPUI action 时避免在 native 回调里重入当前窗口更新。**这是待实现方案，不是已验证修复**；不要直接把网页所有 NSEvent 都转交 GPUI，不然会破坏编辑、输入法和网页自己的快捷键。gpui-wry 的[公开焦点问题 #1787](https://github.com/longbridge/gpui-component/issues/1787)是旧版本的风险旁证，不作为当前版本必现的证据。

### 4.4 滚动、缩放、拖放、右键和 Inspector

| 项目 | 判断 |
| --- | --- |
| 滚动、网页右键 | **推测**原生 WKWebView 可承接触控板/滚轮和默认网页上下文菜单；本轮只见到滚动条，没有操作。WebKit 的原生菜单与 GPUI 自画菜单不是同一层。跨边界/惯性滚动和宿主菜单交接未知 |
| 页面缩放 | **已查证·API**：`pageZoom`、`magnification`；`allowsMagnification` 默认 false，需显式选择是否允许手势。[Apple 文档](https://developer.apple.com/documentation/webkit/wkwebview/allowsmagnification)。⌘+/⌘−/⌘0 需要宿主接线，不能只从这些属性推断已支持 |
| 文件/文字拖放 | **推测**WebKit 可处理网页原生拖放；但文件 `<input>` 的选择面板需 WKUIDelegate `runOpenPanel…`，拖出/拖入和 P5-9 终端文件粘贴不可双重处理。wry 的 `with_drag_drop_handler` 是可选拦截，并非必须启用；本轮未操作 |
| 开发者工具 | **已查证·文档**：macOS 13.3+ 的公开 `isInspectable=true` 可从 Safari Develop 菜单检查。[WebKit 官方说明](https://webkit.org/blog/13936/enabling-the-inspection-of-web-content-in-apps/)。**不等于有公开的“在本面板内打开 Inspector”按钮**：wry 使用 `developerExtrasEnabled` 和 `_inspector` 私有接口。本原型没启用/打开 Inspector；正式版建议先用 Safari 入口 |

本项目 bundle 目前声明最低 macOS 13.0（`app/examples/bundle.rs`），所以 `setInspectable` 必须做系统版本/selector 检查，不能因本机 27.0.1 可用就无条件调用。不能暗中抬高系统要求。

### 4.5 磨砂和外观

**已查证·原型**：Dune 界面的左侧磨砂窄条与右侧不透明本地网页能同时显示。WKWebView 没替换 contentView，也没改 frost 的底层位置。

**建议/推测**：网页内容通常保持不透明，不应通过 KVC 私有 `drawsBackground` 把任意网站改透明；原生网页视图的 `NSAppearance` 可随 paddock 明暗主题明确设置，滚动越界底色可用公开 `underPageBackgroundColor`。网站最终深浅仍由其 CSS/`prefers-color-scheme` 决定，不能承诺任意页面跟随 paddock 配色。**未知**：亮色主题、全屏不磨砂、快速主题切换、原生右键/表单弹窗外观与加载白闪，本轮都没测。

## 5. 网页功能能做到什么

| 功能 | 已查证的接口/现状 | 正式接线与本轮边界 |
| --- | --- | --- |
| 后退/前进/刷新 | W2 `canGoBack`、`canGoForward`、`goBack`、`goForward`、`reload`、`stopLoading` | 工具栏需更新 enabled/loading；本原型没做按钮，也没导航第二页 |
| 地址栏与页面标题 | W2 `loadRequest`、`URL`、`title`，可 KVO/导航回调 | 地址输入先规范化 localhost/协议；显示导航最终 URL，处理重定向和同页变化，不只保留初始字符串；支持哪些 scheme 待定 |
| 加载进度/失败页 | `estimatedProgress`、`isLoading`，WKNavigationDelegate 的 didStart/didCommit/didFinish、didFailProvisionalNavigation/didFailNavigation、webContentProcessDidTerminate | [官方 delegate](https://developer.apple.com/documentation/webkit/wknavigationdelegate)可报告网络/进程失败。应用自己做可重试错误状态；HTTP 404/500 是网页响应，不等于网络失败。原型没有错误页 |
| 默认浏览器打开 | GPUI `App::open_url` 已有系统入口；原生也可 NSWorkspace | 应打开当前已确认的页面 URL，而非未提交的地址栏草稿；本轮没有真正打开外部浏览器 |
| 新窗口、JS 弹窗、上传 | WKUIDelegate 有创建新 webview、alert/confirm/prompt、open-panel 回调 | WKWebView 不是全套 Safari UI；需要明确同面板/外部浏览器的策略。页面显示成功并不包含这些场景已完成 |

### 5.1 localhost、HTTP 与 ATS

**已查证·原型**：未打包 debug 二进制在 macOS 27.0.1 成功打开 `http://localhost:<随机端口>/`；仅访问临时 Rust HTTP 服务，JS 执行。最终一次服务记录 `GET / HTTP/1.1`。这**不是**已安装 paddock.app 的 ATS 验证，也没有测试 127.0.0.1、IPv6、LAN 地址、公网 HTTP、WebSocket/HMR 或 HTTPS 证书。

**已查证·文档**：[NSAllowsLocalNetworking](https://developer.apple.com/documentation/bundleresources/information-property-list/nsapptransportsecurity/nsallowslocalnetworking)涉及无点域名、`.local` 和 IP；Apple 明确列出 macOS 14 起 IP 默认行为变化以及 `NSExceptionDomains` 的 IP/CIDR 例外。不能把 localhost 域名的一次成功推广成所有 IP 都成功。

**建议**：正式本地预览版在 bundle 里明确声明本地网络意图，针对实际支持的 localhost、loopback IP 做打包后的验证；不先全局打开 `NSAllowsArbitraryLoads`。如果用户要访问任意 HTTP 网站，可讨论只对 web content 的 [NSAllowsArbitraryLoadsInWebContent](https://developer.apple.com/documentation/bundleresources/information-property-list/nsapptransportsecurity/nsallowsarbitraryloadsinwebcontent)，它不放宽 URLSession。ATS 例外也不自动解决自签名 HTTPS 证书、混合内容、CORS 或本地网络隐私权限。当前打包模板没有 ATS 项，本轮按任务没有改/重新打包。

### 5.2 Cookie、存储与关闭后重开

**已查证·文档**：[WKWebsiteDataStore](https://developer.apple.com/documentation/webkit/wkwebsitedatastore)默认 store 持久保存网站数据；`nonPersistentDataStore` 用内存；macOS 14+ 可用带 identifier 的独立持久 store。store 在建 WKWebView 前配到 configuration，两个 view 是否共享数据取决于是否用同一 store；不能假定继承 Safari 登录态。

**放在哪**：公开 API 管 store，不把磁盘布局作为应用配置契约。读取 [WebKit 的 Cocoa 存储源码](https://github.com/WebKit/WebKit/blob/main/Source/WebKit/UIProcess/WebsiteData/Cocoa/WebsiteDataStoreCocoa.mm)（2026-10-07，文件头为 BSD 风格许可）可见：非 sandbox 默认网站数据采用用户 Library 下 `WebKit/<application-or-process-id>/WebsiteData/`，缓存在 `Caches/<application-or-process-id>/WebKit/`；容器和命名 store 的路径不同，cookie 还有独立的配置/系统路径处理。**本机已安装 app 的确切 cookie 文件位置未知**，没有读其数据；不建议去读/写 WebKit 内部文件来维护清理功能，应使用 store API。

**重开语义必须拆开**：隐藏/收起但保留 WKWebView，页面状态与前进后退历史仍在对象里；销毁/重启后，持久 cookies/localStorage 等可保留，但不保证 session cookie、JS 内存、sessionStorage、滚动位置、导航历史和当前 URL 全部恢复。后两类需要单独的页面/会话恢复设计。本原型显式用 nonpersistent store，每次全新进程；没有验证持久化。

**建议首版**：普通持久 store 供本地开发登录，收起/切标签只隐藏、不重建；是否按项目隔离由用户决定。别拿不同 localhost 端口当全面的账户隔离：Web Storage 按 origin 区分，cookie 不以端口隔离（[RFC 6265 §8.5](https://www.rfc-editor.org/rfc/rfc6265#section-8.5)）。原型继续非持久，免得测试留下登录数据。`wry::WebContext` 的自定义数据目录在 WKWebView 上并不等价可用；它提供 macOS 14+ 的 `with_data_store_identifier` 替代，不能照搬 Windows 的 data_directory 方案（W1 `src/lib.rs` 约 1615）。

## 6. 最小原型与验证记录

原型独立提交 `9422921` 保留在本分支，**禁止将代码提交整体合入 main**。正式 Browser 功能仍为空；仅设置 `PADDOCK_BROWSER_PROBE_URL` 才创建 WKWebView。另两个 research 开关用于启动时显示现有 palette、选择是否隐藏网页，都是直接设置应用状态，**未模拟键盘或鼠标**。

改动：新增 `app/src/browser_probe.rs`；window 持有/显示/隐藏原生视图；right_panel 给它一个内容区域；lib 注册模块。临时直接依赖 `objc2-web-kit =0.3.2` 与 `objc2-foundation =0.3.2`。Foundation 之前已在锁文件里；锁文件只新增 **一个包** WebKit，其他包版本不变。没加 wry、QuartzCore 新直接依赖或 Inspector/快照 features；没做圆角、工具栏、委托和输入转发。

环境：macOS 27.0.1 (26A434)，rustc 1.96.0。前台构建成功：

```sh
CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-13r-browser cargo build --manifest-path app/Cargo.toml
```

首次构建有一次原型编译错误（误选 GPUI 自带同名 `window_handle`，改成 `HasWindowHandle::window_handle` 后通过）；不是 RED→GREEN 行为证据。最后构建只有既有传递包 `block 0.1.6` 的 future-incompat 提示。按本任务限定，只做源码/文档和原型构建截图，没有跑全量 test/clippy，没有声称正式回归已通过。

scratchpad：`/tmp/paddock-browser-research-P3VKdQ/`，保留 Rust 监督程序 `run_probe.rs` / 可执行文件 `run_probe`、临时 HOME、外部源文件与日志。监督程序在前台运行，内部起 `127.0.0.1:0` 服务和一个子应用，记住 PID；截图后按该 PID 停止并 wait，服务线程退出 join。应用使用临时 HOME/CFFIXED_USER_HOME/XDG 目录、假 corral（空 agents）、预写空布局，没有启动 shell/真实 agent。仅从自己的应用日志取 window ID，用 `screencapture -x -o -l <ID>`，没有全屏截图。运行必带 `PADDOCK_NO_ACTIVATE=1`。

| 场景 | 应用 PID / window ID | 截图（均不入库） | 实际观察 |
| --- | --- | --- | --- |
| 420pt Browser | 63475 / 24259 | `normal.png` | 页面显示，JS 给出 406×698 |
| palette 与网页共存 | 63856 / 24263 | `palette.png` | 右侧 palette 被覆盖，网页不受 GPUI 压暗背景影响 |
| palette 时隐藏网页 | 64235 / 24267 | `hidden.png` | palette 全部可见；网页处为底色 |
| 620pt Browser | 64904 / 24271 | `wide.png` | 页面 606×698，增加的宽度与布局一致 |
| 补 HTTP 请求日志，同 420pt | 65848 / 24275 | `normal2.png` | 请求日志有 `GET / HTTP/1.1`，页面/JS 正常 |

前四次监督程序未把 accept 得到的 socket 改回阻塞，read 日志为空（仍发送了页面响应）；为避免把“没有日志”当请求证据，修正监督程序后补了 normal2。所有截图逐张查看过，全部只含合成数据与本原型窗口。相关程序均已结束，临时文件保留。

复现（需当前本地 scratchpad；不是正式测试入口）：

```sh
rustc --edition=2024 /tmp/paddock-browser-research-P3VKdQ/run_probe.rs -o /tmp/paddock-browser-research-P3VKdQ/run_probe
/tmp/paddock-browser-research-P3VKdQ/run_probe normal2
/tmp/paddock-browser-research-P3VKdQ/run_probe palette
/tmp/paddock-browser-research-P3VKdQ/run_probe hidden
/tmp/paddock-browser-research-P3VKdQ/run_probe wide
```

截图只证明这些启动状态，不证明交互 transition。没有激活/抬起窗口来补帧；如果之后窗口被完全遮住，GPUI 可暂停绘制，不能把旧帧判断为产品错误。

## 7. 建议如何拆正式任务

以下是工作拆分建议，**不是新增验收条件，也不是现在开工授权**。先难点后功能；不需要并行派给多个 agent 才能做。

| 顺序 | 一件活的边界 | 相对工作量 / 主要风险 |
| --- | --- | --- |
| 1 | 焦点/快捷键接入：网页、终端、地址栏的双重焦点交接，宿主快捷键优先级，现有关闭确认照旧；提供人工交互样例 | 大，最高风险；⌘V 误进终端、IME 组字中切换、菜单 key equivalent 重复派发。先由用户实际键盘验，再继续完善浏览器 |
| 2 | 原生 host 的生命周期/布局：真实内容 bounds、native 裁剪、拖宽/窗口/全屏同步、收起/标签隐藏、全部 GPUI 浮层/tooltip 的可见性协调 | 大；不同渲染提交时序、网页命中区吞拖动、漏掉浮层。先用隐藏策略，不含快照优化 |
| 3 | 样稿工具栏：后退/前进/刷新/停止、地址栏、当前 URL/标题/进度、错误状态、默认浏览器入口 | 中；异步回调过期、失败与取消混淆、重定向/同页导航。底层导航 state 与按钮连通后再细调样式 |
| 4 | 本地网页实际使用策略：经批准的 ATS 打包项、store 生命周期与清理、popup/文件选择/JS 对话框边界、公开 Inspector 入口 | 中；最低 macOS 版本、登录状态误共享、网络/页面权限策略、把裸二进制结果错当 bundle 结果。每项范围先写任务给主控/用户看 |
| 可选后续 | 浮层前的快照替身、恢复时机和过期回调处理 | 中到大；快照异步、视频/动图停帧、首次空图和额外内存。只在用户不接受隐藏底色时另议 |

## 8. 要主控/用户决定的事

1. 是否批准正式采用 **直接 WKWebView + objc2-web-kit 0.3.2**（Foundation 0.3.2 从传递变直接依赖），接受这部分先为 macOS 接线。现有 DESIGN §7 的平台范围与 GPUI 渠道不因本文改变。
2. 首版浮层/拖动时短暂隐藏网页露出底色是否可接受；如果不能接受，需要给快照方案单独的验证任务，不能凭这轮截图承诺平滑。
3. Browser 页面生命周期与数据：一个窗口一个页面还是随终端/项目切换；是否保留 URL、是否持久登录、是否项目隔离。建议先每窗口一个页面、普通持久 store、隐藏时保留实例，但都未获本任务批准。
4. 快捷键分配：建议保留 ⌘W/⌘T/⌥⌘B/⌘P/⌘B 宿主语义，Browser 内加 ⌘L/⌘R，⌘C/⌘V 给当前原生编辑目标。
5. 首期网址范围（只本地/HTTPS，还是任意 HTTP），以及是否接受 Safari Develop 作为 Inspector 入口，不使用私有内嵌 Inspector API。

本轮没有需要先停下才能做调研的权限/系统安装阻塞。上述决定属于后续正式开发；本轮已授权的调研和分支原型已完成。不合并、不推送、不动 main、不安装 app、不接触用户 agent/真实配置和网站数据。
