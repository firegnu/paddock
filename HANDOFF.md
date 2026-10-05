# 交接

## 现在在哪（2026-10-05）

- 仓库只有本地 `main`，没有远程仓库。
- `app/`：paddock 应用，第一阶段三件已合并：T1 骨架（窗口布局、配置文件、身份变量清理）、T2 主题（Dune/Tide/Lagoon、终端调色板、`[colors]` 覆盖）、T3 Agents 侧栏（按组列出、点击接入、新 shell）。按提交号引用 Saddle `df1c727`；53 项测试通过；字体可在配置文件设置（T4）。
- 主控 `paddock/main` 已在运行。
- 用户 10-05 体验原型（Claude Code 里中文输入、显示），判定通过。
- 用户要求主题系统，像 Saddle 那样；细节已定，见 `docs/DESIGN.md` §8。
- 上下文：`AGENTS.md`（规矩与 Saddle 边界）、`docs/DESIGN.md`（已定决定、边界、主题、待定问题）、`docs/背景与决策记录.md`（用户原话）。

## 下一步

1. 第一阶段范围已定（DESIGN §9）。
2. 第一阶段 T1–T3 已全部合并收尾；用户 10-05 体验后认可（“可以，我看了”）。
3. T4（字体进配置文件、对比度测试）已合并收尾。编译目录已改为每个工作目录一个子目录（`.target/main`、`.target/<分支>`、`.target/review`，见 AGENTS.md）。主题错误信息带配置文件路径，用户暂不做。

## 悬着

- 原型中滚动回看、⌘C/⌘V、鼠标选择、拖动缩放窗口仍未经用户实际操作。
- 会话变量泄漏已在 T1 按名单清理（17 个）；Saddle 侧“启动时可控制环境变量”的接口仍只是候选需求（DESIGN §5 缺口 4）。
- `docs/DESIGN.md` §7 其余待定：GPUI 渠道、pre-1.0 是否接受、迁移路线、首期是否只做 macOS、Saddle 新增接口、Drover 并存规则、许可与发布。
- Saddle 侧候选需求新增一条：终端解析应答程序的颜色查询时仍用 xterm 默认值，不是 paddock 主题的调色板。
- Saddle 仓库里还留着 `t76-gpui-prototype`（提交 2c59e17）和 `t76-gpui-research` 两个分支及 worktree，由 Saddle 主控决定清理或保留，不合并进 Saddle main。
- Saddle Tasks 里 T76 的状态由 Saddle 主控处理。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
