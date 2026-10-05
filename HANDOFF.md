# 交接

## 现在在哪（2026-10-05）

- `app/`：paddock 应用。已合并：
  - 第一阶段：T1 骨架、T2 主题、T3 Agents 侧栏、T4 字体。
  - 迁移：M0 不再依赖 Saddle 的库，用到的代码迁入 `app/src/`（来源 Saddle `df1c727`）；M1 不再用 ratatui、crossterm。
  - 第二阶段：P2-1 至 P2-4。
  - 第三阶段这一批：P3-1 至 P3-10（设置与 About 独立窗口、关闭 shell 确认、新建/停止 agent、Attention、Go to Agent、终端查找、窗格放大、布局保存与恢复、Diagnostics）。
  - 147 项测试通过。
- `~/Applications/paddock.app` 是第三阶段这一批的版本。重新打包安装：在 `app/` 下 `cargo build --release`，再 `cargo run --release --example bundle -- --install`（编译目录见 AGENTS.md）。
- 远程仓库：`origin` = `github.com/firegnu/paddock`（public，10-05 建）。合并后推送；推送前查隐私（gitleaks、trufflehog）。
- 主控 `paddock/main` 自己实现，不再委派（AGENTS.md「开发方式」）。
- 用户 10-05 定下迁移的整体安排（DESIGN §2、§3“步骤”；原话见 `docs/背景与决策记录.md` §6g、§6h）：
  - 全局只能一份的运行时（corral、遥测、Drover）移到 paddock，不再两边各留一份。
  - 要有插件系统；插件界面走“乙”（插件描述界面，paddock 用 GPUI 原生画）；paddock 不依赖 ratatui（含插件 SDK）。
  - Diff 插件不搬；`saddle ctl` 迁成 `paddock ctl`。
  - dispatch 迁完就全部转到 paddock 开发，Saddle 退出；Drover 全部切换后才迁，在那之前用户口头布置任务、主控拆分委派。
- 上下文：`AGENTS.md`（规矩）、`docs/DESIGN.md`（已定决定、步骤、待定问题）、`docs/背景与决策记录.md`（用户原话）、`docs/任务/`（每件活的任务文件和完成记录）。

## 下一步

1. 用户体验第三阶段这一批，反馈问题（截图已给用户看过）。
2. 按 DESIGN §3“步骤”，下一件是第 2 步 **corral**。动手前：
   - 写任务文件给用户看；
   - 由用户与 Saddle 主控约定冻结点（corral 仍在 Saddle 里活跃开发）。可先写一份给 Saddle 主控的需求说明（冻结点、Saddle 退出后安装上的整理），用户同意后再交；
   - 切换 `~/.local/bin/corral` 时用户在场（DESIGN §7 第 9 条）。
3. 之后的次序：第 3 步遥测的存储与命令（`paddock telemetry`、`paddock agent`）→ 第 4 步插件宿主底层与 dispatch → 切换、Saddle 退出 → `paddock ctl` → 遥测查看页 → 插件界面（先给用户看界面协议设计）→ Drover（核心放法 A/B/C 到时定）。

## 悬着

- 键盘、点击、拖动缩放、系统提示框、菜单与快捷键，以及真实 corral 的新建/停止，都还没经用户实际操作。
- 从程序坞菜单“退出”或注销时由系统直接结束，不问未保存的设置和运行中的 shell（GPUI 没有提供拦截）。
- 拖动分隔线调整分屏大小没做。
- `docs/DESIGN.md` §7 其余待定：GPUI 依赖渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、许可（还没有许可证文件）与发布；Clawd 已定随公开仓库发布。
- 原型发现的缺口（DESIGN §5）由 paddock 在自己的代码里解决，按需排期。
- Saddle 仓库里还留着 `t76-gpui-prototype`、`t76-gpui-research` 两个分支及 worktree；Saddle Tasks 里 T76 的状态。都由 Saddle 主控处理。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
