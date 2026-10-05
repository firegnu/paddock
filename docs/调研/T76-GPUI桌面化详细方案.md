> 参考副本：复制自 Saddle 仓库 `docs/调研/T76-GPUI桌面化详细方案.md`（分支 t76-gpui-research，提交 ea067ea）。文中的相对链接和 `路径:行号` 指 Saddle 仓库。paddock 当前的决定以 `docs/DESIGN.md` 为准；本文作历史依据，不随 paddock 更新。

# T76：用 GPUI 把 Saddle 演进为桌面应用——详细调研与方案

调研日期：2026-10-05。作者：saddle/dev-claude-1（被委派，任务书见 [T76 任务书](../任务/T76-GPUI详细调研与方案.md)）。
性质：静态研究与方案建议。**没有写原型、没有编译或运行任何 GPUI 程序、没有改源码/依赖/配置**。本文不批准任何架构，所有“建议”都等用户决定。凡是说“复用”“保留”“不变”“兼容”的地方，都是复用目标或静态可行性判断，不是已验证的结果。

修订：2026-10-05 按主控首轮审查修正三类表述（未验证的复用/兼容/功能完整性断言、原型三项的来源、许可结论），记录见任务书「首轮修订记录」。事实、来源与推荐方向未变。

补充：2026-10-05 方案复核通过后，把用户与作者的后续讨论补成 §14（发布状态澄清、GPUI 之外的路线、终端集成与 Ghostty 嵌入现状、用户认可 Zed 内置终端的水平、herdr-gpui、作者意见的变化）。§1–§13 的正文只加了指向 §14 的提示（§5 末段另有一处措辞微调），原有事实与建议未改；两处不一致时，§14.6 是作者的最新意见。

文中标记：

- 【查证】当天直接从官方源码、crates.io 接口或本仓库文件读到的事实，附来源。
- 【建议】基于事实的推荐做法。
- 【推测】合理但没有直接证据的判断。
- 【待验证】必须靠原型或实际运行才能确认的事项。

本仓库依据写成 `路径:行号`，对应分支 `t76-gpui-research`（基于 main 的 df1c727）。GPUI 源码依据固定在 Zed 提交 `279fe070`（2026-10-04 23:59 UTC），链接见 §13。

---

## 1. 结论先行

1. **能用 GPUI 写，方向在技术上成立。**【查证＋推测】GPUI 提供自定义元素、等宽强制字宽的文本绘制、输入法接口和按键/鼠标事件，Zed 自己的终端就是用同一个终端解析库（alacritty_terminal）加自定义元素做的；外面已有多款用“GPUI + alacritty_terminal + portable-pty”做的终端/agent 工作台。Saddle 现在用的正是 alacritty_terminal 0.26 和 portable-pty 0.9，所以终端解析与 PTY 这一层是**复用目标**：静态看不必换库，但绘制、输入和生命周期怎么接到 GPUI 仍要适配（§3.3、§4.1）。“成立”目前只是读源码得出的判断，**没有任何一行在本机跑过**。
2. **如果走这条路，Saddle 会变成独立桌面应用，不再依赖外层终端。**随之而来的代价也明确：GPUI 窗口不能通过 SSH 使用，手机 SSH 这条路只有保留 TUI 才有。
3. **最大的不确定性不在“能不能画出来”，而在三处：**
   - GPUI 本身：官方仍标注 pre-1.0、经常有破坏性变更、要求最新稳定版 Rust；官方 crates.io 版本停在 2025-10 的 0.2.2，现在实际可用的渠道是“固定官方仓库某个提交”或“第三方每周快照 gpui-pre”。是否符合本项目“只用成熟、活跃维护的库”，需要用户裁定。
   - 现有代码：`src/` 约 3.17 万行里，约 2.0 万行所在文件直接引用了 ratatui/crossterm，且界面状态、绘制、命中区混在一起；`src/app.rs` 是一个集中所有状态的主循环。迁移成本主要在拆这部分，不在终端。
   - 中文输入法、按键优先级、从 Dock 启动时的环境变量、字符网格里中文与符号的对齐——这些在 TUI 里由外层终端代劳，桌面版要自己负责，全部【待验证】。
4. **推荐路线【建议】**：不做一次性重写，也不退役 TUI。
   - 先做一个限时、可丢弃的原型（§7），建议只验证三件事：真实可交互的终端窗格、Agents 侧栏、基础分屏；顺带验证一块“字符面”能否把现有插件画面画出来。这三件事来自此前讨论的建议，不是用户新增的验收点；原型做不做、做哪些，待用户决定。
   - 原型过关且用户决定继续后，走“**原生外壳 + 字符面兼容层**”：窗口、侧栏、标签/分屏、终端窗格用 GPUI 原生重做；设置、遥测、新建 agent 等现有页面和进程插件设想先通过字符面承载，之后按价值逐页原生化。字符面能否承载这些页面和插件，尚未验证。
   - TUI 保留为 SSH/手机入口，与桌面共用同一套核心。
5. **插件契约预期不用动。**【查证】的部分只有：插件协议 crate 不依赖 ratatui，画面是“按行的样式片段”。由此得出的是静态可行性判断【推测】：桌面宿主有条件按协议绘制这些帧，目标是 Drover、Diff、Counter 的插件程序本身不改。**不能从“协议不依赖 ratatui”推出现有插件已经兼容桌面**：宿主侧的绘制、输入映射和生命周期接入都要重新接并验证（§4.3）。在这个前提下，T61/T70 的插件文档建议按现有字符帧契约继续写。
6. **按本方案的设计意图，Corral、Drover、遥测、`saddle ctl` 的公开边界不因换前端而改变**（§4.4）；能否做到取决于桌面宿主的接入实现，未验证。已知需要处理的是两个实例同时运行时 Drover 的单一持有者锁，以及桌面包里仍要带命令行入口。
7. **提交后的讨论补充见 §14。**用户表明最担心终端集成，并认可 Zed 内置终端的水平；作者据此建议原型先只做终端一条轨，以“不比 Zed 内置终端差”为验收参照，迁移路线等原型之后再选。这些是作者的建议，尚无新的决定获批。

---

## 2. 查证事实：GPUI 的现状

### 2.1 来源、版本与官方定位

| 事项 | 事实 | 来源 |
| --- | --- | --- |
| 所在位置 | Zed 仓库内的 `crates/gpui`，与编辑器同仓开发 | zed@279fe070 `crates/gpui/Cargo.toml` |
| 仓库活跃度 | 默认分支最近提交 2026-10-04 23:59 UTC；约 9.1 万星 | GitHub API，2026-10-05 读取 |
| 仓库内版本号 | `version = "0.2.2"`，`publish = true` | 同上 Cargo.toml 第 3、8 行 |
| 官方自述 | “still pre-1.0. There will often be breaking changes between versions. You'll also need to use the latest version of stable Rust.” | `crates/gpui/README.md` 第 8 行 |
| 官方自述 | “Currently, the best way to learn about these APIs is to read the Zed source code or drop a question in the Zed Discord.” | 同上第 98 行 |
| 官网说法 | “for the near future, GPUI is tied to Zed, so contributions will need to be made there and kept in sync with it.” | gpui.rs，2026-10-05 读取（经摘要工具，措辞以官网为准） |
| 框架形态 | “hybrid immediate and retained mode, GPU accelerated”；三层：Entity 状态、声明式 View（类 Tailwind 的 `div()` 链式写法）、命令式 Element | README 第 3、76–82 行 |

Zed 编辑器本身据二手报道已在 2026-04-29 发布 1.0；我没能访问 zed.dev 核实，本文不依赖这一点。**编辑器 1.0 不等于 GPUI 1.0**：上表 README 仍写 pre-1.0。

### 2.2 发布渠道（这是选型的第一道关）

| 渠道 | 事实 | 评价 |
| --- | --- | --- |
| 官方 crates.io `gpui` | 最新 0.2.2，发布于 2025-10-22，之后近一年没有新版；`gpui_platform`、`gpui_macos`、`gpui_wgpu` 等平台拆分 crate **不在** crates.io | 【查证】crates.io 接口。与仓库现状差一年，缺平台拆分，【建议】不用 |
| 官方仓库 git 依赖 | 仓库里 gpui 依赖大量同仓 crate（collections、scheduler、sum_tree、http_client、refineable 等），只能整体从仓库取；根 `Cargo.toml` 第 1014–1022 行有 `[patch.crates-io]`（async-task、async-process、calloop 等），这类补丁不会自动传给下游 | 【查证】。来源最正，但要克隆整个 Zed 仓库；补丁是否需要下游照抄【待验证】。同类项目 okena 用这种方式（清单里没有写固定提交） |
| 第三方快照 `gpui-pre` | crates.io 上 `gpui-pre`、`gpui-pre-platform`、`gpui-pre-macos`、`gpui-pre-macros` 等，描述为“gpui-pre snapshot of zed@<提交>”；2026-09-03 起 0.3.0，到 2026-10-05 的 0.3.8（对应 zed@279fe07），大约每周一版；发布者是 huacnlee（gpui-component 维护者），不是 Zed 官方 | 【查证】crates.io 接口。用起来最省事；信任对象从 Zed 换成了一个第三方发布者，且这个渠道只存在了一个月 |
| 社区分叉 `gpui-ce` | crates.io 最新发布 2026-08-28，近期下载量约为 gpui-pre 的八分之一 | 【查证】。活跃度低，【建议】不考虑 |

破坏性变更的频度有一条直接证据【查证】：gpui-component 的根 `Cargo.toml` 注释写明，它必须用 `=0.3.7` 精确固定快照，因为宽松版本要求曾让应用被带到一个它编译不过的新快照上（其仓库 #3156）。也就是说**相邻两周的快照之间就可能编译不过**。

### 2.3 平台与渲染

【查证】`crates/gpui_platform/Cargo.toml` 与 README：

- macOS：`gpui_macos`，Metal 渲染；字形光栅化需要打开 `font-kit` 特性，否则“lays text out but renders no glyphs”。
- Linux / FreeBSD：`gpui_linux`，`wayland`、`x11` 特性至少开一个；渲染走 `gpui_wgpu`（wgpu 29.0.4，文本用 cosmic-text 0.19）。
- Windows：`gpui_windows`，Win32 窗口、DirectWrite 文本，无需特性。
- Web：`gpui_web`（wasm32，wgpu/WebGL）；Zed 的工具链文件把 `wasm32-unknown-unknown` 注明为“gpui on the web”。
- 仓库里还有 `gpui_apple`（Metal 公共部分）。

对 Saddle 的含义【推测】：用户日常在 macOS arm64 上用（本机 `uname -m` 为 arm64），Saddle 自身又大量使用 Unix 专有能力（`src/pty.rs` 的进程组信号、`src/control.rs` 的 Unix socket），所以桌面版首期只需要 macOS；Linux 有路可走，Windows 不在讨论范围。

### 2.4 工具链与构建依赖

| 事项 | 事实 | 来源 |
| --- | --- | --- |
| Zed 固定的 Rust | 1.98.1 | zed@279fe070 `rust-toolchain.toml` |
| 本机 Rust | rustc 1.96.0（2026-05-25） | 本机 `rustc --version` |
| Saddle 声明 | `rust-version = "1.96"`，edition 2024 | `Cargo.toml:11-12` |
| macOS 构建前提 | 完整 Xcode（不只是命令行工具）、`xcode-select` 指向它；找不到 `metal` 工具时着色器编译失败 | `crates/gpui/README.md` 第 54–72 行；`docs/src/development/macos.md` 第 145–153 行 |
| 本机 Xcode | Xcode 27.0，路径 `/Applications/Xcode.app/Contents/Developer`；macOS 27.0.1 | 本机 `xcodebuild -version`、`sw_vers` |
| 构建脚本 | macOS 下有 bindgen 构建依赖 | `crates/gpui/Cargo.toml` 第 175–176 行 |
| 体量 | gpui 直接依赖含 taffy（布局）、lyon、resvg/usvg、image、accesskit、http_client 等；单个 gpui-pre 包压缩后约 5.3 MB | 同上；crates.io |

【待验证】1.96 能否编过当前 GPUI。两条相反的旁证：herdr-gpui 声明 `rust-version = "1.96"` 且固定 `gpui-pre =0.3.6`；paneflow 声明 1.98.0，Zed 自己用 1.98.1。README 的要求是“最新稳定版”。稳妥的预期是**桌面版要把工具链升到并持续跟随最新稳定版**。

### 2.5 许可

【查证】

- `gpui`、`gpui_platform`、`gpui_macos`、`gpui_wgpu` 等：Apache-2.0。
- Zed 的 `terminal`、`terminal_view`、`ui` 三个 crate：**GPL-3.0-or-later**。
- alacritty_terminal 0.26.0：Apache-2.0；portable-pty 0.9.0：MIT；gpui-component 0.7.0：Apache-2.0。
- Saddle 仓库根目录目前没有 LICENSE 文件，各 `Cargo.toml` 也没有 license 字段。

含义【建议】：上面只是上游许可的事实，本文不对合规下结论。需要分开看两件事：

- **依赖 GPUI**：以库的方式使用 Apache-2.0 的 gpui 及平台 crate。这要遵守 Apache-2.0 自身的条款，GPUI 的传递依赖还各有许可；不同依赖渠道（官方仓库提交、第三方快照）带进来的内容也可能不完全相同。
- **复用 Zed 应用层代码**：`terminal`、`terminal_view`、`ui` 是 GPL-3.0-or-later。把它们的代码并入 Saddle 会带来 GPL 义务，所以【建议】不复制，只参考“怎么做”的思路，用 GPUI 公开接口和 alacritty_terminal 自己实现。

具体的发布合规（Saddle 自己用什么许可、要附带哪些声明、能否分发）留到选定依赖渠道和发布方式后逐项核对。Saddle 将来若要对外发布（T71/T75），许可选择需要先定。

### 2.6 与“应用内终端”直接相关的能力

【查证】均在 zed@279fe070：

- 自定义元素：`Element` trait（`crates/gpui/src/element.rs:53`），三步 `request_layout` / `prepaint` / `paint`，元素完全控制自己的绘制。
- 绘制原语（`crates/gpui/src/window.rs`）：`paint_quad`（4550，背景/光标/选区矩形）、`paint_glyph`（4687）、`paint_image`（4919）、`paint_path`、`paint_underline`、`paint_strikethrough`、`with_content_mask`（裁剪）、`insert_hitbox`（命中区）。
- 文本：`WindowTextSystem::shape_line(text, font_size, runs, force_width)`（`text_system.rs:638`），最后一个参数可把字形强制到固定格宽；还有 `em_advance`、`ch_advance` 用来算格宽。
- 输入法：`Window::handle_input`（`window.rs:5253`）配合 `InputHandler` trait（`platform.rs:2187`，含选中范围、标记文本、候选框位置等回调）；实体级封装 `EntityInputHandler`（`input.rs:13`）。
- 事件：`on_key_event`、`on_mouse_event`；`Keystroke` 同时给出按键名 `key` 与可打出的字符 `key_char`，修饰键含 control/alt/shift/platform(⌘)/function（`platform/keystroke.rs:18、448`）。
- 按键分发：动作（Action）＋按键上下文＋ `bind_keys`，另有 `intercept_keystrokes`、`observe_keystrokes`（`app.rs:2437–2486`）。
- **Zed 自己的终端就是这么做的**：`crates/terminal` 用 alacritty_terminal（Zed 的分叉，固定 git 提交）；`crates/terminal_view/src/terminal_element.rs` 实现 `Element`（1102 行起），把同样式的格子合并成文本批次并用 `shape_line(..., Some(cell_width))` 绘制（163–167），背景用 `paint_quad`（265），方块字符单独按子格矩形画（188–222），输入法用 `TerminalInputHandler`（1771 行起）。这说明“alacritty 网格 → GPUI 元素”的做法在 Zed 里已用于生产；Saddle 自己实现能否达到同样效果仍待验证。该代码是 GPL，建议只参考思路、不复制（§2.5）。

### 2.7 测试、无障碍与平台服务

【查证】

- 测试：`test-support` 特性、`#[gpui::test]` 宏、`TestAppContext` 可模拟平台输入（README 第 96 行；`examples/testing.rs`）。另有 `VisualTestPlatform`：真实渲染＋可控调度，用于截图测试，注释写明“macOs-only for now”（`platform/visual_test.rs`）。
- 无障碍：集成 AccessKit（`crates/gpui/src/_accessibility.rs`，`examples/a11y.rs`）。
- 应用级服务（`crates/gpui/src/app.rs`）：多窗口 `open_window`、原生菜单 `set_menus`、Dock 菜单 `set_dock_menu`、剪贴板读写、`open_url`、文件选择 `prompt_for_paths`、退出钩子 `on_app_quit`；示例里有 `system_notifications.rs`、`drag_drop.rs`、`gif_viewer.rs`、`image.rs`、`svg.rs`、`popover.rs`、`uniform_list.rs`。

### 2.8 生态与同类项目

【查证】GitHub 与 crates.io，2026-10-05 读取。只读了清单文件和仓库描述，**没有读它们的实现代码，也没有运行**。

| 项目 | 说明 | 依赖方式 / 许可 |
| --- | --- | --- |
| gpui-component（仓库 longbridge/gpui-kit） | GPUI 组件库，0.7.0（2026-09-28），约 1.6 万星；组件目录含 dock、resizable、tab、input、list、table、tree、dialog、sheet、sidebar、menu、notification、setting、title_bar、virtual_list、theme 等 | 精确固定 `gpui-pre =0.3.7`；Apache-2.0 |
| contember/okena | GPUI 终端复用器：标签、分屏、可拆窗口、命令面板、工作区恢复。工作区拆成 core / terminal / layout / state / 多个 views / **独立的 okena-tui（crossterm）** / daemon / remote | gpui 取官方仓库 git；alacritty_terminal 0.25＋portable-pty 0.9；MIT |
| penso/herdr-gpui | Herdr 的原生 macOS 客户端（Herdr 是 Saddle 设计时参考过的同类产品），显示终端会话、工作区、worktree、agent 活动 | `gpui-pre =0.3.6`，声明 rust-version 1.96；Apache-2.0 |
| zortax/gpui-terminal | GPUI 终端组件 | gpui 0.2.2＋alacritty_terminal 0.25.1＋portable-pty 0.9；Apache-2.0；最近推送 2026-01 |
| maddada/Ghostex、arthjean/paneflow | 面向编码 agent 的 GPUI 工作台，终端用 Ghostty 内核（libghostty） | MIT / GPL-3.0 |
| libghostty-vt | Ghostty 终端内核的 Rust 绑定，0.2.2（2026-09-28） | MIT OR Apache-2.0 |

读法【推测】：

- “GPUI + alacritty_terminal + portable-pty”是多人走过的组合，与 Saddle 现有选型（`docs/DESIGN.md` §10）一致，换前端不必同时换终端内核。
- okena 的工作区结构（共享核心、GPUI 视图、独立 TUI）说明“一套核心、两个前端”有人在实际维护，可作结构参考。
- 同类产品（Herdr 客户端、Ghostex、paneflow）都在往原生窗口走，方向本身不孤立。

herdr-gpui 的进一步情况（它怎么画终端、与 Saddle 的结构差别）和 Ghostty 的可嵌入现状见 §14.3、§14.5。

### 2.9 与 Saddle 现用库的对照

| 库 | Saddle 现用（`Cargo.toml`、`Cargo.lock`） | 桌面版去向 |
| --- | --- | --- |
| alacritty_terminal | 0.26.0 | 保留 |
| portable-pty | 0.9 | 保留 |
| ratatui | 0.30.2 | TUI 前端保留；桌面版在兼容层期间仍需要（现有页面和插件 SDK 都画到 ratatui Buffer） |
| crossterm | 0.29 | TUI 前端保留；桌面版只借用它的事件类型喂给旧页面 |
| arboard | 3.6.1（剪贴板） | 桌面版可改用 GPUI 剪贴板，TUI 保留 |

---

## 3. 本仓库现状：哪些能带走，哪些要换

### 3.1 规模（`git ls-files` 统计，2026-10-05）

- 全仓 Rust 约 8.71 万行；`src/` 3.17 万行；`tests/` 2.56 万行；`crates/corral-core` 7,329 行；`plugins/drover` 1.59 万行。
- `src/` 里**完全不提** ratatui/crossterm 的文件 36 个、约 1.17 万行；提到的 25 个文件、约 2.00 万行。
- 全仓 `#[test]` 共 717 个；36 个文件用到 `TestBackend` 或 `Buffer::empty`（即断言 ratatui 画面）。

### 3.2 不依赖界面库、预期可复用

这一组是静态判断：这些文件不引用 ratatui/crossterm，所以预期不必为桌面重写。但“不依赖界面库”不等于“接上就能用”——它们现在由 TUI 主循环驱动（每轮 `tick`、以“格”为单位的尺寸、请求进主循环后再改布局等），怎么接到 GPUI 的事件循环、接入时要不要调整接口，都需要适配和验证。

| 模块 | 作用 | 依据 |
| --- | --- | --- |
| `crates/corral-core` | Corral Rust 核心与 `corral` CLI | 全 crate 无界面依赖 |
| `src/corral.rs`、`src/agents.rs` | 调公开 `corral ls/status/...`、轮询、Agents 数据 | 无界面依赖 |
| `src/pty.rs` | PTY 会话：读写线程、尺寸调整、按进程组回收 shell | `src/pty.rs:15-118` |
| `src/viewer.rs` | 单个终端窗格的生命周期：选择 agent、后台 status 核验后 attach、代际防串、退出处理 | `src/viewer.rs:19-33`、`199-224` |
| `src/layout_state.rs` | 布局文件 v1/v2 读写与校验 | `src/layout_state.rs:41-57` |
| `src/control.rs`、`src/control_cli.rs` | `saddle ctl` 的本机 socket、请求记录、CLI | `src/control.rs:1` “Only the UI loop mutates the workspace” |
| `src/telemetry/*`、`src/agent/*`、`src/agent.rs` | 遥测存储/查询/CLI、`saddle agent` 执行采集 | 无界面依赖 |
| `src/updates.rs`、`src/diagnostics.rs`、`src/git.rs`、`src/config.rs` | 版本事实、诊断数据、Git 摘要、配置 | 无界面依赖 |
| `src/plugins/registry.rs`、`resources.rs`、`capture.rs`、`cli.rs`、`core.rs` | 插件登记、资源、无界面命令 | 无界面依赖 |
| `crates/plugin-protocol` | 插件协议类型与校验 | 依赖只有 anyhow/serde/unicode-*（`crates/plugin-protocol/Cargo.toml`） |
| `plugins/drover` 的核心部分、`plugins/dispatch` | 任务规则、数据、通知；分派路由 | 插件进程内，与宿主前端无关 |

### 3.3 小改后复用（需要开“接缝”）

| 模块 | 现在卡在哪里 | 需要的接缝 |
| --- | --- | --- |
| `src/terminal.rs` | `Screen`（alacritty `Term`＋解析器＋终端查询应答）本身通用，但 `render` 直接写 ratatui Buffer、颜色返回 ratatui Color（`src/terminal.rs:110-216`）；`Screen.term` 已是公开字段（`:33`） | 加一个与界面库无关的“可见网格快照”读取方式；TUI 的 `render` 留作一个适配器 |
| `src/input.rs` | `encode_key`、`encode_mouse` 的编码逻辑通用，但参数是 crossterm 的 `KeyEvent`/`MouseEvent` 和 ratatui `Rect`（`src/input.rs:44`、`107-111`） | 中性的按键/鼠标类型，两个前端各自转换 |
| `src/history.rs` | 回看、搜索、选区都建在 alacritty 网格上（`src/history.rs:1-11`），只有按键入口用 crossterm | 同上 |
| `src/terminals.rs` | 标签、分屏树（`Node::Split { vertical, ratio, first, second }`，`:108-116`）、稳定窗格 ID、请求修订号都通用；几何用 ratatui `Rect` 且绘制混在同文件 | 把数据模型与“按格子摆放/绘制”分开 |
| `src/plugins/runtime.rs`、`src/plugins/mod.rs` | 子进程、编解码、限额都通用；最新画面缓存成 `ratatui::buffer::Buffer`（`runtime.rs:21-27`），输入转换用 crossterm 类型（`mod.rs:840-871`） | 兼容层阶段的设想是继续用这份缓存、由桌面把 Buffer 画出来，输入再转回现有类型；是否可行需验证。长期可改成直接缓存协议帧 |
| `src/theme.rs` | 41 个语义色角色（`:275-317`）通用，但值类型是 ratatui `Color`，允许 `default` 和 ANSI 名 | 桌面需要把每个角色解析成确定的 RGB（见 §4.6） |

### 3.4 需要替换或重做

| 模块 | 行数 | 说明 |
| --- | --- | --- |
| `src/app.rs`（含 `app_control.rs`、`app_plugins.rs`、`app_links.rs`） | 1,928＋1,408 | 主循环直接持有 crossterm 终端（`:355`），每 30 ms 轮询事件（`:501`）；`App` 结构体集中了 52 个字段，业务状态和界面状态不分（`:172-233`）；`event()` 一个函数约 540 行（`:1038-1582`）。这是迁移里最难拆的一块 |
| `src/ui.rs`、`src/buttons.rs`、`src/layout.rs` | 1,588＋261＋27 | Agents 3a 面板、标签条、窗格边框按钮、状态栏的绘制；命中区在绘制时登记成格子矩形（`ui.rs:20-29` 的 `Hits`） |
| `src/settings.rs`、`telemetry_view.rs`、`launch.rs`、`launch_edit.rs`、`search.rs`、`attention.rs`、`header_menu.rs`、`placement.rs`、`plugins/ui.rs`、`plugins/palette.rs` | 合计约 9,470 | 各页面：自带状态、绘制、按键处理。好消息是多数已经是“页面对象＋返回 `Outcome` 枚举，由宿主执行”的形状（`settings.rs:183`、`search.rs:43`、`attention.rs:197`、`telemetry_view.rs:40`、`palette.rs:69`、`plugins/ui.rs:44`、`header_menu.rs:12`），静态看这是兼容层整页托管可以利用的接缝，能否干净地从 `App` 里解出来未验证 |
| `src/mascot.rs`、`src/kitty.rs` | 958＋358 | 宠物：字符版和 Kitty 图片协议版。桌面里设想直接画图片，Kitty 探测与传图（`app.rs:152`）预期不再需要 |

### 3.5 测试资产

- 核心、协议、Corral、Drover 业务、遥测的测试与前端无关，预期继续适用；抽离核心时若改了接口，对应测试要随之调整。
- 36 个断言 ratatui 画面的测试文件只对 TUI 有效。桌面版需要另建检查（GPUI 的 `test-support`；截图类目前仅 macOS）。`docs/UI回归.md` 的分层验证思路可以沿用，但要新增桌面分组。
- 同一批合成数据和假 `corral` 脚本可以两边共用（AGENTS 的“测试不依赖真实 agent”不变）。

---

## 4. 专题设计

### 4.1 应用内交互终端与 PTY

**复用目标【建议】**：`pty::Session`、`terminal::Screen`、`viewer::Viewer` 三层继续承担现在的职责，不换库、不重写逻辑。这是目标，不是“原样可用”的结论，已知至少有三处要适配：

- `Screen` 要先加一个与界面库无关的网格读取方式（§3.3），现在的 `render` 只能写 ratatui Buffer；
- `Viewer` 现在靠主循环每轮 `tick` 推进（`src/viewer.rs:132-135`），尺寸以“格”传入；接到 GPUI 后由谁、在什么时机推进，需要重新安排；
- 输入编码要先换成中性类型（§3.3）。

右侧窗格的设想不变：仍在 PTY 里跑公开的 `corral attach <名字>`（`src/viewer.rs:194`），普通终端仍是 `$SHELL -i`。**目标是保持 agent 的托管方式、attach 语义、“关闭显示不停止 agent”这些已确认的行为**（DESIGN §27、§39）；桌面实现是否真的保持住，要靠原型和实施时验证。

**新写什么**：一个 GPUI 终端元素，职责只有四件：

1. 量格子：按所选等宽字体和字号算出格宽、行高；窗格像素尺寸除以格子得到行列数，调用 `Session::resize`。现在 PTY 的像素尺寸填的是 0（`src/pty.rs:216-217`），终端尺寸查询回的格子像素也是 0（`src/terminal.rs:58-59`），桌面版应填真实值。
2. 画格子：每帧锁住 `Screen` 取可见网格，把同样式的连续格子合成批次，用强制格宽的 `shape_line` 画字，`paint_quad` 画背景、光标、选区。
3. 收输入：按键经中性类型进 `encode_key`；鼠标像素坐标换算成格子后进 `encode_mouse`；粘贴进 `encode_paste`；输入法见下。
4. 被唤醒：现在是主循环每 30 ms 轮询；桌面版应由 PTY 读线程通知前台任务触发重绘，并做合并，避免高输出时每个数据块重绘一次。

**桌面比 TUI 多出来、必须自己负责的事**（TUI 里这些由 Ghostty 等外层终端代劳）：

| 事项 | 说明 | 状态 |
| --- | --- | --- |
| 中文输入法 | 用户用中文与 agent 交流。必须实现 `InputHandler`：组字中的文本画在光标处、候选框位置跟随光标、上屏后把文字送进 PTY | 【待验证】原型第一优先级 |
| 中文、表情与符号对齐 | 终端按“格”排版，字体的实际字宽未必等于 1 格或 2 格；回退字体（如 PingFang）下尤其如此。需要按格定位而不是信任字体步进 | 【待验证】 |
| 制表符/方块字符 | Claude Code 界面、Agents 树线、字符版宠物大量使用。字体自带的这些字形常有缝隙；Zed 的做法是把方块字符按子格矩形自己画 | 【待验证】是否需要同样处理 |
| Shift+Enter 等修饰键 | TUI 靠向外层终端申请 Kitty 键盘增强（DESIGN §5 T52）；桌面里系统直接给出修饰键，现有 `CSI 13;2u` 编码（`src/input.rs:53-55`）预期可以沿用 | 【推测】可能比 TUI 简单，未验证 |
| Option 键 | macOS 上 Option+字母会打出特殊字符；终端程序通常期望它当 Alt/Meta。需要定一个规则（终端类应用通常提供“Option 当 Meta”的开关【推测】） | 【待决定】 |
| 高输出吞吐 | 一屏 200×60 约 1.2 万格。批次合并和 GPUI 的整形缓存应该够用，但需要实测输入不卡 | 【待验证】 |
| 鼠标上报 | Claude Code 等会开鼠标模式；像素到格子的换算、拖动、滚轮累积都要对 | 【待验证】 |

**顺带得到的改进【推测】**：平滑滚动回看、不进“History 模式”就能拖选复制、⌘C/⌘V 不与终端程序抢键、可点击链接。这些都改变现有已确认的交互（DESIGN §44），**做不做要逐项问用户**，首版应先保持与现在一致。

**关于换成 Ghostty 内核**：libghostty-vt 已有 Rust 绑定，Ghostex、paneflow 在用。它在图像协议、键盘协议上更完整。但换内核会让 `history.rs`、`terminal.rs`、`input.rs` 的既有逻辑全部作废，与“换前端”叠加风险太大。【建议】桌面化期间不换，另案评估。后续讨论中用户表示 Zed 内置终端（同样是 alacritty_terminal 加自绘）的水平能满足要求，这条建议维持；Ghostty 嵌入的现状见 §14.3、§14.4。

### 4.2 窗口、分屏、焦点与输入

**分屏与标签**：数据模型已经是带比例的二叉树并持久化（`src/terminals.rs:108-116`；DESIGN §45 保存“tab、分屏方向与比例”）。设想是在 GPUI 里把这棵树递归渲染成横/纵向弹性布局；窗格的稳定 ID、请求修订号、移动整个窗格等规则（DESIGN §29、§39）属于数据层，目标是保持不变，但数据与绘制目前混在同一文件（§3.3），要先分开。分隔条拖动在桌面里很自然，但现有设计明确“不增加比例编辑”，属于新增行为，需用户点头。

**焦点**：现在是自己维护的 `Focus` 枚举加一串“是否有弹层”的判断（`src/input.rs:3-42`、`src/app.rs:417-420`）。GPUI 有每个可聚焦元素一个 `FocusHandle`、按键上下文和动作绑定。映射方式【建议】：

- 宿主保留键（现在的 `Ctrl-]`，插件协议里的 `reserved_keys`）绑定为根上下文的动作；
- 终端窗格聚焦时，其余按键全部交给终端元素；
- 弹层打开时由弹层持有焦点，天然不透传。

【待验证】保留键是否确实先于终端元素的按键处理被截获（GPUI 有 `intercept_keystrokes`，但行为需要实测）。

**桌面带来的输入变化【推测】**：⌘ 系快捷键（⌘T 新标签、⌘D 分屏、⌘W 关闭、⌘, 设置、⌘K 搜索……）不会和终端里的程序冲突，这是 TUI 做不到的。但 UI 设计语言 §2 写明“不授权增加快捷键”，所以**任何新快捷键都要单独批准**；首版只需保证现有键位照常工作。

**窗口级能力**：多窗口、原生菜单、Dock 菜单、系统通知在 GPUI 里都有接口（§2.7）。Attention 待处理数做成 Dock 徽标、把标签拆成独立窗口，都是合理的后续项，不属于首版。

**文本输入框**：新建 agent、设置、搜索都需要带光标、选择、输入法的输入框。GPUI 不自带现成输入框控件，官方的文本输入示例 `examples/input.rs` 就有 784 行。gpui-component 有 `input`。【建议】原型阶段评估是否采用 gpui-component：好处是省掉输入框、列表、对话框等大量基础件；代价是多一个 0.x、每周发版、并且**强制 GPUI 版本与它一致**的依赖。兼容层阶段旧页面走字符面，可以先不需要原生输入框。

### 4.3 进程插件：字符帧兼容与桌面展示

**契约层面的事实【查证】**：

- 协议 crate 不依赖 ratatui（`crates/plugin-protocol/Cargo.toml`）。
- 画面是“每行若干同样式文字片段”，颜色是 `default` / 索引色 / RGB，宽度规则固定为 `saddle-grapheme-v1`（`docs/插件协议.md` §2、§3）。
- 输入是结构化的按键、格子坐标的鼠标、整段粘贴（同上 §4）。
- 尺寸以“列×行”下发（`panel.open/resize`）。

这些事实只说明协议没有绑定某个界面库，由此得出“桌面宿主有条件承载现有插件”是静态可行性判断【推测】。**不能据此认为现有插件已经兼容桌面。**现在的宿主侧实现是为 TUI 写的：画面解码后存成 ratatui Buffer，输入从 crossterm 事件转换（§3.3），进程启停、尺寸修订、焦点、覆盖层、通知与 Attention、`command.v1` 转发都挂在 TUI 主循环上。桌面宿主要把绘制、输入和这些生命周期环节全部重新接入，并逐项验证。

**桌面展示方案【建议】——“字符面”**：用和终端元素同一个网格绘制器，把插件帧画在一块等宽网格上。宿主按“可用像素 ÷ 格子大小”算出列×行下发给插件；鼠标像素换算成格子坐标；按键按协议的命名键/字符键映射；输入法上屏的文字按逐字符按键送出（与现在外层终端的行为一致）；`panel.cursor.v1` 的光标画在对应格子。

这样做想达到的结果（均未验证）：

- **目标**是 Drover（Tasks）、Diff、Counter、Attention demo 的插件程序本身不改，适配全部落在宿主一侧；工作区窗格和居中覆盖层两种位置都要各自接入并验证。如果验证中发现某些行为依赖外层终端的特性，插件或协议仍可能需要调整。
- 插件 SDK 继续用 ratatui 在插件进程里画（`crates/plugin-sdk/Cargo.toml`），预期不受宿主换前端影响。
- 预期观感是“原生窗口里嵌了一块字符界面”，和旁边的终端窗格风格接近，不会像原生控件那样精致。

需要定的细节：

| 事项 | 说明 |
| --- | --- |
| 颜色 | `default` 前景/背景映射到主题的正文/背景色；索引色 0–255 用一张固定调色板（`src/terminal.rs:74-107` 已有同样的表）；RGB 原样。五个主题语义色（text/muted/background/accent/error）下发确定的 RGB |
| 覆盖层尺寸 | 现为“全窗口宽高各 80% 居中”（DESIGN §62 Drover 完整替换）。桌面可沿用比例，换算成格子数 |
| 滚轮 | 像素滚动量累积成整数行再发 |
| 字形越格 | 宽度规则保证“协议上”每个字素占 1 或 2 格，但实际字体里个别表情/符号可能画出格外，需要裁剪或缩放【待验证】 |

**以后要不要给插件“原生界面”能力**：可以想象新增一种结构化视图能力，让插件声明原生列表/表单。但 DESIGN §62 明确“不预造第二运行后端”，UI 设计语言 §3.5 也说公共 UI SDK 属于后续独立工作。【建议】现在不设计，等桌面版稳定、确有插件觉得字符面不够用时再议。

**一个对成本很关键的推论【建议】**：如果字符面验证通过，同一份插件画面两个前端都能承载，那么**做成插件的功能有望不必为 TUI 和桌面各写一遍**。要控制“双前端”的重复成本，就让宿主原生界面只负责外壳（窗口、侧栏、标签/分屏、终端、设置入口），业务页面尽量留在或放进插件——这与 §62“Saddle 是操作台，业务在插件”的方向一致。

### 4.4 Corral / Drover / 遥测 / ctl 的公开边界

本方案的设计意图是换前端**不改变**已确认的边界。下表“桌面版”一列写的是要保持的目标，不是已验证的结果；做不做得到取决于桌面宿主的接入实现。

| 边界 | 现状依据 | 桌面版（目标） |
| --- | --- | --- |
| Corral 只经公开命令访问 | AGENTS 规矩；`src/corral.rs` | 不变，同一份客户端代码 |
| 不干扰用户的 agent | AGENTS 规矩 | 不变；原型需要真实 agent 时自开 `saddle/test-*` |
| Drover 业务全在插件，宿主不读业务文件 | DESIGN §62 Drover 完整替换 | 不变；桌面宿主同样只转发 |
| 主控任务操作走 `saddle ctl plugin` | `plugins/drover/README.md` “主控命令” | 桌面实例必须提供同一套 ctl 服务（`instances/inspect/open/request/close/plugin`），否则现有 skill 和主控流程在桌面下失效 |
| 遥测归宿主核心，独立开关 | DESIGN “Saddle 核心遥测” | 不变；存储、查询、CLI 都与界面无关 |
| 插件拿到 `SADDLE_HOST_BIN`、`SADDLE_AGENT_BIN` | `docs/插件协议.md` §11、§19 | 见下 |

需要处理的四个具体问题：

1. **命令行入口必须随桌面包一起存在。**`SADDLE_HOST_BIN` 的约定是“当前宿主可执行文件”，插件会用它调 `saddle telemetry` / `saddle agent`（Drover 的记录流程依赖它）。`src/main.rs:6-19` 现在是一个多用途二进制（agent / plugin / telemetry / ctl / 界面）。【建议】桌面包里仍带这个 `saddle` 命令行程序，桌面进程把 `SADDLE_HOST_BIN` 指向它；默认 Corral 仍按“宿主同目录”解析（DESIGN “Corral Rust 核心实施”）。
2. **两个实例并存时的 Drover 持有权。**【查证】“每个用户的数据只允许一个 Drover 插件进程持有，第二个实例会明确失败”（`plugins/drover/README.md` 第 15 行）。所以桌面版和 TUI（比如手机 SSH 进来开的那个）同时开着时，只有一个能用 Tasks。这是现有规则的直接后果，不是新缺陷，但会在“保留 TUI 作远程入口”时变成真实场景。处理方式属于【待决定】（§11）。
3. **布局文件。**两个前端默认都读写 `~/.local/state/saddle/layout.json`（DESIGN §45）。轮流用时共享布局是优点；同时开会互相覆盖。是否给桌面单独的布局文件【待决定】。
4. **从 Dock/访达启动时的环境变量。**【推测】macOS 图形程序不继承登录 shell 的 PATH 等环境；而 `corral start` 最终要找到 `claude`、`codex`，普通终端窗格也期望用户的 shell 环境。Corral 本体按“宿主同目录”找不受影响，但 agent 程序的查找会受影响。常见做法是启动时跑一次登录 shell 取环境，或约定从命令行启动。【待验证】实际表现并选定做法。

`saddle ctl` 的“布局只由主循环写入、socket 不阻塞界面”（`src/control.rs:1`；DESIGN §39）在 GPUI 下对应为：socket 线程把请求投递到前台执行器，由前台唯一修改工作区状态。静态看两边模型一致，预期不需要改 ctl 协议；实际接入未验证。

### 4.5 TUI 与 SSH 路径

事实【查证】：

- 设计文档多处把“真实终端/手机 SSH”列为验收维度（`docs/DESIGN.md:1592`、`:1602`）；待办里有 T60 移动端适配、T32 远程 agents。
- 10-04 的排序文档已经指出“GPUI 桌面不自动解决手机 SSH/手机端”。
- GPUI 窗口是本机图形程序，不能经 SSH 使用。

三种处置：

| 做法 | 得到 | 失去 / 代价 |
| --- | --- | --- |
| 退役 TUI | 只维护一个前端 | 失去 SSH/手机入口；T60/T32 需要另找路（独立移动客户端、远程服务等，范围很大） |
| **保留 TUI，与桌面共用核心【建议】** | SSH/手机入口继续可用；桌面出问题时有退路 | 外壳层界面要维护两份；新宿主页面要么做两遍，要么做成插件（设想由字符面两边承载，§4.3） |
| 桌面窗口里嵌一个终端来跑现有 TUI | 可能最快得到一个“应用图标” | 桌面收益很少，等于自带一个终端模拟器；【推测】与用户原话“更加接近桌面”的方向不符 |

【建议】保留 TUI，并明确它的定位是“远程/SSH 入口”，功能以够用为准；桌面版决定为主入口之后，新的外壳类投入优先桌面，TUI 只同步核心行为和插件。是否冻结 TUI 的界面投入由用户在第二个闸门决定（§6）。

关于 `gpui_web`【推测】：GPUI 能编到浏览器，理论上将来可以给远程/手机一个网页前端，但 PTY 和 Corral 都在主机上，必须先有一层远程服务，这已经是 T32 的范围。近期不能当作手机方案。

### 4.6 设计语言在桌面的落点

`docs/UI设计语言.md` v1 的规则分两类：

- **与媒介无关，直接沿用**：语义角色配色（§3.1）；焦点、选中、主操作三者不靠同一种颜色区分（§3.1、§3.6）；成功/失败/进行中/未知/陈旧必须有文字或符号（§3.1）；按类型统一组件而不是全部同形（§3.2）；文案与空态规则（§3.4）；插件只有五个主题色、不扩协议（§3.5）。
- **TUI 的具体画法，桌面需要重新定**：以“列”为单位的宽度与对齐（§3.3）、108×34 这类弹窗尺寸、尖括号紧凑按钮、三行圆角 Tab、Agents 3a 的字符格布局。

桌面需要新定的东西（都属于“新外观”，按设计语言 §4 应该先出代表性画面给用户确认）：

| 事项 | 说明 |
| --- | --- |
| 主题色解析 | `src/theme.rs` 的 41 个角色允许 `default` 和 ANSI 色名，值的含义依赖外层终端。桌面没有“终端默认色”，四个预置主题里 Dune、Tide、Lagoon 可以给出确定 RGB，“Terminal”预置（跟随终端）需要重新定义——比如改成跟随系统浅色/深色 |
| 字体 | 终端与字符面必须等宽；侧栏、标题、按钮可以用系统界面字体（比例字体），这是桌面相对 TUI 最直观的观感提升。中文回退字体、是否随应用带字体需要定 |
| 间距与尺寸 | 从“列/行”换成像素；窄窗退让规则（DESIGN “UI 收尾：窄栏信息退让”）的思路可沿用 |
| 宠物 | 仓库已有图片资源（`assets/pets`）。桌面可以始终显示图片版，不再依赖终端是否支持 Kitty 图片 |
| 兼容层页面 | 走字符面的旧页面保持现有画法，不需要重新设计；逐页原生化时再各自出画面 |

### 4.7 打包、安装与版本事实

- 现状【查证】：`scripts/package.sh` 产出不可变成套目录（`bin/saddle`、`bin/corral`、`plugins/*/bin/*`）和带各程序 SHA-256 的 `BUILD.txt`；Settings → Updates 依赖这份记录判断源码/已安装/运行中的版本关系（DESIGN “T67 版本事实展示”）。
- 桌面版需要一个 `.app` 包。【建议】把 `.app` 作为同一成套目录里的又一个产物，包内仍放命令行 `saddle`、`corral` 和插件，`BUILD.txt` 的约定延续，这样 Updates 的版本事实逻辑预期可以沿用（需核对 `.app` 内的路径解析）。
- 签名与公证【推测】：自己机器上本地构建自用，通常不需要 Apple 开发者签名；要分发给别人（T71/T75）才需要签名和公证。
- 按用户既有偏好，仓库工具保持 Rust/shell，不引入 Python 脚本。
- “Saddle 待重开”的提示逻辑目标是保持；桌面版同样应做到“关掉重开界面，agent 继续跑”，需实施时验证。

---

## 5. 路线比较与推荐

基线和三条有实质差别的路线：

| 路线 | 做法 | 优点 | 缺点 |
| --- | --- | --- | --- |
| A. 维持 TUI | 不做桌面版 | 没有迁移成本；SSH 可用；现有 717 个测试继续适用 | 继续受外层终端限制：比例字体、图片、输入法位置、快捷键冲突、窗口级能力都拿不到；每个新特性都要适配不同终端 |
| B. 全量原生重写 | 所有界面用 GPUI 重写，完成后切换 | 终态最干净 | 约 2 万行界面相关代码一次性重做；切换前桌面版一直不完整，无法日常使用，也就得不到真实反馈；与“核心稳定”原则（DESIGN §50）冲突最大 |
| C. 整窗字符面起步 | 先把现有整套 TUI 画面画进 GPUI 窗口，再逐块换成原生 | 设想中最早接近功能对齐、之后每一步都保持可用。这是预期，未验证：整窗的输入回送、输入法、图片宠物、主循环驱动方式都要先适配 | 早期看起来和 TUI 几乎一样，体现不出桌面价值；容易停在半成品状态 |
| **D. 原生外壳＋字符面兼容层【建议】** | 始终可见的部分（窗口、Agents 侧栏、标签/分屏、终端窗格）原生重做；设置、遥测、新建 agent、搜索、Attention、插件面板等现有页面与插件先走字符面，之后按价值逐页原生化 | 目标是较早有桌面观感，同时靠字符面补齐其余功能；旧页面约 9 千多行预期可以少改；字符面绘制器与终端元素设想共用一套。这些都是静态判断，功能能否补齐、代码能省多少要等原型和 P1 才知道 | 需要先把页面从 `App` 主循环里解出来（§3.4），工作量未知；一段时间内原生与字符面并存，观感不完全统一；桌面构建期间仍带着 ratatui/crossterm |

倾向 D 的理由（都是建议层面的判断）：

1. Agents 侧栏、真实终端、基础分屏是此前讨论中建议原型验证的三样——任务书把它们列为研究方向，不是用户新增的验收，原型范围仍待用户决定。它们正好是“始终可见的外壳”，【推测】最能决定像不像桌面应用，所以建议原生做。
2. 其余页面是偶尔打开的弹层，先用字符面承载，预期观感损失较小、节省较多。
3. 现有页面多数是“页面对象＋`Outcome`”的形状（§3.4），静态看具备整页托管的接缝。
4. 插件要靠字符面，绘制器无论如何要做；【推测】再多托管几个旧页面的边际成本较低。
5. 每一页原生化可以拆成独立、可验收的小任务，符合本项目“拆任务、逐个审查”的工作方式。

C 留作退路【建议】：如果把页面从 `App` 里解出来比预期难，可以先整窗字符面托底，再换外壳。

本节不展开比较其他界面框架：任务问的是 GPUI；同类产品也集中在 GPUI 上（§2.8）。真正的备选是“不做”（A），已列为基线。用户后来问到 GPUI 之外的路线，作者凭已有了解给了一张未查证的对照表，见 §14.2。

---

## 6. 分阶段建议

每个阶段结束都有一个由用户决定的闸门；前一个闸门没过，后面的不启动。规模是相对估计，不是工期承诺。

**P0 可行性原型（小，可丢弃）**——详见 §7。产出：能运行的原型、一份实测记录、对下面各阶段估计的修正。**不合并进 main。**

**闸门一（用户）**：继续 / 停在 TUI / 改方向；同时定依赖渠道、TUI 去留、是否采用 gpui-component（§11）。

**P1 按需抽离核心（中）**。原则：只开 P0 证明必需的接缝，不做预先的大重构；每一步都是“TUI 行为不变”的重构，靠现有测试把关，逐步落到 main。

- 中性的按键/鼠标类型（`input.rs`、`history.rs`）。
- `Screen` 的网格快照读取（`terminal.rs`）。
- 分屏树数据与格子几何/绘制分开（`terminals.rs`）。
- 把 `App` 里与界面无关的工作区状态（终端集合、后台动作、ctl 服务、插件管理、轮询、更新检查）与 TUI 呈现状态分开，使第二个前端能驱动同一套状态。

这一步对 TUI 自身也有好处：`app.rs` 的单体主循环本来就是后续功能的负担。

**P2 桌面最小闭环（大，仅 macOS）**。目标：用户能只用桌面版完成一整天的分派工作。

- 原生：窗口与标题栏、Agents 侧栏（3a 的信息与语义：状态、计时、分组、已接入/未读、固定的 Tasks 入口、`⋯` 菜单）、标签与分屏、终端窗格（attach 与普通 shell、历史/搜索/复制与现状一致）、底部状态提示。
- 字符面：全部进程插件（工作区窗格与覆盖层）；Settings 家族（General/Colors/Advanced/Diagnostics/Plugins/Updates）、Telemetry、New agent、Search、Attention、插件面板、关闭确认。
- 行为对齐：`saddle ctl` 全套、布局恢复、更新提示、插件通知与 Attention 来源。
- 打包：`.app` 与命令行同包。
- 验证：核心与插件沿用现有检查；桌面新增一组检查；TUI 全量检查保持通过。

**闸门二（用户）**：桌面版是否成为日常主入口；TUI 的定位（继续同步 / 只维护 / 冻结界面）。

**P3 逐页原生化（开放式，按价值排队）**。每页一个任务、先出代表性画面。候选顺序【建议】：新建 agent 表单 → 搜索/统一入口 → Attention → Settings → Telemetry 阅读页。同期可做的桌面专属增强（都需单独批准）：分隔条拖动、Dock 徽标与系统通知、原生菜单与 ⌘ 快捷键、图片宠物、标签拆成独立窗口。

**P4 远期（不承诺）**：插件原生界面能力、Linux 构建、与 T32/T60 相关的远程与移动方案。

---

## 7. 最小原型建议（P0）

**目的**：用最少的代码回答“值不值得继续”，并把 §12 的未验证项尽量变成事实。原型是用来丢的，不追求结构。

**放在哪里【建议】**：仓库外或像 `examples/counter-plugin` 那样被工作区 `exclude` 的独立目录，自带 `Cargo.lock`。这样不会把几百个 GPUI 依赖写进主工作区的锁文件，也不影响 `cargo test --all-targets`。Saddle 的 `lib.rs` 已经公开了 `pty`、`terminal`、`viewer`、`corral`、`plugins` 等模块（`src/lib.rs:1-35`），**设想是原型按路径依赖复用它们而不改现有源码**；这一点本身要在原型里确认——如果发现不改源码就接不上，应先报告缺口，而不是直接改 main。

**范围（建议，待用户决定；三条可并行的轨，互不改同一批代码，与 10-04 排序文档“可并行探索桌面界面、终端/PTY 复用和插件兼容边界”一致）**：

| 轨 | 做什么 | 要观察的结果 |
| --- | --- | --- |
| 甲：终端 | GPUI 窗口里一个终端窗格，复用 `pty::Session`＋`terminal::Screen`；跑普通 shell 和一个自开的 `saddle/test-*` agent | 打字、中文输入法组字与上屏、Shift+Enter、Ctrl-C、Claude Code 的鼠标操作、缩放窗口后重排、256 色/真彩色、中文与制表符对齐、大量输出时按键不卡、回看与复制 |
| 乙：外壳 | 原生 Agents 侧栏（用 `corral::Poller` 的真实只读数据或假 `corral` 脚本）、两个窗格的左右/上下分屏、点击切换焦点、`Ctrl-]` 回侧栏 | 3a 的信息层次在比例字体下怎么摆；保留键是否先于终端被截获；每个窗格各自调整 PTY 尺寸；可顺带试 gpui-component 的输入框与列表 |
| 丙：字符面 | 用现有 `plugins::runtime::Runtime` 启动 Counter 示例插件（或 Drover，指向临时 HOME 的合成数据），把它的画面画进一块网格并完成一次点击和一次按键 | 同一个网格绘制器能否同时服务终端与插件；格子坐标换算；颜色映射 |

**同时记录**：

- 选定依赖渠道后，首次完整构建耗时、增量构建耗时、产物体积、新增依赖数量；
- 本机 Rust 1.96 能否编过，不能则需要升到哪一版；
- 从访达/Dock 启动与从命令行启动时，`corral` 和 agent 程序能否找到；
- 遇到的 GPUI 文档缺口与踩坑。

**不做**：设置、任务、遥测等页面；布局持久化；`saddle ctl`；打包签名；与 TUI 共用状态的重构；任何对 main 的改动。

**遵守现有规矩**：不碰用户正在用的 agent；需要真实 agent 时自开 `saddle/test-<名字>` 并在用完后 `corral stop`；不按名字或路径批量杀进程；插件用临时 HOME 和合成数据，不碰真实任务队列。

**建议的判断标准【建议，供用户取舍】**：

- 继续：甲轨的输入法、对齐、吞吐三项都达到“不比在 Ghostty 里差”；乙轨的保留键与分屏行为正确；丙轨画面与交互正确。
- 存疑：输入法或对齐需要大量绕路才能做对——评估成本后再定。
- 停止：GPUI 在本机工具链上构建不稳定，或输入法无法达到可用。

**规模【推测】**：三条轨各自是“几天量级”的单个 agent 任务；主要不确定性在输入法和字形对齐。

**讨论后的调整【建议】**：作者现在倾向先只做甲轨，以“不比 Zed 内置终端差”为验收参照，并参考 herdr-gpui 的字符格绘制；理由和对照样例见 §14.4–§14.6。

---

## 8. 维护与迁移成本

| 成本项 | 说明 | 性质 |
| --- | --- | --- |
| 跟随 GPUI | pre-1.0，相邻周快照可能不兼容；必须精确固定版本，按自己的节奏（例如每月一次）专门安排升级任务，每次都可能要改代码 | 持续 |
| 跟随 Rust | README 要求最新稳定版；Zed 当前 1.98.1，本机 1.96.0。桌面版会把整个仓库的实际构建工具链往前带 | 持续 |
| 构建时间与体积 | 依赖树明显变大，共享编译目录 `.target` 会增长，首次构建会慢；【建议】桌面 crate 不进 `default-members`，让日常 TUI/核心检查不受影响 | 一次性＋持续 |
| 双前端 | 外壳层（侧栏、标签/分屏、终端）两份实现；靠“业务进插件、字符面通用”把重复面压到最小 | 持续 |
| 测试 | TUI 画面断言对桌面无效，要新建桌面检查；截图类测试目前只有 macOS | 一次性＋持续 |
| 文档缺口 | 官方自己说学习 GPUI 的最好办法是读 Zed 源码；而 Zed 应用层代码是 GPL，建议只参考思路、不复制（§2.5） | 持续 |
| 自担的平台细节 | 输入法、字体回退、环境变量、签名公证、系统版本兼容——原来由外层终端承担 | 一次性＋持续 |
| 迁移期重构 | `app.rs` 主循环拆分是硬骨头；做得好对 TUI 也有益 | 一次性 |
| “成熟库”规则 | AGENTS 要求“只用成熟、活跃维护的库”。GPUI 活跃且在 Zed 里经受生产检验，但接口不稳定、官方发布渠道陈旧。是否算“成熟”由用户裁定 | 决策 |

节省的部分【推测】：不再需要适配不同外层终端的能力差异——Kitty 键盘协议协商（DESIGN §5 T52）、真彩色探测与 256 色降级（`src/app.rs:148`、`169-171`）、Kitty 图片探测（`src/app.rs:152`）、各终端的鼠标与粘贴差异。

---

## 9. 风险清单

| 风险 | 可能性 | 影响 | 应对 |
| --- | --- | --- | --- |
| 中文输入法做不顺 | 中 | 高（日常必用） | P0 第一优先；Zed 有可参考的做法 |
| GPUI 快照升级频繁破坏编译 | 高 | 中 | 精确固定；升级单独成任务；不追每周 |
| 第三方快照渠道中断或不可信 | 低–中 | 中 | 可改为固定官方仓库提交；【推测】应用代码基本不用变，未验证 |
| 本机工具链/新系统上构建失败 | 低–中 | 高 | P0 第一步就验证 |
| `App` 拆分牵动 TUI 回归 | 中 | 中 | 小步、行为不变、现有测试把关 |
| 长期停在“半原生”状态 | 中 | 中 | 闸门二明确主入口；P3 每页独立排队 |
| 双实例争用 Drover 与布局文件 | 高（一旦保留 TUI） | 中 | §11 第 6 问先定规则 |
| 字符面里字形与格子不齐 | 中 | 低–中 | 按格定位、裁剪；P0 验证 |
| 从 Dock 启动找不到 agent 程序 | 中 | 中 | P0 验证并选定取环境的做法 |
| 性能不及外层终端 | 低 | 中 | P0 实测高输出场景 |

---

## 10. 对现有待办的影响

- T49、T64、T62、T56：与前端无关，不是 T76 的前置，照常推进（任务书已写明）。
- T74 Tasks 入口、T63 统一搜索：已在 TUI 落地（DESIGN 2026-10-04 两节）。桌面里它们属于外壳与入口，P2 需要对齐行为，原生化时可重新设计呈现。
- T61 插件开发文档、T70 最小插件示例：【建议】不必等桌面版。本方案的设想是字符帧契约不变（§4.3），可以按现有协议写，并在文档里注明“画面是字符网格，宿主可能是终端也可能是桌面窗口”。前提是用户采纳字符面方向且原型验证通过；若验证后需要调整协议，文档再跟着改。
- T60 移动端：依赖“保留 TUI”这个决定；桌面版本身不解决手机。
- T32 远程 agents：与前端独立；若将来做远程服务，可同时服务 TUI、桌面乃至网页。
- T66 UI 回归样例：现有样例属于 TUI；合成数据可共用，桌面需新增分组。
- T75 产品化、T71 宣传页与视频：安装形态、演示素材都受桌面决定影响，【建议】等闸门二之后再定稿。

---

## 11. 待用户决定的问题

1. **是否启动 P0 原型？**范围按 §7 三条轨，还是只做甲轨（终端）先看最大风险？原型是否确定为可丢弃、不合并？（作者讨论后的意见是先只做甲轨，见 §14.6。）
2. **GPUI 依赖渠道**：固定官方仓库提交（来源最正，取用较重）／第三方每周快照 gpui-pre（省事，信任第三方发布者）／官方 crates.io 0.2.2（陈旧，不建议）。
3. **“只用成熟、活跃维护的库”如何适用于 GPUI**：接受 pre-1.0 作为桌面前端的基础，还是等官方给出稳定版信号？
4. **是否接受工具链持续跟随最新稳定版 Rust**（当前至少可能要到 1.98.x）？
5. **TUI 的去留**：保留为 SSH/手机入口（建议）／只维护不再加界面功能／退役。
6. **桌面与 TUI 同时运行时的规则**：Tasks 只允许先启动的那个使用（现有锁的自然结果）／后者明确提示并只读／另行设计；布局文件共用还是分开。
7. **迁移路线**：D（原生外壳＋字符面兼容层，建议）／C（整窗字符面起步）／B（全量重写）。
8. **是否采用 gpui-component**（省基础控件，换来一个强绑定 GPUI 版本的 0.x 依赖），还是只用 GPUI 自己写需要的少量控件？
9. **平台范围**：首期只做 macOS 是否可以？
10. **外观决定**：终端与界面字体、“Terminal”预置主题在桌面的含义（是否跟随系统浅/深色）、是否允许桌面专属的 ⌘ 快捷键与分隔条拖动——这些都属于新外观/新交互，需要逐项确认或授权先出代表性画面。
11. **许可与发布**：Saddle 将来是否开源、用什么许可（决定能否直接借用 GPL 代码，也影响 T71/T75）；应用名称、标识与是否需要签名公证。
12. **闸门一之前，TUI 的界面类新投入是否暂缓**（避免做完再迁），还是照常？

---

## 12. 未验证项清单

以下全部没有运行证据，原型或实施时需要逐项确认：

- 当前 GPUI（任一渠道）能否在本机 macOS 27.0.1 / Xcode 27.0 / Rust 1.96.0 上构建；需要的最低 Rust 版本。
- 固定官方仓库提交时，Zed 根清单里的 `[patch.crates-io]` 是否需要下游照抄。
- gpui-pre 快照是否与对应官方提交逐字一致、是否包含 `test-support` 等特性。
- 终端元素里中文输入法的完整流程（组字、候选框位置、上屏）。
- 中文、表情、制表符、方块字符在等宽网格里的对齐与缝隙。
- 保留键（`Ctrl-]`）相对终端元素按键处理的优先级。
- 高输出场景的帧率与输入延迟；多窗格同时输出时的表现。
- 鼠标上报、滚轮累积、拖选在像素→格子换算下的正确性。
- 从访达/Dock 启动时的环境变量与 agent 程序查找。
- `pty::Session`、`terminal::Screen`、`viewer::Viewer` 等“复用目标”能否不改接口就接到 GPUI 事件循环；原型能否只靠路径依赖、不改现有源码。
- 现有进程插件（Drover、Diff、Counter、Attention demo）在桌面宿主下是否真的不用改：绘制、按键/鼠标/粘贴/输入法映射、尺寸修订、焦点、覆盖层、通知与 Attention、`command.v1` 转发逐项都没有验证。
- 桌面宿主能否原样保持 Corral、Drover、遥测、`saddle ctl` 的公开边界和已确认行为。
- 路线 C、D 设想的功能对齐程度和能省下的代码量。
- `ratatui` 离屏绘制旧页面再画进字符面的实际效果与输入回送（现有测试用 `TestBackend` 离屏绘制，只说明离屏绘制这一步有先例；交互未验证）。
- `App` 主循环拆分的真实工作量。
- 首次/增量构建时间、二进制体积、对共享编译目录的影响。
- Zed 1.0 的发布日期（仅二手来源）。
- 本文引用的同类项目只读了清单与描述，其质量、完成度未评估。
- 提交后讨论新增的未验证项（herdr-gpui 的中文输入法、Ghostty 完整嵌入、其他路线等）见 §14.7。

---

## 13. 来源

官方源码（固定提交，2026-10-05 读取）：

- Zed 仓库提交 279fe070bb389b79652e52065b2f001edcc0b11b（提交时间 2026-10-04T23:59:55Z）：<https://github.com/zed-industries/zed/tree/279fe070bb389b79652e52065b2f001edcc0b11b>
  - `crates/gpui/Cargo.toml`、`crates/gpui/README.md`
  - `crates/gpui_platform/Cargo.toml`、`crates/gpui_platform/src/gpui_platform.rs`
  - `crates/gpui_macos/Cargo.toml`、`crates/gpui_linux/Cargo.toml`、`crates/gpui_windows/Cargo.toml`、`crates/gpui_wgpu/Cargo.toml`、`crates/gpui_web/Cargo.toml`、`crates/gpui_apple/Cargo.toml`
  - `crates/gpui/src/element.rs`、`window.rs`、`text_system.rs`、`input.rs`、`platform.rs`、`platform/keystroke.rs`、`platform/visual_test.rs`、`app.rs`、`_accessibility.rs`
  - `crates/gpui/examples/`（`hello_world.rs`、`input.rs`、`testing.rs` 及目录清单）
  - `crates/terminal/Cargo.toml`、`crates/terminal_view/Cargo.toml`、`crates/ui/Cargo.toml`（许可）；`crates/terminal_view/src/terminal_element.rs`（仅看结构与接口用法）
  - 根 `Cargo.toml`、`rust-toolchain.toml`、`docs/src/development/macos.md`
- GPUI 官网：<https://www.gpui.rs/>

crates.io（接口，2026-10-05 读取）：

- <https://crates.io/crates/gpui>（0.2.2，2025-10-22，Apache-2.0）
- <https://crates.io/crates/gpui-pre>（0.3.0 于 2026-09-03 至 0.3.8 于 2026-10-05）；`gpui-pre-platform`、`gpui-pre-macos`、`gpui-pre-macros` 同版本
- <https://crates.io/crates/gpui-ce>（最近发布 2026-08-28）
- <https://crates.io/crates/gpui-component>（0.7.0，2026-09-28，Apache-2.0）
- <https://crates.io/crates/alacritty_terminal>（0.26.0，2026-04-06，Apache-2.0）
- <https://crates.io/crates/portable-pty>（0.9.0，2025-02-11，MIT）
- <https://crates.io/crates/libghostty-vt>（0.2.2，2026-09-28）
- 查询为不存在：`gpui_platform`、`gpui_macos`、`gpui_wgpu`、`gpui_linux`、`gpui_windows`、`gpui_web`

同类项目（GitHub，2026-10-05 读取仓库信息与清单文件）：

- <https://github.com/longbridge/gpui-kit>（gpui-component）
- <https://github.com/contember/okena>
- <https://github.com/penso/herdr-gpui>
- <https://github.com/zortax/gpui-terminal>
- <https://github.com/maddada/Ghostex>
- <https://github.com/arthjean/paneflow>

二手来源（未核实，本文不依赖）：关于 Zed 1.0 于 2026-04-29 发布的报道。

本仓库依据：`docs/DESIGN.md`（§5、§7、§10、§27、§29、§39、§44、§45、§50、§62 及 2026-10-02 之后各节）、`docs/UI设计语言.md`、`docs/插件协议.md`、`docs/UI回归.md`、`docs/任务/GPUI前后-待办重估与排序-2026-10-04.md`、`plugins/drover/README.md`，以及正文中标注的源码路径。

本机环境（2026-10-05）：macOS 27.0.1（arm64）、Xcode 27.0、rustc 1.96.0、cargo 1.96.0。

---

## 14. 方案提交后的讨论补充（2026-10-05）

方案通过主控复核后，用户与作者又讨论了几轮。本节记录这些讨论带来的新信息和作者意见的变化。**没有新的决定被批准**：原型是否启动、范围、路线、依赖渠道都仍待用户决定。

本节的标记沿用文首约定，另加两种：

- 【用户陈述】用户在讨论中的原话或明确表态。
- 【未查证】作者凭已有了解写的内容，当天没有核对来源。

### 14.1 对“官方发布停了一年”的澄清

作者在讨论中说过“官方发布停了一年”，用户追问后澄清如下，§2.2 的事实不变：

- 停的是 crates.io 上的官方发布，不是开发。`gpui` 在 crates.io 的最新版仍是 2025-10-22 的 0.2.2，到调研当天约 11 个半月；仓库里每天都有提交。
- 【查证】官方 README 第 11–12 行让使用者写 `gpui = { version = "*" }` 和 `gpui_platform = { version = "*" }`，而 `gpui_platform` 当天在 crates.io 上查不到。README 与实际发布状态不一致，可能意味着官方打算发布但还没发；作者没有找到官方对发布计划的说明，也没有查讨论区或博客。

### 14.2 GPUI 之外的路线

【用户陈述】“还有其他的路线吗？除了gpui”

下表全部【未查证】，是作者凭已有了解给出的，只用于判断哪些值得进一步查，不能当作结论。

| 路线 | 终端怎么来 | 主要好处 | 主要代价 |
| --- | --- | --- | --- |
| 继续 TUI，只做增强 | 外层终端 | 没有迁移成本，SSH 照用 | 外层终端的限制都还在 |
| iced（纯 Rust） | alacritty_terminal＋自绘 | 在 crates.io 正常发版；COSMIC 桌面的终端是这个组合 | 也没到 1.0；中文输入法现状不确定 |
| egui（纯 Rust，即时模式） | alacritty_terminal＋自绘 | 发版稳定；即时模式和现在 ratatui 的写法最像 | 文字排版和中文输入法历来偏弱 |
| Tauri＋网页界面 | xterm.js | 框架是稳定版；输入法由浏览器内核处理；同一套界面将来可能给浏览器和手机用 | 界面要用 JS/TS 写，与“仓库工具保持 Rust”的偏好冲突；手机访问还需要一层远程服务，与“不做常驻服务”的约定冲突 |
| Swift 原生外壳＋Rust 核心 | SwiftTerm 或 Ghostty 内核 | macOS 上输入法、菜单、无障碍最地道 | 两种语言，只能 macOS，要维护 Rust 与 Swift 的桥接 |

作者当时的意见【建议】：如果“纯 Rust”是硬约束，能和 GPUI 比的只有 iced；如果手机和远程迟早要做，网页路线值得认真考虑。这两条是否要专门查证，用户没有表态；后续讨论转向了终端集成（§14.3–14.5），在用户认可 Zed 终端水平之后，这项对比的紧迫性下降。

### 14.3 用户的首要顾虑：终端集成

【用户陈述】“对，我担心的主要是terminal那边的集成，我需要接近原声terminal或者类似ghostty直接可以嵌入进去的”

这改变了评估重点：终端的手感优先于其他界面。作者据此查了 Ghostty 的可嵌入现状，只读了下列仓库的 README，**没有读代码、没有运行**：

- 【查证】Ghostty 是 MIT 许可。README 说 libghostty 是可嵌入的 C/Zig 库，正在拆成多个库，从 `libghostty-vt` 开始；`libghostty-vt` 已可用，功能稳定但“API signatures are still in flux”，“We haven't tagged libghostty with a version yet”（ghostty-org/ghostty@5dc28bb8ee，2026-10-04）。
- 【查证】官方示例 Ghostling 的 README 说明 `libghostty-vt` 负责解析终端序列、维护终端状态和渲染状态，“contains no renderer drawing or windowing code; the consumer … provides its own”；它列出的能力包括 Kitty 键盘协议、Kitty 图片协议、带重排的缩放、鼠标上报（ghostty-org/ghostling@63842bf8e5）。
- 【查证】第三方 `gpui-libghostty` 自称“embeds a native Ghostty terminal in GPUI”，可读取用户的 Ghostty 配置；自述状态为 alpha；构建需要 Zig 0.16；仓库建于 2026-08-24，crates.io 上 0.3.1（behzade/gpui-libghostty@2bbde4ae53）。它是用了 Ghostty 自己的渲染还是也自己画，作者没看代码，不能确定。
- 【查证】paneflow 的 README 写明用的是 `libghostty-vt`（arthjean/paneflow@26430c1bb8）。

读法【推测】：现在官方能嵌的只有“内核”，带来的是与 Ghostty 一致的仿真，不是 Ghostty 的手感；字体渲染、输入法、滚动仍要使用方自己做，和用 alacritty_terminal 是同一类工作。“把完整的 Ghostty 终端面嵌进 GPUI”只有第三方 alpha 实现。

按“像不像原生终端”大致分三档【推测】：

| 做法 | 终端手感 | 现状 |
| --- | --- | --- |
| 继续在 Ghostty 里跑 TUI | 就是 Ghostty | 现在就有 |
| 桌面窗口里嵌 Ghostty 的完整终端面 | 有可能接近 Ghostty | 只有第三方 alpha 实现，未验证 |
| GPUI 里自己画（内核用 alacritty 或 libghostty-vt） | 上限估计接近 Zed 的内置终端 | 路走得通，手感要自己磨 |

连带影响【推测】：现在 PTY、回看、搜索、复制都由 Saddle 自己管（§3.2、§3.3）。如果改成嵌入完整的 Ghostty 终端面，这些会交给它，属于设计变更。

### 14.4 用户认可 Zed 内置终端的水平

【用户陈述】“其实zed的终端我试过，是可以满足我的要求的”

用户随后给了一张 Zed 内置终端里运行 Claude Code 的截图（“这是实际的表现”，文件在用户本机 `/Users/firegnu/Desktop/SCR-20261005-jkwx.png`，没有放进仓库）。作者从截图能看到的：

- 中英文混排的正文、列表、行内代码没有错位；
- 带中文单元格的表格，边框闭合、对齐；
- Claude Code 的方块字符图标没有缝隙；
- 表情、真彩色、粗体、底部状态栏正常；
- 中文字与字之间间距偏大，是每个汉字被固定放在两格里的结果。用户接受这个效果。

静态截图看不出的：输入法组字过程中的体验、大量输出时的流畅度、鼠标操作。这三项以用户“试过、满足要求”的表态为准，作者没有另行验证。

含义：

- **目标水平有了明确的参照**：“GPUI＋alacritty_terminal 自己画”这条路的目标定为“不比 Zed 内置终端差”。§4.1 里“桌面化期间不换终端内核”的建议因此维持；嵌入 Ghostty（§14.3）暂时不必追。
- 这证明的是这套组合能做到这个水平，**不是 Saddle 自己的实现已经做到**。Zed 的终端代码是 GPL（§2.5），按现在的打算只能参考思路。
- 许可是一个可以换省事的决定【推测】：如果 Saddle 采用 GPL-3.0，理论上可以直接借用 Zed 的终端代码；但它与 Zed 内部其他模块的耦合程度作者没有查，不能保证省多少。这仍属于 §11 第 11 问。
- 【建议】这张截图的内容可以当原型的对照样例：原型里发同一句话，让 Claude Code 输出同样的自测内容，与截图并排比较。

### 14.5 herdr-gpui

【用户陈述】“这两天有一个gpui-herdr的项目，看起来也在做这个。”

这就是 §2.8 列过的 penso/herdr-gpui。作者又读了它的根 README、GUI README 和性能说明的开头，**没有读代码、没有运行**（penso/herdr-gpui@6ae54eacaa，2026-10-05）：

- 【查证】第三方项目，自述与 Herdr 官方无关；Apache-2.0；仓库建于 2026-09-20，到调研当天两周，约 850 次提交、近 700 星；发布签名公证的 macOS 安装包，Linux 与 Windows 为实验性。
- 【查证】依赖固定 `gpui-pre` 0.3.6，工具链固定 Rust 1.96.1。
- 【查证】它自己不开 PTY、不做终端仿真：“It paints the daemon's terminal cells, split panes included, without running another terminal emulator or wrapping the TUI.” 仿真在 Herdr 的后台服务里；远程主机也是通过 Herdr 的服务经 SSH 接入。
- 【查证】实线制表符和方块字符（含分数块和象限块）是在字符格上自己画的，按设备像素对齐；虚线、双线、圆角线、带组合符的字素仍用字体渲染。
- 【查证】它有性能测试入口，对比带缓存的画法和“逐格整形、逐格背景”的参考画法。

对 Saddle 的意义【推测】：

1. 可行性多了一个独立的证据：两周做出了侧栏、分屏、字符格终端的桌面客户端。它提交很密，投入并不小。
2. 它是一份许可上可以直接参考的实现：Apache-2.0，画字符格的部分可以借鉴，带上声明也可以复用。代码质量作者没看过，不能担保。
3. 本机工具链的疑问（§2.4）有了更强的旁证：它用 Rust 1.96.1 构建，本机是 1.96.0。仍需实际编一次确认。
4. 结构上有一个关键差别：它有后台服务承担仿真，客户端很薄，远程也靠服务。Saddle 现在的设计是不做常驻服务（DESIGN §2），仿真在界面进程里，桌面端要多做一层。

关于中文：

- 【用户陈述】“herdr中文支持没问题，这个应该也没问题”
- 作者的保留意见【推测】：显示和输入要分开。中文显示大概率没问题，因为字符宽度由 Herdr 的后台服务计算，客户端照格子画。中文输入法不能从 Herdr 推出来：在 Herdr 的 TUI 里打中文顺畅，是外层终端替它处理了输入法；herdr-gpui 是独立窗口，输入法要它自己在 GPUI 里实现。它的 README 里作者没有找到输入法的说明。**herdr-gpui 的中文输入法没有人验证过**，装上打几句即可确认。

### 14.6 讨论之后作者意见的变化

以下都是作者的【建议】，不是用户的决定。与正文不一致的地方，以本节为作者的最新意见：

1. **原型先只做终端这一条轨**（§7 的甲轨），而不是三条并行。中文输入法、对齐、本机构建三项里任何一项不过关，另外两条轨就白做；终端过关后再决定是否补乙、丙。
2. **原型的验收标准定为“同样跑 Claude Code、输入中文，和 Zed 内置终端对比不差”**，用 §14.4 的截图内容作对照样例。这个标准用户自己可以判断。
3. **实现时可以参考 herdr-gpui**（Apache-2.0）的字符格绘制，Zed 的终端代码只看思路。
4. **依赖渠道**：原型用第三方快照 `gpui-pre` 图省事；真要落地，倾向改为固定官方仓库提交，因为快照渠道只存在了一个月。
5. **迁移路线不要现在定**。§5 推荐 D 的前提是现有页面能从 `src/app.rs` 的主循环里干净地拆出来，这是整份方案里最弱的一环，等原型之后再在 C、D 之间选。
6. **其他仍维持正文的建议**：保留 TUI、首期只做 macOS、原型阶段先不引入 gpui-component、T49/T64/T62/T56 照常推进。
7. **一件不依赖 GPUI 的事**：把 `src/app.rs` 里的工作区状态和界面状态分开，无论做不做桌面版都对 TUI 有好处，可以单列成行为不变的重构任务。它不小，是否现在做由用户定。

作者认为到此决定“要不要做原型”的依据已经够了：目标水平用户已认可；有两份实现可参考；还没有被任何证据覆盖的只有 Saddle 自己这一侧——现有终端和 PTY 代码接到 GPUI 上要改多少，这只有原型能回答。

### 14.7 本节新增的未验证项

- herdr-gpui 的中文输入法是否可用。
- herdr-gpui 的字符格绘制代码质量如何、能否直接借鉴到 Saddle。
- `gpui-libghostty` 用的是 Ghostty 自己的渲染还是自绘；完整嵌入 Ghostty 终端面是否可行。
- Zed 内置终端在输入法组字、高输出、鼠标操作上的表现（仅有用户表态，作者未验证）。
- §14.2 的其他路线全部未查证。
- 若 Saddle 采用 GPL-3.0，Zed 的终端代码能复用到什么程度。

### 14.8 本节来源

2026-10-05 读取，均为各仓库默认分支当时的 README 等说明文件：

- <https://github.com/ghostty-org/ghostty>（提交 5dc28bb8ee，2026-10-04）
- <https://github.com/ghostty-org/ghostling>（提交 63842bf8e5，2026-08-09）
- <https://github.com/behzade/gpui-libghostty>（提交 2bbde4ae53，2026-09-25）；<https://crates.io/crates/gpui-libghostty>（0.3.1）
- <https://github.com/arthjean/paneflow>（提交 26430c1bb8，2026-10-04）
- <https://github.com/penso/herdr-gpui>（提交 6ae54eacaa，2026-10-05）：根 `README.md`、`crates/herdr-gpui/README.md`、`crates/herdr-gpui/PERFORMANCE.md`
- GPUI README 第 11–12 行：同 §13 的 Zed 固定提交
- 用户提供的截图：`/Users/firegnu/Desktop/SCR-20261005-jkwx.png`（用户本机，未入库）
