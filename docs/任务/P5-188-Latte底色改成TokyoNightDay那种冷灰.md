# 任务：Latte 的终端区和右侧栏底色改成 Tokyo Night Day 那种冷灰

2026-10-25，paddock/main 记。
路由：常规 / 交叉审查不要 / 影响面：看得见（主控定：只改 Latte 一个预设的底色和跟着要压深的几个颜色，有现成的对比度测试把关，一眼看得出、随时能改回）
类型：样式／文案调整
依据：用户看了样稿（`docs/设计稿/P5-188-Latte底色/LatteCool.dc.html`）后问主控推荐，主控推荐“甲 `#e1e2e7`，直接改 Latte 预设”，用户：“可以，照你的推荐写任务文件”。
待用户：重开 paddock 看 Latte 的新底色（已从分支装好）
执行：主控
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。

## 用户原话
- 10-25：“我现在用的这个catpuccin latte浅色主题，我想改一下右侧agent区域和右侧sidebar的背景色，现在我觉得有点白”，先出了水泥灰样稿；用户当时：“只改tokoy night day，latte维持不变”。
- 之后：“carpuccia现在的light theme背景色我要的不是那种你渲染出来的那种水泥灰。我想要的是tokoy night day那种背景色”。
- 看过第三张样稿（现在／甲 `#e1e2e7`／乙 Latte crust `#dce0e8`）：“你的推荐呢？”主控推荐甲、直接改 Latte 预设；用户：“可以，照你的推荐写任务文件”。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-188-latte-ground`，分支 `p5-188-latte-ground`（开工时从 main 建）。
- 只动 `app/src/preset.rs`、`app/src/theme.rs` 里 Latte 的颜色和说明、DESIGN 里 P5-175 那条，以及样稿存档。

## 现状
- Latte 的 `bg` 是 base `#eff1f5`，终端底色跟着它；中间终端区、右侧栏（Kanban、Changes、Cairn）和右侧栏的凸起框都从这个底色来，窗格之间的缝是它压暗 8%（`#dcdee1`）。

## 要做的
- Latte 的 `bg` 从 `#eff1f5` 改成 `#e1e2e7`（Tokyo Night Day 的底色）。终端区、右侧栏一起变，缝跟着变成约 `#cfd0d5`，凸起框的面跟着变。
- 底色变暗后达不到对比度的，照 P5-175 的做法只往黑色压到刚好够，说明里写从多少到多少。样稿算出的值：
  - 终端色（对底色 3:1）：绿 `#3f9e2b`→约 `#3b9328`、黄 `#c27b19`→约 `#b47217`、品红 `#cd67b2`→约 `#be5fa5`、青 `#179299`→约 `#178f96`、亮黑 `#888b96`→约 `#7e818c`；
  - 灰的提示字 `muted`（对底色 4.5:1）：`#6b6d7f`→约 `#626475`。
  - 以现有测试 `preset_ansi_colors_have_at_least_three_to_one_contrast`、`quiet_text_and_status_colors_read_on_their_grounds` 为准，先改底色看它们失败，再压深到通过。
- 选区照 Latte 原来的取法（overlay2 30% 叠在底色上）按新底色重算。
- 右侧栏里直接画在这个底上的其他字（Kanban、Changes、Cairn 用到的颜色）核一遍，读不清的同样压深。
- Latte 的出处说明和 DESIGN 写明：底色不再是 Catppuccin 的 base，改了哪些、为什么。

## 怎么算做完
- 用户：看实际效果后认可（照样稿甲：终端区和右侧栏是 Tokyo Night Day 那种冷灰）。

## 不做
- 左侧栏、正文色、强调色、状态色不动；不加新预设，原版 Catppuccin Latte 不另留。
- 不改别的主题。

## 完成记录（主控，10-25）
- 做了什么：
  - Latte 的 `bg` 从 `#eff1f5` 改成 `#e1e2e7`（`app/src/preset.rs`）。
  - 终端色压深到 3:1（`app/src/theme.rs` 的 `LATTE`，普通和亮色各一份）：绿→`#3b9328`、黄→`#b47217`、品红→`#be5fa5`、青→`#178f96`、亮黑→`#7e818c`。选区按新底重算为 `#c3c4ce`。
  - `muted` 压深到 `#626475`（4.5:1）。
  - 右侧栏用的侧栏安静字和状态字原本只按左侧栏 mantle 调到 4.5:1，在新底上约 4.2:1，按两个底中更暗的那个压深：`agents_dim`→`#626472`、`agents_dimmer`→`#7d808e`（3:1）、`agents_green`→`#2e731f`、`agents_red`→`#c90e37`、`agents_blue`→`#1b5bdc`、`agents_yellow`→`#8d5912`、`agent_stalled`→`#ac4408`。左侧栏上这几个字会略深一点。
  - 两处出处说明、DESIGN P5-175 条下补了 P5-188 子条；样稿存进 `docs/设计稿/P5-188-Latte底色/`（只收用户要的冷灰那张）。
- 验证：
  - TDD：只改底色时 `preset_ansi_colors_have_at_least_three_to_one_contrast`（Latte ANSI 2 只有 2.65:1）和 `quiet_text_and_status_colors_read_on_their_grounds`（Latte muted 3.94:1）先失败；压深后通过。
  - 新加测试 `right_sidebar_text_reads_on_the_terminal_ground`：右侧栏用到的安静字和状态字对终端底色要够 4.5:1（dimmer 3:1），先跑出 Latte 7 处不够，压深后通过。
  - `remote_start` 一条测试的期望值从旧底色改成新底色：paddock 告诉 corral 的终端背景色跟着变成 `#e1e2e7`，这是想要的结果。
  - `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings` 通过；`cargo test --all-targets --no-fail-fast` 共 1092 个全过。已从本分支打包安装到 `~/Applications/paddock.app`。
- 拿主意的地方：
  - 新测试只查浅色主题。全部主题一起查时，深色的 Catppuccin Mocha `agents_dim`（4.44:1）和 Tokyo Night `agents_dimmer`（2.88:1）本来就不够，和这次无关，任务又写明不改别的主题，所以没修，报给用户。
  - 强调色 mauve 照任务不动，右侧栏里当链接用时在新底上约 4.2:1。
- 没做的事：左侧栏、正文色、强调色不动；没加新预设，不另留原版 Latte。
