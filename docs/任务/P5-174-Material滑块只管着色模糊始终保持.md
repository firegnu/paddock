# 任务：Material 滑块只管磨砂的颜色浓淡，模糊始终保持，背后看不清形状

2026-10-23，paddock/main 交给 paddock/dev-frost（Claude Code，opus[1m] / high，不套墙：主控没设 child sandbox）。
路由：中 / 交叉审查不要 / 影响面：改行为（路由：重（置信 0.67）、拿不准、拿不准；推翻：用户点名 Claude Code opus high，读 `middle-design` 格；交叉审查和影响面主控定：不碰数据模型、并发、安全，按改行为）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
依据：用户 10-23 发来 macOS 菜单的截图，要整扇窗户最底下那层磨砂达到这种质感；现在 Material 滑块拉到 80% 左右，背后的东西就能看清形状了。本轮让滑块从 0 到 100 都保持菜单那种强度的模糊，只调颜色浓淡；同时做出侧栏材质和菜单材质两版给用户对比。GPUI 画的各层（侧栏底色、外框、CLI 面板、卡片、标签）不动。
执行：派出
（理由：用户选的：“Claude Code opus high”。）
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- 发来 macOS 苹果菜单截图（后面的橙色、蓝色窗口糊成一团团柔和的颜色，颜色仍然鲜艳）：“Apple的这种质感是不是很难达到？”
- “我要的就是最底下的那层能达到这个效果，现在第三个滑块material变得太快了，到80%的时候就是那种镜面的效果了。我要的在模糊的基础上的透明度。不知道我描述清楚没有”
- 主控复述（不管滑块在哪都保持菜单那样强的模糊，滑块只管磨砂上的颜色多浓），问“镜面”是能看清形状还是糊着但发暗：“对，就是第一种，背后能看清形状了”
- 主控提议两种材质都打包对比，用户选了“两种都打包对比”。

## 先读
- `app/src/frost.rs` 全文：系统磨砂（`NSVisualEffectView`，Sidebar 材质，常亮），`set_opacity` 现在设整块视图的 `alphaValue`。
- `app/src/window.rs`：`background()`（约 795–805 行，P5-170 磨砂时用 GPUI 的 `WindowBackgroundAppearance::Blurred`）和调 `frost.set_opacity` 的地方（约 4616–4625 行）。
- `app/src/theme.rs` 的 `Translucency`（约 17–42 行），`app/src/config.rs` 的 `MATERIAL_OPACITY`（约 14 行）。
- GPUI 的 Blurred 怎么做的（只读，Apache-2.0）：`~/.cargo/registry/src/*/gpui-pre-macos-0.3.8/src/window.rs` 的 `blurred_view_init_with_frame`、`remove_layer_background`（约 3776–3830 行）：Selection 材质，藏掉 `CAChameleonLayer`，从滤镜里删掉描述含 “Saturat” 的那个。
- `docs/DESIGN.md` §13 的 P5-169、P5-170 两条（约 629–630 行）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-174-frost`，分支 `p5-174-frost`（已从 main 建好）。主要动 `app/src/frost.rs`，需要时动 `window.rs`、`theme.rs`、`config.rs`、`settings_view.rs` 里和 Material 有关的几处；`docs/DESIGN.md` §13 补一条 P5-174（用户原话照上面抄），并把 P5-169、P5-170 里已经不对的说法改掉。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-174-frost`（已从 `.target/main` 克隆）。三项检查在 `app/` 下跑（仓库根没有 `Cargo.toml`；单元测试在 lib 目标里，`--bin paddock` 跑不到）。

## 要做的
1. **先查清现状**：在这台机器的系统上，弄清两块磨砂各自的层是什么样的：paddock 自己那块（`frost.rs`）里，哪层管模糊、哪层管提饱和度、哪层管着色（颜色薄膜）；GPUI 垫的那块 Blurred 被 `remove_layer_background` 改过以后，是否还在模糊（主控猜它删 “Saturat” 滤镜时把模糊一起删了，所以 Material 调低后背后能看清形状；没核实）。写进完成记录：查到的事实、推测、没查清的，分开写。
2. **滑块改成只管颜色浓淡**：paddock 那块系统磨砂的模糊和提饱和度始终保持满的，不再整块调 `alphaValue`；Material 滑块只调它的着色薄膜有多浓。100 是现在 100% 的样子；往下拉，背后那团模糊的颜色透得越多；拉到 0，背后仍然只是一团团柔和的颜色，看不清形状，颜色鲜艳（像用户截的菜单）。拉动时的变化大致均匀，不要前一段几乎不变、后一段一下变很多。具体用哪种办法（调着色层的不透明度、换别的层、或者别的路子）你查清后定，但要稳：系统换了层结构、找不到要的层时，退回到磨砂满强度、不出错、不崩。
3. **两种材质对比**：材质种类收成代码里一处（一个常量或一个函数），侧栏材质（现在的 Sidebar）和菜单材质（`NSVisualEffectMaterial::Menu`；你觉得别的更接近截图，也可以说）都要能按第 2 条工作。分支里提交的默认用 Sidebar；完成记录写明主控改哪一行就能打出菜单那一版。主控会两版都打包给用户挑，用户定了再改默认。不加设置项、不加配置键。
4. **兜底那层**：GPUI 的 Blurred（P5-170）在新做法下要不要留，你根据第 1 条查到的事实给建议；不确定就留着不动。全屏照旧不透明、没有磨砂。
5. **别的不变**：磨砂跟主题深浅、常亮（P5-24b）、点击不穿透（P5-169）、`sidebar_opacity` 和 `frame_opacity` 两个滑块、Settings 里 Material 滑块的样子和配置键 `material_opacity`、默认值 100，都不变。

## 怎么算做完
- 用户原话：“我要的在模糊的基础上的透明度”；“背后能看清形状了”是要去掉的问题。「要做的」五条逐条做到。
- 用户装上后实际看（两种材质各一版）：Material 从 100 拉到 0，全程都是磨砂，背后看不清形状，颜色保得住，变化均匀。
- 验证只做这些：一条针对这件活的测试（能测的那部分，比如滑块的数值怎么换算成着色浓淡；层结构的部分测不了就不测）；`git diff --check`；三项检查各跑一次（`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`）。查层结构可以起一个测试用的 paddock 窗口（照下面「不要做」里的规矩）打日志看；外观最终由用户看。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 GPUI 画的各层（侧栏底色、外框、CLI 面板、卡片、标签、标题栏胶囊）。
- 不加新依赖，不改 `gpui-pre-*` 版本，不改 GPUI 的源码（Blurred 要去掉或换掉，只在 paddock 自己的代码里做），不加设置项和配置键，不改用户本机的 `~/.config/paddock/config.toml`。
- 不复制 Zed 的 GPL 代码。私有接口（比如 `CGSSetWindowBackgroundBlurRadius`）不用；读写 `NSVisualEffectView` 内部的层（像 GPUI 那样按类名、滤镜描述找）可以，但要写清楚依赖了什么、找不到时怎么退。
- 起测试窗口时：加 `PADDOCK_NO_ACTIVATE=1`，`CORRAL_HOME` 指向一个空的临时目录（临时 HOME 隔离不了 corral，不然侧栏会列出用户的 agent），记下 PID，用完按 PID 停；截图只截这个测试窗口自己（`GPUI_TERM_WINDOW_ID=1` 拿编号，`screencapture -x -o -l <编号>`），截图放 worktree 外面、不入库；不用脚本模拟键盘鼠标。
- 不用 Python（包括临时批量改文件），命令里不用 `rm`、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名。
- 不碰 `corral ls` 里现有的 agent，不对任何 agent `corral send`。
- 碰到要拿主意的事（比如着色层找不到或调不动、只能靠私有接口、两种材质有一种做不成、要动 GPUI 画的层才像），停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等，不要自己换别的做法。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-174-frost` 上提交。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：查到的层结构（事实、推测、没查清的分开）、做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具或环境的行为和预想不一样的；没有就写「没有」），在你的分支里提交。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 查到的层结构
事实（本机 macOS 27.0.1，起测试窗口打日志看层树；临时打印代码已删）：
- paddock 那块（`frost.rs`，`NSVisualEffectView`）：视图层下面一个容器层（`kCUIVariantSidebarMaterial` / `kCUIVariantMenuMaterial`），里面从下到上四层：
  1. `CABackdropLayer`：滤镜 `sdrNormalize`、`gaussianBlur`（半径 30）、`colorSaturate`（2.1–2.4）。管模糊和提饱和度。
  2. `fill`（`CALayer`）：底色就是着色薄膜。深色 0.157 灰，不透明度 0.8（Menu 0.7）；浅色 0.965 灰，0.84（Menu 0.72）。
  3. `tone`（`CALayer`）：纯灰，深色用 `lightenBlendMode`（0.14 / 0.125），浅色用 `darkenBlendMode`（0.915 / 0.93）。把暗处提亮、亮处压暗，也算着色。
  4. `CAChameleonLayer`：桌面淡色，系统把它的不透明度设成 0.05。
  Sidebar 和 Menu、深色和浅色四种组合都是这个结构，只有数值不同。
- 层名会变：深色叫 `backdrop`、`desktop tint`，浅色叫 `Backdrop`、`Chameleon`。所以按类名找，不按层名。
- 换外观时 AppKit 立刻拆掉整组层，到下一次绘制才重建，新层的不透明度是满的 1。调 `displayIfNeeded` 会马上重建。
- GPUI 垫的那块（`BlurredView`，Selection 材质，经 `remove_layer_background` 改过）：里面只有一层 `CALayer`，合成滤镜 `vibrantColorMatrixSourceOver`，底色被清掉了，**没有 `CABackdropLayer`，没有模糊**。所以它什么都不显示。主控猜“删 Saturat 时把模糊一起删了”不对：本来就没有模糊层，`remove_layer_background` 删的 Saturat 滤镜在它里面也不存在。P5-169/170 的效果其实是：整块视图的 `alphaValue` 往下调，就把没模糊的桌面按比例混进来，所以 80% 左右就能看清形状。
- 标题栏的 `NSTitlebarView` 下面也有一个不带名字的 `CABackdropLayer`，那是系统标题栏自己的，不在我们视图里。

推测：
- AppKit 还会在别的时候重建这些层，比如换屏幕、改缩放、窗口状态变化。每帧都重设能接住，最多晚一帧。
- 合成是在接近 sRGB 的空间里线性混合，所以着色不透明度按比例线性变化，看起来大致均匀。

没查清的：
- 别的 macOS 版本层结构是否一样。只在这台机器上看过。
- 实际画面效果：没截图，留给用户看。测试窗口不在前台，截到的背景不代表实际。

### 做了什么
- `frost.rs`：
  - 去掉整块视图的 `alphaValue`，视图始终完全不透明，所以模糊和提饱和度始终满的。
  - 新增 `set_tint`：Material 的比例（0–1）直接当作着色层（`fill`、`tone`）的不透明度。线性，100 就是系统原样。`CAChameleonLayer` 不动。
  - 找层的办法：从视图层往下最多 4 级，找一组同级层，其中有 `CABackdropLayer` 且它上面还有别的层。上面那些层除去 `CAChameleonLayer` 就是着色层。类名用 `AnyClass::get` 和 `isKindOfClass:` 判断，读写都走 `msg_send!`，不加依赖。
  - 找不到（类不存在、结构变了、模糊上面没有层）就什么都不改，磨砂保持满强度，不会崩。
  - 设不透明度时包在 `CATransaction` 里、关掉隐式动画，拖滑块马上看到变化。不透明度和要的值一样就不写。
  - 着色值存在 `Frost` 里，每帧由 `fit_frost` 重设。换外观（`set_dark`）和显示（`set_shown(true)`）时先 `displayIfNeeded` 让层马上重建，再设。
  - 材质种类收成一个常量 `MATERIAL`。
- `window.rs`：`set_opacity` 改成调 `set_tint`。改了 `background()` 的注释和对应测试的名字、注释，让它们和查到的事实一致。行为没改，`Blurred` 留着。
- `config.rs`、`theme.rs`：注释改成“着色强度”。配置键、默认 100、Settings 都没改。
- `docs/DESIGN.md` §13：给 P5-169、P5-170 不对的地方加了“P5-174 改/查实”的说明，新增 P5-174 一条，用户原话照抄。

### 主控要打菜单那一版
`app/src/frost.rs` 中 `const MATERIAL: NSVisualEffectMaterial = NSVisualEffectMaterial::Sidebar;` 一行，把 `Sidebar` 改成 `Menu`。我在测试窗口里看过，菜单材质深浅两种外观的结构都对，`fill`、`tone` 都按比例变了（设 40% 时都是 0.4）。截图里那种菜单效果更接近哪一种，我没法判断。两种的差别只在着色薄膜浓淡（Menu 稍淡）和饱和度（2.2 / 2.1，Sidebar 深色 2.4），模糊半径一样。

### 验证
- 新测试 `frost::tests::the_tint_is_what_lies_over_the_blur_but_the_desktops_colour`：测“哪些层算着色”和找不到时退回。先看到它失败（“模糊上面没有层”那条），再改实现让它通过。
- 测试窗口（`PADDOCK_NO_ACTIVATE=1`，`CORRAL_HOME` 和 `HOME` 都指向临时空目录，按 PID 停）：
  - Sidebar、Menu 各在深色、浅色下设 `material_opacity = 40`，打出层树：`fill`、`tone` 都是 0.4；`CABackdropLayer` 的模糊半径 30、提饱和度原样；视图本身不透明度 1。
  - 试过换外观：层会重建，满强度的新层被重设回 0.4。
  - 最后用定稿的程序设 `material_opacity = 0` 跑了一次，正常、没有报错。
- `git diff --check` 通过；在 `app/` 下 `cargo test --all-targets` 全部通过（lib 845 个），`cargo clippy --all-targets -- -D warnings` 通过，`cargo fmt --check` 通过。
- 没截图，外观留给用户看。

### 拿主意的地方
- 着色层的范围定为 `fill` 加 `tone`，也就是模糊上面除桌面淡色以外的所有层，只调 `fill` 不够。`tone` 在深色下把暗处提亮、在浅色下把亮处压暗，留着它到 0 也还是一层灰白，颜色不鲜艳。桌面淡色本来就只有 0.05，而且它的不透明度是系统自己管的，不碰。
- 换算用线性（着色不透明度 = Material 比例）。理由见上面“推测”第二条。
- 兜底那层（GPUI 的 `Blurred`）：**建议去掉**（`background()` 返回 `Transparent`）。查实它在本机什么都不显示。现在系统磨砂一直是满的，而且它的模糊取的是窗口后面的东西，下面垫什么都看不见。按“不确定就留着”的规矩，这一轮没改行为，只改了说错的注释。去不去由主控定。

### 没做的事
- 没去掉 `Blurred`（理由见上）。
- 没调模糊强度、饱和度，没加设置项和配置键。
- 没截图。
- 没在别的 macOS 版本上验证。

### 踩过的坑
- 打印层树的临时代码第一次把 `backgroundColor` 的返回类型声明成对象，objc2 在调试构建里检查类型编码直接崩了。改成按 `CGColor` 指针声明才好。
- 同时起三个测试窗口时，后两个一声不响就退了（像是只允许开一个），改成一个一个起。
- 上一轮的测试窗口（PID 23530）忘了停，后来发现了按 PID 停掉。
- 过滤日志时把标题栏自己的 `CABackdropLayer` 看成了我们视图里的。为此给找层的逻辑加了“模糊上面没有层就继续往下找”，这一条有测试。
- 有一次误在命令末尾敲了一个等标准输入的 `python3 -`（什么也没执行），挂住了，已经停掉。
