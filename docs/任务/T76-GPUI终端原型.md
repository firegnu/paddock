> 参考副本：复制自 Saddle 仓库 `docs/任务/T76-GPUI终端原型.md`（分支 t76-gpui-prototype，提交 2c59e17），含主控派发内容和原型完成记录。文中的 worktree、分支和路径指 Saddle 仓库；原型已搬到本仓库 `prototypes/gpui-terminal/`。

# T76：GPUI 单窗格终端原型

2026-10-05，saddle/main 交给 saddle/dev-claude-1（Claude Code，既有 instance 1e0e83ee0a3e）。
类型：功能变更
依据：用户确认进入终端原型阶段，在独立 worktree 开发，不污染主分支，并指定原研究作者继续。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
路由：常规 / 交叉审查不要 / 影响面：改行为（公开 route 三项均拿不准；原型隔离且不改生产核心，由主控审查）。用户指定沿用当前实例，保留其现有模型、强度和启动配置；该实例创建时没有显式记录模型与强度，不冒称已切换档位。
你是被委派的实现者：照本文件做，不再委派开发或审查。

## 用户授权与完成依据

用户前一轮问：“dev-claude-1关于gpui的研究也更新文档了。我觉得是不是可以做原型了？”

主控提出：一个 GPUI 窗口、一个真实终端窗格，先接现有 PTY 和终端内核；跑 shell 和专门创建的测试 Claude，重点看中文输入法、字符对齐、滚动复制、窗口缩放和大量输出时的响应；与 Zed 并排体验，交付可运行原型与实测记录，明确复用和改动边界。侧栏、分屏、插件页面和整体迁移路线留到终端验证之后。

用户确认原话（验收及隔离要求照录）：

> 对，就是在一个独立的worktree上开发。不要污染主分支。你可以交代给dev-claude-1做了。

研究中用户已表态：“其实zed的终端我试过，是可以满足我的要求的”。以这个体验作参照，不将静态截图或构建通过写成已经达到同等体验；主观体验最终留给用户判断。

## 在哪里干活

- 唯一可写 worktree：`/Users/firegnu/Developer/personal_projs/saddle-worktrees/t76-gpui-prototype`。
- 分支：`t76-gpui-prototype`，从研究提交 `ea067ea` 创建；它相对 main `df1c727` 只有两份研究文档，生产源码基线相同。
- 原型放 `prototypes/gpui-terminal/`，使用独立 Cargo manifest/workspace 与 Cargo.lock。原型按路径依赖当前 worktree 的 Saddle 库来检验复用，不修改主工作区的 manifest/lock。
- 只修改该原型目录和本任务书。运行说明、实测记录放原型目录。
- 你的启动 cwd 历史上是主仓库；用户明确指定继续使用你，因此本轮沿用实例。所有工具操作必须明确指定上述 worktree/绝对路径，不依赖默认 cwd。每次 git 写操作显式 `git -C /Users/firegnu/Developer/personal_projs/saddle-worktrees/t76-gpui-prototype ...`，写前核对分支。
- 原研究 worktree `t76-gpui-research` 保留只读参考；T49 设计在另一 worktree 独立进行，不干扰。

## 先读

- 本 worktree `AGENTS.md`。
- `docs/调研/T76-GPUI桌面化详细方案.md` §4.1、§7、§14.4–14.7；最新范围是只做终端一轨，不是旧版三轨。
- `src/lib.rs`、`src/pty.rs`、`src/terminal.rs`、`src/viewer.rs`、`src/input.rs`，按实际接入需要读。
- `docs/UI回归.md` 的分层验证原则。

## 要做的

交付本机可运行的 macOS GPUI 单窗格终端原型，覆盖上述已确认的体验范围。复用现有 PTY 和终端内核是本轮要验证的目标，不是已验证结论。

依赖采用研究建议的固定版本 `gpui-pre` 快照，记录实际来源与版本；保持 Rust stable。原型可以使用独立的显式工具链，但不改机器默认、主仓库工具链或用户配置。需要超出这个范围的系统安装时先报告。

实现只用 GPUI 公共接口及自身代码。外部参考先核对具体文件来源和许可，保留必要声明；不复制 Zed GPL 应用层代码。本轮不决定 Saddle 最终许可和正式发布依赖。

记录实际启动命令、操作方式、通过/不足/未测项，以及现有模块的复用结果。中文输入法必须区分字符直接写入与真实输入法组字；没能亲自操作验证的，明确留待用户体验，不能由截图推断通过。

## 验证预算

- 按项目轻量 TDD 约定，对原型新增的确定性行为先有针对性自动检查，再做最小实现；纯绘制不编造失败测试。
- 使用原型自己的 manifest，运行其针对性测试、构建与 clippy；只覆盖原型影响面，不跑未改动的 Saddle 全量套件。
- 实际运行原型，检查已确认的终端体验主路径；用合成内容。需要真实 agent 时只创建自己的 `saddle/test-t76-*` 测试实例，用完通过公开 corral 命令关闭，不借用用户现有会话。
- 所有 Cargo 命令使用 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/saddle-worktrees/.target`，并显式指定原型 manifest。
- `git diff --check`，核对改动仅在允许范围。检查通过且相关代码未改不重复执行；编译失败如实定位，不扩大成仓库升级。

## 不要做

- 不修改主仓库工作区、main 分支、生产源码、根 Cargo 文件、正式 DESIGN、HANDOFF、用户技能或配置。
- 不做 Agents 侧栏、分屏、插件面板、状态持久化、app.rs 重构、Ghostty 内核替换、打包签名或发布安装。
- 不覆盖现用 Saddle，不重启现用 Saddle/Corral，不修改 Corral 仓库或读取其内部状态目录。
- 不读取私人会话/真实任务数据制作样例；不对用户既有 agent 发话、attach 输入或停止。
- 不操作 Tasks 队列，不把原型完成标为 T76 Done；此前 T76 dispatch 被拒绝，Pending 不等于已经进入队列执行。
- 不启用遥测，不额外开研究或审查 agent。
- 不按项目名/路径批量杀进程；只管理本轮自己创建并记录身份的测试进程。
- 如果现有接口无法在不改生产源码的条件下接入，报告具体缺口、已有原型结果和最小建议，不自行扩展到正式重构。
- 不合并、不推送、不删除 worktree。只在 `t76-gpui-prototype` 提交。

## 完成

在本文件末尾追加完成记录：实现内容、验证结果、运行方式、已知不足及待用户体验部分。原型和文档一并在本分支提交。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

本轮停在原型审查和用户体验，不能推导为完整桌面迁移获批或 T76 全部完成。

## 完成记录

2026-10-05，saddle/dev-claude-1。只在 `prototypes/gpui-terminal/` 和本任务书内改动；没有改生产源码、根 Cargo 文件、正式设计文档或用户配置，没有合并、推送、安装或操作 Tasks，没有开启遥测或另开研究/审查 agent。

### 实现内容

- 独立 Cargo 工作区与锁文件的 GPUI 单窗格终端：`gpui-pre =0.3.8`（zed@279fe07 快照，crates.io，Apache-2.0），本机默认 Rust 1.96.0。
- 按路径依赖本 worktree 的 Saddle 库，原样复用 `pty::Session`、`viewer::Viewer`（shell 与 `corral attach` 生命周期）、`terminal::Screen` 的解析，以及 `input` 的按键/鼠标/粘贴编码。
- 新写：GPUI 按键到 crossterm 按键的适配、像素与格子换算、制表线和方块字符按格子绘制、颜色解析、行拆分、输入法组字状态、基于 alacritty 变更标记的重绘判断、窗口与终端窗格。
- 运行说明、复用结果、缺口和实测记录见 `prototypes/gpui-terminal/README.md`。

### 验证结果

- 先写测试、确认因行为缺失失败（24 项红灯），再实现；最终 `cargo test` 31 项通过，`cargo clippy --all-targets -- -D warnings` 无警告，均使用原型自己的 manifest 和共享编译目录。
- 实际运行（只截原型窗口）：合成样例渲染、表格和框线对齐、删除线、Nerd Font 图标、两种窗口尺寸下的行列协商、交互 shell 启动与关闭、身份变量清理、大量输出，均通过。
- 真实 agent：自建 `saddle/test-t76-claude`（Claude Code），原型接入后界面与中英混排/表格/表情/代码块回复显示正确；关闭窗口后实例继续运行、接入数归零；测试后已用公开 `corral stop` 关闭。没有接触用户已有的 agent。
- 未运行 Saddle 全量套件：生产代码没有改动，原型不在根工作区内。

### 关键取舍与发现

- Xcode 27 缺 Metal 工具链组件；安装属于系统安装，没有做，改用 GPUI 的 `runtime_shaders` 特性（运行时编译着色器）。
- 不改生产源码即可接入；记录的缺口：没有新输出通知（原型 8 ms 轮询）、网格读取与配色绑定 ratatui、输入编码参数为 crossterm/ratatui 类型、`Session::spawn` 不能控制环境变量、写入通道有界。
- GPUI 自带删除线在单独排版的宽字符上不显示，原型改为自己画；窗口被完全遮挡时 GPUI 暂停绘制，可见后恢复。
- 测试中曾两次按屏幕区域截图而拍到用户屏幕上的其他窗口；之后改为只按窗口编号截图，那两张已从临时目录删除，未入库。

### 待用户体验

我没有向原型窗口输入任何按键（模拟系统按键可能误打进用户的其他窗口）。键盘直接输入、真实中文输入法组字、Enter/Shift+Enter/Ctrl-C、⌘C/⌘V、鼠标选择与点击、滚动、拖动缩放窗口、与 Zed 并排的手感，都未经实际操作验证，留待用户判断。详细清单和建议步骤见 README。

本轮停在原型审查和用户体验，不代表完整桌面迁移获批或 T76 完成。
