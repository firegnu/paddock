# 任务：Settings 里用滑块调左右两边的透明度

2026-10-14，paddock/main 记。
依据：用户 10-14 看过 P5-95c 后：“能否把左右两边的透明参数放到settings中，可以随时调，随时看到效果。我理解其实就是透明度的调整”。主控提了做法（General 加一节、两行、百分比，默认现在的值，只在磨砂时起作用），用户：“两件都按你的建议做。改变用滑块。”
执行：派出

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-96-translucency`，分支 `p5-96-translucency`（开工时从 main 建）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-96-translucency`。
- 会动 `app/src/config.rs`、`settings.rs`、`settings_view.rs`（新的滑块控件）、`theme.rs` 或 `window.rs`（把两个值用到底层上）、相关测试、`docs/DESIGN.md`、`README` 里列配置项的地方（如果有）和本文件。

## 现在是怎样
磨砂时（不是全屏），窗口底层分两块（P5-95、P5-95c）：
- 侧栏那一列：系统材质上叠一层侧栏底色，不透明度是主题自带的 `wash`（`preset.rs` 的 `Frost::wash`，七套主题现在都是 0.45），`window.rs` 的 `ground` 里用 `self.theme.frost().wash`。
- 其余（窗格周围、标题栏右半段；侧栏收起时标题栏整行）：叠一层深色外框色，不透明度是 `window.rs` 的常量 `FRAME_SHADE`（0.65）。
Settings 里改了的值写进配置文件后，窗口当场生效（`SettingsEvent::Saved`）；数字类设置现在用 `−`／`+` 步进框，没有滑块。

## 要做的
1. 配置文件加两项，整数百分比 0–100：`sidebar_opacity`（侧栏那一列叠的侧栏色）、`frame_opacity`（右边叠的外框色）。不写时：`sidebar_opacity` 用主题自带的（现在 45），`frame_opacity` 用 65。超出范围的值按设置项现有的报错／限制办法处理。
2. Settings 的 General 页加一节 `Translucency`，两行 `Sidebar`、`Frame`，各一个滑块：拖动或点轨道改值，右边写出当前百分比（如 `45%`），步进 1%。行下面一行淡字说明（英文，意思是“只在窗口不是全屏时起作用”）。滑块的样子跟 Settings 现有控件一致（颜色、圆角、高度和步进框同一套）。
3. 拖动滑块时主窗口的底层跟着实时变，用户能边拖边看；松手时写进配置文件（不要每动一下都写文件）。点轨道直接跳到那个值并写入。
4. 能回到默认：照 Settings 里其他设置回默认的现有办法（回默认后配置文件里删掉这一项，`Sidebar` 回到主题自带的值）。
5. 换主题时，没设 `sidebar_opacity` 的跟着主题自带的值走；设了的保持用户的值。
6. 全屏时窗口照旧整窗不透明，这两项不起作用，样子不变。

## 怎么算做完
- 用户：“能否把左右两边的透明参数放到settings中，可以随时调，随时看到效果。我理解其实就是透明度的调整”“两件都按你的建议做。改变用滑块。”
- 自查：`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 通过。行为改动先写测试、看它因为目标缺陷失败再实现，至少覆盖：两项的读写和默认值（不写时侧栏用主题值、外框 65）；超范围的处理；滑块上鼠标位置换算成值（两端、中间、越界）；回默认删掉配置项；窗口底层用到的两个不透明度来自配置（不依赖 GPUI 的部分）。
- 在本文件末尾写「完成记录」：做了什么、验证了什么、拿主意的地方、没做的事。

## 不要做
- 不改底层分块的形状、卡片、标签、标题栏按钮的样子（标签另由 P5-97 改，同一时间主控在别的分支改 `window.rs` 的标题栏部分，你只碰底层用到不透明度的那几行，合并时由主控处理冲突）。
- 不加别的透明度项（卡片、标签、侧栏卡片面等）；不改主题文件里的颜色。
- 不起测试窗口、不截图；不开真的 agent；不碰用户的 agent（`corral ls` 里现有的）。
- 不用 Python；命令里不用 `rm`、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。
- 不合并、不推送、不安装：做完在分支上提交，报告给主控。
