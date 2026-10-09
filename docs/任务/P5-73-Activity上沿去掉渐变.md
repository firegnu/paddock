# 任务：Activity 面板上沿不再画渐变，卡片滑下来时不出现一块黑

2026-10-09，paddock/main 写，主控自己做（去掉 P5-70 的一段渐变，上下文在手）。
类型：Bug 修复
依据：
- 用户 10-09 给了侧栏截图，箭头指着 Activity 面板上方横跨卡片的一条暗带：“有一个小问题，你自己修一下吧。agent卡片滑下来的时候，提交记录卡片会出现我箭头所示的一块黑色。”
- 原因：P5-70 在面板上沿往上 28pt 画了一段渐变（面板底色从透明到 78%），GPUI 没有遮罩，只能画成一块和面板同宽、直角的长方形；卡片面比面板底亮时，它就成了压在卡片上的一块暗色。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-73-no-fade`，分支 `p5-73-no-fade`（已从 main 建好）。

## 要做的
1. 去掉面板上沿的渐变，以及只为它记滚动位置、重画的代码；面板本身（半透明、圆角、细边）和底栏照 P5-70／70b 不变。
2. DESIGN §13 P5-70 条改掉渐变那几句，写明 P5-73 去掉及原因。

## 怎么算做完
- 用户原话：“agent卡片滑下来的时候，提交记录卡片会出现我箭头所示的一块黑色。”
- `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 都过（只删画法，没有新行为可测；渐变的那条测试随代码删掉）。样子留给用户看。

## 完成记录

- 做了什么：`activity_view.rs` 去掉面板上沿的渐变（`FADE`、`fade_height`、`Frame.fade` 和它的字段）；`sidebar.rs` 去掉只为渐变算浓淡的 `fade()`、记滚动位置的 `scroll`／`scrolled` 和每帧比较后重画的 `on_next_frame`（都是 P5-70 为渐变加的，列表照旧靠 `overflow_y_scroll` 滚动）。面板本身、底栏、列表末尾留出的高度都没动。DESIGN §13 P5-70 条渐变那一句改成 P5-73 去掉及原因。
- 验证了什么：只删画法，没写新测试；渐变浓淡那条测试随 `fade()` 删掉。`app/` 下 `cargo test --all-targets`（509 项全过，少的一项就是它）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 都过。
- 拿主意的地方：没改成更淡或更短的渐变：只要是盖在卡片上的长方形，卡片比面板底亮就会露出一块；直接去掉最干净。
- 没做的事：没起 paddock 窗口；装好后留给用户看卡片滑到面板后面的样子。
