# 任务：加第一套浅色主题 Catppuccin Latte，Settings 里能选

2026-10-23，paddock/main 交给 paddock/dev-latte（Claude Code，opus[1m] / xhigh，不套墙：主控没设 child sandbox）。
路由：高 / 交叉审查不要 / 影响面：改行为（路由：重（置信 0.8）、不要、拿不准；读 `high-design` 格；影响面主控定：动很多处颜色计算，深色必须不变，按改行为）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
依据：paddock 七套内置主题全是深色的。用户 10-23 要加一套浅色的试试，从三种样稿里选了 Catppuccin Latte。本轮加这一套，Settings 的主题选择里能选到，并把界面里默认按深色写的地方在浅色下理顺；七套深色主题的样子一点不变。Rosé Pine Dawn、Tokyo Night Day 不做。
执行：派出
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- 主控说 paddock 没有浅色主题，可以加 Catppuccin 的浅色版 Latte：“可以添加一个浅色主题尝试看看。”
- 看了三种浅色样稿（Catppuccin Latte、Rosé Pine Dawn、Tokyo Night Day）：“先弄第一个。弄出来我看看。settings里面要有对应的选择哦”

## 先读
- 样稿：`docs/设计稿/P5-175-浅色主题/README.md`，和 `Main.dc.html` 里 Latte 那组颜色（`renderVals` 的 `latte`；HTML 源码，不用打开浏览器）。样稿只是大致样子，具体颜色以 Catppuccin 官方 Latte 色板为准。
- `docs/任务/P5-40-四套深色新主题.md` 和 `docs/DESIGN.md` §13 P5-40 那条（约 399–400 行）：上次加主题是怎么加的、出处和许可怎么写、对比度怎么处理。
- `app/src/preset.rs`：`Preset`（`ALL`、`name`、`label`、`frost`）和 `catppuccin()`（Mocha 怎么按角色取色）。
- `app/src/theme.rs`：`CATPPUCCIN` 终端配色、`frame()`（把底色压暗算外框）、`frosted()`、对比度测试（`preset_ansi_colors_have_at_least_three_to_one_contrast` 等）。
- `app/src/settings_view.rs` 主题卡片那段（约 2230–2310 行，按 `Preset::ALL` 生成，每张带一个小窗口缩略图）。
- `app/src/frost.rs` 和 `window.rs` 的 `fit_frost`：系统磨砂按侧栏底色深浅切深色、浅色外观。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-175-latte`，分支 `p5-175-latte`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-175-latte`（已从 `.target/main` 克隆）。三项检查在 `app/` 下跑（仓库根没有 `Cargo.toml`；单元测试在 lib 目标里，`--bin paddock` 跑不到）。
- `docs/DESIGN.md` §13 补一条 P5-175（用户原话照上面抄）。

## 要做的
1. **加主题**：新增一套 Catppuccin Latte，配置里写 `theme = "latte"`，显示名 `Catppuccin Latte`，排在 `Catppuccin Mocha` 后面。界面各角色照 Mocha 的取法换成 Latte 的对应色（base、mantle、crust、surface0–2、overlay0–2、subtext、text、mauve 强调等）；终端 16 色、光标、选区照 Catppuccin 官方发布的 Latte 终端配色。出处和许可（MIT）照 Mocha 那样写在注释里。
2. **对比度**：照现有主题的规矩（终端颜色对背景至少 3:1 等，测试里有），达不到的色像 P5-40 那样只调到刚好够，并在注释写明调了哪个、从多少调到多少。Latte 的黄色、浅色几个大概率要调。
3. **Settings 能选**：Settings 的主题选择里出现 `Catppuccin Latte`，缩略图画的是浅色的样子，点了立即换上，写进配置；换回别的主题也正常。
4. **界面在浅色下理顺**：主窗口（左侧栏、标题栏、标签和胶囊、CLI 面板、右侧栏、卡片、看板、命令面板、弹框、提示）、Settings 窗口、New Agent 窗口，在 Latte 下逐处看一遍。凡是默认按深色写的（比如外框用底色压暗算、某处写死了白色或黑色、提亮用的淡色、阴影、失焦去色 P5-162、跑马灯、状态色的淡底），在浅色下看起来不对的就改成跟着主题深浅走。系统磨砂在浅色下用浅色外观（现在已经按侧栏底色深浅判断，核实一下）。改法尽量是“按主题深浅选方向或数值”，不给每处单独加主题颜色键；真要加，先停下来问。
5. **深色不变**：七套深色主题画出来的颜色和现在完全一样。能用测试守住的就加测试（比如几处按深浅分支的函数，深色时结果和原来一致）。

## 怎么算做完
- 用户原话：“先弄第一个。弄出来我看看。settings里面要有对应的选择哦”。「要做的」五条逐条做到。
- 用户装上后实际看：Settings 里能选 Catppuccin Latte，选了整个 paddock 是浅色的，字都看得清，没有哪块还是深色或者怪怪的；换回原来的主题和以前一样。
- 验证只做这些：主题和对比度的测试按新主题补上（照现有测试的写法，加进按主题循环的那些测试里）；第 5 条的测试；`git diff --check`；三项检查各跑一次（`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`）。第 4 条要逐处看，可以起测试窗口截图看（照下面「不要做」里的规矩），截图不入库；最终外观由用户看。觉得不够，在回复里说，不要自己加。

## 不要做
- 不做 Rosé Pine Dawn、Tokyo Night Day；不加“跟随系统深浅自动换主题”之类的功能；不改七套深色主题的任何颜色。
- 不改界面布局、尺寸、文案（新主题的显示名除外），不加设置项。
- 不加新依赖，不改 `gpui-pre-*` 版本，不改用户本机的 `~/.config/paddock/config.toml`。
- 不复制 Zed 的 GPL 代码；Catppuccin 色值是 MIT，可以用，写明出处。
- 起测试窗口时：用一个临时 HOME 放测试用的 `.config/paddock/config.toml`（写 `theme = "latte"`），加 `PADDOCK_NO_ACTIVATE=1`，`CORRAL_HOME` 指向一个空的临时目录（临时 HOME 隔离不了 corral，不然侧栏会列出用户的 agent），记下 PID，用完按 PID 停；截图只截这个测试窗口自己（`GPUI_TERM_WINDOW_ID=1` 拿编号，`screencapture -x -o -l <编号>`），截图放 worktree 外面、不入库；不用脚本模拟键盘鼠标。窗口被挡住时 GPUI 会停画，截到的可能是旧画面。
- 不用 Python（包括临时批量改文件），命令里不用 `rm`、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西（上一件活误敲过 `python3 -`）。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名。
- 不碰 `corral ls` 里现有的 agent，不对任何 agent `corral send`。
- 碰到要拿主意的事（比如要加主题颜色键、某处浅色下怎么改都别扭要动布局、深色的颜色不得不跟着变），停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等，不要自己换别的做法。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-175-latte` 上提交。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：做了什么（第 4 条改了哪些地方，各一句）、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具或环境的行为和预想不一样的；没有就写「没有」），在你的分支里提交。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
