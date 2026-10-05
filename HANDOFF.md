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
- 用户 10-05 定下迁移的整体安排（DESIGN §2、§3“步骤”；原话见 `docs/背景与决策记录.md` §6g–§6j）：
  - 全局只能一份的运行时（corral、遥测、dispatch、插件协议）独立成新仓库 ranch（`../ranch`，GitHub 公开、不加许可证），paddock 主控兼管；Saddle 和 paddock 都只是前端，只调用 ranch 装好的命令。
  - 要有插件系统；插件界面走“乙”（插件描述界面，paddock 用 GPUI 原生画）；paddock 不依赖 ratatui（含插件 SDK）。
  - Diff 插件不搬；`saddle ctl` 迁成 `paddock ctl`。
  - dispatch 迁完就转到 paddock 开发；Saddle 不放弃，定位为保底版（不加新功能，只保证和运行时对得上）。Drover 全部切换后才迁，在那之前用户口头布置任务、主控拆分委派。
- 上下文：`AGENTS.md`（规矩）、`docs/DESIGN.md`（已定决定、步骤、待定问题）、`docs/背景与决策记录.md`（用户原话）、`docs/任务/`（每件活的任务文件和完成记录）。

## 下一步

- P4-1（⌘, 打不开设置窗口，About、⌘⇧N 同因）已修好并重新安装 paddock.app；这件按用户要求派给 Codex（paddock/dev-open-windows）做，用来试派发和路由，流程走通。用户重启后确认 ⌘, 能打开设置窗口（10-05）；About、⌘⇧N 未单独确认。
- P4-2（界面字体与字号、两种字体都从已装字体里搜索挑选、终端字体 Save 后即时生效）和 P4-3（侧栏底部图标按钮）并行派给两个 Claude Code，主控集成（底部图标跟随界面字号）后合并，已重新安装 paddock.app。用户 10-06 试用后：“我试了一下还挺好的。”建议改未排：New Agent 窗口 “Will run” 预览要重开才换字体；About 窗口在很大字号时可能放不下。
1. 用户体验第三阶段这一批，反馈问题（截图已给用户看过）。
2. **M2 已完成**（`docs/任务/M2-ranch与corral.md`）：corral 在 ranch，`~/.local/bin/corral` 指向 ranch，Saddle 已剥离并改用它，现有会话已升级。
3. 用户定：dispatch 不带遥测剥离到 ranch，之后转回 paddock 开发；遥测、Drover、整个插件系统砍掉（Saddle 删代码和对应测试，数据留在磁盘上）。**M3 已完成**（`docs/任务/M3-dispatch进ranch.md`）：dispatch 在 ranch（`ranch dispatch route`、技能由 `ranch dispatch install-skills` 装）；Saddle 已砍掉插件系统和遥测（被删代码可从 Saddle 标签 `before-cut` 取回，数据留在磁盘上）。两个前端同一起跑线，接下来转回 paddock 开发。
4. M3 做完后转回 paddock 开发：`paddock ctl` 等。遥测查看页、插件界面、Drover 已取消（用户 10-05）。

- 下一件：P5-3 Command palette（DESIGN §13），P5-1、P5-2 收尾后先出样稿。
- 再下一件：P5-4 弹出界面重新设计（DESIGN §13 有盘点清单），P5-3 之后，同样先出样稿；系统对话框是否保留原生待用户定。

- P5-1 侧栏改版、P5-2 窗口与窗格改版并行派给两个 Claude Code，主控集成后合并，已重新安装 paddock.app（10-06）。待用户试用：拖动、双击、全屏、悬停、点击；用户定：宠物回原大小、标题栏宠物开着时 48pt（P5-5 已做）；标签 × 淡入和分屏菜单并进 P5-4；P5-1 的建议改里只补“上次输入”的时间（P5-5 已做），其余因数据拿不到或 GPUI 不支持不做。

## 悬着

- 键盘、点击、拖动缩放、系统提示框、菜单与快捷键，以及真实 corral 的新建/停止，都还没经用户实际操作。
- 从程序坞菜单“退出”或注销时由系统直接结束，不问未保存的设置和运行中的 shell（GPUI 没有提供拦截）。
- 拖动分隔线调整分屏大小没做。
- `docs/DESIGN.md` §7 其余待定：GPUI 依赖渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、发布方式；已定：不加许可证、Clawd 随公开仓库发布。
- 原型发现的缺口（DESIGN §5）由 paddock 在自己的代码里解决，按需排期。
- Saddle 仓库里还留着 `t76-gpui-prototype`、`t76-gpui-research` 两个分支及 worktree；Saddle Tasks 里 T76 的状态。都由 Saddle 主控处理。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
