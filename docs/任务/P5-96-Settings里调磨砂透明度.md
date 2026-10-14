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

## 完成记录

2026-10-14，开发 agent（分支 `p5-96-translucency`）。

### 做了什么
- 配置（`config.rs`）：新增 `sidebar_opacity`、`frame_opacity`，`Option<u8>`，整数百分比。大于 100 报 `<键> must be from 0 to 100`；负数、小数、字符串由 TOML 解析报错，都带键名（和别的设置项一样，配置读不进）。新常量 `FRAME_OPACITY = 65`。
- `theme.rs`：新类型 `Translucency { sidebar, frame }`，`Translucency::of(config, theme)`：写了用写的，没写时侧栏用主题的 `frost().wash`、外框用 65%。
- 窗口（`window.rs`）：`PaddockWindow` 存一份 `translucency`，启动时和 `apply`（Settings 写入后）按配置和新主题重算；`ground` 里侧栏列和外框的不透明度改用它，删掉常量 `FRAME_SHADE`。新增 `preview_translucency`，拖滑块时只改这份值并重画。只碰了底层用到不透明度的几行和 `Under` 两条注释，分块形状、标题栏都没动。全屏走 `Under::Sidebar/Frame`，不受影响。
- `windows.rs`：订阅 Settings 事件时多处理 `SettingsEvent::Preview`。
- Settings 规则（`settings.rs`）：新 `Kind::Opacity`，两项在 General（标签 `Sidebar opacity`、`Frame opacity`，报错时用）。显示值：写了的值，否则默认（侧栏是当前主题自带的 wash×100，外框 65）。范围 0–100。Default 删掉这一项（和颜色、界面字体同一做法）。换主题不碰这两项：没写的跟着新主题，写了的保留。
- Settings 界面（`settings_view.rs`）：General 在 `Interface` 下面加 `Translucency` 一节，两行 `Sidebar`、`Frame`，各一个滑块：外框和步进框同一底色（`field_ground`）、圆角 8、高 32；轨道 4pt 高，已填部分是强调色，旋钮和开关的旋钮同色带阴影；右边 `45%`。按下即跳到鼠标处的值并发 `Preview`，拖动跟着变（整窗跟踪拖动、窗外松手也算松手，做法同主窗口的分隔条），松手调 `set` 写入（照现有“改了就写”）。卡片下面一行淡字 `Only while the window isn't full screen`。Reset 链接写 `Reset to 45%`。
- `docs/DESIGN.md` P5-95 一条下加了 P5-96 的说明；`app/README.md` 配置示例加了两项。

### 验证
- 先写测试并看它们因目标缺陷失败（不是编译错误）再实现：
  - `config::opacities_are_whole_percentages_left_out_by_default`（读写、默认 None、超范围/负数/小数/字符串报错带键名）——RED：101 被接受。
  - `theme::the_grounds_opacities_come_from_the_config_else_the_theme_and_the_frame_default`（七套主题不写时 = 主题 wash 和 0.65；写了用写的）——RED：配置被忽略。
  - `settings::opacities_show_the_theme_or_the_frame_default_until_written`、`an_opacity_is_written_as_a_whole_percentage_and_default_removes_it`、`another_theme_keeps_a_written_opacity_and_leaves_one_unwritten`、`an_opacity_stays_from_0_to_100`——RED：默认显示为空、Default 写回而非删除、没有范围。
  - `settings_view::a_place_on_the_slider_is_a_whole_percentage`（两端、中间、四舍五入到 1%、越界取端点、轨道没排好时为 0；Reset 链接文字）——RED：恒为 0。
  - 改了 `the_refresh_interval_sits_with_the_corral_command_on_agents` 里 General 的项数 9 → 11（多了这两项）。
- `app/` 下 `cargo test --all-targets` 全过（lib 469 个），`cargo clippy --all-targets -- -D warnings` 无警告，`cargo fmt --check` 通过。
- 按任务要求没起测试窗口、没截图：滑块的样子、拖动时主窗口实时变化、窗外松手这些交互留给用户实际看。

### 拿主意的地方
- 说明淡字只写一行，放在 `Translucency` 卡片下面，而不是两行各写一遍（任务写“行下面一行淡字说明”）。英文用 `Only while the window isn't full screen`。
- 一节放在 General 的 `Interface` 后面、`Terminal` 前面。
- 两项都是“写了就算改过”：哪怕写的值等于默认，也显示圆点和 Reset，换主题时保留这个值（因为侧栏写了就不再跟主题走，要让用户能看到并删掉）。
- 松手（或只点一下）时值和在用的相同就不写文件，免得点一下旋钮就把“跟着主题”变成写死的 45。
- 配置里超过 100 当成错误（整份配置读不进，Settings 显示原因、不覆盖文件），和 `sidebar_width = 0` 等现有检查一样；没有悄悄截到 100。
- 拖动中预览值由 Settings 按当前草稿算好（`Translucency`）发给主窗口，主窗口不认键名；写入后照旧由 `Saved` → `apply` 按配置重算，两者一致。

### 没做的事
- 没加键盘调滑块（方向键）——任务没提。
- 没改 `preset.rs` 里 `Frost::wash` 的值和注释，也没加别的透明度项。
- 拖动中关掉 Settings 窗口时，主窗口停在预览的值，直到下次写入或重启；没有专门处理。

## 主控审查

2026-10-14，paddock/main。
- 看了全部 diff：配置两项和范围检查、`Translucency::of`（不写时侧栏用主题 `wash`、外框 65）、主窗口只把 `ground` 里两层的不透明度换成 `self.translucency`、拖动时 `SettingsEvent::Preview` 只改内存不写文件、松手才 `set`、回默认删键、换主题时没写的跟着走。改动范围和任务一致，没有越界。
- 合并进 main（已含 P5-97）时只有 `docs/DESIGN.md` 末尾两边都追加的冲突，两段都留下（P5-96 接在 P5-95c 下面，P5-97 在后）；`window.rs` 自动合并。合并后的工作区：`cargo test --all-targets` 全过（合计 590 个）、`cargo clippy --all-targets -- -D warnings` 退出码 0、`cargo fmt --check`、`git diff --check` 通过。顺手改了 DESIGN 里 P5-95b 那行“（已撤，P5-95c）”放错位置的列表符号（主控 P5-95c 时留下的）。
- 没起窗口看；滑块的样子、拖动时主窗口跟着变，留给用户实际用。
- agent 报告：中途误建的空文件 `paddock-worktrees/p5-96-translucency.tmp` 已被它挪进它的临时目录，主控核对 `paddock-worktrees/` 下没有残留。
- 留意：拖动中途关掉 Settings 窗口，主窗口会停在预览值，直到下次写入或重启（agent 记在“没做的事”）。

## 用户看过

10-14 装好后用户用了：“滑块能工作，有一个疑问（不是问题），划到最小不应该是几乎全透明吗？”主控解释：滑块只调 paddock 叠在系统材质上的那层颜色，0% 剩下纯系统磨砂（自带模糊和染色、不可调），最透也是毛玻璃；要真透明得在低段淡化系统材质，问题是字看不清（后面亮或花时）、后面窗口会动也会进截图、左右要拆成两块材质并实测淡化效果。用户：“先不做了，保持现状”。
