# paddock

paddock 是用 GPUI 重做 Saddle 界面的独立桌面应用：原生窗口里查看 corral agent、运行真实交互终端，终端体验以 Zed 内置终端为参照。它和 Saddle 在代码上完全分开：用到的 Saddle 代码已迁入本仓库自己维护（来源 Saddle 提交 `df1c727`），不依赖 Saddle 的库；Saddle 本身（含 TUI）继续独立开发和使用，两边不能互相影响，尤其是所用的 Rust 库。

设计和当前决定以 `docs/DESIGN.md` 为准。实现中要改设计，先改那份文档，并在提交说明里写清楚。

## 先读

- `docs/DESIGN.md`：权威设计。§2 已定决定，§3 与 Saddle 的关系，§4 依赖隔离，§7 待定问题。
- `docs/背景与决策记录.md`：用户原话、为什么独立成仓库、哪些还没批准（不要把建议当成已批准）。
- `HANDOFF.md`：现在在哪、下一步做什么、悬着什么。
- `docs/原型实测记录.md`：原型的运行方式、复用结果、缺口和实测记录。
- `docs/调研/T76-GPUI桌面化详细方案.md`：从 Saddle 复制来的完整调研（GPUI 现状、迁移路线比较、风险）。只作依据，以本仓库 DESIGN 为准。

## 规矩

- **语言与依赖**：Rust stable；只用成熟、活跃维护的库。GPUI 本身仍是 pre-1.0，是否算“成熟”由用户裁定，见 DESIGN §7。实现、测试和辅助工具都不用 Python（沿用用户在 Saddle 的要求），工具用 Rust 或纯数据文件。不依赖 ratatui、crossterm，包括插件 SDK 和仓库里维护的插件（用户 10-05：“不能依赖ratatui”）。
- **与 Saddle 分开（最重要）**：
  - 不依赖 Saddle 的库，不加 `saddle` 的 git 或路径依赖。迁入的代码（`app/src/` 中开头注明“From Saddle”的文件）在本仓库自己维护；Saddle 的修复不会自动进来，需要时作为单独任务手动移植，注明对应的 Saddle 提交。
  - 和 Saddle 生态只通过公开约定打交道：`corral` 命令及其 JSON 输出、插件协议、遥测和 Drover 的公开命令（DESIGN §3）。不读 Saddle 的内部状态和配置文件。
  - 不修改 Saddle 仓库（包括它的 worktree、分支、Cargo 文件、文档和任务数据）。需要读 Saddle 时只读。
  - 公开约定本身需要变化时，写成需求交给用户/Saddle 主控，在 Saddle 自己的流程里做；paddock 自己代码里能解决的，不找 Saddle。
  - paddock 的依赖、锁文件、编译目录、工具链要求都不能进入 Saddle。
- **依赖固定**：`gpui-pre-*` 系列用 `=` 精确固定，所有 `gpui-pre-*` 包一起升级，升级是单独任务。其余第三方库的版本由 paddock 自己决定，升级同样作为单独任务。
- **不复制 Zed 的 GPL 代码**：Zed 的 `terminal`、`terminal_view`、`ui` 等应用层是 GPL-3.0-or-later，只能参考思路。GPUI 本身（Apache-2.0）可以读源码确认接口。外部参考先核对文件来源和许可，保留必要声明。
- **系统安装**：不自行安装系统组件或工具链（例如 Xcode 的 Metal 工具链、新的 rustup 工具链），需要时先问用户。目前用 GPUI 的 `runtime_shaders` 绕开 Metal 工具链。
- **Corral 只走公开命令**：和 Saddle 相同。agent 状态、接入都通过 `corral ls/status/attach/start/stop` 等公开命令（`app/src/corral.rs`）；不读 Corral 内部状态目录。
- **不要干扰用户正在用的 agent**：`corral ls` 里现有的 agent 都是用户的。可以用 `corral ls/status/reply` 读；不要对它们 `corral stop`、`corral send`、`corral keys`，也不要用原型 attach 上去打字。需要真实 agent 实测时，自己开一个 `paddock/test-<名字>`（例如 `corral start paddock/test-a --cwd <临时目录> --label role=test -- claude`），用完 `corral stop`。
- **不按名字批量杀进程**：不要用 `pkill -f paddock`、`pkill -f corral` 这类命令。停自己起的进程，用启动时记下的 PID。
- **桌面窗口测试**：
  - 截图只截原型自己的窗口：设置 `GPUI_TERM_WINDOW_ID=1` 拿到窗口编号，用 `screencapture -x -o -l <编号>`。不要截全屏或按屏幕区域截，会拍到用户的其他窗口。误拍的立即删掉。
  - 不要用脚本模拟系统按键或鼠标（如 `osascript` 的 System Events），可能打进用户正在用的窗口。键盘、输入法、鼠标交互留给用户实际体验。
  - 截图不入库，尤其是带账号用量等信息的画面。
  - GPUI 在窗口被完全遮挡时暂停绘制，截到旧画面不等于没更新。
- **测试不依赖真实 agent**：用合成数据和假 `corral` 脚本；真实 agent 只用于明确的实测，且只用自己开的测试实例。
- **验证**：按改动影响面选择检查。小改跑直接相关测试；跨模块改动和合并前，对每个 Cargo 清单跑 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`。保留有价值的测试，不靠删测试、放宽断言或缩短超时来通过。
- **独立编译目录**：paddock 的编译产物都放在 `$HOME/Developer/personal_projs/paddock-worktrees/.target/` 下，不和 Saddle 的 `.target` 共用；每个工作目录用自己的子目录，不和别的 worktree 共用：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/<子目录>`。主仓库用 `main`，任务 worktree 用分支名（如 `p1-font`），审查 worktree 用 `review`。原因：并行的 worktree 编的是同一个包，共用目录会互相覆盖产物，cargo 还可能把别人的产物当成最新的（P1 的 T2、T3 都遇到过）。清 worktree 时一并删掉它的子目录。

## 开发方式（主控自己做）

- 用户 10-05 决定：不再把开发任务派给别的 agent，由主控（`paddock/main`）自己实现。P1 的 T1–T4 是按旧的分派流程做的，任务文件里的委派信息只作历史记录。
- 每件活先在 `docs/任务/` 写任务文件（做成什么、范围、怎么算做完），给用户看过再动手（用户 10-05 对第三阶段那一批另有安排：不逐件等回复，一次做完后一并汇报，见 DESIGN §12）。验收照抄用户原话，不补验收点；主控觉得该加的，列出来问用户。调研和设计文档里的“建议”不是用户的验收条件。
- 每件活一个分支，worktree 放 `../paddock-worktrees/<分支>`，编译目录用 `.target/<分支>`（见上文“独立编译目录”）。
- 合并前按影响面自查：至少跑 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`，在任务文件末尾写「完成记录」：做了什么、验证了什么、拿主意的地方、没做的事。
- 合并：本地合并进 main。现在没有远程仓库；以后用户配置了 `origin`，就在合并后推送。不自行创建远程仓库。
- 收尾记号：一件活合并完、worktree、分支和它的编译子目录清干净之后，在 main 上补一条空提交（`git commit --allow-empty`），首行写「收尾: 」加一句话说明这件活。只记真正落地的活。
- 收尾之后更新 `HANDOFF.md`：现在在哪、下一步、悬着什么。设计和理由进 `docs/DESIGN.md`，别写进交接文件。
- 需要真实 agent 实测时，照“不要干扰用户正在用的 agent”一条，自己开 `paddock/test-<名字>`（`--label role=test`），用完 `corral stop`。
