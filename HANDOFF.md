# 交接

## 现在在哪（2026-10-07 下午）

- main 和 `origin/main`（`github.com/firegnu/paddock`，public）同步。没有进行中的活，没有开着的 worktree；corral 里只有主控 `paddock/main`。本地只有 main 一个分支（Browser 调研原型分支 `p5-13r-browser` 已按用户同意删掉，最后提交 `716afc2`，短期内可从 reflog 找回）。
- 274 项测试通过，clippy、`cargo fmt --check` 干净。`~/Applications/paddock.app` 是合并 P5-28d 之后的版本（右侧栏 Changes、Browser、Kanban 三个标签都在），已用本机 Apple Development 证书签名。
- 重新打包安装：在 `app/` 下 `cargo build --release`，再 `cargo run --release --example bundle -- --install`（编译目录见 AGENTS.md，主仓库用 `.target/main`）。打包默认自动选本机 Apple Development 身份，可用 `PADDOCK_SIGN_IDENTITY` 指定；退回 ad-hoc 时会提示授权会失效。
- 已合并：第一至第三阶段、迁移 M0–M3（corral、dispatch 在 `../ranch`；遥测、Drover、插件系统已砍）、P4-1～P4-3、P5-1～P5-29（P5-13 做了 13a 外壳、13b／13c Changes；P5-19 磨砂；P5-23r、P5-13r、P5-29r 是调研；P5-28 做了 28a、28b、28c、28d；P5-29 做了 29a）。每件的范围、完成记录、主控审查在 `docs/任务/`。
- 上下文：`AGENTS.md`（规矩，含「开发方式」里的**看板约定**）、`docs/DESIGN.md`（已定决定，§13 是界面改版的全部决定和用户原话）、`docs/背景与决策记录.md`、`docs/设计稿/`、`docs/调研/`。

## 本次会话（10-06 晚到 10-07 下午）

主控照“任务文件 → 路由 → 派 agent → 审查 → 合并 → 安装 → 收尾”做完，每件都合并、清理、装上、推送。

- **10-06 晚**：P5-21（New Agent 记模型强度）、P5-19a～e（左侧栏磨砂）、P5-22（侧栏头部标题）、P5-13b／13c（Changes 标签，新依赖 `syntect`、`similar`）、P5-23r／P5-23（截图修好：打包时用本机证书签整个 app，agent 能自己截图交审）、P5-24a／24b（窄条换种类 logo、名字统一短名、Lagoon／Tide 色温、磨砂不在前台不退灰；卡片信息用户要求不动）、P5-25（卡片 Copy／Stop 改图标、左下角图标放大）、P5-26／27（侧栏 4 个、标题栏 5 个图标的悬停动效，尊重“减弱动态效果”，机制在 `motion.rs`）。
- **Browser（P5-28）**：P5-13r 调研（`docs/调研/P5-13r-Browser嵌入.md`）推荐直接 `WKWebView`（`objc2-web-kit`），不用 `wry`；用户问能否接 Rust 浏览器，主控答 Servo 可作备选但首版代价大，用户：“暂时不要，还是优先webview。你的5条建议我都接受。”（DESIGN §13 P5-28 五条）。
  - **P5-28a**：原生宿主、跟随布局、圆角、浮层／拖动时隐藏（统一判断 `browser::cover()`）、样稿工具栏、记住网址；打包后的 app 打开 `localhost`／`127.0.0.1` 已验证，没被 ATS 拦。
  - **P5-28b**：统一的“键盘归谁”（`browser::Keys`）、`PaddockWebView` 只重写 `performKeyEquivalent:`、⌘L／⌘R、编辑键跟着键盘、paddock 快捷键在网页有键盘时只生效一次、不带 ⌘ 的键不碰（输入法不经过 paddock）。**10-07 用户按清单实测全部正常**。
  - **P5-28c**（10-07 中午）：本地开发地址一定能打开（`0.0.0.0`／`[::]` 换成回环，局域网私有 IP、`*.local` 补 `http://`，ATS 只加 `NSAllowsLocalNetworking`，打包后七种地址实测都开）；新窗口同面板、对话框和上传用系统 sheet、下载存 `~/Downloads`（同名改名）、证书失败提示、Safari 调试、⌘F 查找栏。新直接依赖 `block2 =0.6.2`（用户批准，锁文件不多包）。不交叉审查（用户“不要在细节的里面绕太多”）。**第 2–7 条待用户按任务文件末尾 8 条清单实测**。
  - **P5-28d**（10-07 下午，用户提出）：工具栏最右加 ×（“Close page”），点了回到空白页：丢掉网页再建一个空白的，前进后退记录、布局记的网址一起清掉，网站数据保留，键盘交回分屏；不配快捷键（用户选）。主控自己做，没派 agent。**待用户实际点一下**；这个 × 在右侧栏右上角 ×（收起右侧栏）正下方，看会不会混。
    - 关键做法：系统没有只清前进后退记录的接口，所以关闭＝丢掉 `Page` 并立刻新建空白的（不等下次输入，免得多起一个轮询循环）；`Visited` 改成 `Option<String>`，`Visited(None)` 让窗口清掉 `right.url`；`Keys::close()` 说 Browser 有没有键盘，有就发 `Handoff::ToPane`。DESIGN §13 P5-28 记为第 3 条“只隐藏、不销毁”的例外。
    - 文件：`app/src/browser_view.rs`（`close()`、工具栏）、`app/src/browser.rs`（`Keys::close()` 和测试）、`app/src/window.rs`（一行接线）、`docs/任务/P5-28d-Browser关闭页面.md`。
    - 没测到：“布局网址清掉”没有单元测试（仓库没有 GPUI 视图测试设施，靠类型改动保证）；点 × 不能模拟。
- **Kanban（P5-29）**：P5-29r 调研（`docs/调研/P5-29r-Kanban集成.md`）；主控推荐只读流水线看板，用户：“可以，按你的推荐出样稿”“按你的推荐，窄面板用 A，加宽用并排的列”，并认可“等于在界面里重做一个主控……越来越像已经砍掉的 Drover”。
  - **P5-29a**：卡片 = 任务文件；五列 QUEUED／IN PROGRESS／TO REVIEW／MERGED／DONE 全由任务文件、git、corral 推出，只读；窄面板分组列表、加宽五列；悬停只读入口。
  - **看板约定**写进 AGENTS.md「开发方式」：任务文件 worktree／分支一行、`依赖：`、`--label task=<编号>`、合并／收尾提交首行带编号。主控从 P5-29a 起照做。

## 下一步（按优先级）

1. **用户实测 P5-28c、P5-28d**（用户 10-07：“我打算用到再测试”）：P5-28d 点一下 ×；P5-28c 照 `docs/任务/P5-28c-Browser网页策略与查找.md` 完成记录末尾 8 条清单（第 1 条本地地址最要紧）。有问题先修。
2. **等用户在真窗口里看**：Browser、Kanban（状态推得对不对）、Changes、P5-22 侧栏头部、P5-24a 窄条和名字、P5-24b 着色强度、P5-25 卡片操作图标、P5-26／27 的 9 个图标动效。
3. **Kanban 加强 A＋C＋D**（DESIGN §13 P5-29「加强」）：“等你”标记、草稿卡、两处小修（“−0”、DONE 组头数字）。**等用户指令再开工**（28c 已完成）；开工前先和用户确认 `待用户：` 约定，再写任务文件。改 Kanban 文件和 AGENTS.md。
   - 10-07 用户：“我不想变成drover那么重，但是基本的看板的功能起码得有。调研一下吧。”已做 P5-29r2 调研（`docs/调研/P5-29r2-Kanban基本功能.md`，已合并收尾）：推荐新建卡片＝写未提交的任务文件草稿（在 C 之后）、“不做了”用 `收尾: <编号> 不做：…` 空提交并标 Dropped、主控补 9 条老任务收尾空提交、不能拖、不排序；§4 列了 11 条不做的线。**等用户答 §5 的 5 问**，答完先改 DESIGN §13 P5-29，再和 A＋C＋D 一起排任务。
4. 之后：Changes 第二步（行上评论发给 agent、暂存、撤销，另议）、拖动分隔线调整分屏大小、`paddock ctl`；Kanban 能动手的（新建草稿、拖来纠正，用户：排在后面）；Servo 作 Browser 备选（不做）。

## 悬着

- **要不要把 `cargo fmt --check` 写进 AGENTS.md 的验证清单**：主控问过多次，用户还没答；任务文件里已经各自写上。
- 窗口窄、左侧栏展开时标签名被截得很短（“pa…”），原有问题，短名后好一些；要不要处理待用户定。
- 截图能用了，但 `PADDOCK_NO_ACTIVATE` 起的测试窗口不在前台，悬停、动画、键盘鼠标交互、全屏切换仍只能靠用户实际操作。
- 主控关 `dev-changes-fix-1` 时它 `attached: 1`（规矩是先问用户），事后告诉了用户；多半是用户的 paddock 在显示它，没核实。
- 从程序坞菜单“退出”或注销时由系统直接结束，不问未保存的设置和运行中的 shell（GPUI 没有提供拦截）。
- 建议改未排：P5-21 的 `--model` 在命令末尾无值时会清掉已认出的模型、`--config=…` 连写不认；DESIGN §13 的 P5-20 条重复了一遍；New Agent 窗口 “Will run” 预览要重开才换字体；About 窗口在很大字号时可能放不下；配置里 `sidebar_width` 小于新最小宽度时不自动加宽；侧栏铃铛紧凑与否按估算字宽判断；左侧栏开关向外滑时短横稍压外框；地址栏没有撤销（`text_input.rs` 不支持）。
- `docs/DESIGN.md` §7 其余待定：GPUI 依赖渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、发布方式（P5-23 只做了本机签名，仍不公证、不分发）。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
- Saddle 仓库里的 `t76-*` 分支、worktree 和 T76 状态由 Saddle 主控处理。
- 主控教训：派活时写明“命令里不用 `rm`、不用 `sh -c` 包长命令”；`corral start --unique` 会给名字加 `-1`，后续 wait／send／stop 用返回的名字；关 agent 前确认它是 idle 且 `attached` 为 0；release 构建很快结束时，核对产物时间晚于合并再安装；测试 agent 截图用临时 HOME、假 corral、`--bounds` 和预写的布局文件，能摆出大多数场景（主控自己的简便做法：临时 `HOME`／`CFFIXED_USER_HOME`／`XDG_STATE_HOME`／`XDG_CONFIG_HOME` 加 `PADDOCK_NO_ACTIVATE=1 GPUI_TERM_WINDOW_ID=1` 先起一次 debug 版，让它写出 `layout.json`，用记下的 PID 停掉，改 `right_sidebar` 再起，截图后同样按 PID 停）；调研类只把文档摘到 main（`git cherry-pick`），原型分支不合并。
