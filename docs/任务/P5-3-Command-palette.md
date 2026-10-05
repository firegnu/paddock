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

## 完成记录

- **做了什么**：
  - 入口：标题栏右端常驻 Search 按钮（放大镜、`Search`、键帽 `⌘` `P`，220×28、圆角 7、淡底加一像素淡线），随界面字号缩放、自身不收缩，标签多时标签先变窄。
  - 打开方式：`⌘P` 打开默认模式，`⌘⇧P` 打开并预填 `>`；同一个快捷键再按关闭，按另一个切到它的模式；Esc 关闭。Agent 菜单里的 “Go to Agent…” 改为 “Search…”（⌘P），并新增 “Command Palette…”（⌘⇧P）；动作 `GoToAgent` 改名 `Search`，新增 `CommandPalette`。
  - `search.rs` 改写为 palette 的规则：三种模式的判定；四组的筛选（每个词都出现在标题或说明里，不分大小写；agent 仍按名字排序、可按项目找）；没输入时命令组只列 New Agent…、New Tab…、New Shell 并注明 `type > for all`；文字模式按窗格分组计数；无结果文案；选中项随结果变化；长行截断，让命中留在视野里。
  - `menu.rs`：`commands()` 直接从菜单栏取全部命令（去掉打开 palette 的两项），`keys()` 从已有键位表取快捷键并写成键帽；执行用 `window.dispatch_action` 从当前窗格派发，和点菜单一样。
  - `find.rs`：`hits()` 按 ⌘F 的规则（同一个 `RegexSearch`：字面匹配，有大写才区分大小写）在一个窗格保留的全部输出里找，最新的在前，每个窗格最多 20 处，多了写 `20+ matches`；被终端折行的行整行显示。`reveal()` 跳转时按历史增长换算位置，并核对那里仍是这处匹配，否则退到最新一处。`find()` 的选中和滚动抽成 `select()` 共用。
  - `view.rs`：只在文件末尾新加一个 `impl` 块，含 `screen()`（给后台搜索）和 `show_match()`（选中、滚到、打开查找栏并填入查询，之后 ⌘G、⌘⇧G 接着找），没改已有代码。
  - `footer_icon.rs`：新加放大镜、设置（两道滑杆）、命令（›）三个路径图标。
  - `window.rs`：palette 的状态和画法，尺寸照样稿（宽 600、标题栏下 44、圆角 12、一像素淡边加大阴影、背后压暗、输入行 48、组名 10.5/600、行 34、提示行 34，右侧 `> Commands`、`/ Search text` 可点）。文字搜索在输入停 120ms 后放到后台线程，结果回来再画。Tab、⇧Tab 在 palette 里移动选中。工作中 agent 的圆点会呼吸。窗格状态圆点的取色抽成自由函数 `dot()`，标签行也用它。
- **验证了什么**：新增 17 项测试：menu 2 项（命令表覆盖菜单全部命令、快捷键与键位表一致、⌘1…⌘9）；find 5 项（最新在前与上限、字面与大小写规则、命中位置含宽字符与折行、新输出之后仍能找回、找不到时退到最新一处）；search 10 项（模式判定、四组筛选、命令模式、标签说明与快捷键、长行截断、窗格计数、无结果、选中项随结果变化）。search 的 10 项是先写，跑红（全部 not yet implemented）后再实现的；折行整行显示也是先改测试跑红再实现；menu 和 find 的其余测试是实现之后补的。`app/` 下 `cargo test --all-targets`（库 145 项及集成测试）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 全部通过（另有上游 `block v0.1.6` 的未来兼容提示）。截图用临时 HOME、临时 `XDG_STATE_HOME`（里面写好两标签三窗格的布局）、假 corral、`PADDOCK_NO_ACTIVATE=1`，只截自己的窗口，不入库，共截了：默认模式、`>`、`split` 筛选、无结果、只有 `/`、`/merge`、Tide、Lagoon、界面字号 18（默认与文字模式），以及 1000pt 宽带 8 个标签的窗口。窗口在后台时 GPUI 只画最初约 0.2 秒，为了截到 palette，截图期间临时在 `PaddockWindow::new` 里加了按环境变量打开 palette 的代码（文字模式那张改成同步搜索），截完已删除，不在提交里。
- **拿主意的地方**：
  - 菜单项名：⌘P 那项叫 “Search…”（与标题栏按钮一致），⌘⇧P 叫 “Command Palette…”，都放在原 Go to Agent 所在的 Agent 菜单。
  - palette 里命令的名字取自菜单项，离开所在菜单就读不懂的三处改写为：`Find…` → Find in Terminal、`Attention…` → Show Attention（这两个照样稿），`Zoom` → Zoom Window。顺序照菜单栏，app 菜单（About、Settings…、Quit）放最后；Copy、Paste、Find Next/Previous、Minimize、Tab 1…9 也在里面（按“菜单里全部命令”）。
  - 键帽顺序照样稿把 ⌘ 放在 ⇧ 前（`⌘ ⇧ D`），而不是 macOS 菜单里的 ⇧⌘D。
  - 筛选规则：每个词都要出现在标题或说明里。agent 的说明含状态和缩写目录，所以按项目、按状态都能找到；设置页的说明写了所含内容，输入 “font” 能找到 General。
  - agent 说明一律写“状态 · 缩写目录”（样稿里 Waiting、Exited 没写目录，当作示意）；标签的说明写 “Tab N” 加上这个标签里 agent 的全名。
  - 文字模式：活动窗格排第一，其余按标签顺序；每个窗格最新的在前（和 ⌘G 往旧处找的顺序一致）；只去掉 `/` 后面紧跟的空格，其余照原样（和 ⌘F 一样不去尾部空格）。只输入 `/` 时只显示输入行和提示行，不留空白列表。
  - 跳到命中后，焦点放在该窗格的查找栏（Enter、⇧Enter、⌘G、⌘⇧G 接着找，Esc 回到底部），而不是留在终端。
  - Tab、⇧Tab 在 palette 里移动选中（等于 ↓、↑），既不跳出，也不往输入框里插制表符。
  - 背后压暗和大阴影用黑色加透明度（和现有选择框的压暗一样），没有取主题色。其余颜色都取主题：底 agents_bg，正文 agents_text，说明和组名 agents_dimmer，组计数和 esc 用 agents_border，选中底 agent_selected，悬停用它的一半透明度，键帽字 muted，命中底 agents_accent 0.28，淡线 agents_rule 0.6。
  - 列表最高 470pt，窗口矮时按剩余高度收，超出的部分滚动；按 ↑↓ 时选中行滚进视野（组里第一行连组名一起）。
- **没做的事**：
  - 键盘、鼠标、输入法都没有实际操作过，以下都留给用户试：⌘P、⌘⇧P 的开关与切换，↑↓ 和 Tab，Enter 执行各类目标，悬停与点击，`> Commands`、`/ Search text`，跳到命中后 ⌘G 接着找，打开真实 agent。截图只看了静态样子，工作中圆点的呼吸看不出。
  - palette 开着时，文字搜索结果不随新输出自动刷新（再打一个字才重新搜）。跳转时如果那一处已经不在原位（历史满了在滚动，或窗格换了会话），退到最新的一处。
  - 呼吸圆点在 window.rs 里另写了一份，和 sidebar.rs 的 `dot` 重复；sidebar 那个改成公开后可以合并，这轮 sidebar.rs 不能动，留给以后。
  - 没去掉终端查找栏和它的快捷键；没改 card.rs、sidebar.rs；没加依赖，没改 Cargo.lock，没加主题颜色键。

## 主控审查

2026-10-06，paddock/main。可以合并，已合并（与 P5-8、P5-9 一起集成）。
- 范围符合约定：没碰 `card.rs`、`sidebar.rs`；`view.rs` 只在末尾新加一个 `impl` 块；`find.rs` 只把“选中并滚到可见”抽成函数供 palette 复用，原行为不变；`footer_icon.rs` 只加图标；`Cargo.lock` 未变。
- 删掉的两个旧 Go to Agent 测试由 `agents_by_name_or_project_ignoring_case`、`settings_pages_and_commands_follow_the_agents_and_tabs` 接替，断言照旧。主控重跑：176 项测试、clippy 通过。
- 对方的取舍，主控认可：菜单项 “Search…”“Command Palette…”、命令名 “Zoom Window”；压暗和阴影用黑色加透明度（与现有选择框一致，不算主题颜色）；文字搜索活动窗格在前、每窗格最多 20 处、停 120ms 后在后台搜；跳到命中后焦点在查找栏。
- 集成（主控做）：palette 和侧栏各有一份一样的呼吸圆点，合成 `sidebar::status_dot`；去掉因此不用的引用。DESIGN §13 补记 P5-3 定下的三项。
- 没做、记为建议改：palette 开着时文字搜索结果不随新输出刷新；要跳的那一处已经滚走时退到最新一处。键盘、鼠标、输入法留给用户试。
