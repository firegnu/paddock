# paddock：设计

状态：第一阶段完成；迁移第一步 M0 已完成，不再依赖 Saddle 的库（2026-10-05）。本文记录已定决定、与 Saddle 的关系和待定问题；完整调研见 `docs/调研/T76-GPUI桌面化详细方案.md`（Saddle 原文的副本，作依据，不随本文更新）。来历和用户原话见 `docs/背景与决策记录.md`。

## 1. 是什么

用 GPUI 重做 Saddle 界面的独立桌面应用：在原生窗口里查看 corral agent、运行真实交互终端，不再依赖外层终端程序。终端体验以 Zed 内置终端为参照（用户已认可该水平）。

paddock 与 Saddle 在代码上完全分开：用到的 Saddle 代码迁入 paddock 自己维护，不再依赖 Saddle 的库；Saddle 的界面逐步用 GPUI 重做。Saddle 本身（含 TUI、插件、Drover、遥测、`saddle ctl`）继续独立开发和使用，不因 paddock 改变。

## 2. 已定决定

| 决定 | 依据 |
| --- | --- |
| 评估 GPUI 桌面化，先做详细调研 | T76 登记与 10-05 用户指示 |
| 终端以“GPUI＋alacritty_terminal 自己画”为路线，目标是 Zed 内置终端的水平；暂不追完整嵌入 Ghostty | 用户：“其实zed的终端我试过，是可以满足我的要求的” |
| 先做只有一个窗口、一个终端窗格的原型，在隔离环境开发 | 用户 10-05 批准 |
| 独立仓库 paddock，原型搬入，上下文写进本仓库 | 用户 10-05 决定 |
| paddock 与 Saddle 互不影响，尤其是 Rust 依赖 | 用户：“即使合并到主分支，也要隔离起来，不能两边互相影响。尤其是所使用的rust库。” |
| 原型体验通过 | 用户 10-05：“我觉得是过了” |
| 要有主题系统，像 Saddle 那样：预置主题＋用户覆盖 | 用户 10-05：“起码有一套theme系统，就像saddle那样。”细节见 §8 |
| 第一阶段范围：一个窗口、左侧 Agents 列表、右侧一个终端窗格，加主题系统 | 用户 10-05：“第一阶段范围就按你的建议”。细节见 §9 |
| **代码与 Saddle 完全分开**：用到的 Saddle 代码迁入 paddock 自己维护，不再依赖 Saddle 的库；所有界面用 GPUI 重做 | 用户 10-05：“paddock和saddle是完全分开的，或者paddock要把用的saddle的文件都迁移过来，所有saddle的界面都用gpui重做”，“按这个方向调整”。细节见 §3、§10 |

## 3. 与 Saddle 的关系

- **代码**：paddock 不依赖 Saddle 的库（M0 起，见 §10）。迁入的代码在 paddock 里自己维护，文件开头注明来源（Saddle 提交 `df1c727` 的哪个文件）。之后两边各自演化：Saddle 的修复不会自动进入 paddock，需要时作为 paddock 的任务手动移植。
- **只共享公开约定**：paddock 与 Saddle 生态只通过下面这些公开约定打交道，不读 Saddle 的内部状态和配置文件：

  | 约定 | 归属 | paddock 怎么用 |
  | --- | --- | --- |
  | `corral` 命令（`ls`/`status`/`attach`/`start`/`stop` 等）及其 JSON 输出 | Corral（随 Saddle 安装） | 照公开格式调用；格式变化时 paddock 跟进 |
  | 插件协议 | Saddle | 若承载插件，paddock 只实现宿主侧，不改协议（尚未验证） |
  | 遥测命令（`saddle telemetry …`）、Drover 的公开命令 | Saddle | 需要时照公开命令调用 |

- **运行时要求**：机器上要有 `corral` 命令。本机的 `corral` 随 Saddle 安装（`~/.local/share/saddle/versions/…/bin/corral`）。paddock 不需要 Saddle TUI 在运行。
- **不改 Saddle**：不修改 Saddle 仓库。原先打算请 Saddle 新增的接口（§5 的缺口）改由 paddock 在自己的代码里解决；只有公开约定本身要变时，才写成需求交给用户/Saddle 主控。
- **Drover 并存**：Drover 每个用户只允许一个插件进程持有数据：paddock 与 Saddle TUI 同时运行并都启用 Drover 时会冲突，规则待定（§7）。
- **许可**：Saddle 仓库目前没有许可证文件。代码属于用户本人，迁入没有问题；paddock 公开发布前，需要先为两边定好许可（§7）。

## 4. 依赖与工具链隔离

- 每个 Cargo 清单有自己的 `[workspace]` 和 `Cargo.lock`。
- 不再有 `saddle` 依赖（M0；锁文件因此少了 32 个包，其余版本未变）。`alacritty_terminal`、`portable-pty` 等库的版本由 paddock 自己决定，不再要求与 Saddle 一致；升级照常作为单独任务。`ratatui`、`crossterm` 只因迁入代码的接口类型而存在，在迁移的第二步去掉（§10）。
- 编译目录：`$HOME/Developer/personal_projs/paddock-worktrees/.target/<子目录>`，不与 Saddle 共用；每个工作目录一个子目录（主仓库 `main`、任务 worktree 用分支名、审查用 `review`），避免并行 worktree 互相覆盖同名包的产物（用户 10-05 同意）。
- GPUI：`gpui-pre =0.3.8` / `gpui-pre-platform =0.3.8`（zed@279fe07 的第三方快照，发布者 huacnlee，Apache-2.0），特性 `font-kit`、`runtime_shaders`。所有 `gpui-pre-*` 一起精确固定、一起升级。
- `runtime_shaders`：本机 Xcode 27 缺 Metal 工具链组件；安装属于系统安装，未做，改为运行时编译着色器。
- 工具链：本机默认 stable `rustc 1.96.0` 可以编过；暂未加 `rust-toolchain.toml`。写精确版本会让 rustup 另装一份同版本工具链，等 GPUI 要求更新的 Rust 时再与用户确定。

## 5. 原型结论（2026-10-05）

详见 `docs/原型实测记录.md`。

- **已验证**：
  - 合成样例渲染：中英混排、表情、表格与框线对齐、样式、颜色。
  - 窗口尺寸与 PTY 行列协商。
  - 交互 shell 启动与关闭。
  - 真实 Claude Code 接入：显示正确；关闭窗口后 agent 继续运行。
  - 大量输出不拖慢，约 100–120 帧/秒。
  - 用户实际体验：在 Claude Code 里中文输入和显示正常（10-05）。
- **原型发现的缺口**（原打算请 Saddle 新增接口；迁移后由 paddock 在自己的代码里解决，按需排期）：
  1. 没有“有新输出”的通知，原型每 8 ms 轮询。
  2. 网格读取与默认配色绑定 ratatui，原型各写了一份。
  3. 输入编码函数参数是 crossterm/ratatui 类型。
  4. `Session::spawn` 不能控制环境变量。用户体验时实际遇到：从 Claude Code 会话里启动原型，窗格中的 Claude Code 继承了 `CLAUDECODE`、`CLAUDE_CODE_CHILD_SESSION`、`CLAUDE_CODE_MESSAGING_*` 等变量，会话记录被关闭。T1 起 paddock 启动时按名单清理 17 个身份变量。
  5. 写入通道容量 64，可能在极大粘贴时返回 busy。
  6. 程序向终端查询颜色时，应答的是 xterm 默认值，不是 paddock 主题的调色板。
- **GPUI 方面的发现**：
  - 单独排版的宽字符上自带删除线不显示，原型自己画。
  - 窗口被完全遮挡时暂停绘制。

## 6. 路线

- **已定**：所有界面用 GPUI 重做（§2）。先做迁移（§10），再逐步补界面。
- **建议的顺序（未批准）**：
  1. 迁移：去掉 `saddle` 依赖，行为不变（M0，已完成）；再去掉 `ratatui`、`crossterm`（M1）。
  2. 第二阶段，补日常离不开的：标签页和分屏（同时解决切换时 shell 被结束）、Agents 面板补齐信息、菜单栏和 `.app` 打包。
  3. 第三阶段：插件（先 Drover）、Attention、新建 agent、Settings 页、历史搜索、布局保存等。
- **插件界面怎么做待定**（§7）：在 GPUI 里原生重做，或先做“字符面兼容层”按插件协议显示插件自己画的字符画面。

## 7. 待定问题

1. ~~原型体验是否达到要求~~：已通过（10-05）；第一阶段已完成，之后的阶段范围待定（§6）。
2. 生产用 GPUI 依赖渠道：固定官方仓库提交，还是继续 `gpui-pre` 快照。
3. pre-1.0 的 GPUI 是否符合“只用成熟、活跃维护的库”；是否接受工具链跟随最新稳定版 Rust。
4. ~~迁移路线~~：已定为全部用 GPUI 重做（10-05）。仍待定：是否引入 gpui-component；首期是否只做 macOS。
5. 插件界面：原生重做，还是先做字符面兼容层（§6）。
6. paddock 与 Saddle TUI 同时运行时，Drover 持有权和布局文件的规则。
7. 许可（paddock 和 Saddle 都还没有）、应用名与标识、签名公证，以及何时建远程仓库。
8. paddock 的任务是否纳入 Saddle 的 Tasks（Drover）管理。

## 8. 主题

用户 10-05 定下（原话和选项见 `docs/背景与决策记录.md` §6）：

- **预置主题沿用 Saddle 的 Dune、Tide、Lagoon**。原先从 Saddle 公开接口读取界面色；用户 10-05 定为与 Saddle 分离，随迁移（§10）把 Saddle `df1c727` 中这三套的色值复制进 paddock，注明来源，此后独立维护，不再自动跟随 Saddle 的主题改动。
- **每套主题自带终端调色板**：16 个基本色，以及终端默认字色、底色、光标、选区。Saddle 的这些颜色由外层终端决定，paddock 没有外层终端，所以要自己定。Saddle 主题里的 `Reset`/默认值也要在 paddock 里换成具体颜色。
- **配置**：paddock 自己的配置文件，写法照 Saddle：`theme = "…"` 选预置主题，`[colors]` 覆盖单个颜色，最终颜色＝预置＋覆盖。不读写 Saddle 的配置文件。
- **第一阶段**只用配置文件，改完重启生效；不做设置页。
- **调色板**（用户 10-05 看截图后批准）：Dune 取 Gruvbox dark，Tide 取 Nord，Lagoon 取 Everforest dark（均 MIT，出处写在 `app/src/theme.rs`）；除 0 号外 15 色在底色上对比度 ≥3:1（有测试）。终端配色的 `[colors]` 键为 `terminal_*`，见 `app/README.md`。
- **terminal 主题**：不提供，写了报错。
- **字体**（T4）：`font`、`font_fallbacks`、`font_size`、`line_height` 写在同一配置文件；命令行参数优先。

## 9. 第一阶段（已完成）

用户 10-05 批准（原话见 `docs/背景与决策记录.md` §6）：

- 一个窗口：左侧 Agents 列表，右侧一个终端窗格。
- 点哪个 agent 就接入哪个（`corral attach`），也能开普通 shell。
- §8 的主题系统。
- 不做：分屏、标签、插件页面、Drover、布局保存、设置页。

第一批任务的划分（用户 10-05 同意）：

- 代码位置：原型提升为 `app/`，不另起新代码。原型 README 改存为 `docs/原型实测记录.md`。
- 侧栏：每个 agent 一行，显示状态色点、名字、状态（精简版）。以后逐步加强度、会话标题、分支和改动数，每样单独做；所以每一行做成独立组件，侧栏宽度可在配置中设置。
- 任务：T1 应用骨架、T2 主题系统、T3 Agents 侧栏、T4 字体配置，均已合并。任务文件见 `docs/任务/P1-T*.md`。

## 10. 迁移：去掉 `saddle` 依赖

目标：paddock 不再依赖 Saddle 的库，行为不变。

- **迁入范围**：paddock 现在用到的 Saddle 模块（Saddle `df1c727`，约 2200 行）：

  | Saddle 文件 | paddock 用它做什么 |
  | --- | --- |
  | `pty.rs`、`terminal.rs` | PTY 会话、alacritty 解析与终端查询应答 |
  | `viewer.rs` | shell 与 `corral attach` 的生命周期 |
  | `input.rs` | 按键、鼠标、粘贴编码 |
  | `corral.rs`、`agents.rs` | 取 agent 列表，排序、分组、判断状态 |
  | `theme.rs` | Dune、Tide、Lagoon 的界面色和颜色写法解析 |

  这些文件还引用了少量 Saddle 内部代码（`command::run`、`history::History`、`git` 的摘要类型、`layout_state::Content`）。只迁 paddock 实际走到的部分；只服务 TUI 的部分（如 `Screen::render` 写 ratatui 缓冲区、历史模式）不迁。
- **原则**：
  - 迁移本身不改行为，现有测试照样通过；Saddle 里对应模块的测试一并迁入。
  - 每个迁入文件开头注明来源文件和提交号。
  - 分两步（用户 10-05 定）：第一步只迁移，`ratatui`、`crossterm` 类型照原样保留；第二步单独任务，换成 paddock 自己的类型，去掉这两个依赖。
- **之后**：AGENTS.md 中“只通过 git 依赖按固定提交号引用 Saddle”“升级 Saddle 引用”“与 Saddle 交换类型的库同版本”等规矩已在 M0 改写为 §3 的关系。
- **M0 结果**（10-05）：迁入 `pty`、`terminal`、`viewer`、`input`、`corral`（含 `command::run`）、`agents`、`preset`（原 `theme`）；Saddle 对应测试迁入 `app/tests/`，原 Python 假程序改为 shell 脚本。`agents::Panel` 中只服务 TUI 键盘操作的字段和方法暂时保留（`absorb` 依赖它们、迁入的测试覆盖它们），留待界面重做时再清理。
