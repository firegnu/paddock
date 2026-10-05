# paddock

paddock 是 Saddle 的 GPUI 桌面前端，目前处在原型阶段：一个 GPUI 窗口里的真实交互终端，终端体验以 Zed 内置终端为参照。它从 Saddle 仓库独立出来，按固定提交号引用 Saddle 的库；Saddle 本身（含 TUI）继续独立开发和使用，两边不能互相影响，尤其是所用的 Rust 库。

设计和当前决定以 `docs/DESIGN.md` 为准。实现中要改设计，先改那份文档，并在提交说明里写清楚。

## 先读

- `docs/DESIGN.md`：权威设计。§2 已定决定，§3 与 Saddle 的边界，§4 依赖隔离，§7 待定问题。
- `docs/背景与决策记录.md`：用户原话、为什么独立成仓库、哪些还没批准（不要把建议当成已批准）。
- `HANDOFF.md`：现在在哪、下一步做什么、悬着什么。
- `docs/原型实测记录.md`：原型的运行方式、复用结果、缺口和实测记录。
- `docs/调研/T76-GPUI桌面化详细方案.md`：从 Saddle 复制来的完整调研（GPUI 现状、迁移路线比较、风险）。只作依据，以本仓库 DESIGN 为准。

## 规矩

- **语言与依赖**：Rust stable；只用成熟、活跃维护的库。GPUI 本身仍是 pre-1.0，是否算“成熟”由用户裁定，见 DESIGN §7。实现、测试和辅助工具都不用 Python（沿用用户在 Saddle 的要求），工具用 Rust 或纯数据文件。
- **与 Saddle 隔离（最重要）**：
  - 只通过 git 依赖按固定提交号引用 Saddle（`saddle = { git = "https://github.com/firegnu/saddle.git", rev = "…" }`）。提交的代码里不用路径依赖。
  - 不修改 Saddle 仓库（包括它的 worktree、分支、Cargo 文件、文档和任务数据）。需要读 Saddle 时只读。
  - 升级引用的 Saddle 提交是一件单独的任务：看清两个提交之间 Saddle 公开接口和依赖的变化，更新锁文件，跑全量检查。
  - 需要 Saddle 提供新能力（例如“有新输出”的通知、带环境变量的启动接口）时，写成需求交给用户/Saddle 主控，在 Saddle 自己的流程里做；优先新增接口、不改旧接口，保持 Saddle TUI 行为不变。paddock 这边不自己改。
  - paddock 的依赖、锁文件、编译目录、工具链要求都不能进入 Saddle。
- **依赖固定**：`gpui-pre-*` 系列用 `=` 精确固定，所有 `gpui-pre-*` 包一起升级，升级是单独任务。与 Saddle 公开类型交换的库（如 `alacritty_terminal`）必须和所引用的 Saddle 提交用同一版本。
- **不复制 Zed 的 GPL 代码**：Zed 的 `terminal`、`terminal_view`、`ui` 等应用层是 GPL-3.0-or-later，只能参考思路。GPUI 本身（Apache-2.0）可以读源码确认接口。外部参考先核对文件来源和许可，保留必要声明。
- **系统安装**：不自行安装系统组件或工具链（例如 Xcode 的 Metal 工具链、新的 rustup 工具链），需要时先问用户。目前用 GPUI 的 `runtime_shaders` 绕开 Metal 工具链。
- **Corral 只走公开命令**：和 Saddle 相同。agent 状态、接入都通过 `corral ls/status/attach/start/stop` 等公开命令或 Saddle 库里对它们的封装；不读 Corral 内部状态目录。
- **不要干扰用户正在用的 agent**：`corral ls` 里现有的 agent 都是用户的。可以用 `corral ls/status/reply` 读；不要对它们 `corral stop`、`corral send`、`corral keys`，也不要用原型 attach 上去打字。需要真实 agent 实测时，自己开一个 `paddock/test-<名字>`（例如 `corral start paddock/test-a --cwd <临时目录> --label role=test -- claude`），用完 `corral stop`。
- **不按名字批量杀进程**：不要用 `pkill -f paddock`、`pkill -f corral` 这类命令。停自己起的进程，用启动时记下的 PID。
- **桌面窗口测试**：
  - 截图只截原型自己的窗口：设置 `GPUI_TERM_WINDOW_ID=1` 拿到窗口编号，用 `screencapture -x -o -l <编号>`。不要截全屏或按屏幕区域截，会拍到用户的其他窗口。误拍的立即删掉。
  - 不要用脚本模拟系统按键或鼠标（如 `osascript` 的 System Events），可能打进用户正在用的窗口。键盘、输入法、鼠标交互留给用户实际体验。
  - 截图不入库，尤其是带账号用量等信息的画面。
  - GPUI 在窗口被完全遮挡时暂停绘制，截到旧画面不等于没更新。
- **测试不依赖真实 agent**：用合成数据和假 `corral` 脚本；真实 agent 只用于明确的实测，且只用自己开的测试实例。
- **验证**：按改动影响面选择检查。小改跑直接相关测试；跨模块改动和合并前，对每个 Cargo 清单跑 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`。保留有价值的测试，不靠删测试、放宽断言或缩短超时来通过。
- **独立编译目录**：所有 worktree 共用 paddock 自己的编译目录，不和 Saddle 的 `.target` 共用：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target`。

## 开发方式（主控分派）

- 这个项目的开发任务由主控（`paddock/main`）拆开，派给别的 agent 做。主控负责拆任务、写任务文件、审查、合并，不自己写功能代码。分派时按 corral-dispatch 技能做；遥测是否记录按技能规定，默认不开。
- 被委派的 agent（任务文件里写明了身份）照任务文件做，不再往下派。
- 需求单只写用户要的结果；验收照抄用户原话，不补验收点。主控觉得该加的，列出来问用户。调研和设计文档里的“建议”不是用户的验收条件。
- agent 名字以 `paddock/dev-` 开头；任务文件放 `docs/任务/`；每个任务一个分支，worktree 放 `../paddock-worktrees/<分支>`，交叉审查用 detached worktree `../paddock-worktrees/review-<分支>`。
- 创建派发的 agent 时，在 `corral start` 参数里注明职责：实现者加 `--label role=implementer`，独立审查者加 `--label role=reviewer`，实测用的 agent 加 `--label role=test`，主控用 `role=controller`。标签只用于显示，不改变职责分工或权限。
- 审查：主控审查每个任务。
- 合并：审查通过后，本地合并进 main。现在没有远程仓库；以后用户配置了 `origin`，就在合并后推送。主控不自行创建远程仓库。
- 收尾记号：一件活合并完、worktree 和分支清干净之后，在 main 上补一条空提交（`git commit --allow-empty`），首行写「收尾: 」加一句话说明这件活。只记真正落地的活。
- 收尾之后更新 `HANDOFF.md`：现在在哪、下一步、悬着什么。设计和理由进 `docs/DESIGN.md`，别写进交接文件。
- 清掉某个 worktree 时，把住在里面的 agent 一并关掉；其余的用户说关才关。
