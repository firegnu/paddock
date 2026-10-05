# 交接

## 现在在哪（2026-10-05）

- 仓库只有本地 `main`，没有远程仓库。
- `app/`：paddock 应用，第一阶段三件已合并：T1 骨架（窗口布局、配置文件、身份变量清理）、T2 主题（Dune/Tide/Lagoon、终端调色板、`[colors]` 覆盖）、T3 Agents 侧栏（按组列出、点击接入、新 shell）。字体可在配置文件设置（T4）。M0 起不再依赖 Saddle 的库，用到的代码已迁入 `app/src/`（来源 Saddle `df1c727`）；M1 起不再用 ratatui、crossterm。第二阶段、第三阶段这一批（P3-1 至 P3-10）已合并。147 项测试通过。
- 主控 `paddock/main` 已在运行；用户 10-05 决定不再委派，主控自己实现（AGENTS.md「开发方式」）。
- 用户 10-05 体验原型（Claude Code 里中文输入、显示），判定通过。
- 用户要求主题系统，像 Saddle 那样；细节已定，见 `docs/DESIGN.md` §8。
- 上下文：`AGENTS.md`（规矩与 Saddle 的关系）、`docs/DESIGN.md`（已定决定、边界、主题、待定问题）、`docs/背景与决策记录.md`（用户原话）。

## 下一步

1. 第一阶段（T1–T4）、迁移（M0、M1）、第二阶段（P2-1 Agents 面板、P2-2 标签页与分屏、P2-3 宠物、P2-4 菜单栏与 .app）都已合并；`paddock.app` 已装到 `~/Applications`（重新打包安装：`cargo build --release` 后 `cargo run --release --example bundle -- --install`，见 `app/README.md`）。
2. 第三阶段这一批（DESIGN §12）已全部合并并重新安装（10-05）：独立的设置、About、New Agent 窗口，关闭 shell 确认，新建/停止 agent，Attention，Go to Agent（⌘P），终端查找（⌘F），窗格放大（⇧⌘↩），布局保存与恢复，Diagnostics。截图已给用户；等用户体验和反馈。
3. 之后按 DESIGN §3 的四步：第 2 步搬 corral。动手前要和用户确认，并经用户与 Saddle 主控约定冻结点（corral 仍在 Saddle 里活跃开发）；切换 `~/.local/bin/corral` 时用户在场。

## 悬着

- 滚动回看、鼠标选择、拖动缩放窗口、标签页与分屏的点击、菜单与快捷键，仍未经用户实际操作。第三阶段这一批的键盘、点击、系统提示框、真实 corral 的新建/停止，同样未经用户实际操作。
- 从程序坞菜单“退出”或注销时由系统直接结束，不问未保存的设置和运行中的 shell（GPUI 没有提供拦截）。
- 拖动分隔线调整分屏大小没做；Clawd 单独分发时是否保留待定。
- 会话变量泄漏已在 T1 按名单清理（17 个）。
- `docs/DESIGN.md` §7 其余待定：GPUI 渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、插件界面做法、许可与发布（Drover 持有权已定由 paddock 持有，§3 第 4 步）。
- 原型发现的缺口（DESIGN §5）由 paddock 在自己的代码里解决，按需排期。
- Saddle 仓库里还留着 `t76-gpui-prototype`（提交 2c59e17）和 `t76-gpui-research` 两个分支及 worktree，由 Saddle 主控决定清理或保留，不合并进 Saddle main。
- Saddle Tasks 里 T76 的状态由 Saddle 主控处理。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
