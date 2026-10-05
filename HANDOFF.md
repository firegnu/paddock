# 交接

## 现在在哪（2026-10-05）

- 仓库刚建立，只有本地 `main`，没有远程仓库。
- `prototypes/gpui-terminal/`：GPUI 单窗格终端原型，从 Saddle 仓库搬来，依赖改为按提交号引用 Saddle `df1c727`。搬迁后在本仓库编译目录里重新跑过：31 项测试通过、clippy 无警告、release 构建正常，一次样例渲染与搬迁前逐字节一致。
- 上下文都在本仓库：`AGENTS.md`（规矩与 Saddle 边界）、`docs/DESIGN.md`（已定决定、边界、待定问题）、`docs/背景与决策记录.md`（用户原话）、`docs/调研/` 与 `docs/任务/` 里的 Saddle 原文副本。
- 还没有 paddock 主控（`paddock/main`）。

## 下一步

1. 用户实际体验原型：构建和运行步骤见 `prototypes/gpui-terminal/README.md` 的“建议的体验步骤”，重点是键盘直接输入、中文输入法组字、滚动复制、窗口缩放，与 Zed 内置终端并排对比。
2. 根据体验结果，由用户决定 `docs/DESIGN.md` §7 的待定问题，首先是“是否继续”和“迁移路线”。
3. 用户决定继续后，主控再拆第一批任务；涉及 Saddle 的需求写成清单交给用户/Saddle 主控，不在本仓库改 Saddle。

## 悬着

- 原型里的键盘、输入法、鼠标、滚动、窗口缩放都没有经过实际操作（为避免模拟按键打进用户的其他窗口）。
- Saddle 仓库里还留着 `t76-gpui-prototype`（提交 2c59e17）和 `t76-gpui-research` 两个分支及 worktree。按约定等用户确认 paddock 可用后，由 Saddle 主控决定清理或保留，不合并进 Saddle main。
- Saddle Tasks 里 T76 的状态由 Saddle 主控处理（此前 T76 派发回执为 rejected、状态 Pending）。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
