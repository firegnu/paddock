# 交接

## 现在在哪（2026-10-06 夜）

- main 和 `origin/main`（`github.com/firegnu/paddock`，public）同步。没有进行中的活，没有开着的 worktree、任务分支；corral 里只有主控 `paddock/main`。
- 231 项测试通过，clippy、`cargo fmt --check` 干净。`~/Applications/paddock.app` 是合并 P5-24a 之后的版本（含 P5-24b、P5-13c、P5-23、P5-13b、P5-22），已用本机 Apple Development 证书签名。
- 重新打包安装：在 `app/` 下 `cargo build --release`，再 `cargo run --release --example bundle -- --install`（编译目录见 AGENTS.md，主仓库用 `.target/main`）。打包默认自动选本机 Apple Development 身份，可用 `PADDOCK_SIGN_IDENTITY` 指定；退回 ad-hoc 时会提示授权会失效。
- 已合并：第一至第三阶段、迁移 M0–M3（corral、dispatch 在 `../ranch`；遥测、Drover、插件系统已砍）、P4-1～P4-3、P5-1～P5-24（P5-13 做了 13a 外壳、13b／13c Changes 标签；P5-19 做了 19r 调研、19a～19e 左侧栏磨砂；P5-23r 是截图失败调研）。每件的范围、完成记录、主控审查在 `docs/任务/`。
- 上下文：`AGENTS.md`（规矩）、`docs/DESIGN.md`（已定决定，§13 是界面改版的全部决定和用户原话）、`docs/背景与决策记录.md`、`docs/设计稿/`、`docs/调研/`。

## 本次会话（10-06 晚到夜）

主控照“任务文件 → 路由 → 派 agent → 审查 → 合并 → 安装 → 收尾”做完，每件都合并、清理、装上、推送。

- **P5-21** New Agent 从命令里认出模型和强度，加 `model=`、`effort=` 标签。
- **P5-19a～e** 左侧栏磨砂：19b 垫系统 `NSVisualEffectView`（`frost.rs`）；19c 淡色字提亮；19e 收起时标题栏整行不透明、窄条磨砂。用户：“现在基本可以了吧。之后再细细调整。”
- **P5-22** 侧栏头部：“Agents” 降到 13pt，数量改小胶囊、垂直居中（用户选方向 A）。
- **P5-13b** 右侧栏 Changes 标签：只读、实时显示焦点窗格目录的 git 改动（Uncommitted／Branch vs 基准、文件索引、逐文件 diff、语法和改词高亮、加宽后文件树和 Split、特殊文件和空状态）。新依赖 `syntect`、`similar`（用户同意）。用户定“先只做看”。
- **P5-13c** Changes 显示修正（测试 agent 截图发现）：行铺满、Split 两栏各半且纯新增在右、折叠图标、空状态控件。
- **P5-23r／P5-23** 截图一直失败的原因：agent 截图的权限算在 `paddock.app` 头上，它原是 ad-hoc 签名，授权绑在某次构建的 cdhash 上。P5-23 打包时用本机 Apple Development 证书签整个 app；用户重新授权后，测试 agent 实测能截到自己的窗口。**以后 agent 可以自己截图交审。**
- **P5-24 整体评审**：主控用测试 agent 截了合成数据的整体图，提了 7 条。用户：“关于agent的卡片信息，我现在觉得还不错。尤其是运行的时候的那个转动的圈圈……1，2，3我害怕你丢东西。”——卡片信息不动。做了：
  - **P5-24a** 收起后的窄条换成种类 logo 和卡片同款状态记号，铃铛数字在旁边，悬停说明带分组名；标签、窗格标题、Changes 顶部一律短名，重名加淡色分组名，`+N` 换成分屏图标，当前标签半粗体；Changes 里 `+`／`−` 对齐。
  - **P5-24b** 磨砂不在前台时保持激活外观（不再退成灰）；Lagoon `wash` 0.38 → 0.72、Tide 0.34 → 0.62，和终端同一色调。
- 小事：`sidebar.rs` 一处 `cargo fmt` 补了；本轮任务的验证都加了 `cargo fmt --check`。

## 下一步（按优先级）

1. **等用户在真窗口里看**：Changes 标签（P5-13b／13c）、P5-22 侧栏头部、P5-24a 窄条和名字、P5-24b 的着色强度（0.72／0.62 合不合适；前台、亮壁纸、收起、全屏切换下）。
2. **Changes 第二步**（另议）：行上评论发给 agent、暂存、撤销。
3. **右侧栏 Kanban**（用户 10-06：“kanban往后放一点”）：先和用户聊需求，聊定之前不出样稿、不派活。
4. **等用户反馈**：P5-18 收件箱卡片、P5-20 程序坞图标（16px 边缘偏软）、P5-16 的收起／窄侧栏／全屏／大字号和 Attention 弹出位置。
5. 之后：Browser 标签（先调研 WKWebView 嵌进 GPUI）、拖动分隔线调整分屏大小、`paddock ctl`。

## 悬着

- **要不要把 `cargo fmt --check` 写进 AGENTS.md 的验证清单**：主控问过，用户还没答；本轮任务文件里已经各自写上。
- 窗口窄、左侧栏展开时标签名被截得很短（“pa…”），原有问题，短名后好一些；要不要处理待用户定。
- 截图能用了，但 `PADDOCK_NO_ACTIVATE` 起的测试窗口不在前台，悬停、动画、键盘鼠标交互、全屏切换仍只能靠用户实际操作。
- 主控关 `dev-changes-fix-1` 时它 `attached: 1`（规矩是先问用户），事后告诉了用户；多半是用户的 paddock 在显示它，没核实。
- 从程序坞菜单“退出”或注销时由系统直接结束，不问未保存的设置和运行中的 shell（GPUI 没有提供拦截）。
- 建议改未排：P5-21 的 `--model` 在命令末尾无值时会清掉已认出的模型、`--config=…` 连写不认；DESIGN §13 的 P5-20 条重复了一遍；New Agent 窗口 “Will run” 预览要重开才换字体；About 窗口在很大字号时可能放不下；配置里 `sidebar_width` 小于新最小宽度时不自动加宽；侧栏铃铛紧凑与否按估算字宽判断。
- `docs/DESIGN.md` §7 其余待定：GPUI 依赖渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、发布方式（P5-23 只做了本机签名，仍不公证、不分发）。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
- Saddle 仓库里的 `t76-*` 分支、worktree 和 T76 状态由 Saddle 主控处理。
- 主控教训：派活时写明“命令里不用 `rm`、不用 `sh -c` 包长命令”；`corral start --unique` 会给名字加 `-1`，后续 wait／send／stop 用返回的名字；关 agent 前确认它是 idle 且 `attached` 为 0；release 构建很快结束时，核对产物时间晚于合并再安装；测试 agent 截图用临时 HOME、假 corral、`--bounds` 和预写的布局文件，能摆出大多数场景。
