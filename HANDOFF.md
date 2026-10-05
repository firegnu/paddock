# 交接

## 现在在哪（2026-10-05）

- 仓库只有本地 `main`，没有远程仓库。
- `app/`：paddock 应用，第一阶段三件已合并：T1 骨架（窗口布局、配置文件、身份变量清理）、T2 主题（Dune/Tide/Lagoon、终端调色板、`[colors]` 覆盖）、T3 Agents 侧栏（按组列出、点击接入、新 shell）。字体可在配置文件设置（T4）。M0 起不再依赖 Saddle 的库，用到的代码已迁入 `app/src/`（来源 Saddle `df1c727`）；M1 起不再用 ratatui、crossterm。95 项测试通过。
- 主控 `paddock/main` 已在运行；用户 10-05 决定不再委派，主控自己实现（AGENTS.md「开发方式」）。
- 用户 10-05 体验原型（Claude Code 里中文输入、显示），判定通过。
- 用户要求主题系统，像 Saddle 那样；细节已定，见 `docs/DESIGN.md` §8。
- 上下文：`AGENTS.md`（规矩与 Saddle 的关系）、`docs/DESIGN.md`（已定决定、边界、主题、待定问题）、`docs/背景与决策记录.md`（用户原话）。

## 下一步

1. 第一阶段（T1–T4）已全部合并，用户体验认可。编译目录已改为每个工作目录一个子目录。
2. 方向已改为与 Saddle 代码完全分开、界面全部用 GPUI 重做（DESIGN §1–§3、§10）。迁移 M0（迁入 Saddle 代码、去掉 `saddle` 依赖）、M1（去掉 `ratatui`、`crossterm`）已合并。将来单独分发前还要迁入 corral 运行时（DESIGN §3）。
3. 第二阶段（DESIGN §11）：P2-1 Agents 面板、P2-2 标签页与分屏已合并（用户认可）；下一件 P2-3 宠物，然后 P2-4 菜单栏与 .app。

## 悬着

- 原型中滚动回看、⌘C/⌘V、鼠标选择、拖动缩放窗口仍未经用户实际操作。
- 会话变量泄漏已在 T1 按名单清理（17 个）。
- `docs/DESIGN.md` §7 其余待定：GPUI 渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、插件界面做法、Drover 并存规则、许可与发布。
- 原型发现的缺口（DESIGN §5）由 paddock 在自己的代码里解决，按需排期。
- Saddle 仓库里还留着 `t76-gpui-prototype`（提交 2c59e17）和 `t76-gpui-research` 两个分支及 worktree，由 Saddle 主控决定清理或保留，不合并进 Saddle main。
- Saddle Tasks 里 T76 的状态由 Saddle 主控处理。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
