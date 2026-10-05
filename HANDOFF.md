# 交接

## 现在在哪（2026-10-05）

- 仓库只有本地 `main`，没有远程仓库。
- `app/`：GPUI 单窗格终端原型，按提交号引用 Saddle `df1c727`。搬迁后 31 项测试通过、clippy 无警告。
- 主控 `paddock/main` 已在运行。
- 用户 10-05 体验原型（Claude Code 里中文输入、显示），判定通过。
- 用户要求主题系统，像 Saddle 那样；细节已定，见 `docs/DESIGN.md` §8。
- 上下文：`AGENTS.md`（规矩与 Saddle 边界）、`docs/DESIGN.md`（已定决定、边界、主题、待定问题）、`docs/背景与决策记录.md`（用户原话）。

## 下一步

1. 第一阶段范围已定（DESIGN §9）。
2. 第一批任务文件已写好（`docs/任务/P1-T1-应用骨架.md`、`P1-T2-主题系统.md`、`P1-T3-Agents侧栏.md`），等用户过目后派 T1；T1 合并后并行派 T2、T3。涉及 Saddle 的需求写成清单交给用户/Saddle 主控，不在本仓库改 Saddle。

## 悬着

- 原型中滚动回看、⌘C/⌘V、鼠标选择、拖动缩放窗口仍未经用户实际操作。
- 从 agent 会话里启动原型会把 Claude Code 的会话变量带进窗格（DESIGN §5 缺口 4）；正式版的清理规则待定。
- `docs/DESIGN.md` §7 其余待定：GPUI 渠道、pre-1.0 是否接受、迁移路线、首期是否只做 macOS、Saddle 新增接口、Drover 并存规则、许可与发布。
- Saddle 仓库里还留着 `t76-gpui-prototype`（提交 2c59e17）和 `t76-gpui-research` 两个分支及 worktree，由 Saddle 主控决定清理或保留，不合并进 Saddle main。
- Saddle Tasks 里 T76 的状态由 Saddle 主控处理。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
