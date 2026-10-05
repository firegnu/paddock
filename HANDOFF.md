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

1. 用户体验第三阶段这一批，反馈问题（截图已给用户看过）。
2. **M2 已完成**（`docs/任务/M2-ranch与corral.md`）：corral 在 ranch，`~/.local/bin/corral` 指向 ranch，Saddle 已剥离并改用它，现有会话已升级。
3. 用户定：dispatch 不带遥测剥离到 ranch，之后转回 paddock 开发；遥测、Drover、整个插件系统砍掉（Saddle 删代码和对应测试，数据留在磁盘上）。正在做 M3（`docs/任务/M3-dispatch进ranch.md`）：ranch 部分已完成。用户又定砍掉 Saddle 的遥测、Drover、整个插件系统（paddock 路线同样拿掉），补充已交 saddle/main；等它删完并部署，再在用户在场时链接 `~/.local/bin/ranch`、装技能、核对。
4. M3 做完后转回 paddock 开发：`paddock ctl` 等。遥测查看页、插件界面、Drover 已取消（用户 10-05）。

## 悬着

- 键盘、点击、拖动缩放、系统提示框、菜单与快捷键，以及真实 corral 的新建/停止，都还没经用户实际操作。
- 从程序坞菜单“退出”或注销时由系统直接结束，不问未保存的设置和运行中的 shell（GPUI 没有提供拦截）。
- 拖动分隔线调整分屏大小没做。
- `docs/DESIGN.md` §7 其余待定：GPUI 依赖渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、发布方式；已定：不加许可证、Clawd 随公开仓库发布。
- 原型发现的缺口（DESIGN §5）由 paddock 在自己的代码里解决，按需排期。
- Saddle 仓库里还留着 `t76-gpui-prototype`、`t76-gpui-research` 两个分支及 worktree；Saddle Tasks 里 T76 的状态。都由 Saddle 主控处理。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
