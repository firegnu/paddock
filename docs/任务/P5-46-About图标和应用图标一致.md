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

## 完成记录

- 做了什么：`about.rs` 去掉画宠物像素猫的 `cat()` 和逐像素画方块的代码，改为用 `icon::icon(size)`（打包写 `AppIcon.icns` 的同一个函数）按当前屏幕的设备像素画出整张图标，换成 BGRA 交给 GPUI 的 `RenderImage` 显示，每个尺寸只画一次缓存；图标自带圆角方块，所以去掉了原来的外框和底色，只留阴影；圆角方块占满原来的 84pt，图标的透明边往外溢出。`Cargo.toml` 直接依赖 `image`（0.25，不开默认特性，GPUI 已锁 0.25.10，锁文件只多一行引用，没有新包），用来把像素包成 GPUI 要的帧。
- 验证了什么：临时开关让测试窗口启动后自动打开 About（临时 HOME、假 corral、`PADDOCK_NO_ACTIVATE=1`，只截 About 窗口，截图在 /tmp 看完已删，开关已撤），图标是黑底白猫头带青／品红色散，和 Dock 里的应用图标一致。`app/` 下 `cargo test --all-targets`（438 项全过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 都过。
- 拿主意的地方：用 `image` 而不是把 `png` 从开发依赖挪成正式依赖再编码解码一遍：`RenderImage` 本来就要 `image::Frame`，少一次编码。
- 没做的事：没加测试（纯显示改动）。
