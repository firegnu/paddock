# paddock：设计

状态：原型阶段（2026-10-05）。本文记录已定决定、与 Saddle 的边界和待定问题；完整调研见 `docs/调研/T76-GPUI桌面化详细方案.md`（Saddle 原文的副本，作依据，不随本文更新）。来历和用户原话见 `docs/背景与决策记录.md`。

## 1. 是什么

Saddle 的 GPUI 桌面前端：在原生窗口里查看 corral agent、运行真实交互终端，不再依赖外层终端程序。终端体验以 Zed 内置终端为参照（用户已认可该水平）。

Saddle 本身（含 TUI、插件、Drover、遥测、`saddle ctl`）继续独立存在。paddock 不是 Saddle 的替代品，也不是“把 Saddle 改成 GPUI”。

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

## 3. 与 Saddle 的边界

- **依赖方向单一**：paddock 依赖 Saddle 的库；Saddle 不知道 paddock 存在。
- **引用方式**：git 依赖，固定提交号（当前 `df1c727`，Saddle 公开仓库 main 上的提交；到 2026-10-05 的 main 头 `a81f710` 之间只有文档变化）。不用路径依赖。
- **升级引用**：是单独任务，需要：
  - 查看两个提交之间 Saddle 公开接口和依赖的变化；
  - 同步与 Saddle 交换类型的库版本；
  - 更新锁文件，跑全量检查。
- **复用的 Saddle 模块**（原型已验证能不改 Saddle 接上）：

  | 模块 | 用途 |
  | --- | --- |
  | `pty::Session` | PTY 会话 |
  | `viewer::Viewer` | shell 与 `corral attach` 生命周期 |
  | `terminal::Screen` | alacritty 解析与终端查询应答 |
  | `input::encode_key` / `encode_mouse` / `encode_paste` | 输入编码 |

  Saddle 的公开接口按对外接口对待。
- **需要 Saddle 改动时**：写成需求交给用户/Saddle 主控，在 Saddle 自己的流程里做，优先新增接口、不改旧接口，保持 TUI 行为不变。原型发现的候选需求见 §5。
- **插件**：Saddle 的进程插件通过字符帧协议画面。若 paddock 将来承载插件，协议归 Saddle 所有，paddock 只实现宿主侧，不改协议。尚未验证。
- **Corral、Drover、遥测**：只通过公开命令或 Saddle 库的封装使用。Drover 每个用户只允许一个插件进程持有数据：paddock 与 Saddle TUI 同时运行并都启用 Drover 时会冲突，规则待定（§7）。

## 4. 依赖与工具链隔离

- 每个 Cargo 清单有自己的 `[workspace]` 和 `Cargo.lock`。原型锁文件中第三方库版本与 Saddle 根锁文件一致，只多出 GPUI 带来的依赖。
- 编译目录：`$HOME/Developer/personal_projs/paddock-worktrees/.target`，不与 Saddle 共用。
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
- **未经实际操作**：键盘直接输入、真实中文输入法组字、Enter/Shift+Enter/Ctrl-C、⌘C/⌘V、鼠标选择与点击、滚动、拖动缩放窗口、与 Zed 并排的手感。等用户体验。
- **Saddle 侧缺口**（候选需求，未提交给 Saddle）：
  1. 没有“有新输出”的通知，原型每 8 ms 轮询。
  2. 网格读取与默认配色绑定 ratatui，原型各写了一份。
  3. 输入编码函数参数是 crossterm/ratatui 类型。
  4. `Session::spawn` 不能控制环境变量。用户体验时实际遇到：从 Claude Code 会话里启动原型，窗格中的 Claude Code 继承了 `CLAUDECODE`、`CLAUDE_CODE_CHILD_SESSION`、`CLAUDE_CODE_MESSAGING_*` 等变量，会话记录被关闭。原型只清理 corral 和 Saddle 的六个身份变量（`CORRAL_NAME`、`CORRAL_INSTANCE`、`CORRAL_EVENTS`、`SADDLE_INSTANCE`、`SADDLE_PANE`、`SADDLE_REVISION`）。
  5. 写入通道容量 64，可能在极大粘贴时返回 busy。
- **GPUI 方面的发现**：
  - 单独排版的宽字符上自带删除线不显示，原型自己画。
  - 窗口被完全遮挡时暂停绘制。

## 6. 建议路线（未批准）

以下是作者的建议，需要用户决定：

1. 先由用户实际体验原型（键盘、中文输入法、滚动复制、窗口缩放），与 Zed 内置终端对比。
2. 体验过关后，再决定是否做完整桌面版，以及路线：
   - 调研推荐“原生外壳（窗口、Agents 侧栏、标签/分屏、终端）＋字符面兼容层（现有页面和插件先用字符网格承载）”；
   - “整窗字符面起步”为退路。
   - 两条路线都依赖 Saddle 提供更多可复用接口。在保持 Saddle 独立的前提下，每项都要作为 Saddle 的新增接口经用户同意，这是 paddock 最大的不确定因素。
3. 体验不过关时，可回到继续在外层终端里用 Saddle TUI。

## 7. 待定问题

1. ~~原型体验是否达到要求~~：已通过（10-05）；第一阶段范围已定（§9），之后做到哪一步仍待定。
2. 生产用 GPUI 依赖渠道：固定官方仓库提交，还是继续 `gpui-pre` 快照。
3. pre-1.0 的 GPUI 是否符合“只用成熟、活跃维护的库”；是否接受工具链跟随最新稳定版 Rust。
4. 迁移路线（§6）；是否引入 gpui-component；首期是否只做 macOS。
5. 哪些 Saddle 接口可以为 paddock 新增（§5 的候选需求），由谁、何时在 Saddle 里做。
6. paddock 与 Saddle TUI 同时运行时，Drover 持有权和布局文件的规则。
7. 许可、应用名与标识、签名公证，以及何时建远程仓库。
8. paddock 的任务是否纳入 Saddle 的 Tasks（Drover）管理。

## 8. 主题

用户 10-05 定下（原话和选项见 `docs/背景与决策记录.md` §6）：

- **预置主题沿用 Saddle**：Dune、Tide、Lagoon 的界面色从所引用 Saddle 提交的公开接口读取（`saddle::theme::Preset`、`Theme`、`parse_color`），不复制色值；升级 Saddle 引用时一并检查主题变化。
- **每套主题自带终端调色板**：16 个基本色，以及终端默认字色、底色、光标、选区。Saddle 的这些颜色由外层终端决定，paddock 没有外层终端，所以要自己定。Saddle 主题里的 `Reset`/默认值也要在 paddock 里换成具体颜色。调色板由实现者出方案，截图给用户批准。
- **Saddle 的 Terminal 主题**（全部跟随外层终端）在 paddock 里没有对应物：不提供，或另作处理，实现时给方案。
- **配置**：paddock 自己的配置文件，写法照 Saddle：`theme = "…"` 选预置主题，`[colors]` 覆盖单个颜色，最终颜色＝预置＋覆盖。不读写 Saddle 的配置文件。
- **第一阶段**只用配置文件，改完重启生效；不做设置页。
- **调色板**（用户 10-05 看截图后批准）：Dune 取 Gruvbox dark，Tide 取 Nord，Lagoon 取 Everforest dark（均 MIT，出处写在 `app/src/theme.rs`）；除 0 号外 15 色在底色上对比度 ≥3:1。终端配色的 `[colors]` 键为 `terminal_*`，见 `app/README.md`。
- **terminal 主题**：不提供，写了报错。

## 9. 第一阶段

用户 10-05 批准（原话见 `docs/背景与决策记录.md` §6）：

- 一个窗口：左侧 Agents 列表，右侧一个终端窗格。
- 点哪个 agent 就接入哪个（`corral attach`），也能开普通 shell。
- §8 的主题系统。
- 不做：分屏、标签、插件页面、Drover、布局保存、设置页。

第一批任务的划分（用户 10-05 同意）：

- 代码位置：原型提升为 `app/`，不另起新代码。原型 README 改存为 `docs/原型实测记录.md`。
- 侧栏：每个 agent 一行，显示状态色点、名字、状态（精简版）。以后逐步加强度、会话标题、分支和改动数，每样单独做；所以每一行做成独立组件，侧栏宽度可在配置中设置。
- 任务：T1 应用骨架（先做）→ T2 主题系统、T3 Agents 侧栏并行。任务文件见 `docs/任务/P1-T*.md`；T2 和 T3 之间的约定是 T1 定下的 `Theme::fg/bg` 取色接口。
