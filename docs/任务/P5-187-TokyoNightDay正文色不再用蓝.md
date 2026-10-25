# 任务：Tokyo Night Day 正文色改成深石板灰，不再满屏是蓝

2026-10-25，paddock/main 记。
路由：常规 / 交叉审查不要 / 影响面：看得见（主控定：只改一个预设里三处同一个颜色，一眼看得出、随时能改回）
类型：样式／文案调整
依据：用户看了样稿（`docs/设计稿/P5-187-TokyoNightDay正文色/Main.dc.html`）选“甲 #343b58”；Latte 维持不变。
待用户：在 Tokyo Night Day 下重开 paddock 看正文色（已从分支装好）
执行：主控
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。

## 用户原话
- 10-25 发截图：“我想改两个浅色主题的配色。1: tokoy night day其他都还好，就是字体蓝色的太多了。2: 我现在用的这个catpuccin latte浅色主题，我想改一下右侧agent区域和右侧sidebar的背景色……这两点我希望先出设计图”
- 看过样稿：“只改tokoy night day，latte维持不变”；正文色选“甲 #343b58”。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-187-tokyo-day-text`，分支 `p5-187-tokyo-day-text`（开工时从 main 建）。
- 只动 `app/src/preset.rs`、`app/src/theme.rs` 里 Tokyo Night Day 的颜色和说明，以及相关测试。

## 现状
- Tokyo Night Day 的正文色 `text` 是 `#3760bf`（蓝），终端默认字色跟着它；`bright`（界面亮字）和终端第 15 色（亮白）也是这个蓝。在底色 `#e1e2e7` 上只有 4.5:1。

## 要做的
- 这三处从 `#3760bf` 改成 `#343b58`（Tokyo Night 上游 enkia 浅色版的正文色，8.5:1）：
  - `preset.rs` 的 `text`、`bright`；
  - `theme.rs` 里 `TOKYO_NIGHT_DAY` 的第 15 色（亮白）。
- 光标、链接、目录、强调色、状态色都保留原来的蓝和其他颜色。
- 两处文件开头的出处说明里写明改了什么、为什么。

## 怎么算做完
- 用户：看实际效果后认可（照样稿甲：正文不再是蓝）。

## 不做
- Latte 不动（用户原话）。
- 左侧栏卡片的正文（`agents_text`，已是深藏青 `#243f7e`）、pi 图标色（`#3760bf`）不改；要改另说。
- 不加新预设、不改别的主题。

## 完成记录（主控，10-25）
- 做了什么：Tokyo Night Day 的 `text`、`bright`（`app/src/preset.rs`）和终端第 15 色亮白（`app/src/theme.rs` 的 `TOKYO_NIGHT_DAY`）从 `#3760bf` 改成 `#343b58`；两处的出处说明写了改了什么、为什么。DESIGN P5-176 那条“终端的正文色照官方不变”后补了 P5-187 的例外。样稿存进 `docs/设计稿/P5-187-TokyoNightDay正文色/`（只收 Tokyo Night Day 那张）。
- 验证：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings` 通过；`cargo test --all-targets --no-fail-fast` 共 1091 个全过。纯颜色改动，没另造失败测试；最终效果要用户在 Tokyo Night Day 下实际看。已从本分支打包安装到 `~/Applications/paddock.app`（10-25 12:36）。
- 拿主意的地方：光标仍是蓝 `#3760bf`（样稿三栏都保留蓝光标）；终端默认字色跟着 `text` 走，所以不用单独改。
- 没做的事：左侧栏卡片正文 `agents_text`（`#243f7e`）、pi 图标色（`#3760bf`）没改；Latte 不动。
