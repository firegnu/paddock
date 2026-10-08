# 任务：About 窗口的图标换成现在的应用图标

2026-10-08，paddock/main 自己做（`about.rs` 一处）。
类型：Bug 修复
依据：用户 10-08：“about对话框还有一个问题，就是logo和我现在的不一样”。P5-20 把应用图标改成黑底白猫头带青／品红色散（`app/src/icon.rs`，打包写进 `AppIcon.icns`），About 还在方块里画宠物的像素猫，注释说“和 icon.rs 一样”已经不对。
提示：依据证据定位并修复导致问题的原因，保持无关行为不变。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-46-about-icon`，分支 `p5-46-about-icon`（已从 main 建好）。

## 要做的
- About 的图标直接用 `icon::icon(size)` 按窗口里需要的设备像素大小画出来显示，以后应用图标再改，About 自动一致；去掉 About 里画像素猫的旧代码和外框、底色（图标自带圆角方块）。
- 其余文字和布局不动。

## 怎么算做完
- 用户原话：“about对话框还有一个问题，就是logo和我现在的不一样”“可以，写完就做”。
- 截一张 About 的图，和 Dock 里的应用图标是同一个样子；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 都过。
