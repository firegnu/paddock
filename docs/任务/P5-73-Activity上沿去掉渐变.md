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
