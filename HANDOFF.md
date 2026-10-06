# 交接

## 现在在哪（2026-10-06 晚）

- main 和 `origin/main`（`github.com/firegnu/paddock`，public）同步。**进行中：P5-23r 截图失败调研**——`paddock/research-shot-1`（Codex，常规）在 worktree `../paddock-worktrees/p5-23r-shot` 里只读调查 agent 为什么截不到 paddock 窗口（用户已把 paddock 加进屏幕录制允许列表仍失败），任务文件 `docs/任务/P5-23r-截图失败调研.md`；主控挂了 `--after` 提醒。
- 225 项测试通过。`~/Applications/paddock.app` 是合并 P5-13b 之后的版本（含 P5-22）。重新打包安装：在 `app/` 下 `cargo build --release`，再 `cargo run --release --example bundle -- --install`（编译目录见 AGENTS.md，主仓库用 `.target/main`）。
- 已合并：第一至第三阶段、迁移 M0–M3（corral、dispatch 在 `../ranch`；遥测、Drover、插件系统已砍）、P4-1～P4-3、P5-1～P5-22（P5-13 做了 13a 右侧栏外壳和 13b Changes 标签；P5-19 做了 19r 调研、19a～19e 左侧栏磨砂）。每件的范围、完成记录、主控审查在 `docs/任务/`。
- 上下文：`AGENTS.md`（规矩）、`docs/DESIGN.md`（已定决定，§13 是界面改版的全部决定和用户原话）、`docs/背景与决策记录.md`、`docs/设计稿/`、`docs/调研/`。

## 本次会话（10-06 晚）

主控照“任务文件 → 路由 → 派 agent → 审查 → 合并 → 安装 → 收尾”做完六件，每件都合并、清理、装上、推送。

- **P5-21** New Agent 从命令里认出模型和强度（claude 的 `--model`／`--effort`，codex 的 `-m`／`--model`、`-c model_reasoning_effort=`），开 agent 时加 `model=`、`effort=` 标签，卡片 MODEL 格因此有显示。起因：用户问“Model现在显示 -”——MODEL 只读这两个标签，手动开的和 New Agent 开的都没有；派活技能开的一直有。命令没写的仍是 “—”。
- **P5-19a** GPUI `Blurred`＋整窗半透明：在 macOS 27.0.1 上**没有模糊**（用户：桌面“基本清晰”，“这是玻璃效果，而不是毛玻璃”），还透到右边。被 19b 取代。
- **P5-19b** 左边一列垫系统 `NSVisualEffectView`（`app/src/frost.rs`，sidebar 材质、behindWindow，`objc2-app-kit =0.3.2`），右边恢复不透明，全屏退回不透明。用户：“左侧栏不收的效果不错”。
- **P5-19c** 磨砂上的淡色字（`agents_dim`／`agents_dimmer`）往正文色提亮，三套主题各定比例（用户：“有些字看不清楚 STAUS之类的”）。
- **P5-19d** 收起时倒 L 形磨砂——用户：“L不好看”。被 19e 取代。
- **P5-19e** 收起时标题栏整行不透明，窄条从标题栏下磨砂到底；展开时整列从上到下磨砂。用户：“还可以吧”，“现在基本可以了吧。之后再细细调整。”
- 小事：P5-21 测试的 `cargo fmt` 格式补了一个提交。

## 10-06 晚续

- **P5-22** 侧栏头部：“Agents” 降到 13pt，数量改小胶囊、垂直居中（用户：“Agents太突兀了……数字也没有和agents对齐”，选方向 A）。已合并、安装、推送；截图没截到，观感待用户看。
- **P5-13b** Changes 标签：只读、实时显示焦点窗格目录的 git 改动（Uncommitted／Branch vs 基准，文件索引、逐文件 diff、语法高亮和改词高亮、加宽后文件树和 Split、特殊文件和空状态）。新依赖 `syntect`、`similar`。已合并、安装、推送；截图没截到，界面和交互全部待用户看。取舍见任务文件的完成记录和主控审查。

## 下一步（按优先级）

1. **P5-23r 截图调研出结果后和用户定怎么办**；**等用户看 P5-13b 和 P5-22**。之后是 Changes 第二步（行上评论发给 agent、暂存、撤销，另议）。
2. **右侧栏 Kanban**（用户 10-06：“kanban往后放一点”）：先和用户聊需求，聊定之前不出样稿、不派活（右侧栏定为 Changes、Browser、Kanban 三个标签）。
3. **等用户反馈**：P5-22 侧栏头部观感；P5-18 收件箱卡片（角标动画、等你卡片和 Reply、空闲预览、展开布局、空心“在此打开”环是否太小）、P5-20 程序坞图标（16px 边缘偏软）、P5-16 的收起／窄侧栏／全屏／大字号和 Attention 弹出位置。
4. **磨砂细调（用户说“之后再细细调整”，等用户提）**：三套主题的 wash／lit／waiting 和提亮比例（`preset.rs` `Preset::frost`）、亮壁纸上的可读性、拖宽／收起／全屏时边缘闪不闪；收起时窄条右边那道不透明的缝要不要跟着磨砂或收窄（主控提过，用户没答）；浮动圆角面板和边缘高光没做。
5. 之后：Browser 标签（先调研 WKWebView 嵌进 GPUI）、拖动分隔线调整分屏大小、`paddock ctl`。

## 悬着

- 磨砂的实际效果 agent 一直截不到（`screencapture` 报 could not create image from window，可能缺屏幕录制权限），全靠用户截图。全屏切换、收起展开时的边缘只测了计算函数，没有真实切换的自动化测试。
- 键盘、点击、拖动缩放、系统提示框、菜单与快捷键，以及真实 corral 的新建／停止，大多还没经用户实际操作；palette 三种模式、从访达拖图片到 agent 窗格也待试。
- 从程序坞菜单“退出”或注销时由系统直接结束，不问未保存的设置和运行中的 shell（GPUI 没有提供拦截）。
- 建议改未排：P5-21 的 `--model` 在命令末尾无值时会清掉已认出的模型、`--config=…` 连写不认；状态角标和窄条小格圆点的“圈”在磨砂上只能近似；DESIGN §13 的 P5-20 条重复了一遍；New Agent 窗口 “Will run” 预览要重开才换字体；About 窗口在很大字号时可能放不下；配置里 `sidebar_width` 小于新最小宽度时不自动加宽；侧栏铃铛紧凑与否按估算字宽判断。
- `docs/DESIGN.md` §7 其余待定：GPUI 依赖渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、发布方式。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
- Saddle 仓库里的 `t76-*` 分支、worktree 和 T76 状态由 Saddle 主控处理。
- 主控教训：派活时写明“命令里不用 `rm`、不用 `sh -c` 包长命令”；`corral start --unique` 会给名字加 `-1`，后续 wait／send／stop 用返回的名字；corral 的 `--after` 提醒常比后台 wait 晚到，处理过的忽略即可；关 agent 前确认它是 idle。
