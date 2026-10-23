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
