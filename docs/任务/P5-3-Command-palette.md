# 任务：Command palette——一个入口搜 agent、标签、设置、命令和终端文字

2026-10-06，paddock/main 交给 paddock/dev-palette（Claude Code，重：opus[1m] / xhigh）。
路由：重 / 交叉审查不要 / 影响面：改行为（路由：重；交叉审查拿不准（并发 0.25），按不碰数据并发安全定为不要；影响面拿不准，按会改命令执行和跳转定为改行为）
类型：功能变更
依据：本轮把 Go to Agent 做成 command palette，加看得见的入口；终端查找栏（⌘F）保留。侧栏卡片（P5-8）和终端拖放（P5-9）由别的活在做。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」一节（编译目录、桌面窗口测试、截图规则、`PADDOCK_NO_ACTIVATE`、不模拟按键）。
- `docs/DESIGN.md` §13（界面改版原则、界面文字用英文、P5-3 的用户原话）。
- `docs/设计稿/P5-界面改版/README.md` 和 `Palette.dc.html`（本任务的依据，组件逻辑在文件末尾的脚本里：三种模式、分组、筛选、提示行）。读 HTML 和内联样式里的尺寸、层次；颜色只看用的是哪种主题色，不照抄十六进制。
- `app/src/window.rs` 的 `search_panel`、`open_search` 和 `Popup::Search`（现在的 Go to Agent，用户认可它的样子）；`app/src/search.rs`；`app/src/menu.rs`（全部菜单命令和快捷键）；`app/src/find.rs`（终端查找怎么在 `Term` 里找）；`app/src/view.rs` 的查找部分；`app/src/fonts.rs`、`app/src/footer_icon.rs`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-3-palette`，分支 `p5-3-palette`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-3-palette`。
- **并行任务**：paddock/dev-cards 在 `p5-8-cards` 改 `card.rs`、`sidebar.rs`；paddock/dev-drop 在 `p5-9-drop` 改 `view.rs` 里接收拖放和写入终端的部分。你**不要动** `card.rs`、`sidebar.rs`；`view.rs` 里只可以**新加**给 palette 搜文字用的函数，不改已有代码（尤其不碰拖放、粘贴、查找栏）。

## 要做的
用户原话：“我觉得应该有一个类似command palette（zed那种）可以兼具这两个功能。”；“我看到这两个入口了，ui我还是认可的。就是没找到入口。ui能调整的更加精细些最好。”；看过样稿后：“我觉得设计是可以的。但是要有入口。”

1. **入口**：标题栏右端常驻一个搜索按钮（放大镜、`Search`、键帽 `⌘` `P`，约 220pt 宽、28pt 高、圆角 7、淡底），点它打开 palette。标签多时它不被挤掉（标签先变窄）；全屏和不同界面字号下都在。
2. **打开方式**：`⌘P` 打开、什么都找；`⌘⇧P` 打开并预填 `>`（命令模式）；菜单里原来的 “Go to Agent…” 改成打开 palette，并加一项打开命令模式的菜单项。再按一次同样的快捷键或 Esc 关闭。
3. **样子**：沿用现在 Go to Agent 的结构（上面输入框、下面结果），按样稿细化：宽约 600、离标题栏下方约 44pt、圆角 12、一像素淡边加大阴影、背后的窗口压一层淡暗色；输入行约 48pt（放大镜、模式标签、输入框 15、右侧淡色 `esc`）；结果按组：组名小号大写（10.5、600、淡色）加数量；每行约 34pt（左侧状态圆点或小图标、标题 13、说明 12 淡色、右侧快捷键键帽）；第一行默认选中（选中底色）；没有结果时写 `No results for “…”`；底部一行提示：`↑↓ Select`、`↵ Open`（命令模式 `Run`，文字模式 `Jump to match`），右侧可点的 `> Commands`、`/ Search text`。
4. **三种模式**（按输入的第一个字符）：
   - 默认：AGENTS（状态圆点、名字、状态和缩写目录）、TABS（标签标题、说明、`⌘1`…`⌘9`）、SETTINGS（各设置页）、COMMANDS；没有输入时命令组只列几条常用的并注明 `type > for all`；有输入时四组一起筛选。选中后打开 agent、切到标签、打开对应设置页、执行命令。
   - `>`：只列命令，即菜单里全部命令（New Agent…、New Tab…、New Shell、Split Right/Down/Left/Up…、Close Pane、Close Tab、Zoom Pane、Stop Agent…、Show Attention、Find in Terminal、Settings…、Fold Agents、Sort Agents by Name、Next/Previous Tab、About paddock、Quit paddock 等），右侧显示它的快捷键（从已有的键位表取，不要另写一份）；执行和菜单里点它完全一样。
   - `/`：在当前窗口**所有打开的窗格**保留的输出里找文字（规则同终端查找：字面匹配，有大写字母时才区分大小写），按窗格分组（组名是窗格名，注明几处），每行显示命中的那一行、命中部分高亮；选中后切到那个窗格（必要时切标签），把那一处选中并滚到可见，之后可以接着用 `⌘G`、`⌘⇧G`。结果数要有上限，输入时不能让界面卡顿（例如每个窗格只取前若干处，搜索放后台或防抖）。
5. 键盘：`↑↓` 选、`Enter` 执行、`Esc` 关、`Tab` 不跳出；鼠标悬停有淡底、点击执行。
6. 界面文字一律英文（样稿里就是英文）。

## 共同的视觉规则
- 层次靠字号、字重、颜色深浅，不靠边框和分隔线；只在确有必要时用一像素的淡线。
- 字号用 `ui.px`：10.5 组名、11.5 提示与键帽、12 说明、13 正文、15 输入框。字重只用常规、500、600。
- 间距取 2 的倍数；圆角 6 到 12；热区至少 28pt 见方；悬停底色用主题的选中底色。
- 只从主题取色，可以加透明度；不新增主题颜色键。三套预置主题和大小界面字号下都要好看。
- 图标照 `footer_icon.rs` 用路径画；不用 emoji。

## 怎么算做完
- 用户原话和上面六条达到，看起来和 `Palette.dc.html` 一致。
- 验证只做这些：为不靠界面的规则加测试（三种模式的判定和筛选、命令表和快捷键的对应、文字搜索的匹配规则和上限、选中项随结果变化）；跑一次 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`（在 `app/` 下）；用 `PADDOCK_NO_ACTIVATE=1` 启动自己的窗口截几张看一眼（截图不入库；窗口在后面时 GPUI 不重画，看不到的写进完成记录留给用户）。觉得不够，在回复里说，不要自己加。

## 不要做
- 不动 `card.rs`、`sidebar.rs`；`view.rs` 只新加函数，不改已有代码。
- 不去掉终端查找栏（⌘F）和它的快捷键；不改已有快捷键的含义（`⌘P` 从 Go to Agent 变成 palette 默认模式，这是本任务要的）。
- 不新增主题颜色键，不加依赖，不改 `Cargo.lock`。
- 不读 corral 的内部状态目录；不为了搜文字去读没有打开的 agent 的输出。
- 不要用 `osascript`、System Events 等任何方式模拟按键或鼠标。截图只截自己开的窗口（`GPUI_TERM_WINDOW_ID=1`、`screencapture -x -o -l <编号>`），启动时加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME、临时 `XDG_STATE_HOME` 和假 corral，不读写用户的配置和状态。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`（Claude Code 会弹确认框，无人值守时卡住）。临时文件留在你的 scratchpad 里。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 `~/Applications/paddock.app`。
- 遇到做不到的，停下来报告，等决定，不要自己换别的办法。
- 不合并到 main，不推送。只在 `p5-3-palette` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
