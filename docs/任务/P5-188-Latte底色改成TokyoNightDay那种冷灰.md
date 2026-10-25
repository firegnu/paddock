# 任务：Latte 的终端区和右侧栏底色改成 Tokyo Night Day 那种冷灰

2026-10-25，paddock/main 记。
路由：常规 / 交叉审查不要 / 影响面：看得见（主控定：只改 Latte 一个预设的底色和跟着要压深的几个颜色，有现成的对比度测试把关，一眼看得出、随时能改回）
类型：样式／文案调整
依据：用户看了样稿（`docs/设计稿/P5-188-Latte底色/LatteCool.dc.html`）后问主控推荐，主控推荐“甲 `#e1e2e7`，直接改 Latte 预设”，用户：“可以，照你的推荐写任务文件”。
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
