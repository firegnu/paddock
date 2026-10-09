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
  - 和 Saddle 生态只通过公开约定打交道：`corral`、`ranch` 命令及其 JSON 输出（DESIGN §3）。遥测、Drover、插件系统已砍掉（用户 10-05）。corral 由独立仓库 ranch（`../ranch`，本主控兼管）维护，paddock 只调用它装好的命令；改 ranch 在 ranch 自己的仓库和规矩里做。不读 Saddle 的内部状态和配置文件。
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
  - 启动测试或截图用的 paddock 时加 `PADDOCK_NO_ACTIVATE=1`，不让它抢到前台：否则用户在别处打的字会落进测试窗口（P5-2 时发生过）。代价是窗口被挡住时 GPUI 暂停重画，截到的可能是刚启动那一帧；要看完整效果的画面，在完成记录里写明留给用户实际看。
  - 不要用脚本模拟系统按键或鼠标（如 `osascript` 的 System Events），可能打进用户正在用的窗口。键盘、输入法、鼠标交互留给用户实际体验。
  - 截图不入库，尤其是带账号用量等信息的画面。
  - GPUI 在窗口被完全遮挡时暂停绘制，截到旧画面不等于没更新。
- **测试不依赖真实 agent**：用合成数据和假 `corral` 脚本；真实 agent 只用于明确的实测，且只用自己开的测试实例。
- **验证**：按改动影响面选择检查。小改跑直接相关测试；跨模块改动和合并前，对每个 Cargo 清单跑 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 和 `cargo fmt --check`（用户 10-07 定）。保留有价值的测试，不靠删测试、放宽断言或缩短超时来通过。
- **独立编译目录**：paddock 的编译产物都放在 `$HOME/Developer/personal_projs/paddock-worktrees/.target/` 下，不和 Saddle 的 `.target` 共用；每个工作目录用自己的子目录，不和别的 worktree 共用：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/<子目录>`。主仓库用 `main`，任务 worktree 用分支名（如 `p1-font`），审查 worktree 用 `review`。原因：并行的 worktree 编的是同一个包，共用目录会互相覆盖产物，cargo 还可能把别人的产物当成最新的（P1 的 T2、T3 都遇到过）。清 worktree 时一并删掉它的子目录。

## 开发方式

- 每件活先在 `docs/任务/` 写任务文件（做成什么、范围、怎么算做完），给用户看过再动手（用户 10-05 对第三阶段那一批另有安排：不逐件等回复，一次做完后一并汇报，见 DESIGN §12）。验收照抄用户原话，不补验收点；主控觉得该加的，列出来问用户。调研和设计文档里的“建议”不是用户的验收条件。
- 每件活一个分支，worktree 放 `../paddock-worktrees/<分支>`，编译目录用 `.target/<分支>`（见上文“独立编译目录”）。
- 挑下一件活先看优先级（任务文件开头的 `优先：` 一行，没写的按中）；用户点名先做哪件，以用户为准（用户 10-09，P5-60a）。
- 看板约定（右侧栏 Kanban 靠这些把任务文件、worktree、agent 对上，DESIGN §13 P5-29，用户 10-07 同意）：任务文件名以编号开头（`P5-29a-…md`），首行 `# 任务：…`；「在哪里干活」一节第一行固定写 ``- worktree：`<路径>`，分支 `<分支>`（已从 main 建好）。``；要等别的活时，在「依据」后加一行 `依赖：<编号>`；开 agent 时加 `--label task=<编号>`；合并提交首行写「合并 <编号>：」，收尾空提交首行写「收尾: <编号> 」。
  - 一条合并或收尾提交只写一件活的编号（看板只认首行第一个编号）。
  - 决定不做的活：在 main 上补空提交，首行写「收尾: <编号> 不做：<原因>」，看板把它放进 DONE、标成 Dropped（用户 10-07 定，P5-29r2）。
  - 主控自己做的活（不派 agent）：在「依据」（有 `依赖：` 就在它）之后加一行 `执行：主控`，看板在这件活进行中时把主控当作它的 agent 显示（头像、实时状态，等用户时标 Needs you；用户 10-09 定，P5-67）。
  - 某件活在等用户（等实测、等回答）：在任务文件开头「依据」（有 `依赖：` 就在它）之后加一行 `待用户：<什么事>`，写完立即在 main 上提交；用户处理完删掉这一行，再提交；用户也可以在看板上点 Clear 自己清掉（paddock 只提交这一个文件，不推送）。看板据此显示 Needs you（用户 10-07 定）。
  - 优先级（用户 10-09 定，P5-60a）：在任务文件开头（第一个 `##` 之前，和 `待用户：` 一样）可选一行 `优先：高` 或 `优先：低`，放在开头那一段的末尾。没写就是中，老任务都按中；改回中就删掉这一行，不写 `优先：中`。只认高、中、低，别的值看板按中排并标出格式问题；有好几行时只认第一行。用户可以在看板 Queued 和 DRAFT 卡片上改档：已提交的由 paddock 改这一行并只提交这一个文件（`<编号>：优先级改为高`，不推送），草稿只改文件。看板的 Queued 按档排（高、中、低），同档草稿在前，再按编号。
  - 主仓库工作区里没提交的任务文件是草稿（看板显示 DRAFT，可能是用户在看板上新建的）：提交时按路径 `git add`，不用 `git add -A`，免得把别的草稿带进去。
  - 不要在主仓库工作区改已提交、还没合并的任务文件：分支会往同一份文件末尾追加完成记录，合并时 git 会因工作区有改动而停下。要改就改 worktree 里那份，或改完立即提交（P5-29r2 §3.1）。
- 合并前按影响面自查：至少跑 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings` 和 `cargo fmt --check`，在任务文件末尾写「完成记录」：做了什么、验证了什么、拿主意的地方、没做的事。
- 合并：本地合并进 main，合并后推送到 `origin`（`github.com/firegnu/paddock`，public，用户 10-05 建）。仓库是公开的：推送前确认没有密钥、截图和私人数据进入提交。
- 收尾记号：一件活合并完、worktree、分支和它的编译子目录清干净之后，在 main 上补一条空提交（`git commit --allow-empty`），首行写「收尾: 」加一句话说明这件活。只记真正落地的活（决定不做的活按上面看板约定写「不做」）。
- 收尾之后更新 `HANDOFF.md`：现在在哪、下一步、悬着什么。设计和理由进 `docs/DESIGN.md`，别写进交接文件。
- 需要真实 agent 实测时，照“不要干扰用户正在用的 agent”一条，自己开 `paddock/test-<名字>`（`--label role=test`），用完 `corral stop`。
