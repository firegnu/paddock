# 任务：Cairn 的 Notes | Ledger | Lessons 切换做醒目（凹槽外框、实心胶囊）

2026-10-16，paddock/main 自己做。
类型：样式／文案调整
依据：用户 10-16 看了 P5-121 装好的样子：“现在的这个三个胶囊看起来很不明显。”看过样稿 `docs/设计稿/P5-121a-三胶囊醒目/switch.html` 选甲（原样加重），又说：“整体外框（包围三个胶囊的那个组件不明显），你要好好设计一下。既要明显又要精致”；看过 `track.html`：“一，开始做吧”。本轮只改切换的画法，不动三页的内容。
执行：主控
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。

主控自己做的理由：只改 `cairn_view.rs` 里切换这一个函数和它的几个常量，P5-118a、P5-121 是主控写的；规则已由样稿定死，属于看得见、随时能改回的活。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-121a-switch`，分支 `p5-121a-switch`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-121a-switch`（从 `.target/main` 克隆）。
- 动 `app/src/cairn_view.rs`、`docs/DESIGN.md` 和本文件。

## 要做的（照样稿 track.html「一　凹槽」配 switch.html「甲」）
1. 外框（凹槽）：比面板底色暗一档；里面顶上一道内阴影、一圈很淡的暗边；外沿下面一道 1 点亮线、外面一圈极淡的亮边。全圆角，仍靠左、按字宽。
2. 选中的胶囊：实心强调色，字用面板底色、加粗；顶上一道内亮线、底下一道内暗线，下面一点投影和很淡的强调色光。
3. 没选中的：字用正文色（不再是淡色），悬停变亮。
4. 条数小签：没选中的正文色字、文字色淡底；选中的底色字、底色淡底。
5. 字号和内边距照样稿（12.5，胶囊左右 14、上下 4，槽内边 3）。颜色都从主题取。

## 怎么算做完
- 用户原话：“三个胶囊看起来很不明显”“整体外框……既要明显又要精致”；样子和样稿一致。
- `git diff --check`，`app/` 下 `cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`cargo test --all-targets`。样子留给用户实际看。
